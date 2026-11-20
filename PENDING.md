# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-19._

## Suggested next

1. The Gantt chart (with a keyboard-ready table), the dashboard grid, touch dragging, saved data table views, grouping for tables from an API and quarter presets are in (2026-11-19–20). Next: `cargo fmt` in CI (one reformatting commit).

## Gaps you'd hit in real use

- [ ] **CSS-wide keywords in conditional dynamic styles.** A dynamic `inherit`, `initial`, `unset` or `revert` works in a plain declaration (it's set on the property inline), but under `:hover`, a media query or a nested selector it can't reach the property and counts as no value. Idea: for those, compile one rule per keyword keyed on a data attribute.
- [ ] **Touch dragging isn't tried on real phones.** The kanban board, tree and flow palette pick an item up after a 300 ms hold (2026-11-19); that's tested with Chrome's emulated touch input and with touch pointer events in Chrome, WebKit and Firefox, not on an iPhone or an Android phone.
- [ ] **Older browsers aren't tested.** The browser tests run in Chrome, and in Playwright's WebKit and Firefox (CI, and `tools/playwright/dump-dom.js` locally), which are recent builds (Safari 27.2, Firefox 159 on 2026-11-17). Safari releases and Firefox ESR without the Popover API, invoker commands, `closedby` or `<details name>` aren't covered. Anchor positioning has a fallback script (2026-11-20), tested by turning the browser's own off; `anchor()` and `anchor-size()` in a document's own CSS aren't handled by it.
  - Idea: run the tests in the installed Safari through `safaridriver`.

- [x] **Charts with `series`: `sort` and `limit` rank categories by their totals** (2026-11-20), keeping each one's series together; report-01's Top N works on split charts.
- [x] **Chart value labels are formatted** (2026-11-20): `format` (number, currency, percent), `currency` and `decimals`, for labels, tooltips and the value axis; at most two decimals by default.
- [x] **Lookup by a computed key in expressions** (2026-11-20): `names[dim]`, `rows[i + 1].id`; report-01's title uses it.

## UI kit: not built yet

- [ ] **Gaps against client-ui-next** (`~/Documents/GitHub/client-ui-next/components`, compared 2026-11-17). Generic components it uses that no kit has yet, most used first:
  - [x] **Filters** (2026-11-18): `ui-filters` + `ui-filter`, a side panel of list, number, currency, date and text fields with operators, shown as removable chips, writing the chart kit's `filter` format (block data-table-06). Not done yet:
    - [ ] No "Add filter" dialog: every field is listed in the panel. Fine up to a dozen or so fields; past that, a picker of fields to add would help.
    - [x] Date presets (today, last 7 days, this month, this quarter, …) and saved views (`bindViews`) (2026-11-18).
    - [x] Dates pick on the kit's calendar, and amounts are grouped as typed with the currency's symbol (2026-11-18). No quarter type yet.
    - [ ] List values come from `options` or `rows` only, not from an API as you search, so very long lists (thousands of customers) need their values in state.
  - [x] **Tree view** (2026-11-18): `ui-tree` selects, checks (with partial parents), searches, adds by name, deletes one or the checked, and moves nodes by dragging or Alt and the arrow keys, writing to `bindNodes` and firing events (block tree-01, gallery section). Not done yet:
    - [x] Renaming in place (`renamable`: F2, a double click), icons per node, and children loaded from `childrenUrl` when a node first opens (2026-11-20).
    - [ ] Very large trees draw every node (no windowing). Search doesn't look inside children that haven't loaded.
  - [x] **Calendar views** (2026-11-18): `ui-event-calendar`, events in month, week (hours, overlaps side by side) and list views, coloured by category, with `bindEvent`, `bindDay` and the keyboard (block calendar-01, gallery section). Not done yet:
    - [x] Year and day views (2026-11-19).
    - [x] Events move by dragging (to another day, or time in the week and day views, and resized by their foot) or with Alt and the arrow keys, written back with `bindEvents` (2026-11-19). Adding is still up to the page (`bindDay`), and the year and list views don't move events.
    - [ ] No recurring events: each one is listed. No time zones: times are local to the browser.
  - [x] **File upload and preview** (2026-11-18): `ui-file-upload`, a drop zone with type, size and count checks, uploads with progress, errors and retry, a grid or list, previews of images, PDFs, CSV and .xlsx (read in the browser) and text, and Download all as a zip, without libraries (block expense-01, gallery section). Not done yet:
    - [ ] No thumbnails for PDFs or sheets (client-ui-next draws the first page); they show a type badge.
    - [ ] Word and PowerPoint files aren't previewed, only downloaded; .xls (the old format) isn't read.
    - [ ] Uploads are one request per file: no chunked or resumable uploads, and no direct-to-storage (presigned) uploads.
    - [ ] Download all stores files without compressing them, and holds the zip in memory, so very large sets are slow.
  - [x] **Gantt chart** (2026-11-19): `ui-chart` `type: gantt`, a table of tasks beside their bars, groups that fold, progress, `after` arrows, milestones, `markers` and today, colours by `series` with a legend, zoom buttons, and a height that follows the rows (block gantt-01, charts gallery). Not done yet:
    - [x] The table is an HTML tree grid over the chart (2026-11-20): rows read out and moved through with the keys, groups opened with Right, Left and Enter, tasks picked with Enter.
    - [x] Editing (2026-11-20): bars dragged to move or stretch them (a hold first by touch), Alt and the arrows on a row, written back to `bindData`; tasks that start before what they wait for ends are marked. Links drawn from a bar's dot and removed by clicking their arrow, and `cascade` moving the tasks that wait on a moved one (2026-11-20). Not done yet: dates snap to whole days only; cascade only pushes later, never pulls tasks earlier.
    - [ ] No scrolling inside the chart: a long plan is as tall as its rows. No critical path, baselines or working days (weekends count).
    - [x] The axis's dates are in the document's `lang` (2026-11-20).
  - [x] **Kanban board** (2026-11-18): `ui-kanban`, cards dragged between columns or moved with Alt and the arrow keys, written to `bindCards` with `move` events; limits, folding, Add card, and `bindCard` following its card (block kanban-01, gallery section). Not done yet:
    - [x] Swimlanes by a card field, card `fields` (text, badge, progress, date) and moving columns by a grip (2026-11-20; block kanban-01). Not done yet: a card is still drawn by the board, not from a template of the page's own components; a lane folds but can't be moved; moving a column has no touch drag (the grip's keys work). `swimlanes` and `fields` may use state (2026-11-19; block crm-01's Board settings).
    - [x] `filter` on the board (2026-11-20), as the data table and charts have; crm-01's board has a filter panel. Applied in the browser only: the server's first drawing shows every card.
  - [x] **Dashboard grid** (2026-11-19): `ui-dashboard-grid` + `ui-tile`, tiles dragged by their header and resized from their corner (pointer events, so touch too) or with the arrow keys, pushing the others aside and floating up, locked tiles, the layout in `bind`, and one column on a narrow grid (block dashboard-02, gallery section). Not done yet:
    - [x] A layout per width (2026-11-20): `breakpoints` with each size's columns, tiles' `layouts` per size, the rest worked out from the widest, all in the page's CSS, and `bind` keeping each size's (block dashboard-02's tablet layout).
    - [ ] Tiles come from the document, not from a list in state, so a dashboard can't add or remove tiles on its own; show and hide them with `if` around the content.
    - [ ] Only vertical packing: no horizontal compaction, and tiles can't be dropped in from outside the grid (a palette of widgets).
  - [x] **Activity timeline** (2026-11-18): `ui-timeline` + `ui-timeline-item`, with avatars or initials, field changes (old → new), comments and event dots, from children or state (block ticket-01). No grouping by day, and no "show more" for long histories.
  - [x] **Notification centre** (2026-11-18): `ui-notifications`, a bell with an unread count and a panel (or `inline`) with Unread and All, read toggles, archive and Mark all as read, from state (dashboard-01 header, gallery section). Not done yet:
    - [ ] No notification preferences (which kinds, by email or in the app), and no grouping by day.
    - [ ] Nothing arrives by itself: new notifications need the page's state to change (a `ui-live-data` feed, or the host).
  - [x] **Progress ring** (2026-11-18): `ui-progress-ring`, following state without a script.
  - [x] **Status badge, rating and password checklist** (2026-11-18): `ui-status-badge` (built-in tones for common statuses, `tones` and `labels`), `ui-rating` (pick, or show a fractional average) and `ui-password-checklist` (rules, strength, `bindValid`; in the signup block). The rating has no half-star picking.
  - [x] **Number and currency input** (2026-11-18): `ui-number-input`, grouped as typed in the locale's way, with a currency symbol, prefix or suffix, steps and clamping.
  - [x] **Sidebar icon rail** (2026-11-19): `ui-rail-layout` + `ui-rail-panel`, a rail of icons each opening its own panel, with the section and the panel's open state in state (block workspace-01, gallery section). The panel resizes by dragging its edge (2026-11-19). The rail can't be on the right.
  - [x] **Resizable panels** (2026-11-19): `ui-resizable` + `ui-resizable-panel`, dragging and the keys, limits, collapsible panels, sizes in state (block mail-01, gallery section). Sizes are percentages only (no fixed px panel).
  - [x] **Code block and JSON editor** (2026-11-19): `ui-code-block` (highlighting for ten languages, line numbers, marked lines, copy) and `ui-json-editor` (checks as you type with line, column and reason, Format, value to state while valid). In docs-01 and the settings block's Webhooks. Not done: a general code editor (other languages, a schema to check JSON against, folding), and the highlighter is a tokenizer, so nested languages (JS in HTML) stay plain.
  - [x] **Drawer** (2026-11-19): `ui-drawer`, a bottom sheet dragged down (or flicked) to close, in the dialog family (block mail-01's Compose, gallery). No snap points (half-open, full), and like the other dialogs it opens from a button, not from state.
  - [x] **Interface text in other languages** (2026-11-19): a document's `lang` and `labels` give the kits' fixed words in its language, keyed by the English, with plural forms by `Intl.PluralRules`; every key is listed in skills/irender/references/labels.md (generated by tools/labels.py); block i18n-01 is a page in Spanish. Not done: no ready-made translations (each document gives its own), the server knows the plural rules and number formats of the common languages only (others as English), and the chart kit's numbers follow the browser, not `lang` (2026-11-19: `locale` defaults to `lang`).
  - [x] **Right to left** (2026-11-19): `lang` of a right-to-left language sets `dir="rtl"`; the UI and Markdown kits use logical CSS (inline start and end), mirror their directional icons and popovers, and swap Left and Right in keys and drags. Not done: a block's own styles (`margin-left` in a document) aren't converted; English text inside right-to-left pages keeps the browser's bidi ordering (".Acme Inc"); and the chart kit doesn't mirror (axes stay left to right).
- [x] **Keyboard-driven components** (2026-11-18): data table, date picker, context menu, menubar, navigation menu, one-time-code input.
- [ ] The gallery's Live section needs `python3 examples/serve.py`. Plain `python3 -m http.server` has no live endpoints, so the section shows `error` there.
- [ ] **`ui-data-table` gaps** (shipped 2026-11-18 with sorting, column filters, search, paging, selection with bulk actions, detail rows, a View menu, summaries, formats, badge/progress/sparkline cells, and charts both ways; blocks data-table-01 to -03):
  - [x] Paging options (pages, Load more, infinite scroll), rows from an API in batches (`url`), virtual rows, nested rows with children loaded when opened, and pinned columns (2026-11-18; blocks data-table-04 and -05).
  - [x] Remote tables: facets and totals (2026-11-20): a query's first batch asks `facets=` and `summary=`, and the API's `facets` and `summary` fill the select filters (every value, with counts) and the footer. `bindView` still holds only the rows loaded so far.
  - [x] Nested rows (2026-11-20): picking a row picks the rows under it (part-picked parents, rows loaded later under a picked one), Expand all and Collapse all in View, `*` for siblings, and a footer counting every loaded row.
  - [x] Virtual rows keep a row's focus when it scrolls out of the drawn window (2026-11-20): the scroll area holds it, and the row gets it back.
  - [x] Grouped headers (`group`), row grouping with counts, subtotals and folding (`groupBy`, Group by in View), resizing and moving columns (2026-11-19; block data-table-07). Grouping doesn't work with virtual or nested rows; groups don't nest; a header row's checkbox doesn't select its group.
  - [x] Grouping rows from an API (2026-11-20): `groupBy` with `url` asks the API for a row per group (count and subtotals) and loads a group's rows when it's opened. A group's rows load a batch at a time, with a Load more row after them (2026-11-20); with `paging: scroll` they still wait for that row rather than loading as you scroll.
  - [x] Column order, widths and hidden columns are state with `bindColumns` (2026-11-19).
  - [x] Saved views (2026-11-20): a Views menu with ready-made `views` and people's own in `bindViews` (sort, filters, search, columns, grouping, page size), "(edited)" with Undo and Save changes, rename and delete (block data-table-01). Not done yet: no default chosen from the menu (only `defaultView`), no sharing a view by link, and views aren't ordered by dragging.
  - [ ] Formats are applied by the browser, so the server's HTML shows raw values until the script runs.
  - [x] The server's starting order is the browser's (2026-11-20): numbers by value, case and Latin accents aside, checked against Chrome, WebKit and Firefox. Scripts other than Latin (and language rules such as Swedish å after z) still follow code points.
  - [x] Interface text follows the document's `labels` (2026-11-19).
  - [x] CSV export of the rows that pass, in the columns shown (2026-11-19). A remote table exports only the rows it has loaded; no Excel (.xlsx) file.
- [ ] **Date picker gaps** (`ui-date-picker` and `ui-calendar` shipped 2026-11-18: days with a time, months, quarters and years on a financial year, single, range and multiple, limits, disabled days, presets, month and year menus, `bindRange`; the data table's date filter uses the calendar):
  - [x] Typing a date (`input: true`, 2026-11-18): the locale's numeric order, month names, two-digit years, ranges, times, quarters and years, with errors for `ui-field`.
  - [ ] Typing: month names are matched in the locale and in English only; error messages are English.
  - [ ] Times go with single days only; a range has no times, and there's no time zone (values are local wall-clock times).
  - [x] Interface text follows the document's `labels`; dates and month names follow `locale` (2026-11-19).
  - [ ] No week picker and no "multiple months" mode beyond three months side by side.
  - [x] Quarter presets (2026-11-20): `quarters` (the last four, named by their financial year), `lastQuarter`, `lastYear` and `yearToDate` in the calendar, the filters panel (with its own `fiscalStart`) and the data table's date filter (with the column's `presets` and `fiscalStart`).
- [ ] **Menus** (`ui-menubar`, `ui-navigation-menu`, submenus, checkbox and radio items shipped 2026-11-18):
  - [ ] The navigation menu's panels don't share one animated viewport that morphs between items, as Radix's does; each panel opens on its own.
  - [x] Submenus keep a "safe triangle" (2026-11-20): heading diagonally for an open submenu across other items doesn't close it; stopping on another item does, after 400ms.
  - [x] Checkbox and radio items can keep the menu open (`keepOpen`, 2026-11-20).
- [ ] **`ui-context-menu` gaps** (shipped 2026-11-18):
  - [x] Submenus, checkbox and radio items (2026-11-18), in every menu.
  - [x] A long press (500ms) by touch opens it, for iOS Safari, which sends no `contextmenu` (2026-11-20).
  - [x] Right-clicking a data table row selects it (2026-11-20), or keeps the selection on a selected row, as file lists do; block data-table-01's row menu acts on the selection.
- [ ] **`ui-command` gaps:**
  - [x] Ranked, fuzzy matching (2026-11-20): best match first in each group and of the groups, letters in order, keywords, matched letters marked; `rank: false` for document order.
  - [x] Pages (`ui-command-page`, an item's `page`, Backspace back) and `bindSearch` (2026-11-20). A loading state for items fetched as you type: `loading` and `loadingText` (2026-11-20; gallery).
- [ ] **`ui-sidebar` gaps:**
  - [x] `remember` keeps it open or closed across reloads, in localStorage (2026-11-20; blocks sidebar-07, dashboard-01). The server can't read it (shadcn's cookie can), so a page drawn closed but remembered open opens as its script runs.
  - [x] Group and item actions (2026-11-20): `ui-sidebar-action`, a button or a ⋯ menu, shown always or on hover (block sidebar-07, gallery).
  - [x] The phone sheet keeps Tab inside the panel and swipes closed (2026-11-20). Tried with touch pointer events in Chrome, WebKit and Firefox, not on a real phone.
- [ ] **`ui-illustration` gaps** (15 drawings and the error and empty-state blocks shipped 2026-11-18):
  - [x] Seven more drawings (2026-11-20): `no-access`, `upload-failed`, `too-many-requests`, `no-people`, `deleted`, `empty-calendar`, `welcome` (gallery, block empty-states).
  - [x] `animate` (2026-11-20): parts of twelve drawings float, blink, turn, twinkle, drift or wobble, and stop for reduced motion (gallery's Animate switch, block error-404).
  - [ ] With the kit's default near-black primary, the drawings are black and amber; they look their best with a palette or custom colours picked.
- [ ] **More blocks:** shadcn has many more variants of each (sidebar-01 to -16, login-01 to -05, signup-01 to -05). We have dashboard-01 and -02, sidebar-03 and -07, login-03 and -04, and one sign-up; and app pages beyond shadcn's: a report builder (report-01), an incident room (incident-01) and a sales pipeline (crm-01).
- [ ] **shadcn/ui components still missing, beyond the keyboard-driven ones above:** item, and the chat ones not covered by the chat components below (attachment, questionnaire, trace).
- [ ] **`ui-markdown-editor` rich text gaps** (rich text became the default on 2026-11-18):
  - [ ] No toolbar buttons to add a table, image or rule; they come from Markdown (the Markdown tab) or a paste, and a table's cells can then be edited.
  - [ ] Quoting, leaving a quote or code block, and ticking a task change the page directly, so ⌘Z doesn't undo them; the other edits go through the browser's editing and undo.
  - [ ] `maxLength` isn't enforced while typing rich text; the count turns red past it.
  - [ ] No "add link" on a bare pasted URL (it stays text until the link button is used); no headings other than `##` from the toolbar (`#`…`######` typed work).
- [ ] **Chat gaps** (`ui-conversation`, `ui-message`, `ui-chat-input`):
  - [x] Messages show their content as Markdown when irender-markdown-kit is registered, rendered on the server and again as a reply streams (2026-11-18).
  - [x] Code blocks are highlighted, with a copy button, in messages and everywhere else `ui-markdown` shows Markdown, and in the editor's preview (2026-11-19). Not in the rich text editor itself, where the code stays plain while you edit it; and a streamed reply's fence is highlighted again on each chunk.
  - [ ] No citations or footnotes.
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
- [x] **`ui-carousel`** (2026-11-20): dots, `loop`, `autoplay` with a pause button, and stepping to each slide's own start. Not done yet: no fade transition (it always scrolls).
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

- [ ] **Browser tests under heavy load.** The tests wait on the page rather than fixed times: chat and server-backed tests (combobox, remote and lazy data tables) wait for the answer with message-channel turns, which hold Chrome's virtual time still, and the menubar and data table tree tests `settle()` until the page stops changing (2026-11-19). Two full Firefox runs at once now pass (4 of 4; before, 2 of 4 failed). Left: with three full Chrome runs at once, the chat threads page sometimes uses up its 30 s virtual-time budget (about 1 run in 3 at that load; a single run, as in CI, hasn't failed).
- [x] **`{{ }}` inside an action's object or list is filled in** (2026-11-20): a `set_state` value, `submit`'s `body` fields and `call`'s `args`, however deep, are built when the action runs (Rust and the browser runtime agree). Component props are left to their components.
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
