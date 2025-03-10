use buttplug::{ client::{ ButtplugClient
                        , ButtplugClientError
                        , ButtplugClientEvent
                        , device::ScalarValueCommand
                        }
              , core::{ connector::{ ButtplugWebsocketClientTransport
                                   , new_json_ws_client_connector
                                   }
                      , errors::ButtplugError
                      }
              };
use std::{ sync::{ atomic::{ AtomicUsize
                           , Ordering
                           }, OnceLock
                 }
         , time::Duration
         , cmp::PartialOrd
         };
use tokio::{ runtime::Runtime
           , sync::mpsc::{ channel
                         , Receiver
                         , Sender
                         , error::{ TryRecvError
                                  , TrySendError
                                  }
                         }
           , time::{ Instant
                   , sleep
                   , sleep_until
                   // , timeout
                   }
           // , select
           };
// sadly, i have to introduce another dependency to use my fucking futures correctly
// every day i become more of a js dev.
// but i rlly dont wanna write rust async library code myself rn
// and it would amount to stackoverflow copypasta anyways
use futures::{ StreamExt
             , future::{ FutureExt
                       , join_all
                       }
             };
use mlua;
use mlua::prelude::*;

const MSGCHANSZ: usize = 65535;

// INFO: set the server to run at 8tps.
//  cant run it too fast, bc otherwise might end up overwhelming the plug with commands
const TICK_TIME_MILLIS: u64 = 125;

macro_rules! log {
    ($($arg:tt)*) => {
        print!("[buttplug-lua] ");
        println!($($arg)*);
    }
}

trait LuaPrint {
    fn log(&self, s: &str);
}
impl LuaPrint for Lua {
    fn log(&self, s: &str) {
        let lua_print: LuaFunction = self.globals().get("print")
            .expect("failed to load lua print function");
        lua_print.call::<LuaValue>(String::from(s))
            .expect("failed to call lua print function");
        log!("{}", s);
    }
}

// macro_rules! do_while {
//     ($blk:block, $cond:expr) => {
//         loop {
//             $blk;
//             if ($cond) { break; }
//         }
//     }
// }

// TODO: think about vibration types
enum Msg {
    SetVibration(f64),
    AddVibration(f64),
    StopVibration,
    Shutdown,
}

enum PushMsgError {
    NotInitialized,
    Full,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static SEND: OnceLock<Sender<Msg>> = OnceLock::new();

fn init(lua: &Lua, server_port: u16) -> LuaResult<()> {
    // most of this fn is just copied from https://github.com/qdot/buttplug-nightmare-kart/blob/master/buttplug-mlua/src/lib.rs
    // ..whagever. thx qdot!
    lua.log("initializing runtime...");
    eprintln!("[buttplug] Hello STDERR\\n");

    if let Some(_) = RUNTIME.get() {
        lua.log("runtime already initialized.");
        return Ok(());
    }

    lua.log("> creating runtime");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::Relaxed);
            format!("buttplug-lua-thread-{id}")
        })
        .on_thread_start(|| {})
        .build()
        .unwrap();

    lua.log("> creating channel");
    let (send, recv) = channel::<Msg>(MSGCHANSZ);
    let _ = SEND.set(send);

    lua.log("> starting thread...");
    lua.log("> [WARN] logs from this thread will only be visible on stdout.");
    runtime.spawn(async move {
        run(recv, server_port).await
    });
    let _ = RUNTIME.set(runtime);

    lua.log("init complete");

    Ok(())
}

fn get_ws_addr(server_port: u16) -> String {
    format!("ws://localhost:{}", server_port)
}

