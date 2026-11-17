# irender-wasm

iRender's renderer for the browser. A chat message whose `ui` (an iRender document or node) arrives without `html` can be rendered in the page: point `ui-conversation`'s `render.browser` at this module, and the page loads it the first time a message needs it. See [UI in messages](../irender-ui-kit/README.md#ui-in-messages) for how it fits with rendering on the server.

```sh
rustup target add wasm32-unknown-unknown
cargo build -p irender-wasm --target wasm32-unknown-unknown --profile wasm
# → target/wasm32-unknown-unknown/wasm/irender_wasm.wasm; serve it with your pages
```

`examples/build.sh` builds it into `target/site/irender/` when the target is installed.

## What it renders

- Under the untrusted policy, since UI in a reply usually comes from a model: no scripts, event attributes or `javascript:` URLs.
- With the built-ins, the UI kit and the chart kit. The icon kit's packs are too large to ship, so icon names fall back to text. YAML parsing and CSS validation are left out to keep the module small: unknown CSS properties aren't reported, though the checks that keep CSS from breaking out of a declaration still run.
- With `irender_ui_kit::render_ui`, the function the CLI and a server's render endpoint use, so the markup is the same wherever it's rendered. Keep the module's version the same as the crates that render your pages.

## Your own components

A UI using a component the module wasn't built with fails to render, and the page asks the server instead when `render.url` is set. To render your components in the browser, make a `cdylib` crate that depends on this one without its default features and exports your engine:

```rust
fn engine() -> irender_html::Engine {
    let mut engine = irender_wasm::engine();
    engine.registry.register(my_components::Gauge);
    engine
}
irender_wasm::export_renderer!(engine);
```

## The module

It imports nothing and exports `memory` and:

| export | |
|---|---|
| `irender_alloc(len) -> ptr` | Room for the input. |
| `irender_render(ptr, len) -> status` | Renders the UTF-8 UI JSON there and frees it: 0 when it rendered, 1 for an error. |
| `irender_out_ptr() -> ptr`, `irender_out_len() -> len` | The markup, or the error message, as UTF-8. |

The UI kit's `ui-message-html` behaviour calls these; you only need them to render outside a chat.

## Limits

- The page must be allowed to compile WebAssembly: a Content-Security-Policy needs `'wasm-unsafe-eval'` in `script-src`. Without it, set `render.url` too, and the page renders on the server.
- A message's UI is self-contained: no server `data`, and no fragments other than its own.
- It renders on the page's main thread.
