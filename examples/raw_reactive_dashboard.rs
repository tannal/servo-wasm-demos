use crate::bindings::servo::dom::{console, document};
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering::SeqCst};

static METRIC_VAL: AtomicI32 = AtomicI32::new(42);
static LOG_COUNT: AtomicU32 = AtomicU32::new(0);

fn update_metric(val: i32) {
    if let Ok(display) = document::get_element_by_id("metric-display") {
        display.set_text_content(&format!("{} ops/sec", val));
    }
}

fn add_telemetry(msg: &str) {
    if let Ok(list) = document::get_element_by_id("telemetry-log") {
        if let Ok(li) = document::create_element("li") {
            li.set_attribute("class", "log-item");
            li.set_text_content(msg);
            list.append_child(&li);
        }
    }
}

pub fn run() {
    console::log("[Raw WIT]: Mounting telemetry dashboard...");
    let body = document::get_body().expect("body");

    let card = document::create_element("div").expect("card");
    card.set_attribute("class", "wasm-card");

    let h1 = document::create_element("h1").expect("h1");
    h1.set_text_content("⚡ High-Frequency Wasm Telemetry Dashboard");
    card.append_child(&h1);

    let p = document::create_element("p").expect("p");
    p.set_attribute("class", "subtitle");
    p.set_text_content("Zero-JS Native Component Model DOM streaming");
    card.append_child(&p);

    // Metric display
    let stat_box = document::create_element("div").expect("div");
    stat_box.set_attribute("class", "counter-section");

    let badge = document::create_element("div").expect("div");
    badge.set_attribute("id", "metric-display");
    badge.set_attribute("class", "counter-badge");
    badge.set_text_content("42 ops/sec");
    stat_box.append_child(&badge);

    for (id, label, class) in [
        ("btn-throttle", "📉 Throttle (-10)", "btn btn-secondary"),
        ("btn-reset", "🔄 Calibrate", "btn btn-secondary"),
        ("btn-boost", "🚀 Boost (+10)", "btn btn-primary"),
    ] {
        let btn = document::create_element("button").expect("button");
        btn.set_attribute("id", id);
        btn.set_attribute("class", class);
        btn.set_text_content(label);
        btn.add_event_listener("click", id);
        stat_box.append_child(&btn);
    }
    card.append_child(&stat_box);

    // Telemetry log list
    let log_box = document::create_element("div").expect("div");
    log_box.set_attribute("class", "log-section");
    let title = document::create_element("h3").expect("h3");
    title.set_text_content("📡 Real-time Host Event Stream");
    log_box.append_child(&title);

    let list = document::create_element("ul").expect("ul");
    list.set_attribute("id", "telemetry-log");
    log_box.append_child(&list);
    card.append_child(&log_box);

    body.append_child(&card);
}

pub fn on_event(handler_id: &str) {
    match handler_id {
        "btn-boost" => {
            let val = METRIC_VAL.fetch_add(10, SeqCst) + 10;
            update_metric(val);
            add_telemetry(&format!("Telemetry: Boosted to {} ops/sec", val));
        }
        "btn-throttle" => {
            let val = METRIC_VAL.fetch_sub(10, SeqCst) - 10;
            update_metric(val);
            add_telemetry(&format!("Telemetry: Throttled to {} ops/sec", val));
        }
        "btn-reset" => {
            METRIC_VAL.store(42, SeqCst);
            update_metric(42);
            add_telemetry("Telemetry: Recalibrated baseline (42 ops/sec)");
        }
        _ => {}
    }
}
