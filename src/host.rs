pub mod exec {
    use wasmtime::Trap;
    #[allow(unused_imports)]
    use wit_bindgen_wasmtime::{anyhow, wasmtime};
    pub trait Exec: Sized {
        type Context: Ctx;
        fn events_get(caller: wasmtime::Caller<'_, Self::Context>) -> Result<i32, Trap>;

        fn events_exec(caller: wasmtime::Caller<'_, Self::Context>, _arg0: i32)
            -> Result<(), Trap>;

        fn drop_events(
            caller: wasmtime::Caller<'_, Self::Context>,
            handle: u32,
        ) -> Result<(), Trap>;
    }
    use crate::Ctx;

    pub struct ExecTables {
        pub(crate) events_table: wit_bindgen_wasmtime::Table<()>,
    }
    impl Default for ExecTables {
        fn default() -> Self {
            Self {
                events_table: Default::default(),
            }
        }
    }
    pub fn add_to_linker<T: Ctx + Exec>(
        linker: &mut wasmtime::Linker<T::Context>,
    ) -> anyhow::Result<()> {
        linker.func_wrap(
            "exec",
            "events::get",
            move |caller: wasmtime::Caller<'_, T::Context>| T::events_get(caller),
        )?;
        linker.func_wrap(
            "exec",
            "events::exec",
            move |mut caller: wasmtime::Caller<'_, T::Context>, _arg0: i32| {
                T::events_exec(caller, _arg0)
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
