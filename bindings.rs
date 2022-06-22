pub mod exec {
  #[allow(unused_imports)]
  use wit_bindgen_wasmtime::{wasmtime, anyhow};
  pub trait Exec: Sized {
    fn exec(&mut self,) -> ();
    
  }
  
  pub fn add_to_linker<T, U>(linker: &mut wasmtime::Linker<T>, get: impl Fn(&mut T) -> &mut U+ Send + Sync + Copy + 'static) -> anyhow::Result<()> 
  where U: Exec
  {
    linker.func_wrap("exec", "exec", move |mut caller: wasmtime::Caller<'_, T>| {
      let host = get(caller.data_mut());
      let result = host.exec();
      let () = result;
      Ok(())
    })?;
    Ok(())
  }
}
