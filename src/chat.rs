//! A ChatGPT-style chat application written entirely in Rust.
//!
//! It runs as a native WebAssembly component inside Servo.  Dioxus 0.6's
//! virtual DOM drives native DOM nodes through the `servo:dom` WIT interface,
//! and all networking goes through `servo:dom/fetch`, which executes requests
//! off the ScriptThread.
//!
//! The same component supports local Ollama / llama.cpp servers and hosted
//! OpenAI-compatible APIs (DeepSeek, boat.dev-style gateways, OpenRouter, ...).
//! When served from `tannal.github.io` it defaults to DeepSeek and asks for an
//! API key in the sidebar; when served from localhost it defaults to Ollama.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use base64::Engine as _;
use dioxus_core::{ElementId, Event, IntoDynNode, VirtualDom};
use dioxus_core_macro::rsx;
use dioxus_hooks::use_signal;
use dioxus_html::PlatformEventData;
use dioxus_html::point_interaction::ModifiersInteraction;
use dioxus_html::prelude::Modifiers;
use dioxus_html as dioxus_elements;
use dioxus_signals::{Readable, Signal, Writable};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::bindings::EventData;
use crate::bindings::servo::dom::console;
use crate::bindings::servo::dom::document;
use crate::bindings::servo::dom::fetch;
use crate::dioxus_renderer::{ServoDomApplier, ServoFormData, ServoKeyboardData, ServoMouseData};

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    User,
    Assistant,
}

#[derive(Clone)]
struct Message {
    role: Role,
    text: String,
    image_data_url: Option<String>,
    image_base64: Option<String>,
}

#[derive(Clone)]
struct Attachment {
    name: String,
    mime: String,
    base64: String,
    data_url: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Provider {
    Ollama,
    LlamaCpp,
    DeepSeek,
    Custom,
}

const PROVIDERS: [Provider; 4] = [
    Provider::Ollama,
    Provider::LlamaCpp,
    Provider::DeepSeek,
    Provider::Custom,
];

impl Provider {
    fn label(self) -> &'static str {
        match self {
            Provider::Ollama => "Ollama",
            Provider::LlamaCpp => "llama.cpp",
            Provider::DeepSeek => "DeepSeek",
            Provider::Custom => "Custom",
        }
    }

    fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Provider::Ollama => ("http://127.0.0.1:11434", "qwen3.5-27b-q4:latest"),
            Provider::LlamaCpp => ("http://127.0.0.1:8080/v1", "local-model"),
            Provider::DeepSeek => ("https://api.deepseek.com/v1", "deepseek-chat"),
            Provider::Custom => ("https://api.openai.com/v1", "gpt-4o-mini"),
        }
    }

    fn requires_key(self) -> bool {
        matches!(self, Provider::DeepSeek)
    }

    fn uses_auth_header(self) -> bool {
        matches!(self, Provider::DeepSeek | Provider::Custom)
    }

    fn is_ollama(self) -> bool {
        matches!(self, Provider::Ollama)
    }
}

#[derive(Clone, Copy)]
struct Signals {
    messages: Signal<Vec<Message>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    models: Signal<Vec<String>>,
    model: Signal<String>,
    provider: Signal<Provider>,
    attachment: Signal<Option<Attachment>>,
    images_enabled: Signal<bool>,
    status: Signal<String>,
    online: Signal<bool>,
}

#[derive(Clone)]
struct InitConfig {
    provider: Provider,
    base: String,
    model: String,
}

struct AppState {
    vdom: VirtualDom,
    applier: ServoDomApplier,
}

#[derive(Clone, Copy)]
enum FetchKind {
    ListModels(Provider),
    Chat(Provider),
}

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
    static SIGNALS: RefCell<Option<Signals>> = const { RefCell::new(None) };
    static PENDING: RefCell<HashMap<u32, FetchKind>> = RefCell::new(HashMap::new());
    static INIT: RefCell<Option<InitConfig>> = const { RefCell::new(None) };
}

fn signals() -> Option<Signals> {
    SIGNALS.with(|signals| *signals.borrow())
}

fn set_error(message: impl Into<String>) {
    if let Some(mut signals) = signals() {
        signals.error.set(Some(message.into()));
    }
}

fn render_now() {
    STATE.with(|state| {
        if let Some(AppState { vdom, applier }) = state.borrow_mut().as_mut() {
            vdom.render_immediate(applier);
        }
    });
}

fn scroll_to_bottom() {
    if let Some(anchor) = document::query_selector("#scroll-anchor") {
        anchor.scroll_into_view();
    }
}

fn input_value(id: &str) -> String {
    document::get_element_by_id(id)
        .ok()
        .map(|element| element.get_property("value"))
        .unwrap_or_default()
}

