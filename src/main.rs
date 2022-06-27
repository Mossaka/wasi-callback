use std::sync::{Arc, Mutex};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use host::exec::{self, ExecTables};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{
    Config, Engine, Global, GlobalType, Linker, Module, Mutability, Store, Val, ValType,
};
use wasmtime_wasi::*;

mod host;

wit_bindgen_wasmtime::import!("event-handler.wit");

pub struct Context {
    pub wasi: WasiCtx,
    pub host: (
        Option<Arc<Mutex<EventHandler<Context2>>>>,
        Option<Arc<Mutex<ExecTables>>>,
        Option<Arc<Mutex<Store<Context2>>>>,
    ),
}

pub struct Context2 {
    pub wasi: WasiCtx,
    pub guest: EventHandlerData,
}

fn main() -> Result<()> {
    let guest = EventHandlerData::default();
    let ctx = Context {
        wasi: default_wasi()?,
        host: (None, None, None),
    };

    let ctx2 = Context2 {
        wasi: default_wasi()?,
        guest,
    };

    let engine = Engine::new(&default_config()?)?;
    let mut linker = Linker::new(&engine);
    let mut linker2 = Linker::new(&engine);
    let mut store = Store::new(&engine, ctx);
    let mut store2 = Store::new(&engine, ctx2);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context| &mut cx.wasi)?;
    wasmtime_wasi::add_to_linker(&mut linker2, |cx: &mut Context2| &mut cx.wasi)?;
    exec::add_to_linker(&mut linker)?;

    // put dummy implementation to these import functions
    let ty = GlobalType::new(ValType::I32, Mutability::Const);
    let g = Global::new(&mut store, ty, Val::I32(0x1234))?;
    linker2.define("exec", "events::get", g)?;
    linker2.define("exec", "events::exec", g)?;
    linker2.define("canonical_abi", "resource_drop_events", g)?;

    let module = "./target/wasm32-wasi/release/demo.wasm";
    let module = Module::from_file(&engine, module)?;
    let instance = linker.instantiate(&mut store, &module)?;
    let instance2 = linker2.instantiate(&mut store2, &module)?;

    let handler = EventHandler::new(&mut store2, &instance2, |cx: &mut Context2| &mut cx.guest)?;
    store.data_mut().host = (
        Some(Arc::new(Mutex::new(handler))),
        Some(Arc::new(Mutex::new(ExecTables::default()))),
        Some(Arc::new(Mutex::new(store2))),
    );
    instance
        .get_typed_func::<(), (), _>(&mut store, "_start")?
        .call(&mut store, ())?;
    Ok(())
}

pub fn default_config() -> Result<Config> {
    let mut config = Config::new();
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_multi_memory(true);
    config.wasm_module_linking(true);
    Ok(config)
}

pub fn default_wasi() -> Result<WasiCtx, StringArrayError> {
    let mut ctx: WasiCtxBuilder = WasiCtxBuilder::new().inherit_stdio().inherit_args()?;
    ctx = ctx
        .preopened_dir(
            Dir::open_ambient_dir("./target", ambient_authority()).unwrap(),
            "cache",
        )
        .unwrap();

    Ok(ctx.build())
}
