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

// Represents an in-memory task item
#[derive(Clone, Debug)]
struct TodoItem {
    id: u32,
    text: String,
    completed: bool,
}

thread_local! {
    static VDOM: RefCell<Option<VirtualDom>> = RefCell::new(None);
    static TODOS: RefCell<Vec<TodoItem>> = RefCell::new(Vec::new());
}

static NEXT_TODO_ID: AtomicU32 = AtomicU32::new(4);

// Re-renders the entire todo list to the DOM based on the TODOS vector
fn render_todos_to_dom() {
    if let Ok(container) = document::get_element_by_id("todo-list-container") {
        container.set_text_content("");

        TODOS.with(|todos| {
            let list = todos.borrow();
            let mut active_count = 0;

            for item in list.iter() {
                if !item.completed {
                    active_count += 1;
                }

                if let Ok(row) = document::create_element("div") {
                    row.set_attribute("class", "task-card");
                    row.set_attribute(
                        "style",
                        "display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; margin-bottom: 8px; border-radius: 8px; background: #313244;"
                    );

                    // Left Side: Checkbox + Text
                    let left = document::create_element("div").expect("div");
                    left.set_attribute("style", "display: flex; align-items: center; gap: 10px;");

                    // Checkbox toggle button
                    let check_btn = document::create_element("button").expect("button");
                    let check_id = format!("toggle-{}", item.id);
                    check_btn.set_attribute("id", &check_id);
                    check_btn.set_attribute(
                        "style",
                        "border: none; background: #1e1e2e; color: #a6e3a1; font-weight: bold; border-radius: 4px; padding: 4px 8px; cursor: pointer;"
                    );
                    check_btn.set_text_content(if item.completed { "☑️" } else { "⬜" });
                    check_btn.add_event_listener("click", &check_id);
                    left.append_child(&check_btn);

                    // Task text with strike-through if completed
                    let text_span = document::create_element("span").expect("span");
                    if item.completed {
                        text_span.set_attribute("style", "text-decoration: line-through; color: #6c7086;");
                    } else {
                        text_span.set_attribute("style", "color: #cdd6f4; font-weight: 500;");
                    }
                    text_span.set_text_content(&item.text);
                    left.append_child(&text_span);

                    row.append_child(&left);

                    // Right Side: Delete Button
                    let del_btn = document::create_element("button").expect("button");
                    let del_id = format!("del-{}", item.id);
                    del_btn.set_attribute("id", &del_id);
                    del_btn.set_attribute(
                        "style",
                        "border: none; background: #f38ba8; color: #11111b; font-weight: bold; border-radius: 6px; padding: 4px 10px; cursor: pointer;"
                    );
                    del_btn.set_text_content("🗑️ Delete");
                    del_btn.add_event_listener("click", &del_id);
                    row.append_child(&del_btn);

                    container.append_child(&row);
                }
            }

            // Update stats badge
            if let Ok(badge) = document::get_element_by_id("todo-stats") {
                badge.set_text_content(&format!("{} tasks remaining", active_count));
            }
        });
    }
}

fn TodoApp() -> Element {
    rsx! {
        div { class: "wasm-card",
            h1 { "📋 Dioxus TodoMVC App" }
            p { class: "subtitle", "Native Component Model + Dioxus VirtualDom (Zero JS)" }

            // Action Header & Form
            div { class: "counter-section",
                div { id: "todo-stats", class: "counter-badge", style: "font-size: 1rem; color: #89b4fa;", "3 tasks remaining" }
                button { class: "btn btn-primary", id: "btn-add-todo", "➕ Add Task" }
                button { class: "btn btn-secondary", id: "btn-clear-done", "🧹 Clear Completed" }
            }

            // Task List Container
            div { id: "todo-list-container" }
        }
    }
}

pub fn run() {
    console::log("[Dioxus Todo]: Mounting TodoMVC VirtualDom...");
    // let body = document::get_body().expect("body");
    let container = document::get_element_by_id("todo-app-root")
    .unwrap_or_else(|_| document::get_body().expect("Reason"));

    // Clear previous contents
    // body.set_text_content("");

    // Initialize default items
    TODOS.with(|todos| {
        let mut list = todos.borrow_mut();
        list.clear();
        list.push(TodoItem { id: 1, text: "Implement First-Class Wasm in Servo".to_string(), completed: true });
        list.push(TodoItem { id: 2, text: "Eliminate wasm-bindgen and JS glue".to_string(), completed: true });
        list.push(TodoItem { id: 3, text: "Demo Dioxus TodoMVC to Fred and Igalia".to_string(), completed: false });
    });

    let mut vdom = VirtualDom::new(TodoApp);
    let mut applier = ServoDomApplier::new(container);

    vdom.rebuild(&mut applier);

    // Initial render of tasks
    render_todos_to_dom();

    VDOM.with(|v| {
        *v.borrow_mut() = Some(vdom);
    });

    console::log("[Dioxus Todo]: Mounted successfully!");
}

pub fn on_event(handler_id: &str) {
    console::log(&format!("[Dioxus Todo]: Action triggered: {}", handler_id));

    if handler_id == "btn-add-todo" {
        let id = NEXT_TODO_ID.fetch_add(1, SeqCst);
        let task_names = [
            "Test MathML Core layout in Servo",
            "Present W3C Wasm CG Issue #371 slides",
            "Benchmark Wasm Component vs JS DOM latency",
            "Write RFC for WHATWG script type=wasm-component",
        ];
        let name = task_names[(id as usize) % task_names.len()];

        TODOS.with(|todos| {
            todos.borrow_mut().push(TodoItem {
                id,
                text: format!("{}: {}", id, name),
                completed: false,
            });
        });
        render_todos_to_dom();
    } else if handler_id == "btn-clear-done" {
        TODOS.with(|todos| {
            todos.borrow_mut().retain(|item| !item.completed);
        });
        render_todos_to_dom();
    } else if let Some(id_str) = handler_id.strip_prefix("toggle-") {
        if let Ok(id) = id_str.parse::<u32>() {
            TODOS.with(|todos| {
                if let Some(item) = todos.borrow_mut().iter_mut().find(|i| i.id == id) {
                    item.completed = !item.completed;
                }
            });
            render_todos_to_dom();
        }
    } else if let Some(id_str) = handler_id.strip_prefix("del-") {
        if let Ok(id) = id_str.parse::<u32>() {
            TODOS.with(|todos| {
                todos.borrow_mut().retain(|i| i.id != id);
            });
            render_todos_to_dom();
        }
    }
}
