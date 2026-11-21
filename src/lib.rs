#[allow(warnings)]
mod bindings;
mod chat;
mod dioxus_renderer;

use bindings::EventData;
use bindings::Guest;

struct Component;

impl Guest for Component {
    fn run() {
        chat::run();
    }

    fn on_event(event: EventData) {
        chat::on_event(&event);
    }

    fn on_fetch(
        request_id: u32,
        ok: bool,
        status: u16,
        body: Vec<u8>,
        error: Option<String>,
    ) {
        chat::on_fetch(request_id, ok, status, body, error);
    }
}

bindings::export!(Component with_types_in bindings);
