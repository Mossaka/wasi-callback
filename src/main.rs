use std::sync::{Arc, Mutex};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{AsContext, AsContextMut, Config, Engine, Linker, Module, Store};
use wasmtime_wasi::*;

wit_bindgen_wasmtime::import!("event-handler.wit");

pub struct ExecHost {}

impl ExecHost {
    pub fn add_to_linker(linker: &mut wasmtime::Linker<Context>) -> anyhow::Result<()> {
        linker.func_wrap(
            "exec",
            "exec",
            move |mut caller: wasmtime::Caller<Context>| {
                let store = caller.as_context();
                let host = store.data().host.as_ref().unwrap();
                let _res = host
                    .clone()
                    .lock()
                    .unwrap()
                    .event_handler(caller.as_context_mut(), "event-a");
                Ok(())
            },
        )?;
        Ok(())
    }
}

pub struct Context {
    pub wasi: WasiCtx,
    pub guest: EventHandlerData,
    pub host: Option<Arc<Mutex<EventHandler<Self>>>>,
}

fn main() -> Result<()> {
    let wasi = default_wasi()?;
    let guest = EventHandlerData::default();
    let ctx = Context {
        wasi,
        guest,
        host: None,
    };

    let engine = Engine::new(&default_config()?)?;
    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context| &mut cx.wasi)?;
    ExecHost::add_to_linker(&mut linker)?;

    let module = "./target/wasm32-wasi/release/demo.wasm";
    let module = Module::from_file(&engine, module)?;
    let instance = linker.instantiate(&mut store, &module)?;

    let handler = EventHandler::new(&mut store, &instance, |cx: &mut Context| &mut cx.guest)?;
    store.data_mut().host = Some(Arc::new(Mutex::new(handler)));
    instance
        .get_typed_func::<(i32, i32), i32, _>(&mut store, "main")?
        .call(&mut store, (0, 0))?;
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
