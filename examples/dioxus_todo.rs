use dioxus_core::{Element, ElementId, Event, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_core::IntoDynNode;
use dioxus_html::point_interaction::InteractionLocation;
use dioxus_html::{self as dioxus_elements, PlatformEventData};
use dioxus_signals::{Readable, Signal, Writable};
use dioxus_hooks::{use_memo, use_signal};

use std::cell::RefCell;
use std::rc::Rc;

use crate::bindings::servo::dom::{console, document};
use crate::dioxus_renderer::{ServoDomApplier, ServoMouseData};
use dioxus_html::point_interaction::PointerInteraction;

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
    /// Preserves both the VirtualDom and the ServoDomApplier across events
    static STATE: RefCell<Option<AppState>> = RefCell::new(None);
}

/// Idiomatic Dioxus 0.6 Component with local closures
fn TodoApp() -> Element {
    let mut todos = use_signal(|| vec![
        TodoItem { id: 1, text: "First-Class Wasm Component Model in Servo".into(), completed: true },
        TodoItem { id: 2, text: "Dioxus 0.6 with zero JS glue".into(), completed: true },
        TodoItem { id: 3, text: "Demo native reactive DOM".into(), completed: false },
    ]);

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
                    style: "white-space: nowrap;",
                    onclick: move |evt| {
                        let coords = evt.client_coordinates();
                        let trigger = evt.trigger_button();
                        console::log(&format!(
                            "[Click]: Position ({}, {}), Button: {:?}", 
                            coords.x, coords.y, trigger
                        ));
                        let mut task_text = String::new();
                        if let Ok(input_elem) = document::get_element_by_id("new-todo-input") {
                            task_text = input_elem.get_property("value").trim().to_string();
                            input_elem.set_property("value", "");
                        }
                        if task_text.is_empty() {
                            task_text = "Task: Measure Wasm performance".to_string();
                        }
                        let next_id = todos.read().iter().map(|i| i.id).max().unwrap_or(0) + 1;
                        todos.write().push(TodoItem { id: next_id, text: task_text, completed: false });
                    },
                    "➕ Add Task"
                }
                button {
                    class: "btn btn-secondary",
                    style: "white-space: nowrap;",
                    onclick: move |_| {
                        todos.write().retain(|item| !item.completed);
                    },
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
                    {
                        let item_id = item.id;
                        let is_completed = item.completed;
                        rsx! {
                            div {
                                key: "{item_id}",
                                class: "task-card",
                                style: "display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; margin-bottom: 8px; border-radius: 8px; background: #313244;",

                                // Left: Checkbox + Text
                                div { style: "display: flex; align-items: center; gap: 10px;",
                                    button {
                                        style: "border: none; background: #1e1e2e; color: #a6e3a1; font-weight: bold; border-radius: 4px; padding: 4px 8px; cursor: pointer;",
                                        onclick: move |_| {
                                            if let Some(todo) = todos.write().iter_mut().find(|i| i.id == item_id) {
                                                todo.completed = !todo.completed;
                                            }
                                        },
                                        if is_completed { "☑️" } else { "⬜" }
                                    }
                                    span {
                                        style: if is_completed {
                                            "text-decoration: line-through; color: #6c7086;"
                                        } else {
                                            "color: #cdd6f4; font-weight: 500;"
                                        },
                                        "{item.text}"
                                    }
                                }

                                // Right: Delete button
                                button {
                                    style: "border: none; background: #f38ba8; color: #11111b; font-weight: bold; border-radius: 6px; padding: 4px 10px; cursor: pointer;",
                                    onclick: move |_| {
                                        todos.write().retain(|i| i.id != item_id);
                                    },
                                    "🗑️ Delete"
                                }
                            }
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

    console::log("[Dioxus Todo]: Mounted successfully with encapsulated event listeners!");
}

/// Universal Event Dispatcher: Routes native Servo events directly into Dioxus VDom closures!
pub fn on_event(handler_id: &str) {
    if let Some(id_str) = handler_id.strip_prefix("dioxus-") {
        if let Ok(raw_id) = id_str.parse::<usize>() {
            let element_id = ElementId(raw_id);
            STATE.with(|s| {
                if let Some(AppState { vdom, applier }) = s.borrow_mut().as_mut() {
                    // 1. Pack native ServoMouseData directly (zero serde, zero serialization)
                    let mouse_data = ServoMouseData::default();
                    let raw_data = PlatformEventData::new(Box::new(mouse_data));
                    let event_data: Rc<dyn std::any::Any> = Rc::new(raw_data);
                    let event: Event<dyn std::any::Any> = Event::new(event_data, true);
                    // 2. Dispatch event to target element closure
                    vdom.runtime().handle_event("click", event, element_id);
                    // 3. Render and apply reactive DOM mutations immediately
                    vdom.render_immediate(applier);
                }
            });
        }
    }
}