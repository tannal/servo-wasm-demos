use crate::bindings::servo::dom::{console, document};
use core::sync::atomic::{AtomicU32, Ordering::SeqCst};

static LIKES_SERVO: AtomicU32 = AtomicU32::new(128);
static LIKES_WASM: AtomicU32 = AtomicU32::new(256);

pub fn run() {
    console::log("[Zulip Feed]: Rendering message stream directly via WIT DOM...");
    let body = document::get_body().expect("body");

    let card = document::create_element("div").expect("card");
    card.set_attribute("class", "wasm-card");

    // Header
    let h1 = document::create_element("h1").expect("h1");
    h1.set_text_content("💬 Zulip Live Feed: #servo-devel");
    card.append_child(&h1);

    let p = document::create_element("p").expect("p");
    p.set_attribute("class", "subtitle");
    p.set_text_content("Ultra-low-latency message feed rendered via WebAssembly Component Model");
    card.append_child(&p);

    // Stream Controls
    let controls = document::create_element("div").expect("div");
    controls.set_attribute("class", "counter-section");

    let btn_post = document::create_element("button").expect("button");
    btn_post.set_attribute("id", "btn-new-message");
    btn_post.set_attribute("class", "btn btn-primary");
    btn_post.set_text_content("✉️ Post Live Message");
    btn_post.add_event_listener("click", "btn-new-message");
    controls.append_child(&btn_post);

    card.append_child(&controls);

    // Feed Message Container
    let feed = document::create_element("div").expect("div");
    feed.set_attribute("id", "message-feed");

    // Pre-populate with realistic messages
    feed.append_child(&create_message_card(
        "Martin Robinson",
        "@mrobinson",
        "10:42 AM",
        "Excited to see the first-class WebAssembly Component Model working in Servo without any JS glue code! 🎉",
        "btn-like-servo",
        128
    ));

    feed.append_child(&create_message_card(
        "Luke Wagner",
        "@luke",
        "10:45 AM",
        "This is exactly what the W3C WebAssembly CG envisioned for Issue #371. Language-agnostic web APIs are the future.",
        "btn-like-wasm",
        256
    ));

    card.append_child(&feed);
    body.append_child(&card);
}

fn create_message_card(
    author: &str,
    handle: &str,
    time: &str,
    content: &str,
    like_btn_id: &str,
    initial_likes: u32,
) -> document::Element {
    let msg = document::create_element("div").expect("div");
    msg.set_attribute("class", "task-card");
    msg.set_attribute("style", "margin-bottom: 14px; border-left: 4px solid #89b4fa; padding: 12px;");

    let header = document::create_element("div").expect("div");
    header.set_attribute("style", "font-size: 0.85rem; color: #a6adc8; margin-bottom: 6px; display: flex; justify-content: space-between;");
    header.set_text_content(&format!("👤 {} ({}) · {}", author, handle, time));
    msg.append_child(&header);

    let body_text = document::create_element("div").expect("div");
    body_text.set_attribute("style", "color: #cdd6f4; font-size: 0.95rem; line-height: 1.4;");
    body_text.set_text_content(content);
    msg.append_child(&body_text);

    let footer = document::create_element("div").expect("div");
    footer.set_attribute("style", "margin-top: 8px; display: flex; gap: 8px;");

    let like_btn = document::create_element("button").expect("button");
    like_btn.set_attribute("id", like_btn_id);
    like_btn.set_attribute("class", "btn btn-secondary");
    like_btn.set_attribute("style", "font-size: 0.8rem; padding: 4px 10px;");
    like_btn.set_text_content(&format!("❤️ Like ({})", initial_likes));
    like_btn.add_event_listener("click", like_btn_id);
    footer.append_child(&like_btn);

    msg.append_child(&footer);
    msg
}

pub fn on_event(handler_id: &str) {
    match handler_id {
        "btn-like-servo" => {
            let likes = LIKES_SERVO.fetch_add(1, SeqCst) + 1;
            if let Ok(btn) = document::get_element_by_id("btn-like-servo") {
                btn.set_text_content(&format!("❤️ Like ({})", likes));
            }
        }
        "btn-like-wasm" => {
            let likes = LIKES_WASM.fetch_add(1, SeqCst) + 1;
            if let Ok(btn) = document::get_element_by_id("btn-like-wasm") {
                btn.set_text_content(&format!("❤️ Like ({})", likes));
            }
        }
        "btn-new-message" => {
            if let Ok(feed) = document::get_element_by_id("message-feed") {
                let card = create_message_card(
                    "You (Intern @ Igalia)",
                    "@intern",
                    "Just now",
                    "Running native Dioxus and Zulip feeds on Servo via Wasmtime 49 with ZERO JavaScript!",
                    "btn-like-new",
                    1,
                );
                feed.append_child(&card);
            }
        }
        _ => {}
    }
}
