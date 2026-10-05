#[allow(warnings)]
mod bindings;
use bindings::Guest;
use bindings::servo::dom::{console, document};

struct Component;

impl Guest for Component {
    fn run() {
        console::log("Hello from First-Class WebAssembly in Servo!");

        // Manipulate the DOM directly from Wasm without JS!
        let body = document::get_body().expect("failed to get body");
        
        let h1 = document::create_element("h1").expect("failed to create h1");
        h1.set_text_content("🚀 Rendered natively by WebAssembly Component without JS!");
        
        let p = document::create_element("p").expect("failed to create p");
        p.set_text_content("This DOM node was instantiated via Wasm Component canonical ABI & WIT.");

        body.append_child(&h1);
        body.append_child(&p);

        console::log("DOM manipulation from Wasm completed successfully.");
    }
}

bindings::export!(Component with_types_in bindings);
