// fully qualified imports, because they rlly are just easier to work with
// exactly 1 dep per line + easily visible fully qualified names are kinda nice
// honestly, rare java w
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;
use std::time::Duration;

use buttplug::client::ButtplugClient;
use buttplug::core::connector::new_json_ws_client_connector;
use buttplug::core::connector::ButtplugWebsocketClientTransport;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

use mlua;
use mlua::prelude::*;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static SENDER: OnceLock<Sender<f64>> = OnceLock::new();

async fn run(recv: Receiver<f64>) {
    let client = ButtplugClient::new("Buttplug Lua Native Binding");
    
    loop {
        println!("Attempting to connect...");
        let connector = new_json_ws_client_connector("ws://localhost:12345");
        if client.connect(connector).await.is_err() {
            println!("Connection failed.");
        } else {
            println!("Connected.");
            break;
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn hello(_: &Lua, name: String) -> LuaResult<()> {
    println!("hello, {}!", name);

    Ok(())
}

fn init(_: &Lua, _: ()) -> LuaResult<()> {
    println!("Starting init");
    if RUNTIME.get().is_some() {
    println!("Runtime already set");
      return Ok(());
    }

    println!("Setting sender");
    let (send, recv) = channel(255);
    SENDER.set(send);
    
    println!("creating runtime");
    let runtime = {
        tokio::runtime::Builder::new_multi_thread()
          .enable_all()
          .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::Relaxed);

            format!("buttplug-lua-thread-{id}")
          })
          .on_thread_start(|| {})
          .build()
          .unwrap()
    };
    println!("Spawning task");
    runtime.spawn(async move {
        run(recv).await
    });
    println!("setting runtime");
    RUNTIME.set(runtime);
    std::thread::sleep(Duration::from_secs(10));

    Ok(())  
}

fn vibrate(_: &Lua, speed: f64) -> LuaResult<()> {
    Ok(())
}

#[mlua::lua_module]
fn buttplug_mlua(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("hello", lua.create_function(hello)?)?;
    exports.set("init", lua.create_function(init)?)?;
    exports.set("vibrate", lua.create_function(vibrate)?)?;
    Ok(exports)
}
