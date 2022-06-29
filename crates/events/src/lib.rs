use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use wasmtime::{AsContext, AsContextMut, Trap};
use wit_bindgen_wasmtime::rt::{get_func, get_memory, RawMem};
#[allow(unused_imports)]
use wit_bindgen_wasmtime::{anyhow, wasmtime};
#[derive(Clone)]
pub struct Observable<'a> {
    pub rd: &'a str,
    pub key: &'a str,
}
impl<'a> std::fmt::Debug for Observable<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observable")
            .field("rd", &self.rd)
            .field("key", &self.key)
            .finish()
    }
}
#[derive(Clone)]
pub enum Error {
    ErrorWithDescription(String),
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorWithDescription(e) => f
                .debug_tuple("Error::ErrorWithDescription")
                .field(e)
                .finish(),
        }
    }
}
pub trait Exec: Sized {
    type Context: Default;
    type Events: Default;
    fn events_get(&mut self, data: &mut Self::Context) -> Result<Self::Events, Error>;

    fn events_listen(
        &mut self,
        self_: &Self::Events,
        ob: Observable<'_>,
    ) -> Result<Self::Events, Error>;

    fn events_exec(
        &mut self,
        data: &mut Self::Context,
        self_: &Self::Events,
        duration: u64,
    ) -> Result<(), Error>;

    fn drop_events(&mut self, state: Self::Events) {
        drop(state);
    }
}

