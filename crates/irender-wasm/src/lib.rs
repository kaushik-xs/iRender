//! iRender's renderer for the browser. A chat message whose `ui` arrives without
//! `html` can be rendered in the page: `ui-conversation`'s `render.browser` names this
//! module's `.wasm` file, and the page loads it the first time it needs it.
//!
//! ```sh
//! rustup target add wasm32-unknown-unknown
//! cargo build -p irender-wasm --target wasm32-unknown-unknown --profile wasm
//! # target/wasm32-unknown-unknown/wasm/irender_wasm.wasm
//! ```
//!
//! It renders with [`irender_ui_kit::render_ui`], like the CLI and a server's render
//! endpoint, under the untrusted policy, with the built-ins and the UI and chart kits.
//! Icons, YAML and CSS validation are left out to keep it small. For your own
//! components, make a `cdylib` crate that depends on this one without its default
//! features and exports your engine:
//!
//! ```ignore
//! fn engine() -> irender_html::Engine {
//!     let mut engine = irender_wasm::engine();
//!     engine.registry.register(my_components::Gauge);
//!     engine
//! }
//! irender_wasm::export_renderer!(engine);
//! ```
//!
//! The module imports nothing. Its exports: `irender_alloc(len) -> ptr` for the
//! input, `irender_render(ptr, len) -> status` (0 when it rendered, 1 for an error;
//! it frees the input), then `irender_out_ptr()` and `irender_out_len()` for the
//! markup or the error message, as UTF-8.

use irender_html::Engine;
use serde_json::Value as Json;

/// The browser's engine: the untrusted policy, since UI in a reply usually comes from
/// a model, with the built-ins and the UI and chart kits.
pub fn engine() -> Engine {
    let mut engine = Engine::untrusted();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register(&mut engine.registry);
    engine
}

/// Renders a message's `ui`, given as JSON, with `engine`: the kits' themes are added
/// first, as the CLI adds them.
pub fn render(engine: &Engine, ui: &str) -> Result<String, String> {
    let ui: Json = serde_json::from_str(ui).map_err(|e| format!("the UI isn't JSON: {e}"))?;
    irender_ui_kit::render_ui(engine, &ui, |doc| {
        irender_ui_kit::apply_theme(doc);
        irender_chart_kit::apply_theme(doc);
    })
}

/// Exports the WebAssembly functions the page calls, rendering with the engine `make`
/// returns (built once). See the crate docs.
#[macro_export]
macro_rules! export_renderer {
    ($make:path) => {
        thread_local! {
            static IRENDER_ENGINE: $crate::__Engine = $make();
            static IRENDER_OUT: ::std::cell::RefCell<::std::vec::Vec<u8>> = const { ::std::cell::RefCell::new(::std::vec::Vec::new()) };
        }

        /// Room for `len` bytes of input.
        #[unsafe(no_mangle)]
        pub extern "C" fn irender_alloc(len: usize) -> *mut u8 {
            let mut buf = ::std::vec::Vec::<u8>::with_capacity(len);
            let ptr = buf.as_mut_ptr();
            ::std::mem::forget(buf);
            ptr
        }

        /// Renders the `len` bytes of UI JSON at `ptr` (from `irender_alloc`, freed here).
        ///
        /// # Safety
        /// `ptr` must come from `irender_alloc(len)`, with `len` bytes written.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn irender_render(ptr: *mut u8, len: usize) -> u32 {
            // SAFETY: the caller passes the buffer `irender_alloc(len)` returned, filled.
            let input = unsafe { ::std::vec::Vec::from_raw_parts(ptr, len, len) };
            let result = match ::std::str::from_utf8(&input) {
                Ok(ui) => IRENDER_ENGINE.with(|engine| $crate::render(engine, ui)),
                Err(_) => Err("the UI isn't UTF-8".to_string()),
            };
            let (status, out) = match result {
                Ok(html) => (0, html),
                Err(e) => (1, e),
            };
            IRENDER_OUT.with(|o| *o.borrow_mut() = out.into_bytes());
            status
        }

        /// Where the last result is.
        #[unsafe(no_mangle)]
        pub extern "C" fn irender_out_ptr() -> *const u8 {
            IRENDER_OUT.with(|o| o.borrow().as_ptr())
        }

        /// The last result's length in bytes.
        #[unsafe(no_mangle)]
        pub extern "C" fn irender_out_len() -> usize {
            IRENDER_OUT.with(|o| o.borrow().len())
        }
    };
}

#[doc(hidden)]
pub use irender_html::Engine as __Engine;

#[cfg(all(feature = "exports", target_family = "wasm"))]
export_renderer!(engine);

#[cfg(test)]
mod tests {
    use super::*;

    // The exports, as the page calls them.
    mod abi {
        crate::export_renderer!(crate::engine);
    }

    fn call(ui: &str) -> (u32, String) {
        let ptr = abi::irender_alloc(ui.len());
        // SAFETY: `ptr` has room for `ui.len()` bytes, and `irender_render` takes it back.
        let status = unsafe {
            std::ptr::copy_nonoverlapping(ui.as_ptr(), ptr, ui.len());
            abi::irender_render(ptr, ui.len())
        };
        // SAFETY: the result stays put until the next render.
        let out = unsafe { std::slice::from_raw_parts(abi::irender_out_ptr(), abi::irender_out_len()) };
        (status, String::from_utf8(out.to_vec()).unwrap())
    }

    #[test]
    fn exports_render_and_report_errors() {
        let (status, html) = call(r#"{ "type": "ui-badge", "props": { "text": "Hi" } }"#);
        assert_eq!(status, 0);
        assert!(html.contains(">Hi</span>"), "{html}");
        let (status, err) = call(r#"{ "type": "nope" }"#);
        assert_eq!(status, 1);
        assert!(err.contains("unknown component type `nope`"), "{err}");
        assert_eq!(call("").0, 1);
    }

    #[test]
    fn renders_a_node_or_a_document_untrusted() {
        let engine = engine();
        let html = render(&engine, r#"{ "type": "ui-badge", "props": { "text": "Hi" } }"#).unwrap();
        assert!(html.starts_with("<style>") && html.contains(">Hi</span>") && !html.contains(":root"), "{html}");
        let html = render(&engine, r#"{ "state": { "n": 2 }, "root": { "type": "ui-chart", "props": { "type": "bar", "data": [1, 2] } } }"#).unwrap();
        assert!(html.contains(r#"data-i-state>{"n":2}"#), "{html}");
        // The untrusted policy keeps scripts and `javascript:` out.
        let err = render(&engine, r#"{ "type": "element", "props": { "tag": "script" } }"#).unwrap_err();
        assert!(err.contains("script"), "{err}");
        let err = render(&engine, r#"{ "type": "element", "props": { "tag": "a" }, "attrs": { "href": "javascript:alert(1)" } }"#).unwrap_err();
        assert!(err.contains("URL") || err.contains("url"), "{err}");
        assert!(render(&engine, "{").unwrap_err().starts_with("the UI isn't JSON"));
    }
}
