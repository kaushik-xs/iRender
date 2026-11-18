# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-18._

## Suggested next

1. The keyboard-driven UI kit components are all in (data table, date picker, context menu, menubar, navigation menu and the one-time-code input, 2026-11-18). Next candidates: a kanban board, the filters' calendar and presets, a calendar of events, and the rest of the smaller pieces (code editor, currency input, icon rail).

## Gaps you'd hit in real use

- [ ] **CSS-wide keywords in conditional dynamic styles.** A dynamic `inherit`, `initial`, `unset` or `revert` works in a plain declaration (it's set on the property inline), but under `:hover`, a media query or a nested selector it can't reach the property and counts as no value. Idea: for those, compile one rule per keyword keyed on a data attribute.
- [ ] **Older browsers aren't tested.** The browser tests run in Chrome, and in Playwright's WebKit and Firefox (CI, and `tools/playwright/dump-dom.js` locally), which are recent builds (Safari 27.2, Firefox 159 on 2026-11-17). Safari releases and Firefox ESR without anchor positioning, the Popover API, invoker commands, `closedby` or `<details name>` aren't covered; without anchor positioning, popovers open in the middle of the screen.
  - Idea: run the tests in the installed Safari through `safaridriver`.

## UI kit: not built yet

- [ ] **Gaps against client-ui-next** (`~/Documents/GitHub/client-ui-next/components`, compared 2026-11-17). Generic components it uses that no kit has yet, most used first:
  - [x] **Filters** (2026-11-18): `ui-filters` + `ui-filter`, a side panel of list, number, currency, date and text fields with operators, shown as removable chips, writing the chart kit's `filter` format (block data-table-06). Not done yet:
    - [ ] No "Add filter" dialog: every field is listed in the panel. Fine up to a dozen or so fields; past that, a picker of fields to add would help.
    - [ ] Dates use the browser's date input, not the kit's calendar, and there are no relative presets (last 7 days, this quarter) or a quarter type.
    - [ ] List values come from `options` or `rows` only, not from an API as you search, so very long lists (thousands of customers) need their values in state.
    - [ ] Number inputs don't group digits or show the currency symbol while typing.
    - [ ] No saved views (named sets of filters).
  - [x] **Tree view** (2026-11-18): `ui-tree` selects, checks (with partial parents), searches, adds by name, deletes one or the checked, and moves nodes by dragging or Alt and the arrow keys, writing to `bindNodes` and firing events (block tree-01, gallery section). Not done yet:
    - [ ] No renaming in place (F2), and no icons per node.
    - [ ] Children can't load from an API when a node opens, as the data table's nested rows can; very large trees draw every node.
    - [ ] Dragging uses the browser's drag and drop, which touch screens don't start; on a phone, moves need a keyboard or the host.
  - [ ] **Calendar views:** events in month, week, year and list views (`factory-calendar/`). More than shadcn's `calendar`.
  - [x] **File upload and preview** (2026-11-18): `ui-file-upload`, a drop zone with type, size and count checks, uploads with progress, errors and retry, a grid or list, previews of images, PDFs, CSV and .xlsx (read in the browser) and text, and Download all as a zip, without libraries (block expense-01, gallery section). Not done yet:
    - [ ] No thumbnails for PDFs or sheets (client-ui-next draws the first page); they show a type badge.
    - [ ] Word and PowerPoint files aren't previewed, only downloaded; .xls (the old format) isn't read.
    - [ ] Uploads are one request per file: no chunked or resumable uploads, and no direct-to-storage (presigned) uploads.
    - [ ] Download all stores files without compressing them, and holds the zip in memory, so very large sets are slow.
  - [ ] **Gantt chart** in chart-kit: a `type: gantt` shorthand with a table of rows on the left, zoom, collapsible groups and markers with a legend (`charts/GanttChart`). Today it has to be an ECharts `custom` series in `option`.
  - [ ] **Kanban board:** columns of cards moved by drag and drop (`kanban/SimpleDragKanban`).
  - [ ] **Dashboard grid:** tiles that can be dragged and resized (react-grid-layout).
  - [x] **Activity timeline** (2026-11-18): `ui-timeline` + `ui-timeline-item`, with avatars or initials, field changes (old → new), comments and event dots, from children or state (block ticket-01). No grouping by day, and no "show more" for long histories.
  - [ ] **Notification centre:** a bell with an unread count and an inbox (read/unread, archive), plus preferences (`notifications/`).
  - [x] **Progress ring** (2026-11-18): `ui-progress-ring`, following state without a script.
  - [x] **Status badge, rating and password checklist** (2026-11-18): `ui-status-badge` (built-in tones for common statuses, `tones` and `labels`), `ui-rating` (pick, or show a fractional average) and `ui-password-checklist` (rules, strength, `bindValid`; in the signup block). The rating has no half-star picking.
  - [ ] **Smaller pieces still to do:** a JSON/code editor with validation, a grouped number/currency input, and a sidebar icon rail that opens a second panel.
- [x] **Keyboard-driven components** (2026-11-18): data table, date picker, context menu, menubar, navigation menu, one-time-code input.
- [ ] The gallery's Live section needs `python3 examples/serve.py`. Plain `python3 -m http.server` has no live endpoints, so the section shows `error` there.
- [ ] **`ui-data-table` gaps** (shipped 2026-11-18 with sorting, column filters, search, paging, selection with bulk actions, detail rows, a View menu, summaries, formats, badge/progress/sparkline cells, and charts both ways; blocks data-table-01 to -03):
  - [x] Paging options (pages, Load more, infinite scroll), rows from an API in batches (`url`), virtual rows, nested rows with children loaded when opened, and pinned columns (2026-11-18; blocks data-table-04 and -05).
  - [ ] Remote tables: select filters offer only `options` or the values loaded so far, and summaries and `bindView` cover the loaded rows only. Idea: let the API answer facets and totals too.
  - [ ] Nested rows: selecting a parent doesn't select its children; no "expand all" control; the footer counts the rows shown, children included.
  - [ ] Virtual rows: a row's focus is lost when it scrolls out of the drawn window.
  - [ ] No grouped headers, row grouping (`GroupedDataTable`), resizing or reordering columns.
  - [ ] Hidden columns and page size aren't in state or remembered; no saved column views.
  - [ ] Formats are applied by the browser, so the server's HTML shows raw values until the script runs.
  - [ ] The server's starting order compares text without locale rules, so mixed-case text can reorder once the script runs.
  - [ ] Interface text (Rows per page, Select all, Clear filter, …) is English only; only `emptyText` can be changed.
  - [ ] No CSV export; a bulk action can `call` the host with the selected keys.
- [ ] **Date picker gaps** (`ui-date-picker` and `ui-calendar` shipped 2026-11-18: days with a time, months, quarters and years on a financial year, single, range and multiple, limits, disabled days, presets, month and year menus, `bindRange`; the data table's date filter uses the calendar):
  - [x] Typing a date (`input: true`, 2026-11-18): the locale's numeric order, month names, two-digit years, ranges, times, quarters and years, with errors for `ui-field`.
  - [ ] Typing: month names are matched in the locale and in English only; error messages are English.
  - [ ] Times go with single days only; a range has no times, and there's no time zone (values are local wall-clock times).
  - [ ] Interface text (Pick a date, Previous month, Time, preset names) is English only; dates and month names follow `locale`.
  - [ ] No week picker and no "multiple months" mode beyond three months side by side.
- [ ] **Menus** (`ui-menubar`, `ui-navigation-menu`, submenus, checkbox and radio items shipped 2026-11-18):
  - [ ] The navigation menu's panels don't share one animated viewport that morphs between items, as Radix's does; each panel opens on its own.
  - [ ] Submenus open on hover without the "safe triangle", so moving diagonally toward a submenu over another item can close it.
  - [ ] Checkbox and radio items close the menu when chosen; there's no option to keep it open.
- [ ] **`ui-context-menu` gaps** (shipped 2026-11-18):
  - [x] Submenus, checkbox and radio items (2026-11-18), in every menu.
  - [ ] iOS Safari sends no `contextmenu` on a long press, so touch users there can't open it. Idea: open after a 500 ms press.
  - [ ] Right-clicking a data table row doesn't select it; items act on `bind`'s row, and the selection is separate.
- [ ] **`ui-command` gaps:**
  - [ ] Matching is by words, in document order. cmdk ranks fuzzy matches and puts the best first.
  - [ ] No nested pages (cmdk's pattern of an item opening a sub-list, with Backspace going back), and no loading state for items fetched as you type.
  - [ ] No way to read the search text from state, e.g. to fetch items for it. Idea: a `bind` for the query.
- [ ] **`ui-sidebar` gaps:**
  - [ ] No remembered state: a reload opens it as `state` says (shadcn keeps it in a cookie).
  - [ ] No group actions or per-item action buttons (shadcn's `SidebarGroupAction`, `SidebarMenuAction`); put a `ui-dropdown-menu` beside the item instead.
  - [ ] The phone sheet makes the page behind it `inert` but doesn't trap Tab inside the panel against the browser's own UI, and it doesn't swipe closed.
- [ ] **`ui-illustration` gaps** (15 drawings and the error and empty-state blocks shipped 2026-11-18):
  - [ ] More drawings people ask for: no permission to a file, upload failed, too many requests (429), not found in a list ("no matches" for a person or place), a deleted item, an empty calendar or schedule, welcome / onboarding.
  - [ ] The drawings don't animate (a floating planet, a blinking light); they'd need to stop for reduced motion.
  - [ ] With the kit's default near-black primary, the drawings are black and amber; they look their best with a palette or custom colours picked.
- [ ] **More blocks:** shadcn has many more variants of each (sidebar-01 to -16, login-01 to -05, signup-01 to -05). We have dashboard-01, sidebar-03 and -07, login-03 and -04, and one sign-up.
- [ ] **shadcn/ui components still missing, beyond the keyboard-driven ones above:** calendar, drawer (a draggable bottom sheet; `ui-sheet` with `side: bottom` covers the look), resizable panels, item, code block, direction (RTL), and the chat ones not covered by the chat components below (attachment, questionnaire, trace).
- [ ] **`ui-markdown-editor` rich text gaps** (rich text became the default on 2026-11-18):
  - [ ] No toolbar buttons to add a table, image or rule; they come from Markdown (the Markdown tab) or a paste, and a table's cells can then be edited.
  - [ ] Quoting, leaving a quote or code block, and ticking a task change the page directly, so ⌘Z doesn't undo them; the other edits go through the browser's editing and undo.
  - [ ] `maxLength` isn't enforced while typing rich text; the count turns red past it.
  - [ ] No "add link" on a bare pasted URL (it stays text until the link button is used); no headings other than `##` from the toolbar (`#`…`######` typed work).
- [ ] **Chat gaps** (`ui-conversation`, `ui-message`, `ui-chat-input`):
  - [x] Messages show their content as Markdown when irender-markdown-kit is registered, rendered on the server and again as a reply streams (2026-11-18).
  - [ ] No citations or footnotes, and no syntax highlighting in code blocks.
  - [ ] A streamed reply re-renders its whole Markdown on each chunk; unfinished syntax (an open code fence or `**`) shows as text until it closes.
  - [ ] A message's `html` (and `ui`) is kept in state and in an attribute, so long conversations with UI are heavy, and it goes back to the endpoint with `messages`.
- [ ] **irender-wasm:**
  - [ ] Not built or run as WebAssembly yet: this machine's Rust (Homebrew) has no `wasm32-unknown-unknown` target. Its render function and exports are tested natively, and the page's loader against a hand-made module with the same exports; build it once rustup is installed, measure its size, and run the chat block with it.
  - [ ] No icons (the packs are 9.3 MB). Idea: a small default set, or packs fetched on demand.
  - [ ] Renders on the main thread. Idea: a worker for large UIs.
  - [ ] Each message's UI carries its own copy of the styles it uses.
  - [ ] The conversation list in a sidebar can't star, rename or delete a thread on its own; the chat block hands those to the host with `call`.
  - [ ] No message actions (copy, retry, thumbs up/down) or edit-and-resend; put buttons in a `ui-message`'s children for messages written in the document.
  - [ ] No attachments or images in the input, and no tool-call or reasoning ("trace") blocks.
- [ ] **`ui-carousel` gaps:** no slide indicators (dots), autoplay or looping, and the arrows scroll by the first slide's width, so slides of mixed widths step unevenly.
  - [ ] The endpoint gets `{ messages }` only: no extra body fields or headers (auth), and no retry after a failure.
  - [ ] Replies in a thread don't nest further, and a reply can't be sent into a thread from the input.
  - [ ] A streamed reply rebuilds its message on each chunk, so screen readers may announce it more than once.
- [ ] **The built-in `select` has no `multiple`.** `ui-combobox` covers multi-select, and `ui-toggle-group` has `multiple`.
- [ ] **`ui-combobox`:**
  - [ ] No custom option rendering (react-select's `formatOptionLabel` and friends); options take a `description` instead.
  - [ ] No chip animations.
  - [ ] Values are plain values, not option objects. A pre-selected value that only `searchUrl` would return shows its raw value until it's searched for. Idea: a `labels` prop or an initial lookup request.
- [ ] **Theme options:**
  - [ ] Picking and applying them needs irender-ui-kit (its picker and early script), like the mode. Without the kit a page shows the default choices.
  - [ ] A document can't read the current choice from `state` (to show "Rose" in text, say); the picker and `window.__irenderTheme.option()` know it. Idea: an option to mirror choices into state keys.
  - [ ] The kit's icon sizes, 1px borders, the switch's knob and other small fixed sizes don't follow `space.scale`; everything else does.
  - [ ] A swatch shows a choice's light value, in dark mode too.
  - [ ] Custom gradients are always 135° from the colour to its end; a theme's `gradient.*` token can be any CSS background. The built-in `button` (without the kit) ignores gradients.
  - [ ] Custom backgrounds don't change the text colour on them, so a dark card in light mode needs a custom text colour that doesn't exist yet. Idea: derive `color.text` per background like `on-primary`.
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

Shipped so far, each in an outline and a filled style: `core`, `arrows`, `status`, `logistics`, `warehouse`, `retail`, `manufacturing`, `accounting`, `agriculture`, `automotive`, `aviation`, `banking`, `construction`, `education`, `energy`, `finance`, `healthcare`, `hospitality`, `hr`, `insurance`, `it`, `legal`, `marketing`, `maritime`, `media`, `mining`, `oil-gas`, `public-sector`, `robotics`, `simulation`, `social`, `space`, `sports` and `telecom`, plus `brands` (logo marks from Simple Icons, not drawn to the guidelines). Every new icon is drawn from scratch to `crates/irender-icon-kit/GUIDELINES.md` and must pass `tools/icons.py lint` and a contact-sheet review. Draw a pack's first few icons as references, then the rest from them.

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
- [ ] **`brands` gaps:**
  - [ ] No LinkedIn or Microsoft: both asked Simple Icons to remove their logos. Add them from the owners' brand kits after checking their terms.
  - [ ] Marks are single-colour only. Idea: a `brand` colour per mark (Simple Icons has the hex) for `style: { color }`, and multi-colour marks like Google's "G".
  - [ ] Pinned to Simple Icons 15.0.0; the marks change when brands rebrand, so refresh them now and then.
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
  - social: `invite` (f) loses the right of the envelope around the plus; `follow` and `hr:onboarding` are the same drawing
  - telecom: `roaming` (f), `network-outage`, `fibre-optic`

## Housekeeping

- [ ] **Browser tests under heavy load.** The chat tests waited on timers, which Chrome's virtual time runs ahead of a real test server; they now wait on the page (message-channel yields, which hold virtual time) and the budget is 30 s (2026-11-18). With three full runs at once, 1 run in 9 still had a page whose driver didn't finish in time; a single run hasn't failed since. In Firefox (Playwright, real time), a full local run on a busy machine sometimes fails `data_table_tree_and_pinned_columns_in_chrome` or `menubar_and_navigation_menu_in_chrome` (focus moving before the driver reads it); both pass alone and in CI. Idea: wait for the expected focus instead of a fixed delay, as the scroll spy test now does.

- [ ] **`{{ }}` inside a literal `set_state` value is never evaluated, and nothing warns.** `value: { description: "Hi {{ name }}" }` stores the braces as text, while `submit` `body` fields are evaluated. Found by a model writing a toast. Fix: evaluate templated leaves of literal values (as `body` does), or at least warn in `validate`. Then drop the caveat from `skills/irender/references/authoring.md`.
- [x] **`tools/llm-docs.sh --check` runs in CI** (2026-11-18): a change that doesn't regenerate the skill's references and `llms-full.txt` fails the lint job. Run `tools/llm-docs.sh` after changing a component, icon or README.
- [ ] **MCP server (`irender mcp`), deferred.** Agents with a shell already get everything from the skill and the CLI, so an MCP server only pays off for chat clients without a shell (Claude Desktop, claude.ai, IDE chat) and for apps whose own agent generates UI at runtime. If built, it would be a stdio subcommand of the CLI with these tools:
  - `validate`, with kits and `untrusted`
  - `component`, for one component's props, so the 40 KB listing stays out of context
  - `search_icons`
  - `render`, returning HTML or a preview file path
  - `schema`

  It would also offer `authoring.md` as a resource.
- [x] **CI builds on Rust 1.88**, the `rust-version` in `Cargo.toml` (2026-11-18). No dependency needs newer, and clippy's `incompatible_msrv` passes, but 1.88 itself couldn't be installed locally, so the job's first run is its first test.
- [ ] **`cargo fmt --check` isn't in CI.** The code is formatted by hand and no `rustfmt.toml` matches it: the closest (`max_width = 160`, `use_small_heuristics = "Max"`, `reorder_imports = false`) still rewrites about 2,900 lines in 56 files. Adding it means one reformatting commit, best made when no other work is in flight.
- [x] **`examples/build.sh` clears pages from earlier builds** (2026-11-18), but only in a folder it built itself (marked with `.irender-site`); any other folder is left as it is.
