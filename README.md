# iRender

Build UI from configuration. Write a document in JSON or YAML, then validate it and render it to HTML with a ~4 KB (gzipped) runtime for state and actions.

```sh
cargo run -p irender-cli -- render examples/product.json --data examples/product.data.json -o product.html
cargo run -p irender-cli -- validate examples/counter.yaml
cargo run -p irender-cli -- components   # built-in components and their props
cargo run -p irender-cli -- schema       # JSON Schema for documents
```

See [`examples/`](examples/README.md) for complete pages: a dashboard built from fragments, a live API search, a multi-step form and a storefront whose config is partly generated in Rust.

## Crates

| crate | role |
|---|---|
| `irender-core` | Document model (serde), expressions, theme tokens, policy, validation → IR. No HTML. |
| `irender-style` | Compiles style blocks into hashed, deduplicated classes in cascade layers; CSS validation and minification via lightningcss. |
| `irender-html` | Server-side renderer, component registry, built-ins, browser runtime. No web-framework dependency. |
| `irender-cli` | `validate`, `render` (with `--timings`, `--repeat`, `--parallel`, `--minify`), `schema`, `components`. |

## A node

```json
{
  "type": "button",
  "props": { "label": "{{ qty == 1 ? 'Buy now' : 'Buy ' + qty }}", "disabled": "{{ qty < 1 }}" },
  "attrs": { "id": "buy", "aria-label": "Buy", "data-sku": "{{ product.sku }}" },
  "class": ["cta"],
  "style": {
    "padding": "12px 20px",
    "background": "$color.primary",
    "width": "{{ qty * 10 }}%",
    ":hover": { "transform": "translateY(-1px)" },
    "@md": { "padding": 16 },
    "@dark": { "box-shadow": "none" }
  },
  "parts": { "label": { "style": { "font-weight": 600 } } },
  "on": { "click": { "action": "set_state", "key": "qty", "value": "{{ qty + 1 }}" } }
}
```

