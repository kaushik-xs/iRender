# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-17._

## Suggested next

1. Re-run the benchmarks (housekeeping 1).
2. Fix the light frame around dark pages (housekeeping 3).
3. Keyed list updates (gaps 1).

## Gaps you'd hit in real use

- [ ] **Lists rebuilt in the browser redraw every item on each change.**
  - Problem: there's no keyed reconciliation, so focus inside an item is lost (e.g. typing in an input inside a list item), and long lists redraw fully.
  - Where: `rebuild()` in `crates/irender-html/src/runtime.js`.
  - Idea: an optional `key` prop on `list` (`"{{ item.id }}"`), reusing nodes whose key survives.
- [ ] **Overlays inside lists the browser rebuilds don't fully work.**
  - Problem: `ui-dialog-close` and `ui-menu-item` find their overlay through a server-side scope value (`ui_dialog`, `ui_popover`), which is null in a list's item template. Workaround: keep overlays outside those lists.
  - Where: `context()` in `crates/irender-ui-kit/src/lib.rs`, and `overlay.rs`.
  - Idea: let the browser look up the nearest `<dialog>` or `[popover]` ancestor for close buttons and menu items.
- [ ] **One toast at a time.**
  - Problem: a new toast replaces the current one, and the first toast's `wait` can hide its replacement early.
  - Where: `ui-toaster` in `crates/irender-ui-kit/src/feedback.rs`, and the `wait` action.
  - Idea: a toast queue as a client behaviour, with each toast's own timer.
- [ ] **No `else` for `if`.** Today you write a second `if` with the negated condition.
  - Where: `If` in `crates/irender-html/src/components.rs`, and `conditional()` in `render.rs`.
- [ ] **Browser support is only verified in Chrome.**
  - Problem: invoker commands, `closedby`, the Popover API, CSS anchor positioning and `<details name>` were verified in Chrome 157 (locally and in CI), but not in Safari or Firefox. Without anchor positioning, popovers open in the middle of the screen.
  - Idea: run the browser tests in Firefox and WebKit as well (e.g. via Playwright in CI).

## UI kit: not built yet

- [ ] **Keyboard-driven components:**
  - command palette
  - date picker
  - data table with sorting and filtering
  - carousel
  - context menu
  - menubar / navigation menu
  - OTP input

  These need hand-written keyboard and focus handling, which client behaviours (`global_js`) now make possible.
  - The gallery's Live section needs `python3 examples/serve.py`. Plain `python3 -m http.server` has no live endpoints, so the section shows `error` there.
- [ ] **Arrow-key navigation in `ui-dropdown-menu`.** Tab works; there's no roving focus yet.
- [ ] **Lists expanded on the server can't follow state:** `ui-radio-group` options, `ui-breadcrumb` items and `ui-pagination`. Using state in them is a render error today.
- [ ] **Single choice only:** `ui-toggle-group` allows one pressed option at most (no multiple mode), and the built-in `select` has no `multiple`. `ui-combobox` covers multi-select.
- [ ] **`ui-combobox`:**
  - [ ] No custom option rendering (react-select's `formatOptionLabel` and friends); options take a `description` instead.
  - [ ] No chip animations.
  - [ ] Values are plain values, not option objects. A pre-selected value that only `searchUrl` would return shows its raw value until it's searched for. Idea: a `labels` prop or an initial lookup request.
  - [ ] Not tested inside `ui-dialog` / `ui-sheet`. The menu is a top-layer popover opened after the dialog, so it should sit above it, but this is unverified.
- [ ] **Tooltip accessibility:** `ui-tooltip` doesn't link the tip to its trigger with `aria-describedby`, because the tooltip can't add attributes to the child it wraps. Idea: a `trigger` prop that renders the trigger itself, or a small behaviour.

## Housekeeping

- [ ] **Re-run the benchmarks** (`cargo bench -p irender-html`, with and without `--features parallel`). `if`, browser-rebuilt lists, include parameters, behaviours and `scope_key` have landed since the README's table was measured. Update the Performance section with the results.
- [ ] **CI checks we skipped:**
  - Rust 1.88 (the `rust-version` in `Cargo.toml`): skipped because 1.88 couldn't be installed locally to confirm it passes.
  - `cargo fmt --check`: skipped because the code isn't formatted with default rustfmt. Add a `rustfmt.toml` matching the existing style first.
- [ ] **Light frame around dark pages:** `Page::document` leaves the browser's default `body` margin and background, which shows as a light frame around dark-mode pages. Set `margin: 0` and the theme's surface colour on `body`.
- [ ] **`examples/build.sh` doesn't delete pages from earlier builds.** It accepts any output folder, so it shouldn't blindly delete one. Clear only `target/site`, or only files it knows it wrote.
