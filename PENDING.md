# Pending

Work we deferred on purpose, and known limitations. Keep it current: tick or remove an item when it lands, and add new ones as they come up. Newest decisions go at the top of each section.

_Last updated: 2026-11-28._

## Suggested next

1. Data sources (records joined by id from APIs that can't be joined on the server) landed on 2026-11-27, phases 1 to 4: in memory, kept on the device per person with private fields encrypted under the app lock's key, queries synced by the server's cursor, and changes made offline, sent with their version and settled when they clash. The gaps are under Data sources below; next, trying it all on real phones.
2. The twin kit (`ui-twin`, 3D digital twins with machine and sensor data) landed on 2026-11-23, history on readouts, big worlds, simpler models far away, colours by value as instances, alarm tags that don't pile up, several objects at once and measuring on 2026-11-26: next, trying it on phones and tablets (needs real devices), walls and zones drawn by hand, and their corners dragged, came on 2026-11-28 (see Twin kit next phases).
3. Thirty-seven components for installable apps and devices, and seat-plan layouts with floors, landed on 2026-11-22 after forty-two others (below). The device ones need trying on real phones and devices. Next: their gaps and the older ones under it, most used first; then `cargo fmt` in CI (one reformatting commit).

## Gaps you'd hit in real use

- [ ] **Data sources** (`ui-data-source`, `ui-data-query`, `expand` on `ui-data-table`; 2026-11-27; gallery, blocks data-table-09 and clinic-01): records fetched many ids a call (GET or POST) or one at a time, ids asked for together sent together and each once, kept in memory for `ttl`, the service worker leaving them alone. Not done yet:
  - Joined fields are the page's: a remote table sends their sort to its API (nothing stops a joined column being sortable there). In a table of rows, search, filters, sorting, grouping, `bindView` and the context menu's row see them (2026-11-28), asking for every row's records once one is used. Export has them only for records already fetched. `expand` inside a joined record came on 2026-11-28 (three levels at most). No `expand` with editable cells, or with grouping a table of rows (its rows are drawn by the server).
  - `invalidates` on `submit` (2026-11-28; block data-table-09) names the records a save changed: ids by source, or whole sources. Not done: no tags from the server (it can't say what changed), a query's rows from `url` aren't named by it (their `refresh` is), and nothing in the answer can be used for the ids (a record made by the save is named by the page's state, not by its id in the answer).
  - A failed joined cell has a retry button and an id with no record is remembered for `missingTtl` (2026-11-28); a column template of its own draws its own (`row.$failed`), and a query of `source` has no retry but `refresh`. `If-None-Match` for `one` and for queries from `url` (2026-11-28), not for a batch (its answer has no ETag per record).
  - Kept on the device (phase 2, 2026-11-27; block clinic-01): `version`, `maxAge`, `maxRecords` and a query's `keep` bound what's kept, and `navigator.storage.persist()` is asked for (2026-11-28); nothing measures bytes (a few huge answers aren't let go for their size). A PIN-wrapped key is only as strong as the PIN (4 to 8 digits: easy to try every one offline); the passkey's PRF isn't tried on real devices (headless Chrome has no PRF), and without it a passkey unlock opens no key. Each tab unlocks its own; another tab's lock doesn't open this one's key. The service worker can't read sealed fields, so a background sync can't either.
  - A source named in `expand` or a query's `source` and declared nowhere fails once the page has loaded (`$failed`, a query's `error`, and a warning in the console naming it), and is fetched if it's declared later (2026-11-28). It's found only in the browser: rendering doesn't check the names, as a source may be in a part of the page added later.
  - Sync (phase 3, 2026-11-27; block clinic-01): queries only. A source's records follow `ttl` (and `ETag` for `one`), with no `since` of their own; a paged table's `url` has no sync. Rows merged from a delta keep their places, new ones go at the end (no re-sorting). Without tombstones or a `deleted` list, `sync.ids` (2026-11-28) asks for the ids there are now at each delta (all of them: no paging of ids), else deletions wait for the next `full` load. A 401 only fires `unauthorized`. Fetched while locked, rows are kept without their private fields (no key), and the next unlocked load fetches them in full again.
  - Writes (phase 4, 2026-11-27; `write`, `ui-data-conflicts`, block clinic-01): to a source's records only, through `submit` or fetch with a JSON body (no forms or files). A query's rows show them when it names the source as its `records` (2026-11-28): matched by the source's `key`, new ones at the end (not sorted or filtered as the server would), and `total` isn't changed. Seen in a test (2026-11-28): writes to another origin from a page opened from a file stayed waiting; not looked into. A temporary id is replaced in the source and in writes still waiting, not in other records or rows that name it. Kept with private fields only while unlocked (else in memory, lost with the page); the service worker doesn't send them in the background. Without versions from the server, the last write wins. Editable table cells write through a source with `records` (2026-11-28): a field at a time (a nested field sends its whole top-level field), undo doesn't take back what was sent, and an added row stays under its own key in the table (`new-1`) after it's saved.
  - Seen once under load (2026-11-27): the join test's "an id was asked for twice" failed in one run of the data tests, and not in the next eight; maybe a failed call retried after 15 seconds. Not reproduced.

- [ ] **Pareto charts** (`type: pareto`, `threshold`; 2026-11-26; chart gallery, block incident-01): `horizontal` lays it down the side (2026-11-26; chart gallery's Pareto across). Stacked by `series` (2026-11-28; chart gallery's Pareto by channel). Other in its place by size (`other: sorted`) and more lines on the percent axis (`targets`, each naming the bar it's reached at) came on 2026-11-28 (chart gallery's Pareto against targets, incident-01). Not done: a target's line is the same dotted style for all (no tone of its own), and Other can't be put first.

- [ ] **`ui-iframe`** (2026-11-25; gallery, block docs-01): loading, ready and failed (`bindStatus`, `timeout`), `height: auto` from a message `{ height }` (or measured on this site), and messages both ways (`bindMessage`, `send`) came on 2026-11-28. A site that refuses to be framed often still loads, blank, so it shows as ready; no message format other than `{ height }` (iframe-resizer's isn't read). The untrusted policy refuses it unless it allows sandboxed frames (`with_sandboxed_frames`, `--sandboxed-frames`; 2026-11-28): then a fixed `https:` address, a sandbox without `same-origin` or ways out, and four features. Not done there: an address from state (the runtime would have to check `https:` as it sets it) and an allowlist of sites.

- [ ] **Marked days on `ui-event-calendar`** (`days`, `yearEvents`; 2026-11-24; gallery, block calendar-01): a shade in a tone, a label and badges per day, apart from events. Marks repeat (`repeat`, as events) and a badge with an `id` is a button with `bindBadge` (2026-11-27; calendar-01): the focused day's badge buttons are in the tab order (2026-11-28), and there's no slot for arbitrary content in a day: the label and badges are text.

- [ ] **`ui-mindmap`** (2026-11-24; gallery, block brainstorm-01): one long line per topic (long labels are cut short with … and shown whole on hover), icons and notes (2026-11-27), and links on topics, lines between branches (`links`), reordering by dragging or Alt and the arrows, and a tree from the top (`layout: down`) on 2026-11-28 (brainstorm-01). Pictures on topics (`image`), lines between topics drawn, picked, labelled and removed on the map (Alt+L, a click, the list under it), and round the middle (`layout: radial`) came on 2026-11-28 (gallery's second map). Not done: a picture isn't added from the map (only in state, and no upload), a line's label is written under the map (not on it), and in the radial layout dragging a topic onto another's side places it by its top or foot like the others.

- [ ] **`ui-whiteboard`** (2026-11-23; gallery, block retro-01): arrows joined to shapes (2026-11-27), and frames, pictures, grouping, front and back, locking, copy and paste between boards, resizing drawings and PNG export (2026-11-28; retro-01). One person at a time (several people's edits from a server just replace the items: no merging and no live cursors); arrows join straight (no elbows or curves); their ends are dragged onto another shape to join it, pinned to a spot on its edge when let go near it (`fromAt`, `toAt`, 2026-11-28), but not from the keyboard; pictures are kept in state as their data (big ones make big state), and the undo history is the page's only.

- [ ] **`ui-approval`** (2026-11-23; gallery, block expense-01): stages side by side (`parallel`) and taking back a handoff came on 2026-11-28 (expense-01), after handing a turn on, due dates and earlier rounds (2026-11-27). Nothing sends reminders, and the people to hand to are the page's list, not searched. Who may decide is shown, not enforced: the server checks it.

- [ ] **`ui-file-browser`** (2026-11-23; gallery, block drive-01): folders load from `url`, again with Refresh or `refresh`, a page at a time (`{ items, next }`), and uploads to `uploadUrl` show their progress and can be tried again (2026-11-28). The next page comes by scrolling too (`loadOnScroll`, 2026-11-28); a refresh asks for the first page only, and a failed upload is kept for retry only while the page is open. No copy and paste, no previews beyond `thumb`, and no sharing or permissions. A box dragged round items selects them, with a mouse (2026-11-28; by touch a drag scrolls); ⌘/Shift and the keys too.

- [ ] **`ui-virtual-list`** (2026-11-23; gallery, block inventory-01): a grid of cards (`columns`, `minItemWidth`), `scrollTo` from state landing on the item, and the group at the top (`groupBy`) came on 2026-11-28. The group is a line of text over the items (no headers between groups in the list, no template of its own), a grid's rows are as tall as their tallest card's average, and a jump past items never drawn can take a few steps to land. Find in page (⌘F) only finds what's drawn.

- [ ] **`ui-presence`** (2026-11-23; gallery, block workspace-01): it shows who's in state; keeping that up to date (a WebSocket, polling, a presence service) is the page's job, and there's no example server. No live cursors or selections in a document. Following someone (`bindFollow`, 2026-11-28; gallery) says whom; going there is the page's.

- [ ] **`ui-timeline` as a feed** (`group`, `relative`, `since`, `limit`; 2026-11-23; gallery, block workspace-01): entries must come in time order; filters by person and kind, and Load more with `hasMore` (2026-11-27; workspace-01); several at once from state, a `filter` event (and `more` with the filter) for a server to filter, and `bindSeen` for next visit's `since` (2026-11-28). Several from the menus with `filters: several` (tick boxes, 2026-11-28; workspace-01); the page fetches the older entries itself, and `bindSeen` is written as the page is hidden, which a phone may never get to before it's closed. Days are the browser's local days.

- [ ] **`ui-compare-slider`** (2026-11-23; gallery, block job-report-01): up to four pictures (`pictures`, a divider between each two) and zooming into all together (`zoom`, `bindZoom`) came on 2026-11-28 (gallery; job-report-01 zooms); a fade between two is `mode: fade`. No video, no fade across three or more, and zoomed, the view is moved by dragging only (not the keys, which move the dividers). On a phone the picture is a range input, so a sideways swipe moves the divider rather than the page; tried in Chrome, not on real phones.

- [ ] **`ui-countdown`** (2026-11-23; gallery, block ticket-01): the browser's clock is trusted (a server's time isn't asked for), only the two largest units. Pausing (`paused`, 2026-11-26; gallery): the pause moves the deadline on the page only; the server keeps it by setting a new `to`. Working time (`workdays`, `hours`, `holidays`, 2026-11-27; gallery): hours past midnight and hours by weekday came on 2026-11-28; one stretch of hours a day (no lunch break), and no time zones other than the browser's. Times from state are read as the browser reads dates: one without a zone (`2026-12-01T17:00`) is local time.

- [ ] **`ui-form-builder`** (2026-11-23; gallery, block form-designer-01): conditions of several fields matched all or any (and is not), field widths, phone, time, rating and file fields, undo and redo, and its own document read back (as JSON) came on 2026-11-28 (form-designer-01). Groups inside the conditions (`(a) and (b or c)`, two deep in the inspector; any depth in state) came on 2026-11-28 (form-designer-01); a group is turned round (`not`) only in state or a document, not in the inspector. A document edited by hand reads back only if it keeps the shape this makes, and YAML doesn't (no YAML reader in the browser). Dragging from the palette is the browser's drag and drop, so by touch a type is added with a tap.

- [ ] **`ui-sortable-list`** (2026-11-23; gallery, block settings): lists in a `group` share items (dragged across, or ←/→ once picked up), the others make way as one's dragged, and moves glide (2026-11-28; gallery). Items inside items (`childrenField`, `maxDepth`; 2026-11-28; gallery's outline): drawn by the script with each item's label only (no children templates there), and not in a `group`. A list in a group is found by the page's order (← and → go to the one before or after it in the page, whatever the layout), and dragging is tried with pointer events in Chrome, WebKit and Firefox, not on real phones.

- [x] **Gauge and progress ring arcs in Firefox** (2026-11-24): their arc's `stroke-dashoffset` was a unitless `calc()`, which Firefox drops (to 0), so both showed full there; now `calc(… * 1px)`, the same value in Chrome, WebKit and Firefox. Shipped in 0.9.0 without the fix.
- [ ] **`ui-gauge`** (2026-11-23; gallery, block dashboard-02): `min`, `max` and the bands follow state too (2026-11-28; gallery), with a small script for the ends, tick numbers and bands (the value and target need none); ticks (`ticks`) and a figure rounded to `decimals`, from state too (2026-11-27; with the expressions' new `round`); tick numbers round the outside (`tickLabels`, 2026-11-28; gallery), and a needle's figure sits under the hub. The chart kit's `gauge` covers the rest.

- [ ] **`ui-color-picker`** (2026-11-23; gallery, block report-01): recent colours (`recent`, kept in the browser) and a contrast check (`contrastWith`) came on 2026-11-28 (gallery); `format: oklch` writes OKLCH and any `oklch()` typed is read (2026-11-28; gallery), but colours are picked in sRGB: a wider one comes back cut to it, and there's no Display P3 or other wide-gamut picking, and the eyedropper only where the browser has one (Chrome and Edge). The area isn't tried with touch on real phones.

- [x] **CSS-wide keywords in conditional dynamic styles** (2026-11-28): under `:hover`, a media query or a nested selector, each dynamic declaration gets a rule per keyword keyed on a `d<n>=<keyword>` token in the element's `data-i-kw`, set by the server and the runtime. Five rules more for each such declaration: a page with many of them has a little more CSS.
- [ ] **Touch dragging isn't tried on real phones.** The kanban board, tree and flow palette pick an item up after a 300 ms hold (2026-11-19); that's tested with Chrome's emulated touch input and with touch pointer events in Chrome, WebKit and Firefox, not on an iPhone or an Android phone.
- [ ] **`data_table_remote_groups` in Firefox** (seen 2026-11-28): failed once at "searched" while 23 data tests ran together, and passed three times alone. Not looked into.
- [x] **Twin kit browser tests outside Chrome** (2026-11-28): WebKit's `alarm_tags_that_overlap_are_counted` failed because a tag that grew wider with its `+n` count wasn't placed again, so it could cover a neighbour until the next frame; tags whose count changes are now measured and placed again at once (twice at most), and the test's far object sits clear of the widened tag. Local Firefox's `models_are_simpler_far_away` and `big_worlds_stay_instanced_and_culled` failed because Firefox throws on capturing a pointer it isn't tracking (Playwright's synthetic ones), which three's OrbitControls does on each press; the tests' prelude lets that error pass, as a real pointer never raises it. All ten pass in Chrome, WebKit and Firefox on a Mac; in Chrome, `coloured_by_value_and_shapes_are_instances` failed once in eight full runs (right after a build, under load) and not alone in six.
- [ ] **Older browsers aren't tested.** The browser tests run in Chrome, and in Playwright's WebKit and Firefox (CI, and `tools/playwright/dump-dom.js` locally), which are recent builds (Safari 27.2, Firefox 159 on 2026-11-17). Safari releases and Firefox ESR without the Popover API, invoker commands, `closedby` or `<details name>` aren't covered. Anchor positioning has a fallback script (2026-11-20), tested by turning the browser's own off; `anchor()` and `anchor-size()` in a document's own CSS aren't handled by it.
  - Idea: run the tests in the installed Safari through `safaridriver`.

- [x] **Charts with `series`: `sort` and `limit` rank categories by their totals** (2026-11-20), keeping each one's series together; report-01's Top N works on split charts.
- [x] **Chart value labels are formatted** (2026-11-20): `format` (number, currency, percent), `currency` and `decimals`, for labels, tooltips and the value axis; at most two decimals by default.
- [x] **Lookup by a computed key in expressions** (2026-11-20): `names[dim]`, `rows[i + 1].id`; report-01's title uses it.

## UI kit: not built yet

- [x] **Installable apps and devices** (2026-11-22): a document's `app`, `irender app` (manifest, icons, service worker), and 37 components, each with a gallery section marked New, a place in a block (field-01, keep-01, visit-01, capture-01, devices-01) and browser tests. Not done yet:
  - [ ] **Nothing has been tried on real phones or devices.** Cameras, microphones, location, passkeys (a virtual one), shares, offline use and updates were driven in desktop browsers; sensors, push delivery, NFC, Bluetooth, serial and USB only against stand-ins.
  - [ ] **The service worker** keeps everything the first time; there's no progress for it. Libraries on jsDelivr are kept, map tiles and other sites' files aren't. An app is one folder: pages elsewhere on the site aren't in it.
  - [ ] **`ui-offline-queue`**: sends one at a time; nothing settles a clash with what changed on the server meanwhile, and what's queued isn't shown in lists as if it were saved. Not encrypted on the device. No background sync on iPhones or in Firefox.
  - [ ] **Nothing pulls data down** for offline: `submit` with GET and data tables read through the service worker's kept copies (records from `ui-data-source` aren't kept at all, 2026-11-27). The data sources' phases 2 and 3 are the plan.
  - [ ] **`ui-push-opt-in`**: the server sends the pushes; there's no example of one. The badge counts only what it's told.
  - [ ] **`ui-app-shell`**: one level of Back per screen; no sliding between screens, sheets or split view on tablets.
  - [ ] **`ui-app-lock`** covers the page; nothing behind it is encrypted. **`ui-passkey`**: no example server.
  - [ ] **`ui-camera`**: no picking of a lens, focus or exposure. **`ui-photo-annotate`**: no cropping or zooming while marking.
  - [ ] **`ui-document-scanner`**: edges come from brightness, so a white page on a white desk is set by hand; no text recognition.
  - [ ] **`ui-video-recorder`, `ui-screen-capture`** keep a `data:` address in state, which suits short clips; nothing streams to a server.
  - [ ] **`ui-field-map`** and the map kit's `ui-map` overlap: this is the light one, without a library and with drawing.
  - [ ] **`ui-route-tracker`, `ui-geo-checkin`**: only while the app is open, as the web has it; a device can be made to lie about where it is.
- [x] **Forty-two components** (2026-11-22), each generic, with a gallery section marked New, a place in a block and browser tests (`tests/browser/components.rs`). Not done yet, by component:
  - [ ] **Editable cells** (`edit` on data table columns; block data-table-08): no editing in nested, grouped or API-grouped tables; a fill handle (2026-11-27; repeats the picked cells, no series like 1, 2, 3 or dates by the day, and only up and down); one date at a time (the browser's own date field); undo is the table's, not the form's. A remote table's edits are the page's to save.
  - [ ] **`ui-import`** (import-01): no `.xls`; everything is read in the browser, so files past about 50,000 rows are slow; no mapping of values (a sheet's "M" to Male), only of columns; matches aren't remembered between files.
  - [ ] **`ui-repeater`** (invoice-01, profile-01): a row's fields are its own controls; a kit component that binds a state key (a combobox, a date picker) can't be one: `type: combobox` (2026-11-27; gallery) gives a typed choice from `options` with the browser's suggestions, which don't search inside words or show descriptions. Rows move without animation.
  - [ ] **`ui-form`** (invoice-01, profile-01, data-table-08): a `ui-repeater`'s rows are checked with the form (2026-11-27: submit, summary, unsaved changes; invoice-01), but a field inside a plain `list` item still isn't tracked. A `remote` rule (2026-11-23) asks the server with GET only, one key's value at a time (not two fields together).
  - [x] **Form columns and sections** (2026-11-23): `columns` on `ui-form`, `ui-form-section` (title, description, its own `columns`, heading `aside`), `span` on fields and sections; they nest and stack by container width (block supplier-01). Folding (2026-11-26): `collapsible` and `open`, a native `<details>`; a stopped submit opens sections with wrong fields, and going to a field opens the ones around it (gallery, supplier-01's Payment). A folded section says how many of its fields need fixing (2026-11-26). `bindOpen` keeps its open state in state (2026-11-27). Not done: `span` is only on `ui-field` and `ui-form-section` (others use `grid-column`).
  - [x] **`ui-stepper` furthest step** (2026-11-26): `bindFurthest` keeps the furthest step reached, so going back doesn't mean Next through each step again; steps before it show as done (gallery, intake-01). Not done: a step reached and left isn't told apart from one finished, and nothing checks a step's fields on the way past (a step without `valid` counts as done once passed).
  - [ ] **`ui-comments`, `ui-mention-input`** (record-01, review-01): plain text only, no Markdown or attachments; `people` are in the page, not searched from an API.
  - [ ] **`ui-phone-input`** (intake-01): groups and lengths of some 40 countries, general rules for the rest; numbers aren't checked against real ranges. **`ui-time-picker`**: no time zones.
  - [ ] **`ui-query-builder`** (rules-01): a field against another (`fieldCompare`) and the query as SQL (`bindSql`, parameterised) or a link (`bindUrl`) came on 2026-11-29 (gallery, rules-01). The SQL's multiselect operators are Postgres arrays, and "in the last n days" is two dates worked out in the browser; the link is flat (`field[op]=value`) only for rules all joined by and.
  - [ ] **`ui-tree-select`**: as `ui-tree`, every node is drawn. **`ui-transfer-list`**: options come all at once.
  - [ ] **`ui-pivot`** (pivot-01): computed in the browser over the rows given; no calculated fields, running totals or groups by range.
  - [ ] **`ui-scheduler`** (schedule-01): bookings repeat, but one day of a repeating one isn't moved on its own; no dependencies or time zones; every row is drawn; making one by dragging is for a mouse.
  - [x] **`ui-plan` layouts and floors** (2026-11-22): aircraft, bus, train, cinema, theatre, auditorium, stadium, hall (eight styles), parking and office from a few words; several picked with a total; floors with stairs and lifts (blocks seats-flight-01, seats-cinema-01, seats-hall-01, floors-01).
  - [ ] **`ui-plan`** (plan-01): items are moved, not sized or turned on the plan; nothing picks the best seats together; every seat is an SVG element, so some thousands on one floor are slow.
  - [ ] **`ui-pdf`** (review-01): needs pdf.js from a CDN (or `library`); no thumbnails, outline, form filling or search of its own; notes are kept beside the file, not written into it.
  - [ ] **`ui-qr-code`** takes a logo (a picture, an icon or letters); a logo from another site that doesn't share it is left out of a saved file.
  - [x] **`ui-code-scanner`** reads with the camera in Safari and Firefox too, with a decoder in WebAssembly (zxing-wasm, about 1 MB, loaded when it's first used), and from a picture (2026-11-22).
  - [ ] **`ui-code-editor`** (rules-01, ops-01): the highlighter's ten languages; no folding, find and replace, several carets or language server.
  - [ ] **`ui-diff`** (record-01, review-01, ops-01): two versions only; each change accepted or rejected with `review` (2026-11-29; gallery, review-01), its result the earlier version with the accepted changes: a change is a whole run of lines, not a word. **`ui-log`**: keeps the last 2,000 lines drawn.
  - [ ] **`ui-image-cropper`, `ui-signature`** (profile-01, invoice-01): straightening by a free angle isn't offered; a typed signature uses the device's own script font.
  - [ ] **`ui-trace`, `ui-tool-call`, `ui-questionnaire`, `ui-sources`, `ui-attachment`** (agent-01): a message has `sources` and `attachments`; the `[1]`s in a message's Markdown aren't links (only in `ui-sources`' own `text`).
- [ ] **Gaps against client-ui-next** (`~/Documents/GitHub/client-ui-next/components`, compared 2026-11-17). Generic components it uses that no kit has yet, most used first:
  - [x] **Filters** (2026-11-18): `ui-filters` + `ui-filter`, a side panel of list, number, currency, date and text fields with operators, shown as removable chips, writing the chart kit's `filter` format (block data-table-06). Not done yet:
    - [x] Finding a field (2026-11-20): past eight fields (or `searchFields`), a box over the panel's list narrows it by name or section (block data-table-06, gallery). Every field is still in the list; there's no "Add filter" step that keeps fields out until picked.
    - [x] Date presets (today, last 7 days, this month, this quarter, …) and saved views (`bindViews`) (2026-11-18).
    - [x] Dates pick on the kit's calendar, and amounts are grouped as typed with the currency's symbol (2026-11-18). No quarter type yet.
    - [x] List values from an API as they're searched for (`optionsUrl`, 2026-11-22). A chip for a value picked before and not looked up since shows its raw value.
  - [x] **Tree view** (2026-11-18): `ui-tree` selects, checks (with partial parents), searches, adds by name, deletes one or the checked, and moves nodes by dragging or Alt and the arrow keys, writing to `bindNodes` and firing events (block tree-01, gallery section). Not done yet:
    - [x] Renaming in place (`renamable`: F2, a double click), icons per node, and children loaded from `childrenUrl` when a node first opens (2026-11-20).
    - [ ] Very large trees draw every node (no windowing). Search doesn't look inside children that haven't loaded.
  - [x] **Calendar views** (2026-11-18): `ui-event-calendar`, events in month, week (hours, overlaps side by side) and list views, coloured by category, with `bindEvent`, `bindDay` and the keyboard (block calendar-01, gallery section). Not done yet:
    - [x] Year and day views (2026-11-19).
    - [x] Events move by dragging (to another day, or time in the week and day views, and resized by their foot) or with Alt and the arrow keys, written back with `bindEvents` (2026-11-19). Adding is still up to the page (`bindDay`), and the year and list views don't move events.
    - [x] Recurring events (`repeat`, 2026-11-22): days, weeks on weekdays, months, years, until, count and left-out days; moving one day makes it an event of its own (block calendar-01). Not done yet: no editing of the rule from the calendar, "the third Monday" rules, or time zones (times are local to the browser).
  - [x] **File upload and preview** (2026-11-18): `ui-file-upload`, a drop zone with type, size and count checks, uploads with progress, errors and retry, a grid or list, previews of images, PDFs, CSV and .xlsx (read in the browser) and text, and Download all as a zip, without libraries (block expense-01, gallery section). Not done yet:
    - [ ] No thumbnails for PDFs or sheets (client-ui-next draws the first page); they show a type badge.
    - [ ] Word and PowerPoint files aren't previewed, only downloaded; .xls (the old format) isn't read.
    - [ ] Uploads are one request per file: no chunked or resumable uploads, and no direct-to-storage (presigned) uploads.
    - [ ] Download all stores files without compressing them, and holds the zip in memory, so very large sets are slow.
  - [x] **Gantt chart** (2026-11-19): `ui-chart` `type: gantt`, a table of tasks beside their bars, groups that fold, progress, `after` arrows, milestones, `markers` and today, colours by `series` with a legend, zoom buttons, and a height that follows the rows (block gantt-01, charts gallery). Not done yet:
    - [x] The table is an HTML tree grid over the chart (2026-11-20): rows read out and moved through with the keys, groups opened with Right, Left and Enter, tasks picked with Enter.
    - [x] Editing (2026-11-20): bars dragged to move or stretch them (a hold first by touch), Alt and the arrows on a row, written back to `bindData`; tasks that start before what they wait for ends are marked. Links drawn from a bar's dot and removed by clicking their arrow, and `cascade` moving the tasks that wait on a moved one (2026-11-20). `snap` (2026-11-20) moves tasks with times of day by minutes or hours, on that grid, in drags, keys and `cascade`. `cascade: "both"` (2026-11-21) pulls the waiting tasks back too (block gantt-01). Not done yet: `snap` with `workdays` doesn't know working hours.
    - [x] Scrolling inside the chart (2026-11-20): `visibleRows` keeps it that many rows tall, the rest scrolled to by the wheel, a bar, a drag or the table's keys (block gantt-01).
    - [x] Working days (2026-11-20): `workdays` and `holidays` count, move and cascade whole-day tasks on working days, days off shaded (block gantt-01). Tasks with times of day still use calendar days.
    - [x] The critical path (2026-11-20): `critical` rings the tasks with no slack (in working days with `workdays`) and draws their links red (block gantt-01).
    - [x] Baselines (2026-11-20): `baselineStart` and `baselineEnd` drawn as a thin bar under each task (a hollow diamond for a milestone), groups' from their tasks', tooltips saying how many days late or early; `baselines: false` hides them (block gantt-01's switch). Not done yet: no way to save the current dates as the baseline, other than the host copying them.
    - [x] The axis's dates are in the document's `lang` (2026-11-20).
  - [x] **Kanban board** (2026-11-18): `ui-kanban`, cards dragged between columns or moved with Alt and the arrow keys, written to `bindCards` with `move` events; limits, folding, Add card, and `bindCard` following its card (block kanban-01, gallery section). Not done yet:
    - [x] Swimlanes by a card field, card `fields` (text, badge, progress, date) and moving columns by a grip (2026-11-20; block kanban-01). Cards from a template of the page's own components: the board's children, with the card as `card` (2026-11-21; block kanban-02). Lanes fold and move by a grip, written to `bindLanes` (2026-11-21; block kanban-01). Columns move by touch too, holding the grip (2026-11-21). `swimlanes` and `fields` may use state (2026-11-19; block crm-01's Board settings).
    - [x] `filter` on the board (2026-11-20), as the data table and charts have; crm-01's board has a filter panel. Applied in the browser only: the server's first drawing shows every card.
  - [x] **Dashboard grid** (2026-11-19): `ui-dashboard-grid` + `ui-tile`, tiles dragged by their header and resized from their corner (pointer events, so touch too) or with the arrow keys, pushing the others aside and floating up, locked tiles, the layout in `bind`, and one column on a narrow grid (block dashboard-02, gallery section). Not done yet:
    - [x] A layout per width (2026-11-20): `breakpoints` with each size's columns, tiles' `layouts` per size, the rest worked out from the widest, all in the page's CSS, and `bind` keeping each size's (block dashboard-02's tablet layout).
    - [x] Tiles added and removed from state (2026-11-20): a tile's `when` takes it off the dashboard (the rest close up) and back (its place, or the first free one), and dashboard-02 has a Widgets menu. Tiles are still written in the document, a catalogue to pick from; a tile can't be made from a list item.
    - [ ] Only vertical packing: no horizontal compaction, and tiles can't be dropped in from outside the grid (a palette of widgets).
  - [x] **Activity timeline** (2026-11-18): `ui-timeline` + `ui-timeline-item`, with avatars or initials, field changes (old → new), comments and event dots, from children or state (block ticket-01). No grouping by day, and no "show more" for long histories.
  - [x] **Notification centre** (2026-11-18): `ui-notifications`, a bell with an unread count and a panel (or `inline`) with Unread and All, read toggles, archive and Mark all as read, from state (dashboard-01 header, gallery section). Not done yet:
    - [ ] No notification preferences (which kinds, by email or in the app), and no grouping by day.
    - [ ] Nothing arrives by itself: new notifications need the page's state to change (a `ui-live-data` feed, or the host).
  - [x] **Progress ring** (2026-11-18): `ui-progress-ring`, following state without a script.
  - [x] **Status badge, rating and password checklist** (2026-11-18): `ui-status-badge` (built-in tones for common statuses, `tones` and `labels`), `ui-rating` (pick, or show a fractional average) and `ui-password-checklist` (rules, strength, `bindValid`; in the signup block). Half stars can be picked (`half`, 2026-11-22).
  - [x] **Number and currency input** (2026-11-18): `ui-number-input`, grouped as typed in the locale's way, with a currency symbol, prefix or suffix, steps and clamping.
  - [x] **Sidebar icon rail** (2026-11-19): `ui-rail-layout` + `ui-rail-panel`, a rail of icons each opening its own panel, with the section and the panel's open state in state (block workspace-01, gallery section). The panel resizes by dragging its edge (2026-11-19). The rail can't be on the right.
  - [x] **Resizable panels** (2026-11-19): `ui-resizable` + `ui-resizable-panel`, dragging and the keys, limits, collapsible panels, sizes in state (block mail-01, gallery section). Sizes are percentages only (no fixed px panel).
  - [x] **Code block and JSON editor** (2026-11-19): `ui-code-block` (highlighting for ten languages, line numbers, marked lines, copy) and `ui-json-editor` (checks as you type with line, column and reason, Format, value to state while valid). In docs-01 and the settings block's Webhooks. Not done: a general code editor (other languages, a schema to check JSON against, folding), and the highlighter is a tokenizer, so nested languages (JS in HTML) stay plain.
  - [x] **Drawer** (2026-11-19): `ui-drawer`, a bottom sheet dragged down (or flicked) to close, in the dialog family (block mail-01's Compose, gallery). No snap points (half-open, full), and like the other dialogs it opens from a button, not from state.
  - [x] **Interface text in other languages** (2026-11-19): a document's `lang` and `labels` give the kits' fixed words in its language, keyed by the English, with plural forms by `Intl.PluralRules`; every key is listed in skills/irender/references/labels.md (generated by tools/labels.py); block i18n-01 is a page in Spanish. Not done: no ready-made translations (each document gives its own), the server knows the plural rules and number formats of the common languages only (others as English), and the chart kit's numbers follow the browser, not `lang` (2026-11-19: `locale` defaults to `lang`).
  - [x] **Right to left** (2026-11-19): `lang` of a right-to-left language sets `dir="rtl"`; the UI and Markdown kits use logical CSS (inline start and end), mirror their directional icons and popovers, and swap Left and Right in keys and drags. Not done: a block's own styles (`margin-left` in a document) aren't converted; English text inside right-to-left pages keeps the browser's bidi ordering (".Acme Inc"); and the chart kit doesn't mirror (axes stay left to right).
- [x] **Keyboard-driven components** (2026-11-18): data table, date picker, context menu, menubar, navigation menu, one-time-code input.
- [x] The UI kit gallery no longer scrolls sideways on a phone (2026-11-22): `ui-theme-option` wraps its choices, and `ui-hover-card` stays on screen and out of the page's width. Not done yet: in a right-to-left part of a left-to-right page Firefox puts a hover card at the screen's left edge (a right-to-left page is fine); without anchor positioning (Safari before 26, Firefox ESR) a hover card can still run off the side; `ui-tooltip` isn't kept on screen.
- [x] `ui-tabs` by keyboard (2026-11-22): the list is one stop for Tab, and the arrow keys, Home and End move and select; a list with more tabs than fit keeps the selected one in view and fades on the sides with more. Vertical tabs and `activation: manual` (2026-11-22).
- [x] No example page scrolls sideways at 390 or 1280px wide (all 57 checked in Chrome, 2026-11-22): a `ui-tabs` list scrolls within itself, `ui-toggle-group` wraps when it's wider than what it's in, and `ui-input-otp`'s boxes narrow to fit. Not done yet: at 320px the UI kit gallery is 6px too wide (the Card section's member list); narrower widths of the other pages aren't checked.
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
  - [x] CSV export of the rows that pass, in the columns shown (2026-11-19). Excel too (2026-11-22), with a name ending in `.xlsx`. A remote table exports only the rows it has loaded.
- [ ] **Date picker gaps** (`ui-date-picker` and `ui-calendar` shipped 2026-11-18: days with a time, months, quarters and years on a financial year, single, range and multiple, limits, disabled days, presets, month and year menus, `bindRange`; the data table's date filter uses the calendar):
  - [x] Typing a date (`input: true`, 2026-11-18): the locale's numeric order, month names, two-digit years, ranges, times, quarters and years, with errors for `ui-field`.
  - [ ] Typing: month names are matched in the locale and in English only; error messages are English.
  - [x] A range with times (2026-11-20): `time` with `mode: range` gives a start and an end time, typed too (gallery's Room booking).
  - [ ] No time zone: values are local wall-clock times.
  - [x] Interface text follows the document's `labels`; dates and month names follow `locale` (2026-11-19).
  - [x] A week picker (`picker: week`, 2026-11-27; gallery): one ISO week, Monday to Sunday, picked from the days. Not done: weeks that start on another day, ranges of weeks, and typing a week.
  - [ ] No "multiple months" mode beyond three months side by side.
  - [x] A period to compare with (`compare`, `bindCompare`, 2026-11-23): the previous period, the same one last year, or a custom one picked on the calendar, drawn as a band in the second chart colour and as "vs …" in the button (gallery's Compared period, block data-table-02). Not done yet: not with `time`; nothing typed for it with `input`; a chart can't draw both periods as two series of one chart (the block shows the comparison as a chart of its own), and charts take the band's colour only from `color.chart-2`.
  - [x] Quarter presets (2026-11-20): `quarters` (the last four, named by their financial year), `lastQuarter`, `lastYear` and `yearToDate` in the calendar, the filters panel (with its own `fiscalStart`) and the data table's date filter (with the column's `presets` and `fiscalStart`).
- [ ] **Menus** (`ui-menubar`, `ui-navigation-menu`, submenus, checkbox and radio items shipped 2026-11-18):
  - [ ] The navigation menu's panels don't share one animated viewport that morphs between items, as Radix's does; each panel opens on its own.
  - [x] Submenus keep a "safe triangle" (2026-11-20): heading diagonally for an open submenu across other items doesn't close it; stopping on another item does, after 400ms.
  - [x] Checkbox and radio items can keep the menu open (`keepOpen`, 2026-11-20).
- [x] **`ui-context-menu` gaps** (shipped 2026-11-18):
  - [x] Submenus, checkbox and radio items (2026-11-18), in every menu.
  - [x] A long press (500ms) by touch opens it, for iOS Safari, which sends no `contextmenu` (2026-11-20).
  - [x] Right-clicking a data table row selects it (2026-11-20), or keeps the selection on a selected row, as file lists do; block data-table-01's row menu acts on the selection.
- [x] **`ui-command` gaps:**
  - [x] Ranked, fuzzy matching (2026-11-20): best match first in each group and of the groups, letters in order, keywords, matched letters marked; `rank: false` for document order.
  - [x] Pages (`ui-command-page`, an item's `page`, Backspace back) and `bindSearch` (2026-11-20). A loading state for items fetched as you type: `loading` and `loadingText` (2026-11-20; gallery).
- [x] **`ui-sidebar` gaps:**
  - [x] `remember` keeps it open or closed across reloads, in localStorage (2026-11-20; blocks sidebar-07, dashboard-01). The server can't read it (shadcn's cookie can), so a page drawn closed but remembered open opens as its script runs.
  - [x] Group and item actions (2026-11-20): `ui-sidebar-action`, a button or a ⋯ menu, shown always or on hover (block sidebar-07, gallery).
  - [x] The phone sheet keeps Tab inside the panel and swipes closed (2026-11-20). Tried with touch pointer events in Chrome, WebKit and Firefox, not on a real phone.
- [ ] **`ui-illustration` gaps** (15 drawings and the error and empty-state blocks shipped 2026-11-18):
  - [x] Seven more drawings (2026-11-20): `no-access`, `upload-failed`, `too-many-requests`, `no-people`, `deleted`, `empty-calendar`, `welcome` (gallery, block empty-states).
  - [x] `animate` (2026-11-20): parts of twelve drawings float, blink, turn, twinkle, drift or wobble, and stop for reduced motion (gallery's Animate switch, block error-404).
  - [ ] With the kit's default near-black primary, the drawings are black and amber; they look their best with a palette or custom colours picked.
- [ ] **More blocks:** shadcn has many more variants of each (sidebar-01 to -16, login-01 to -05, signup-01 to -05). We have dashboard-01 and -02, sidebar-03, -07 and -08, login-03 to -05, and two sign-ups (signup-02 on 2026-11-27); and app pages beyond shadcn's: a report builder (report-01), an incident room (incident-01) and a sales pipeline (crm-01).
- [x] **shadcn/ui components still missing:** item, attachment, questionnaire and trace are in (2026-11-22).
- [ ] **`ui-markdown-editor` rich text gaps** (rich text became the default on 2026-11-18):
  - [x] Table, Image and Rule on the toolbar (2026-11-21): a table to fill in with Tab between its cells, an image from a bar for its address and description, a rule; in the Markdown tab, their Markdown. Not done yet: no columns added or removed from the toolbar (a row is).
  - [x] Undo takes back every change (2026-11-22): the rich editor keeps its own history (content and caret, typing merged into one step), for ⌘Z, ⌘⇧Z, Ctrl+Y and the Edit menu, so quoting, leaving a quote or code block, ticking a task and a table's new row are undone too, alike in every browser. Not done yet: the history is kept for the page's life only, and holds the last 100 steps.
  - [x] `maxLength` holds in rich text (2026-11-22): an edit that takes the Markdown past it is taken back, caret and all, and a paste is cut to fit. Not done yet: the limit counts Markdown, so formatting costs characters (bold four), and a formatted paste past it comes in as plain text.
  - [x] Headings and pasted links (2026-11-22): the heading button goes `##` → `###` → text, and ⌘⌥1–6 / ⌘⌥0 set any level, in rich text and the Markdown tab; an address pasted over text links it, a pasted link is written `<https://…>`, and a pasted line keeps the spaces at its ends. Not done yet: no heading menu (levels 1 and 4–6 are by keyboard or typing `#`); in the Markdown tab a pasted address isn't turned into a link over the selection.
- [ ] **Chat gaps** (`ui-conversation`, `ui-message`, `ui-chat-input`):
  - [x] Messages show their content as Markdown when irender-markdown-kit is registered, rendered on the server and again as a reply streams (2026-11-18).
  - [x] Code blocks are highlighted, with a copy button, in messages and everywhere else `ui-markdown` shows Markdown, and in the editor's preview (2026-11-19). Not in the rich text editor itself, where the code stays plain while you edit it; and a streamed reply's fence is highlighted again on each chunk.
  - [x] Sources under a message and `[1]`s that show them (`ui-sources`, 2026-11-22). Not done yet: citations inside a message's Markdown, and footnotes.
  - [x] Unfinished Markdown in a streamed reply (2026-11-22): the message is `streaming` while it arrives, and formatting left open at its end (`**`, code spans, links, a table's first row) shows as it will once closed. Not done yet: a mark right after a letter or digit waits until it closes (so `2*3` isn't emphasis), and the server's render of a `streaming` message isn't finished.
  - [x] A streamed reply draws only its last, unfinished block for each chunk (2026-11-27), and the whole once it's done. A list or table still arriving is drawn again whole with each chunk.
  - [ ] A message's `html` (and `ui`) is kept in state and in an attribute, so long conversations with UI are heavy, and it goes back to the endpoint with `messages`.
- [ ] **irender-wasm:**
  - [ ] Not built or run as WebAssembly yet: this machine's Rust (Homebrew) has no `wasm32-unknown-unknown` target. Its render function and exports are tested natively, and the page's loader against a hand-made module with the same exports; build it once rustup is installed, measure its size, and run the chat block with it.
  - [ ] No icons (the packs are 9.3 MB). Idea: a small default set, or packs fetched on demand.
  - [ ] Renders on the main thread. Idea: a worker for large UIs.
  - [ ] Each message's UI carries its own copy of the styles it uses.
  - [ ] The conversation list in a sidebar can't star, rename or delete a thread on its own; the chat block hands those to the host with `call`.
  - [x] Message actions (2026-11-22): copy, try again, edit and resend, up and down (block chat).
  - [x] Attachments in the input (`bindFiles`), tool calls and traces (2026-11-22). Images aren't pasted into the input yet.
- [x] **`ui-carousel`** (2026-11-20): dots, `loop`, `autoplay` with a pause button, and stepping to each slide's own start. Not done yet: no fade transition (it always scrolls).
  - [x] `body`, `headers` and Try again after a failure (2026-11-22).
  - [ ] Replies in a thread don't nest further, and a reply can't be sent into a thread from the input.
  - [ ] A streamed reply rebuilds its message on each chunk, so screen readers may announce it more than once.
- [x] **The built-in `select` takes `multiple`** (2026-11-27): a list box, the key a list. `ui-combobox` is still the friendlier multi-select.
- [ ] **`ui-combobox`:**
  - [ ] No custom option rendering from the page's own components; options take a `description`, an `image` and a `badge` (2026-11-27; gallery's People).
  - [ ] No chip animations.
  - [ ] Values are plain values, not option objects. A pre-selected value that only `searchUrl` would return shows its raw value until it's searched for. `labels` and `lookup` give its name (2026-11-22).
- [ ] **Theme options:**
  - [ ] Picking and applying them needs irender-ui-kit (its picker and early script), like the mode. Without the kit a page shows the default choices.
  - [x] `ui-theme-option`'s `bind` mirrors the choice into state, and picks what's set there (2026-11-22).
  - [x] Density and text size reach everything (2026-11-21): `$space.*` and `$font.size.*` follow `space.scale` and `font.scale`, and so do the built-in `button`, `input` and numeric `stack` gaps, charts (text, tooltip padding), flow diagrams, the switch and the kit's remaining paddings. A choice also applies to part of a page through `data-theme-<option>` on a node (theme block). Not done yet: the kit's icon sizes, 1px borders and a few hairline offsets stay fixed; charts' symbol and legend marker sizes don't scale; `ui-theme-option` only picks the page's choice, not a part's.
  - [x] Separate scales (2026-11-21): Padding (`space.inset`), Gaps (`space.gap`), Control size (`size.control`) and Icon size (`size.icon`) options, on top of density, in the kit, built-ins, charts, flow and markdown. Not done yet: an explicit `ui-icon` `size` stays as given; chart symbol and legend marker sizes don't follow Icon size.
  - [x] Typography and shape (2026-11-21): Heading font, Weight, Line height and Letter spacing; Borders, Shadows (`elevation`), Control radius and Card radius, in the kit, the built-in `button` and `input`, chart tooltips, flow nodes and Markdown; the kit sets the page's `body` font. Not done yet: line height, letter spacing and heading font have no default value, so picking the default on part of a page doesn't undo a choice above it; bespoke shadows (focus rings, drawers' edges) and 1.5–3px accent borders stay as they are.
  - [ ] Finer controls: card style (flat, outlined, elevated) and sidebar/header tints; motion speed.
  - [ ] A swatch shows a choice's light value, in dark mode too.
  - [ ] Custom gradients are always 135° from the colour to its end; a theme's `gradient.*` token can be any CSS background. The built-in `button` (without the kit) ignores gradients.
  - [ ] Custom backgrounds don't change the text colour on them, so a dark card in light mode needs a custom text colour that doesn't exist yet. Idea: derive `color.text` per background like `on-primary`.
- [x] **Intermittent core browser test** (2026-11-22): `conditionals_lists_and_mount_actions_in_chrome` waited a fixed 200ms for a fetched list (13 failures in 336 under heavy load, then 0 in 336 once it waits for the rows). Every kit's browser harness also loads a page again when Chrome gives it back with no result.
- [ ] **Pages rendered without the kit ignore `ui-theme-toggle`'s choice.** Every kit page applies it now, and since 2026-11-22 every example page uses the kit; a page rendered without `--ui-kit` still follows the system.

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

- [x] **Reconnecting an edge** (2026-11-29): the picked edge's ends drag to another port (`reconnectable`, `reconnect`); not from the keyboard.
- [x] **Edges drawn on the server** (2026-11-29): straight lines between the nodes' middles, by their saved sizes (or 150 by 40), the nodes placed in their containers by `nodeOrigin`; not with `layout: layered`, and a node's real size, when it isn't saved, is only the browser's.
- [ ] **Layered layout is approximate.** Long edges get no dummy nodes, so edges that skip ranks can cross nodes. Only top-level nodes are laid out; children keep their `position`. With `layout: layered`, an added node's position is replaced by the layout.
- [ ] **Undo records only the flow's own edits.** Changes made to the state outside the flow aren't recorded, and an undo puts back the arrays as the flow last saw them.
- [x] **`extent: parent`** (2026-11-29; subflow-01): a node (or a template) keeps inside its container, dragged or by the keys; the container doesn't grow to it.
- [x] **Pasting remaps ids inside `data`** (2026-11-29): any string in a copy's `data` that's the id of a node pasted with it becomes the copy's id (`remapPasted: false` to leave it); an id inside a longer string isn't.
- [x] **Tried on the builder's 8 saved workflows** (2026-11-17; `server/data/workspaces/**/*.workflow`, up to 30 nodes with 5 loops):
  - Every node and edge draws, and every handle resolves to its port (`true`/`false`, the loop's `start`/`done`).
  - Loop bodies sit inside their loops.
  - Drags change only `position` and keep the editor's leftover keys (`measured`, `selected`, `dragging`) byte for byte.
  - Undo restores the saved JSON exactly.
  - Fixed on the way: the dot background packed too tight below about 0.5 zoom, and new and pasted ids didn't follow the builder's snake_case.
- [x] **Node shapes** (2026-11-17): `shape` on `ui-flow-node` covers the builder's diamond `if` and hexagon `switch`, and flowchart shapes generally.
  - Ports and floating edges meet the outline.
  - Gap: a port on a positioned row inside a shaped node (the switch's per-case rows) sits on the row's edge, not on the shape's slanted outline.
- [x] **Tried on the builder's ER diagram** (2026-11-17), as `examples/flow/er-01.yaml` with the `package_er` converter. Run on the dwigital packages (manufacturing_core, siemens, checklist, launchpad, sample_ecommerce): every table and relationship draws, with no overlaps.
  - Added for it: `filter` with neighbours as context, `flow.fullscreen`, loops for floating self-edges, and packing unconnected groups into rows (the layout used to stack them in one tall column).
- [x] **Tried on client-ui-next's process canvas** (2026-11-17), as `examples/flow/process-01.yaml`, loading a made-up canvas in the shape the app saves. There is no saved canvas in the repos; they live behind signed URLs.
  - These work: `nodeOrigin [0.5, 0]`, step colours, status icons, the palette, dropping a connection on empty canvas, the side form, cut, copy and paste, delete with a dialog, and undo.
  - Added for it: `bindNode`, `flow.update` and `nodeStyles: false`.
- [x] **One Flow page** (2026-11-22): `examples/flow/index.yaml` previews each flow as the blocks page does, with the ER diagram laid out four ways and the process canvas three. Previews start on click, since a flow takes the wheel.
- [ ] **Process canvas: still the host's job.**
  - Keeping a milestone's `dependencies` in step with its edges, and the delete impact check, need code: listen for `connect` / `delete` and `call` the host.
  - New nodes get ids like `custom_1`, where the app uses UUIDs. Idea: an `idPattern` or a host hook for new ids.
- [ ] **Check against a real exported canvas:** export one from the process canvas sidebar and load it into `examples/flow/process-01.yaml`.
- [x] **The yard view** is `ui-plan` in the UI kit (2026-11-22).

## Map kit next phases

- [x] **Maps** (2026-11-22): `irender-map-kit`'s `ui-map`, on OpenStreetMap through Leaflet or on Google Maps: markers, popups of any components, panels, alarms and animated markers, overlays (pictures, tiles, heat), clusters, lines, areas, circles, GeoJSON, the shared `filter`, picking a place, dragging markers, My location, and the selection, view and markers in view as state (the Maps page, block map-01).
- [ ] **Google Maps with a real key:** checked against Google's API without one (its watermarked mode) and against a stand-in; a key, a `mapId` and satellite types haven't been tried. Dashed areas and circles draw solid there.
- [ ] Not built: searching for an address and routes (both need a service), turned pictures, markers of any components, vector tiles (MapLibre). Shapes drawn by hand (`draw`, `bindShapes`) and markers on one spot fanned out (`spread`) came on 2026-11-29 (maps gallery): drawn shapes aren't reshaped (only taken away), not from the keyboard, and a fan is round its spot, whatever's near.
- [ ] OpenStreetMap's own tiles are for light use; a busy site needs its own tile server or a provider (`tiles`). Opened as a file (no address to send), a page may be refused tiles.
- [ ] The other kits' browser renderer (irender-wasm) doesn't include maps.

## Twin kit next phases

- [x] **Digital twins** (2026-11-23): `irender-twin-kit`'s `ui-twin`, drawn with three.js 0.183.2: models of most formats (glTF with Draco, meshopt and KTX2, OBJ, FBX, STL, PLY, DAE, 3MF, 3DS, WRL, USD) put right by `units`, `up`, `fit`; objects, shapes, paths with flow, zones, walls, ground, light presets and views, all configuration that may use state, or one `world` / `src`; readouts with limits per kind (`when`), alarms, tones, effects, `colorBy`; a `live` feed (polled or streamed) and `bindLive`; paths driven by speed or progress, `attach`, `follow`; popups of any components and panels; editing with a palette, snap, keys and write-back; the objects as a list for the keyboard; a list of objects and values without WebGL (the Twins pages, blocks twin-01 to twin-04, `tools/simready/`); twin-04 (2026-11-23) is a dairy's bottling line with its food safety checks. Browser tests in Chrome (SwiftShader), WebKit and Firefox.
- [x] **One 3D view a page** (2026-11-23): the Twins page had eight, and Chrome ran out of memory with them and the blocks' previews. Each example is now a page of its own, the Twins index and the Blocks page show pictures of them (`examples/twin/previews.cjs` takes them), and a palette's new object is selected as soon as it's drawn. Not done: a twin doesn't let go of its WebGL context when it's far out of view, so a page of several still holds them all.
- [x] **Smoother frames** (2026-11-23, from reports/Optimizing three js digital twins.md): shadows redrawn only when a caster changes (none while the camera turns), objects of one model drawn as instances (copies only while they need a look of their own), and each model's shaders and textures prepared before it's shown. twin-01 went from about 30 to a steady 60 fps while orbiting (no frame over 20 ms) and from 350–580 ms to 120–190 ms blocked while loading, on an M4 Pro in Chrome. Lower resolution while the camera moves (`motionScale`, 2026-11-23): no hitch when it switches, but these scenes are bound by triangles, not pixels: drawn in software, twin-04 took about 600–800 ms a frame at full and at 65% resolution alike. Next, and what weak GPUs need most: fewer triangles (simplified models or detail levels; twin-01 draws 1.2 million a frame with its shadow pass, twin-04 0.94 million). Not done either: skipping tag work on still frames, and a frame-time check in CI.
- [x] **Big worlds** (2026-11-26): tried with 300, 1,000 and 3,000 objects (a tag and a readout on each). Tags no longer blur what's behind them (thousands of blurred layers stopped the browser drawing: 3,000 objects never finished loading), are measured before they're moved and moved only when something has, and overlaps are found by screen cells; objects in alarm and the rings under warnings are instances; only instances in view are drawn; number formats and the reduced-motion query are made once; `data:model/…` models open (they never had: the type was read from an extension a `data:` address doesn't have). 1,000 objects went from 17–27 fps to 43–53, and 3,000 load in about a second (README, Smoothness).
- [x] **Simpler models far away** (`lod`, a model's `far`; 2026-11-26; twin/big-world): a simpler model of your own or one made in the browser with meshoptimizer's simplifier (1.3.0, from jsDelivr when needed), past a distance from the camera, for the objects drawn together. 1,000 detailed objects went from 39 to 6–8 million triangles and from 45–53 to 61 fps; 3,000 from 19 to 47–51 still and 14–16 to 43 turning. Not done:
  - [ ] Two levels only (full and far), switched at once (no fade between them), and the same distance for every part of a model.
  - [ ] Simplified on the page's own thread, a part at a time: a model of one huge part (a million triangles) holds it while it's made. Not kept between visits: each page load makes them again. Idea: in a worker, kept in the service worker's cache.
  - [ ] A simplified part shares its model's vertices (only its triangles are fewer), so it saves drawing, not memory. Parts with several materials aren't simplified. Far, a model can look a little lighter (inner frames left out).
  - [ ] Objects drawn as copies of their own (glowing or blinking by `effect`, see-through, animated, skinned, shapes in alarm) are always in full.
- [x] **Colours by value and shapes as instances** (2026-11-26): a model coloured by `colorBy` is an instance in a batch whose materials glow in each instance's colour (a per-instance attribute read by a patched shader); shapes are batched by kind and size, each in its own colour, sensors glowing. 1,000 models coloured by value and 1,000 sensors went from 4,003 draw calls and 36 million triangles (copies had no LOD) to 11 and 7 million: 32 to 57 fps turning, 128 to 47 MB. The glowing batches' shaders compile in the background (the longest pause while loading 340 → 205 ms).
- [ ] **Still copies of their own:** glowing or blinking by `effect`, see-through, animated and skinned objects, and shapes in alarm (their pulse is per object). Idea: per-instance opacity and a pulse in the shader.
- [ ] **A glowing batch's shader is patched** (`onBeforeCompile`) where three.js adds the emissive: a later three.js that renames `emissivemap_fragment` would stop the glow (with a warning in the console). The browser tests check the colours, not the pixels.
- [x] **Alarm tags don't pile up** (2026-11-26): an alarm's tag still covers any other, but not another alarm's; one hidden behind it is counted on it (`+2`). Not done: the count isn't a button (it doesn't list or zoom to the ones it stands for; the alarm list and `bindAlarms` do), and thousands of tags are still thousands of elements (hidden ones aren't moved, so it costs little: 3,000 turn at 43 fps with tags, 48 without).
- [x] **The browser tests waited too little in Chrome's virtual time** (2026-11-26): it races ahead while SwiftShader compiles shaders, so a test sometimes went on before its models were drawn (about one run in three of big_worlds). `ready` now allows 300 s of it, and the budget is 600 s. The other kits' browser tests use the same kind of wait; none has been seen to fail this way.
- [ ] **Nothing tried on a phone or tablet.** Drawn on a Mac in Chrome, Firefox and WebKit.
- [x] **History on readouts** (`history: n`; 2026-11-26; twin/live-data, block twin-01): an object's last n values, a point each time one changes, drawn as a sparkline in its popup. Not done: the points have no times (a steady value is one point however long it holds), nothing seeds them from a server (they start when the page opens), they're not on tags or in a `ui-twin-popup` of the page's own (`object` has only the latest values), and no limit lines on the sparkline.
- [x] **Several objects at once** (`multiple`, `bindSelected`; 2026-11-26; twin/editing, block twin-03): ⌘/Ctrl/Shift and a click, Shift and a box dragged from the floor, ⌘A; moved, turned and removed together, written back in one change. Not done: no copy and paste, no aligning or spacing them out, no turning them about their middle (each turns about itself), and on a phone there's no way to add one to the picked (no Shift; a long press could).
- [x] **A twin opened in view started twice** (found 2026-11-26): once when the page opened and again when it came into view, before the first had its renderer, so it made two renderers and did its work twice, keeping the second.
- [x] **Measuring** (`measure`, `bindMeasure`; 2026-11-26; twin/editing, block twin-03): a ruler among the controls (or M): two clicks on objects, walls or the floor, or two objects from the tags or the list, and the distance between them, with across and up when it slopes; `measure(a, b)` on `__twin`. Not done: one measurement at a time (no chain of them, no areas or angles), points don't snap to corners or edges, and the line can't be dragged to change it.
- [x] **Walls and zones drawn by hand** (`bindWalls`, `bindZones`; 2026-11-28; block twin-03): corners clicked on the floor (snapped), finished with Enter, a double click or (a zone) its first corner; picked and deleted. A picked one's corners drag along the floor (`reshape`, 2026-11-28; block twin-03). Not done: corners can't be added or taken away afterwards, or moved from the keys, no height or thickness while drawing (the row's to edit), and walls meet end to end without mitred corners.
- [ ] Not built: sizing and stretching by hand, cutting a building away floor by floor (a `floors` list, as `ui-plan` has), labels in 3D that hide behind walls, outlines on hover, VR and AR, a top view without perspective.
- [ ] **USD straight from SimReady:** three.js's USD reader takes a single file; SimReady's assets refer to other layers and textures, so they're converted with `tools/simready/` (Python with usd-core) first. The converter hasn't been run in this repo's CI.
- [ ] Models, worlds and feeds from other sites need CORS. Feeds are anonymous: no headers or credentials (`live` has no `headers` yet).
- [ ] The other kits' browser renderer (irender-wasm) doesn't include twins.

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

- [x] **Intermittent `components::app_lock_asks_for_the_pin`** (2026-11-24): about 1 run in 10, six at a time. The lock read its saved record at start without waiting, and a late answer (a busy machine's IndexedDB) wrote over a PIN set meanwhile, so the next check threw inside a click and nothing showed. The first read now leaves alone what was saved since (0 failures in 90). The components harness's `until` also gave up silently after 40,000 turns, which had hidden a second bug: `ui-offline-queue`'s `add` waited for `serviceWorker.ready`, which never settles on a page without a service worker, so it never returned; the background sync is now asked for without waiting. `until` now waits up to 10 seconds (letting the page's clock move every 50 turns, so IndexedDB's answers get back) and says when it gives up.

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
