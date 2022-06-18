wit_bindgen_rust::import!("../exec.wit");
wit_bindgen_rust::export!("../event-handler.wit");

fn main() {
    println!("executing in the guest...");
    exec::exec();
    println!("finishing in the guest...")
}

pub struct EventHandler {}

impl event_handler::EventHandler for EventHandler {
    fn event_handler(event: String) -> String {
        let s = format!("this event {} has been handled", event);
        println!("{}", s);
        s
    }
}