// HACK: lots of copypasta in the following.
// TODO: factor out some of the more egregious, legibility-harming copypasta
//  - introduce state struct maybe?
async fn run(mut recv: Receiver<Msg>, server_port: u16) {
    let client = ButtplugClient::new("buttplug-lua");

    // INFO: currently, the idea is to try to connect every 5s until it succeeds
    //  ..i'm wondering about whether there is a better way.
    loop {
        let connector = new_json_ws_client_connector(&get_ws_addr(server_port));
        // match client.connect(connector).await {
        //     Ok(()) => break,
        //     Err(e) => ...
        if client.connect(connector).await.is_ok() {
            break;
        }
        log!("Failed to connect to server. Reattempting in 5s...");
        sleep(Duration::from_secs(5)).await;
    }
    assert!(client.connected());
    log!("Successfully connected to server.");

    let mut evt_stream = client.event_stream();

    let tick_time = Duration::from_millis(TICK_TIME_MILLIS);
    let devices = client.devices();
    if devices.len() > 0 {
        log!("[INFO] connected devices:");
        for device in devices {
            log!("[INFO]     {}", device.name());
        }
    } else {
        log!("no devices currently connected.");
    }

    let mut last_vibration_strength: f64 = 0.0;
    let mut next_vibration_strength: f64 = 0.0;

    // main loop
    'outer: loop {
        let now = Instant::now();
        let devices = client.devices();
        while let Some(Some(evt)) = evt_stream.next().now_or_never() {  // hmgh. sure. whagever
            // it's safe to use .now_or_never() to consume the evts, bc the evt queue only serves
            // items that have already been received locally by the BP client
            // so it's a case of Either having to wait for an evt (when the stream's empty)
            //      in which case we sorta just discard empty future
            // Or we just instantly get an evt out of it
            // either way, no evts should be discarded.
            // thx qdot
            log!("[INFO] event get!: {evt:?}");
            match evt {
                ButtplugClientEvent::ScanningFinished => {
                    log!("[INFO] Scanning finished");
                }
                ButtplugClientEvent::ServerConnect => {
                    log!("[INFO] received server connection event");
                }
                ButtplugClientEvent::ServerDisconnect => {
                    log!("[WARN] Server disconnected. Attempting to reconnect.");
                    loop {
                        let connector = new_json_ws_client_connector(&get_ws_addr(server_port));
                        // match client.connect(connector).await {
                        //     Ok(()) => break,
                        //     Err(e) => ...
                        if client.connect(connector).await.is_ok() {
                            break;
                        }
                        log!("Failed to connect to server. Reattempting in 5s...");
                        sleep(Duration::from_secs(5)).await;
                    }
                    assert!(client.connected(),
                            "Client unexpectedly disconnected from server immediately after successfully connecting. the fuck?");
                    log!("[INFO] Successfully reconnected to server.");
                }
                ButtplugClientEvent::DeviceAdded(device) => {
                    log!("[INFO] New device connected: {}", device.name());
                }
                ButtplugClientEvent::DeviceRemoved(device) => {
                    log!("[INFO] Device disconnected: {}", device.name());
                }
                ButtplugClientEvent::PingTimeout => {
                    log!("[FATAL..?] Ping timeout");
                }
                ButtplugClientEvent::Error(e) => {
                    match e {
                        ButtplugError::ButtplugHandshakeError(_) => {
                            log!("[ERROR] Handshake error");
                        }
                        ButtplugError::ButtplugMessageError(_) => {
                            log!("[ERROR] Message error");
                        }
                        ButtplugError::ButtplugPingError(_) => {
                            log!("[ERROR] Ping error");
                        }
                        ButtplugError::ButtplugDeviceError(_) =>{
                            log!("[ERROR] Device error");
                        } 
                        ButtplugError::ButtplugUnknownError(_) =>{
                            log!("[ERROR] Unknown error");
                        } 
                    }
                    // INFO: when an error happens, just proceed to shutdown routine for now
                    //  it's someone's ass we're talking about. lets just try to 
                    break 'outer;
                }
            }
        }

        loop {
            match recv.try_recv() {
                Ok(msg) => match msg {
                    Msg::SetVibration(strength) => {
                        next_vibration_strength = strength;
                        log!("[INFO] set vibration to {strength:.2}");
                    }
                    Msg::AddVibration(strength) => {
                        next_vibration_strength += strength;
                    }
                    Msg::StopVibration => {
                        log!("[INFO] stop vibration");
                        next_vibration_strength = 0.0;  // same thing
                    }
                    Msg::Shutdown => {
                        log!("[INFO] Shutdown");
                        break 'outer;
                    }
                }
                Err(e) => {
                    // legit no idea how this could possibly happen.
                    // thats what asserts are for tho
                    assert_ne!(e, TryRecvError::Disconnected,
                               "Sender side unexpectedly and impossibly disconnected.");

                    // the only other thing that can possibly be is that the channel is empty for now.
                    // we just stop reading the channel in that case
                    break;
                }
            };

        }

        next_vibration_strength = clamp(next_vibration_strength, 0.0, 1.0);

        // it's oke to use == on floats here i think
        // mainly wanna prevent spamming the device with the same vibration strength over n over
        // again for no good reason
        // this is enough to prevent that.
        if next_vibration_strength == 0.0 {  // special case where next_vibration_strength == 0:
                                             // use stop() command to make sure it absolutely stops
            let mut vibration_futures = vec![];
            let devices = client.devices();
            for device in &devices {
                vibration_futures.push(device.stop());
            }
            for (idx, device_result) in join_all(vibration_futures).await.into_iter().enumerate() {
                if let Err(e) = device_result {
                    match e {
                        ButtplugClientError::ButtplugConnectorError(_) => {
                            log!("[WARN] Buttplug connector error (device {})", devices[idx].name());
                        }
                        ButtplugClientError::ButtplugError(_) => {
                            log!("[WARN] Buttplug error (device {})", devices[idx].name());
                        }
                    }
                }
            }
        } else if next_vibration_strength != last_vibration_strength {
            let mut vibration_futures = vec![];
            let devices = client.devices();
            for device in &devices {
                vibration_futures.push(device.vibrate(&ScalarValueCommand::ScalarValue(next_vibration_strength)));
            }
            for (idx, device_result) in join_all(vibration_futures).await.into_iter().enumerate() {
                if let Err(e) = device_result {
                    match e {
                        ButtplugClientError::ButtplugConnectorError(_) => {
                            log!("[WARN] Buttplug connector error (device {})", devices[idx].name());
                        }
                        ButtplugClientError::ButtplugError(_) => {
                            log!("[WARN] Buttplug error (device {})", devices[idx].name());
                        }
                    }
                }
            }
        }

        last_vibration_strength = next_vibration_strength;

        sleep_until(now + tick_time).await;
    }

    // shutdown logic
    log!("[INFO] Attempting to stop all known devices.");
    // dont trust the server alone to stop all devices properly
    // not that i think it's bad
    // just.
    // it's someone's ass we're talking about
    // might as well add another attempt at a failsafe
    let mut shutdown_futures = vec![];
    for device in client.devices() {
        shutdown_futures.push(device.stop());
    }
    let _ = join_all(shutdown_futures).await;
    let _ = client.disconnect().await;
    log!("[INFO] Shuting down");
}

