use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_hooks::use_signal;
use dioxus_signals::Readable;
use std::cell::RefCell;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering::SeqCst};

use crate::bindings::servo::dom::{console, document};
use crate::dioxus_renderer::ServoDomApplier;

use dioxus_html as dioxus_elements;

static COMPLETED_TASKS: AtomicI32 = AtomicI32::new(0);
static TOTAL_SPAWNED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static VDOM: RefCell<Option<VirtualDom>> = RefCell::new(None);
}

fn TaskBoard() -> Element {
    let completed = use_signal(|| 0);

    rsx! {
        div { class: "wasm-card",
            h1 { "📋 Dioxus Reactive Task Board" }
            p { class: "subtitle",
                "Full RSX VirtualDom running on Servo via WebAssembly Component Model"
            }

            // Overview stats
            div { class: "counter-section",
                div { id: "completed-count", class: "counter-badge", "{completed}" }
                button { class: "btn btn-primary", id: "btn-complete-task", "✅ Complete Task" }
                button { class: "btn btn-secondary", id: "btn-reset-tasks", "🔄 Reset" }
            }

            // Task Spawner
            div { class: "tasks-section",
                button { class: "btn btn-accent", id: "btn-spawn-task", "🚀 Spawn Wasm Worker Task" }
                div { id: "task-container" }
            }

            // Event Audit Log
            div { class: "log-section",
                h3 { "📝 Component Event Audit Log" }
                ul { id: "event-log" }
            }
        }
    }
}

pub fn run() {
    console::log("[Dioxus]: Initializing Task Board VirtualDom...");
    let body = document::get_body().expect("body");

    let mut vdom = VirtualDom::new(TaskBoard);
    let mut applier = ServoDomApplier::new(body);

    vdom.rebuild(&mut applier);

    if let Ok(elem) = document::get_element_by_id("completed-count") {
        elem.set_text_content("0");
    }

    VDOM.with(|v| {
        *v.borrow_mut() = Some(vdom);
    });

    console::log("[Dioxus]: Task Board mounted successfully!");
}

pub fn on_event(handler_id: &str) {
    match handler_id {
        "btn-complete-task" => {
            let val = COMPLETED_TASKS.fetch_add(1, SeqCst) + 1;
            if let Ok(elem) = document::get_element_by_id("completed-count") {
                elem.set_text_content(&val.to_string());
            }
            log_event(&format!("Completed task item (Total: {})", val));
        }
        "btn-reset-tasks" => {
            COMPLETED_TASKS.store(0, SeqCst);
            if let Ok(elem) = document::get_element_by_id("completed-count") {
                elem.set_text_content("0");
            }
            log_event("Reset completed task counter");
        }
        "btn-spawn-task" => {
            let id = TOTAL_SPAWNED.fetch_add(1, SeqCst) + 1;
            if let Ok(container) = document::get_element_by_id("task-container") {
                if let Ok(card) = document::create_element("div") {
                    card.set_attribute("class", "task-card");
                    card.set_text_content(&format!("⚡ Worker Task #{}: Spawned by Dioxus VirtualDom in Wasm", id));
                    container.append_child(&card);
                }
            }
            log_event(&format!("Spawned dynamic task card #{}", id));
        }
        _ => {}
    }
}

fn log_event(msg: &str) {
    if let Ok(list) = document::get_element_by_id("event-log") {
        if let Ok(li) = document::create_element("li") {
            li.set_attribute("class", "log-item");
            li.set_text_content(msg);
            list.append_child(&li);
        }
    }
}
