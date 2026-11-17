# iRender

[![CI](https://github.com/kaushik-xs/iRender/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/kaushik-xs/iRender/actions/workflows/ci.yml)

Build UI from configuration. Write a document in JSON or YAML, then validate it and render it to HTML with a ~4.5 KB (gzipped) runtime for state and actions.

```sh
cargo run -p irender-cli -- render examples/product.json --data examples/product.data.json -o product.html
cargo run -p irender-cli -- validate examples/counter.yaml
cargo run -p irender-cli -- components   # built-in components and their props
cargo run -p irender-cli -- schema       # JSON Schema for documents
```

See [`examples/`](examples/README.md) for complete pages: a dashboard built from fragments, a live API search, a multi-step form and a storefront whose config is partly generated in Rust. To browse them all, run `examples/build.sh`, then `python3 examples/serve.py` (a static server plus the live endpoints the chart gallery uses).

## Crates

| crate | role |
|---|---|
| `irender-core` | Document model (serde), expressions, theme tokens, policy, validation → IR. No HTML. |
| `irender-style` | Compiles style blocks into hashed, deduplicated classes in cascade layers; CSS validation and minification via lightningcss. |
| `irender-html` | Server-side renderer, component registry, built-ins, browser runtime. No web-framework dependency. |
| `irender-cli` | `validate`, `render` (with `--timings`, `--repeat`, `--parallel`, `--minify`, `--ui-kit`, `--chart-kit`, `--flow-kit`, `--icon-kit`, `--icons`), `schema`, `components`, `icons`. |
| `irender-ui-kit` | Optional, shadcn-style components (`ui-button`, `ui-card`, `ui-alert`, form controls, table, pagination, …) and a matching theme. See its [README](crates/irender-ui-kit/README.md). |
| `irender-chart-kit` | Optional charts drawn by Apache ECharts: `ui-chart` (shorthand for 22 series types, or any ECharts option), `ui-sparkline` and `ui-live-data`: filters, grouping and drill-downs in the browser, clicks, zoom, legend and brush as state, live polling, SSE and WebSocket data, themed from the document's tokens. See its [README](crates/irender-chart-kit/README.md). |
| `irender-icon-kit` | Optional `ui-icon`: original SVG icons, outline and filled, in packs (core, arrows, status, logistics, warehouse, retail, manufacturing, accounting, agriculture, automotive, aviation, banking, construction, education, energy, finance, healthcare, hospitality, hr, insurance, it, legal, marketing, maritime, media, mining, oil-gas, public-sector, robotics, simulation, space, sports, telecom) drawn to one standard, plus your own packs from SVG files, sanitized on load. See its [README](crates/irender-icon-kit/README.md). |
| `irender-flow-kit` | Optional `ui-flow`: node-and-edge diagrams from React Flow's JSON, with node and edge types written as ordinary iRender trees, ports from node data, layered layout, pan and zoom, selection, dragging, connecting with rules, deleting with confirmation, a palette, containers, copy and paste, undo, search and a minimap. See its [README](crates/irender-flow-kit/README.md). |

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
- **on** maps DOM events to declarative actions: `set_state`, `navigate`, `submit`, `call` and `wait`. A `call` dispatches an `irender:call` DOM event for the host app to handle. A `wait` (`{ "action": "wait", "ms": 3000 }`, at most 60 000) pauses before the next action, e.g. to hide a message again.
- **Expressions** (`{{ … }}`) support paths, literals, arithmetic, comparisons, `&& || !`, `?:` and the functions `range`, `includes` and `toggle`. An expression that references `state` updates live in the browser. Any other path is filled in from server `data` at render time.
  - `range(n)` is the list `[0, 1, …, n - 1]`, and `range(start, end)` runs from `start` to `end - 1`, for a `list` that repeats something a number of times: `"items": "{{ range(stars) }}"`.
  - Arguments are truncated to whole numbers, and anything that isn't a number counts as 0.
  - At most 1000 items.
  - `includes(list, value)` says whether the list has the value (compared as `==`), and `toggle(list, value)` is the list without it if it has it, or with it added at the end. Together they keep a set of choices in state: a button whose `aria-pressed` is `"{{ includes(tags, 'rust') }}"` and whose click sets `tags` to `"{{ toggle(tags, 'rust') }}"`. Anything not a list counts as empty.
  - These are the only functions. `.field` and `[i]` follow paths only, so `range(n).length` isn't allowed.
- **Built-in components:** `element`, `text`, `stack`, `button`, `input`, `select`, `textarea`, `list` and `if`. Run `irender components` for their props. `include` and `else` are handled by the resolver; see below.
- **Cascade order:** theme variables → `i-base` layer (component defaults) → `i-instance` layer (the document's `style`). The document always wins.

## Form controls

`input`, `select` and `textarea` take `bind: <state key>` to keep a state value in sync both ways.

```json
{ "type": "select", "props": { "bind": "country", "placeholder": "Country",
  "options": [{ "value": "in", "label": "India" }, { "value": "de", "label": "Germany", "disabled": true }] } }
{ "type": "input", "props": { "type": "radio", "bind": "size", "value": "m" } }
{ "type": "input", "props": { "type": "checkbox", "bind": "gift" } }
{ "type": "textarea", "props": { "bind": "notes", "rows": 4 } }
```

- **`select`:** `options` is an array of plain values or `{ value, label, disabled }`. If it comes from server data, the options are fixed. If it reads `state`, the browser rebuilds them when the array changes and keeps the current choice. `placeholder` adds a first, empty option that shows while the key is empty.
- **Radios:** a bound radio needs a `value`. It is checked when the state equals that value, and choosing it writes the value. Radios bound to the same key share `name` automatically, so the browser keeps one checked.
- **Checkboxes:** a bound checkbox stores `true` / `false`.
- **Types:** the DOM holds option and radio values as text. If a key starts out as a number, choosing a radio or option keeps it a number, as a `number` input does. Otherwise it becomes a string.

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

It renders its children only when `when` is truthy, and adds no element of its own. A condition that only reads server `data` is decided on the server and leaves nothing behind. A condition that reads `state` puts the content between two empty `<template>` markers, and the runtime moves it in and out of the page as the condition changes. The markers are valid anywhere, including inside `<table>` and `<ul>`. Hidden content keeps its DOM nodes, so input values and listeners survive being hidden. Note that the markers count as siblings for `:first-child` / `:nth-child`.

An `else` right after an `if` renders its children when the `if` doesn't. Give it a `when` to make it an else-if, and chain as many as you like; a plain `else` ends the chain:

```json
{ "type": "if", "props": { "when": "{{ n > 2 }}" }, "children": [ … ] },
{ "type": "else", "props": { "when": "{{ n > 0 }}" }, "children": [ … ] },
{ "type": "else", "children": [ … ] }
```

The resolver turns each `else` into an `if` whose condition is that no earlier branch matched, so it behaves exactly like one, including updating live when a condition reads `state`. An `else` takes only `when` and children, and must directly follow an `if` or an `else` with `when`.

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

Inside a rebuilt item, `item` and `index` (or your `as` / `index` names) are known only in the browser. They work in everything that updates live: text, attributes, classes, `{{ }}` style values, `if`, nested `list` and action arguments. Props that a component reads once on the server, such as `tag`, `variant` or `type`, cannot depend on them. Actions in a list run one after another, and `submit` waits for its response, so `loading` flags work as shown above.

Without a `key`, every item is rebuilt when the array changes, so focus and typed input inside an item are lost. Give `list` a `key` (e.g. `"key": "{{ r.id }}"`) and an item whose key is still in the new array keeps its DOM nodes: input values, focus and hidden `if` content survive, `mount` doesn't run again, and only text and attributes that read the item or `index` update. Items are moved only when they're out of order. Keys should be unique; a repeated key reuses one old item per occurrence. A keyed list's server-rendered items are replaced by template items when the page starts, because the server baked their values into plain HTML.

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

`include` takes `props.name`, which must be a fixed name, and optionally `props.with`. An error from your source is reported at the include's path. Cycles are rejected, and nesting, including fragments, is limited to `MAX_DEPTH` (64) levels. Fragments are expanded at load time, so a fragment that changes per request means loading per request. Loading costs tens of µs for typical documents (see Performance). Cache the `Ir` per fragment version if that matters.

#### Parameters

`with` passes values into the fragment, so one fragment serves different data:

```yaml
fragments:
  stat:
    type: stack
    children:
      - { type: text, props: { value: "{{ label }}" } }
      - { type: text, props: { value: "{{ value }}" }, style: { color: "{{ tone }}" } }
root:
  type: stack
  children:
    - { type: include, props: { name: stat, with: { label: Revenue, value: "{{ totals.revenue }}", tone: $color.primary } } }
    - { type: include, props: { name: stat, with: { label: In cart, value: "{{ cart.length }}", tone: green } } }
```

A parameter is substituted into the fragment's expressions when the document loads. Inside the fragment above, `{{ value }}` becomes the caller's `{{ totals.revenue }}`, so it behaves exactly as if written in place:
- Server data is filled in on the server.
- An argument that reads state, like `{{ cart.length }}`, keeps updating in the browser.
- An `item` from a list, including one the browser rebuilds, still resolves.

Literal arguments stay literal, so the usual checks apply to them: prop types, `bind` keys, URL policy, and CSS tokens and values. A parameter that is a path or a literal object can be read further (`{{ user.name }}` with `with: { user: "{{ order.customer }}" }`); a computed one (`"{{ a + b }}"`) can't.

Names the fragment uses but no `with` passes are looked up at render time as usual, so fragments can still read the scope they're included in. Defaults are expressions: `{{ size || 'md' }}`. A parameter the fragment never uses gives a warning, which catches typos. A `list` inside the fragment whose `item` or `index` name matches a parameter is an error, since it would hide the parameter.

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

Results on an Apple M4 Pro, release build (measured 2026-11-17):

| fixture | load | render | per element | render, parallel |
|---|---|---|---|---|
| product example (17 elements) | 75 µs | 56 µs | ~3.3 µs | — |
| table, 100 rows (~500 elements) | 32 µs | 442 µs | ~0.88 µs | 210 µs |
| table, 1000 rows (~5000 elements) | 32 µs | 4.24 ms | ~0.85 µs | 1.06 ms |
| 1000 cards with unique styles (~2000 elements) | 5.6 ms | 2.88 ms | ~1.4 µs | — |
| 50 nested levels | 104 µs | 24 µs | ~0.47 µs | — |

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
| `irender-html/tests/components.rs` | every built-in, `if` markers and `else` chains, list item templates, `include` (scope, sources, cycles, depth, `FragmentDir`), which values update live, actions, URL and `</script>` safety, deterministic output, custom components, rendering from many threads, parallel output identical to sequential |
| `irender-html/tests/runtime_parity.rs` | the browser runtime has its own copy of the expression evaluator; ~90 expressions run through both (Rust, and JS via `node`), and any disagreement fails. Skips with a message if `node` isn't installed. |
| `irender-html/tests/browser.rs` | opt-in, needs Chrome: clicks, types and checks the DOM of the product example; toggles `if` blocks, rebuilds lists from state, runs item-scoped and `mount` actions; binds radios, checkboxes, selects (with options from state) and textareas |

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

### Page-level CSS and client behaviours

A component that needs more than per-element styles and the built-in actions can add two kinds of page-level code. Both take `&'static str`, so they come from component code and never from a document:

- `cx.global_css(name, css)` adds a rule such as `@keyframes` once per page, however many instances ask for it.
- `cx.global_js(name, js)` adds a client behaviour's script once per page. `el.behavior(name, config)` marks an element for it. The runtime starts the behaviour on each marked element, including ones an `if` or a rebuilt list inserts later, and calls its `update` after every state change:

```js
(window.__irenderBehaviors ||= {})["counter"] = (el, api) => {
  // api.config (from `behavior`), api.state, api.set(key, value), api.emit(event, detail)
  el.addEventListener("click", () => api.set("n", api.state.n + api.config.step));
  return { update(state) { el.textContent = state.n; } };
};
```

- `cx.early_js(name, js)` adds a script once per page that runs right after the page's `<style>`, before any content is shown. Use it only for what must happen before the first paint. `ui-theme-toggle` uses it to apply a stored colour scheme without a flash.
- `registry.early_js(name, js)` adds such a script to every page rendered with that registry, whatever components it uses. A kit uses it for a setting that covers the whole page: `irender-ui-kit` applies the colour scheme chosen with `ui-theme-toggle` on every page, with a toggle on it or not.

Documents can't mark elements themselves: `data-i-*` attributes are reserved. `irender-ui-kit`'s `ui-combobox` is built this way.

### Passing values to nested components

- `cx.with_context(name, value, f)` makes a `Val` available to the components rendered inside `f`, and `cx.context(name)` reads the innermost one. Documents can't read or set it. A value that depends on an item only the browser knows stays a template, so a dialog can hand its id to the close buttons inside it, even in a list the browser rebuilds.
- `cx.scope_key()` tells the items of a server-rendered list apart, for element ids. Inside a list the browser rebuilds, `cx.client_key()` is a template of the surrounding list indexes (`{{ index }}`) to add as well.
