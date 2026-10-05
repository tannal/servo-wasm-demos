#[allow(warnings)]
mod bindings;
use bindings::Guest;

pub mod dioxus_renderer;

#[path = "../examples/raw_reactive_dashboard.rs"]
pub mod raw_reactive_dashboard;

#[path = "../examples/dioxus_task_board.rs"]
pub mod dioxus_task_board;

#[path = "../examples/dioxus_mathml_playground.rs"]
pub mod dioxus_mathml_playground;

#[path = "../examples/dioxus_todo.rs"]
pub mod dioxus_todo;

#[path = "../examples/zulip_feed.rs"]
pub mod zulip_feed;

struct Component;

impl Guest for Component {
    fn run() {
        // --- CHOOSE YOUR DEMO HERE ---
        // dioxus_task_board::run();
        // raw_reactive_dashboard::run();
        // dioxus_mathml_playground::run();
        dioxus_todo::run();

        // zulip_feed::run();
    }

    fn on_event(handler_id: String, _event_type: String) {
        // dioxus_task_board::on_event(&handler_id);
        // raw_reactive_dashboard::on_event(&handler_id);
        // dioxus_mathml_playground::on_event(&handler_id);
        dioxus_todo::on_event(&handler_id);
        // zulip_feed::on_event(&handler_id);
    }
}

bindings::export!(Component with_types_in bindings);
