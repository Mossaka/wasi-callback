pub mod exec {
    use wasmtime::{AsContext, AsContextMut};
    #[allow(unused_imports)]
    use wit_bindgen_wasmtime::{anyhow, wasmtime};

    use crate::Context;
    pub trait Exec: Sized {}

    pub fn add_to_linker(linker: &mut wasmtime::Linker<Context>) -> anyhow::Result<()> {
        linker.func_wrap(
            "exec",
            "exec",
            move |mut caller: wasmtime::Caller<Context>| {
                let store = caller.as_context();
                let host = store.data().host.as_ref().unwrap();
                let _res = host
                    .clone()
                    .lock()
                    .unwrap()
                    .event_handler(caller.as_context_mut(), "event-a");
                Ok(())
            },
        )?;
        Ok(())
    }
}
