pub mod exec {
    use wasmtime::{AsContext, AsContextMut};
    #[allow(unused_imports)]
    use wit_bindgen_wasmtime::{anyhow, wasmtime};

    use crate::Context;

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
    pub fn add_to_linker(linker: &mut wasmtime::Linker<Context>) -> anyhow::Result<()> {
        linker.func_wrap(
            "exec",
            "events::get",
            move |mut caller: wasmtime::Caller<'_, Context>| {
                let store = caller.as_context();
                let _tables = store.data().host.1.as_ref().unwrap();
                let handle = _tables.clone().lock().unwrap().events_table.insert(());
                Ok(handle as i32)
            },
        )?;
        linker.func_wrap(
            "exec",
            "events::exec",
            move |mut caller: wasmtime::Caller<'_, Context>, arg0: i32| {
                let store = caller.as_context();
                let handler = store.data().host.0.as_ref().unwrap();
                let _res = handler
                    .clone()
                    .lock()
                    .unwrap()
                    .event_handler(caller.as_context_mut(), "event-a");
                Ok(())
            },
        )?;
        linker.func_wrap(
            "canonical_abi",
            "resource_drop_events",
            move |mut caller: wasmtime::Caller<'_, Context>, handle: u32| {
                let store = caller.as_context();
                let _tables = store.data().host.1.as_ref().unwrap();
                let handle = _tables
                    .clone()
                    .lock()
                    .unwrap()
                    .events_table
                    .remove(handle)
                    .map_err(|e| wasmtime::Trap::new(format!("failed to remove handle: {}", e)))?;
                drop(handle);
                Ok(())
            },
        )?;
        Ok(())
    }
}