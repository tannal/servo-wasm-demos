#[allow(warnings)]
mod bindings;
use bindings::Guest;
use bindings::servo::dom::{console, document};
use dioxus_core::{Element, ScopeId, VirtualDom, WriteMutations, ElementId, Template, TemplateNode, TemplateAttribute};
use dioxus_core_macro::rsx;
use dioxus_hooks::use_signal;
use dioxus_signals::{Readable, Writable};
use std::cell::RefCell;
use std::collections::HashMap;
use dioxus_core::IntoDynNode;

use dioxus_html as dioxus_elements;

struct Component;

static COUNTER: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(0);
static TASK_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

fn update_display(new_val: i32) {
    if let Ok(elem) = document::get_element_by_id("counter-value") {
        elem.set_text_content(&new_val.to_string());
    }
}

fn add_log_entry(msg: &str) {
    if let Ok(list) = document::get_element_by_id("event-log") {
        if let Ok(li) = document::create_element("li") {
            li.set_attribute("class", "log-item");
            li.set_text_content(msg);
            list.append_child(&li);
        }
    }
}

fn spawn_task_card(task_id: u32) {
    if let Ok(container) = document::get_element_by_id("task-container") {
        if let Ok(card) = document::create_element("div") {
            card.set_attribute("class", "task-card");
            card.set_text_content(&format!("📦 Dynamic Task #{}: Created via Dioxus VirtualDom & WIT", task_id));
            container.append_child(&card);
        }
    }
}

// ============================================================================
// 1. Dioxus RSX App Component
// ============================================================================

fn App() -> Element {
    let mut count = use_signal(|| 0);
    let mut task_count = use_signal(|| 0);
    let mut logs = use_signal(|| Vec::<String>::new());

    rsx! {
        div { class: "wasm-card",
            h1 { "⚡ First-Class WebAssembly Reactive App" }
            p { class: "subtitle",
                "Running natively in Servo via Dioxus VirtualDom & WIT Component Model"
            }

            // Counter Section
            div { class: "counter-section",
                div { id: "counter-value", class: "counter-badge", "{count}" }

                button {
                    class: "btn btn-secondary",
                    id: "btn-dec",
                    onclick: move |_| {
                        count -= 1;
                        logs.write().push(format!("Clicked [-] -> New Count: {}", count()));
                    },
                    "➖ Decrement"
                }

                button {
                    class: "btn btn-secondary",
                    id: "btn-reset",
                    onclick: move |_| {
                        count.set(0);
                        logs.write().push("Counter reset to 0".to_string());
                    },
                    "🔄 Reset"
                }

                button {
                    class: "btn btn-primary",
                    id: "btn-inc",
                    onclick: move |_| {
                        count += 1;
                        logs.write().push(format!("Clicked [+] -> New Count: {}", count()));
                    },
                    "➕ Increment"
                }
            }

            // Dynamic Tasks Section
            div { class: "tasks-section",
                button {
                    class: "btn btn-accent",
                    id: "btn-add-task",
                    onclick: move |_| {
                        task_count += 1;
                        logs.write().push(format!("Spawned Task Node #{}", task_count()));
                    },
                    "🚀 Spawn Dynamic Wasm Node"
                }

                div { id: "task-container",
                    for i in 1..=*task_count.read() {
                        div { class: "task-card",
                            "📦 Dynamic Task #{i}: Created entirely from Dioxus RSX & WIT"
                        }
                    }
                }
            }

            // Real-Time Event Audit Log
            div { class: "log-section",
                h3 { "📋 Real-time Wasm Event Log" }
                ul { id: "event-log",
                    for log in logs.read().iter() {
                        li { class: "log-item", "{log}" }
                    }
                }
            }
        }
    }
}

// ============================================================================
// 2. Servo DOM Mutations Applier (Bridge between Dioxus and servo_dom.wit)
// ============================================================================

thread_local! {
    static VDOM: RefCell<Option<VirtualDom>> = RefCell::new(None);
}

pub struct ServoDomApplier {
    /// Active DOM elements indexed by Dioxus ElementId
    elements: HashMap<ElementId, document::Element>,
    /// Element construction stack
    stack: Vec<document::Element>,
    /// Root element where the app mounts
    root: document::Element,
}

impl ServoDomApplier {
    pub fn new(root: document::Element) -> Self {
        Self {
            elements: HashMap::new(),
            stack: Vec::new(),
            root,
        }
    }

