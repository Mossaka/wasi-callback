use std::{cell::RefCell, rc::Rc};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::*;

wit_bindgen_wasmtime::import!("event-handler.wit");
wit_bindgen_wasmtime::export!("exec.wit");

pub struct ExecHost {
    store: Rc<RefCell<Store<Context>>>,
    handler: EventHandler<Context>,
}

impl ExecHost {
    pub fn new(store: Rc<RefCell<Store<Context>>>, handler: EventHandler<Context>) -> Self {
        Self { store, handler }
    }
}

impl exec::Exec for ExecHost {
    fn exec(&mut self) -> () {
        println!("executing in the host...");
        unsafe {
            let store = &mut (*(*self.store).as_ptr());
            self.handler.event_handler(store, "event-a");
        }
    }
}

pub struct Context {
    pub wasi: WasiCtx,
    pub guest: EventHandlerData,
    pub host: Option<ExecHost>,
}

fn main() -> Result<()> {
    let wasi = default_wasi()?;
    let guest = EventHandlerData::default();
    let host = None;
    let ctx = Context { wasi, guest, host };

    let engine = Engine::new(&default_config()?)?;
    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context| &mut cx.wasi)?;
    exec::add_to_linker(&mut linker, |cx: &mut Context| cx.host.as_mut().unwrap())?;

    let module = "./target/wasm32-wasi/release/demo.wasm";
    let module = Module::from_file(&engine, module)?;
    let instance = linker.instantiate(&mut store, &module)?;

    let handler = EventHandler::new(&mut store, &instance, |cx: &mut Context| &mut cx.guest)?;
    let store_rc = Rc::new(RefCell::new(store));
    let host = ExecHost::new(store_rc.clone(), handler);
    store_rc.borrow_mut().data_mut().host = Some(host);
    unsafe {
        let mut store = Rc::into_raw(store_rc);
        instance
            .get_typed_func::<(i32, i32), i32, _>(&mut (*(*store).as_ptr()), "main")?
            .call(&mut (*(*store).as_ptr()), (0, 0))?;
    }
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
