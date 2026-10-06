use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_core::IntoDynNode;
use dioxus_html as dioxus_elements;
use dioxus_signals::{Readable, Signal, Writable};
use dioxus_hooks::{use_memo, use_signal};

use std::cell::RefCell;

use crate::bindings::servo::dom::{console, document};
use crate::dioxus_renderer::ServoDomApplier;

#[derive(Clone, Debug, PartialEq)]
pub struct TodoItem {
    pub id: u32,
    pub text: String,
    pub completed: bool,
}

struct AppState {
    vdom: VirtualDom,
    applier: ServoDomApplier,
}

thread_local! {
    /// Preserves both the VirtualDom AND the ServoDomApplier across events
    static STATE: RefCell<Option<AppState>> = RefCell::new(None);
    /// Handle to the component's reactive Signal
    static TODOS_SIGNAL: RefCell<Option<Signal<Vec<TodoItem>>>> = RefCell::new(None);
}

/// Idiomatic Dioxus 0.6 Component using reactive hooks
fn TodoApp() -> Element {
    let mut todos = use_signal(|| vec![
        TodoItem { id: 1, text: "First-Class Wasm Component Model in Servo".into(), completed: true },
        TodoItem { id: 2, text: "Dioxus 0.6 with zero JS glue".into(), completed: true },
        TodoItem { id: 3, text: "Demo native reactive DOM".into(), completed: false },
    ]);

    TODOS_SIGNAL.with(|s| *s.borrow_mut() = Some(todos));

    let active_count = use_memo(move || {
        todos.read().iter().filter(|i| !i.completed).count()
    });

    let items = todos.read().clone();

    rsx! {
        div { class: "wasm-card",
            h1 { "📋 Dioxus TodoMVC App" }
            p { class: "subtitle", "Native Component Model + Dioxus 0.6 (Zero JS)" }

            // Input Form Section
            div { style: "display: flex; gap: 8px; margin-bottom: 16px;",
                input {
                    id: "new-todo-input",
                    placeholder: "What needs to be done?",
                    style: "flex: 1; padding: 10px 14px; border-radius: 8px; border: 1px solid #45475a; background: #11111b; color: #cdd6f4; font-size: 0.95rem; outline: none;",
                }
                button {
                    class: "btn btn-primary",
                    id: "btn-add-todo",
                    style: "white-space: nowrap;",
                    "➕ Add Task"
                }
                button {
                    class: "btn btn-secondary",
                    id: "btn-clear-done",
                    style: "white-space: nowrap;",
                    "🧹 Clear Done"
                }
            }

            // Memoized Status Counter
            div { style: "margin-bottom: 12px; font-size: 0.9rem; color: #89b4fa; font-weight: 500;",
                span { id: "todo-stats", "{active_count.read()} tasks remaining" }
            }

            // Declarative Task List
            div { id: "todo-list-container",
                for item in items.iter() {
                    div {
                        key: "{item.id}",
                        class: "task-card",
                        style: "display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; margin-bottom: 8px; border-radius: 8px; background: #313244;",

                        // Left: Checkbox + Text
                        div { style: "display: flex; align-items: center; gap: 10px;",
                            button {
                                id: "toggle-{item.id}",
                                style: "border: none; background: #1e1e2e; color: #a6e3a1; font-weight: bold; border-radius: 4px; padding: 4px 8px; cursor: pointer;",
                                if item.completed { "☑️" } else { "⬜" }
                            }
                            span {
                                style: if item.completed {
                                    "text-decoration: line-through; color: #6c7086;"
                                } else {
                                    "color: #cdd6f4; font-weight: 500;"
                                },
                                "{item.text}"
                            }
                        }

                        // Right: Delete button
                        button {
                            id: "del-{item.id}",
                            style: "border: none; background: #f38ba8; color: #11111b; font-weight: bold; border-radius: 6px; padding: 4px 10px; cursor: pointer;",
                            "🗑️ Delete"
                        }
                    }
                }
            }
        }
    }
}

pub fn run() {
    console::log("[Dioxus Todo]: Mounting Dioxus 0.6 VirtualDom with Hooks...");
    let container = document::get_element_by_id("todo-app-root")
        .unwrap_or_else(|_| document::get_body().expect("body"));

    let mut vdom = VirtualDom::new(TodoApp);
    let mut applier = ServoDomApplier::new(container);

    vdom.rebuild(&mut applier);

    STATE.with(|s| {
        *s.borrow_mut() = Some(AppState { vdom, applier });
    });

    console::log("[Dioxus Todo]: Mounted successfully!");
}

pub fn on_event(handler_id: &str) {
    console::log(&format!("[Dioxus Todo]: Action triggered: {}", handler_id));
    let mut updated = false;

    if handler_id == "btn-add-todo" {
        let mut task_text = String::new();
        if let Ok(input_elem) = document::get_element_by_id("new-todo-input") {
            task_text = input_elem.get_property("value").trim().to_string();
            input_elem.set_property("value", "");
        }

        if task_text.is_empty() {
            task_text = "Task: Measure Wasm performance".to_string();
        }

        TODOS_SIGNAL.with(|s| {
            if let Some(mut todos) = *s.borrow() {
                let next_id = todos.read().iter().map(|i| i.id).max().unwrap_or(0) + 1;
                todos.write().push(TodoItem { id: next_id, text: task_text, completed: false });
                updated = true;
            }
        });
    } else if handler_id == "btn-clear-done" {
        TODOS_SIGNAL.with(|s| {
            if let Some(mut todos) = *s.borrow() {
                todos.write().retain(|item| !item.completed);
                updated = true;
            }
        });
    } else if let Some(id_str) = handler_id.strip_prefix("toggle-") {
        if let Ok(id) = id_str.parse::<u32>() {
            TODOS_SIGNAL.with(|s| {
                if let Some(mut todos) = *s.borrow() {
                    if let Some(item) = todos.write().iter_mut().find(|i| i.id == id) {
                        item.completed = !item.completed;
                        updated = true;
                    }
                }
            });
        }
    } else if let Some(id_str) = handler_id.strip_prefix("del-") {
        if let Ok(id) = id_str.parse::<u32>() {
            TODOS_SIGNAL.with(|s| {
                if let Some(mut todos) = *s.borrow() {
                    todos.write().retain(|i| i.id != id);
                    updated = true;
                }
            });
        }
    }

    if updated {
        // Reuse the exact same persistent applier so ElementId mappings survive!
        STATE.with(|s| {
            if let Some(AppState { vdom, applier }) = s.borrow_mut().as_mut() {
                vdom.render_immediate(applier);
            }
        });
    }
}