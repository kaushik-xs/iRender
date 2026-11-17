# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-17._

## Suggested next

1. Keyboard-driven UI kit components (command palette, date picker, data table).

## Gaps you'd hit in real use

- [ ] **Older browsers aren't tested.** The browser tests run in Chrome, and in Playwright's WebKit and Firefox (CI, and `tools/playwright/dump-dom.js` locally), which are recent builds (Safari 27.2, Firefox 159 on 2026-11-17). Safari releases and Firefox ESR without anchor positioning, the Popover API, invoker commands, `closedby` or `<details name>` aren't covered; without anchor positioning, popovers open in the middle of the screen.
  - Idea: run the tests in the installed Safari through `safaridriver`.

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
- [ ] **`ui-sidebar` gaps:**
  - [ ] No phone layout. shadcn's turns into a sheet on small screens; ours stays beside the page, so on a phone it should start closed or use `collapsible: icon`.
  - [ ] No keyboard shortcut (shadcn toggles with ⌘B / Ctrl+B) and no remembered state: a reload opens it as `state` says.
  - [ ] Collapsed to icons, items show no tooltip with their label (screen readers still read it).
  - [ ] No group actions or per-item action buttons (shadcn's `SidebarGroupAction`, `SidebarMenuAction`); put a `ui-dropdown-menu` beside the item instead.
- [ ] **More blocks:** shadcn has many more variants of each (sidebar-01 to -16, login-01 to -05, signup-01 to -05). We have dashboard-01, sidebar-03 and -07, login-03 and -04, and one sign-up.
- [ ] **shadcn/ui components still missing, beyond the keyboard-driven ones above:** calendar, drawer (a draggable bottom sheet; `ui-sheet` with `side: bottom` covers the look), resizable panels, item, code block, direction (RTL), and the chat ones (attachment, bubble, message, message scroller, questionnaire, trace).
- [ ] **The built-in `select` has no `multiple`.** `ui-combobox` covers multi-select, and `ui-toggle-group` has `multiple`.
- [ ] **`ui-combobox`:**
  - [ ] No custom option rendering (react-select's `formatOptionLabel` and friends); options take a `description` instead.
  - [ ] No chip animations.
  - [ ] Values are plain values, not option objects. A pre-selected value that only `searchUrl` would return shows its raw value until it's searched for. Idea: a `labels` prop or an initial lookup request.
- [ ] **Pages rendered without the kit ignore `ui-theme-toggle`'s choice.** Every kit page applies it now, but the search, wizard, counter and product examples are rendered without `--ui-kit`, so they still follow the system.

## Flow kit next phases

These came out of the React Flow research on the builder and client-ui-next repos.

- Phase 1, the viewer, has shipped: React Flow JSON, node, edge and port templates, layered layout, pan and zoom, selection, neighbour highlighting, dragging, the minimap and panels.
- Phase 2, editing, has shipped:
  - connecting, with flow and port rules
  - dropping a connection on empty canvas to add a `newNode`
  - edge, multi and box selection, and dragging a group
  - deleting, with a confirmation dialog
  - a `ui-flow-item` palette (drag or click)
  - `flow.*` commands

- Phase 3 has shipped:
  - containers (`container`, `resizable`, moving nodes in and out on drop, `scope: children` ports, stacking order)
  - copy, cut and paste
  - undo and redo
  - search with dimming and stepping through matches

- [ ] **Reconnecting an edge** by dragging its end to another port isn't supported; delete it and connect again.
- [ ] **The palette's drag uses HTML drag and drop**, which touch screens don't support. A tap adds the node in the middle of the view instead.
- [ ] **Edges are drawn only by the script.** Nodes have server positions, but a page without JS shows no edges. Idea: draw straight edges on the server when the positions are known.
- [ ] **Layered layout is approximate.** Long edges get no dummy nodes, so edges that skip ranks can cross nodes. Only top-level nodes are laid out; children keep their `position`. With `layout: layered`, an added node's position is replaced by the layout.
- [ ] **Undo records only the flow's own edits.** Changes made to the state outside the flow aren't recorded, and an undo puts back the arrays as the flow last saw them.
- [ ] **No `extent: parent`.** A node inside a container can be dragged past its edges, and dropping it outside moves it out.
- [ ] **Pasting doesn't remap ids inside `data`** (the process canvas does this for its own ids); the host app can, from a `call` on `nodeadd`.
- [x] **Tried on the builder's 8 saved workflows** (2026-11-17; `server/data/workspaces/**/*.workflow`, up to 30 nodes with 5 loops):
  - Every node and edge draws, and every handle resolves to its port (`true`/`false`, the loop's `start`/`done`).
  - Loop bodies sit inside their loops.
  - Drags change only `position` and keep the editor's leftover keys (`measured`, `selected`, `dragging`) byte for byte.
  - Undo restores the saved JSON exactly.
  - Fixed on the way: the dot background packed too tight below about 0.5 zoom, and new and pasted ids didn't follow the builder's snake_case.
- [x] **Node shapes** (2026-11-17): `shape` on `ui-flow-node` covers the builder's diamond `if` and hexagon `switch`, and flowchart shapes generally.
  - Ports and floating edges meet the outline.
  - Gap: a port on a positioned row inside a shaped node (the switch's per-case rows) sits on the row's edge, not on the shape's slanted outline.
- [x] **Tried on the builder's ER diagram** (2026-11-17), as `examples/flow/er.yaml` with the `package_er` converter. Run on the dwigital packages (manufacturing_core, siemens, checklist, launchpad, sample_ecommerce): every table and relationship draws, with no overlaps.
  - Added for it: `filter` with neighbours as context, `flow.fullscreen`, loops for floating self-edges, and packing unconnected groups into rows (the layout used to stack them in one tall column).
- [x] **Tried on client-ui-next's process canvas** (2026-11-17), as `examples/flow/process.yaml`, loading a made-up canvas in the shape the app saves. There is no saved canvas in the repos; they live behind signed URLs.
  - These work: `nodeOrigin [0.5, 0]`, step colours, status icons, the palette, dropping a connection on empty canvas, the side form, cut, copy and paste, delete with a dialog, and undo.
  - Added for it: `bindNode`, `flow.update` and `nodeStyles: false`.
- [ ] **Process canvas: still the host's job.**
  - Keeping a milestone's `dependencies` in step with its edges, and the delete impact check, need code: listen for `connect` / `delete` and `call` the host.
  - New nodes get ids like `custom_1`, where the app uses UUIDs. Idea: an `idPattern` or a host hook for new ids.
- [ ] **Check against a real exported canvas:** export one from the process canvas sidebar and load it into `examples/flow/process.yaml`.
- [ ] **The yard view doesn't need the kit:** a pan/zoom viewport around absolutely positioned elements would cover it.

## Icon kit: packs not drawn yet

Shipped so far, each in an outline and a filled style: `core`, `arrows`, `status`, `logistics`, `warehouse`, `retail`, `manufacturing`, `accounting`, `agriculture`, `automotive`, `aviation`, `banking`, `construction`, `education`, `energy`, `finance`, `healthcare`, `hospitality`, `hr`, `insurance`, `it`, `legal`, `marketing`, `maritime`, `media`, `mining`, `oil-gas`, `public-sector`, `robotics`, `simulation`, `space`, `sports` and `telecom`. Every new icon is drawn from scratch to `crates/irender-icon-kit/GUIDELINES.md` and must pass `tools/icons.py lint` and a contact-sheet review. Draw a pack's first few icons as references, then the rest from them.

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
  - facilities
  - pharma & life sciences
  - sales & CRM
  - support
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
- [ ] **Aliases:** no `previous` → `chevron-left`-style aliases between packs.
- [ ] **Icons worth a second pass:** they pass lint and read, but the metaphor is weak, they're crowded at 16 px, or the filled style lost too much. (o) = outline, (f) = filled.
  - logistics: `freight-train`, `fleet`, `cold-chain`, `seal`, `dimensions`, `eta` (f)
  - warehouse: `bin`, `kitting`, `aisle`, `rack`, `expiry` (f), `serial-number` (f)
  - retail: `variant`, `catalog`, `bundle`, `rating`, `gift-card` (f), `pos-terminal` (f)
  - manufacturing: `lathe`, `tank`, `gears` (the teeth don't mesh), `gloves` (f), `downtime` (f), `safety-goggles` (f)
  - core / status: `eye-off` (f), `offline`, `scheduled`
  - accounting: `calculator` (nearly the same as `finance:budget`), `cost-centre`, `profit-centre`, `petty-cash` (f)
  - agriculture: `weather-station`, `harvester`, `plough`
  - automotive: `tyre`, `brake` (f), `vin` (f); nine front-view car icons differ only by their corner badge
  - aviation: `gate`, `jet-bridge`, `de-icing`, `connecting-flight`
  - banking: `account`, `coins`, `kyc` (f), `loan` (f)
  - construction: `cement-mixer` (f), `progress-photo`, `square-footage`
  - education: `course` (f), `lesson` (close to `book-open`), `language` (f)
  - energy: `substation`, `geothermal`, `load-balancing`
  - finance: `revenue` / `expense` (arrows read as download / upload), `balance-sheet` (f), coin stacks (f)
  - healthcare: `nurse` (cap reads as a chef's hat), `lab-result` (f), `vaccine`
  - hospitality: `parking` uses the letter P; `hotel`, `cake` (f), `allergen`
  - hr: `diversity`, `bonus` (f), `pension` (f), `salary`
  - insurance: `auto-insurance`, `accident` (f), `deductible`, `underwriting`
  - it: `dns`, `vpn`, `app-container` (f, close to `logistics:package`), `network-switch` (f)
  - legal: `mediation`, `settlement`, `evidence`, `clause`
  - maritime: `ais-tracking`, `harbour`, `voyage`, `knot`
  - marketing: `reach`, `brand`, `roi` (f); `banner-ad`, `landing-page` and `social-post` look alike filled
  - media: `magazine` (f), `news-anchor`, `cinema-seat`, `mixing-desk`
  - mining: `wheel-loader`, `explosive-storage`, `shaft`
  - oil-gas: `flare-stack`, `reservoir`, `compressor-station`, `meter-skid`
  - public-sector: `ballot-box`, `disaster-relief`, `town-planning`, `fire-station`
  - robotics: `swarm`, `autonomy`, `agv` (f)
  - simulation: `sandbox`, `optimize`, `physics`
  - space: `astronaut`, `space-helmet`, `thruster`, `docking`
  - sports: `tennis` (reads as a magnifier), `table-tennis` (o), `stadium`, `boxing-glove`
  - telecom: `roaming` (f), `network-outage`, `fibre-optic`

## Housekeeping

- [ ] **CI checks we skipped:**
  - Rust 1.88 (the `rust-version` in `Cargo.toml`): skipped because 1.88 couldn't be installed locally to confirm it passes.
  - `cargo fmt --check`: skipped because the code isn't formatted with default rustfmt. Add a `rustfmt.toml` matching the existing style first.
- [ ] **`examples/build.sh` doesn't delete pages from earlier builds.** It accepts any output folder, so it shouldn't blindly delete one. Clear only `target/site`, or only files it knows it wrote.
