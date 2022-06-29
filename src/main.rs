use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use events::ExecTables;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::*;

wit_bindgen_wasmtime::import!("event-handler.wit");

#[derive(Default)]
pub struct Exec {
    guest_handler: Option<Arc<Mutex<EventHandler<GuestContext>>>>,
    guest_store: Option<Arc<Mutex<Store<GuestContext>>>>,
    observables: Vec<MyObservable>,
}

impl events::Exec for Exec {
    type Context = Context;
    type Events = ();

    fn events_get(&mut self, data: &mut Self::Context) -> Result<Self::Events, events::Error> {
        Ok(())
    }

    fn events_listen(
        &mut self,
        self_: &Self::Events,
        ob: events::Observable<'_>,
    ) -> Result<Self::Events, events::Error> {
        let _ob = MyObservable {
            rd: ob.rd.to_string(),
            key: ob.key.to_string(),
        };
        self.observables.push(_ob);
        Ok(())
    }

    fn events_exec(
        &mut self,
        data: &mut Self::Context,
        self_: &Self::Events,
        duration: u64,
    ) -> Result<(), events::Error> {
        let mut thread_handles = vec![];
        for i in 0..10 {
            let mut host = data.host.as_ref().unwrap().lock().unwrap();
            let handler = host.guest_handler.as_ref().unwrap().clone();
            let store = host.guest_store.as_mut().unwrap().clone();
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
impl events::Exec for GuestExec {
    type Context = GuestContext;
    type Events = ();

    fn events_get(&mut self, data: &mut Self::Context) -> Result<Self::Events, events::Error> {
        Ok(())
    }

    fn events_listen(
        &mut self,
        self_: &Self::Events,
        ob: events::Observable<'_>,
    ) -> Result<Self::Events, events::Error> {
        Ok(())
    }

    fn events_exec(
        &mut self,
        data: &mut Self::Context,
        self_: &Self::Events,
        duration: u64,
    ) -> Result<(), events::Error> {
        Ok(())
    }
}

fn main() -> Result<()> {
    let engine = Engine::new(&default_config()?)?;
    let path = "target/wasm32-wasi/release/demo.wasm";

    let ctx = Context {
        wasi: default_wasi()?,
        host: None,
        host_tables: None,
    };
    let guest_ctx = GuestContext {
        wasi: default_wasi()?,
        host: GuestExec {
            data: EventHandlerData::default(),
        },
    };

    let (mut store, mut linker, module) = wasmtime_init(&engine, ctx, path)?;
    let (mut store2, mut linker2, module2) = wasmtime_init(&engine, guest_ctx, path)?;

    events::add_to_linker(&mut linker, |cx: &Context| {
        (
            cx.host.as_ref().unwrap().clone(),
            cx.host_tables.as_ref().unwrap().clone(),
        )
    })?;
    let instance = linker.instantiate(&mut store, &module)?;
    store.data_mut().host_tables = Some(Arc::new(Mutex::new(ExecTables::default())));

    events::add_to_linker_dummy::<GuestExec>(&mut linker2)?;
    let instance2 = linker2.instantiate(&mut store2, &module2)?;

    let handler = EventHandler::new(&mut store2, &instance2, |cx: &mut _| &mut cx.host.data)?;
    store.data_mut().host = Some(Arc::new(Mutex::new(Exec {
        guest_handler: Some(Arc::new(Mutex::new(handler))),
        guest_store: Some(Arc::new(Mutex::new(store2))),
        observables: vec![],
    })));

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

pub fn wasmtime_init<T: Ctx>(
    engine: &Engine,
    ctx: T,
    path: &str,
) -> Result<(Store<T>, Linker<T>, Module)>
where
{
    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ctx);
    let module = Module::from_file(&engine, path)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut T| cx.wasi_mut())?;

    Ok((store, linker, module))
}

pub struct Context {
    pub wasi: WasiCtx,
    pub host: Option<Arc<Mutex<Exec>>>,
    pub host_tables: Option<Arc<Mutex<ExecTables<Exec>>>>,
}

pub struct GuestContext {
    pub wasi: WasiCtx,
    pub host: GuestExec,
}

pub trait Ctx {
    fn wasi_mut(&mut self) -> &mut WasiCtx;
}

impl Ctx for Context {
    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl Ctx for GuestContext {
    fn wasi_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            wasi: default_wasi().unwrap(),
            host: Default::default(),
            host_tables: Default::default(),
        }
    }
}

impl Default for GuestContext {
    fn default() -> Self {
        Self {
            wasi: default_wasi().unwrap(),
            host: Default::default(),
        }
    }
}

pub struct MyObservable {
    pub rd: String,
    pub key: String,
}