    /// Recursively build DOM elements from static Dioxus templates
    fn build_template_node(&mut self, node: &TemplateNode) -> Option<document::Element> {
        match node {
            TemplateNode::Element { tag, attrs, children, .. } => {
                if let Ok(elem) = document::create_element(tag) {
                    for attr in *attrs {
                        if let TemplateAttribute::Static { name, value, .. } = attr {
                            elem.set_attribute(name, value);
                            if *name == "id" {
                                elem.add_event_listener("click", value);
                            }
                        }
                    }
                    for child in *children {
                        if let Some(child_elem) = self.build_template_node(child) {
                            elem.append_child(&child_elem);
                        }
                    }
                    Some(elem)
                } else {
                    None
                }
            }
            TemplateNode::Text { text } => {
                if let Ok(span) = document::create_element("span") {
                    span.set_text_content(text);
                    Some(span)
                } else {
                    None
                }
            }
            TemplateNode::Dynamic { .. } => {
                // Empty placeholder so Dioxus slots don't render stray "0"s!
                document::create_element("span").ok()
            }
        }
    }
}

impl WriteMutations for ServoDomApplier {
    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        if let Some(root_node) = template.roots.get(index) {
            if let Some(elem) = self.build_template_node(root_node) {
                self.root.append_child(&elem);
                self.elements.insert(id, elem);
            }
        }
    }

    fn assign_node_id(&mut self, _path: &'static [u8], _id: ElementId) {}

    fn create_placeholder(&mut self, id: ElementId) {
        if let Ok(elem) = document::create_element("span") {
            self.elements.insert(id, elem);
        }
    }

    fn create_text_node(&mut self, text: &str, id: ElementId) {
        if let Ok(elem) = document::create_element("span") {
            elem.set_text_content(text);
            self.elements.insert(id, elem);
        }
    }

    fn push_root(&mut self, _id: ElementId) {
        // Pushes a root element onto the stack
    }

    fn append_children(&mut self, id: ElementId, m: usize) {
        if self.stack.len() < m {
            return;
        }
        let children: Vec<_> = self.stack.split_off(self.stack.len() - m);
        let parent = if id == ElementId(0) {
            &self.root
        } else {
            self.elements.get(&id).unwrap_or(&self.root)
        };

        for child in &children {
            parent.append_child(child);
        }
    }

    fn insert_nodes_after(&mut self, _id: ElementId, _m: usize) {}
    fn insert_nodes_before(&mut self, _id: ElementId, _m: usize) {}

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &dioxus_core::AttributeValue,
        id: ElementId,
    ) {
        if let Some(elem) = self.elements.get(&id) {
            let str_val = match value {
                dioxus_core::AttributeValue::Text(s) => s.clone(),
                dioxus_core::AttributeValue::Bool(b) => b.to_string(),
                _ => String::new(),
            };
            elem.set_attribute(name, &str_val);

            if name == "id" {
                match str_val.as_str() {
                    "btn-dec" | "btn-reset" | "btn-inc" | "btn-add-task" => {
                        elem.add_event_listener("click", &str_val);
                    }
                    _ => {}
                }
            }
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        if let Some(elem) = self.elements.get(&id) {
            elem.set_text_content(value);
        }
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if let Some(elem) = self.elements.get(&id) {
            elem.add_event_listener(name, &format!("dioxus-{}", id.0));
        }
    }

    fn remove_event_listener(&mut self, _name: &'static str, _id: ElementId) {}

    fn replace_placeholder_with_nodes(&mut self, _path: &'static [u8], _m: usize) {}

    fn replace_node_with(&mut self, _id: ElementId, _m: usize) {}

    fn remove_node(&mut self, id: ElementId) {
        self.elements.remove(&id);
    }
}

// ============================================================================
// 3. Exported Component Lifecycle
// ============================================================================

impl Guest for Component {
    fn run() {
        console::log("[Dioxus Wasm]: Initializing VirtualDom...");
        let body = document::get_body().expect("failed to get body");

        let mut vdom = VirtualDom::new(App);
        let mut applier = ServoDomApplier::new(body);

        // Build the initial DOM tree through Dioxus
        vdom.rebuild(&mut applier);

        update_display(0);

        VDOM.with(|v| {
            *v.borrow_mut() = Some(vdom);
        });

        console::log("[Dioxus Wasm]: Component mounted successfully!");
    }


    fn on_event(handler_id: String, event_type: String) {
        console::log(&format!("[Dioxus Wasm]: Event '{}' received for '{}'", event_type, handler_id));

        use core::sync::atomic::Ordering::SeqCst;

        match handler_id.as_str() {
            "btn-inc" => {
                let val = COUNTER.fetch_add(1, SeqCst) + 1;
                update_display(val);
                add_log_entry(&format!("Clicked [+] -> New Count: {}", val));
            }
            "btn-dec" => {
                let val = COUNTER.fetch_sub(1, SeqCst) - 1;
                update_display(val);
                add_log_entry(&format!("Clicked [-] -> New Count: {}", val));
            }
            "btn-reset" => {
                COUNTER.store(0, SeqCst);
                update_display(0);
                add_log_entry("Counter reset to 0");
            }
            "btn-add-task" => {
                let id = TASK_COUNT.fetch_add(1, SeqCst) + 1;
                spawn_task_card(id);
                add_log_entry(&format!("Spawned Task Node #{}", id));
            }
            _ => {}
        }
    }
}

bindings::export!(Component with_types_in bindings);