// not currently sure of the utility of returning an error from this function
// since the error handling would have to be done on a per-wrapper-function basis
fn push_msg(lua: &Lua, msg: Msg) -> Result<(), PushMsgError> {
    let send = match SEND.get() {
        Some(send) => send,
        None => {
            lua.log("[WARN] Failed to send message: Sender is not initialized.");
            return Err(PushMsgError::NotInitialized);
        }
    };
    match send.try_send(msg) {
        Ok(_) => {
            return Ok(());
        }
        Err(e) => match e {
            TrySendError::Closed(_) => {
                lua.log("[FATAL] Receiver side unexpectedly and impossibly disconnected.");
                panic!("Receiver side unexpectedly and impossibly disconnected.");
            }
            TrySendError::Full(_) => {
                lua.log("[WARN] Message channel is full; messages will be dropped until there is space.");
                return Err(PushMsgError::Full);
            }
        }
    };
}

fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        b
    } else {
        a
    }
}
fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        b 
    } else {
        a 
    }
}
fn clamp<T: PartialOrd>(x: T, lo: T, hi: T) -> T {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

fn set_vibration(lua: &Lua, strength: f64) -> LuaResult<()> {
    log!("[INFO] request vibration to {strength:.2}");
    let _ = push_msg(lua, Msg::SetVibration(strength));

    Ok(())
}
fn set_vibration_percent(lua: &Lua, strength: u8) -> LuaResult<()> {
    log!("[INFO] request vibration to {strength}%");
    let _ = push_msg(lua, Msg::SetVibration((strength as f64) / 100.));

    Ok(())
}

fn stop_vibration(lua: &Lua, _: ()) -> LuaResult<()> {
    log!("[INFO] request stop vibration");
    let _ = push_msg(lua, Msg::StopVibration);

    Ok(())
}
fn shutdown(lua: &Lua, _: ()) -> LuaResult<()> {
    log!("[INFO] request shutdown");
    let _ = push_msg(lua, Msg::Shutdown);

    Ok(())
}

fn hello_from_rs(lua: &Lua, _: ()) -> LuaResult<()> {
    lua.log("Hello from Rust!");

    Ok(())
}

#[mlua::lua_module]
fn luabutt(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table().expect("failed to create exports table");

    exports.set("hello_from_rs", lua.create_function(hello_from_rs)?)?;
    exports.set("init", lua.create_function(init)?)?;
    exports.set("set_vibration", lua.create_function(set_vibration)?)?;
    exports.set("set_vibration_percent", lua.create_function(set_vibration_percent)?)?;
    exports.set("stop_vibration", lua.create_function(stop_vibration)?)?;
    exports.set("shutdown", lua.create_function(shutdown)?)?;

    Ok(exports)
}

/*
#[cfg(test)]
mod test {
    #[test]
    fn test() {

    }
}
*/
