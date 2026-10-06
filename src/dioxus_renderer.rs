use std::collections::HashMap;
use std::rc::Rc;

use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use dioxus_html::{
    set_event_converter, HtmlEventConverter, MouseData, PlatformEventData, SerializedMouseData,
    AnimationData, ClipboardData, CompositionData, DragData, FocusData, FormData,
    ImageData, KeyboardData, MediaData, MountedData, PointerData,
    ResizeData, ScrollData, SelectionData, ToggleData, TouchData, TransitionData,
    VisibleData, WheelData,
};

use dioxus_html::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use dioxus_html::input_data::{MouseButton, MouseButtonSet, keyboard_types};
use dioxus_html::point_interaction::{
    InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
};
use dioxus_html::HasMouseData;
use keyboard_types::Modifiers;

use crate::bindings::servo::dom::{console, document::{
    create_element, create_text_node, Element,
}};

/// Native, zero-serialization in-memory mouse event data for Servo DOM
#[derive(Default, Clone)]
pub struct ServoMouseData {
    pub client_x: f64,
    pub client_y: f64,
    pub button: Option<MouseButton>,
}
impl HasMouseData for ServoMouseData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl InteractionLocation for ServoMouseData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.client_x, self.client_y)
    }
    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.client_x, self.client_y)
    }
    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.client_x, self.client_y)
    }
}
impl InteractionElementOffset for ServoMouseData {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.client_x, self.client_y)
    }
    fn coordinates(&self) -> Coordinates {
        todo!("")
    }
}
impl ModifiersInteraction for ServoMouseData {
    fn modifiers(&self) -> Modifiers {
        Modifiers::empty()
    }
}
impl PointerInteraction for ServoMouseData {
    fn held_buttons(&self) -> MouseButtonSet {
        MouseButtonSet::empty()
    }
    fn trigger_button(&self) -> Option<MouseButton> {
        self.button.or(Some(MouseButton::Primary))
    }
}

/// Event converter that bridges native Servo click events to Dioxus MouseData
struct ServoEventConverter;
impl HtmlEventConverter for ServoEventConverter {
    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<ServoMouseData>()
            .cloned()
            .map(MouseData::new)
            .unwrap_or_else(|| MouseData::new(ServoMouseData::default()))
    }
    fn convert_animation_data(&self, _: &PlatformEventData) -> AnimationData { unimplemented!() }
    fn convert_clipboard_data(&self, _: &PlatformEventData) -> ClipboardData { unimplemented!() }
    fn convert_composition_data(&self, _: &PlatformEventData) -> CompositionData { unimplemented!() }
    fn convert_drag_data(&self, _: &PlatformEventData) -> DragData { unimplemented!() }
    fn convert_focus_data(&self, _: &PlatformEventData) -> FocusData { unimplemented!() }
    fn convert_form_data(&self, _: &PlatformEventData) -> FormData { unimplemented!() }
    fn convert_image_data(&self, _: &PlatformEventData) -> ImageData { unimplemented!() }
    fn convert_keyboard_data(&self, _: &PlatformEventData) -> KeyboardData { unimplemented!() }
    fn convert_media_data(&self, _: &PlatformEventData) -> MediaData { unimplemented!() }
    fn convert_mounted_data(&self, _: &PlatformEventData) -> MountedData { unimplemented!() }
    fn convert_pointer_data(&self, _: &PlatformEventData) -> PointerData { unimplemented!() }
    fn convert_resize_data(&self, _: &PlatformEventData) -> ResizeData { unimplemented!() }
    fn convert_scroll_data(&self, _: &PlatformEventData) -> ScrollData { unimplemented!() }
    fn convert_selection_data(&self, _: &PlatformEventData) -> SelectionData { unimplemented!() }
    fn convert_toggle_data(&self, _: &PlatformEventData) -> ToggleData { unimplemented!() }
    fn convert_touch_data(&self, _: &PlatformEventData) -> TouchData { unimplemented!() }
    fn convert_transition_data(&self, _: &PlatformEventData) -> TransitionData { unimplemented!() }
    fn convert_visible_data(&self, _: &PlatformEventData) -> VisibleData { unimplemented!() }
    fn convert_wheel_data(&self, _: &PlatformEventData) -> WheelData { unimplemented!() }
}

/// Bridges Dioxus 0.6 VirtualDom mutations directly to native Servo DOM host calls
pub struct ServoDomApplier {
    pub elements: HashMap<ElementId, Rc<Element>>,
    pub template_stack: Vec<HashMap<Vec<u8>, Rc<Element>>>,
    pub root: Rc<Element>,
    pub stack: Vec<Rc<Element>>,
}

impl ServoDomApplier {
    pub fn new(root: Element) -> Self {
        // Register the event converter so Dioxus can convert PlatformEventData to MouseData/FormData
        set_event_converter(Box::new(ServoEventConverter));

        let root = Rc::new(root);
        let mut elements = HashMap::new();
        elements.insert(ElementId(0), Rc::clone(&root));
        Self {
            elements,
            template_stack: Vec::new(),
            root,
            stack: Vec::new(),
        }
    }