#[derive(Default)]
pub struct ExecTables<T: Exec> {
    pub(crate) events_table: wit_bindgen_wasmtime::Table<T::Events>,
}
pub fn add_to_linker<T: Exec>(
    linker: &mut wasmtime::Linker<T::Context>,
    get: impl Fn(&T::Context) -> (Arc<Mutex<T>>, Arc<Mutex<ExecTables<T>>>)
        + Send
        + Sync
        + Copy
        + 'static,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "exec",
        "events::get",
        move |mut caller: wasmtime::Caller<'_, T::Context>, arg0: i32| {
            let func = get_func(&mut caller, "canonical_abi_realloc")?;
            let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
            let memory = &get_memory(&mut caller, "memory")?;
            let host = get(caller.data());
            let (host, _tables) = host;
            let result = host.clone().lock().unwrap().events_get(caller.data_mut());
            match result {
                Ok(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                    caller_memory.store(
                        arg0 + 4,
                        wit_bindgen_wasmtime::rt::as_i32(
                            _tables.lock().unwrap().events_table.insert(e) as i32,
                        ),
                    )?;
                }
                Err(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg0 + 0, wit_bindgen_wasmtime::rt::as_i32(1i32) as u8)?;
                    match e {
                        Error::ErrorWithDescription(e) => {
                            caller_memory
                                .store(arg0 + 4, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                            let vec0 = e;
                            let ptr0 = func_canonical_abi_realloc
                                .call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
                            let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                            let (_, _tables) = get(data);
                            caller_memory.store_many(ptr0, vec0.as_bytes())?;
                            caller_memory.store(
                                arg0 + 12,
                                wit_bindgen_wasmtime::rt::as_i32(vec0.len() as i32),
                            )?;
                            caller_memory
                                .store(arg0 + 8, wit_bindgen_wasmtime::rt::as_i32(ptr0))?;
                        }
                    };
                }
            };
            Ok(())
        },
    )?;
    linker.func_wrap(
        "exec",
        "events::listen",
        move |mut caller: wasmtime::Caller<'_, T::Context>,
              arg0: i32,
              arg1: i32,
              arg2: i32,
              arg3: i32,
              arg4: i32,
              arg5: i32| {
            let func = get_func(&mut caller, "canonical_abi_realloc")?;
            let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
            let memory = &get_memory(&mut caller, "memory")?;
            let (mem, data) = memory.data_and_store_mut(&mut caller);
            let mut _bc = wit_bindgen_wasmtime::BorrowChecker::new(mem);
            let host = get(data);
            let (host, _tables) = host;
            let ptr0 = arg1;
            let len0 = arg2;
            let ptr1 = arg3;
            let len1 = arg4;
            let param0 = _tables.lock().unwrap();
            let param0 = param0
                .events_table
                .get((arg0) as u32)
                .ok_or_else(|| wasmtime::Trap::new("invalid handle index"))?;
            let param1 = Observable {
                rd: _bc.slice_str(ptr0, len0)?,
                key: _bc.slice_str(ptr1, len1)?,
            };
            let result = host.lock().unwrap().events_listen(param0, param1);
            match result {
                Ok(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg5 + 0, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                    caller_memory.store(
                        arg5 + 4,
                        wit_bindgen_wasmtime::rt::as_i32(
                            _tables.lock().unwrap().events_table.insert(e) as i32,
                        ),
                    )?;
                }
                Err(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg5 + 0, wit_bindgen_wasmtime::rt::as_i32(1i32) as u8)?;
                    match e {
                        Error::ErrorWithDescription(e) => {
                            caller_memory
                                .store(arg5 + 4, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                            let vec2 = e;
                            let ptr2 = func_canonical_abi_realloc
                                .call(&mut caller, (0, 0, 1, vec2.len() as i32))?;
                            let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                            let (_, _tables) = get(data);
                            caller_memory.store_many(ptr2, vec2.as_bytes())?;
                            caller_memory.store(
                                arg5 + 12,
                                wit_bindgen_wasmtime::rt::as_i32(vec2.len() as i32),
                            )?;
                            caller_memory
                                .store(arg5 + 8, wit_bindgen_wasmtime::rt::as_i32(ptr2))?;
                        }
                    };
                }
            };
            Ok(())
        },
    )?;
    linker.func_wrap(
        "exec",
        "events::exec",
        move |mut caller: wasmtime::Caller<'_, T::Context>, arg0: i32, arg1: i64, arg2: i32| {
            let func = get_func(&mut caller, "canonical_abi_realloc")?;
            let func_canonical_abi_realloc = func.typed::<(i32, i32, i32, i32), i32, _>(&caller)?;
            let memory = &get_memory(&mut caller, "memory")?;
            let host = get(caller.data_mut());
            let (host, _tables) = host;
            let param0 = _tables.lock().unwrap();
            let param0 = param0
                .events_table
                .get((arg0) as u32)
                .ok_or_else(|| wasmtime::Trap::new("invalid handle index"))?;
            let param1 = arg1 as u64;
            let result =
                host.clone()
                    .lock()
                    .unwrap()
                    .events_exec(caller.data_mut(), param0, param1);
            match result {
                Ok(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg2 + 0, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                    let () = e;
                }
                Err(e) => {
                    let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                    let (_, _tables) = get(data);
                    caller_memory.store(arg2 + 0, wit_bindgen_wasmtime::rt::as_i32(1i32) as u8)?;
                    match e {
                        Error::ErrorWithDescription(e) => {
                            caller_memory
                                .store(arg2 + 4, wit_bindgen_wasmtime::rt::as_i32(0i32) as u8)?;
                            let vec0 = e;
                            let ptr0 = func_canonical_abi_realloc
                                .call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
                            let (caller_memory, data) = memory.data_and_store_mut(&mut caller);
                            let (_, _tables) = get(data);
                            caller_memory.store_many(ptr0, vec0.as_bytes())?;
                            caller_memory.store(
                                arg2 + 12,
                                wit_bindgen_wasmtime::rt::as_i32(vec0.len() as i32),
                            )?;
                            caller_memory
                                .store(arg2 + 8, wit_bindgen_wasmtime::rt::as_i32(ptr0))?;
                        }
                    };
                }
            };
            Ok(())
        },
    )?;
    linker.func_wrap(
        "canonical_abi",
        "resource_drop_events",
        move |mut caller: wasmtime::Caller<'_, T::Context>, handle: u32| {
            let (host, tables) = get(caller.data());
            let handle = tables
                .lock()
                .unwrap()
                .events_table
                .remove(handle)
                .map_err(|e| wasmtime::Trap::new(format!("failed to remove handle: {}", e)))?;
            host.clone().lock().unwrap().drop_events(handle);
            Ok(())
        },
    )?;
    Ok(())
}

pub fn add_to_linker_dummy<T: Exec>(
    linker: &mut wasmtime::Linker<T::Context>,
) -> anyhow::Result<()> {
    linker.func_wrap(
        "exec",
        "events::get",
        move |mut caller: wasmtime::Caller<'_, T::Context>, arg0: i32| Ok(()),
    )?;
    linker.func_wrap(
        "exec",
        "events::listen",
        move |mut caller: wasmtime::Caller<'_, T::Context>,
              arg0: i32,
              arg1: i32,
              arg2: i32,
              arg3: i32,
              arg4: i32,
              arg5: i32| { Ok(()) },
    )?;
    linker.func_wrap(
        "exec",
        "events::exec",
        move |mut caller: wasmtime::Caller<'_, T::Context>, arg0: i32, arg1: i64, arg2: i32| Ok(()),
    )?;
    linker.func_wrap(
        "canonical_abi",
        "resource_drop_events",
        move |mut caller: wasmtime::Caller<'_, T::Context>, handle: u32| Ok(()),
    )?;
    Ok(())
}
