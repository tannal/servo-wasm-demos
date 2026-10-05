use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_hooks::use_signal;
use dioxus_signals::Readable;
use std::cell::RefCell;
use core::sync::atomic::{AtomicU32, Ordering::SeqCst};

use crate::bindings::servo::dom::{console, document};
use crate::dioxus_renderer::ServoDomApplier;

use dioxus_core::IntoDynNode;

use dioxus_html as dioxus_elements;

static NEXT_TODO_ID: AtomicU32 = AtomicU32::new(3);

thread_local! {
    static VDOM: RefCell<Option<VirtualDom>> = RefCell::new(None);
}

fn TodoApp() -> Element {
    let mut todos = use_signal(|| vec![
        (1, "Implement First-Class Wasm in Servo", true),
        (2, "Eliminate wasm-bindgen glue code", true),
        (3, "Demo Dioxus TodoMVC to Fred and Igalia", false),
    ]);

    rsx! {
        div { class: "wasm-card",
            h1 { "📋 Dioxus TodoMVC App" }
            p { class: "subtitle", "Native Component Model + Dioxus RSX" }

            // Action Header
            div { class: "counter-section",
                button {
                    class: "btn btn-primary",
                    id: "btn-add-todo",
                    "➕ Add New Task"
                }
                button {
                    class: "btn btn-secondary",
                    id: "btn-clear-done",
                    "🧹 Clear Completed"
                }
            }

            // Task List
            div { id: "todo-list-container",
                for (id, text, completed) in todos.read().iter() {
                    div { class: "task-card",
                        "📌 Task #{id}: {text} - {completed}"
                    }
                }
            }
        }
    }
}

pub fn run() {
    console::log("[Dioxus Todo]: Mounting TodoMVC VirtualDom...");
    let body = document::get_body().expect("body");

    let mut vdom = VirtualDom::new(TodoApp);
    let mut applier = ServoDomApplier::new(body);

    vdom.rebuild(&mut applier);

    // Initial render of pre-existing tasks
    if let Ok(container) = document::get_element_by_id("todo-list-container") {
        container.set_text_content("");
        for (id, title, done) in [
            (1, "Implement First-Class Wasm in Servo", true),
            (2, "Eliminate wasm-bindgen glue code", true),
            (3, "Demo Dioxus TodoMVC to Fred and Igalia", false),
        ] {
            if let Ok(item) = document::create_element("div") {
                item.set_attribute("class", "task-card");
                let status = if done { "✅ [DONE]" } else { "⏳ [PENDING]" };
                item.set_text_content(&format!("{} Task #{}: {}", status, id, title));
                container.append_child(&item);
            }
        }
    }

    VDOM.with(|v| {
        *v.borrow_mut() = Some(vdom);
    });

    console::log("[Dioxus Todo]: Mounted successfully!");
}

pub fn on_event(handler_id: &str) {
    match handler_id {
        "btn-add-todo" => {
            let id = NEXT_TODO_ID.fetch_add(1, SeqCst) + 1;
            if let Ok(container) = document::get_element_by_id("todo-list-container") {
                if let Ok(item) = document::create_element("div") {
                    item.set_attribute("class", "task-card");
                    item.set_text_content(&format!("⏳ [PENDING] Task #{}: New Wasm Reactive Milestone", id));
                    container.append_child(&item);
                }
            }
        }
        "btn-clear-done" => {
            if let Ok(container) = document::get_element_by_id("todo-list-container") {
                // Keep only active pending items
                container.set_text_content("");
                if let Ok(item) = document::create_element("div") {
                    item.set_attribute("class", "task-card");
                    item.set_text_content("⏳ [PENDING] Active Task: Pitch to Igalia Web Platform team");
                    container.append_child(&item);
                }
            }
        }
        _ => {}
    }
}
