use std::sync::{Arc, Mutex};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use host::exec::{self, ExecTables};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Linker, Module, Store};
use wasmtime_wasi::*;

mod host;

wit_bindgen_wasmtime::import!("event-handler.wit");

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

    let (_engine, mut linker, mut store, module) =
        wasmtime_init(ctx, "target/wasm32-wasi/release/demo.wasm")?;
    let (_engine2, mut linker2, mut store2, module2) =
        wasmtime_init(ctx2, "target/wasm32-wasi/release/demo.wasm")?;

    exec::add_to_linker(&mut linker)?;

    // put dummy implementation to these import functions
    linker2.func_wrap(
        "exec",
        "events::exec",
        move |mut _caller: CallerCtx2, _arg0: i32| Ok(()),
    )?;
    linker2.func_wrap("exec", "events::get", move |mut _caller: CallerCtx2| {
        Ok(0 as i32)
    })?;
    linker2.func_wrap(
        "canonical_abi",
        "resource_drop_events",
        |mut _caller: CallerCtx2, _handle: u32| Ok(()),
    )?;

    let instance = linker.instantiate(&mut store, &module)?;
    let instance2 = linker2.instantiate(&mut store2, &module2)?;

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

pub fn wasmtime_init<T>(
    ctx: T,
    path: &str,
) -> Result<(Engine, Linker<T>, Store<T>, wasmtime::Module)>
where
    T: Ctx,
{
    let engine = Engine::new(&default_config()?)?;
    let mut linker = Linker::new(&engine);
    let store = Store::new(&engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut T| cx.wasi_mut())?;
    let module = Module::from_file(&engine, path)?;
    Ok((engine, linker, store, module))
}

pub trait Ctx {
    type Data;
    fn wasi_mut(&mut self) -> &mut WasiCtx;
    fn data_mut(&mut self) -> &mut Self::Data;
}

pub struct Context {
    pub wasi: WasiCtx,
    pub host: (
        Option<Arc<Mutex<EventHandler<Context2>>>>,
        Option<Arc<Mutex<ExecTables>>>,
        Option<Arc<Mutex<Store<Context2>>>>,
    ),
}

impl Ctx for Context {
    type Data = (
        Option<Arc<Mutex<EventHandler<Context2>>>>,
        Option<Arc<Mutex<ExecTables>>>,
        Option<Arc<Mutex<Store<Context2>>>>,
    );

    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.host
    }
}

pub struct Context2 {
    pub wasi: WasiCtx,
    pub guest: EventHandlerData,
}

impl Ctx for Context2 {
    type Data = EventHandlerData;

    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.guest
    }
}

type CallerCtx2<'a> = wasmtime::Caller<'a, Context2>;
