// wasm_guest/src/dioxus_renderer.rs
use crate::bindings::servo::dom::document;
use dioxus_core::{ElementId, Template, TemplateNode, TemplateAttribute, WriteMutations};
use std::collections::HashMap;

pub struct ServoDomApplier {
    pub elements: HashMap<ElementId, document::Element>,
    pub root: document::Element,
}

impl ServoDomApplier {
    pub fn new(root: document::Element) -> Self {
        Self {
            elements: HashMap::new(),
            root,
        }
    }

    pub fn build_template_node(&mut self, node: &TemplateNode) -> Option<document::Element> {
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

    fn push_root(&mut self, _id: ElementId) {}

    fn append_children(&mut self, id: ElementId, m: usize) {
        // Appending logic for dynamic elements
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
                elem.add_event_listener("click", &str_val);
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
