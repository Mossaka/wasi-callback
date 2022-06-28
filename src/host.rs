pub mod exec {
    use wasmtime::Trap;
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
        type Context: Ctx;
        fn events_get(caller: wasmtime::Caller<'_, Self::Context>, _arg0: i32) -> Result<(), Trap>;

        fn events_listen(
            caller: wasmtime::Caller<'_, Self::Context>,
            arg0: i32,
            arg1: i32,
            arg2: i32,
            arg3: i32,
            arg4: i32,
            arg5: i32,
        ) -> Result<(), Trap>;

        fn events_exec(
            caller: wasmtime::Caller<'_, Self::Context>,
            arg0: i32,
            arg1: i64,
            arg2: i32,
        ) -> Result<(), Trap>;

        fn drop_events(
            caller: wasmtime::Caller<'_, Self::Context>,
            handle: u32,
        ) -> Result<(), Trap>;
    }
    use crate::Ctx;

    #[derive(Default)]
    pub struct ExecTables {
        pub(crate) events_table: wit_bindgen_wasmtime::Table<()>,
    }

    pub fn add_to_linker<T: Ctx + Exec>(
        linker: &mut wasmtime::Linker<T::Context>,
    ) -> anyhow::Result<()> {
        linker.func_wrap(
            "exec",
            "events::get",
            move |caller: wasmtime::Caller<'_, T::Context>, arg0: i32| {
                T::events_get(caller, arg0)
            },
        )?;
        linker.func_wrap(
            "exec",
            "events::listen",
            move |caller: wasmtime::Caller<'_, T::Context>,
                  arg0: i32,
                  arg1: i32,
                  arg2: i32,
                  arg3: i32,
                  arg4: i32,
                  arg5: i32| {
                T::events_listen(caller, arg0, arg1, arg2, arg3, arg4, arg5)
            },
        )?;
        linker.func_wrap(
            "exec",
            "events::exec",
            move |caller: wasmtime::Caller<'_, T::Context>, arg0: i32, arg1: i64, arg2: i32| {
                T::events_exec(caller, arg0, arg1, arg2)
            },
        )?;
        linker.func_wrap(
            "canonical_abi",
            "resource_drop_events",
            move |caller: wasmtime::Caller<'_, T::Context>, handle: u32| {
                T::drop_events(caller, handle)
            },
        )?;
        Ok(())
    }
}
