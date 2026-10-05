use crate::bindings::servo::dom::{console, document};

pub fn run() {
    console::log("[MathML Wasm]: Mounting MathML generator...");
    let body = document::get_body().expect("body");

    let card = document::create_element("div").expect("card");
    card.set_attribute("class", "wasm-card");

    let h1 = document::create_element("h1").expect("h1");
    h1.set_text_content("📐 Native MathML Formula Renderer");
    card.append_child(&h1);

    let p = document::create_element("p").expect("p");
    p.set_attribute("class", "subtitle");
    p.set_text_content("Generating dynamic MathML Core markup from Wasm directly into Servo's DOM");
    card.append_child(&p);

    let controls = document::create_element("div").expect("div");
    controls.set_attribute("class", "counter-section");

    for (id, label) in [
        ("btn-fraction", "Render Fraction (a / b)"),
        ("btn-quadratic", "Render Quadratic Formula"),
        ("btn-sum", "Render Summation ∑"),
    ] {
        let btn = document::create_element("button").expect("button");
        btn.set_attribute("id", id);
        btn.set_attribute("class", "btn btn-primary");
        btn.set_text_content(label);
        btn.add_event_listener("click", id);
        controls.append_child(&btn);
    }
    card.append_child(&controls);

    // MathML output container
    let math_container = document::create_element("div").expect("div");
    math_container.set_attribute("id", "math-output");
    math_container.set_attribute("style", "padding: 24px; background: #11111b; border-radius: 12px; margin-top: 16px; font-size: 1.8rem; text-align: center;");
    
    // Initial formula: x^2 + y^2
    render_fraction(&math_container);
    card.append_child(&math_container);

    body.append_child(&card);
}

fn render_fraction(container: &document::Element) {
    container.set_text_content("");
    // Build <math display="block"><mfrac><mi>x + y^2</mi><mi>k + 1</mi></mfrac></math>
    if let Ok(math) = document::create_element("math") {
        math.set_attribute("display", "block");
        if let Ok(mfrac) = document::create_element("mfrac") {
            if let Ok(num) = document::create_element("mi") {
                num.set_text_content("x + y²");
                mfrac.append_child(&num);
            }
            if let Ok(den) = document::create_element("mi") {
                den.set_text_content("k + 1");
                mfrac.append_child(&den);
            }
            math.append_child(&mfrac);
        }
        container.append_child(&math);
    }
}

pub fn on_event(handler_id: &str) {
    if let Ok(container) = document::get_element_by_id("math-output") {
        match handler_id {
            "btn-fraction" => render_fraction(&container),
            "btn-quadratic" => {
                container.set_text_content("x = (-b ± √(b² - 4ac)) / 2a");
            }
            "btn-sum" => {
                container.set_text_content("∑ P(i, j)");
            }
            _ => {}
        }
    }
}