fn set_input_value(id: &str, value: &str) {
    if let Ok(element) = document::get_element_by_id(id) {
        element.set_property("value", value);
    }
}

/// The Dioxus component.  State lives in signals; the DOM is patched in place
/// by [`ServoDomApplier`].
fn ChatApp() -> dioxus_core::Element {
    let init = INIT.with(|slot| slot.borrow().clone()).unwrap_or_else(|| InitConfig {
        provider: Provider::Ollama,
        base: "http://127.0.0.1:11434".to_string(),
        model: "qwen3.5-27b-q4:latest".to_string(),
    });

    let messages = use_signal(Vec::<Message>::new);
    let loading = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let models = use_signal(Vec::<String>::new);
    let model = use_signal(|| init.model.clone());
    let provider = use_signal(|| init.provider);
    let attachment = use_signal(|| None::<Attachment>);
    let images_enabled = use_signal(|| true);
    let initial_status = if init.provider == Provider::DeepSeek {
        "Enter your API key".to_string()
    } else {
        "Connecting…".to_string()
    };
    let status = use_signal(|| initial_status);
    let online = use_signal(|| false);

    SIGNALS.with(|slot| {
        *slot.borrow_mut() = Some(Signals {
            messages,
            loading,
            error,
            models,
            model,
            provider,
            attachment,
            images_enabled,
            status,
            online,
        });
    });

    let message_list = messages.read().clone();
    let model_list = models.read().clone();
    let selected_model = model.read().clone();
    let selected_provider = *provider.read();
    let provider_label = selected_provider.label().to_string();
    let pending_attachment = attachment.read().clone();
    let is_loading = *loading.read();
    let status_text = status.read().clone();
    let is_online = *online.read();
    let error_text = error.read().clone();

    let suggestions = [
        "Explain how Servo executes this Rust chat app without JavaScript.",
        "Write a Rust function that parses HTTP/1.1 chunked responses.",
        "Compare the WebAssembly Component Model with wasm-bindgen.",
        "Summarize the design of a native WebAssembly DOM binding.",
    ];

    rsx! {
        div { class: "flex h-screen w-full overflow-hidden bg-slate-950 text-slate-100",
            aside { class: "flex flex-col w-72 shrink-0 gap-4 p-5 bg-slate-900 border-r border-slate-700 overflow-y-auto",
                div { class: "flex items-center gap-3",
                    div { class: "flex items-center justify-center w-9 h-9 rounded-xl bg-indigo-600 text-white font-bold text-lg", "S" }
                    div {
                        div { class: "text-sm font-bold tracking-tight", "Servo Wasm Chat" }
                        div { class: "text-xs text-slate-400 font-mono", "wasm32-unknown-unknown" }
                    }
                }

                div { class: "rounded-xl border border-slate-700 bg-slate-800 p-3 flex flex-col gap-2",
                    div { class: "text-xs font-semibold uppercase tracking-wide text-slate-500", "Provider" }
                    div { class: "grid grid-cols-2 gap-1.5",
                        for provider_option in PROVIDERS.iter() {
                            {
                                let candidate = *provider_option;
                                let label = candidate.label();
                                let active = selected_provider == candidate;
                                rsx! {
                                    button {
                                        class: if active {
                                            "px-2 py-1.5 rounded-lg border bg-indigo-900 border-indigo-700 text-indigo-100 text-xs font-semibold"
                                        } else {
                                            "px-2 py-1.5 rounded-lg border bg-slate-900 border-slate-700 text-slate-400 text-xs font-semibold hover:text-slate-100 transition-colors"
                                        },
                                        onclick: move |_| select_provider(candidate),
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }

                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-400", "Base URL" }
                        input {
                            id: "base-url",
                            class: "w-full px-2 py-1.5 rounded-lg border border-slate-700 bg-slate-900 text-xs text-slate-200 font-mono outline-none focus:border-indigo-500",
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-400", "Model" }
                        input {
                            id: "model-input",
                            class: "w-full px-2 py-1.5 rounded-lg border border-slate-700 bg-slate-900 text-xs text-slate-200 font-mono outline-none focus:border-indigo-500",
                        }
                    }
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs text-slate-400", "API key" }
                        input {
                            id: "api-key",
                            r#type: "password",
                            placeholder: "sk-…",
                            class: "w-full px-2 py-1.5 rounded-lg border border-slate-700 bg-slate-900 text-xs text-slate-200 font-mono outline-none focus:border-indigo-500",
                        }
                    }
                    button {
                        class: "px-2 py-1.5 rounded-lg border border-slate-700 text-xs text-slate-300 hover:bg-slate-700 transition-colors",
                        onclick: move |_| refresh_models(),
                        "Refresh models"
                    }
                    div { class: "text-xs leading-relaxed text-slate-500",
                        "Any OpenAI-compatible base URL works: DeepSeek, llama.cpp, boat.dev, OpenRouter…"
                    }
                }

                div {
                    div { class: "text-xs font-semibold uppercase tracking-wide text-slate-500 mb-2", "Models" }
                    div { class: "flex flex-col gap-1.5 max-h-48 overflow-y-auto",
                        if model_list.is_empty() {
                            div { class: "text-xs text-slate-400 leading-relaxed",
                                if selected_provider.requires_key() {
                                    "Enter an API key, then refresh."
                                } else {
                                    "Refresh to discover models."
                                }
                            }
                        }
                        for name in model_list.iter() {
                            {
                                let candidate = name.clone();
                                let active = *name == selected_model;
                                rsx! {
                                    button {
                                        class: if active {
                                            "flex items-center justify-between w-full px-3 py-2 rounded-lg border bg-indigo-900 border-indigo-700 text-indigo-100 text-xs font-mono"
                                        } else {
                                            "flex items-center justify-between w-full px-3 py-2 rounded-lg border bg-transparent border-transparent text-slate-400 text-xs font-mono hover:bg-slate-800 hover:text-slate-100 transition-colors"
                                        },
                                        onclick: move |_| select_model(&candidate),
                                        span { "{name}" }
                                        span { class: "w-1.5 h-1.5 rounded-full bg-green-500 shrink-0" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "flex-1" }

                div { class: "rounded-xl border border-slate-700 bg-slate-800 p-4",
                    div { class: "text-xs font-bold mb-1", "Native component" }
                    div { class: "text-xs text-slate-400 leading-relaxed",
                        "Dioxus 0.6 virtual DOM with Ollama, llama.cpp and OpenAI-compatible APIs. Tailwind CSS is compiled by a second component."
                    }
                }

                div { class: "flex items-center gap-2 text-xs text-slate-400",
                    div { class: if is_online { "w-1.5 h-1.5 rounded-full bg-green-500" } else { "w-1.5 h-1.5 rounded-full bg-amber-500" } }
                    span { "{status_text}" }
                }
            }

            section { class: "flex-1 flex flex-col min-w-0",
                header { class: "h-16 shrink-0 flex items-center justify-between px-6 border-b border-slate-800",
                    div {
                        div { class: "text-sm font-bold", "Conversation" }
                        div { class: "text-xs text-slate-400 font-mono", "{selected_model} · {provider_label}" }
                    }
                    button {
                        class: "px-3 py-1.5 rounded-lg border border-slate-700 text-xs font-semibold text-slate-400 hover:text-slate-100 hover:bg-slate-800 transition-colors",
                        onclick: move |_| clear_chat(),
                        "New chat"
                    }
                }

                main { class: "flex-1 overflow-y-auto px-6 py-7",
                    div { class: "max-w-3xl mx-auto w-full flex flex-col gap-5",
                        if message_list.is_empty() {
                            div { class: "max-w-2xl mx-auto mt-12 text-center",
                                div { class: "flex items-center justify-center w-16 h-16 mx-auto mb-5 rounded-2xl bg-indigo-600 text-white text-2xl font-bold shadow-xl", "S" }
                                h1 { class: "text-3xl font-bold tracking-tight mb-2",
                                    span { class: "text-indigo-400", "Servo" }
                                    " Wasm Chat"
                                }
                                p { class: "text-sm text-slate-400 max-w-xl mx-auto mb-2 leading-relaxed",
                                    "A pure-Rust chat client running as a native WebAssembly component. No JavaScript, no wasm-bindgen, no glue code."
                                }
                                p { class: "text-xs text-slate-500 max-w-xl mx-auto mb-6 font-mono",
                                    "Hosted: choose DeepSeek and enter your API key. Local: Ollama or llama.cpp."
                                }
                                div { class: "grid grid-cols-2 gap-3 text-left",
                                    for suggestion in suggestions.iter() {
                                        {
                                            let prompt = suggestion.to_string();
                                            rsx! {
                                                button {
                                                    class: "rounded-xl border border-slate-700 bg-slate-900 px-4 py-3 text-xs text-slate-300 text-left hover:bg-slate-800 hover:border-indigo-700 transition-colors",
                                                    onclick: move |_| send_prompt(&prompt),
                                                    "{suggestion}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        for (index, message) in message_list.iter().enumerate() {
                            {
                                let is_user = message.role == Role::User;
                                let text = message.text.clone();
                                let image = message.image_data_url.clone();
                                let avatar = if is_user { "YOU" } else { "S" };
                                let meta = if is_user { "you" } else { "servo-wasm" };
                                let key = index;
                                let row_class = if is_user { "flex flex-row-reverse gap-3 items-start" } else { "flex gap-3 items-start" };
                                let avatar_class = if is_user {
                                    "flex items-center justify-center w-7 h-7 shrink-0 rounded-lg bg-blue-600 text-white text-xs font-bold"
                                } else {
                                    "flex items-center justify-center w-7 h-7 shrink-0 rounded-lg bg-indigo-600 text-white text-xs font-bold"
                                };
                                let body_class = if is_user { "flex flex-col gap-1 max-w-2xl min-w-0 items-end" } else { "flex flex-col gap-1 max-w-2xl min-w-0" };
                                let bubble_class = if is_user {
                                    "px-4 py-2.5 rounded-2xl border bg-blue-900 border-blue-700 text-sm whitespace-pre-wrap break-words"
                                } else {
                                    "px-4 py-2.5 rounded-2xl border bg-slate-800 border-slate-700 text-sm whitespace-pre-wrap break-words"
                                };
                                rsx! {
                                    div {
                                        key: "{key}",
                                        class: "{row_class}",
                                        div { class: "{avatar_class}", "{avatar}" }
                                        div { class: "{body_class}",
                                            if let Some(source) = image.clone() {
                                                img { class: "max-w-xs max-h-60 rounded-xl object-cover border border-slate-700", src: "{source}" }
                                            }
                                            if !text.is_empty() {
                                                div { class: "{bubble_class}", "{text}" }
                                            }
                                            div { class: "text-xs text-slate-500 font-mono px-1", "{meta}" }
                                        }
                                    }
                                }
                            }
                        }

                        if is_loading {
                            div { class: "flex gap-3 items-start",
                                div { class: "flex items-center justify-center w-7 h-7 shrink-0 rounded-lg bg-indigo-600 text-white text-xs font-bold", "S" }
                                div { class: "flex flex-col gap-1 max-w-2xl min-w-0",
                                    div { class: "servo-typing flex items-center gap-1 px-4 py-3 rounded-2xl border bg-slate-800 border-slate-700",
                                        span {}
                                        span {}
                                        span {}
                                    }
                                }
                            }
                        }

                        if let Some(message) = error_text.clone() {
                            div { class: "flex items-center justify-between gap-3 px-4 py-3 rounded-xl border border-red-600 bg-red-900 text-red-100 text-xs",
                                span { "{message}" }
                                button {
                                    class: "text-lg leading-none px-1",
                                    onclick: move |_| dismiss_error(),
                                    "×"
                                }
                            }
                        }

                        div { id: "scroll-anchor", class: "h-px" }
                    }
                }

                footer { class: "px-6 pb-6 pt-3",
                    div { class: "max-w-3xl mx-auto rounded-2xl border border-slate-700 bg-slate-900 shadow-xl px-4 py-3 focus-within:border-indigo-500 transition-colors",
                        if let Some(attached) = pending_attachment.clone() {
                            div { class: "flex items-center gap-3 mb-2 p-2 rounded-xl border border-slate-700 bg-slate-800",
                                img { class: "w-10 h-10 rounded-lg object-cover", src: "{attached.data_url}" }
                                div { class: "min-w-0",
                                    div { class: "text-xs text-slate-100 truncate max-w-xs", "{attached.name}" }
                                    div { class: "text-xs text-slate-500 font-mono", "{attached.mime}" }
                                }
                                button {
                                    class: "ml-auto px-2 py-1 rounded-lg border border-slate-700 text-xs text-slate-400 hover:text-red-400 transition-colors",
                                    onclick: move |_| remove_attachment(),
                                    "Remove"
                                }
                            }
                        }

                        div { class: "flex items-end gap-3",
                            textarea {
                                id: "composer-input",
                                class: "w-full min-h-10 max-h-40 resize-none outline-none bg-transparent text-sm text-slate-100",
                                placeholder: "Message Servo Wasm Chat…",
                                onkeydown: move |event| {
                                    if event.key().to_string() == "Enter"
                                        && !event.modifiers().contains(Modifiers::SHIFT)
                                    {
                                        send_now();
                                    }
                                },
                            }
                        }

                        div { class: "flex items-center gap-3 pt-3 mt-3 border-t border-slate-800",
                            input {
                                id: "file-input",
                                class: "max-w-xs text-xs text-slate-400",
                                r#type: "file",
                                accept: "image/*",
                                onchange: move |_| attach_from_input(),
                            }
                            button {
                                class: "ml-auto px-4 py-2 rounded-lg bg-indigo-600 text-white text-xs font-bold hover:bg-indigo-500 disabled:opacity-50 transition-colors",
                                disabled: is_loading,
                                onclick: move |_| send_now(),
                                "Send"
                            }
                        }
                    }
                    div { class: "max-w-3xl mx-auto mt-2 text-xs text-slate-500 text-center font-mono",
                        "Enter to send · Shift+Enter for a newline · native DOM, zero JS"
                    }
                }
            }
        }
    }
}

pub fn run() {
    console::log("[chat] run() starting");

    // The hosted page defaults to DeepSeek; a locally served page defaults to
    // the local Ollama server.  The sidebar always lets the user override both.
    let page_url = document::get_url();
    console::log(&format!("[chat] page_url={page_url}"));
    let hosted = page_url.contains("tannal.github.io");
    let (provider, base, model) = if hosted {
        (
            Provider::DeepSeek,
            "https://api.deepseek.com/v1".to_string(),
            "deepseek-chat".to_string(),
        )
    } else {
        (
            Provider::Ollama,
            "http://127.0.0.1:11434".to_string(),
            "qwen3.5-27b-q4:latest".to_string(),
        )
    };
    let config = InitConfig {
        provider,
        base,
        model,
    };
    INIT.with(|slot| *slot.borrow_mut() = Some(config.clone()));

    // The app is pure WebAssembly: the page only contains script tags.  Mount
    // into `#app` when a host page provides one, otherwise use the body and
    // leave the script elements untouched.
    let root = match document::get_element_by_id("app") {
        Ok(root) => {
            root.set_text_content("");
            root
        },
        Err(_) => document::get_body().expect("document has no body"),
    };
    let mut vdom = VirtualDom::new(ChatApp);
    let mut applier = ServoDomApplier::new(root);
    vdom.rebuild(&mut applier);
    console::log("[chat] VirtualDom rebuilt and mounted");

    set_input_value("base-url", &config.base);
    set_input_value("model-input", &config.model);
    set_input_value("api-key", "");

    STATE.with(|state| {
        *state.borrow_mut() = Some(AppState { vdom, applier });
    });

    if provider == Provider::DeepSeek {
        if let Some(mut signals) = signals() {
            signals.status.set("Enter your DeepSeek API key".to_string());
        }
    } else {
        refresh_models();
    }
    scroll_to_bottom();
}

/// Dispatch a native DOM event into the Dioxus runtime.
pub fn on_event(event: &EventData) {
    let Some(element_id) = event
        .handler_id
        .strip_prefix("dioxus-")
        .and_then(|id| id.parse::<usize>().ok())
    else {
        return;
    };

    let payload: Rc<dyn Any> = match event.event_type.as_str() {
        "keydown" => {
            let mut keyboard = ServoKeyboardData::from_key_string(&event.key);
            if event.shift {
                keyboard.modifiers = Modifiers::SHIFT;
            }
            Rc::new(PlatformEventData::new(Box::new(keyboard)))
        },
        "input" | "change" => Rc::new(PlatformEventData::new(Box::new(ServoFormData {
            value: event.value.clone(),
        }))),
        _ => Rc::new(PlatformEventData::new(Box::new(ServoMouseData {
            client_x: event.client_x,
            client_y: event.client_y,
            button: Some(dioxus_html::input_data::MouseButton::Primary),
        }))),
    };

    let event_name = event.event_type.clone();
    STATE.with(|state| {
        if let Some(AppState { vdom, applier }) = state.borrow_mut().as_mut() {
            let event = Event::new(payload, true);
            vdom.runtime()
                .handle_event(&event_name, event, ElementId(element_id));
            vdom.render_immediate(applier);
        }
    });
    scroll_to_bottom();
}

/// Completion callback for `servo:dom/fetch`.
pub fn on_fetch(
    request_id: u32,
    ok: bool,
    status: u16,
    body: Vec<u8>,
    error: Option<String>,
) {
    let kind = PENDING.with(|pending| pending.borrow_mut().remove(&request_id));
    let Some(kind) = kind else {
        return;
    };

    if !ok {
        if let Some(mut signals) = signals() {
            signals.loading.set(false);
            signals.online.set(false);
            signals.status.set("Connection failed".to_string());
        }
        set_error(error.unwrap_or_else(|| "network request failed".to_string()));
        render_now();
        return;
    }

    if status != 200 {
        let detail = describe_error(&body);
        if let Some(mut signals) = signals() {
            signals.loading.set(false);
            signals.online.set(true);
            let lower = detail.to_ascii_lowercase();
            let image_related = matches!(kind, FetchKind::Chat(_)) && lower.contains("image");
            if image_related {
                signals.images_enabled.set(false);
                signals.status.set("Model does not accept images".to_string());
                signals.error.set(Some(
                    "This endpoint/model does not support image input. The attachment stays in the transcript but is not sent. Choose a vision-capable model or remove the attachment."
                        .to_string(),
                ));
            } else {
                signals.status.set("API error".to_string());
                signals.error.set(Some(format!("HTTP {status}: {detail}")));
            }
        }
        render_now();
        return;
    }

    match kind {
        FetchKind::ListModels(provider) => handle_models(provider, &body),
        FetchKind::Chat(provider) => handle_chat(provider, &body),
    }
    render_now();
    scroll_to_bottom();
}

fn handle_models(provider: Provider, body: &[u8]) {
    #[derive(Deserialize)]
    struct Tags {
        #[serde(default)]
        models: Vec<Tag>,
    }
    #[derive(Deserialize)]
    struct Tag {
        name: String,
    }
    #[derive(Deserialize)]
    struct OpenAiModels {
        #[serde(default)]
        data: Vec<OpenAiModel>,
    }
    #[derive(Deserialize)]
    struct OpenAiModel {
        id: String,
    }

    let parsed: Result<Vec<String>, String> = match provider {
        Provider::Ollama => serde_json::from_slice::<Tags>(body)
            .map(|tags| tags.models.into_iter().map(|tag| tag.name).collect())
            .map_err(|error| error.to_string()),
        _ => serde_json::from_slice::<OpenAiModels>(body)
            .map(|models| {
                models
                    .data
                    .into_iter()
                    .map(|model| model.id)
                    .collect()
            })
            .map_err(|error| error.to_string()),
    };

    let Some(mut signals) = signals() else {
        return;
    };

    match parsed {
        Ok(mut names) => {
            names.sort();
            names.dedup();
            if names.is_empty() {
                signals.status.set(format!("{} online · no models", provider.label()));
                signals.online.set(true);
                return;
            }
            let current = input_value("model-input");
            let chosen = if names.iter().any(|name| name == &current) {
                current
            } else if provider == Provider::Ollama {
                names
                    .iter()
                    .find(|name| name.to_ascii_lowercase().contains("qwen"))
                    .cloned()
                    .unwrap_or_else(|| names[0].clone())
            } else if provider == Provider::DeepSeek {
                names
                    .iter()
                    .find(|name| name.as_str() == "deepseek-chat")
                    .cloned()
                    .unwrap_or_else(|| names[0].clone())
            } else {
                names[0].clone()
            };
            set_input_value("model-input", &chosen);
            signals.model.set(chosen);
            signals.status.set(format!(
                "{} online · {} models",
                provider.label(),
                names.len()
            ));
            signals.online.set(true);
            signals.models.set(names);
        },
        Err(error) => {
            signals.status.set("Model discovery failed".to_string());
            set_error(format!("could not parse model list: {error}"));
        },
    }
}

fn handle_chat(provider: Provider, body: &[u8]) {
    let Some(mut signals) = signals() else {
        return;
    };
    signals.loading.set(false);
    signals.online.set(true);
    signals.status.set(format!("{} online", provider.label()));

    let content = match provider {
        Provider::Ollama => {
            #[derive(Deserialize)]
            struct ChatResponse {
                #[serde(default)]
                message: Option<ChatMessage>,
                #[serde(default)]
                error: Option<String>,
            }
            #[derive(Deserialize)]
            struct ChatMessage {
                #[serde(default)]
                content: String,
            }
            match serde_json::from_slice::<ChatResponse>(body) {
                Ok(response) => {
                    if let Some(error) = response.error {
                        set_error(error);
                        return;
                    }
                    response.message.map(|message| message.content)
                },
                Err(error) => {
                    set_error(format!("invalid chat response: {error}"));
                    return;
                },
            }
        },
        _ => match serde_json::from_slice::<Value>(body) {
            Ok(value) => {
                if let Some(error) = value.get("error") {
                    set_error(describe_value_error(error));
                    return;
                }
                value
                    .pointer("/choices/0/message/content")
                    .map(content_text)
            },
            Err(error) => {
                set_error(format!("invalid chat response: {error}"));
                return;
            },
        },
    };

    let Some(content) = content else {
        set_error("the endpoint returned an empty response");
        return;
    };
    let content = clean_reply(&content);
    if content.is_empty() {
        set_error("the endpoint returned an empty response");
        return;
    }

    signals.messages.write().push(Message {
        role: Role::Assistant,
        text: content,
        image_data_url: None,
        image_base64: None,
    });
}

fn content_text(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    if let Some(parts) = value.as_array() {
        return parts
            .iter()
            .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
            .collect::<Vec<_>>()
            .join("");
    }
    String::new()
}

fn describe_value_error(value: &Value) -> String {
    if let Some(message) = value.get("message").and_then(|message| message.as_str()) {
        return message.to_string();
    }
    if let Some(message) = value.as_str() {
        return message.to_string();
    }
    value.to_string()
}

fn describe_error(body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        if let Some(error) = value.get("error") {
            return describe_value_error(error);
        }
        if let Some(message) = value.get("message").and_then(|message| message.as_str()) {
            return message.to_string();
        }
    }
    let text = String::from_utf8_lossy(body).trim().to_string();
    text.chars().take(500).collect()
}

pub fn refresh_models() {
    let Some(mut signals) = signals() else {
        return;
    };
    let provider = *signals.provider.read();
    let base = input_value("base-url").trim().to_string();
    let key = input_value("api-key").trim().to_string();
    if base.is_empty() {
        set_error("enter a base URL");
        return;
    }
    if provider.requires_key() && key.is_empty() {
        signals.status.set("Enter an API key".to_string());
        set_error("enter an API key to list models");
        return;
    }

    let (url, headers) = if provider.is_ollama() {
        (
            format!("{}/api/tags", base.trim_end_matches('/')),
            Vec::new(),
        )
    } else {
        (
            openai_url(&base, "models"),
            provider_model_headers(provider, &key),
        )
    };

    signals.status.set("Loading models…".to_string());
    let request_id = fetch::fetch(&url, "GET", &headers, None);
    PENDING.with(|pending| {
        pending
            .borrow_mut()
            .insert(request_id, FetchKind::ListModels(provider));
    });
}

fn select_provider(provider: Provider) {
    let Some(mut signals) = signals() else {
        return;
    };
    let (base, model) = provider.defaults();
    signals.provider.set(provider);
    signals.model.set(model.to_string());
    signals.images_enabled.set(true);
    signals.models.write().clear();
    signals.error.set(None);
    signals.online.set(false);
    set_input_value("base-url", base);
    set_input_value("model-input", model);

    if provider.requires_key() && input_value("api-key").trim().is_empty() {
        signals.status.set("Enter an API key".to_string());
    } else {
        signals.status.set("Connecting…".to_string());
        refresh_models();
    }
}

fn select_model(name: &str) {
    if let Some(mut signals) = signals() {
        signals.model.set(name.to_string());
        signals.images_enabled.set(true);
        set_input_value("model-input", name);
    }
}

fn clear_chat() {
    if let Some(mut signals) = signals() {
        signals.messages.write().clear();
        signals.error.set(None);
        signals.loading.set(false);
        signals.attachment.set(None);
    }
}

fn dismiss_error() {
    if let Some(mut signals) = signals() {
        signals.error.set(None);
    }
}

fn remove_attachment() {
    if let Some(mut signals) = signals() {
        signals.attachment.set(None);
    }
}

fn read_composer() -> String {
    document::get_element_by_id("composer-input")
        .ok()
        .map(|element| element.get_property("value"))
        .unwrap_or_default()
}

fn clear_composer() {
    if let Ok(element) = document::get_element_by_id("composer-input") {
        element.set_property("value", "");
        element.focus();
    }
}

pub fn send_now() {
    let text = read_composer().trim().to_string();
    let attachment = signals().and_then(|signals| signals.attachment.read().clone());
    if text.is_empty() && attachment.is_none() {
        return;
    }
    clear_composer();
    send_message(text, attachment);
}

fn send_prompt(prompt: &str) {
    send_message(prompt.trim().to_string(), None);
}

fn send_message(text: String, attachment: Option<Attachment>) {
    let Some(mut signals) = signals() else {
        return;
    };
    let provider = *signals.provider.read();
    let base = input_value("base-url").trim().to_string();
    let model = input_value("model-input").trim().to_string();
    let key = input_value("api-key").trim().to_string();

    if base.is_empty() {
        set_error("enter a base URL in the sidebar");
        return;
    }
    if model.is_empty() {
        set_error("enter a model name in the sidebar");
        return;
    }
    if provider.requires_key() && key.is_empty() {
        signals.status.set("Enter an API key".to_string());
        set_error("enter an API key for this provider");
        return;
    }

    signals.model.set(model.clone());
    let include_images = *signals.images_enabled.read();
    let (image_data_url, image_base64) = match &attachment {
        Some(attachment) => (
            Some(attachment.data_url.clone()),
            Some(attachment.base64.clone()),
        ),
        None => (None, None),
    };

    signals.messages.write().push(Message {
        role: Role::User,
        text: text.clone(),
        image_data_url,
        image_base64,
    });
    signals.attachment.set(None);
    signals.error.set(None);
    signals.loading.set(true);
    signals.status.set(format!("Waiting for {}…", provider.label()));

    let history = signals.messages.read().clone();
    let request = if provider.is_ollama() {
        let payload = build_ollama_request(&model, &history, include_images);
        serde_json::to_vec(&payload).map(|body| {
            (
                format!("{}/api/chat", base.trim_end_matches('/')),
                json_headers(""),
                body,
            )
        })
    } else {
        let payload = build_openai_request(&model, &history, include_images);
        serde_json::to_vec(&payload).map(|body| {
            (
                openai_url(&base, "chat/completions"),
                provider_json_headers(provider, &key),
                body,
            )
        })
    };

    let (url, headers, body) = match request {
        Ok(request) => request,
        Err(error) => {
            signals.loading.set(false);
            set_error(format!("could not encode request: {error}"));
            return;
        },
    };

    let request_id = fetch::fetch(&url, "POST", &headers, Some(&body));
    PENDING.with(|pending| {
        pending
            .borrow_mut()
            .insert(request_id, FetchKind::Chat(provider));
    });
    scroll_to_bottom();
}

fn openai_url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/{path}")
    } else {
        format!("{base}/v1/{path}")
    }
}

fn json_headers(key: &str) -> Vec<(String, String)> {
    let mut headers = vec![(
        "Content-Type".to_string(),
        "application/json".to_string(),
    )];
    if !key.is_empty() {
        headers.push((
            "Authorization".to_string(),
            format!("Bearer {key}"),
        ));
    }
    headers
}

fn provider_json_headers(provider: Provider, key: &str) -> Vec<(String, String)> {
    if provider.uses_auth_header() {
        json_headers(key)
    } else {
        json_headers("")
    }
}

fn provider_model_headers(provider: Provider, key: &str) -> Vec<(String, String)> {
    if provider.uses_auth_header() && !key.is_empty() {
        vec![(
            "Authorization".to_string(),
            format!("Bearer {key}"),
        )]
    } else {
        Vec::new()
    }
}

fn attach_from_input() {
    let Some(mut signals) = signals() else {
        return;
    };
    let Ok(input) = document::get_element_by_id("file-input") else {
        return;
    };
    if input.file_count() == 0 {
        return;
    }
    let bytes = match input.file_bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => {
            set_error(format!("could not read selected file: {error}"));
            return;
        },
    };
    if bytes.is_empty() {
        set_error("the selected file is empty");
        return;
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        set_error(format!(
            "image is too large ({} MB, maximum {} MB)",
            bytes.len() / (1024 * 1024),
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
        return;
    }

    let name = input.file_name(0);
    let mime = input.file_type(0);
    let mime = if mime.is_empty() {
        "image/png".to_string()
    } else {
        mime
    };
    let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:{mime};base64,{base64}");
    signals.images_enabled.set(true);
    signals.attachment.set(Some(Attachment {
        name,
        mime,
        base64,
        data_url,
    }));
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatRequestMessage<'a>>,
    stream: bool,
    /// Qwen3-family models emit a separate thinking stream unless this is off.
    think: bool,
}

#[derive(Serialize)]
struct ChatRequestMessage<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<&'a str>>,
}

fn build_ollama_request<'a>(
    model: &'a str,
    history: &'a [Message],
    include_images: bool,
) -> ChatRequest<'a> {
    let messages = history
        .iter()
        .map(|message| {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            let images = if include_images {
                message
                    .image_base64
                    .as_deref()
                    .map(|image| vec![image])
            } else {
                None
            };
            ChatRequestMessage {
                role,
                content: &message.text,
                images,
            }
        })
        .collect();
    ChatRequest {
        model,
        messages,
        stream: false,
        think: false,
    }
}

fn build_openai_request(
    model: &str,
    history: &[Message],
    include_images: bool,
) -> Value {
    let messages: Vec<Value> = history
        .iter()
        .map(|message| {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            if include_images {
                if let Some(image) = &message.image_data_url {
                    return json!({
                        "role": role,
                        "content": [
                            {"type": "text", "text": &message.text},
                            {"type": "image_url", "image_url": {"url": image}}
                        ]
                    });
                }
            }
            json!({"role": role, "content": &message.text})
        })
        .collect();
    json!({
        "model": model,
        "messages": messages,
        "stream": false
    })
}

fn clean_reply(content: &str) -> String {
    let content = match content.rfind("</think>") {
        Some(index) => &content[index + "</think>".len()..],
        None => content,
    };
    content
        .replace("<|endoftext|>", "")
        .replace("<|im_start|>", "")
        .replace("<|im_end|>", "")
        .trim()
        .to_string()
}
