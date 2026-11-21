use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, WriteMutations,
};
use dioxus_html::geometry::{ClientPoint, Coordinates, ElementPoint, PagePoint, ScreenPoint};
use dioxus_html::input_data::{MouseButton, MouseButtonSet};
use dioxus_html::point_interaction::{
    InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
};
use dioxus_html::prelude::{Code, Key, Location, Modifiers};
use dioxus_html::{
    AnimationData, ClipboardData, CompositionData, DragData, FocusData, FormData, HasFileData,
    HasFormData, HasKeyboardData, HasMouseData, HtmlEventConverter, ImageData, KeyboardData,
    MediaData, MountedData, MouseData, PlatformEventData, PointerData, ResizeData, ScrollData,
    SelectionData, ToggleData, TouchData, TransitionData, VisibleData, WheelData,
    set_event_converter,
};

use crate::bindings::servo::dom::document::{self, Element};

/// Mouse payload produced by Servo's native `click` dispatch.
#[derive(Default, Clone)]
pub struct ServoMouseData {
    pub client_x: f64,
    pub client_y: f64,
    pub button: Option<MouseButton>,
}

impl HasMouseData for ServoMouseData {
    fn as_any(&self) -> &dyn Any {
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
        unimplemented!("element-relative coordinates are not exposed yet")
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

/// Keyboard payload produced by Servo's native `keydown` dispatch.
#[derive(Clone, Debug)]
pub struct ServoKeyboardData {
    pub key: Key,
    pub code: Code,
    pub location: Location,
    pub repeat: bool,
    pub composing: bool,
    pub modifiers: Modifiers,
}

impl Default for ServoKeyboardData {
    fn default() -> Self {
        Self {
            key: Key::Character(String::new()),
            code: Code::Unidentified,
            location: Location::Standard,
            repeat: false,
            composing: false,
            modifiers: Modifiers::empty(),
        }
    }
}

impl ServoKeyboardData {
    pub fn from_key_string(key: &str) -> Self {
        Self {
            key: key
                .parse::<Key>()
                .unwrap_or_else(|_| Key::Character(key.to_string())),
            ..Default::default()
        }
    }
}

impl ModifiersInteraction for ServoKeyboardData {
    fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

impl HasKeyboardData for ServoKeyboardData {
    fn key(&self) -> Key {
        self.key.clone()
    }

    fn code(&self) -> Code {
        self.code
    }

    fn location(&self) -> Location {
        self.location
    }

    fn is_auto_repeating(&self) -> bool {
        self.repeat
    }

    fn is_composing(&self) -> bool {
        self.composing
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Form payload produced by native `input`/`change` dispatch.
#[derive(Default, Clone, Debug)]
pub struct ServoFormData {
    pub value: String,
}

impl HasFileData for ServoFormData {}

impl HasFormData for ServoFormData {
    fn value(&self) -> String {
        self.value.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Converts native Servo event data into Dioxus's event payloads.
struct ServoEventConverter;

impl HtmlEventConverter for ServoEventConverter {
    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<ServoMouseData>()
            .cloned()
            .map(MouseData::new)
            .unwrap_or_else(|| MouseData::new(ServoMouseData::default()))
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        KeyboardData::from(
            event
                .downcast::<ServoKeyboardData>()
                .cloned()
                .unwrap_or_default(),
        )
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        FormData::from(
            event
                .downcast::<ServoFormData>()
                .cloned()
                .unwrap_or_default(),
        )
    }

    fn convert_animation_data(&self, _: &PlatformEventData) -> AnimationData {
        unimplemented!()
    }

    fn convert_clipboard_data(&self, _: &PlatformEventData) -> ClipboardData {
        unimplemented!()
    }

    fn convert_composition_data(&self, _: &PlatformEventData) -> CompositionData {
        unimplemented!()
    }

    fn convert_drag_data(&self, _: &PlatformEventData) -> DragData {
        unimplemented!()
    }

    fn convert_focus_data(&self, _: &PlatformEventData) -> FocusData {
        unimplemented!()
    }

    fn convert_image_data(&self, _: &PlatformEventData) -> ImageData {
        unimplemented!()
    }

    fn convert_media_data(&self, _: &PlatformEventData) -> MediaData {
        unimplemented!()
    }

    fn convert_mounted_data(&self, _: &PlatformEventData) -> MountedData {
        unimplemented!()
    }

    fn convert_pointer_data(&self, _: &PlatformEventData) -> PointerData {
        unimplemented!()
    }

    fn convert_resize_data(&self, _: &PlatformEventData) -> ResizeData {
        unimplemented!()
    }

    fn convert_scroll_data(&self, _: &PlatformEventData) -> ScrollData {
        unimplemented!()
    }

    fn convert_selection_data(&self, _: &PlatformEventData) -> SelectionData {
        unimplemented!()
    }

    fn convert_toggle_data(&self, _: &PlatformEventData) -> ToggleData {
        unimplemented!()
    }

    fn convert_touch_data(&self, _: &PlatformEventData) -> TouchData {
        unimplemented!()
    }

    fn convert_transition_data(&self, _: &PlatformEventData) -> TransitionData {
        unimplemented!()
    }

    fn convert_visible_data(&self, _: &PlatformEventData) -> VisibleData {
        unimplemented!()
    }

    fn convert_wheel_data(&self, _: &PlatformEventData) -> WheelData {
        unimplemented!()
    }
}

/// Bridges Dioxus 0.6 virtual-DOM mutations to native Servo DOM calls.
pub struct ServoDomApplier {
    pub elements: HashMap<ElementId, Rc<Element>>,
    pub template_stack: Vec<HashMap<Vec<u8>, Rc<Element>>>,
    pub root: Rc<Element>,
    pub stack: Vec<Rc<Element>>,
}

impl ServoDomApplier {
    pub fn new(root: Element) -> Self {
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
                let elem = Rc::new(document::create_element(tag).ok()?);
                for attr in *attrs {
                    if let TemplateAttribute::Static { name, value, .. } = attr {
                        let _ = elem.set_attribute(name, value);
                    }
                }
                for (index, child) in children.iter().enumerate() {
                    let mut child_path = current_path.clone();
                    child_path.push(index as u8);
                    if let Some(child_elem) = self.build_template_node(child, child_path, map) {
                        let _ = elem.append_child(&child_elem);
                    }
                }
                map.insert(current_path, Rc::clone(&elem));
                Some(elem)
            },
            TemplateNode::Text { text } => {
                let node = Rc::new(document::create_text_node(text).ok()?);
                map.insert(current_path, Rc::clone(&node));
                Some(node)
            },
            TemplateNode::Dynamic { .. } => {
                let node = Rc::new(document::create_text_node("").ok()?);
                map.insert(current_path, Rc::clone(&node));
                Some(node)
            },
        }
    }

    fn drain_stack(&mut self, m: usize) -> Vec<Rc<Element>> {
        let start = self.stack.len().saturating_sub(m);
        self.stack.drain(start..).collect()
    }
}

impl WriteMutations for ServoDomApplier {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let children = self.drain_stack(m);
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
        println!("[Servo Wasm Renderer]: assign_node_id failed for path {path:?}");
    }

    fn create_placeholder(&mut self, id: ElementId) {
        if let Ok(node) = document::create_text_node("") {
            let node = Rc::new(node);
            self.elements.insert(id, Rc::clone(&node));
            self.stack.push(node);
        }
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        if let Ok(node) = document::create_text_node(value) {
            let node = Rc::new(node);
            self.elements.insert(id, Rc::clone(&node));
            self.stack.push(node);
        }
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let node = &template.roots[index];
        let mut map = HashMap::new();
        if let Some(root_node) = self.build_template_node(node, Vec::new(), &mut map) {
            if id != ElementId(0) {
                self.elements.insert(id, Rc::clone(&root_node));
            }
            self.stack.push(root_node);
            self.template_stack.push(map);
        }
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.drain_stack(m);
        if let Some(old_node) = self.elements.get(&id).cloned() {
            if let Some(parent) = old_node.parent_node() {
                for new_node in &new_nodes {
                    let _ = parent.insert_before(new_node, &old_node);
                }
                let _ = parent.remove_child(&old_node);
            }
        }
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let new_nodes = self.drain_stack(m);
        let mut placeholder = None;
        for map in self.template_stack.iter_mut().rev() {
            if let Some(found) = map.remove(path) {
                placeholder = Some(found);
                break;
            }
        }
        if let Some(placeholder) = placeholder {
            if let Some(parent) = placeholder.parent_node() {
                for new_node in &new_nodes {
                    let _ = parent.insert_before(new_node, &placeholder);
                }
                let _ = parent.remove_child(&placeholder);
            }
        }
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.drain_stack(m);
        if let Some(target) = self.elements.get(&id).cloned() {
            if let Some(parent) = target.parent_node() {
                match target.next_sibling() {
                    Some(next) => {
                        for node in &new_nodes {
                            let _ = parent.insert_before(node, &next);
                        }
                    },
                    None => {
                        for node in &new_nodes {
                            let _ = parent.append_child(node);
                        }
                    },
                }
            }
        }
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        let new_nodes = self.drain_stack(m);
        if let Some(target) = self.elements.get(&id).cloned() {
            if let Some(parent) = target.parent_node() {
                for node in &new_nodes {
                    let _ = parent.insert_before(node, &target);
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
        let Some(elem) = self.elements.get(&id) else {
            return;
        };
        match value {
            AttributeValue::Text(text) => {
                let _ = elem.set_attribute(name, text);
            },
            AttributeValue::Bool(true) => {
                let _ = elem.set_attribute(name, name);
            },
            AttributeValue::Bool(false) => {
                elem.remove_attribute(name);
            },
            AttributeValue::Float(value) => {
                let _ = elem.set_attribute(name, &value.to_string());
            },
            AttributeValue::Int(value) => {
                let _ = elem.set_attribute(name, &value.to_string());
            },
            // Dioxus also has listener / non-string variants.  The matching
            // value is delivered through the event payload instead.
            _ => {},
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
