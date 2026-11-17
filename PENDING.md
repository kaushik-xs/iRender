# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-17._

## Suggested next

1. Browser tests in Firefox and WebKit (gaps 2): overlays rely on Chrome-first features.

## Gaps you'd hit in real use

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
- [ ] **Lists expanded on the server can't follow state:** `ui-radio-group` options, `ui-breadcrumb` items and `ui-pagination`. Using state in them is a render error today.
- [ ] **Single choice only:** `ui-toggle-group` allows one pressed option at most (no multiple mode), and the built-in `select` has no `multiple`. `ui-combobox` covers multi-select.
- [ ] **`ui-combobox`:**
  - [ ] No custom option rendering (react-select's `formatOptionLabel` and friends); options take a `description` instead.
  - [ ] No chip animations.
  - [ ] Values are plain values, not option objects. A pre-selected value that only `searchUrl` would return shows its raw value until it's searched for. Idea: a `labels` prop or an initial lookup request.
  - [ ] Not tested inside `ui-dialog` / `ui-sheet`. The menu is a top-layer popover opened after the dialog, so it should sit above it, but this is unverified.
- [ ] **`ui-theme-toggle`'s choice only applies on pages that have a toggle.** The index and the UI kit and chart kit galleries share it; the other examples still follow the system.
- [ ] **Tooltip accessibility:** `ui-tooltip` doesn't link the tip to its trigger with `aria-describedby`, because the tooltip can't add attributes to the child it wraps. Idea: a `trigger` prop that renders the trigger itself, or a small behaviour.

## Icon kit: packs not drawn yet

Phase 1 shipped `core`, `arrows`, `status`, `logistics`, `warehouse`, `retail` and `manufacturing`. Every new icon is drawn from scratch to `crates/irender-icon-kit/GUIDELINES.md` and must pass `tools/icons.py lint` and a contact-sheet review. Draw a pack's first few icons as references, then the rest from them.

- [ ] **Base packs:**
  - files & documents
  - communication
  - users & people
  - data & charts
  - media & editor
  - devices & tech
  - security & access
  - location & map
  - commerce basics (currencies, wallet, card, bank)
  - time & scheduling
  - weather & environment
  - social (generic, no logos)
- [ ] **Industry packs:**
  - energy & utilities
  - oil, gas & mining
  - construction & real estate
  - facilities
  - healthcare
  - pharma & life sciences
  - banking & finance
  - insurance
  - accounting & HR
  - education
  - hospitality & travel
  - food & beverage
  - agriculture
  - automotive
  - aviation
  - maritime
  - public sector
  - legal
  - telecom
  - IT & DevOps
  - cybersecurity
  - marketing & media
  - sales & CRM
  - support
  - sports & fitness
  - events
  - beauty & wellness
  - textile & apparel
  - chemicals (GHS hazard pictograms)
  - ESG
  - nonprofit
  - pets & veterinary
  - science
  - IoT
  - workplace safety (EHS)
- [ ] **Add-on packs:**
  - country flags (these need fills, so a separate style)
  - payment methods (generic)
  - file-type badges
  - reactions
  - detailed weather
  - accessibility
- [ ] **`ui-icon`'s `name` can't use state.** Icons are inlined on the server, so a name from `state` (or from a list item the browser rebuilds) is a render error; `if` / `else` covers fixed choices. Idea: a `names` prop listing the possible icons, all rendered with only the current one shown.
- [ ] **No tags or search.** Packs have names only. Idea: a tags line per icon (in `_pack.txt` or a sidecar) for the gallery and `irender icons --search`.
- [ ] **`ui-alert`'s `icon` is text only.** It can't take a `ui-icon` yet. Idea: when the kit is registered, accept `icon: { name: … }`, or a part slot for a child.
- [ ] **Aliases:** no `previous` → `chevron-left`-style aliases between packs.

## Housekeeping

- [ ] **CI checks we skipped:**
  - Rust 1.88 (the `rust-version` in `Cargo.toml`): skipped because 1.88 couldn't be installed locally to confirm it passes.
  - `cargo fmt --check`: skipped because the code isn't formatted with default rustfmt. Add a `rustfmt.toml` matching the existing style first.
- [ ] **`examples/build.sh` doesn't delete pages from earlier builds.** It accepts any output folder, so it shouldn't blindly delete one. Clear only `target/site`, or only files it knows it wrote.
