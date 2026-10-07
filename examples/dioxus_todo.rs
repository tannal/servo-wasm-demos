use dioxus_core::{Element, ElementId, Event, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_core::IntoDynNode;
use dioxus_html::point_interaction::InteractionLocation;
use dioxus_html::{self as dioxus_elements, PlatformEventData};
use dioxus_signals::{GlobalSignal, Readable, Signal, Writable};
use dioxus_hooks::{use_memo, use_signal};

use std::cell::RefCell;
use std::rc::Rc;

use crate::bindings::servo::dom::{console, document};
use crate::dioxus_renderer::{ServoDomApplier, ServoMouseData};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn label(&self) -> &'static str {
        match self {
            Priority::Low => "[LOW]",
            Priority::Medium => "[MED]",
            Priority::High => "[HIGH]",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Priority::Low => "#a6adc8",
            Priority::Medium => "#f9e2af",
            Priority::High => "#f38ba8",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterMode {
    All,
    Active,
    Completed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TodoItem {
    pub id: u32,
    pub text: String,
    pub priority: Priority,
    pub completed: bool,
}

struct AppState {
    vdom: VirtualDom,
    applier: ServoDomApplier,
}

thread_local! {
    static STATE: RefCell<Option<AppState>> = RefCell::new(None);
}

/// Dioxus 0.6 Reactive Task Engine with Dynamic Search and Zero Emojis
fn TodoApp() -> Element {
    // 1. Core Reactive Signals
    let mut todos = use_signal(|| vec![
        TodoItem {
            id: 1,
            text: "Implement Wasm Component Model in Servo".into(),
            priority: Priority::High,
            completed: true,
        },
        TodoItem {
            id: 2,
            text: "Dioxus 0.6 VirtualDOM diffing with zero JS glue".into(),
            priority: Priority::High,
            completed: true,
        },
        TodoItem {
            id: 3,
            text: "Zero-copy JS and Wasm DOM synchronization".into(),
            priority: Priority::Medium,
            completed: false,
        },
        TodoItem {
            id: 4,
            text: "Houdini MathML fraction worklet layout in Wasm".into(),
            priority: Priority::High,
            completed: false,
        },
        TodoItem {
            id: 5,
            text: "Benchmark IPC latency vs in-process WIT calls".into(),
            priority: Priority::Low,
            completed: false,
        },
    ]);

    let mut current_filter = use_signal(|| FilterMode::All);
    let mut search_query = use_signal(String::new);
    let mut selected_priority = use_signal(|| Priority::Medium);

    // 2. Computed / Memoized Statistics
    let total_count = use_memo(move || todos.read().len());
    let active_count = use_memo(move || todos.read().iter().filter(|i| !i.completed).count());
    let completed_count = use_memo(move || todos.read().iter().filter(|i| i.completed).count());

    // 3. Dynamic Search + Filter Memoization
    let filter = *current_filter.read();
    let query_str = search_query.read().trim().to_lowercase();
    let visible_todos = use_memo(move || {
        let q = search_query.read().trim().to_lowercase();
        let f = *current_filter.read();
        todos.read().iter().filter(|item| {
            let matches_filter = match f {
                FilterMode::All => true,
                FilterMode::Active => !item.completed,
                FilterMode::Completed => item.completed,
            };
            let matches_query = q.is_empty() 
                || item.text.to_lowercase().contains(&q) 
                || item.priority.label().to_lowercase().contains(&q);
            matches_filter && matches_query
        }).cloned().collect::<Vec<_>>()
    });

    let match_count = visible_todos.read().len();

    // 4. Publish metrics to host DOM data attributes for external JS observation
    if let Ok(root) = document::get_element_by_id("todo-app-root") {
        root.set_attribute("data-total", &total_count.read().to_string());
        root.set_attribute("data-active", &active_count.read().to_string());
        root.set_attribute("data-completed", &completed_count.read().to_string());
        root.set_attribute("data-matches", &match_count.to_string());
        root.set_attribute("data-query", &search_query.read());
    }

    let items = visible_todos.read().clone();
    let prio = *selected_priority.read();
    let active_query = search_query.read().clone();

    rsx! {
        div { class: "wasm-card",
            // Header
            div { style: "display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 6px;",
                h1 { style: "margin: 0; font-size: 1.4rem; letter-spacing: -0.5px; color: #89b4fa;",
                    "SERVO WASM TASK ENGINE"
                }
                span { style: "font-family: monospace; font-size: 0.72rem; color: #a6adc8; background: #11111b; padding: 3px 8px; border-radius: 4px; border: 1px solid #313244;",
                    "DIOXUS-0.6 // NATIVE-WASM"
                }
            }
            p { class: "subtitle", "In-process reactive state and dynamic search with zero JS glue" }

            // Dynamic Search Bar + Quick Tag Chips
            div { style: "background: #11111b; border: 1px solid #313244; border-radius: 8px; padding: 10px; margin-bottom: 16px;",
                div { style: "display: flex; gap: 8px; align-items: center;",
                    input {
                        id: "search-input",
                        placeholder: "Type keyword to filter tasks...",
                        style: "flex: 1; padding: 8px 10px; border-radius: 6px; border: 1px solid #45475a; background: #181825; color: #cdd6f4; font-size: 0.85rem; outline: none;",
                    }
                    button {
                        id: "btn-search-apply",
                        class: "btn btn-secondary",
                        style: "padding: 8px 12px; font-size: 0.8rem;",
                        onclick: move |_| {
                            if let Ok(elem) = document::get_element_by_id("search-input") {
                                let val = elem.get_property("value");
                                search_query.set(val);
                            }
                        },
                        "SEARCH"
                    }
                    button {
                        id: "btn-search-clear",
                        class: "btn btn-secondary",
                        style: "padding: 8px 12px; font-size: 0.8rem; color: #f38ba8;",
                        onclick: move |_| {
                            if let Ok(elem) = document::get_element_by_id("search-input") {
                                elem.set_property("value", "");
                            }
                            search_query.set(String::new());
                        },
                        "RESET"
                    }
                }

                // Quick Topic Filter Chips
                div { style: "display: flex; gap: 6px; margin-top: 8px; align-items: center;",
                    span { style: "font-size: 0.75rem; color: #6c7086;", "QUICK TAGS:" }
                    for tag in &["Wasm", "Servo", "Houdini", "IPC"] {
                        {
                            let tag_val = tag.to_string();
                            rsx! {
                                button {
                                    class: "chip-btn",
                                    onclick: move |_| {
                                        if let Ok(elem) = document::get_element_by_id("search-input") {
                                            elem.set_property("value", &tag_val);
                                        }
                                        search_query.set(tag_val.clone());
                                    },
                                    "[{tag}]"
                                }
                            }
                        }
                    }
                }
            }

            // Input Form Section
            div { style: "display: flex; gap: 8px; margin-bottom: 12px;",
                input {
                    id: "new-todo-input",
                    placeholder: "Create new task...",
                    style: "flex: 1; padding: 9px 12px; border-radius: 6px; border: 1px solid #45475a; background: #11111b; color: #cdd6f4; font-size: 0.85rem; outline: none;",
                }
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 0.78rem; padding: 0 10px;",
                    onclick: move |_| {
                        let next = match prio {
                            Priority::Low => Priority::Medium,
                            Priority::Medium => Priority::High,
                            Priority::High => Priority::Low,
                        };
                        selected_priority.set(next);
                    },
                    "Priority: {prio.label()}"
                }
                button {
                    id: "btn-add-task",
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let mut text = String::new();
                        if let Ok(input_elem) = document::get_element_by_id("new-todo-input") {
                            text = input_elem.get_property("value").trim().to_string();
                            input_elem.set_property("value", "");
                        }
                        if text.is_empty() {
                            text = "New Wasm Worklet Task".into();
                        }
                        let next_id = todos.read().iter().map(|i| i.id).max().unwrap_or(0) + 1;
                        todos.write().push(TodoItem {
                            id: next_id,
                            text,
                            priority: prio,
                            completed: false,
                        });
                    },
                    "ADD TASK"
                }
            }

            // Filter Tabs & Search Summary
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px;",
                div { style: "display: flex; gap: 4px;",
                    button {
                        class: if filter == FilterMode::All { "filter-btn active" } else { "filter-btn" },
                        onclick: move |_| current_filter.set(FilterMode::All),
                        "ALL ({total_count.read()})"
                    }
                    button {
                        class: if filter == FilterMode::Active { "filter-btn active" } else { "filter-btn" },
                        onclick: move |_| current_filter.set(FilterMode::Active),
                        "ACTIVE ({active_count.read()})"
                    }
                    button {
                        class: if filter == FilterMode::Completed { "filter-btn active" } else { "filter-btn" },
                        onclick: move |_| current_filter.set(FilterMode::Completed),
                        "DONE ({completed_count.read()})"
                    }
                }

                div { style: "font-size: 0.75rem; color: #a6adc8;",
                    if !active_query.is_empty() {
                        span { "Matches: {match_count} for '{active_query}'" }
                    } else {
                        span { "Showing {match_count} items" }
                    }
                }

                button {
                    class: "btn-action",
                    onclick: move |_| {
                        todos.write().retain(|item| !item.completed);
                    },
                    "CLEAR COMPLETED"
                }
            }

            // Declarative Task List
            div { id: "todo-list-container",
                if items.is_empty() {
                    div { style: "padding: 24px 0; text-align: center; color: #6c7086; font-size: 0.85rem; border: 1px dashed #313244; border-radius: 6px;",
                        "No tasks matching filter criteria"
                    }
                }
                for item in items.iter() {
                    {
                        let item_id = item.id;
                        let is_done = item.completed;
                        let item_priority = item.priority;
                        rsx! {
                            div {
                                key: "{item_id}",
                                class: "task-card",
                                style: "display: flex; align-items: center; justify-content: space-between; padding: 9px 12px; margin-bottom: 6px; border-radius: 6px; background: #313244;",

                                // Left: Checkbox + Priority Badge + Title
                                div { style: "display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0;",
                                    button {
                                        class: "btn-checkbox",
                                        onclick: move |_| {
                                            if let Some(t) = todos.write().iter_mut().find(|i| i.id == item_id) {
                                                t.completed = !t.completed;
                                            }
                                        },
                                        if is_done { "[X]" } else { "[ ]" }
                                    }
                                    span {
                                        style: "font-family: monospace; font-size: 0.75rem; font-weight: bold; color: {item_priority.color()}; min-width: 44px;",
                                        "{item_priority.label()}"
                                    }
                                    span {
                                        style: if is_done {
                                            "text-decoration: line-through; color: #6c7086; font-size: 0.85rem;"
                                        } else {
                                            "color: #cdd6f4; font-size: 0.85rem; font-weight: 500;"
                                        },
                                        "{item.text}"
                                    }
                                }

                                // Right: Delete button
                                button {
                                    class: "btn-delete",
                                    onclick: move |_| {
                                        todos.write().retain(|i| i.id != item_id);
                                    },
                                    "DELETE"
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
    console::log("[Dioxus Todo]: Mounting Dioxus 0.6 VirtualDOM with Dynamic Search...");
    let container = document::get_element_by_id("todo-app-root")
        .unwrap_or_else(|_| document::get_body().expect("body"));

    let mut vdom = VirtualDom::new(TodoApp);
    let mut applier = ServoDomApplier::new(container);

    vdom.rebuild(&mut applier);

    STATE.with(|s| {
        *s.borrow_mut() = Some(AppState { vdom, applier });
    });

    console::log("[Dioxus Todo]: VirtualDOM successfully mounted!");
}

pub fn on_event(handler_id: &str) {
    if let Some(id_str) = handler_id.strip_prefix("dioxus-") {
        if let Ok(raw_id) = id_str.parse::<usize>() {
            let element_id = ElementId(raw_id);
            STATE.with(|s| {
                if let Some(AppState { vdom, applier }) = s.borrow_mut().as_mut() {
                    let mouse_data = ServoMouseData::default();
                    let raw_data = PlatformEventData::new(Box::new(mouse_data));
                    let event_data: Rc<dyn std::any::Any> = Rc::new(raw_data);
                    let event: Event<dyn std::any::Any> = Event::new(event_data, true);

                    vdom.runtime().handle_event("click", event, element_id);
                    vdom.render_immediate(applier);
                }
            });
        }
    }
}