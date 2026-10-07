# Servo WebAssembly Component Model: Dioxus Native UI

A first-class WebAssembly Component Model runtime embedded inside the Servo browser engine. This project demonstrates running idiomatic [Dioxus 0.6](https://dioxuslabs.com/) applications directly within Servo's DOM pipeline with **zero JavaScript glue code** and **no `wasm-bindgen`**.

![Dioxus Todo in Servo](dixous-todo.png)

---

## Key Architectural Highlights

* **Zero-Glue DOM Integration**: Guest components interact with Servo's DOM through typed WebAssembly Interface Type (WIT) bindings defined in `servo_dom.wit`.
* **Full Dioxus 0.6 Reactivity**: Uses signals (`use_signal`), memoized computations (`use_memo`), dynamic search filtering, priority tags, and encapsulated event closures.
* **Bidirectional JS <-> Wasm Interoperability**:
  * **JS -> Wasm**: Page-level JavaScript relays real-time input events and batches data directly into the Dioxus VirtualDOM.
  * **Wasm -> JS**: Dioxus publishes reactive state metrics into DOM `data-*` attributes (`data-total`, `data-active`, `data-matches`), allowing external JS observers to track telemetry in real time.
---

## Directory Layout

```text
wasm_guest/
├── wit/
│   └── servo_dom.wit          # WIT contract between Servo host and guest components
├── src/
│   ├── bindings.rs            # Auto-generated WIT Component Model bindings
│   ├── dioxus_renderer.rs     # Dioxus WriteMutations -> Servo DOM applier
│   └── lib.rs                 # Guest entry point, event dispatcher, and exports
├── examples/
│   └── dioxus_todo.rs         # Reactive TodoMVC implementation (signals, filters, search)
├── test_wasm.html             # Host HTML document with JS <-> Wasm bridge console
├── Cargo.toml                 # Cargo manifest configured with cargo-component
└── README.md                  # This file
```

---

## Prerequisites

Install the standard WebAssembly target and the Bytecode Alliance Component Model build tools:

```bash
# 1. Add wasm32 compilation target
rustup target add wasm32-unknown-unknown

# 2. Install cargo-component (Bytecode Alliance Component Model tool)
cargo install cargo-component --locked

# 3. (Optional) Install wasm-tools for inspecting component interfaces
cargo install wasm-tools --locked
```

---

## Build Instructions

Build the guest component binary from the `wasm_guest/` directory:

```bash
# Compile the WebAssembly Component in release mode
cargo component build --release --target wasm32-unknown-unknown

# Copy the built artifact to the HTML root
cp target/wasm32-unknown-unknown/release/wasm_guest.wasm ./wasm_guest.wasm
```

To inspect the exported component interfaces and imports:
```bash
wasm-tools component wit wasm_guest.wasm
```

---

## Running in Servo

Servo loads and initializes the WebAssembly binary directly via the native `<script type="wasm-component">` tag:

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Servo Wasm Component Platform Demo</title>
  </head>
  <body>
    <!-- Container where the Wasm Component mounts its VirtualDOM -->
    <div id="todo-app-root"></div>

    <!-- First-Class Wasm Component loading (No JS wrapper) -->
    <script type="wasm-component" src="wasm_guest.wasm"></script>
  </body>
</html>
```

### 1. Build Servo (if engine changes were made)
```bash
./mach build
```

### 2. Launch the Application
From the repository root:
```bash
./mach run wasm_guest/test_wasm.html
```

---

## Experimenting with the Demo

The test harness (`test_wasm.html`) provides a side-by-side interactive environment:

1. **Live Search Filtering**:
   * Type keywords into either the Dioxus search bar or the left-hand JavaScript search field.
   * Both inputs dynamically synchronize and filter the list via reactive `use_memo` hooks.
   * Click quick-tag chips (`[Wasm]`, `[Servo]`, `[Houdini]`, `[IPC]`) for one-click filtering.

2. **Cross-Boundary Data Ingestion**:
   * Click `[JS] Seed 3 Custom Tasks` in the left panel to observe JavaScript dispatching batch task items through the DOM into Dioxus closures.

3. **Live Telemetry Observation**:
   * The JavaScript panel inspects the `data-*` attributes published by the Dioxus component (`data-total`, `data-active`, `data-matches`) and displays live stats updated on every state mutation.

4. **Dynamic Theme Switching**:
   * Click `[JS] Toggle Palette Theme` to modify CSS variables on `:root`. The styles immediately apply across all Dioxus-rendered DOM nodes.

---

## Host-Guest WIT Interface

The bridge between Servo and WebAssembly is defined in `wit/servo_dom.wit`:

```wit
package servo:dom@0.1.0;

interface document {
    resource element {
        set-attribute: func(name: string, value: string);
        get-attribute: func(name: string) -> option<string>;
        get-property: func(name: string) -> string;
        set-property: func(name: string, value: string);

        append-child: func(child: borrow<element>);
        insert-before: func(child: borrow<element>, reference-child: borrow<element>);
        remove-child: func(child: borrow<element>);
        replace-child: func(new-child: borrow<element>, old-child: borrow<element>);
        parent-node: func() -> option<element>;

        set-text-content: func(text: string);
        get-text-content: func() -> string;
        add-event-listener: func(event-type: string, handler-id: string);
        remove-event-listener: func(event-type: string, handler-id: string);
    }

    create-element: func(tag: string) -> result<element, string>;
    create-text-node: func(text: string) -> result<element, string>;
    get-body: func() -> result<element, string>;
    get-element-by-id: func(id: string) -> result<element, string>;
    query-selector: func(selector: string) -> option<element>;
}

world app {
    import console;
    import document;

    export run: func();
    export on-event: func(handler-id: string, event-type: string);
}
```