    fn build_template_node(
        &mut self,
        node: &'static TemplateNode,
        current_path: Vec<u8>,
        map: &mut HashMap<Vec<u8>, Rc<Element>>,
    ) -> Option<Rc<Element>> {
        match node {
            TemplateNode::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                let elem = Rc::new(create_element(tag).ok()?);
                for attr in *attrs {
                    if let TemplateAttribute::Static { name, value, .. } = attr {
                        let _ = elem.set_attribute(name, value);
                    }
                }
                for (i, child) in children.iter().enumerate() {
                    let mut child_path = current_path.clone();
                    child_path.push(i as u8);
                    if let Some(child_elem) = self.build_template_node(child, child_path, map) {
                        let _ = elem.append_child(&child_elem);
                    }
                }
                map.insert(current_path, Rc::clone(&elem));
                Some(elem)
            }
            TemplateNode::Text { text } => {
                let node = Rc::new(create_text_node(text).ok()?);
                map.insert(current_path, Rc::clone(&node));
                Some(node)
            }
            TemplateNode::Dynamic { .. } => {
                let node = Rc::new(create_text_node("").ok()?);
                map.insert(current_path, Rc::clone(&node));
                Some(node)
            }
        }
    }
}

impl WriteMutations for ServoDomApplier {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let start = self.stack.len().saturating_sub(m);
        let children: Vec<Rc<Element>> = self.stack.drain(start..).collect();
        if let Some(parent) = self.elements.get(&id) {
            for child in children {
                let _ = parent.append_child(&child);
            }
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        for map in self.template_stack.iter().rev() {
            if let Some(node) = map.get(path) {
                self.elements.insert(id, Rc::clone(node));
                return;
            }
        }
        console::log(&format!("[Renderer Warn]: assign_node_id failed for path {:?}", path));
    }

    fn create_placeholder(&mut self, id: ElementId) {
        if let Ok(node) = create_text_node("") {
            let node = Rc::new(node);
            self.elements.insert(id, Rc::clone(&node));
            self.stack.push(node);
        }
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        if let Ok(node) = create_text_node(value) {
            let node = Rc::new(node);
            self.elements.insert(id, Rc::clone(&node));
            self.stack.push(node);
        }
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let node = &template.roots[index];
        let path = Vec::new();
        let mut map = HashMap::new();
        if let Some(root_node) = self.build_template_node(node, path, &mut map) {
            if id != ElementId(0) {
                self.elements.insert(id, Rc::clone(&root_node));
            }
            self.stack.push(root_node);
            self.template_stack.push(map);
        }
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let start = self.stack.len().saturating_sub(m);
        let new_nodes: Vec<Rc<Element>> = self.stack.drain(start..).collect();
        if let Some(old_node) = self.elements.get(&id) {
            if let Some(parent) = old_node.parent_node() {
                for new_node in &new_nodes {
                    let _ = parent.append_child(new_node);
                }
                let _ = parent.remove_child(old_node);
            }
        }
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let start = self.stack.len().saturating_sub(m);
        let new_nodes: Vec<Rc<Element>> = self.stack.drain(start..).collect();

        let mut found_placeholder = None;
        for map in self.template_stack.iter_mut().rev() {
            if let Some(placeholder) = map.remove(path) {
                found_placeholder = Some(placeholder);
                break;
            }
        }

        if let Some(placeholder) = found_placeholder {
            if let Some(parent) = placeholder.parent_node() {
                for new_node in &new_nodes {
                    let _ = parent.append_child(new_node);
                }
                let _ = parent.remove_child(&placeholder);
            }
        }
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        let start = self.stack.len().saturating_sub(m);
        let new_nodes: Vec<Rc<Element>> = self.stack.drain(start..).collect();
        if let Some(target) = self.elements.get(&id) {
            if let Some(parent) = target.parent_node() {
                for node in new_nodes {
                    let _ = parent.append_child(&node);
                }
            }
        }
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        let start = self.stack.len().saturating_sub(m);
        let new_nodes: Vec<Rc<Element>> = self.stack.drain(start..).collect();
        if let Some(target) = self.elements.get(&id) {
            if let Some(parent) = target.parent_node() {
                for node in new_nodes {
                    let _ = parent.append_child(&node);
                }
            }
        }
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        if let Some(elem) = self.elements.get(&id) {
            match value {
                AttributeValue::Text(s) => {
                    let _ = elem.set_attribute(name, s);
                }
                AttributeValue::Bool(b) => {
                    if *b {
                        let _ = elem.set_attribute(name, name);
                    } else {
                        let _ = elem.set_attribute(name, "");
                    }
                }
                AttributeValue::Float(f) => {
                    let _ = elem.set_attribute(name, &f.to_string());
                }
                AttributeValue::Int(i) => {
                    let _ = elem.set_attribute(name, &i.to_string());
                }
                _ => {}
            }
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        if let Some(elem) = self.elements.get(&id) {
            let _ = elem.set_text_content(value);
        }
    }

    fn remove_node(&mut self, id: ElementId) {
        if let Some(elem) = self.elements.remove(&id) {
            if let Some(parent) = elem.parent_node() {
                let _ = parent.remove_child(&elem);
            }
        }
    }

    fn push_root(&mut self, id: ElementId) {
        if let Some(node) = self.elements.get(&id) {
            self.stack.push(Rc::clone(node));
        }
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if let Some(elem) = self.elements.get(&id) {
            let handler_id = format!("dioxus-{}", id.0);
            elem.add_event_listener(name, &handler_id);
        }
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        if let Some(elem) = self.elements.get(&id) {
            let handler_id = format!("dioxus-{}", id.0);
            elem.remove_event_listener(name, &handler_id);
        }
    }
}