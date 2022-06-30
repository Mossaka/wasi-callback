use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use event_handler::{EventHandler, EventHandlerData};
use events::{self, Exec, ExecTables};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_common::{StringArrayError, WasiCtx};
use wasmtime::{AsContext, AsContextMut, Config, Engine, Linker, Module, Store};
use wasmtime_wasi::*;
use wit_bindgen_wasmtime::rt::{get_func, get_memory, RawMem};

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
    let path = "target/wasm32-wasi/release/demo.wasm";
    let (mut store, instance) = wasmtime_init(&engine, ctx, path)?;
    let (mut store2, instance2) = wasmtime_init(&engine, ctx2, path)?;

    // put dummy implementation to these import functions
    let handler = EventHandler::new(&mut store2, &instance2, |cx: &mut GuestContext| {
        &mut cx.guest
    })?;
    store.data_mut().host = (
        Some(Arc::new(Mutex::new(handler))),
        Some(ExecTables::default()),
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
) -> Result<(Store<T>, wasmtime::Instance)>
where
    T: Ctx + Exec<Context = T>,
{
    let mut linker = Linker::new(engine);
    let mut store = Store::new(engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut T| cx.wasi_mut())?;
    let module = Module::from_file(engine, path)?;
    events::add_to_linker::<T>(&mut linker)?;
    let instance = linker.instantiate(&mut store, &module)?;
    Ok((store, instance))
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

    fn events_get(
        mut caller: wasmtime::Caller<'_, Self::Context>,
        arg0: i32,
    ) -> Result<(), wasmtime::Trap> {
        let func = get_func(&mut caller, "canonical_abi_realloc")?;
        let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
        let memory = &get_memory(&mut caller, "memory")?;
        let _tables = caller.data_mut().host.1.as_mut().unwrap();
        let result = Ok(());
        match result {
            Ok(e) => {
                let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                let _tables = data.host.1.as_mut().unwrap();
                caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                caller_memory.store(
                    arg0 + 4,
                    wit_bindgen_wasmtime::rt::as_i32(_tables.events_table.insert(e) as i32),
                )?;
            }
            Err(e) => {
                let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                let _tables = data.host.1.as_mut().unwrap();
                caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(1i32) as u8)?;
                match e {
                    events::Error::ErrorWithDescription(e) => {
                        caller_memory
                            .store(arg0 + 4, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                        let vec0 = e;
                        let ptr0 = func_canonical_abi_realloc
                            .call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
                        let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                        let _tables = data.host.1.as_mut().unwrap();
                        caller_memory.store_many(ptr0, vec0.as_bytes())?;
                        caller_memory.store(
                            arg0 + 12,
                            wit_bindgen_wasmtime::rt::as_i32(vec0.len() as i32),
                        )?;
                        caller_memory.store(arg0 + 8, wit_bindgen_wasmtime::rt::as_i32(ptr0))?;
                    }
                };
            }
        };
        Ok(())
    }

    fn events_listen(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
        _arg1: i32,
        _arg2: i32,
        _arg3: i32,
        _arg4: i32,
        _arg5: i32,
    ) -> Result<(), wasmtime::Trap> {
        todo!()
    }

    fn events_exec(
        mut caller: wasmtime::Caller<'_, Self::Context>,
        arg0: i32,
        arg1: i64,
        arg2: i32,
    ) -> Result<(), wasmtime::Trap> {
        let func = get_func(&mut caller, "canonical_abi_realloc")?;
        let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
        let memory = &get_memory(&mut caller, "memory")?;
        let _tables = caller.data_mut().host.1.as_mut().unwrap();
        let param0 = _tables
            .events_table
            .get((arg0) as u32)
            .ok_or_else(|| wasmtime::Trap::new("invalid handle index"))?;
        let param1 = arg1 as u64;
        let mut thread_handles = vec![];
        for i in 0..10 {
            let handler = caller.data_mut().host.0.as_ref().unwrap().clone();
            let store = caller.data_mut().host.2.as_mut().unwrap().clone();
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
        let result = Ok(());
        match result {
            Ok(e) => {
                let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                let _tables = data.host.1.as_mut().unwrap();
                caller_memory.store(arg2 + 0, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                let () = e;
            }
            Err(e) => {
                let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                let _tables = data.host.1.as_mut().unwrap();
                caller_memory.store(arg2 + 0, wit_bindgen_wasmtime::rt::as_i32(1i32) as u8)?;
                match e {
                    events::Error::ErrorWithDescription(e) => {
                        caller_memory
                            .store(arg2 + 4, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                        let vec0 = e;
                        let ptr0 = func_canonical_abi_realloc
                            .call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
                        let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                        let _tables = data.host.1.as_mut().unwrap();
                        caller_memory.store_many(ptr0, vec0.as_bytes())?;
                        caller_memory.store(
                            arg2 + 12,
                            wit_bindgen_wasmtime::rt::as_i32(vec0.len() as i32),
                        )?;
                        caller_memory.store(arg2 + 8, wit_bindgen_wasmtime::rt::as_i32(ptr0))?;
                    }
                };
            }
        };
        Ok(())
    }

    fn drop_events(
        mut caller: wasmtime::Caller<'_, Self::Context>,
        handle: u32,
    ) -> Result<(), wasmtime::Trap> {
        let _tables = caller.data_mut().host.1.as_mut().unwrap();
        _tables
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

    fn events_listen(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
        _arg1: i32,
        _arg2: i32,
        _arg3: i32,
        _arg4: i32,
        _arg5: i32,
    ) -> Result<(), wasmtime::Trap> {
        Ok(())
    }

    fn events_get(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
    ) -> Result<(), wasmtime::Trap> {
        Ok(())
    }

    fn events_exec(
        _caller: wasmtime::Caller<'_, Self::Context>,
        _arg0: i32,
        _arg1: i64,
        _arg2: i32,
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

type GuestStore = Store<GuestContext>;
type HostData = (
    Option<Arc<Mutex<EventHandler<GuestContext>>>>,
    Option<ExecTables>,
    Option<Arc<Mutex<GuestStore>>>,
);