- **props** are typed per component and validated.
- **attrs** accept any HTML attribute and override the component's defaults.
- **style** accepts any CSS property, including nested `:pseudo`, `&selector`, `@breakpoint`, `@variant` and `@media …` blocks. Numbers get `px` where it makes sense, and `$tokens` compile to `var(--…)`.
- **parts** style a component's inner elements.
- **on** maps DOM events to declarative actions: `set_state`, `navigate`, `submit` and `call`. A `call` dispatches an `irender:call` DOM event for the host app to handle.
- **Expressions** (`{{ … }}`) support paths, literals, arithmetic, comparisons, `&& || !` and `?:`. An expression that references `state` updates live in the browser. Any other path is filled in from server `data` at render time.
- **Built-in components:** `element`, `text`, `stack`, `button`, `input`, `list` and `if`. Run `irender components` for their props. `include` is handled by the resolver; see below.
- **Cascade order:** theme variables → `i-base` layer (component defaults) → `i-instance` layer (the document's `style`). The document always wins.

## Dynamic content

Parts of a page can come from API calls or other runtime work in four ways. Pick by *when* the data is known.

| known when | use |
|---|---|
| the request comes in | server `data`: `engine.render(&ir, &data)`. `{{ path }}`, `list` and `if` read it. The document is loaded once and reused. |
| the document is loaded | `include` fragments built by your code (below), or build the whole `Document` in code and call `engine.resolve(&doc)`. |
| after the page loads, in the browser | `submit` with `into` stores a JSON response in `state`. Text, attributes, `if` and `list` that use that key update. |
| the page opens | `on: { mount: … }` runs actions as soon as an element appears. |

### `if`

```json
{ "type": "if", "props": { "when": "{{ user.admin }}" }, "children": [ … ] }
```

It renders its children only when `when` is truthy, and adds no element of its own. A condition that only reads server `data` is decided on the server and leaves nothing behind. A condition that reads `state` puts the content between two empty `<template>` markers, and the runtime moves it in and out of the page as the condition changes. The markers are valid anywhere, including inside `<table>` and `<ul>`. Hidden content keeps its DOM nodes, so input values and listeners survive being hidden. Note that the markers count as siblings for `:first-child` / `:nth-child`. There is no `else`; use a second `if` with the negated condition.

### Lists that change in the browser

When `items` reads `state`, `list` also writes its children once into a `<template>` as its first child. When the array changes, the runtime rebuilds every item from that template. The server still renders the initial items, so the page is complete without JavaScript.

```json
{ "version": 1,
  "state": { "results": [], "loading": false },
  "root": { "type": "stack", "children": [
    { "type": "element",
      "on": { "mount": [
        { "action": "set_state", "key": "loading", "value": true },
        { "action": "submit", "method": "GET", "endpoint": "/api/search", "into": "results" },
        { "action": "set_state", "key": "loading", "value": false } ] } },
    { "type": "if", "props": { "when": "{{ loading }}" },
      "children": [{ "type": "text", "props": { "value": "Loading…" } }] },
    { "type": "list", "props": { "items": "{{ results }}", "as": "r", "tag": "ul" },
      "children": [{ "type": "text", "props": { "tag": "li", "value": "{{ r.title }}" },
                     "on": { "click": { "action": "call", "name": "open", "args": { "id": "{{ r.id }}" } } } }] } ] } }
```

Inside a rebuilt item, `item` and `index` (or your `as` / `index` names) are known only in the browser. They work in everything that updates live: text, attributes, classes, `{{ }}` style values, `if`, nested `list` and action arguments. Props that a component reads once on the server, such as `tag`, `variant` or `type`, cannot depend on them. Actions in a list run one after another, and `submit` waits for its response, so `loading` flags work as shown above. Items are rebuilt wholesale, so focus inside a rebuilt item is lost.

### `mount` actions

`on: { mount: … }` runs when the element first appears: at page start, or when an `if` shows it or a list rebuilds it. A server-rendered element mounts once. The name doesn't clash with `<img>`'s native `load` event, which still works through `on: { load: … }`.

### `include`

```json
{ "version": 1,
  "fragments": { "card": { "type": "stack", "children": [ { "type": "text", "props": { "value": "{{ item.name }}" } } ] } },
  "root": { "type": "list", "props": { "items": "{{ products }}" },
            "children": [{ "type": "include", "props": { "name": "card" } }] } }
```

An `include` node is replaced by its fragment when the document is loaded. The fragment is validated like the rest of the document, with paths such as `root.children[0]@card.props.value`, and it sees the scope it's included in. Fragments are looked up in the document's `fragments` first, then in `engine.fragments`:

```rust
use irender_core::{FragmentDir, NodeSpec};

engine.fragments = Some(Arc::new(FragmentDir("ui/fragments".into())));   // <name>.json / .yaml; CLI: --fragments DIR
// or build them from anything known at load time:
engine.fragments = Some(Arc::new(move |name: &str| -> Result<Option<NodeSpec>, String> {
    match name {
        "promo" => Ok(Some(serde_json::from_value(fetch_promo_config()?).map_err(|e| e.to_string())?)),
        _ => Ok(None),
    }
}));
```

`include` only takes `props.name`, which must be a fixed name. An error from your source is reported at the include's path. Cycles are rejected, and nesting, including fragments, is limited to `MAX_DEPTH` (64) levels. Fragments are expanded at load time, so a fragment that changes per request means loading per request. Loading costs tens of µs for typical documents (see Performance). Cache the `Ir` per fragment version if that matters.

## Policies

`Engine::new()` trusts the document. `Engine::untrusted()` (or `--untrusted` in the CLI) is for user- or LLM-written configs. It blocks `on*` attributes, `javascript:` URLs, dangerous tags and CSS `expression()`. CSS values that could break out of their declaration are always rejected. So are `data-i-*` attributes, which the runtime reads its bindings and actions from.

lightningcss reports unknown CSS properties and invalid values as **warnings**, because it doesn't know every valid property. Standard properties it doesn't model, such as `border-collapse`, `float` or `pointer-events`, are on an allowlist in `irender-style/src/validate.rs`. Their names are accepted, but their values aren't checked. Use `strict_css` (`--strict-css` in the CLI) to make them errors.

## Performance

```sh
cargo bench -p irender-html                                  # criterion; filter with: -- render/table
cargo bench -p irender-html --features parallel              # adds render/html_parallel/*
cargo run --release -p irender-cli -- render doc.json --data data.json --repeat 200 -o out.html
```

`--timings` / `--repeat N` print parse, resolve, render and serialize times for your own document. Load (parse + resolve) is meant to happen once per document change. Render happens per request.

Results on an Apple M4 Pro, release build:

| fixture | load | render | per element | render, parallel |
|---|---|---|---|---|
| product example (17 elements) | 73 µs | 55 µs | ~3.2 µs | — |
| table, 100 rows (~500 elements) | 32 µs | 426 µs | ~0.85 µs | 205 µs |
| table, 1000 rows (~5000 elements) | 32 µs | 4.15 ms | ~0.83 µs | 1.00 ms |
| 1000 cards with unique styles (~2000 elements) | 5.4 ms | 2.89 ms | ~1.4 µs | — |
| 50 nested levels | 103 µs | 23 µs | ~0.45 µs | — |

**Load** is parse + resolve + CSS validation. **Render, parallel** uses `parallel_min_items = Some(64)`; "—" means the document has no list that large, so it renders on one thread with no overhead.

What keeps rendering fast:
- **Styles are cached by content.** A style block already compiled in this render costs one hash and one comparison (~0.3 µs instead of ~1.4 µs). Built-in components' default styles are built once per process.
- **Browser data is written straight to the output.** Bindings and actions are folded and serialized directly into the HTML, without building intermediate JSON values.
- **Elements borrow from the node.** An element borrows its node's attributes, classes, styles and actions instead of copying them.

`--minify` costs about as much as rendering a small page and saves ~17% of the CSS.

Documents can nest at most ~60 UI levels, and at most 64 counting included fragments. Deeper input is rejected with a located error, which stops hostile input from overflowing the stack.

## Concurrency

**Across requests.** `Engine` and `Ir` are `Send + Sync` (checked at compile time), and `Engine::render` takes `&self` and `&Ir`. Load a document once and render it from any number of threads or async tasks without locks:

```rust
let engine = Arc::new(Engine::new());
let ir = Arc::new(engine.load(src, Format::Json)?);
// in each request handler:
let page = engine.render(&ir, &request_data)?;
```

**Within one page.** With the `parallel` feature, lists with at least `parallel_min_items` items render in chunks on rayon's thread pool:

```rust
let engine = Engine { parallel_min_items: Some(64), ..Engine::new() };  // CLI: --parallel 64
```

The output is byte-identical to a single-thread render, and the first error in document order is reported. This lowers the latency of a large page but uses slightly more total CPU. On a server that is already busy, every core is serving some request, so parallel rendering is off by default. Turn it on when large pages are rare and their latency matters, such as reports, exports or admin tables.

## Features

| crate | feature | default | effect |
|---|---|---|---|
| `irender-core` | `yaml` | yes | Accept `.yaml` / `.yml` documents. |
| `irender-style` | `lightningcss` | yes | Validate CSS declarations and allow `--minify`. |
| `irender-html` | `lightningcss`, `yaml` | yes | Forward to the crates above. |
| `irender-html` | `parallel` | no | Parallel list rendering (`rayon`). The CLI enables it. |

## Tests

```sh
cargo test                                            # everything (the CLI enables `parallel`, so its tests run too)
cargo test -p irender-html                               # renderer without `parallel`
cargo test -p irender-html --features parallel           # renderer with `parallel`
cargo test -p irender-html --test browser -- --ignored   # drives the example in headless Chrome (CHROME=… to override)
```

| suite | covers |
|---|---|
| `irender-core` unit tests + `tests/core.rs` | expressions and type conversions, error paths, JSON vs YAML, prop/style/action/theme validation, untrusted policy, warnings |
| `irender-style` unit tests + `tests/compile.rs` | nesting, selector variants, dynamic values, layer order, dark-mode variables, cache, pinned class names |
| `irender-html/tests/components.rs` | every built-in, `if` markers, list item templates, `include` (scope, sources, cycles, depth, `FragmentDir`), which values update live, actions, URL and `</script>` safety, deterministic output, custom components, rendering from many threads, parallel output identical to sequential |
| `irender-html/tests/runtime_parity.rs` | the browser runtime has its own copy of the expression evaluator; ~90 expressions run through both (Rust, and JS via `node`), and any disagreement fails. Skips with a message if `node` isn't installed. |
| `irender-html/tests/browser.rs` | opt-in, needs Chrome: clicks, types and checks the DOM of the product example; toggles `if` blocks, rebuilds lists from state, runs item-scoped and `mount` actions |

## Custom components

```rust
use irender_html::{Component, Engine, RenderCx, RenderError};
use irender_core::{ComponentInfo, Node, PropDef, PropKind};

struct Badge;
impl Component for Badge {
    fn name(&self) -> &'static str { "badge" }
    fn info(&self) -> ComponentInfo {
        const PROPS: &[PropDef] = &[PropDef::new("text", PropKind::Any, "").required()];
        ComponentInfo { doc: "A badge", props: PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("span", node);
        if let Some(text) = node.prop("text") {
            el.text(text);
        }
        cx.open(&el, node)?;
        cx.close("span");
        Ok(())
    }
}

let mut engine = Engine::new();
engine.registry.register(Badge);
```
