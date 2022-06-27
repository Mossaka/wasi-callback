use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use host::exec::{self, Exec, ExecTables};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{AsContext, AsContextMut, Config, Engine, Linker, Module, Store};
use wasmtime_wasi::*;

mod host;

wit_bindgen_wasmtime::import!("event-handler.wit");

fn main() -> Result<()> {
    let guest = EventHandlerData::default();
    let ctx = HostContext {
        wasi: default_wasi()?,
        host: (None, None, None),
    };

    let ctx2 = GuestContext {
        wasi: default_wasi()?,
        guest,
    };
    let engine = Engine::new(&default_config()?)?;
    let (mut linker, mut store, module) =
        wasmtime_init(&engine, ctx, "target/wasm32-wasi/release/demo.wasm")?;
    let (mut linker2, mut store2, module2) =
        wasmtime_init(&engine, ctx2, "target/wasm32-wasi/release/demo.wasm")?;

    exec::add_to_linker::<HostContext>(&mut linker)?;

    // put dummy implementation to these import functions
    linker2.func_wrap(
        "exec",
        "events::exec",
        move |mut _caller: CallerCtx2, _arg0: i32| Ok(()),
    )?;
    linker2.func_wrap("exec", "events::get", move |mut _caller: CallerCtx2| {
        Ok(0_i32)
    })?;
    linker2.func_wrap(
        "canonical_abi",
        "resource_drop_events",
        |mut _caller: CallerCtx2, _handle: u32| Ok(()),
    )?;

    let instance = linker.instantiate(&mut store, &module)?;
    let instance2 = linker2.instantiate(&mut store2, &module2)?;

    let handler = EventHandler::new(&mut store2, &instance2, |cx: &mut GuestContext| {
        &mut cx.guest
    })?;
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
    engine: &Engine,
    ctx: T,
    path: &str,
) -> Result<(Linker<T>, Store<T>, wasmtime::Module)>
where
    T: Ctx,
{
    let mut linker = Linker::new(engine);
    let store = Store::new(engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut T| cx.wasi_mut())?;
    let module = Module::from_file(engine, path)?;
    Ok((linker, store, module))
}

pub trait Ctx {
    type Data;
    fn wasi_mut(&mut self) -> &mut WasiCtx;
    fn data_mut(&mut self) -> &mut Self::Data;
}

pub struct HostContext {
    pub wasi: WasiCtx,
    pub host: HostData,
}

impl Ctx for HostContext {
    type Data = HostData;

    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.host
    }
}

impl Exec for HostContext {
    type Context = Self;

    fn events_get(caller: wasmtime::Caller<'_, Self::Context>) -> Result<i32, wasmtime::Trap> {
        let store = caller.as_context();
        let _tables = store.data().host.1.as_ref().unwrap();
        let handle = _tables.clone().lock().unwrap().events_table.insert(());
        Ok(handle as i32)
    }

    fn events_exec(
        mut caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
    ) -> Result<(), wasmtime::Trap> {
        let mut thread_handles = vec![];
        for i in 0..10 {
            let store = caller.as_context();
            let handler = store.data().host.0.as_ref().unwrap().clone();
            let mut store = caller.as_context_mut();
            let store = store.data_mut().host.2.as_mut().unwrap().clone();
            thread_handles.push(thread::spawn(move || {
                let mut store = store.lock().unwrap();
                let _res = handler
                    .lock()
                    .unwrap()
                    .event_handler(store.deref_mut(), format!("event-{i}").as_str());
            }));
        }
        for handle in thread_handles {
            handle.join().unwrap();
        }
        Ok(())
    }

    fn drop_events(
        caller: wasmtime::Caller<'_, Self::Context>,
        handle: u32,
    ) -> Result<(), wasmtime::Trap> {
        let store = caller.as_context();
        let _tables = store.data().host.1.as_ref().unwrap();
        _tables
            .clone()
            .lock()
            .unwrap()
            .events_table
            .remove(handle)
            .map_err(|e| wasmtime::Trap::new(format!("failed to remove handle: {}", e)))?;
        drop(handle);
        Ok(())
    }
}

pub struct GuestContext {
    pub wasi: WasiCtx,
    pub guest: EventHandlerData,
}

impl Ctx for GuestContext {
    type Data = EventHandlerData;

    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.guest
    }
}

impl Exec for GuestContext {
    type Context = Self;

    fn events_get(_caller: wasmtime::Caller<'_, Self::Context>) -> Result<i32, wasmtime::Trap> {
        Ok(0)
    }

    fn events_exec(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
    ) -> Result<(), wasmtime::Trap> {
        Ok(())
    }

    fn drop_events(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _handle: u32,
    ) -> Result<(), wasmtime::Trap> {
        Ok(())
    }
}

type CallerCtx2<'a> = wasmtime::Caller<'a, GuestContext>;
type GuestStore = Store<GuestContext>;
type HostData = (
    Option<Arc<Mutex<EventHandler<GuestContext>>>>,
    Option<Arc<Mutex<ExecTables>>>,
    Option<Arc<Mutex<GuestStore>>>,
);
