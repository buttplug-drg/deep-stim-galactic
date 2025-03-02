use mlua;
use mlua::prelude::*;

fn lua_print(lua: &Lua, s: String) {
    let lua_print: mlua::Function = lua.globals().get("print")
        .expect("failed to load lua print function");
    lua_print.call::<LuaValue>(s)
        .expect("failed to call lua print function");
}

fn hello_from_rs(lua: &Lua, _: ()) -> LuaResult<()> {
    lua_print(lua, "Hello from Rust!".into());
    Ok(())
}

#[mlua::lua_module]
fn luabutt(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table().expect("failed to create exports table");

    exports.set("hello_from_rs", lua.create_function(hello_from_rs)?)?;
    
    Ok(exports)
}
