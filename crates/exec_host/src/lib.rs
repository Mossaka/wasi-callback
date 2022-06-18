wit_bindgen_wasmtime::export!("../../exec.wit");

pub use exec::add_to_linker;

#[derive(Default)]
pub struct ExecHost {}

impl exec::Exec for ExecHost {
    fn exec(&mut self) -> () {
        println!("executing in the host...");
    }
}
