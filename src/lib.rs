#[allow(warnings)]
mod bindings;
use bindings::Guest;

pub mod dioxus_renderer;
#[path = "../examples/dioxus_todo.rs"]
pub mod dioxus_todo;

struct Component;

impl Guest for Component {
    fn run() {
        dioxus_todo::run();
    }

    fn on_event(handler_id: String, _event_type: String) {
        dioxus_todo::on_event(&handler_id);
    }
}

bindings::export!(Component with_types_in bindings);
