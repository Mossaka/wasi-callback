use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
// use events::ExecTables;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::*;

wit_bindgen_wasmtime::import!("event-handler.wit");
wit_bindgen_wasmtime::export!("exec.wit");

#[derive(Default)]
pub struct Exec {
    guest_handler: Option<Arc<Mutex<EventHandler<GuestContext>>>>,
    guest_store: Option<Arc<Mutex<Store<GuestContext>>>>,
    observables: Vec<MyObservable>,
}

impl exec::Exec for Exec {
    type Events = ();

    fn events_get(&mut self) -> Result<Self::Events, exec::Error> {
        Ok(())
    }

    fn events_listen(
        &mut self,
        self_: &Self::Events,
        ob: exec::Observable<'_>,
    ) -> Result<Self::Events, exec::Error> {
        let _ob = MyObservable {
            rd: ob.rd.to_string(),
            key: ob.key.to_string(),
        };
        self.observables.push(_ob);
        Ok(())
    }

    fn events_exec(&mut self, self_: &Self::Events, duration: u64) -> Result<(), exec::Error> {
        let mut thread_handles = vec![];
        for i in 0..10 {
            let handler = self.guest_handler.as_ref().unwrap().clone();
            let store = self.guest_store.as_mut().unwrap().clone();
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
}

#[derive(Default)]
pub struct GuestExec {
    data: EventHandlerData,
}

impl exec::Exec for GuestExec {
    type Events = ();

    fn events_get(&mut self) -> Result<Self::Events, exec::Error> {
        Ok(())
    }

    fn events_listen(
        &mut self,
        self_: &Self::Events,
        ob: exec::Observable<'_>,
    ) -> Result<Self::Events, exec::Error> {
        Ok(())
    }

    fn events_exec(&mut self, self_: &Self::Events, duration: u64) -> Result<(), exec::Error> {
        Ok(())
    }
}

fn main() -> Result<()> {
    let engine = Engine::new(&default_config()?)?;
    let path = "target/wasm32-wasi/release/demo.wasm";

    let (mut store, mut linker, instance) = wasmtime_init(&engine, path)?;
    let (mut store2, mut linker2, instance2) = wasmtime_init(&engine, path)?;

    let handler = EventHandler::new(&mut store2, &instance2, |cx: &mut GuestContext| {
        &mut cx.host.data
    })?;
    store.data_mut().host = Exec {
        guest_handler: Some(Arc::new(Mutex::new(handler))),
        guest_store: Some(Arc::new(Mutex::new(store2))),
        observables: vec![],
    };
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

pub fn wasmtime_init<T: exec::Exec + Default>(
    engine: &Engine,
    path: &str,
) -> Result<(Store<Context<T>>, Linker<Context<T>>, Instance)>
where
{
    let ctx = Context::default();
    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ctx);
    let module = Module::from_file(&engine, path)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context<T>| &mut cx.wasi)?;
    exec::add_to_linker(&mut linker, |cx: &mut Context<T>| {
        (&mut cx.host, &mut cx.host_tables)
    })?;
    let instance = linker.instantiate(&mut store, &module)?;
    Ok((store, linker, instance))
}

pub struct Context<T>
where
    T: exec::Exec + Default,
{
    pub wasi: WasiCtx,
    pub host: T,
    pub host_tables: exec::ExecTables<T>,
}

impl<T> Default for Context<T>
where
    T: exec::Exec + Default,
{
    fn default() -> Self {
        Self {
            wasi: default_wasi().unwrap(),
            host: Default::default(),
            host_tables: Default::default(),
        }
    }
}

pub struct MyObservable {
    pub rd: String,
    pub key: String,
}

pub type GuestContext = Context<GuestExec>;
