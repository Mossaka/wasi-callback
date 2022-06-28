use anyhow::Result;

wit_bindgen_rust::import!("../exec.wit");
wit_bindgen_rust::export!("../event-handler.wit");
wit_error_rs::impl_error!(exec::Error);

fn main() -> Result<()> {
    println!("executing in the guest...");
    let events = exec::Events::get()?;
    for _ in 0..2 {
        events.exec(5)?;
    }
    println!("finishing in the guest...");
    Ok(())
}

pub struct EventHandler {}

impl event_handler::EventHandler for EventHandler {
    fn event_handler(event: String) -> String {
        let s = format!("this event {} has been handled", event);
        println!("{}", s);
        s
    }
}
