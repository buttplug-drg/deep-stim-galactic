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
use mlua::{self
          , prelude::*
          };

const MSGCHANSZ: usize = 65535;

// INFO: set the server to run at 8tps.
//  cant run it too fast, bc otherwise might end up overwhelming the plug with commands
const TICK_TIME_MILLIS: u64 = 1250;

macro_rules! log {
    ($($arg:tt)*) => {
        println!("[buttplug-lua] {}", format!($($arg)*));
    }
}
macro_rules! info {
    ($($arg:tt)*) => {
        log!("[INFO] {}", format!($($arg)*));
    }
}
macro_rules! warn {
    ($($arg:tt)*) => {
        log!("[WARN] {}", format!($($arg)*));
    }
}
macro_rules! error {
    ($($arg:tt)*) => {
        log!("[ERROR] {}", format!($($arg)*));
    }
}
macro_rules! critical {
    ($($arg:tt)*) => {
        log!("[CRITICAL] {}", format!($($arg)*));
    }
}
macro_rules! fatal {
    ($($arg:tt)*) => {
        log!("[FATAL] {}", format!($($arg)*));
    }
}
macro_rules! log_todo {
    ($($arg:tt)*) => {
        warn!("[TODO] {}", format!($($arg)*));
    }
}

trait LuaLog {
    fn log(&self, s: &str);
    fn log_info(&self, s: &str);
    fn log_warn(&self, s: &str);
    fn log_error(&self, s: &str);
    fn log_critical(&self, s: &str);
    fn log_fatal(&self, s: &str);
}
impl LuaLog for Lua {
    fn log(&self, s: &str) {
        let lua_print: LuaFunction = self.globals().get("print")
            .expect("failed to load lua print function");
        lua_print.call::<LuaValue>(String::from(s))
            .expect("failed to call lua print function");
        log!("{}", s);
    }
    fn log_info(&self, s: &str) {
        self.log(&format!("[INFO] {}", s));
    }
    fn log_warn(&self, s: &str) {
        self.log(&format!("[WARN] {}", s));
    }
    fn log_error(&self, s: &str) {
        self.log(&format!("[ERROR] {}", s));
    }
    fn log_critical(&self, s: &str) {
        self.log(&format!("[CRITICAL] {}", s));
    }
    fn log_fatal(&self, s: &str) {
        self.log(&format!("[FATAL] {}", s));
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
// macro_rules! c_for {
//     (($init:stmt; $cond:expr; $iter:stmt) $blk:block) => {
//         $init
//         while $cond
//             $blk
//     }
// }

// TODO: think about vibration types
//
// limit ticks to u8 to make sure effects stay short
// i rlly dont wanna bother with finding some esoteric way to stop specific effects.
enum VibrationCmd {
    SetBase{ strength: f64 },
    AddTempStatic{ strength: f64, ticks: u8 },
    AddLinearDecay{ peak: f64, ticks: u8 },
    AddLinearRamp{ peak: f64, ticks: u8 },
    AddTempSqWave{ lo: f64, hi: f64, ticks_lo: u8, ticks_hi: u8, ticks: u8 },
    StopVibration,
    Shutdown,
}

enum VibrationEffect {
    Static{ strength: f64, ticks_total: u8, ticks_elapsed: u8 },
    LinDecay{ peak: f64, ticks_total: u8, ticks_elapsed: u8 },
    LinRamp{ peak: f64, ticks_total: u8, ticks_elapsed: u8 },
    SqWave{ lo: f64, hi: f64, ticks_lo: u8, ticks_hi: u8, ticks_total: u8, ticks_elapsed: u8 },
}
impl VibrationEffect {
    fn get_vibration(&self) -> f64 {
        match *self {
            Self::Static{ strength, .. } => strength,
            Self::LinDecay{ peak, ticks_total, ticks_elapsed } => {
                peak - peak * (ticks_elapsed as f64 / ticks_total as f64)
            }
            Self::LinRamp{ peak, ticks_total, ticks_elapsed } => {
                peak * (ticks_elapsed as f64 / ticks_total as f64)
            }
            Self::SqWave{ lo, hi, ticks_lo, ticks_hi, ticks_total: _, ticks_elapsed } => {
                let modded = ticks_elapsed % (ticks_lo + ticks_hi);
                return if modded < ticks_lo {
                    lo
                } else {
                    hi
                }
            }
        }
    }
    fn should_stop(&self) -> bool {
        match *self {
            // this copypasta is a little bit dumb, considering.
            // whagever;
            // theres no way to generically access enum struct members that exist on all variants
            // (tbf the offsets within the struct arent guaranteed to be consistent in rust)
            // so this stupid destructuring is unfortunately necessary.
            Self::Static{ ticks_total, ticks_elapsed, .. } => ticks_elapsed >= ticks_total,
            Self::LinDecay{ ticks_total, ticks_elapsed, .. } => ticks_elapsed >= ticks_total,
            Self::LinRamp{ ticks_total, ticks_elapsed, .. } => ticks_elapsed >= ticks_total,
            Self::SqWave{ ticks_total, ticks_elapsed, .. } => ticks_elapsed >= ticks_total,
        }
    }
}

enum PushMsgError {
    NotInitialized,
    Full,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static SEND: OnceLock<Sender<VibrationCmd>> = OnceLock::new();

fn init(lua: &Lua, server_port: u16) -> LuaResult<()> {
    // most of this fn is just copied from https://github.com/qdot/buttplug-nightmare-kart/blob/master/buttplug-mlua/src/lib.rs
    // ..whagever. thx qdot!
    lua.log_info("initializing runtime...");

    if let Some(_) = RUNTIME.get() {
        lua.log_info("runtime already initialized.");
        return Ok(());
    }

    lua.log_info("> creating runtime");
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

    lua.log_info("> creating channel");
    let (send, recv) = channel::<VibrationCmd>(MSGCHANSZ);
    let _ = SEND.set(send);

    lua.log_info("> starting thread...");
    lua.log_warn("> logs from this thread will only be visible on stdout.");
    runtime.spawn(async move {
        run(recv, server_port).await
    });
    let _ = RUNTIME.set(runtime);

    lua.log_info("init complete");

    Ok(())
}

fn get_ws_addr(server_port: u16) -> String {
    format!("ws://localhost:{}", server_port)
}

macro_rules! connect {
    ($client:ident, $port:ident) => {
        // INFO: currently, the idea is to try to connect every 5s until it succeeds
        //  ..i'm wondering about whether there is a better way.
        loop {
            let connector = new_json_ws_client_connector(&get_ws_addr($port));
            if $client.connect(connector).await.is_ok() {
                break;
            }
            info!("Failed to connect to server. Reattempting in 5s...");
            sleep(Duration::from_secs(5)).await;
        }
        assert!($client.connected());
        info!("Successfully connected to server.");
    }
}

async fn run(mut recv: Receiver<VibrationCmd>, server_port: u16) {
    let client = ButtplugClient::new("buttplug-lua");
    connect!(client, server_port);

    let mut evt_stream = client.event_stream();

    let tick_time = Duration::from_millis(TICK_TIME_MILLIS);
    let devices = client.devices();
    if devices.len() > 0 {
        info!("connected devices:");
        for device in devices {
            info!("    {}", device.name());
        }
    } else {
        info!("no devices currently connected.");
    }

    let mut last_vibration: f64 = 0.0;
    let mut base_vibration: f64 = 0.0;
    let mut additional_effects = Vec::<VibrationEffect>::new();

    // main loop
    'outer: loop {
        let now = Instant::now();
        let was_vibration_stop = false;

        macro_rules! continue_if_stop_vibration {
            () => {
                if (was_vibration_stop) {
                    info!("StopVibration was requested; dropping command.");
                    continue;
                }
            }
        }
        
        info!("start buttplug event handling");
        while let Some(Some(evt)) = evt_stream.next().now_or_never() {  // hmgh. sure. whagever
            // it's safe to use .now_or_never() to consume the evts, bc the evt queue only serves
            // items that have already been received locally by the BP client
            // so it's a case of Either having to wait for an evt (when the stream's empty)
            //      in which case we sorta just discard empty future
            // Or we just instantly get an evt out of it
            // either way, no evts should be discarded.
            // thx qdot
            info!("event get!: {evt:?}");
            match evt {
                ButtplugClientEvent::ScanningFinished => {
                    info!("Scanning finished");
                }
                ButtplugClientEvent::ServerConnect => {
                    info!("received server connection event");
                }
                ButtplugClientEvent::ServerDisconnect => {
                    warn!("Server disconnected. Attempting to reconnect.");
                    connect!(client, server_port);
                }
                ButtplugClientEvent::DeviceAdded(device) => {
                    info!("New device connected: {}", device.name());
                }
                ButtplugClientEvent::DeviceRemoved(device) => {
                    info!("Device disconnected: {}", device.name());
                }
                ButtplugClientEvent::PingTimeout => {
                    fatal!("(fatal..?) Ping timeout");
                }
                ButtplugClientEvent::Error(e) => {
                    match e {
                        ButtplugError::ButtplugHandshakeError(_) => {
                            error!("Handshake error");
                        }
                        ButtplugError::ButtplugMessageError(_) => {
                            error!("Message error");
                        }
                        ButtplugError::ButtplugPingError(_) => {
                            error!("Ping error");
                        }
                        ButtplugError::ButtplugDeviceError(_) =>{
                            error!("Device error");
                        } 
                        ButtplugError::ButtplugUnknownError(_) =>{
                            error!("Unknown error");
                        } 
                    }
                    // INFO: when an error happens, just proceed to shutdown routine for now
                    //  it's someone's ass we're talking about. lets just try to 
                    break 'outer;
                }
            }
        }
        info!("buttplug event handling complete");

        info!("start deepcock event handling");
        loop {
            match &recv.try_recv() {
                Ok(msg) => match *msg {
                    VibrationCmd::SetBase{ strength } => {
                        info!("set base vibration to {strength:.2}");
                        continue_if_stop_vibration!();
                        base_vibration = clamp(strength, 0.0, 1.0);
                    }
                    VibrationCmd::AddTempStatic{ strength, ticks} => {
                        info!("add temp static vibration {strength:.2} for {ticks} ticks");
                        continue_if_stop_vibration!();
                        additional_effects.push(VibrationEffect::Static{ strength, ticks_total: ticks, ticks_elapsed: 0 });
                    }
                    VibrationCmd::AddLinearDecay{ peak, ticks } => {
                        info!("add linear decay from {peak:.2} over {ticks} ticks");
                        continue_if_stop_vibration!();
                        additional_effects.push(VibrationEffect::LinDecay{ peak, ticks_total: ticks, ticks_elapsed: 0 });
                    }
                    VibrationCmd::AddLinearRamp{ peak, ticks} => {
                        info!("add linear ramping to {peak:.2} over {ticks} ticks");
                        continue_if_stop_vibration!();
                        additional_effects.push(VibrationEffect::LinRamp{ peak, ticks_total: ticks, ticks_elapsed: 0 });
                    }
                    VibrationCmd::AddTempSqWave{ lo, hi, ticks_lo, ticks_hi, ticks } => {
                        info!("add sq wave from {lo:.2} ({ticks_lo} ticks) to {hi:.2} ({ticks_hi} ticks) for {ticks} ticks");
                        continue_if_stop_vibration!();
                        additional_effects.push(VibrationEffect::SqWave{ lo, hi, ticks_lo, ticks_hi, ticks_total: ticks, ticks_elapsed: 0 });
                    }
                    VibrationCmd::StopVibration => {
                        info!("stop vibration");
                        warn!("All vibrations will be stopped. Any yet-unprocessed vibrations from this tick will be stopped.");
                        base_vibration = 0.0;
                        additional_effects.clear();
                    }
                    VibrationCmd::Shutdown => {
                        info!("Shutdown");
                        break 'outer;
                    }
                }
                Err(e) => {
                    // legit no idea how this could possibly happen.
                    // thats what asserts are for tho
                    assert!(!matches!(*e, TryRecvError::Disconnected),
                            "Sender side unexpectedly and impossibly disconnected.");

                    // the only other thing that can possibly be is that the channel is empty for now.
                    // we just stop reading the channel in that case
                    break;
                }
            };
        }
        info!("deepcock event handling complete");

        // clear out all the already-over vibrations
        // do this after adding all the new effects to instantly remove zero-duration effects
        let mut idx = 0;
        while idx < additional_effects.len() {
            if additional_effects[idx].should_stop() {
                additional_effects.swap_remove(idx);  // swap-remove is fine bc order of elems
                                                      // within the arr doesnt matter
                continue;
            }
            idx += 1;
        }

        let mut next_vibration = base_vibration;
        for effect in &additional_effects {
            next_vibration += effect.get_vibration();
        }
        next_vibration = clamp(next_vibration, 0.0, 1.0);

        if next_vibration == 0.0 {  // special case where next_vibration_strength == 0:
                                    // use stop() command to make sure it absolutely stops
                                    // == is oke here since maybe a slight, nearly imperceptible
                                    // rumble is actually desired
            let mut vibration_futures = vec![];
            let devices = client.devices();
            for device in &devices {
                vibration_futures.push(device.stop());
            }
            for (idx, device_result) in join_all(vibration_futures).await.into_iter().enumerate() {
                if let Err(e) = device_result {
                    match e {
                        ButtplugClientError::ButtplugConnectorError(_) => {
                            warn!("Buttplug connector error (device {})", devices[idx].name());
                        }
                        ButtplugClientError::ButtplugError(_) => {
                            warn!("Buttplug error (device {})", devices[idx].name());
                        }
                    }
                }
            }
        } else if !roughly_eq(next_vibration, last_vibration) {
            let mut vibration_futures = vec![];
            let devices = client.devices();
            for device in &devices {
                vibration_futures.push(device.vibrate(&ScalarValueCommand::ScalarValue(next_vibration)));
            }
            for (idx, device_result) in join_all(vibration_futures).await.into_iter().enumerate() {
                if let Err(e) = device_result {
                    match e {
                        ButtplugClientError::ButtplugConnectorError(_) => {
                            warn!("Buttplug connector error (device {})", devices[idx].name());
                        }
                        ButtplugClientError::ButtplugError(_) => {
                            warn!("Buttplug error (device {})", devices[idx].name());
                        }
                    }
                }
            }
            last_vibration = next_vibration;
        }

        sleep_until(now + tick_time).await;
    }

    // shutdown logic
    info!("Attempting to stop all known devices.");
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
    info!("Shuting down");
}

// not currently sure of the utility of returning an error from this function
// since the error handling would have to be done on a per-wrapper-function basis
fn push_msg(lua: &Lua, msg: VibrationCmd) -> Result<(), PushMsgError> {
    let send = match SEND.get() {
        Some(send) => send,
        None => {
            lua.log_warn("Failed to send message: Sender is not initialized.");
            return Err(PushMsgError::NotInitialized);
        }
    };
    match send.try_send(msg) {
        Ok(_) => {
            return Ok(());
        }
        Err(e) => match e {
            TrySendError::Closed(_) => {
                lua.log_fatal("Receiver side unexpectedly and impossibly disconnected.");
                panic!("Receiver side unexpectedly and impossibly disconnected.");
            }
            TrySendError::Full(_) => {
                lua.log_warn("Message channel is full; messages will be dropped until there is space.");
                return Err(PushMsgError::Full);
            }
        }
    };
}

fn roughly_eq(a: f64, b: f64) -> bool {
    return (a - b).abs() < 0.01;
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

fn percent_to_float(percent: u8) -> f64 {
    (percent as f64) / 100.0
}
fn set_base(lua: &Lua, strength: f64) -> LuaResult<()> {
    lua.log_info("request base vibration to {strength:.2}");
    let _ = push_msg(lua, VibrationCmd::SetBase{ strength });

    Ok(())
}

fn add_temp_static(lua: &Lua, (strength, ticks): (f64, u8)) -> LuaResult<()> {
    lua.log_info("request transient vibration: strength={strength:.2}; duration={duration_ticks}");
    let _ = push_msg(lua, VibrationCmd::AddTempStatic{ strength, ticks });

    Ok(())
}

fn add_linear_decay(lua: &Lua, (peak, ticks): (f64, u8)) -> LuaResult<()> {
    lua.log_info("request transient vibration: strength={strength:.2}; duration={duration_ticks}");
    let _ = push_msg(lua, VibrationCmd::AddLinearDecay{ peak, ticks });

    Ok(())
}
fn add_linear_ramp(lua: &Lua, (peak, ticks): (f64, u8)) -> LuaResult<()> {
    lua.log_info("request transient vibration: strength={strength:.2}; duration={duration_ticks}");
    let _ = push_msg(lua, VibrationCmd::AddLinearRamp{ peak, ticks });

    Ok(())
}
fn add_temp_sq_wave(lua: &Lua, (lo, hi, ticks_lo, ticks_hi, ticks): (f64, f64, u8, u8, u8)) -> LuaResult<()> {
    lua.log_info("request transient vibration: strength={strength:.2}; duration={duration_ticks}");
    let _ = push_msg(lua, VibrationCmd::AddTempSqWave{ lo, hi, ticks_lo, ticks_hi, ticks });

    Ok(())
}

fn stop_vibration(lua: &Lua, _: ()) -> LuaResult<()> {
    info!("request stop vibration");
    let _ = push_msg(lua, VibrationCmd::StopVibration);

    Ok(())
}
fn shutdown(lua: &Lua, _: ()) -> LuaResult<()> {
    info!("request shutdown");
    let _ = push_msg(lua, VibrationCmd::Shutdown);

    Ok(())
}

// fn hello_from_rs(lua: &Lua, _: ()) -> LuaResult<()> {
//     lua.log("Hello from Rust!");
//
//     Ok(())
// }

// fn add_export<F, A, R>(lua: &Lua, table: &LuaTable, name: &str, func: F) -> LuaResult<()>
// where
//     F: Fn(&Lua, A) -> LuaResult<R> + LuaMaybeSend + 'static,
//     A: FromLuaMulti,
//     R: IntoLuaMulti,
// {
//     table.set(name, lua.create_function(func)?)
// }

#[mlua::lua_module]
fn luabutt(lua: &Lua) -> LuaResult<LuaTable> {
    lua.log_info("create module table");
    let exports = lua.create_table().expect("failed to create exports table");

    // fuck
    // your
    // type
    // system                                                   // ok boss, what now
    macro_rules! add_export {
        ($fn:ident) => {
            exports.set(stringify!($fn), lua.create_function($fn)?)
        }
    }
    lua.log_info("add fn 'init'");
    add_export!(init)?;
    lua.log_info("add fn 'set_base'");
    add_export!(set_base)?;
    lua.log_info("add fn 'add_temp_static'");
    add_export!(add_temp_static)?;
    lua.log_info("add fn 'add_linear_decay'");
    add_export!(add_linear_decay)?;
    lua.log_info("add fn 'add_linear_ramp'");
    add_export!(add_linear_ramp)?;
    lua.log_info("add fn 'add_temp_square_wave'");
    add_export!(add_temp_sq_wave)?;
    lua.log_info("add fn 'stop_vibration'");
    add_export!(stop_vibration)?;
    lua.log_info("add fn 'shutdown'");
    add_export!(shutdown)?;

    lua.log_info("module table filled. good job everynyan");

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
