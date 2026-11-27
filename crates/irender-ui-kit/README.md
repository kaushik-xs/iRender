# irender-ui-kit

Ready-made components for [iRender](../../README.md) documents, styled after [shadcn/ui](https://ui.shadcn.com/). Every component is plain config, and nothing needs JavaScript beyond iRender's own runtime. Overlays use the browser's own `<dialog>`, invoker commands, Popover API and CSS anchor positioning.

```rust
let mut engine = irender_html::Engine::new();
irender_ui_kit::register(&mut engine.registry);

let mut doc = irender_core::parse(src, irender_core::Format::Yaml)?;
irender_ui_kit::apply_theme(&mut doc);   // optional: the kit's neutral palette, dark mode included
let ir = engine.resolve(&doc)?;
```

From the CLI, add `--ui-kit` to `validate`, `render` or `components`:

```sh
cargo run -p irender-cli -- render examples/ui-kit/gallery.yaml --ui-kit -o gallery.html
cargo run -p irender-cli -- components --ui-kit      # every component and its props
```

```yaml
- type: ui-card
  props: { title: Create project, description: Deploy your new project in one click. }
  children:
    - { type: ui-input, props: { bind: name, placeholder: Project name } }
    - type: ui-card-footer
      children:
        - { type: ui-button, props: { label: Cancel, variant: outline } }
        - { type: ui-button, props: { label: Deploy } }
```

## Components

**Right to left.** On a page whose `lang` is written right to left (`dir="rtl"`), every component mirrors: what was on the left is on the right, `left` and `right` in `side` and `pin` mean the start and end sides, icons that point along the line turn round, and Left and Right swap in the keyboard (the calendar's next day is to the left) and in dragging. Charts keep their geometry; one-time codes, shortcuts and counts like `2 / 3` read left to right; and a badge takes its direction from its own text.

| component | props | notes |
|---|---|---|
| `ui-button` | `label`, `icon`, `variant` (default, secondary, destructive, outline, ghost, link, brand), `size` (default, sm, lg, icon), `href`, `type`, `disabled` | With `href` it renders a link that looks like a button. Children follow the label. A `variant` that reads state follows it. |
| `ui-code-block` | `code` (required), `language`, `title`, `lineNumbers`, `highlight`, `copy` (default true), `wrap`, `maxHeight`, `label` | Code, highlighted for json, yaml, js (ts), rust, python, bash, html, css or sql, with line numbers, marked lines (`[2, "5-7"]`), a copy button that says when it's copied, and a title bar. `code` can read state, and an object shows as JSON; like any prop, a literal with `{{ }}` in it is a template, so code that shows braces comes from state or a list item. The server draws the lines plain, so it reads without the script. Token colours follow `--code-string`, `--code-keyword` and the like. Parts: `head`, `pre`, `copy`; event `copy` with `{ ok }`. |
| `ui-badge` | `text`, `icon`, `variant` (default, secondary, outline, brand, info, success, warning, destructive), `dot` | A pill. `info`, `success`, `warning` and `destructive` are a soft wash of their colour, like `ui-alert`. `dot` puts a dot in the badge's colour before the text, for a status. |
| `ui-card` | `title`, `description`, `icon`, `variant` (default, elevated, outline, muted), `href`, `tag` | `icon` sits in a tinted square beside the title. `href` makes the whole card a link that lifts when hovered; keep buttons and other links out of it. Parts: `header`, `icon`, `title`, `description`. |
| `ui-card-footer` | | A row of actions; put it last in a card. |
| `ui-alert` | `title`, `description`, `icon`, `variant` (default, info, success, warning, destructive), `dismissible` | `role="alert"`. A wash of the variant's colour, a bar along its leading edge and the variant's icon (`icon: none` hides it). Children (e.g. buttons) go under the description. `dismissible` adds a ✕ that hides it. Parts: `icon`, `title`, `description`, `dismiss`. |
| `ui-illustration` | `name` (not-found, server-error, forbidden, maintenance, offline, no-data, no-results, out-of-stock, empty-cart, empty-inbox, no-notifications, session-expired, payment-failed, coming-soon, all-done, no-access, upload-failed, too-many-requests, no-people, deleted, empty-calendar, welcome), `size` (default 240), `primary`, `secondary`, `label`, `animate` | An original, flat drawing for an error page or an empty state, 4 : 3. It's coloured by the theme: the primary colour, a secondary one (`color.brand-2`, else `color.chart-2`, else amber), light tints of both, and neutrals mixed from the text and background colours, so it follows a palette, the custom colours picked with `ui-theme-colors`, and dark mode. `primary` and `secondary` set its own colours (and may use state; null goes back to the theme's). Decorative unless it has a `label`. `animate` (true, or from state) brings it to life a little: a planet floats, a gear turns, a light blinks, sparkles twinkle; still for readers who prefer less motion. The blocks have whole error pages (404, 500, 403, 503, offline, an expired session) and a page of empty states. |
| `ui-empty` | `title`, `description`, `icon`, `illustration`, `illustrationSize` (default 200) | A placeholder for a page or list with nothing in it yet; children (usually buttons) follow the text. Parts: `icon`, `title`, `description`. |
| `ui-avatar` | `src`, `alt`, `fallback`, `size` | Initials when there's no `src`. |
| `ui-presence` | `people` (required), `max` (default 4), `size` (default 28), `me`, `announce`, `align`, `label`, `id` | Who's here now: stacked avatars with a status dot and a colour each, then `+3`; a click lists everyone, their status and what they're on. See below. |
| `ui-separator` | `orientation`, `decorative`, `label` | With `label`, a horizontal line with text in the middle, like "Or continue with". |
| `ui-skeleton` | `width`, `height`, `round` | Pulses gently; still for people who ask their system for reduced motion. |
| `ui-spinner` | `size` (default 16), `label` (default `Loading`) | A spinning ring in the text colour, `role="status"`. Slows down for people who ask for reduced motion. |
| `ui-kbd` | `text` | |
| `ui-progress` | `value` (0–100) | Updates live when `value` uses state, and is clamped to 0–100 in CSS. |
| `ui-aspect-ratio` | `ratio` (`1.5` or `"16 / 9"`) | Its children fill it. |
| `ui-table` | `caption` | Children are the table's rows and sections, so `list` with `tag: tbody` works. Part: `table`. |
| `ui-filters` + `ui-filter` | see [Filters](#filters-panel) | Filters for a list in a side panel, shown as chips, written to state for a table's or chart's `filter`. |
| `ui-file-upload` | see [File upload](#file-upload) | A drop zone, uploads with progress, the files as a grid or list, previews, and Download all as a zip. |
| `ui-number-input` | `bind` (required), `currency`, `prefix`, `suffix`, `locale`, `min`, `max`, `step`, `decimals`, `stepper`, `placeholder`, `label`, `name`, `disabled`, `invalid`, `id` | A number or an amount, grouped in the locale's way as it's typed (`1,25,000` in en-IN, `125.000` in de-DE), keeping the caret among the same digits. State gets a plain number, or null when empty. `currency` (an ISO code) puts its symbol first, tells screen readers ("in INR") and sets the decimals; Up and Down step it by `step` (Shift for ten), as do the − and + buttons with `stepper`; leaving the field clamps it to `min` and `max` and rounds it to `decimals`. Works in a `ui-field`, like the other controls. Part: `input`. |
| `ui-notifications` | `items` (required), `bindItems`, `bindItem`, `inline`, `tab`, `title`, `label`, `emptyText`, `caughtUp`, `allHref`, `closeOnOpen` | A bell with an unread count that opens a panel of notifications (`{ id, title, body, who, when, read, archived, tone }`, newest first): Unread and All tabs, a dot to mark one read or unread, an archive button, and Mark all as read. Changes go to `bindItems`; a click opens one into `bindItem`, marks it read and closes the panel, firing `open` (`read` and `archive` fire too). `who` (a name, or `{ name, initials, avatar }`) shows initials or a photo; without it, a dot in the item's `tone`. Up and Down move through the list, Escape closes it, and the bell's label says how many are unread. `inline` puts the panel in the page instead, for a notifications page. Parts: `bell`, `panel`. |
| `ui-status-badge` | `status` (required), `tones`, `labels`, `dot` (default true) | A status in its colour. See [Status and activity](#status-and-activity). |
| `ui-gauge` | `value` (required), `min`, `max`, `unit`, `text`, `bands`, `tone`, `target`, `needle`, `shape` (arc, half), `size`, `thickness`, `scale`, `caption`, `label` | A figure on a dial with coloured bands, a target and a needle; follows state without a script. See below. |
| `ui-countdown` | `to` (required), `from`, `warn`, `danger`, `overdue`, `endText`, `format` (units, clock), `variant` (inline, tiles), `paused`, `workdays`, `hours`, `holidays`, `bindLeft`, `bindStatus`, `label` | The time left to a deadline, turning to warning and danger as thresholds pass and counting up once overdue; fires `end` at zero. See below. |
| `ui-progress-ring` | `value` (0–100, required), `size` (px, default 48), `thickness`, `text`, `label`, `tone` | A circular progress with the value in the middle; follows state without a script. |
| `ui-json-editor` | `bind` (required), `bindValid`, `type` (`object`, `array`, `any`), `rows` (default 10), `indent` (default 2), `label`, `format` (default true) | A JSON text area: highlighted, with line numbers, checked as you type. An error waits for a pause in the typing (or leaving the field), then gives its line, column and a plain reason (`Trailing comma before '}'`, `Keys need double quotes`) and marks the line; a fix shows at once. The value goes to `bind` only while the text is valid (and of `type`), and a change to it from outside is shown formatted. Tab indents (Shift+Tab takes it back; Escape, then Tab, leaves), Enter keeps the indent, and Format tidies the text or jumps to the error. Works in a `ui-field`. Parts: `bar`, `textarea`, `error`; event `change` with `{ value }`. |
| `ui-rating` | `bind` or `value`, `half`, `max` (default 5), `size`, `label`, `clearable` (default true), `disabled`, `name` | Stars to pick a rating, or to show one (fractions fill part of a star). |
| `ui-password-checklist` | `value` (required), `rules`, `bindValid`, `meter` (default true), `label` | A password's rules, ticked as they're met, with a strength bar. |
| `ui-timeline` + `ui-timeline-item` | `items`, `label`, `group` (day), `relative`, `since`, `limit`; item: `who`, `action`, `when`, `initials`, `avatar`, `changes`, `comment`, `tone` | Who did what and when, with field changes (old → new) and comments. |
| `ui-event-calendar` | see [Event calendar](#event-calendar) | Events in a month grid, a week of hours or a list by day, coloured by category. |
| `ui-kanban` | see [Kanban board](#kanban-board) | Columns of cards to drag between, or move with Alt and the arrow keys. |
| `ui-whiteboard` | `bind` (required), `bindView`, `tools`, `colors`, `readOnly`, `height`, `label`, `id` | A board without edges: pen, sticky notes, shapes, arrows and text to select, move, resize and write in, with pan, zoom, undo and SVG export. See below. |
| `ui-mindmap` | `bind` (required), `bindSelected`, `layout` (balanced, right), `colors`, `readOnly`, `height`, `label`, `id` | A mind map: a central topic with coloured branches both sides; topics added, written, folded, moved and deleted from the keyboard or the mouse. See below. |
| `ui-tree` | see [Tree](#tree) | A hierarchy to browse, select, check, search, add to, delete from and rearrange. |
| `ui-file-browser` | `bind` (required), `bindFolder`, `bindSelection`, `bindOpen`, `view` (grid, list), `editable` (default true), `rootLabel`, `height`, `locale`, `label`, `id` | Folders and files to browse one folder at a time, as a grid or a list: search, sort, select, rename, new folders, delete with undo, and move by dragging or a menu. See below. |
| `ui-data-table` | see [Data table](#data-table) | Sorting, column filters, search, paging, row selection with bulk actions, expandable rows, a View menu and summaries, over an array of rows. |
| `ui-virtual-list` | `items` (required), `itemKey`, `height` (default 400), `itemHeight` (default 40), `gap`, `bindRange`, `endThreshold` (default 10), `emptyText`, `label` | A long list that keeps only the items in view in the page; children are each item, with `item` and `index` in scope. See below. |
| `ui-scroll-area` | `height`, `maxHeight`, `label` | Scrolls its children with a thin scrollbar in the theme's colours. Focusable, so the keyboard scrolls it; `label` makes it a named region. |
| `ui-heading` | `text`, `level` (1–4) | |
| `ui-text` | `text`, `variant` (p, lead, large, small, muted, code, blockquote), `tag` | Takes children, e.g. inline `code`. |
| `ui-label` | `text`, `for` | Takes children, so it can wrap a control. |
| `ui-field` | `label`, `for`, `description`, `error`, `orientation` (vertical, horizontal), `span` | A label, the control (its children), help text and an error. The label names the control, and the control's `aria-describedby` points at the help text and error, so screen readers read them with it. The control is the one whose id is `for`, or else the first control inside, which gets an id if it has none. The error is hidden while empty, so `"{{ … ? 'Too short' : '' }}"` shows it only when needed, and it is announced when it appears. Parts: `label`, `description`, `error`. |
| `ui-input-group` | `prefix`, `suffix` | Text or buttons inside one field's border: children are a `ui-input` (or `ui-textarea`, `ui-select`) and any `ui-button`s. The group shows the focus ring and the invalid border. Parts: `prefix`, `suffix`. |
| `ui-button-group` | `orientation` (horizontal, vertical), `label` | Buttons joined into one control (`role="group"`); outline buttons look best. |
| `ui-input`, `ui-textarea`, `ui-select` | same as `input`, `textarea`, `select` | Same binding behaviour as the built-ins, restyled. |
| `ui-checkbox`, `ui-switch` | `label`, `bind`, `checked`, `name`, `disabled` | The switch is a checkbox with `role="switch"`. Parts: `control`, `label`. |
| `ui-radio-group` | `options`, `bind`, `value`, `name`, `orientation`, `disabled` | Options are values or `{ value, label, disabled }`, and may come from state. |
| `ui-breadcrumb` | `items` (`{ label, href }`), `separator` | The last item is the current page. `items` may come from state. |
| `ui-pagination` | `page`, `pages`, `href`, `siblings` | `href` is evaluated per page with `p` in scope: `"/orders?page={{ p }}"`. `page`, `pages` and `siblings` may come from state. |
| `ui-slider` | `bind`, `min`, `max`, `step`, `name`, `disabled` | A range input; `bind` holds a number. |
| `ui-accordion` + `ui-accordion-item` | accordion: `multiple`, `id`; item: `title`, `open` | Native `<details>`. One item open at a time unless `multiple`. |
| `ui-collapsible` | `trigger`, `open` | Native `<details>`. |
| `ui-tabs` + `ui-tab-panel` | tabs: `bind` (required), `orientation` (horizontal, vertical), `activation` (automatic, manual), `id`; panel: `value`, `label` | `bind` holds the open panel's `value`. Tabs carry `role=tab`, `aria-selected`, `aria-controls`. The list is one stop for Tab; the arrow keys, Home and End move between tabs and select them. More tabs than fit scroll within their list, which keeps the selected one in view and fades on the sides with more. |
| `ui-toggle` | `bind`, `label`, `variant` (default, outline), `size`, `disabled` | `aria-pressed` follows `bind`. |
| `ui-toggle-group` | `bind`, `options`, `multiple`, `variant`, `size`, `disabled` | One option pressed at most; pressing it again sets `bind` to null. With `multiple`, `bind` holds a list and each press adds or removes its option. Wraps when it's wider than what it's in. |
| `ui-theme-toggle` | `labels` (`{ system, light, dark }`), `variant` (default `outline`), `size` (default `sm`) | System / Light / Dark. Light and Dark override `prefers-color-scheme` for the theme's dark mode and `@dark` styles; the choice is kept in `localStorage` (`irender-theme`) and applied before the page is shown, so a reload doesn't flash. Every page rendered with the kit applies it, with a toggle on it or not. Its behaviour is added only to pages that use it. |
| `ui-theme-option` | `option` (required), `swatch` (a token, or a list for several dots), `label`, `variant` (default `outline`), `size` (default `sm`) | Picks a choice of a theme option for every page; see [Theme options](#theme-options). |
| `ui-theme-panel` | `options` (default: every theme option), `mode` (default true) | A labelled `ui-theme-option` per option, palette first and chart colours last, and the mode. Put it in a `ui-popover` for a Theme button. |
| `ui-theme-colors` | `option` (default `palette`), `gradients` (default true), `backgrounds` (default true) | Colour inputs (picked, or typed as hex) for a custom primary and second brand colour, each with an optional gradient end, and the backgrounds of the page, cards, popovers, dialogs, sheets and the sidebar; changing one picks the `custom` choice. `ui-theme-panel` shows it under the palette. |
| `ui-dialog` | `trigger`, `triggerVariant`, `title`, `description`, `id` | A modal `<dialog>`. Escape, its ✕ or a click outside closes it. |
| `ui-alert-dialog` | same as `ui-dialog` | `role=alertdialog`; only its own buttons close it. |
| `ui-sheet` | same, plus `side` (right, left, top, bottom) | A dialog along one edge. |
| `ui-drawer` | `trigger`, `triggerVariant`, `title`, `description`, `width`, `id` | A panel that slides up from the bottom, with rounded top corners and a grab bar: the standard sheet on a phone. Drag the bar down (on a touch screen, anywhere while the content is scrolled to the top) and let go past a quarter of its height, or flick it, to close it; otherwise it springs back. A drag doesn't count as a click on what it started on. Escape and the ✕ close it too, and it opens like a dialog (its trigger, or `command: show-modal`). `width` caps it, centred, on a wide screen. Event `dismiss` when a drag closes it. |
| `ui-dialog-close` | `label`, `variant` | Closes the dialog it is in; its own `on.click` actions run too. |
| `ui-dialog-footer` | | A right-aligned row of buttons. |
| `ui-popover` | `trigger`, `triggerVariant`, `side` (bottom, top, right, left), `align` (center, start, end), `id` | Native popover, placed next to its trigger and flipped when there's no room. |
| `ui-dropdown-menu` | same as `ui-popover`, plus `triggerSize` | Children: `ui-menu-item` (`label`, `shortcut`, `variant`, `href`, `disabled`; choosing it runs its `on.click` and closes the menu), `ui-menu-label`, `ui-menu-separator`. Keyboard: opening focuses the first item; Up/Down, Home/End and typing move between items; Down or Up on the button opens it; Tab closes it. |
| `ui-menu-sub`, `ui-menu-checkbox`, `ui-menu-radio-group` | sub: `label`, `disabled`, `id`; checkbox: `label`, `bind`, `shortcut`, `disabled`, `keepOpen`; radio group: `bind`, `options` (values or `{ value, label, shortcut, disabled }`), `label`, `keepOpen` | Items for any menu (dropdown, context, menubar, and submenus themselves). A submenu opens beside its item on hover, Enter, Space or Right, and closes on Left or Escape, focus going back to its item; heading for an open submenu across other items doesn't close it (a "safe triangle" from where the pointer left to the submenu's edge), while stopping on another item does after a moment. Its children are menu items. A checkbox item (`role=menuitemcheckbox`) flips `bind` and shows a check while it's on; radio items (`role=menuitemradio`) write their value to `bind` and show a dot by the chosen one. Choosing either closes the menu, unless it has `keepOpen` (for flipping several settings in a row). Parts (sub): `trigger`, `menu`. |
| `ui-menubar` + `ui-menubar-menu` | menubar: `label` (default `Menu bar`); menu: `label` (required), `disabled`, `id` | A bar of menus, as at the top of an application, in the WAI-ARIA menubar pattern. The bar's buttons are one stop in the tab order: Left and Right move along them (wrapping), and an open menu follows; Down, Enter and Space open a menu on its first item, Up on its last; Home and End jump. Inside an open menu, Left and Right go to the neighbouring menus unless a submenu takes them. With a menu open, hovering another button opens that one. Closing a menu returns focus to its button. A menu's children are menu items. |
| `ui-navigation-menu` + `ui-navigation-item`, `ui-navigation-link` | menu: `label` (default `Main`), `align` (start, center, end); item: `label`, `width`, `id`; link: `label`, `href`, `description`, `active` | A site's navigation, in the disclosure pattern rather than as a menu. Links sit in the bar; an item's button opens a panel of anything, usually links, which show their `description` under the text. The pointer opens a panel after a moment over its button and closes it a moment after leaving both; a click toggles it. Down on the button opens the panel and moves into it, Escape closes it and returns to the button, and Left and Right move along the bar. One panel is open at a time. `active` marks the current page. |
| `ui-context-menu` | `bind`, `label` (default `Actions`), `disabled`, `id` | A menu opened with a right-click on its area, at the pointer and kept on screen, or by a long press on a touch screen (500ms; moving first scrolls, as iOS sends no right-click event for it), or from the keyboard with Shift+F10 or the menu key on something focused in it. `ui-menu-item`, `ui-menu-label` and `ui-menu-separator` children are the menu, with the dropdown menu's keys; any other children are the area. `bind` gets what was right-clicked: the nearest element's `data-context` (parsed as JSON when it is), or, around a `ui-data-table`, the row. A table's row also becomes the focused one and, in a `selectable` table, the selection when it isn't selected already (on a selected row the selection stays), so the items can act on `bindSelection`. Closing returns focus to where it was. `disabled` leaves the browser's own menu. It fires a bubbling `contextopen` event with `{ context }`. Part: `menu`. |
| `ui-split-button` | `label`, `variant`, `size`, `disabled`, `menuLabel` (default `More actions`), `side`, `align` (default `end`), `id` | A primary action joined to an arrow that opens a menu of secondary ones, e.g. Download PDF with Excel and CSV in the menu. Its `on.click` is the primary action; children are the menu's items, as in `ui-dropdown-menu`, keyboard included. Parts: `primary`, `trigger`. |
| `ui-tooltip` | `text`, `side`, `id` | Shown while its children are hovered or focused. CSS only. The first child (usually a button) gets `aria-describedby` pointing at the tip. |
| `ui-hover-card` | `trigger`, `href` | Children are the card, shown while the trigger is hovered or focused, below it (above when there's no room) and kept on screen. CSS only; a browser without anchor positioning always shows it below. |
| `ui-toaster` | `bind`, `duration` (default 4000 ms), `max` (default 3) | Toasts at the bottom right. Setting `bind` to `{ title, description, variant, duration }` adds one to the stack. `variant` is `info`, `success`, `warning` or `destructive`, each with its colour and icon, like `ui-alert`. A bar along the bottom counts down the time left, pausing while the pointer or focus is on the toasts. |
| `ui-resizable` + `ui-resizable-panel` | group: `direction` (`horizontal`, `vertical`), `bind`, `handle`, `height`, `label`; panel: `size`, `min` (default 10), `max` (default 100), `collapsible`, `label` | Panels side by side or stacked with a line between each two that resizes them: drag it (mouse or touch), or focus it and use the arrow keys (5%, Shift for 1%), Home and End. Sizes are percentages: each panel's `size`, the panels without one sharing what's left, or from state with `bind`, which gets them back after each move. A `collapsible` panel folds when dragged under half its `min`, or with Enter on its line, and Enter brings it back to its last size; a double click on a line goes back to the starting sizes. Each line is a focusable `separator` with the size of the panel before it. `handle: true` shows a grip. Groups nest. Part: `handle`; event `resize` with `{ sizes }`. |
| `ui-rail-layout` + `ui-rail-panel` | layout: `items` (required; `{ id, label, icon, badge, bottom, href }`), `bind` (required), `bindOpen`, `label`, `panelWidth`, `height`, `resizable`, `bindWidth`, `minPanelWidth` (default 180), `maxPanelWidth` (default 560); panel: `for` (required), `title` | An app shell: a rail of icons down the left (labels as tooltips, badges, items at the foot with `bottom`, links with `href`), each opening its own panel beside it, then the main content (the layout's other children). The section shown is `bind`, the panel's open state `bindOpen`: a click on the section shown, or the panel's fold button, folds it. The rail is a vertical tab list (Up, Down, Home, End; one tab stop) and each panel its tabpanel; the server draws the section and panel as state has them. On a phone the panel lies over the page, and a click on the page or Escape folds it. `icon` is an icon kit name or a character (default: the label's first letter). With `resizable`, the panel's edge is a separator to drag (or focus and use the arrow keys: 16 px, Shift for 4, Home and End to the limits, Enter to fold the panel; a double click resets it), and `bindWidth` keeps its width in state. Parts: `rail`, `panels`, `main`; event `resize` with `{ width }`. |
| `ui-scroll-spy` | `offset` (pixels a sticky header covers; default 0), `label` (default `On this page`), `tag` (default `nav`) | Navigation that follows the reading, for a table of contents or a docs sidebar. Its children hold in-page links (`href="#id"`) in any layout; as the page scrolls, the link for the current section (the last whose top has passed a line just below `offset`, or the last one at the very bottom) gets `aria-current="location"`, shown in bold on the accent colour, and is kept in view inside a list that scrolls on its own, without moving the page. A sticky bar that sets `--site-nav` on the page counts towards the offset. Links that set their own `color` or `background` outrank its highlight, as a document's styles do: give them `"&[aria-current=location]": { … }` too. |
| `ui-sidebar-layout` + `ui-sidebar` | layout: `bind`, `collapsible` (offcanvas, icon, none), `variant` (sidebar, inset, floating), `side`, `width`, `remember`; sidebar: `label` | See [Sidebar](#sidebar). |
| `ui-sidebar-header`, `ui-sidebar-content`, `ui-sidebar-footer`, `ui-sidebar-group`, `ui-sidebar-item`, `ui-sidebar-trigger` | group: `label`; item: `label`, `icon`, `href`, `active`, `badge`, `open`; trigger: `label` | The parts of a sidebar. An item with children opens a sub-menu. |
| `ui-sidebar-action` | `label` (required), `icon` (default ⋯), `showOnHover` | A small button beside a group's label (Add project) or at the end of an item's row (⋯). It runs its `on.click`, or opens its `ui-menu-item` children as a dropdown menu. `showOnHover` hides an item's until the row is hovered or has focus. Hidden while the sidebar is narrowed to icons. Part: `menu`. |
| `ui-input-otp` | `bind`, `length` (default 6, at most 12), `pattern` (digits, letters, alphanumeric), `groups` (e.g. `[3, 3]`), `mask`, `label` (when no visible label names it), `name`, `disabled`, `invalid`, `id` | A one-time code in a row of boxes, after shadcn/ui's InputOTP. One real text field lies over the boxes, so typing, pasting, and the phone's suggestion of a code from a message (`autocomplete="one-time-code"`) all work, it's one stop in the tab order, and screen readers read it as one field; the boxes draw its characters and the caret. What the `pattern` doesn't allow is dropped (a pasted `123-456` gives `123456`), letters are kept in capitals, and `mask` shows dots. `groups` puts a dash between groups. Filling the last box fires a bubbling `complete` event (`detail.value`) for `on: { complete: … }`; clearing `bind` from state empties it. In a `ui-field`, the label names it. Part: `input`. |
| `ui-combobox` | see [Combobox](#combobox) | A searchable select in the style of react-select: multi-select, groups, async, creatable. |
| `ui-color-picker` | see [Colour picker](#colour-picker) | A colour: picked on a saturation and brightness area with hue and opacity sliders, typed as any CSS colour, taken from the screen, or chosen from swatches. A button opens it; `inline` shows it on the page. |
| `ui-date-picker`, `ui-calendar` | see [Date picker](#date-picker) | A day (with a time), month, quarter or year; one, a range or several; presets, limits and financial years. The picker opens the calendar from a button; `ui-calendar` shows it inline. |
| `ui-command` | `placeholder`, `label`, `empty` (default `No results found.`), `id`, `rank` (default true), `bindSearch`, `loading`, `loadingText` (default `Loading…`) | A command palette: a search box over a list of actions, filtered as you type: every word must match the label, at its start, at a word's start, inside it or as its letters in order (`stg` finds Settings), or the `keywords`. The best matches come first, in each group and of the groups (a label starting with what's typed, then a word starting with it, then inside, then letters in order; keywords count less, shorter labels win ties), and the matched letters are marked (with the CSS Custom Highlight API, where the browser has it). `rank: false` keeps the items' order. An item with `page: theme` opens the `ui-command-page` with that `id` (and the palette stays open): its items show in place of the rest, the page's `title` as a crumb in the search box, and Backspace in the empty box (or Escape, or the crumb) goes back; pages can open pages. `bindSearch` keeps the search text in state, both ways, to fetch items for it; while they're fetched, `loading` (e.g. `{{ searching }}`) shows a Loading row with a spinner, announced to screen readers, marks the list busy and holds back the empty message. Up and Down move between items, skipping disabled ones; Home and End jump while the box is empty; Enter runs the active item; Escape clears the search. Focus stays in the search box. Parts: `search`, `input`, `list`, `loading`, `empty`. |
| `ui-command-dialog` | as `ui-command`, plus `trigger`, `triggerVariant`, `shortcut` (e.g. `mod+k`: ⌘K on Apple devices, Ctrl+K elsewhere), `title` (default `Command palette`) | The palette in a modal, opened by its trigger, the shortcut, or a button with `commandfor` its id. Running an item closes it, and it opens with an empty search. |
| `ui-command-group`, `ui-command-item`, `ui-command-separator` | group: `heading`; item: `label`, `keywords` (a string or a list), `icon`, `shortcut`, `href`, `disabled` | The palette's contents. An item runs its `on.click`, or follows `href`. A group hides while none of its items match, and separators hide while searching. |

| `ui-conversation` | `messages`, `actions`, `typing`, `assistant`, `render`, `markdown`, `empty`, `label`, `height`, `maxHeight` | A chat log; see [Chat](#chat). |
| `ui-message` | `sources`, `attachments`, `actions`, `messageId`, `feedback`, `role` (assistant, user, system), `content`, `markdown`, `name`, `time`, `avatar`, `initials`, `html`, `ui`, `render`, `replies`, `open` | One message bubble, with optional UI under it (see [UI in messages](#ui-in-messages)) and a thread. Children go in the bubble. |
| `ui-chat-input` | `messages` (required), `endpoint`, `body`, `headers`, `bindFiles`, `bind`, `waiting`, `threads`, `thread`, `placeholder`, `label`, `suggestions`, `disabled` | The box a message is written in; sends to `endpoint` and streams the reply in. Children go in its toolbar. |
| `ui-typing-indicator` | `label` (default `Typing`) | Three bouncing dots. |
| `ui-carousel` | `label` (default `Carousel`), `size` (each slide's width; default `min(280px, 85%)`), `gap` (default 12), `dots`, `loop`, `autoplay` (ms) | Slides in a row that scrolls sideways and snaps to each; children are the slides, and a `list` with `style: { display: contents }` works. The arrow buttons, Left and Right on the focused track, and swiping move a slide at a time, to the next slide's own start, so slides of different widths step evenly; a button hides at its end, unless `loop` goes round to the other. `dots` adds a dot per slide under it, the current one marked, each going to its slide. `autoplay: 5000` moves on every 5 seconds, round and round, with a pause button; it waits while the pointer or focus is on it, the tab is hidden, or the reader prefers less motion. Each slide is a `group` named "2 of 5". Parts: `track`, `previous`, `next`, `dots`, `play`. |
| `ui-form` | `rules`, `bindErrors`, `bindValid`, `bindChecking`, `bindTried`, `keys`, `bindDirty`, `guard`, `guardText`, `disabled`, `columns` | A form whose fields are checked by `rules` as they're left and when it's submitted (`on: { submit: … }` runs only when all is right), which knows its unsaved changes. A `remote` rule asks the server (is this name taken?). Put a `ui-error-summary` and a `ui-save-bar` in it. See [Forms](#forms). |
| `ui-error-summary` | `title`, `errors` | What's wrong in the `ui-form` it's in, listed once submitting was tried; each message goes to its field. See [Forms](#forms). |
| `ui-form-section` | `title`, `description`, `columns`, `layout` (stacked, aside), `span`, `level`, `collapsible`, `open` | A titled group of fields with columns of its own; sections go in a `ui-form`, side by side when it has `columns`, or in each other, and fold away with `collapsible`. See [Forms](#forms). |
| `ui-save-bar` | `text`, `saveLabel`, `discardLabel`, `saving`, `disabled`, `position` (sticky, fixed, inline) | Shown while the `ui-form` it's in has unsaved changes: Save submits the form, Discard puts its fields back. Children go before the buttons. See [Forms](#forms). |
| `ui-stepper` + `ui-step` | stepper: `bind` (required), `bindFurthest`, `orientation` (horizontal, vertical), `linear`, `clickable`, `buttons`, `nextLabel`, `backLabel`, `finishLabel`, `complete`, `label`, `id`; step: `value`, `label` (required), `description`, `icon`, `optional`, `valid`, `error`, `disabled` | The steps of a process and the current one's panel, with Back, Next and Finish. Children are `ui-step`s. See [Forms](#forms). |
| `ui-approval` | `bind` (required), `me`, `owner`, `bindStatus`, `readOnly`, `locale`, `label`, `id` | A sign-off in stages, each approved when its rule is met: who decided what, when and why; Approve, Request changes and Reject on your turn. See below. |
| `ui-repeater` | `bind` (required), `fields` (required), `layout` (table, stack), `newItem`, `min`, `max`, `reorderable`, `duplicable`, `addLabel`, `itemLabel`, `label`, `emptyText`, `disabled`, `bindErrors`, `bindTotals`, `showErrors`, `locale`, `id` | Rows of fields to add, fill in, reorder and remove: line items, contacts, rules. State gets a list of objects. Children are drawn after the fields, with the row as `item` and its place as `index`. See [Forms](#forms). |
| `ui-sortable-list` | `bind` (required), `itemKey` (default `id`), `labelField` (default `label`), `numbered`, `removable`, `emptyText`, `label`, `disabled` | A list in state put in order by dragging each item's grip (mouse, touch or pen) or from the keyboard; children are each item's content. See below. |
| `ui-form-builder` | `bind` (required), `bindDocument`, `types`, `submitLabel`, `endpoint`, `label`, `id` | A form put together in the browser: field types from a palette, put in order, set up in an inspector, tried in a preview, and given out as an iRender document of the kit's own form. See below. |
| `ui-import` | `bind` (required), `bindImported`, `fields` (required), `url`, `params`, `batchSize`, `accept`, `maxRows`, `skipInvalid`, `template`, `title`, `locale`, `id` | Rows from a spreadsheet (CSV, Excel, or pasted cells): its columns are matched to `fields`, the rows put right in an editable table, and the good ones handed over (an `import` event and `bindImported`) or posted to `url`. See [Import](#import). |
| `ui-tags-input` | `bind` (required), `suggestions`, `only`, `max`, `pattern`, `strict`, `lowercase`, `placeholder`, `label`, `name`, `disabled`, `invalid` | Tags typed one after another: Enter or a comma adds one, a pasted list adds them all, Backspace takes the last. |
| `ui-masked-input` | `bind` (required), `mask` (required), `raw`, `bindComplete`, `placeholder`, `label`, `name`, `disabled`, `invalid` | Text in a fixed shape, like a card number, a postcode or a policy number, put in shape as it's typed. |
| `ui-phone-input` | `bind` (required), `country`, `countries`, `bindCountry`, `bindValid`, `placeholder`, `label`, `name`, `disabled`, `invalid`, `locale` | A phone number with its country, picked from a searched list or from a pasted `+…` number, grouped as it's typed. |
| `ui-time-picker` | `bind` (required), `step`, `min`, `max`, `disabledTimes`, `seconds`, `hour12`, `durationFrom`, `placeholder`, `label`, `name`, `disabled`, `invalid`, `locale` | A time of day, typed any way (`9`, `930`, `9:30pm`, `21.30`) or picked from a list. |
| `ui-mention-input` | `bind` (required), `people` (required), `bindMentions`, `trigger`, `placeholder`, `rows`, `maxLength`, `label`, `name`, `disabled`, `invalid` | A text area that names people with `@`, from a list that opens at the caret. State gets the text, and the names in it. |
| `ui-tree-select` | `bind` (required), `nodes` (required), `multiple`, `leavesOnly`, `path`, `search`, `expanded`, `clearable`, `placeholder`, `label`, `disabled`, `invalid`, `height` | A choice from a hierarchy: a field that opens a searchable tree, for one node or several. |
| `ui-transfer-list` | `bind` (required), `options` (required), `titles`, `search`, `reorderable`, `max`, `height`, `disabled` | Two lists: what there is and what's chosen, moved between with buttons, a double click, Enter or dragging. |
| `ui-query-builder` | `bind` (required), `fields` (required), `maxDepth`, `negate`, `bindText`, `bindValid`, `rows`, `bindMatches`, `showText`, `label`, `disabled`, `locale` | Conditions put together with and / or, nested in groups: for segments, alerts, rules, access. State gets the query, its words, and the rows that match. |
| `ui-signature` | `bind` (required), `bindMeta`, `methods`, `format` (png, svg), `height`, `color`, `hint`, `label`, `disabled` | A signature, drawn with a finger, pen or mouse, or typed; state gets it as a picture. |
| `ui-image-cropper` | `bind` (required), `src`, `ratio`, `round`, `width`, `maxWidth`, `type` (jpeg, png, webp), `quality`, `bindCrop`, `height`, `label` | A picture cropped, turned and zoomed in the browser, by dragging or the keys. |
| `ui-qr-code` | `value` (required), `size`, `level` (L, M, Q, H), `logo`, `logoIcon`, `logoText`, `logoSize`, `logoShape` (rounded, circle, square), `logoBackground`, `logoColor`, `dots` (square, round), `color`, `background`, `download`, `label` | A QR code of any text, drawn in the page without a service, with a logo in it if there's one. |
| `ui-barcode` | `value` (required), `type` (code128, code39, ean13, ean8, upca, itf), `height`, `barWidth`, `showText`, `download`, `label` | A barcode: Code 128, Code 39, EAN, UPC or ITF. |
| `ui-code-scanner` | `bind` (required), `bindCode`, `bindList`, `formats`, `continuous`, `manual`, `sound`, `picture`, `decoder` (auto, native, wasm), `library`, `wasm`, `label` | Reads QR codes and barcodes with the camera in any browser, from a picture, or from a field (a handheld scanner types into it). |
| `ui-stat` | `label` (required), `value` (required), `format` (number, currency, percent, compact, none), `currency`, `decimals`, `unit`, `previous`, `change`, `changeAs` (percent, value), `changeLabel`, `goodWhen` (up, down, neutral), `trend`, `target`, `targetLabel`, `description`, `icon`, `href`, `variant` (card, plain), `loading`, `locale` | A figure: its label, value, change from before, a small trend line and progress to a target. Children go at the foot. |
| `ui-description-list` + `ui-description-item` | description-list: `items`, `columns`, `layout` (stacked, inline), `divided`, `labelWidth`; description-item: `label` (required), `value`, `format` (number, currency, percent, date, datetime), `currency`, `href`, `copy`, `span`, `bind`, `edit` (text, multiline, number, date, select), `options`, `placeholder`, `locale` | Labels and their values, as at the top of a record's page. Children are `ui-description-item`s. |
| `ui-item-group` + `ui-item` | item-group: `label`, `variant` (divided, cards, plain); item: `title`, `description`, `icon`, `image`, `initials`, `meta`, `tone` (neutral, info, success, warning, danger, primary), `href`, `selected`, `disabled`, `size` (default, sm), `variant` (plain, outline, muted) | A list of `ui-item`s (a `list` works as its child). |
| `ui-comments` | `bind` (required), `user`, `people`, `reactions`, `resolvable`, `replies`, `editable`, `order` (oldest, newest), `placeholder`, `emptyText`, `maxLength`, `label`, `disabled`, `locale` | Comments in threads: replies, mentions, reactions, resolving, editing one's own. Each change is written to `bind` and fires an event. |
| `ui-lightbox` | `images` (required), `layout` (grid, row, masonry, stack), `minWidth`, `ratio`, `max`, `captions`, `download`, `bind`, `label` | Pictures that open over the page: step with the arrows or a swipe, zoom with the wheel, a pinch or a double click, and save. |
| `ui-compare-slider` | `before`, `after` (required), `beforeAlt`, `afterAlt`, `beforeLabel`, `afterLabel`, `position`, `bind`, `orientation` (horizontal, vertical), `hover`, `ratio`, `caption`, `label` | Two pictures, before and after, one over the other, with a divider dragged across (or moved with the keys) to show more of either. See below. |
| `ui-iframe` | `src` (required), `title`, `height` (default 400), `ratio`, `sandbox`, `allow`, `loading` (lazy, eager), `referrerPolicy`, `bordered` (default true) | Another page shown inside this one, sandboxed and loaded as it nears the screen; anywhere in the tree. See below. |
| `ui-pivot` | `rows` (required), `fields`, `down`, `across`, `values`, `filter`, `bind`, `editable`, `totals` (both, rows, columns, none), `subtotals`, `heat`, `bindCell`, `export`, `height`, `title`, `emptyText`, `locale` | A pivot table: rows summed up by fields down and across, with subtotals that fold, shares, a heat tint, sorting, and the rows behind a cell. See [Pivot table](#pivot-table). |
| `ui-scheduler` | `resources` (required), `events` (required), `bindEvents`, `bindEvent`, `bindDate`, `bindView`, `view` (day, week, month), `views`, `from`, `to`, `snap`, `overlap` (allow, mark, prevent), `creatable`, `newEvent`, `moveResources`, `bindClashes`, `rowLabel`, `rowHeight`, `height`, `now`, `label`, `locale` | Bookings against time, a row per person, room, vehicle or machine: moved, stretched and made by dragging or the keys, with clashes marked. See [Scheduler](#scheduler). |
| `ui-plan` | `items`, `layout`, `data`, `floors`, `bindFloor`, `multiple`, `max`, `available`, `bindPicked`, `bindTotal`, `width`, `height`, `image`, `bindItems`, `bind`, `bindItem`, `tones`, `grid`, `filter`, `search`, `legend`, `viewHeight`, `label` | Things placed on a drawing: rooms, seats, bays, beds, plots, machines. Zoomed and moved, picked (one or several), coloured by status, and rearranged by dragging. Aircraft cabins, buses, trains, cinemas, theatres, auditoriums, stadiums, halls, car parks and offices come from a few words, on one floor or several. See [Plan](#plan). |
| `ui-pdf` | `src` (required), `bindPage`, `bindPages`, `notes`, `bindNotes`, `tools`, `signature`, `user`, `bindNote`, `download`, `library`, `height`, `title` | A PDF on the page: zoomed, its text selected and found with the browser, and marked with highlights, boxes, notes and signatures. Drawn by pdf.js, loaded when there's one to show. See [PDF](#pdf). |
| `ui-diff` | `before` (required), `after` (required), `mode` (text, fields), `view` (split, unified), `bindView`, `language`, `context`, `labels`, `fieldLabels`, `unchanged`, `wrap`, `title`, `height`, `label` | Two versions compared: text or code line by line, with the words that changed, or two records field by field. |
| `ui-log` | `lines` (required), `levels`, `search`, `time` (time, datetime, relative, none), `follow`, `wrap`, `maxLines`, `bindLine`, `download`, `title`, `emptyText`, `height`, `label`, `locale` | Lines that follow as they arrive: levels to filter by, a search, wrapping, and saving. Terminal colours in the text are drawn. |
| `ui-code-editor` | `bind` (required), `language`, `rows`, `maxRows`, `lineNumbers`, `wrap`, `indent`, `diagnostics`, `completions`, `bindCursor`, `placeholder`, `readOnly`, `label`, `name` | Code or a template, typed over its highlighted copy: line numbers, indenting, closing brackets, comments, words offered, and problems marked. |
| `ui-banner` | `text`, `title`, `tone` (neutral, info, success, warning, danger, primary), `variant` (soft, solid), `icon`, `bind`, `remember`, `sticky` (top, bottom), `align` (start, center), `until`, `from`, `countdown` | A message across the page: maintenance, an offer, a trial's end, a setting to finish. Children (links, buttons) go after it. |
| `ui-tour` | `bind` (required), `steps` (required), `autoStart`, `id`, `backdrop`, `interactive`, `doneLabel` | Steps that point at parts of the page one after another, for someone new to it. Draws nothing until it starts. |
| `ui-session-timeout` | `after` (required), `warn`, `expires`, `bindLeft`, `extendUrl`, `signOutUrl`, `title`, `text`, `stayLabel`, `signOutLabel`, `channel` | Asks whether someone's still there before their session ends, counting down, across the tabs they have open. |
| `ui-attachment` | `files` (required), `bindFiles`, `layout` (chips, cards), `label` | Files sent with a message or being sent: a name, size and picture, progress, what went wrong, and a button to remove each. |
| `ui-sources` | `items` (required), `text`, `layout` (list, chips, fold), `label` | What an answer is based on: numbered sources, and the `[1]`s in its text as links that show them. |
| `ui-tool-call` | `title` (required), `status`, `name`, `input`, `output`, `error`, `duration`, `open`, `bindApproval`, `label` | Something an assistant or a job did, or asks leave to do: what, how it went, and what went in and out. |
| `ui-trace` | `spans` (required), `kinds`, `bindSpan`, `expanded`, `height`, `title`, `label` | The steps of a run, nested and against time: an assistant's, a job's, a request's. A step opens to what went in and out of it. |
| `ui-questionnaire` | `questions` (required), `bind` (required), `mode` (steps, all), `bindDone`, `skippable`, `submitLabel`, `doneText`, `title`, `label` | Questions answered one at a time or together: choices, text, numbers, a scale. For an assistant's questions, surveys and intake forms. |
| `ui-install-prompt` | `variant` (button, card, floating), `label`, `title`, `text`, `icon`, `bindState`, `remember`, `days`, `always` | Offers to install the app: the browser's own prompt where there is one, else the steps for this browser (on an iPhone, Add to Home Screen). Hidden in the installed app. Needs the document's `app`. See [Apps](#apps). |
| `ui-update-prompt` | `title`, `text`, `label`, `laterLabel`, `every`, `inline`, `show`, `bind` | Says when a new version of the app has been fetched, and reloads into it when asked. `reload` comes first, for saving what's open. See [Apps](#apps). |
| `ui-offline-indicator` | `bind`, `variant` (badge, banner, dot, none), `showOnline`, `checkUrl`, `every`, `offlineText`, `onlineText`, `reconnectingText`, `detail` | Whether there's a connection: a badge, a banner or a dot, and state for the rest of the page. See [Apps](#apps). |
| `ui-offline-queue` | `paths`, `methods`, `labels`, `labelField`, `variant` (list, badge, none), `bind`, `bindCount`, `retryEvery`, `title`, `emptyText`, `hideEmpty` | Keeps what's sent without a connection (submits, forms and uploads) and sends it once there is one, in order, even after the app's been closed. Shows what's waiting and what was refused. Each is sent with an `x-queued-id` header, so a server can tell one it has had. See [Apps](#apps). |
| `ui-local-store` | `keys` (required), `name`, `where` (device, session), `maxAge`, `version`, `sync`, `bindReady`, `bindSaved`, `clear` | Keeps state keys on the device between visits. Draws nothing. Not for secrets: anyone with the device can read them. See [Apps](#apps). |
| `ui-app-shell` + `ui-app-screen`, `ui-app-actions` | app-shell: `bind` (required), `title`, `height`, `rail`, `bar`, `pullToRefresh`, `bindRefreshing`, `history`, `label`; app-screen: `value` (required), `label` (required), `title`, `icon`, `badge`, `parent`, `flush` | An app's frame: a bar with the screen's title and a Back button, the screen, and tabs at the foot (down the side with room for them), clear of a phone's notch. Children are `ui-app-screen`s and one `ui-app-actions`, at the end of the bar. See [Apps](#apps). |
| `ui-camera` | `bind` (required), `multiple`, `max`, `facing` (back, front), `ratio`, `maxSize`, `quality`, `type` (jpeg, webp, png), `review`, `grid`, `timer`, `guide` (none, card, face, square), `stamp`, `files`, `autoStart`, `label`, `disabled` | Takes photos: front and back camera, light, zoom, composing lines, a timer, retakes and several shots, made smaller before they're kept. Where there's no camera to use, the device's own picker. See [Device and hardware](#device-and-hardware). |
| `ui-permission-manager` | `permissions` (required), `bind`, `bindReady`, `title` | What the app may use (camera, location, notifications…): why, whether it's allowed, a button to ask, and how to allow what was blocked. See [Device and hardware](#device-and-hardware). |
| `ui-photo-annotate` | `src` (required), `bind`, `bindMarks`, `tools`, `colors`, `width`, `unit`, `type` (jpeg, png, webp), `quality`, `maxSize`, `readonly`, `label` | Marks on a photo: arrows, boxes, circles, freehand, text, measurements and blurring (for faces and number plates), kept apart from the picture and drawn onto a copy of it. See [Device and hardware](#device-and-hardware). |
| `ui-location` | `bind` (required), `watch`, `auto`, `precise`, `enough`, `manual`, `link`, `label` | Where the device is, once or as it moves, with how accurate that is; typed instead where it can't be found. See [Device and hardware](#device-and-hardware). |
| `ui-field-map` | `center`, `zoom`, `markers`, `areas`, `lines`, `cluster`, `bindSelected`, `bindView`, `bindPoint`, `bindDrawn`, `draw`, `locate`, `position`, `fit`, `height`, `tiles`, `attribution`, `darkTiles`, `wheel`, `minZoom`, `maxZoom`, `label` | A light map to move and zoom, drawn without a library: markers (grouped when they're close), areas, lines, where the device is, and drawing on it. See [Device and hardware](#device-and-hardware). |
| `ui-geo-checkin` | `bind` (required), `places` (required), `bindLog`, `outside` (block, reason, allow), `enough`, `ready`, `photo`, `inLabel`, `outLabel` | Checks in and out where someone has to be: how far they are from the place, the time, how long it's been, and a photo. Children (a `ui-camera`, say) go before the button. A device can be made to lie about where it is, so it's a record, not proof. See [Device and hardware](#device-and-hardware). |
| `ui-route-tracker` | `bind` (required), `step`, `enough`, `units` (metric, imperial), `awake`, `download` | Records a path while the app is open and in front: distance, time and speed, with pause, and GPX to save. Draw it on a `ui-field-map`, and keep it with `ui-local-store`. See [Device and hardware](#device-and-hardware). |
| `ui-compass` | `bind`, `heading`, `target`, `from`, `size` | Which way the device faces, and the way to a place from where one is. A phone's magnet is easily put off by metal. See [Device and hardware](#device-and-hardware). |
| `ui-level` | `bind`, `value`, `tolerance`, `mode` (surface, edge, auto), `size` | A spirit level from the device's own sensors, set to zero against a surface known to be level. See [Device and hardware](#device-and-hardware). |
| `ui-push-opt-in` | `publicKey`, `saveUrl`, `topics`, `bind`, `title`, `text`, `label`, `when`, `remember`, `days`, `test` | Offers notifications at a good moment, says why, then asks; subscribes to pushes and sends the subscription to a server, by topic. The app's service worker (`irender app`) shows them. See [Apps](#apps). |
| `ui-app-badge` | `count` (required), `title` | A number on the installed app's icon, and in the tab's title. Draws nothing. See [Apps](#apps). |
| `ui-share` | `url`, `title`, `text`, `files`, `label`, `variant` (primary, outline, ghost), `targets`, `sheet` (auto, own) | Shares a link, text or files with the device's own sheet; where there's none, a list: copy, email, download and others. See [Apps](#apps). |
| `ui-share-target` | `bind` (required), `quiet`, `acceptLabel`, `emptyText` | Takes what other apps share with this one, once it's installed: put it on the page that the document's `app.share` names. See [Apps](#apps). |
| `ui-contact-picker` | `bind` (required), `multiple`, `fields`, `manual`, `label` | Contacts picked from the phone's own list (Chrome on Android); anywhere else, from a contact card file (.vcf) or typed. See [Apps](#apps). |
| `ui-passkey` | `mode` (signin, register), `optionsUrl`, `verifyUrl`, `body`, `bind`, `autofill`, `local`, `name`, `label`, `variant` (primary, outline) | Signs in, or makes the passkey to, with a fingerprint, a face or the device's PIN. The server makes the challenge and checks the answer. See [Apps](#apps). |
| `ui-app-lock` | `methods`, `after`, `hidden`, `digits`, `tries`, `name`, `bindLocked`, `title`, `logo`, `signOutUrl`, `reset`, `settings` | Covers the app after a while, or once it's been out of sight, until it's opened with a passkey or a PIN. It keeps someone who picks up the device out; it doesn't encrypt anything, and a server still has to check who's asking. See [Apps](#apps). |
| `ui-audio-recorder` | `bind` (required), `multiple`, `max`, `bitrate`, `label`, `disabled` | Voice notes: records with a moving waveform, pauses, plays back and goes to any point of it. See [Device and hardware](#device-and-hardware). |
| `ui-video-recorder` | `bind` (required), `max`, `facing` (front, back), `audio`, `bitrate`, `height`, `label`, `disabled` | Records a short video with a time limit, pauses, and plays it back to keep or take again. See [Device and hardware](#device-and-hardware). |
| `ui-screen-capture` | `bind` (required), `modes`, `max`, `audio` | A picture or a video of a screen, a window or a tab, which someone picks: for reporting a problem, say. Phones can't. See [Device and hardware](#device-and-hardware). |
| `ui-voice-input` | `bind` (required), `append`, `continuous`, `lang`, `transcribeUrl`, `iconOnly`, `label` | Speech to text, shown as it's heard. Some browsers send what's said to their maker's servers to do it. See [Device and hardware](#device-and-hardware). |
| `ui-read-aloud` | `text`, `target`, `lang`, `voice`, `rate`, `controls`, `bindState`, `label` | Reads text, or a part of the page, out loud with the device's own voices, with pause and speed. See [Device and hardware](#device-and-hardware). |
| `ui-document-scanner` | `bind` (required), `bindPdf`, `max`, `filter` (colour, grey, bw, none), `paper` (auto, a4, letter), `maxSize`, `quality`, `name`, `label` | Scans papers with the camera or from a picture: finds the page's edges, straightens it, cleans it up, and makes a PDF of several. See [Device and hardware](#device-and-hardware). |
| `ui-swipe-actions` | `start`, `end`, `full`, `menu`, `label`, `disabled` | A row that's swiped sideways to show what can be done with it: archive, delete, call. Its children are the row. See [Device and hardware](#device-and-hardware). |
| `ui-wake-lock` | `active`, `bind`, `label`, `bindHeld` | Keeps the screen on: a recipe, a score, a boarding pass, a timer. See [Device and hardware](#device-and-hardware). |
| `ui-nfc` | `bind` (required), `bindList`, `write`, `lock`, `label` | Reads NFC tags held to the phone, and writes to them (Chrome on Android). See [Device and hardware](#device-and-hardware). |
| `ui-bluetooth` | `read` (required), `bind`, `bindState`, `commands`, `namePrefix`, `label` | Connects to a Bluetooth Low Energy device, reads and follows its values, and sends to it (Chrome and Edge; not iPhones). See [Device and hardware](#device-and-hardware). |
| `ui-serial` | `baud`, `bind`, `bindLog`, `bindState`, `parse` (text, number, json, csv), `end` (lf, crlf, cr, none), `filters`, `send`, `commands`, `log`, `max`, `label` | Talks to a serial port, a line at a time: scales, meters, microcontrollers (Chrome and Edge on a computer). See [Device and hardware](#device-and-hardware). |
| `ui-usb` | `filters` (required), `bind`, `bindState`, `bindDevice`, `interface`, `as` (text, hex, bytes), `commands`, `label` | Talks to a USB device that no driver has taken: label printers, scanners, instruments (Chrome and Edge). See [Device and hardware](#device-and-hardware). |
| `ui-file-handler` | `bind` (required), `as` (text, json, data), `accept`, `save`, `name`, `bindDirty`, `label` | Opens files, from a button or, installed, when one's opened with the app (the document's `app.files`), and saves back to them. See [Device and hardware](#device-and-hardware). |
As with the built-ins, a document can override any attribute with `attrs`, any style with `style`, and inner elements with `parts`. The kit only sets defaults.

Options, items and page numbers can come from `state` as well as server data: `ui-radio-group` and `ui-toggle-group` options, `ui-breadcrumb` items, and `ui-pagination`'s `page`, `pages` and `siblings`. The server renders them for the starting state and the browser redraws them as the state changes. A breadcrumb's `items` from state must be a path, like `"{{ crumbs }}"`, so it can tell which item is last. Pagination from state adds a small behaviour that redraws its links.

`icon` on `ui-button`, `ui-badge`, `ui-alert`, `ui-sidebar-item` and `ui-command-item` takes an icon name, like `settings` or `it:database`, written in the document or coming from server data such as a list item, drawn by [irender-icon-kit](../irender-icon-kit/README.md) when it's registered. Otherwise, or for anything that isn't a name, it is shown as text, so an emoji or a character like `⚠` works too.

## Sidebar

A page with a navigation panel that slides away or narrows to its icons, in the style of shadcn/ui's sidebar:

```yaml
state: { sidebar: true }
root:
  type: ui-sidebar-layout
  props: { bind: sidebar, collapsible: icon, variant: inset }
  children:
    - type: ui-sidebar
      children:
        - type: ui-sidebar-header
          children: [ { type: ui-sidebar-item, props: { label: Acme Inc, icon: grid, href: / } } ]
        - type: ui-sidebar-content
          children:
            - type: ui-sidebar-group
              props: { label: Platform }
              children:
                - { type: ui-sidebar-item, props: { label: Dashboard, icon: grid, href: /, active: true } }
                - type: ui-sidebar-item
                  props: { label: Settings, icon: settings }
                  children:
                    - { type: ui-sidebar-item, props: { label: Team, href: /team } }
                    - { type: ui-sidebar-item, props: { label: Billing, href: /billing } }
        - type: ui-sidebar-footer
          children: [ { type: ui-sidebar-item, props: { label: Account, icon: user } } ]
    # Everything after the ui-sidebar is the main area.
    - type: element
      props: { tag: header }
      children: [ { type: ui-sidebar-trigger } ]
```

- **Open and closed:** `bind` holds whether the sidebar is open, and `ui-sidebar-trigger` flips it. `collapsible: offcanvas` (the default) slides the panel away, `icon` narrows it to its icons, and `none` keeps it open. Narrowed to icons, labels are hidden from sight but still read by screen readers, and sub-menus close.
- **Phones:** below 768px wide the sidebar is a sheet over the page instead. It starts hidden, the trigger opens it whatever `bind` says, and a tap on the backdrop, Escape or following one of its links closes it. While it's open the page behind is `inert`, focus is in the panel and Tab goes round its controls, not out to the browser's own; closing it returns focus to the trigger. A swipe toward its edge closes it too: the panel follows the finger, and shuts past a third of its width or with a flick, else springs back.
- **Keyboard:** ⌘B (Ctrl+B) toggles the sidebar, or the sheet on a phone. With several layouts on a page, the first takes the shortcut.
- **Remembered:** `remember: true` keeps the sidebar open or closed across reloads, in the browser's localStorage under `bind`'s name (`irender-sidebar:<name>`); `remember: app` uses `app` instead, so the pages of a site share one choice. It's restored as the page loads, without sliding. The server's HTML follows `bind`'s starting value, so a page drawn closed but remembered open opens as its script runs.
- **Labels when collapsed:** narrowed to icons, an item shows its label beside it on hover or keyboard focus.
- **Variants:** `sidebar` puts a border between the panel and the page, `inset` makes the main area a raised panel on the sidebar's colour, and `floating` floats the panel itself.
- **Items:** a link with `href`, otherwise a button that runs its `on.click`. `active` marks the current page (`aria-current="page"`). Children make a sub-menu, a native `<details>` the item opens and closes; `open` starts it open.
- **Actions:** a `ui-sidebar-action` in a group sits at the end of its label row; in an item, over the end of the item's row, which leaves room for it. Give it `on.click`, or menu items as children for a dropdown menu (shadcn's `SidebarGroupAction` and `SidebarMenuAction`). The block sidebar-07 has an Add project button and a ⋯ menu per project.
- **Colour:** the panel uses `color.sidebar` when the theme has it, otherwise a step off the surface colour, in light and dark. The main area takes `color.background`.

The [blocks](../../examples/blocks/) use it: a dashboard, a sidebar that collapses to icons and one with sub-menus.

## Theme

Components read the theme's colour tokens. Tokens the default theme doesn't have fall back to values derived from `color.text` and `color.surface`, so the kit looks right with any theme, in light and dark mode:

| token | used for | fallback |
|---|---|---|
| `color.secondary` / `color.on-secondary` | secondary buttons and badges | text mixed 6% into surface / text |
| `color.accent` | hover backgrounds | text mixed 6% into surface |
| `color.subtle` | skeletons, avatars, code, table hover | text mixed 6% into surface |
| `color.destructive` / `color.on-destructive` | destructive variants | `#dc2626` / white |
| `color.ring` | focus rings | `color.muted` |
| `color.brand-2` / `color.on-brand-2` | the `brand` variant of buttons and badges: a second brand colour | `color.primary` / `color.on-primary` |
| `color.background` | the page: the body of a full HTML page, the sidebar layout's main area | `color.surface` |
| `color.card` | cards, alerts, the inline calendar, the chat input | `color.surface` |
| `color.popover` | popovers, dropdown and context menus, hover cards, toasts, the command palette, the combobox's, date picker's and data table's pop-ups | `color.surface` |
| `color.dialog` | dialogs, alert dialogs, the command dialog | `color.surface` |
| `color.sheet` | sheets | `color.dialog` |
| `gradient.primary` / `gradient.primary-hover` | brand fills: default buttons and badges, progress bars, the chat's own messages and send button. Any CSS `background` value, e.g. `linear-gradient(135deg, #6366f1, #ec4899)` | `color.primary` / `color.primary-hover` |
| `gradient.brand-2` | the `brand` variant's fill | `color.brand-2`, then `gradient.primary` |

Inputs, the switch's knob and other controls stay on `color.surface`. A card, popover, dialog, sheet or sidebar sets `--ui-bg` to its background, so what's drawn inside it (pinned data table columns, tints, charts) matches what it sits on. Set the hover gradient alongside a gradient: without one, hovering shows the plain `color.primary-hover`.

```yaml
theme:
  tokens:
    color: { background: "#f4f4f5", card: "#ffffff" }
    gradient: { primary: "linear-gradient(135deg, #6366f1, #ec4899)", primary-hover: "linear-gradient(135deg, #4f46e5, #db2777)" }
  modes:
    dark: { color: { background: "#09090b", card: "#18181b" } }
```

`apply_theme` merges a neutral, shadcn-style palette (zinc) with a dark mode under the document's theme. Tokens the document sets itself keep their values.

### Theme options

`apply_theme` also adds theme options, and `ui-theme-option` picks one:

```yaml
- { type: ui-theme-option, props: { option: palette, swatch: color.primary } }
- { type: ui-theme-option, props: { option: density } }
```

| option | choices | sets |
|---|---|---|
| `palette` | zinc (default), slate, stone, rose, blue, green, orange, violet, custom | every colour (neutrals) or the primary and ring (colours), each with its dark mode; custom: the colours from `ui-theme-colors`. The colours and custom also set `palette-chart-1` … `8`, shades charts take while their own colour scheme is left at Default (see the [chart kit](../irender-chart-kit/README.md#theme)). |
| `radius` | 0, 0.3, 0.5 (default), 0.75, 1.0 | `radius.sm`, `radius.md`, `radius.lg` |
| `border` | thin (default), medium, thick | `border.width`: 1, 1.5, 2px, on every 1px border |
| `elevation` | flat, default, raised | `elevation.control`, `.card`, `.card-raised`, `.popover`, `.dialog`: the kit's shadows |
| `control-radius` | same as radius (default), square, small, large, round | `radius.control` (default `var(--radius-md)`): buttons, inputs, selects, toggles |
| `card-radius` | same as radius (default), square, small, large, extra large | `radius.card` (default `var(--radius-lg)`): cards, popovers, menus, dialogs |
| `density` | compact, default, comfortable | `space.scale`: 0.8, 1, 1.2 |
| `padding` | tight, default, roomy | `space.inset`: 0.75, 1, 1.25 |
| `gap` | tight, default, roomy | `space.gap`: 0.75, 1, 1.25 |
| `control-size` | small, default, large | `size.control`: 0.889, 1, 1.111 (32, 36, 40px controls) |
| `icon-size` | small, default, large | `size.icon`: 0.875, 1, 1.25 |
| `text-size` | small, default, large | `font.scale`: 0.9, 1, 1.125 |
| `font` | sans (default), serif, mono | `font.body` |
| `heading-font` | same as text (default), sans, serif, mono | `font.heading`: titles and headings |
| `font-weight` | lighter, default, bolder | `font.weight.medium`, `.semibold`, `.bold`: 400/500/600, 500/600/700, 600/700/800 |
| `line-height` | tight, default, relaxed | `font.leading`: 1.35, the page's own, 1.7; body text, descriptions, Markdown |
| `letter-spacing` | tight, default, wide | `font.tracking`: -0.01em, the page's own, 0.025em; headings add it to their own |

- **Picking:** `ui-theme-option` shows one button per choice; `swatch` names a token shown as a colour dot, or a list of them: `ui-theme-panel` shows a chart scheme's first four colours. A choice is kept in `localStorage` (`irender-theme-options`) and applied before the page shows on every page rendered with the kit, like `ui-theme-toggle`'s mode. Picking the default clears it. Pages already open follow at once, other tabs and pages in iframes included, so a picker on one page restyles the rest.
- **A Theme button:** `{ type: ui-popover, props: { trigger: Theme }, children: [ { type: ui-theme-panel } ] }` gives every option and the mode, including ones the document adds. The examples index, the blocks page and the dashboard block have one.
- **Scales:** the kit's paddings, gaps, margins and control heights are `calc(Npx * var(--space-scale, 1))`, and its font sizes `calc(Npx * var(--font-scale, 1))`, so density and text size reach every component. The finer options multiply on top of density: paddings also by `space.inset`, gaps and margins by `space.gap`, the heights and widths of controls, rows and cells by `size.control`, and icons (`ui-icon` at its default size, and the kit's own) by `size.icon`. Charts, flow diagrams and the built-ins follow them too (the built-in `button` and `input` paddings, and `stack` gaps given in px). Use `$space.*` and `$font.size.*`, or the scales directly, in your own styles to follow them: `padding: "$space.md"`, `padding: "calc(12px * var(--space-scale, 1))"`. Set a choice on a node to change one part of the page: `attrs: { data-theme-density: compact }` on a table's card.
- **The page's text:** every page rendered with the kit gets `font.body` on its `body`, and the line height and letter spacing once those options are picked (at no specificity, so a document's own styles win). An element with `data-theme-font`, `data-theme-line-height` or `data-theme-letter-spacing` gets them for its part of the page. Line height, letter spacing and heading font have no value until picked, so their defaults can't undo a choice made further up the page.
- **Custom colours:** `ui-theme-colors` takes a primary and a second brand colour as hex (`#rgb` or `#rrggbb`, nothing else), typed or picked. They're kept in `localStorage` (`irender-theme-custom`) and set on `<html>` while the palette's `custom` choice is picked, with a hover shade, a ring and dark or light text on them (whichever reads better) derived from each. A gradient end after each makes its fills a 135° gradient (darker on hover, text chosen for both ends); clearing it goes back to the plain colour. The backgrounds are kept per mode (`card` for light, `card-dark` for dark): the fields show and set the mode showing, and the heading says which. Clearing a field goes back to the theme's colour. The chart kit's `custom` chart colours are built from the same two, and setting a colour picks them too unless a chart scheme was picked by hand; Reset puts both back. In script: `window.__irenderTheme.custom()` and `.setCustom("primary", "#1e3a8a")`.
- **Your own:** add choices to an option (`theme.options.palette.choices.brand`) or options of your own (`theme.options.shadow`); `ui-theme-option` picks those too. The [theme block](../../examples/blocks/theme.yaml) does both.
- **In script:** `window.__irenderTheme.option("palette")` and `.setOption("palette", "rose")`; changes fire `irender:theme-option`.

## Combobox

`ui-combobox` is a searchable select modelled on [react-select](https://react-select.com/). Its behaviour (about 4 KB gzipped) is added only to pages that use it. It takes the theme's primary colour: the focus ring, the chosen values' chips, the highlighted option, the ✓ on chosen options, the spinner and "Create …" follow the palette, custom colours included.

```yaml
- type: ui-combobox
  props:
    bind: flavours            # state key: a value, or an array with `multiple`
    multiple: true
    clearable: true
    creatable: true           # offers "Create “…”" for new text
    fixed: [chocolate]        # chips that can't be removed
    options:
      - { value: vanilla, label: Vanilla, description: Creamy and classic }
      - { label: Fruity, options: [mango, { value: lime, disabled: true }] }   # a group
```

| react-select | `ui-combobox` |
|---|---|
| `isMulti`, `isClearable`, `isSearchable`, `isDisabled`, `isLoading` | `multiple`, `clearable`, `searchable` (default true), `disabled`, `loading`. The last two can be expressions. |
| grouped options, `isOptionDisabled` | groups `{ label, options }`, `disabled` on an option |
| `getOptionValue`, `getOptionLabel` | `valueField`, `labelField`, `descriptionField`: read options from objects shaped your way, e.g. `{ href, title, summary }` |
| `formatOptionLabel` (a picture, a count) | `image` (with `imageField`): a picture before the label, also on its chip, such as an avatar or a flag; `badge` (with `badgeField`): short text after it, such as a count or a status. The picture is decoration, the label names the option. |
| `AsyncSelect` `loadOptions`, `cacheOptions` | `searchUrl`: `GET <url>?q=<text>` answering an array (or `{ options }`), debounced (`debounce`, default 250 ms) after `minChars` (default 1), cached per query, stale answers dropped |
| `CreatableSelect` | `creatable`, `createText` |
| `closeMenuOnSelect`, `hideSelectedOptions` | `closeOnSelect` (default: unless `multiple`), `hideSelected` (default: with `multiple`) |
| fixed options | `fixed` |
| `name` for forms | `name`: one hidden input per value |
| `noOptionsMessage`, `loadingMessage`, `placeholder` | `noOptionsText`, `loadingText`, `placeholder` |
| options from props | `options` from data, or a state path (`"{{ cities }}"`) read live |
| `onChange` | `bind`; the element also fires a bubbling `change` event |

Matching ignores case and accents and looks at labels and values. The keyboard follows react-select:
- ↑ ↓ move through the options, skipping disabled ones.
- Home, End, Page Up and Page Down jump.
- Enter chooses. Tab chooses when you've typed something.
- Escape closes the menu, then clears the text. Inside a dialog or sheet, only an Escape with nothing left to undo closes it.
- Inside a dialog or sheet, the menu sits on top of it and flips above the field when there's no room below.
- Backspace on an empty field removes the last value.
- Space opens a non-searchable combobox.

It follows the WAI-ARIA combobox pattern (`role=combobox` / `listbox` / `option`, `aria-activedescendant`), and a live region announces result counts and choices. The menu is a popover placed under the field at its width, and flips above near the bottom of the screen.

Not replicated:
- Custom render functions (`formatOptionLabel` and friends). Options take a `description` instead.
- Chip animations.
- Values that are whole option objects: values are plain values, and their labels come from the options.
- `searchUrl` must be a path on the same site or an http(s) URL, and options loaded from it keep only the labels the API gives.

## Colour picker

`ui-color-picker` is a button showing a colour that opens a panel to pick one: a saturation and brightness area, a hue slider, an opacity slider (`alpha`), a text field that takes any CSS colour (`#1e3a8a`, `rgb(30 58 138)`, `hsl(…)`, `rebeccapurple`), an eyedropper where the browser has one (Chrome and Edge), and swatches. Its behaviour (about 4 KB gzipped) is added only to pages that use it.

```yaml
- type: ui-field
  props: { label: Brand colour }
  children:
    - type: ui-color-picker
      props:
        bind: brand                      # "#1e3a8a"
        alpha: true                      # "#1e3a8a80" at half opacity
        clearable: true
        swatches: ["#0f172a", "#2563eb", "#16a34a", { value: "#ffffff", label: White }]
- { type: ui-color-picker, props: { bind: tint, inline: true, format: rgb } }      # "rgb(30, 58, 138)"
- { type: ui-color-picker, props: { bind: tag, custom: false, swatches: [{ value: "#ef4444", label: Red }, { value: "#22c55e", label: Green }] } }
```

| prop | |
|---|---|
| `bind`, `value` | The state key holding the colour, or the starting colour when not bound: text in `format`, or null for none. Whatever state holds is read as any CSS colour. |
| `format` | `hex` (default: `#1e3a8a`, `#1e3a8a80` with opacity), `rgb` (`rgb(30, 58, 138)`, `rgba(…, 0.5)`) or `hsl` (`hsl(224, 64%, 33%)`, `hsla(…)`): what's written to state, in the comma form charts read too. |
| `alpha` | An opacity slider; without it colours are written opaque. |
| `swatches` | Colours to pick with one click: CSS colours, or `{ value, label }` (the label names it for screen readers and on hover). The one matching the value shows pressed. |
| `custom` | The area, sliders and text field (default true). `false` leaves only the swatches, and a pick closes the popover: a palette of set colours. |
| `eyedropper` | The button that takes a colour from the screen (default true); it shows only where the browser has the EyeDropper API. |
| `inline` | The panel on the page, without the button and popover. In a `ui-field`, the label names its text field. |
| `placeholder`, `clearable`, `align`, `disabled` | The button's text while there's no colour (default `Pick a colour`), a ✕ that clears it (a Clear button inline), which edge the popover lines up with, and turning it off. |
| `name` | A hidden input carries the colour in a form. |

The area is a slider for the keyboard: ←/→ change the saturation and ↑/↓ the brightness, a point at a time (Shift: ten). The hue and opacity are range inputs. A typed colour is taken as soon as it's one, and a mistake is marked on Enter or on leaving the field; Escape puts back the colour. The hue stays where it was while the colour is grey, white or black, so moving to a colour again keeps it. Each change fires a bubbling `change` event with `{ value }`.

## Date picker

`ui-date-picker` is a button that opens a calendar in a popover, in the style of shadcn/ui's date picker; `ui-calendar` is the calendar on its own, inline. They take the same props, and their behaviour (about 8 KB gzipped) is added only to pages that use them.

```yaml
- type: ui-field
  props: { label: Reporting period }
  children:
    - type: ui-date-picker
      props:
        bind: period                     # { from: 2026-03-01, to: 2026-03-31 }
        bindRange: periodDates           # the dates it covers, for a chart's or a table's filter
        mode: range
        months: 2
        max: today
        presets: [last7, last30, thisMonth, lastMonth, thisQuarter, yearToDate]
        compare: true                    # a Compare switch: the period before, last year's, or one picked
        bindCompare: periodVs            # { type: previous, from: 2026-01-29, to: 2026-02-28 }
- { type: ui-date-picker, props: { bind: meeting, time: true, step: 15 } }            # 2026-03-10T14:30
- { type: ui-date-picker, props: { bind: quarter, picker: quarter, fiscalStart: 4 } }  # 2026-Q1: April to June 2026
- { type: ui-calendar, props: { bind: shifts, mode: multiple, disabledDays: [0, 6] } }
- { type: ui-date-picker, props: { bind: sprint, picker: week, bindRange: sprintDays } }  # 2026-W48: 23 to 29 November
```

| prop | |
|---|---|
| `bind`, `value` | The state key holding the value, or the starting value when not bound. A day is `2026-03-01` (with `time`, `2026-03-01T14:30`), a month `2026-03`, a quarter `2026-Q1`, a year `2026`; a range is `{ from, to }` (each with its time, with `time`) and `multiple` a list. Partial ranges keep `to: null` while the end is being picked. |
| `picker` | `day` (default), `week`, `month`, `quarter` or `year`. A week is picked from the days, a click taking its whole week, and is kept as the ISO week, `2026-W48` (Monday to Sunday, the first week of a year holding its first Thursday), with `bindRange` giving its days; it's one week at a time, not with `range`, `multiple` or `input`. |
| `mode` | `single` (default), `range` or `multiple`. |
| `time`, `step` | A time under the calendar, for days: one for a single day; with `mode: range` a start and an end time, so a range is `{ from: 2026-03-02T09:00, to: 2026-03-06T17:30 }` (00:00 and 23:59 until they're picked, the whole of each day). A typed range takes them too: `03/02/2026 9:00 – 03/06/2026 5:30 pm`. `step` is the minutes between times (default 1). Not with `multiple`. |
| `min`, `max` | The earliest and latest dates: `2026-01-01`, or `today`, `today-30d`, `today+2w`, `today+1m`, `today+1y`. They may use state, and the arrows don't page past them. |
| `disabledDates`, `disabledDays` | Dates that can't be picked (`2026-12-25`, or `{ from, to }`), and weekdays (0 Sunday … 6 Saturday). |
| `months` | Months side by side (default 1); two suit ranges. |
| `weekStart`, `locale` | The first day of the week (default: the locale's) and the locale for names and formats (default: the document's `lang`, else the browser's). |
| `fiscalStart` | The month a financial year starts (1–12, default 1). Quarters and years follow it: with `fiscalStart: 4`, `2026-Q1` is April to June 2026 and shows as `Q1 FY2026–27`, and the year `2026` is April 2026 to March 2027. |
| `presets` | Shortcuts beside the calendar: `today`, `yesterday`, `tomorrow`, `last7`, `last30`, `last90`, `next7`, `next30`, `thisWeek`, `lastWeek`, `thisMonth`, `lastMonth`, `nextMonth`, `thisQuarter`, `lastQuarter`, `thisYear`, `lastYear`, `yearToDate`, `monthToDate`, `quarters` (the quarter today is in and the three before it, each its own button: `Q3 2026`, or `Q3 FY2026–27` on a financial year), or `{ label, from, to }` with dates or `today±N`. Quarters and years follow `fiscalStart`; the preset that matches the value shows pressed. |
| `dropdowns` | Month and year menus in place of the caption, for dates far away such as a birthday. |
| `bindRange` | A state key that gets the dates the value covers as `{ from, to }`: a quarter's first and last days, say. Give it to a chart's or a data table's `filter` to filter by the pick. |
| `compare`, `bindCompare` | With `mode: range` (not `time`): a Compare switch under the calendar, and a period to compare the range with. `true` offers `previous` (as many days, months, quarters or years just before), `lastYear` (the same dates a year earlier) and `custom` (the next two picks on the calendar set it); a list offers just those, the first being what the switch turns on. `bindCompare` holds `{ type, from, to }`, the dates compared, or null while it's off: give it to a chart's or a table's `filter` beside `bindRange`. The previous period and last year's follow the range as it changes; a custom one stays put. The comparison shows on the calendar as a band in the second chart colour, and after the value in the button as `vs …`. Clearing the value turns it off. |
| `name` | Hidden inputs carry the value in a form: `name`, or `name[from]` and `name[to]` for a range, or one per date; with a comparison, `name[compareFrom]` and `name[compareTo]` too. |
| `label` | The accessible name. In a `ui-field`, its label names the picker's button. |
| `clearable` | A ✕ in the picker's button (a Clear link under `ui-calendar`). |
| `input`, `bindError` | The picker only: a text field to type the value in, with the calendar's button beside it; see [Typing a date](#typing-a-date). |
| `placeholder`, `closeOnSelect`, `align`, `disabled` | The picker only: the text while empty (default `Pick a date`, `Pick a range`, …), closing once a value is complete (default true; not with `time` or `multiple`), which edge the popover lines up with (`start` or `end`), and turning it off. |

The calendar follows the WAI-ARIA date picker pattern: a grid with one cell in the tab order.
- Arrows move a day (or a month, quarter or year), up and down a week (or a row).
- Page Up and Page Down change month (Shift: year), or year for the other pickers.
- Home and End go to the start and end of the week (or row).
- Enter or Space picks. A range takes two picks and shows the span under the pointer while the end is chosen.
- Escape closes the picker, and focus goes back to its button.
- A live region says what was picked.

Days outside the month show muted (left out when two months show), today is marked, and the value's text in the button follows the locale. Each change fires a bubbling `change` event with `{ value, range }` (and `compare`, with `compare`).

### Typing a date

With `input: true`, the picker is a text field with the calendar's button beside it, as shadcn's "date picker with input". The field shows the value in the locale's numeric form (`15/03/2026` in `en-GB`, `03/15/2026` in `en-US`), and its placeholder is the pattern (`DD/MM/YYYY`). What's typed becomes the value on Enter or on leaving the field:

- Day, month and year in the locale's order, with any separator: `15/3/2026`, `15.03.26`, `15 3 2026`. Two-digit years are within 20 years ahead, or else the century before. Day and month alone mean this year.
- ISO dates (`2026-03-15`), month names in the locale or English (`15 Mar 2026`, `March 15`), and `today`, `tomorrow`, `yesterday`, `today+3d`.
- With `time`, a time after the date: `14:30` or `2:30 pm`.
- A range is two dates with `–`, `-`, `to` or `until` between them, in either order.
- Months as `03/2026` or `Mar 2026`, quarters as `Q2 FY2026–27` or `q2 2026`, years as `2026` or `FY2026–27`.

Text that isn't a value, or is outside `min`, `max` or the disabled days, stays as typed, marked `aria-invalid` with the reason in its `title` and a live region: "Enter a date like DD/MM/YYYY", "Pick a date on or after 5 Mar 2026". `bindError` puts the reason in state for a `ui-field`'s `error`. Escape puts back the value as it was, an empty field clears it, and Down opens the calendar. In a `ui-field`, the label names the text field. `input` takes one value or a range, not `multiple`.

```yaml
- type: ui-field
  props: { label: Start date, error: "{{ startError }}" }
  children:
    - { type: ui-date-picker, props: { bind: start, bindError: startError, input: true, min: today } }
```

## Data table

`ui-data-table` shows an array of rows with the controls of shadcn/ui's data table: sorting, column filters, search, paging, row selection with bulk actions, expandable detail rows, a View menu that shows and hides columns, and summary rows. It also pages, loads more or scrolls infinitely, loads rows from an API in batches, keeps long tables fast by drawing only the rows in view, nests rows inside rows (loading children when they're opened) and pins columns while the rest scroll sideways. Its behaviour (about 13 KB gzipped) is added only to pages that use it. The blocks [`data-table-01`](../../examples/blocks/data-table-01.yaml) (tasks), [`data-table-02`](../../examples/blocks/data-table-02.yaml) (orders wired to charts), [`data-table-03`](../../examples/blocks/data-table-03.yaml) (inventory from plain column objects), [`data-table-04`](../../examples/blocks/data-table-04.yaml) (5,000 rows from an API: infinite scroll, Load more and server pages) and [`data-table-05`](../../examples/blocks/data-table-05.yaml) (a budget tree with pinned columns, and an org chart from flat rows) show each feature.

```yaml
- type: ui-data-table
  props: { rows: "{{ orders }}", search: true, sort: -date, selectable: true, bindSelection: picked, pageSize: 10 }
  children:
    - { type: ui-data-table-column, props: { field: id, label: Order } }
    - { type: ui-data-table-column, props: { field: date, format: date, filter: date } }
    - { type: ui-data-table-column, props: { field: amount, format: currency, currency: EUR, filter: range, summary: sum } }
    - type: ui-data-table-column
      props: { field: status, filter: select, cell: badge, tones: { Paid: success, Failed: destructive } }
    - type: ui-data-table-column
      props: { id: actions, label: "" }
      children:                                  # a cell template: the row is `row`
        - { type: ui-button, props: { label: Open, size: sm }, on: { click: { action: call, name: open, args: { id: "{{ row.id }}" } } } }
    - type: ui-data-table-bulk                   # shown while rows are selected
      children:
        - { type: ui-button, props: { label: Delete, size: sm, variant: destructive }, on: { click: { action: call, name: delete, args: { ids: "{{ picked }}" } } } }
```

**Children.** `ui-data-table-column` is a column; its children, if any, draw each cell with the row in scope as `row` (and its position in `rows` as `rowIndex`). `ui-data-table-toolbar` adds controls to the toolbar, before the View menu. `ui-data-table-bulk` holds actions shown while rows are selected, next to "N selected", "Select all N" and "Clear selection". `ui-data-table-detail` is drawn under a row when its arrow is opened. Tables without cell templates can list their columns as objects in `columns` instead.

| table prop | |
|---|---|
| `rows` | Objects, from data or state. Rows from state are rebuilt when the array changes, and keep the table's sort, filters, page and selection. |
| `url`, `params` | Rows from an API instead of `rows`; see [Paging and loading](#paging-and-loading). |
| `paging` | `pages` (default), `more`, `scroll` or `none`; see [Paging and loading](#paging-and-loading). |
| `virtual`, `rowHeight` | Draw only the rows in view; see [Paging and loading](#paging-and-loading). |
| `childrenField`, `parentField`, `childrenUrl`, `expanded` | Rows inside rows; see [Nested rows](#nested-rows). |
| `columns` | Column objects with the column props below, for columns without templates. |
| `rowKey` | The field that identifies a row in the selection and the open details (default `id`; its position when a row has none). |
| `selectable` | A checkbox per row and one in the header for the rows on the page. Shift-click selects a run of rows. Implied by `ui-data-table-bulk`. |
| `search` | A search box: `true`, or its placeholder. Every word must appear in one of `searchFields` (default: the columns' fields), ignoring case and accents. |
| `sort` | Where it starts: `field`, `-field` for descending, or a list. Clicking a header sorts ascending, then descending, then not at all; Shift-click adds the column to the sort. Blank values go last either way. |
| `filter` | Conditions from outside the table, e.g. `{ region: "{{ region }}" }` from a chart's `bind` or a toggle. |
| `pageSize`, `pageSizes` | Rows per page, or per batch with `more` and `scroll` (default 10; `0` shows every row), and the choices offered with `pages` (default `[10, 20, 50, 100]`; `[]` hides the menu). |
| `columnsMenu` | The View menu (default true). |
| `groupBy`, `bindGroupBy` | Group the rows by a field, under header rows with counts and subtotals; see [Grouping, column layout and export](#grouping-column-layout-and-export). |
| `resizable`, `reorderable`, `bindColumns` | Columns resized by their header's edge and moved by dragging their header, the layout kept in state. |
| `export` | An Export button: the rows that pass, as CSV, or as Excel with a name ending in `.xlsx` (`true`, or the file's name). |
| `height` | Most the table grows to; past it the rows scroll under a header that stays put. |
| `striped`, `loading`, `title`, `emptyText`, `locale` | Shade every other row; dim it while `loading` is true; the table's accessible name; the empty message (default `No results.`); the locale for formatted values. |
| `bindSelection`, `bindSort`, `bindFilters`, `bindSearch`, `bindPage` | State keys for the selection (a list of row keys), the sort, the column filters, the search text and the page, both ways. |
| `bindView` | A state key that gets the rows that pass the filters and search, sorted, every page. Give it to a chart's `data` to chart what the table shows. |
| `bindRow` | A state key that gets the row last clicked, or opened with Enter. Rows then show a pointer. |

| column prop | |
|---|---|
| `field` | The row field shown, sorted, filtered and searched: `amount`, or a path such as `customer.name`. Without a cell template, the cell is the value. |
| `label`, `width`, `align` | The header, the column's width, and `left`, `center` or `right` (default: right for number formats). |
| `sortable`, `sortField` | Whether the header sorts (default: when there's a field or `sortField`), and a field to sort by instead, such as a rank behind a label. |
| `filter` | A filter button in the toolbar: `select` (checkboxes with counts; choices from `options` or the column's values; list-valued fields such as tags match on any value), `text` (contains), `range` (min and max) or `date` (a range calendar with presets: the column's `presets`, as `ui-calendar`'s, default last 7 and 30 days, this and last month, this and last quarter and this year; `fiscalStart` sets the financial year they follow). |
| `format`, `currency`, `decimals` | `number`, `currency`, `percent` (0.25 is 25%), `date` or `datetime`, formatted in the browser with `Intl` in `locale`. Plain dates (`2026-03-01`) show the same day in every time zone. |
| `cell`, `tones` | Draw the value as a `badge` (with a tone per value: default, secondary, outline, success, warning, destructive, info), a `progress` bar (0–100) or a `sparkline` of a list of numbers (needs the chart kit). |
| `summary` | A footer cell: `sum`, `avg`, `min`, `max` or `count` of the rows that pass the filters. |
| `pin` | `left` or `right`: the column stays in view while the rest scroll sideways. The checkbox and arrow columns stay with the left ones, and a shadow marks where pinned columns end. |
| `tree` | The column that shows nesting, indented with the arrow that opens a row (default: the first column, in a table with nested rows). |
| `group` | A header over this column and its neighbours with the same `group`. |
| `groupable` | Offered in the View menu's Group by. |
| `resizable` | `false` keeps this column's width when the table is `resizable`. |
| `hideable`, `hidden` | Listed in the View menu (default true), and hidden to start with. |
| `id` | The column's key when it has no field. |

### Paging and loading

`paging` picks how rows past the first `pageSize` are reached:

| `paging` | |
|---|---|
| `pages` | The default: a pager with rows per page, and Page Up / Page Down on a focused row. |
| `more` | A Load more button under the rows adds the next batch. |
| `scroll` | Infinite scroll: the next batch comes as the end of the rows nears the bottom of the table (with `height`) or of the window. |
| `none` | Every row at once. Also what `pageSize: 0` means. |

With `more` and `scroll`, new filters, a search or a sort start again from the first batch, at the top, and the footer says how many rows show (`Showing 40 of 96`).

**From an API.** Give `url` instead of `rows`, and the table asks for rows a batch at a time: `GET <url>?offset=40&limit=20&sort=-date,id&q=text&filters={…}`, answering `{ rows, total }` or a plain array. `sort` lists fields, `-` for descending; `filters` is the column filters and `filter` as JSON, in the [filter conditions](../irender-chart-kit/README.md#filters-and-grouping) below. `params` adds query parameters, and may use state, so a control can narrow what the server sends. The server sorts, searches and filters, and any change asks again from the first batch, after a pause while typing; answers to an older query are dropped.
- With `pages`, each page is one request, and the total gives the page count. Without a total, the pager goes on until a short page comes back.
- With `more` and `scroll`, batches add up. Without a total, a short batch is the last.
- A failed request shows a Retry button.
- Facets and totals: a query's first batch also asks for the select filters' fields as `facets=status,region` and the footer's summaries as `summary=net:sum,qty:avg`. The API may answer `facets: { status: [{ value: open, count: 41 }, …] }` (or a plain list of values, or `{ open: 41, … }`) and `summary: { "net:sum": 125000, "qty:avg": 3.2 }` beside `rows`, over every row that matches; then the filters offer every value with its count, and the summaries and a `count` summary (from `total`) cover all those rows. Counts are best taken with every filter but the field's own, as a local table does. Without them, the choices come from `options` or the rows loaded so far, and the summaries cover the rows loaded so far.
- `bindView` holds the rows loaded so far.
- `url` must be a path or an http(s) URL, as `ui-combobox`'s `searchUrl`.

**Virtual.** With `virtual`, only the rows in view (and a few either side) are in the page, with space above and below standing in for the rest, so a table of thousands of rows scrolls smoothly. It scrolls inside `height` (default 480). Rows of different heights work: the table measures the rows it draws, starting from `rowHeight` (default 41). Arrow keys, Home and End reach rows that aren't drawn yet. It works with every `paging`, rows from `rows` or `url`, and nested rows; with `scroll` it's the usual infinite list. A focused row keeps its focus while it's scrolled out of view: the scroll area holds it until the row is drawn again, and a key pressed meanwhile (Down, Space, Enter, …) brings the row back and acts from it.

### Nested rows

Rows can hold rows. The tree column (`tree: true`, or the first) indents each row by its depth, behind an arrow that opens it, and the table is a WAI-ARIA tree grid (`aria-level`, `aria-expanded`).
- **Children in the row:** a `children` array (another field with `childrenField`). Rows from state that start out empty need `childrenField` or a `tree` column to say they nest.
- **Flat rows:** `parentField: parentId` builds the tree from rows that name their parent's key. A row whose parent isn't there is a top row.
- **Loaded when opened:** a row with `children: true` (or `hasChildren: true`) gets its children from `childrenUrl` the first time it's opened: `GET <childrenUrl>?parent=<key>&sort=…`, answering an array or `{ rows }`. The arrow dims while they load; if they don't, the row closes again and opening it retries.

`expanded: true` opens every row to start with. Siblings sort among themselves under their parent. A search or filter shows the rows that match with the rows above them, opened. Summaries add up the top rows only, so nothing counts twice. Give rows a `rowKey`: it identifies them when they open, load and are selected.

On a focused row, Right opens it, then steps to its first child; Left closes it, then steps to its parent; `*` opens every row beside it. The other keys work as in any table. The View menu has Expand all and Collapse all; rows whose children load from `childrenUrl` stay closed under Expand all until they're opened, so it doesn't send a request per row.

**Selection.** With `selectable`, picking a row picks every row under it, and a row is picked once all the rows under it are; one with only some of them picked shows a part-filled box. Rows loaded later under a picked row come in picked. `selectChildren: false` picks rows one at a time instead. The footer counts every loaded row, open or not.

### Grouping, column layout and export

The [`data-table-07`](../../examples/blocks/data-table-07.yaml) block uses all of these.

**Grouped rows.** `groupBy: region` (a field; it may read state) puts the rows under a header row per value, in the value's order (descending when the table is sorted by that field descending; rows without a value last, as "(none)"). The header shows the value, how many rows it has, and a subtotal under each column with a `summary`; its button folds and unfolds the group. Within a group the rows keep the table's sort. Columns with `groupable: true` are offered in the View menu's Group by (with None), which writes `bindGroupBy`. Paging counts the header rows as lines. Grouping doesn't work with `virtual` or nested rows.

**Grouped rows from an API.** With `url`, grouping asks the API for the groups: `GET <url>?groupBy=region&…` (with the sort, search, filters and `params`, and `offset` and `limit` when paging) answers a row per group, `{ region: North, count: 12, net: 1250 }`: the value in the field, a `count`, and subtotals in the columns' own fields. Each shows as a row with its value and count, and opening it (a click, Enter, or Right) loads its rows from `<url>?groupBy=region&group=North&…`, the same query for that group, answering `{ rows }` or a list. Groups start closed; open ones load their rows again when the query changes. Group rows aren't selected, written to `bindView` or exported, and the footer's summaries add up their subtotals.

A group's rows come a batch at a time, `pageSize` rows each (with `offset` and `limit`): the first when it's opened, and the next from a "Load 20 more of 812" row after them, clicked or with Enter, until all have loaded. How many the group has comes from the answer's `total`, else the group's `count`; without either, the row shows until a batch comes back short. A batch that fails can be tried again from the same row. Loaded rows stay while the group is closed and opened again, and go when the query changes. With `paging: none` a group's rows come in one request.

**Grouped headers.** A column's `group` puts a header over it and the columns next to it with the same `group` (a `colgroup`), in a row above the column headers; it follows columns as they move and hide.

**Resizing and moving columns.** With `resizable`, each header's right edge is a separator: drag it, or focus it and use Left and Right (16 px, Shift for 4); a double click puts the column back to its own width. A column's `resizable: false` opts out. With `reorderable`, drag a header onto another to move the column before or after it (a press that doesn't move still sorts), or press Alt with Left or Right on its header; pinned columns stay put. `bindColumns` keeps the layout as `{ order, widths, hidden }`, and setting it lays the columns out that way (hidden columns from the View menu are in it too). A move is announced, and fires `columns`.

**Export.** `export: true` (or a file name) adds an Export button that downloads the rows that pass the filters and the search, all pages, as CSV: the shown columns that have a `field`, in their order, under their labels, with raw values (numbers unformatted, so a spreadsheet can add them up). The file starts with a byte-order mark so spreadsheet apps read it as UTF-8, and a text that starts like a formula (`=`, `+`, `-`, `@`) is kept as text. It fires `export` with `{ csv, rows, file }`. With a name ending in `.xlsx` it writes an Excel file instead, numbers in their column's format and dates as dates. A remote table exports the rows it has loaded.

### Editable cells

A column with `edit` (`text`, `multiline`, `number`, `date`, `select` or `checkbox`) is edited in place, and the table works as a spreadsheet does. The rows must be in state, as `rows: "{{ products }}"`: that's where the edits go. (With `url`, the rows loaded change, and the page saves them from the events.)

```yaml
- type: ui-data-table
  props: { rows: "{{ products }}", addRow: true, newRow: { status: Draft }, deletable: true, bindChanges: changes, bindErrors: wrong }
  children:
    - { type: ui-data-table-column, props: { field: sku, edit: text, required: true, unique: true, pattern: "[A-Z]{3}-[0-9]+" } }
    - { type: ui-data-table-column, props: { field: price, edit: number, min: 1, format: currency, currency: EUR } }
    - { type: ui-data-table-column, props: { field: status, edit: select, options: [Draft, Live] } }
```

- **Keys:** the arrow keys move between cells (with Shift they pick a range, with Ctrl or ⌘ they go to the end), Tab goes to the next editable one, Enter, F2, a double click or typing edits, Enter and Tab keep the edit and Escape leaves it. Delete clears, Space ticks a checkbox, Ctrl+D (⌘D) fills down, Ctrl+Z and Ctrl+Y undo and redo.
- **The fill handle:** the square on the picked cells' corner, dragged down or up (mouse, pen or finger), repeats them over the rows it reaches, in order away from them, as one step to undo; the rows filled are then picked.
- **Copy and paste** go to and from spreadsheets. Pasted text is read as the column's: `1,234`, `5%`, `(120)` and the locale's own numbers, dates as the locale writes them, a choice by its label. What can't be read is kept as it is and marked. Rows pasted past the end are added, with `addRow`.
- **Checks:** `required`, `min`, `max`, `pattern` and `unique` on a column, with `message` in place of the kit's words, and `errors` from the page or server (`{ rowKey: { field: message } }`). Wrong cells are marked and say why, the Problems button shows only their rows, and `bindErrors` gets `[{ key, field, message }]`.
- **Changes:** `bindChanges` gets `{ rowKey: { field: value } }` (null for a deleted row) and changed cells get a mark in their corner; set it to `{}` once they're saved. Events: `celledit` (`{ key, row, field, value, previous }`), `rowadd`, `rowdelete`, and `rowschange` for an undo or a large paste.
- `editable` says which rows may be edited, e.g. `{{ row.status != 'Closed' }}`. `addRow` is the button under the rows, `deletable` adds Delete to the bulk bar, and `export: prices.xlsx` writes Excel.
- Editable tables can't have nested rows, or be grouped.

### Saved views

A view is a name for how the table is set up: its sort, filters, search, columns (order, widths and the hidden ones), grouping and rows per page. With `views` or `bindViews`, a Views menu starts the toolbar. The [`data-table-01`](../../examples/blocks/data-table-01.yaml) block has two ready-made views and saves your own.

```yaml
- type: ui-data-table
  props:
    rows: "{{ tasks }}"
    bindViews: myViews           # the views people save: [{ id, name, sort, filters, search, columns, groupBy, pageSize }]
    bindCurrentView: view        # the id of the view shown
    defaultView: open            # opened with when `view` is empty
    views:                       # ready-made, can't be changed or deleted
      - { id: open, name: Open work, filters: { status: [Todo, In Progress] }, sort: -priority }
      - { id: lean, name: Lean, columns: { hidden: [label, assignee] } }
```

- **The menu** lists Default view (the table as its props set it up), the ready-made views and the saved ones, with the one shown ticked. Picking one shows it from the first page. A saved view has Rename and Delete buttons; renaming and Save as new view… ask for the name in the menu (Enter saves, Escape cancels).
- **Changes.** Once the table is changed from the view shown, the button reads `Open work (edited)`, and the menu offers Undo changes and, for a saved view, Save changes. A view leaves out what it doesn't set: the parts it lacks are the table's own.
- **State.** `bindViews` gets the saved views after each change, and `bindCurrentView` the id of the one shown (null for the default), so a page can keep both, per person, anywhere. Writing an id to `bindCurrentView` shows that view, and the table opens with it. Changes not saved to a view aren't kept by the view; bind `bindSort`, `bindFilters` and the rest too to keep them. Events: `view` with `{ id, view }`, and `views` with `{ views }`.

### Filters

**Filters** use the chart kit's [`filter` conditions](../irender-chart-kit/README.md#filters-and-grouping): `bindFilters` holds `{ status: [Paid, Pending], amount: { from: 100 }, date: { to: "2026-03-31" } }`, and the same object filters a chart. Two differences: a list-valued field matches when any of its values does, and a plain date as `to` takes in the whole day.

### Charts

Charts and the table work together in three ways, all in the [`data-table-02`](../../examples/blocks/data-table-02.yaml) block:
- In cells: `cell: sparkline`, or any `ui-chart` or `ui-sparkline` in a cell template or in `ui-data-table-detail`. Charts in hidden rows start when they're first shown.
- From the table: `bindView` gives charts the rows the table shows, so its filters and search redraw them.
- Into the table: a chart's `bind` (the clicked bar) in the table's `filter`.

### Keyboard, events and how it works

**Keyboard.** With `selectable`, `bindRow`, a detail row or nested rows, the rows are one stop in the tab order:
- Up and Down move between rows, Home and End jump, and Page Up and Page Down change page.
- Space selects the focused row. Enter opens it (`bindRow`) or its details, and Right and Left open and close its details (in a table with nested rows, the row itself).
- Escape in the search box clears it.
- The headers' sort buttons, the checkboxes and the toolbar are ordinary buttons and inputs, and `aria-sort` marks the sorted column.

**Events.** The table fires bubbling `rowclick` (`detail.row`) and `select` (`detail.selection`) events for `on:` actions.

**How it works.** The server renders the rows, in the starting order and with the first page shown, so the table reads without JavaScript. It sorts text as the browser will (numbers by value, so Item 9 comes before Item 10; case and the accents of Latin letters aside), so rows stay put when the script takes over. The browser then sorts, filters and pages by moving and hiding the rendered rows rather than drawing them again, so what's inside a cell keeps its state. Virtual, remote and nested tables instead hand the runtime the rows to show, as a list keyed by `rowKey` (`api.list` in the [runtime](../irender-html/src/runtime.js)); a row that stays keeps its nodes, charts included.

## Filters panel

`ui-filters` filters a list from a side panel. The button opens a panel listing the fields, each with its operators. The applied filters show beside the button as chips: clicking a chip opens its field, and its ✕ removes it. Each `ui-filter` child is one field.

```yaml
- type: ui-filters
  props: { bind: filters, rows: "{{ orders }}", currency: INR }
  children:
    - { type: ui-filter, props: { field: status, label: Status, section: Order } }
    - { type: ui-filter, props: { field: customer, label: Customer, section: Order } }
    - { type: ui-filter, props: { field: amount, label: Amount, type: currency } }
    - { type: ui-filter, props: { field: date, label: Date, type: date, operators: [between, before, after] } }
    - { type: ui-filter, props: { field: note, label: Note, type: text } }
- type: ui-data-table
  props: { rows: "{{ orders }}", filter: "{{ filters }}" }
```

| Type | Operators | Writes |
|---|---|---|
| `list` (default) | is any of, is none of | `[Paid, Pending]`, `{ not: [Paid] }` |
| `number`, `currency` | equals, more than, less than, between | `500`, `{ gt: 500 }`, `{ lt: 500 }`, `{ from: 100, to: 500 }` (either end may be left out) |
| `date` | between, on, before, after | `{ from, to }`, `{ from: d, to: d }`, `{ lt: d }`, `{ gt: d }` |
| `text` | contains, is | `{ contains: rush }`, `"Rush"` |
| every type | is empty, is not empty | `{ empty: true }`, `{ empty: false }` (an unassigned ticket, say) |

**What it writes.** `bind` holds the applied filters as `{ field: condition }`, in the chart kit's [`filter` format](../irender-chart-kit/README.md#filters-and-grouping). Give the same key to a `ui-data-table`'s `filter`, a `ui-chart`'s `filter`, or a remote table, which sends it to its API as `filters`. Set the key yourself to apply filters from elsewhere; the chips follow.

**List values.** A list filter offers its `options` (text, or `{ value, label }`, possibly from state). Without them it offers the field's values in the filters' `rows`, each with how many rows have it. A list longer than five gets a search box. Values already picked stay listed even when they're no longer among the options.

**Applying.** The panel edits a draft: Apply writes it, and Cancel, Escape or a click outside drops it. A `between` whose ends are the wrong way round shows an error and can't be applied. With `live`, each change is written at once and the panel has a Done button. Clear all empties the draft, or the filters with `live`; the Clear all beside the chips removes every filter at once.

**Finding a field.** Past eight fields (or with `searchFields: true`; `false` turns it off), a box over the list finds one: every word typed must be in the field's name or section, and the rest hide, section headings with them. Enter opens the first match, Down goes into the list, Up from its top comes back, and Escape clears the box before it closes the panel.

**Keyboard.** The fields are tabs: Up and Down (and Home, End) move between them. Opening the panel focuses the field's first control, and closing it returns focus to the button. The button's label says how many filters are applied.

**Dates and amounts.** A date filter picks on the kit's calendar: a range for Between, one day for On, Before and After. Amounts (`number` and `currency`) are grouped in the `locale`'s way as they're typed (`1,25,000`), with the currency's symbol before them, and the filter gets a plain number.

**Date presets.** A date filter offers quick ranges under its calendar: Today, Yesterday, Last 7 days, Last 30 days, This month, Last month, This quarter and This year, counted from `today` (default: the real one). A preset sets Between with its dates and shows as pressed while they're the ones set. `presets: false` hides them, and a list (`[last7, thisMonth]`) picks some, from those and `lastQuarter`, `lastYear`, `yearToDate` and `quarters` (the quarter today is in and the three before it, named `Q3 2026`). The quarter and year presets follow the panel's `fiscalStart`: with `fiscalStart: 4`, This year is April to March and the quarters read `Q3 FY2026–27`. They show only where the filter offers Between.

**Saved views.** With `bindViews`, a Views menu beside the button lists the views in that key (`[{ name, filters }]`), the one matching the filters now marked: a click applies one, ✕ deletes it, and the name box saves the filters as a view (a view of the same name is replaced). Escape or a click outside closes the menu. Keep the key in state your server saves, or in a host's storage, so views last.

**Props.** `ui-filters`: `bind` (required), `rows`, `title` (the panel's heading, default `Filters`), `label` (the button's text, default `Filters`), `live`, `chips` (default true), `currency` (default `USD`), `locale`, `bindViews`, `today`, `fiscalStart` (the month a financial year starts, 1–12), `id`; parts `button` and `panel`. `ui-filter`: `field` (required), `label`, `type`, `options`, `operators` (a subset of the type's, in the order to show them: `any`, `none`, `eq`, `gt`, `lt`, `between`, `on`, `before`, `after`, `contains`, `is`, `empty`, `set`), `section` (a heading to group fields under), `unit` (after a number, e.g. `kg`), `currency` and `presets` (a date filter's quick ranges).

## Status and activity

**`ui-status-badge`** shows a status in its colour. Common statuses know their tone, ignoring case, spaces, `-` and `_`: Done, Complete, Paid, Approved, Active, Resolved, Delivered and Published are `success`; Open, New, To do and Scheduled are `info`; In progress is `primary`; In review, Pending, Waiting, On hold and Partial are `warning`; Overdue, Blocked, Failed, Rejected, Cancelled, Unpaid and Error are `danger`; Draft, Closed, Archived and Inactive are `neutral`, as is anything unknown. `tones` adds or overrides (`{ Shipped: success }`), and `labels` changes the text shown (`{ in_progress: In progress }`). Tones take the theme's `color.success`, `color.warning`, `color.info` and `color.destructive` when it has them. A status from state changes colour as it changes.

**`ui-gauge`** shows a figure on a dial from `min` (default 0) to `max` (default 100): a 240° arc, or a semicircle with `shape: half`, the figure in the middle with its `unit`, and `min` and `max` at the ends (`scale: false` hides them). `bands` colour stretches of the dial, `[{ to: 60, tone: danger, label: Poor }, { to: 85, tone: warning, label: Fair }, { tone: success, label: Good }]`, the last running to `max`, and the value's arc takes the tone of the band it's in (`tone` sets one for good). `target` marks a value across the dial, and `needle: true` adds a needle. Like the ring, it's drawn on the server and follows state without a script: the arc, needle, target and tone move together, easing unless reduced motion is asked for. A value past either end stops the dial there and still shows itself. It's a `meter` for screen readers, read as the figure, its unit and its band's label ("82.6 %, Getting full"); `label` (default `caption`) names it. `text` replaces the figure, for one that's rounded or formatted. The chart kit's `gauge` type is the full one, with ECharts: several pointers, ticks and animation; this one is for a figure in a tile.

```yaml
- type: ui-gauge
  props:
    value: "{{ line.oee }}"
    unit: "%"
    target: 85
    caption: Overall equipment effectiveness
    bands: [{ to: 60, tone: danger, label: Poor }, { to: 85, tone: warning, label: Fair }, { tone: success, label: Good }]
- { type: ui-gauge, props: { value: "{{ oil }}", min: 20, max: 120, unit: °C, shape: half, needle: true } }
```

**`ui-countdown`** shows the time left to `to`: a ticket's SLA, an offer, a launch. `to` (and `from`) is a date and time, milliseconds, or a time from when the page opens (`+15m`, `-2h 30m`), and may come from state. Under `warn` it turns to the warning tone and under `danger` to the danger one (seconds, or `90s`, `30m`, `4h`, `2d`); past the deadline it counts up as overdue, or with `overdue: false` shows `endText` (default `Time's up`). `format: units` (default) shows the two largest units that matter (`2d 4h`, `13m 05s`), `clock` a clock (`2:04:13`, `3d 02:04:13`); `variant: tiles` shows days, hours, minutes and seconds in boxes, with the deadline under them. With `from`, a bar shows how much of the time is used. It's a `timer` for screen readers: the text isn't read out every second, and each status is said once as it starts ("Reply due: 59 minutes left", "Overdue"). `bindLeft` gets the seconds left every second (below zero once overdue), `bindStatus` gets `ok`, `warning`, `danger` or `over`, and `on: { end: … }` runs when it reaches zero on the page (not for a deadline already past when it opens). While `paused` is true (`{{ ticket.status == 'waiting' }}`, say) the time left stands still and it says Paused; resumed, it goes on from there, the deadline moved out by the pause. That's kept on the page only: to keep it, set a new `to` (which replaces the moved one) or the time already paused in it, from the server. An SLA that only runs in working time takes `workdays` (`true` for Monday to Friday, or the weekdays that work, 0 being Sunday), `hours` (`09:00-17:30`, local time; alone, every day works) and `holidays` (dates): the time left, `warn` and `danger`, the bar and overdue all count working time only, and outside it the countdown stands still and says Out of hours. `to` is still the moment it's due, worked out by the server; hours that run past midnight aren't supported.

```yaml
- { type: ui-countdown, props: { to: "{{ ticket.due }}", from: "{{ ticket.opened }}", warn: 1h, danger: 15m, label: First reply due, bindStatus: slaStatus } }
- { type: ui-countdown, props: { to: "2026-12-24T18:00", variant: tiles, label: Doors open } }
- type: ui-countdown
  props: { to: +15m, overdue: false, endText: The offer has ended }
  on: { end: { action: set_state, key: offerOpen, value: false } }
```

**`ui-compare-slider`** puts two pictures of the same thing one over the other, `before` and `after`: a repair, an edit, a renovation, a season. A divider shows the first on one side and the second on the other; drag anywhere on the picture to move it (mouse, finger or pen), or focus it: the arrow keys move it a step, Page Up and Down ten, Home and End to either side. It's a range input underneath, so screen readers hear "Before 40%, after 60%"; give each picture a `beforeAlt` and `afterAlt`. `beforeLabel` and `afterLabel` (default `Before` and `After`, empty for none) sit on their sides and fade as the divider covers them. `position` (0–100, default 50) is where it starts, or `bind` keeps it in state both ways, so buttons can jump to either side. `orientation: vertical` runs it up and down, `hover` lets a mouse move it without pressing, and `ratio` (`16/9`, `1`) crops both pictures to that shape; without it the first picture sets the size and the second covers it, so they should match. Right to left, the first picture is on the right.

```yaml
- type: ui-compare-slider
  props: { before: /photos/wall-12-nov.jpg, after: /photos/wall-21-nov.jpg, beforeAlt: The damp wall, afterAlt: The wall repaired, bind: split, caption: Living room, north wall }
- { type: ui-compare-slider, props: { before: a.jpg, after: b.jpg, orientation: vertical, ratio: 4/3, hover: true } }
```

**`ui-iframe`** shows another page inside this one: a map, a video, a form, a report from another tool, or one of your own pages. It has no children and goes anywhere a component can (a card, a tab, a dialog, a grid cell). `src` may come from state; the frame follows it, and an unsafe address (`javascript:`, `data:`) is dropped. Give it a `title` for screen readers (default `Embedded page`). It's `height` px tall (a number or a CSS length, default 400) and as wide as its container, or with `ratio` (`"16 / 9"`, `1.5`) as tall as its width makes it. `sandbox` lists what the page may do, from `scripts`, `same-origin`, `forms`, `popups`, `popups-to-escape-sandbox`, `modals`, `downloads`, `presentation`, `pointer-lock`, `orientation-lock`, `top-navigation-by-user-activation` and `storage-access-by-user-activation`; the default, `[scripts, same-origin, forms, popups, popups-to-escape-sandbox]`, lets most embeds work without letting them move this page or open dialogs over it. `[]` allows nothing (shown, no scripts), and `false` leaves the sandbox off. A page from your own site with both `scripts` and `same-origin` can lift its own sandbox, so give your own pages less, or none, when that matters. `allow` lists browser features it may use (`[fullscreen, clipboard-write, camera]`, default `[fullscreen]`). It loads as it nears the screen (`loading: eager` for at once), and `bordered: false` drops the border. The untrusted policy refuses it, as it refuses a plain `iframe`; `attrs` (for instance `srcdoc` or `name`) go on the `<iframe>`.

```yaml
- { type: ui-iframe, props: { src: https://example.com/report, title: Monthly report, height: 480 } }
- { type: ui-iframe, props: { src: "{{ videoUrl }}", title: Product tour, ratio: "16 / 9", allow: [fullscreen, picture-in-picture] } }
- { type: ui-iframe, props: { src: /widgets/status.html, title: Status, sandbox: [], bordered: false } }
```

**`ui-presence`** shows who's here now: the people looking at a page, a document or a board. Each of `people` (`{ id, name, avatar, initials, status, color, where }`) is an avatar (their picture, else their initials, else their name's first letter) with a dot for their `status` (active, idle, busy, away; offline fades them) and a ring in their `color` (1–8, the theme's chart colours; by default by place). The first `max` overlap, then `+3` for the rest. The stack is one button, named "People here: 6", that opens everyone in a list, each with their status in words and `where` they are ("Active · Editing Budget"), `(you)` after the one whose `id` is `me`. From state, it follows as people come and go, with no script; `announce` adds one that says who joins and who leaves, for screen readers.

```yaml
- { type: ui-presence, props: { people: "{{ viewers }}", me: "{{ user.id }}", max: 3, announce: true } }
# viewers: [{ id: u1, name: Asha Rao, status: active, where: Editing Budget }, { id: u2, name: Ravi Kumar, status: idle, avatar: /a/ravi.jpg }]
```

**`ui-virtual-list`** shows a long list, thousands of items or a hundred thousand, keeping only those in view (and a few either side) in the page, with space above and below standing in for the rest, so it scrolls smoothly. Its children are each item, with `item` and `index` in scope; items may differ in height, as they're measured while they're drawn (`itemHeight` is where it starts), and they're placed by that average height. It scrolls inside `height` (default 400) and takes focus, so the keys scroll it; each item is a `listitem` with its place in the whole list (`aria-posinset`, `aria-setsize`). `bindRange` gets the places in view, `{ from, to, total }`, for "Showing 120–140 of 5,000"; `end` fires when the last `endThreshold` items come into view, once for each length of list, for the page to load more. `itemKey` (default `id`) keeps an item's nodes as the list changes. The server writes the first 30 items, for a page read before the script runs. For rows with columns, sorting and filters, the data table's `virtual` does the same.

```yaml
- type: ui-virtual-list
  props: { items: "{{ rows }}", height: 480, itemHeight: 48, bindRange: showing }
  on: { end: { action: submit, endpoint: /api/rows, method: GET, into: more } }
  children:
    - { type: text, props: { value: "{{ item.name }}" } }
```

**`ui-file-browser`** shows folders and files one folder at a time, as a grid (with `thumb` pictures) or a list (name, modified, size), with a breadcrumb back up. `bind` holds them as a flat list, `{ id, name, parent, folder, size, modified, thumb, url }`, `parent` being the folder's `id` (none: the top) and `folder: true` making one; every change goes back to it. The bar searches the open folder, sorts (name, newest, largest, kind; folders first), switches the view, adds a folder, moves and deletes. Selecting: a click, ⌘/Ctrl or Shift with it for several, or the keys (arrows, by rows in the grid; Shift and ⌘/Ctrl too; Space; ⌘/Ctrl+A). Enter or a double click opens a folder, or a file into `bindOpen` with an `open` event; Backspace or Alt+↑ goes up. F2 renames in place (a name already in the folder is refused), Delete deletes with an Undo at the foot for ten seconds, and items move by dragging onto a folder or the breadcrumb (by touch, hold first), or from the Move to menu, which never offers a folder inside what's moving. Names that clash on a move get `(2)`. `bindFolder` keeps the open folder in state, both ways, and `bindSelection` gets the selection. `editable: false` leaves browsing and opening. Uploading is the page's: put a `ui-file-upload` beside it and add what it uploads to the list.

```yaml
- { type: ui-file-browser, props: { bind: files, bindFolder: folder, bindOpen: opened, height: 460 } }
# files: [{ id: f1, name: Contracts, folder: true }, { id: a1, name: NDA.pdf, parent: f1, size: 52000, modified: 2026-10-12T15:30 }]
```

**`ui-whiteboard`** is a board without edges, drawn as SVG. `bind` holds its items in the board's own units, `{ id, type, x, y, w, h, points, text, color }`: `pen` (a stroke, `points` from `x`, `y`), `note` (a sticky note), `rect`, `ellipse`, `arrow` (two `points`) and `text`. The tools, each with a key: Select (V: a click, Shift for more, or a box; drag to move; the corner to resize), Hand (H, or hold Space, the middle button or two fingers), Pen (P), Sticky note (N), Rectangle (R), Ellipse (O), Arrow (A) and Text (T); a shape is dragged out, or a click gives it its usual size, and the tool goes back to Select. Double-click (or Enter on) a note or text to write in it. A colour picked colours what's selected. ⌘/Ctrl+Z undoes and Shift with it redoes, Delete deletes, ⌘/Ctrl+D duplicates, the arrows nudge (Shift: ten); with something selected, Tab goes to the next item, and Escape lets go so Tab leaves the board. The wheel scrolls, with ⌘/Ctrl (or a pinch) it zooms at the pointer; + and − zoom, 0 fits everything. Export downloads it as an SVG file. `bindView` keeps the view, `{ x, y, zoom }`, both ways; `tools` and `colors` narrow the toolbar; `readOnly` leaves looking and moving around. The board is one stop for the keyboard, named and described as a whiteboard; what's selected is said.

```yaml
- { type: ui-whiteboard, props: { bind: board, bindView: view, height: 600 } }
# board: [{ id: n1, type: note, x: 20, y: 20, w: 170, h: 120, text: Ideas }, { id: a1, type: arrow, x: 200, y: 80, points: [[0, 0], [80, 0]] }]
```

**`ui-mindmap`** is a mind map: the central topic in the middle, its first branches shared between both sides by size (`layout: right` puts them all on the right), each in a colour of its own (`colors`, or a topic's own `color`), joined by curves. `bind` holds the map as a tree, `{ id, label, children, collapsed, color }`, the shape `ui-tree` reads, so the same state can be its outline too. Pick a topic (a click, or the keys) and: the arrows go to its parent (towards the middle), its first child (away) and its siblings (up and down), Home to the middle; Tab adds a topic inside it and Enter one beside it, each ready to write in; typing writes over it (F2 or a double click edits it); Space folds it (a folded topic shows how many are inside); Delete deletes it and what's inside (never the middle); ⌘/Ctrl+Z undoes, with Shift redoes. Drag a topic onto another to move it there (not into itself); drag the space to pan; ⌘/Ctrl and the wheel, or a pinch, zoom; Fit shows it all, and a map too big to start with is fitted. Export saves it as an SVG file. It's a `tree` for screen readers, each topic a `treeitem` with its level and whether it's folded; `bindSelected` keeps the topic picked. `readOnly` leaves looking, folding and moving around.

```yaml
- { type: ui-mindmap, props: { bind: plan, bindSelected: picked, height: 560 } }
- { type: ui-tree, props: { nodes: "{{ plan.children }}", bind: picked } }   # the same map as an outline
# plan: { id: root, label: Launch, children: [{ id: g, label: Goals, children: [{ id: g1, label: 500 sign-ups }] }] }
```

**`ui-progress-ring`** is `ui-progress` as a ring: `value` from 0 to 100 (clamped), the value in the middle (or `text`, empty for none), a `size` in px and a `tone`. It follows state without a script, easing to the new value unless reduced motion is asked for.

**`ui-rating`** picks a rating with stars, as radio buttons: a click or the arrow keys pick, hovering previews, and a click on the chosen star clears it (`clearable: false` stops that). With `value` instead of `bind`, it shows a rating read-only, like an average of 4.3, filling part of a star, with the number beside it; screen readers hear "Rating: 4.3 out of 5".

**`ui-password-checklist`** goes under a password input: `value: "{{ password }}"`. Each rule is ticked as it's met, and a bar shows the strength (Weak, Fair, Good, Strong). `rules` are `length` (8 characters, or `{ length: 12 }`), `lower`, `upper`, `number`, `symbol`, or `{ label, pattern }` with a regular expression; the default is the five presets. `bindValid` gets true while every rule is met, for a button's `disabled: "{{ !valid }}"`. Each rule is read with "met" or "not met".

**`ui-timeline`** lists activity: `ui-timeline-item` children and/or `items` from state, which redraw as they change. An entry has `who` (with an avatar, the initials, or for a name from state its first letter), `action`, `when`, `changes` (`[{ field, from, to }]`, shown as from → to with the old value struck through), a quoted `comment`, and children under it. An entry with no `who` is an event, shown with a dot in its `tone`.

As an activity feed, with each entry's `when` a date and time (`2026-11-20T16:40`) and the entries in order: `group: day` puts a heading over each day's first entry (Today, Yesterday, a weekday for the last week, then the date) and shows each entry's time of day; `relative` shows how long ago instead (`just now`, `5 min. ago`, `3 hr. ago`; past a day the time or the date), kept up to date each minute, with the full date on hover; `since` (a last visit, say, from state) marks the entries after it New; and `limit` shows the first so many, with a "Show 4 more" button for the rest, which moves focus to the first of them. These need a small script, added only to pages that use them; entries from state get them again whenever they change.

```yaml
- { type: ui-timeline, props: { items: "{{ activity }}", group: day, relative: true, since: "{{ lastVisit }}", limit: 10, label: Team activity } }
```

## Event calendar

`ui-event-calendar` shows `events` in a month grid, a week or a day of hours, a year of small months, or a list by day, with a toolbar to move through time and switch views. The block [`calendar-01`](../../examples/blocks/calendar-01.yaml) is a plant's schedule built on it.

```yaml
- type: ui-event-calendar
  props:
    events: "{{ events }}"     # [{ id, title, start, end, allDay, category, tone, description }]
    categories: { Meetings: info, Maintenance: warning, Holidays: success }
    bindEvent: event           # the event clicked
    bindDay: day               # the day picked, e.g. to add an event on it
    bindEvents: events         # makes events movable; the list is written back
    days: "{{ coverage }}"     # marks on days: [{ date, end, tone, label, items }]
    view: month                # or day, week, year, list
```

**Events.** `start` and `end` are dates (`2026-11-04`, an all-day event; `end` is its last day) or local times (`2026-11-04T09:30`). An event's colour is its `tone`, else its `category`'s tone in `categories` (shown as a legend), else `primary`. Tones are the status badge's: `neutral`, `info`, `success`, `warning`, `danger`, `primary`.

**Month** shows each day's events as chips, all-day and longer ones first, with `+N more` past `maxPerDay` (default 3), which opens that day's week. An event over several days appears on each of them, joined at the edges.

**Week** has a column per day and a row per hour (`hours`, default `[7, 20]`): timed events sit at their time and length, side by side where they overlap; all-day events and those over several days sit in a row above; today's column shows the time now.

**Day** is the week's layout for one day: its hours, its all-day events above, the time now if it's today.

**Year** shows twelve small months, the days with events tinted in the first one's colour with a dot. A click or Enter on a day picks it and opens it in the day view; a month's name opens the month.

**Marked days.** `days` marks days apart from events, say a rota's cover or closures: `[{ date, end, tone, label, items }]`, or a map of date → `{ tone, label, items }`. `tone` shades the day in that colour (in the month, the week's column and the year), `label` is written beside the date, and `items` are small badges under it, text or `{ text, tone }`; `end` makes one a run of days (its last). Two on one day share it, the later tone winning. Each day's name for screen readers gets its label and badges. In the year, a marked day with a tone takes it over its events' colour; `yearEvents: false` tints only the marked days. The list shows marked days too, the label beside the date and the badges under it.

**List** gives the month's days that have events (or marks), each event with its time (or All day, or "Until 6:00" on the last day of one that started the day before), its colour and category.

**Moving.** The arrows move a day, a week, a month or a year, as the view shows; Today comes back, and the switch changes view (`views` picks which, in order, from day, week, month, year and list, all five by default; `bindView` gets the change). It starts in `view`, or Month when it's offered. `date` sets where it starts, `today` the day treated as today, `weekStart` Sunday (0) or Monday (1, the default), and `locale` how dates and times are written.

**Moving events.** With `bindEvents` (the key `events` reads), events can be moved, and the list goes back to it with their new dates, written as they were (a date stays a date, a time a time; an event with no `end` keeps none unless it's made longer). In the month, drag an event onto another day; in the week and day views, drag a timed one to another time (snapped to `snap` minutes, default 15) or day, or its foot to make it longer or shorter, and an all-day one across the days. A press that doesn't move still opens the event, and a drag doesn't. With the keyboard, focus an event and use Alt and the arrow keys: Left and Right a day, Up and Down a snap in the week and day views (with Shift, the end only) or a week in the month. Each move fires `move` with `{ event, from }` and is announced.

**Keyboard and screen readers.** The days are a grid with one tab stop: the arrow keys move a day (or a week, up and down), Home and End to the week's ends, Page Up and Page Down a month, and Enter or Space picks the day (`bindDay`, and a `day` event). Tab then reaches that day's events. Each day is named with its date and how many events it has, each event with its title, time and category, and a change of month or week is announced. A click on an event fires `open` (and sets `bindEvent`); moving fires `navigate`.

**Props.** `events` (required), `view`, `views`, `date`, `days`, `yearEvents`, `categories`, `bindEvent`, `bindEvents`, `snap`, `bindDay`, `bindView`, `hours`, `weekStart`, `maxPerDay`, `today`, `locale`, `emptyText`, `label` (default `Calendar`); part `toolbar`.

## Kanban board

`ui-kanban` lays `cards` out in `columns` and lets you move them. The block [`kanban-01`](../../examples/blocks/kanban-01.yaml) is a sprint board built on it.

```yaml
- type: ui-kanban
  props:
    columns: [ { id: todo, title: To do }, { id: doing, title: In progress, limit: 3 }, { id: done, title: Done, tone: success } ]
    cards: "{{ tasks }}"     # [{ id, column, title, description, tags, assignee, due, priority }]
    bindCards: tasks         # moves and new cards are written back
    bindCard: task           # the card clicked or opened with Enter
    addable: true
    collapsible: true
```

**Cards** show their `title`, a two-line `description`, `tags` (text, or `{ label, tone }` with the status badge's tones), and a footer with the `priority` (Urgent and Critical red, High amber, Medium blue, Low grey), the `due` date and the `assignee`'s initials or `avatar` (a name, or `{ name, initials, avatar }`). The cards' order in `cards` is their order in each column.

**Columns** are `{ id, title, tone, limit }`, or plain names. A column with more cards than its `limit` shows its count in red (`4 / 3`). With `collapsible`, a button folds a column to a narrow strip; `collapsed` lists those folded to start with.

**Moving.** Drag a card into a column, between cards: a line shows where it lands. On a touch screen, hold a card still for a moment to pick it up, then drag it; a quick swipe still scrolls, and the board scrolls when the card nears its edge. From the keyboard, Alt with Up and Down moves the focused card within its column, and Alt with Left and Right moves it to the next column, keeping its place as near as it can. Each move rewrites `bindCards` (the card's `column`, and the order) and fires `move` with `{ id, from, to, index }`. Without `bindCards` nothing moves, but the event still fires, for a host that saves first.

**Adding.** With `addable` (and `bindCards`), Add card under a column asks for a title: Enter adds the card at the column's end with a new id and fires `add` with `{ card }`, and Escape cancels.

**Keyboard and screen readers.** The cards are one tab stop. Up and Down move between cards in a column, Left and Right to the nearest card in the next column with any, Home and End to a column's ends, and Enter or Space opens the card (`bindCard`, and an `open` event). `bindCard` follows its card as the cards change, and empties when it's removed. Each card's label gives its title, priority, due date and assignee; columns are named with their card count; moves and additions are announced.

**Swimlanes.** `swimlanes: assignee` splits the board into a row per value of that card field (an object's `name`, `label` or `id`): the columns' headers once at the top, then each lane's header (its name and count, a button that folds it) over a cell per column. A card's place is its column and its lane. Dragging it into another lane's cell, or Alt with Down at the end of a cell (Up at its start), moves it there and writes the lane's value into the field; Up and Down go on into the next lane's cell in the same column. Lanes come from `lanes` (values or `{ value, label }`, kept even when empty) and then from the cards' values as they come, the cards without one last ("No assignee"). A column's `limit` counts its cards in every lane. Add card in a lane's cell gives the new card that lane's value.

**Fields.** `fields` adds lines to each card, in order: `{ field, label, cell }` (a dotted field works), `cell` being `text` (default), `badge` (a pill in `tones`' tone for its value), `progress` (a bar, 0–100) or `date` (written in `locale`). A card without the field leaves its line out.

**Settings from state.** `swimlanes` and `fields` may use state, so a board's settings can change them: `swimlanes: "{{ laneBy == 'none' ? null : laneBy }}"` from a select, or `fields: "{{ detailed ? long : short }}"` with the two lists in state. The board redraws with the new lanes or lines, and keeps them as cards move (block crm-01).

**Your own cards.** By default the board draws each card: its title, description, `fields`, tags, priority, due date and assignee. Give `ui-kanban` children and they are each card's content instead, with the card in scope as `card` (and its place in its cell as `index`), so a card can be any of the page's components: an avatar, stars, badges, a list, a button. The server draws them that way too. The board still owns the card itself: it's dragged and moved with Alt and the arrows, named for screen readers from `title`, `priority`, `due` and `assignee`, and opened by a click or Enter, except a click on a link or control inside it, which does its own thing. The block [`kanban-02`](../../examples/blocks/kanban-02.yaml) is a hiring pipeline built this way.

**Moving columns.** With `reorderable`, each column's header has a grip: drag it onto another header (its first half puts it before that column, its second after), or focus it and press Left or Right. By touch, hold the grip for a moment first; the header then follows the finger. With `swimlanes`, each lane's header has a grip too (not the lane of cards without a value, which stays last): drag it onto another lane (its top half puts it before, its bottom half after), or focus it and press Up or Down. The lanes' new order goes to `bindLanes`, the state key `lanes` reads, written as `lanes` had them (values, or `{ value, label }`s), with a `lanes` event of `{ lane, from, to, lanes }`. The columns' new order goes to `bindColumns` (the state key `columns` reads), with a `columns` event, and is announced.

**Filtering.** `filter` shows only the cards that pass, `{ field: condition }` in the same format as `ui-data-table`'s and `ui-chart`'s `filter`, so a `ui-filters` panel's `bind` narrows a board too: `filter: "{{ boardFilter }}"`, or `{ assignee: "{{ owner }}" }`. A column's count reads "2 of 6" while some of its cards are hidden; a `limit` still counts all of them. Moving a card places it among the cards shown, and the hidden ones keep their places in `bindCards`. A card added or moved so that it no longer passes drops out of view. The filter is applied in the browser, so the server's first drawing shows every card (block crm-01).

**Props.** `columns` and `cards` (required), `bindCards`, `bindCard`, `movable` (default true), `addable`, `collapsible`, `collapsed`, `addLabel`, `emptyText`, `columnWidth` (px, default 280), `label` (default `Board`), `swimlanes`, `lanes`, `fields`, `reorderable`, `bindColumns`, `bindLanes`, `filter`, `locale`; part `board`.

## Dashboard grid

`ui-dashboard-grid` lays `ui-tile`s out on a grid of `columns` (default 12) and rows `rowHeight` px tall (default 80), for dashboards people arrange themselves. The block [`dashboard-02`](../../examples/blocks/dashboard-02.yaml) is an operations dashboard built on it.

```yaml
- type: ui-dashboard-grid
  props: { bind: layout, rowHeight: 72 }
  children:
    - { type: ui-tile, props: { id: alerts, x: 0, y: 0, w: 12, h: 1, locked: true }, children: [ ... ] }
    - { type: ui-tile, props: { id: sales, title: Sales, x: 0, y: 1, w: 8, h: 4, minW: 4 }, children: [ { type: ui-chart, props: { …, height: 100% } } ] }
    - { type: ui-tile, props: { id: goals, title: Goals, w: 4, h: 4 }, children: [ ... ] }
```

**Places.** A tile's `x` and `y` are its column and row from 0, and `w` and `h` its size in cells (default 4 by 2). A tile without `x` and `y` takes the first free place. Overlapping tiles move down, and with `compact` (the default) every tile floats up into the space above it. The server draws the tiles where they end up, so nothing jumps when the script starts.

**Moving.** Drag a tile by its header (its `title`), or by the grip in its corner when it has none; the tile follows the pointer and a dashed outline shows where it lands. Drag the bottom corner to resize it, within `minW`, `minH`, `maxW` and `maxH`. The tiles in the way move down, or up into the space the tile left when they fit there, as in react-grid-layout. Pointer events, so touch works. Escape during a drag puts the tile back. A `locked` tile has no header handle or corner: it stays put and the others go around it. `editable: false` turns moving off for every tile.

**Keyboard and screen readers.** A tile's header is a button: the arrow keys move the tile a cell (a step down goes past the tile below), and Shift with them resizes it. The new place is announced (`Sales: column 1, row 2, 8 by 4`). The grid is a list of its tiles, named by `label`.

**State.** `bind` holds the layout as `[{ id, x, y, w, h }]`, written after each move and read to place the tiles, on the server and when it changes. Tiles it doesn't name keep their own places, and setting it to null puts every tile back where its props say (a Reset button). Each move also fires `layout` with `{ layout, id, how }` (`move` or `resize`).

**Small screens.** Under `stackBelow` px (default 640, of the grid's own width) the tiles stack in one column in reading order, each `h` rows tall, and can't be moved. A chart in a tile redraws at the tile's size; give it `height: 100%`.

**A layout per width.** `breakpoints` names sizes of grid, each with the `width` (of the grid, in px) it's used from and its `columns`: `{ desktop: { width: 960, columns: 12 }, tablet: { width: 560, columns: 6 } }`. The tiles' `x`, `y`, `w` and `h` are for the widest size; a tile's `layouts: { tablet: { x, y, w, h } }` sets its place in a smaller one (any of them: `{ w: 3 }` keeps the width and packs the tile in). Tiles without one are worked out from the widest layout: widths scaled to the size's columns, packed in reading order. Each size's layout is in the page's CSS, so the right one shows before the script runs, and the grid switches as its width crosses a breakpoint. A move or resize changes the layout of the size it's made in; `bind` then holds one layout per size, `{ desktop: [...], tablet: [...] }` (a plain list is taken as the widest size's), and `layout` events carry the `size`. Under `stackBelow` the tiles still stack. The [`dashboard-02`](../../examples/blocks/dashboard-02.yaml) block has a tablet layout.

**Adding and removing tiles.** A tile's `when` (e.g. `"{{ showReturns }}"`, or `"{{ includes(widgets, 'returns') }}"`) puts it on the dashboard while it's true. Hidden, it takes no room: the tiles below float up into its space. Shown again, it goes back to its place, from `bind` or its own `x` and `y`, and the tiles there make way; a tile without one takes the first free spot. A menu of `ui-menu-checkbox` items (with `keepOpen`) bound to the flags makes a Widgets picker, as in block dashboard-02. The layout in `bind` isn't rewritten when tiles come and go.

**Right to left.** Column 0 is on the right, and dragging and the arrow keys follow.

**Props.** Grid: `columns`, `breakpoints`, `rowHeight`, `gap` (px, default 16), `bind`, `editable`, `compact`, `stackBelow`, `label`; parts `tile` and `header`. Tile: `id` (default `tile-1`, `tile-2`, …), `title`, `x`, `y`, `w`, `h`, `minW`, `minH`, `maxW`, `maxH`, `locked`, `layouts`, `when`.

## Tree

`ui-tree` shows nodes `{ id, label, icon, children }` (`name` works as the label too) as a tree to browse, select, check, search, add to, delete from and rearrange. The block [`tree-01`](../../examples/blocks/tree-01.yaml) has all of it.

```yaml
- type: ui-tree
  props:
    nodes: "{{ places }}"
    bindNodes: places        # moves, adds and deletions are written back here
    bind: picked             # the selected node's id
    bindNode: place          # the selected node, for a details panel
    checkable: true
    bindChecked: checked
    addable: true
    deletable: true
    movable: true
    search: Search locations…
    counts: true
```

**Selecting and opening.** A click or Enter selects a node (`bind`, `bindNode`, and a `select` event). The arrow beside it, a double-click, or Right and Left open and close it. `expanded` sets what's open to start with: `true` for everything, a number of levels, or a list of ids. The selected node's parents always start open.

**Checking.** With `checkable`, a checkbox per node: checking a node checks everything under it, and a parent shows a dash while only some of it is checked. `bindChecked` holds the checked ids.

**Adding.** With `addable`, an Add button above the tree and a + on each row. With `bindNodes`, they put a name box where the node will go: Enter adds it with a new id and moves focus to it, and Escape or an empty name cancels. Either way they write the parent's id (null for the top) to `bindParent` and fire `add` with `{ parent, node }`. Without `bindNodes`, nothing is added: handle the `add` event yourself.

**Deleting.** With `deletable`, a bin on each row and Delete on the keyboard; with `checkable` too, a bar above the tree deletes the checked nodes. A node goes with everything under it. Deleting fires `delete` with the top ids, and with `bindNodes` removes them from state, along with the selection and checks.

**Moving.** With `movable`, drag a node onto another to put it inside, or near its top or bottom edge to put it before or after. On a touch screen, hold a node still for a moment to pick it up, then drag it. From the keyboard, Alt with Up and Down moves the focused node among its siblings, Alt+Right puts it inside the node above, and Alt+Left moves it out to after its parent. A node can't go inside itself. Moving fires `move` with `{ id, parent, index }` and, with `bindNodes`, writes the new tree. Dragging is off while searching.

**Renaming.** With `renamable`, F2 or a double click on a node's name puts a box in its place: Enter (or leaving the box) keeps the new name, Escape doesn't. It fires `rename` with `{ id, label, node }` and, with `bindNodes`, writes it (to `name` for nodes that use that).

**Icons.** A node's `icon` shows before its name: an icon kit name (`folder`, `logistics:warehouse`) or text such as an emoji. The icon kit draws the names the starting nodes use; list others that nodes added later may use in `icons`.

**Loading children.** With `childrenUrl`, a node with `children: true` loads its children when it's first opened: a GET to the URL with `?parent=<id>`, answering a list of nodes (or `{ nodes }`). Its arrow turns while it loads; the children are kept, so closing and opening it again doesn't ask again, and they can have `children: true` too. If the request fails the node closes, says so, and opening it tries again. With `bindNodes`, the loaded children are written along with the next change.

**Search.** `search` adds a box above the tree that shows the matches with the nodes above them and marks the match, ignoring case and accents. Down moves from the box into the tree, and Escape clears it.

**Keyboard and screen readers.** The tree follows the ARIA tree pattern: one tab stop; Up, Down, Home and End move; Right opens a node or steps into it, Left closes it or steps out; `*` opens a node's siblings; Enter selects and Space checks (or selects). The row buttons are left out of the tab order, since Delete, the Add button and Alt+arrows do the same. Moves, additions and deletions are announced.

**Props.** `nodes` (required), `bindNodes`, `bind`, `bindNode`, `checkable`, `bindChecked`, `addable`, `bindParent`, `addLabel` (default `Add`), `deletable`, `movable`, `renamable`, `childrenUrl`, `icons`, `search` (`true` or a placeholder), `counts` (how many nodes are under each), `expanded`, `title` (a heading that also names it), `label` (its name without a title, default `Tree`), `emptyText`, `height` (most it grows to before it scrolls); part `tree`.

## File upload

`ui-file-upload` takes files from a drop zone (or a click, or the keyboard), uploads them with their progress, shows them as a grid of cards or a list, previews them, and downloads them all as one zip. The block [`expense-01`](../../examples/blocks/expense-01.yaml) uses it for an expense claim's receipts.

```yaml
- type: ui-file-upload
  props:
    bind: receipts                 # [{ id, name, size, type, url, status, progress, error }]
    url: /api/upload               # each file POSTed as multipart, field `file`
    params: { claim: "{{ claim.id }}" }
    accept: "image/*,.pdf,.xlsx,.csv"
    maxSize: 10 MB
    maxFiles: 20
    hint: Images, PDFs or spreadsheets, up to 10 MB each
```

**State.** `bind` holds the files as `{ id, name, size, type, url, status, progress, error }`, so a form can send the list, and files already there (with a `url`) are shown too. `status` is `uploading`, `done`, `error`, or `local` for a file kept in the browser.

**Uploading.** With `url`, each file is sent as `multipart/form-data` (the file as `file`, with the `params` fields), three at a time, showing its progress. A JSON answer is merged into the file's entry: answer `{ id, url }` and the entry takes your id and the address to show it from. A failed upload shows the server's `error` (or `message`) with a retry button. Without `url`, files stay in the browser: they can be previewed, removed and zipped, and a host can read them with the `add` event.

**Checks.** Files that don't match `accept` (extensions like `.pdf`, types like `image/*`), that are over `maxSize` (bytes, or `10 MB`, `500 KB`), or that would go past `maxFiles` are left out, each with a reason under the zone. `multiple: false` keeps one file, replacing it with the next.

**Preview.** A click on a file opens it in a panel: images, PDFs (in the browser's viewer), CSV and Excel `.xlsx` sheets as a table (the first sheet, first 500 rows), and text. Arrows (and Left and Right) go through the files, and Download saves the one shown. Other kinds offer the download. No libraries: `.xlsx` is unzipped with the browser's `DecompressionStream`.

**Download all** saves the files as one zip (`zipName`, default `attachments`), named so that two files of the same name both arrive. Files on a server are fetched, so they must be on the same site or allow it (CORS).

**Events.** `add` (`{ files }`), `uploaded` and `error` (`{ file }`), `remove` (`{ file }`: delete it on your server here) and `download` (`{ count }`), all bubbling, for `on:` actions or a host.

**Keyboard and screen readers.** The zone is a label around the file input, so Tab reaches it and Enter or Space opens the picker. Each file is a button that opens its preview, with its remove (and retry) buttons after it. Additions, uploads, failures and removals are announced; after a removal, focus goes to the next file.

**Props.** `bind` (required), `url`, `params`, `accept`, `multiple` (default true), `maxFiles`, `maxSize`, `view` (`grid` or `list`), `label`, `hint`, `readOnly` (the files only), `removable` (default true), `download` (default true), `zipName`, `emptyText`, `disabled`, `id`; parts `zone` and `files`.

## Chat

**Markdown.** With [irender-markdown-kit](../irender-markdown-kit/README.md) registered, a message's `content` is shown as Markdown (headings, lists, code, tables, links), rendered on the server and again in the browser as it changes, so a streamed reply formats as it comes in. While a reply streams its message has `streaming: true` (the input sets it and takes it off at the end; set it yourself on messages you stream in), and formatting left open at the end of the text shows as it will once closed: an unfinished `**`, code span, link or table doesn't flash as raw Markdown. It's as safe as `ui-markdown`: HTML in the text shows as text and only safe links are kept. Links and code take the bubble's colours. `markdown: false` on a message or conversation keeps the text plain; without the kit, text is always plain, with its line breaks.

Messages are `{ id, role, content, name, avatar, initials, time, ui, html, replies }`: `role` is `assistant` (on the left; the default), `user` (on the right, in the primary colour) or `system` (a small centred note). Keep them in state and the conversation redraws as they change, keeping each message whose `id` it has seen.

```yaml
state: { messages: [], draft: "", waiting: false }
root:
  type: stack
  children:
    - type: ui-conversation
      props: { messages: "{{ messages }}", typing: "{{ waiting }}", height: 480, empty: Ask anything. }
    - type: ui-chat-input
      props: { messages: messages, bind: draft, waiting: waiting, endpoint: /api/chat, suggestions: [What can you do?] }
```

- **Sending.** Enter sends and Shift+Enter starts a new line. Sending appends `{ id, role: user, content }` to `messages`, then runs the input's `on.send` actions (the bound key still holds the text then), then clears the box. Suggestions show while `messages` is empty, and clicking one sends it.
- **Replies.** With `endpoint`, the input POSTs `{ messages }` there and adds the reply as an assistant message. The reply may be JSON (a message, `{ message }` or `{ content }`), plain text streamed as it arrives, or server-sent events whose `data` is text or JSON with `content`, `delta`, `text` or OpenAI's `choices[0].delta.content`, ending with `[DONE]`. `waiting` is true from sending until the first text arrives, which suits the conversation's `typing`. While a reply streams the send button stops it. A failed request adds a `system` message saying so.
- **Without an endpoint**, listen for `send` (an `on.send` action, or the bubbling DOM event, whose `detail` has `text` and `message`) and append the reply to the state key yourself, e.g. with `root.__irender.set`.
- **Threads.** A message's `replies` fold away under "N replies", indented with a rule; `open` unfolds them. Replies don't nest further.
- **Scrolling.** The conversation keeps to its latest message as messages arrive or stream in. Once you scroll up it stays put and shows a button that jumps back down; sending a message always does. It's a `role="log"`, so screen readers announce new messages.
- **Conversations.** Give the input `threads` (a state key holding `[{ id, title, messages }]`) and `thread` (the current one's id) to keep several. `messages` then shows the current thread's, and switching `thread` shows another's. Sending with `thread` null starts a thread, titled after the message, at the top of the list; sending in one moves it there. A reply keeps streaming into its own thread if you switch away. List the threads in a sidebar with `list` over the threads key, setting `thread` on click; see the [chat block](../../examples/blocks/chat.yaml).
- `parts.bubble`, `parts.avatar`, `parts.meta` and so on, set on the conversation, style every message it draws from `messages`.

### UI in messages

A message can carry UI as well as text: its `ui` is an iRender document (`{ state, root }`, with `version` optional) or a single node. A reply can then be a chart, a form, a carousel of images, a video, link cards or tiles, with its own state, shown under the text.

```yaml
- role: assistant
  content: "Sales by region:"
  ui:
    type: ui-chart
    props: { type: bar, x: region, data: [{ region: Asia, sales: 14 }, { region: Europe, sales: 13 }] }
```

A message's UI is shown from its `html`, markup rendered by iRender, made one of three ways. Use any of them, or several together:

| | where it renders | set up |
|---|---|---|
| **Server, ahead** | Your server renders the `ui` and sends `html` with the message. | Nothing on the page. Messages in the document are rendered by `irender render --ui-kit` (wherever they sit in state: a message is an object with a `role` and a `ui`), or by `irender_ui_kit::render_message_ui(&mut doc, …)` from Rust. A reply's comes from `irender_ui_kit::render_ui` or `irender render reply.json --embed --ui-kit`. |
| **Browser** | The page renders a `ui` that arrived without `html`, with [irender-wasm](../irender-wasm/README.md). | `render: { browser: /irender/irender_wasm.wasm }` on `ui-conversation` (or `ui-message`). |
| **Server, on request** | The page posts `{ ui }` to your render endpoint, which answers with the markup (or JSON `{ html }`). | `render: { url: /api/render }`. |

The page uses the first that works: `html` when the message has it, then `browser`, then `url`; with none of them it shows a short note in the message. `browser` falls back to `url` when the module can't load (a Content-Security-Policy without `'wasm-unsafe-eval'`, say) or can't render the `ui` (a component it wasn't built with). So a typical setup renders the document's messages ahead, streams replies as `ui`, renders them in the browser, and keeps an endpoint for the rest:

```yaml
- type: ui-conversation
  props:
    messages: "{{ chat }}"
    render: { browser: /irender/irender_wasm.wasm, url: /api/render }
```

A reply sends its UI as `ui` or `html`: in the JSON reply, or as a server-sent event `{ "ui": … }` or `{ "html": … }`, usually after the text. The markup leaves out the theme's variables, so it takes the page's theme. The message starts it as a root of its own: its state is its own, it survives the conversation redrawing, and its `call` actions reach the host like the page's. `render_ui` is the one function behind the CLI, a render endpoint and irender-wasm, so all three give the same markup.

- **Trust.** Markup inserted into the page is as powerful as a script, so a message shows UI only in trusted documents (a policy that allows `<script>`). Render UI a model wrote under the untrusted policy (`Engine::untrusted()`, `--untrusted`), which keeps scripts, event attributes and `javascript:` URLs out; irender-wasm always does.
- **Self-contained.** A message's `ui` has its own state and fragments; it can't read the page's server `data` or `include` fragments from your fragment directory.
- `examples/serve.py` shows each way: the chat block's sample replies are rendered ahead, "photos …" is answered with `html`, and "chart …" with `ui`, which the page renders with irender-wasm when it's built and through `api/render` when it isn't.

## Forms

`ui-form` checks its fields by `rules`, keyed by state key, as each is left and all of them on submit; its `on: { submit: … }` actions run only when everything is right. `bindErrors` holds the messages of the fields left so far, for each `ui-field`'s `error`.

```yaml
- type: ui-form
  props:
    bindErrors: errors
    bindDirty: dirty
    guard: true                      # ask before leaving with unsaved changes
    rules:
      email: { label: Email, required: true, type: email }
      start: { label: Start, required: true, type: date }
      end: { label: End, type: date, min: "{{ start }}", message: After the start }
      topics: { label: Topics, min: 1, max: 5 }          # a list, by how many it has
      again: { match: password, message: The two don't match }
      handle: { label: Handle, required: true, remote: /api/available }   # asked of the server
  on: { submit: { action: submit, endpoint: /api/save, method: POST, body: { email: "{{ email }}" } } }
  children:
    - { type: ui-error-summary }     # what's wrong, once submitting was tried; each goes to its field
    - { type: ui-field, props: { label: Email, error: "{{ errors.email }}" }, children: [ { type: ui-input, props: { bind: email } } ] }
    - { type: ui-save-bar }          # shown while there are unsaved changes: Save submits, Discard puts them back
```

Unsaved changes are the `keys` (by default, those with rules) compared with how they were. They're saved when a `submit` action answers ok, or when the page sets `bindDirty` to false. Any rule may use state.

A `remote` rule asks the server whether a value is accepted (is this name taken?), once the other rules pass and 400 ms after typing stops (`wait`): a GET to the url with `?value=…` (`param` renames it, `{value}` in the url is replaced instead). The server answers `true` or `{ valid: true }`, or what's wrong: `false`, a text, or `{ valid: false, message }`. Answers are remembered, `bindChecking` lists the keys being asked (for a "Checking…" under the field), `bindValid` is false meanwhile, and a submit waits for the answer. A server that can't be reached lets the value pass, as the server checks it again when it's sent, and the form fires `failed`. `examples/serve.py` answers `api/available` for profile-01's handle.

### Columns and sections

`columns` on `ui-form` lays its children out in that many columns; `ui-form-section` groups fields under a `title` and `description` with `columns` of its own. They mix: a form of two columns can hold sections side by side, one of them `span: full` with three columns inside, and a section with no title is just a row of columns inside another. `layout: aside` puts a section's heading beside its fields (up to 240px) while there's room.

```yaml
- type: ui-form
  props: { columns: 2, bindErrors: errors, rules: { … } }
  children:
    - { type: ui-error-summary }                 # the summary and the save bar always take the full width
    - type: ui-form-section
      props: { title: Company }
      children:
        - { type: ui-field, props: { label: Name }, children: [ { type: ui-input, props: { bind: company } } ] }
        - type: ui-form-section                  # no title: two columns inside the section
          props: { columns: 2 }
          children: [ … ]
    - type: ui-form-section
      props: { title: Contact }
      children: [ … ]
    - type: ui-form-section
      props: { title: Address, layout: aside, columns: 3, span: full }
      children:
        - { type: ui-field, props: { label: Street, span: full }, children: [ … ] }
        - { type: ui-field, props: { label: City }, children: [ … ] }
        - { type: ui-field, props: { label: Country, span: 2 }, children: [ … ] }
    - { type: ui-save-bar }
```

`span` (on `ui-field` and `ui-form-section`) is a number of columns or `full`; anything else takes more with `style: { grid-column: 1 / -1 }`. The columns follow the space they have, not the window (container queries), so a form in a drawer stacks: three or more fall to two under 640px, and everything to one under 400px. `collapsible: true` folds a section under its heading, which opens and closes it (a native `<details>`, so it needs a `title`); `open: false` starts it folded, and `open` may follow state. A folded section never hides a mistake: its heading says how many of its fields need fixing ("2 to fix") once their messages show, a submit that's stopped opens the ones with wrong fields in them, and going to a field (from `ui-error-summary`, say) opens the sections around it. Block supplier-01 shows them together, its Payment section folded.

**`ui-stepper`** shows a process's steps and the current one's panel, with Back, Next and Finish (`on: { finish: … }`). `bind` holds the current `ui-step`'s `value`. A later step waits until the ones before it are done, or are `valid`; `linear: false` lets people go anywhere. `bindFurthest` keeps the `value` of the furthest step reached, so someone who goes back can go forward again by the steps or Back and Next without pressing Next through each one; the steps before it show as done, unless an earlier step is no longer `valid`, which shuts the ones after it again. It starts unset (`null`) and follows the current step; it's never behind it. It needs no script.

**`ui-approval`** shows a sign-off in stages: a manager, then finance, then a director. `bind` holds the stages, `{ id, title, rule, approvers: [{ id, name, avatar, decision, comment, at }] }`; a stage is approved when its `rule` is met (`all`, the default, `any`, or a number of its approvers), rejected once it no longer can be, and waiting when someone asks for changes. The first stage not approved is the current one; those after it wait. At the top, what it comes to ("Waiting for Finance: 1 of 2 approvals", "Rejected by Sam Okafor" with the reason); under it each stage with its approvers, what they decided, when, and their comment. When `me` is an approver on the current stage who hasn't decided, they get Approve, Request changes and Reject, with a comment (needed for the last two); the decision goes back to state with its time, a `decide` event fires, and `done` when it's approved or rejected. After changes or a rejection, `owner` (when it's `me`) can send it again, which clears the decisions. `bindStatus` gets `pending`, `changes`, `rejected` or `approved`; `readOnly` leaves out the buttons. A stage's `due` (a date) shows under it, and on the current stage how long is left or that it's overdue. With `people`, an approver on their turn can hand it to one of them (not already on the stage): they take the approver's place, shown deciding for them, and a `handoff` event fires. Sending it again keeps each stage's decisions as an earlier round in its `rounds`, shown folded under it. Who may decide is for the server to check too: this only shows the buttons to the right person.

```yaml
- { type: ui-approval, props: { bind: approval, me: "{{ user.id }}", owner: "{{ claim.by }}", bindStatus: status } }
# approval: [{ title: Manager, approvers: [{ id: ravi, name: Ravi Kumar }] }, { title: Finance, rule: any, approvers: [{ id: asha, name: Asha Rao }, { id: mei, name: Mei Lin }] }]
```

**`ui-repeater`** is rows of `fields`, kept as a list of objects. `type: computed` fields work out `compute` over the row's fields (`qty * price`), `summary` totals them under the rows and in `bindTotals`, and `bindErrors` says what's wrong. Enter goes to the same field of the next row. Children are drawn after the fields with the row as `item`, and a native control of theirs marked `attrs: { data-field: name }` is kept in step too; components that `bind` a state key can't be a row's field. For a choice from many, `type: combobox` is typed into with the `options` suggested as you go (the browser's own list): a label or a value typed is the option's value, and anything else is said not to be one of the choices. In a `ui-form`, its rows are checked with the form's fields: a wrong row stops the submit, is in `bindErrors` as `lines.2.qty` ("Line 3, Qty: At least 1" in the error summary, which goes to that field), and shows all its messages once a submit was tried; the rows are unsaved changes too, put back by Discard.

**`ui-sortable-list`** puts a list in state in order: priorities, a menu, steps, a playlist. Each item has a grip to drag with a mouse, a finger or a pen (it doesn't scroll the page, so there's no wait before a drag), and a line shows where the item lands; the page scrolls near the window's edges. From the keyboard, Tab reaches each grip (↑ and ↓ go between them): Space or Enter picks the item up, ↑ and ↓ (Home and End) move it, Space puts it down and Escape puts it back; Alt with ↑ or ↓ moves it straight away. Each step is said in a live region ("Bravo moved to 4 of 4"). The list is plain values or objects with an `id` (`itemKey` names another field); children are each item's content, with `item` and `index` in scope, and without them it shows the item (or its `label`, `labelField`). `numbered` puts each item's place before it, and `removable` adds a ✕ that takes it out, focus moving to the next item. The new order goes straight to state, and a bubbling `change` event says `{ value, moved, from, to }` (or `{ value, removed }`).

```yaml
- type: ui-sortable-list
  props: { bind: priorities, numbered: true, removable: true }
  children:
    - { type: text, props: { value: "{{ item.label }}" } }
- { type: ui-sortable-list, props: { bind: menuOrder } }   # ["Home", "Orders", "Reports"]
```

**`ui-form-builder`** lets people put a form together in the page: a sign-in sheet, a survey, a request form. Under **Build**, a palette of field types (short and long text, email, number, date, a dropdown, one choice, several choices, yes or no, and a section heading) adds one after the field picked, with a click or by dragging it onto the form. Fields are put in order by dragging their grips (mouse, finger or pen) or with ↑ and ↓ on a grip, picked to be set up in the inspector (label, key, help, placeholder, choices one on each line, lowest and highest or earliest and latest, shortest and longest, a pattern with its message, required, and **Show only when** an earlier field has an answer), and duplicated or deleted (Delete on a field). A field's key follows its label until one is typed, and keys stay unique. **Preview** tries the form, with its required, email, limit, length and pattern checks, and shows what it would send; a field shown only when another has an answer comes and goes as it's answered, and isn't sent while hidden. **Document** gives the form out as YAML or JSON, with Copy: an iRender page of the kit's own form, `ui-form` with `rules` for what was asked, `ui-field`s with the right control (`ui-input`, `ui-textarea`, `ui-number-input`, `ui-date-picker`, `ui-select`, `ui-radio-group`, `ui-toggle-group`, `ui-checkbox`), sections as `ui-form-section`, an error summary and a submit button. With `endpoint` it posts the answers there as JSON; without, it thanks the person. `bind` holds the fields (`{ id, type, label, description, placeholder, required, options, min, max }`) and `bindDocument` the document, both kept up to date; `types` narrows the palette.

```yaml
- type: ui-form-builder
  props: { bind: fields, bindDocument: formDoc, submitLabel: Sign in, endpoint: /api/sign-ins }
# fields: [{ id: name, type: text, label: Your name, required: true }, { id: team, type: select, label: Team, options: [Ops, Sales] }]
```

## Import

`ui-import` takes rows from CSV, Excel (`.xlsx`) or cells pasted from a sheet, read in the browser. Its columns are matched to `fields` by `label`, `field` and `aliases` (people can change the match), the rows are put right in an editable data table, and those that pass are handed over: the `import` event, `bindImported`, and with `url`, a POST of `{ rows, offset, total }` per `batchSize`. The server may answer `{ errors: [{ row, field, message }] }` (`row` counted in the batch) to send rows back to the review. Dates, numbers and choices are read in the locale's way; `template` offers an empty file.

## Pivot table

`ui-pivot` groups `rows` by the fields `down` and `across`, and sums `values` (`sum`, `avg`, `min`, `max`, `count`, `distinct`, `median`, `first`, `last`), as they are or as a share (`show: row`, `column` or `total`). A date field's `part` groups it by year, quarter, month, week or day. Outer groups fold, a heading sorts by its column, `heat` tints, `editable` lets people move fields about (kept in `bind`), `bindCell` gets a clicked cell with the rows behind it, and `export` writes Excel or CSV. It works on the rows given to it: sums over more than the browser holds belong on the server.

## Scheduler

`ui-scheduler` shows bookings against time, a row per resource, by day, week or month. With `bindEvents`, bookings are moved and stretched by dragging or with Alt and the arrow keys (and Shift for the end), and with `creatable`, dragging over a free stretch makes one. `hours` shades when a resource isn't free, `capacity` lets it take several, and `overlap` allows, marks (`bindClashes`) or prevents more than that. Times are local, as `YYYY-MM-DDTHH:MM`. Events: `move`, `resize`, `create`, `open`.

## Plan

`ui-plan` places `items` on a drawing, in the plan's own units: rectangles, circles, pins and polygons, with `image` behind them. It zooms with the wheel, a pinch, a double click or the buttons, goes between items with the arrow keys, and with `bindItems` moves them by dragging or Alt and the arrow keys, kept to `grid`. `tones` colours them by `status`, `filter` fades those that don't pass, `search` finds one, and `legend` counts them.

`layout` makes the items from a few words, with walls, screens and row numbers that can't be picked:

```yaml
- type: ui-plan
  props:
    layout:
      kind: aircraft          # bus, train, cinema, theatre, auditorium, stadium, hall, parking, office
      status: Free
      rows: 30
      seats: "ABC DEF"        # spaces are aisles; `last` is the last row's
      exits: [12]
      skip: [13]
      wings: [10, 16]
      sections: [ { name: Business, from: 1, to: 4, seats: "AC DF", size: 1.25, price: 420 } ]
      blocks: [ { after: 4, label: Galley } ]
    data: { 12A: { status: Taken } }
    bind: seats
    multiple: true
    max: 2
    available: [Free]
    bindTotal: total
```

- **Cabins** (`aircraft`, `bus`, `train`): `rows`, `first`, `seats`, `last`, `sections`, `exits`, `skip`, `missing`, `blocks`, `pitch`, `facing` (trains, with tables), `wings`, `direction` (`down` for the front at the top).
- **`cinema`**: `rows` (a number or names), `seats` (a number, a list, or `{ A: 14, default: 20 }`), `aisles`, `cross` (after rows), `curve` 0 to 1, `sections` by `rows`, `pairs`, `wheelchair`, `missing`, `doors`, `screen`, `from: right`.
- **`auditorium`, `theatre`**: rows in arcs: `rows` or `tiers: [{ name, rows, price, gap }]`, `blocks`, `angle`, `radius`, `sections`, `stage`.
- **`stadium`**: `blocks`, `tiers`, `width`, `height`, `field` (`stage` for a concert), `label`.
- **`hall`**: `style`: banquet or cabaret (`tables`, `seats`, `columns`, `stagger`), classroom (`rows`, `columns`, `seats`), theatre, ushape, boardroom or hollow (`seats` a side, `ends`), exhibition (`rows`, `columns`, `standWidth`, `standHeight`); `stage: false` leaves the stage out.
- **`parking`**: `rows`, `bays`, `charging`, `accessible`. **`office`**: `pods`, `desks`, `columns`.

A seat's id is its row and letter or number (`12A`, `C7`, `T3-2` for a table's chair); `prefix` goes before it. `data` gives what's known of each, by id. A status's tone comes before its section's.

`floors` are several of them, the top first: `[{ id, label, title, layout, items, image, width, height }]`. Buttons, Page Up and Page Down go between them, an item with `to` (stairs, a lift) goes to that floor, `search` looks on all of them, and what's picked can be on any.

## PDF

`ui-pdf` draws a PDF's pages with [pdf.js](https://mozilla.github.io/pdf.js/), which a page loads only when it has one to show: from jsDelivr, pinned, or from `library` (its `pdf.worker.min.mjs` beside it). `notes` (or `bindNotes`, to add and change them) mark it with highlights, boxes, notes and signatures, placed as parts of the page from 0 to 1 so they hold at any zoom. The file itself isn't changed. The browser fetches it, so it must be on the same site or allow others.

## Apps

A document's `app` makes its pages an installable app (see [the main README](../../README.md#apps)). The UI kit has what goes round them:

- `ui-install-prompt` offers to install it: the browser's prompt, or the steps for this browser (an iPhone has none). `ui-update-prompt` says when a new version has been fetched, and reloads into it.
- `ui-offline-indicator` says whether there's a connection. `checkUrl` asks a server, as a device can be on a network that goes nowhere.
- `ui-offline-queue` keeps requests under `paths` that can't reach the server (the `submit` action, forms, uploads, anything that uses `fetch`) and answers them with `202 { queued: true, id }`. They're sent in order once there's a connection, after the app's been closed too where the browser has background sync (not on iPhones, which send them when it's next opened). One the server refuses (4xx) stays, to be sent again or deleted; one it can't take now (5xx, 429) is tried later. Each has an `x-queued-id` header, so a server can tell one it has had. Events: `queued`, `sent`, `refused`, `drained`.
- `ui-local-store` keeps state keys on the device. Not for secrets.
- `ui-app-shell` is the frame. `history` follows the phone's back button; `pullToRefresh` runs `refresh`.
- `ui-push-opt-in` subscribes with a VAPID `publicKey` and POSTs the subscription to `saveUrl`. Send pushes as JSON, `{ title, body, url, icon, image, tag, actions, count }`; the service worker shows them, sets the badge to `count`, and goes to `url` when one's tapped. On an iPhone, only once it's on the home screen.
- `ui-share-target` takes what's shared with the app: name its page in `app.share`. `ui-share` shares on.
- `ui-passkey` needs a server that makes WebAuthn options and checks the answer. With `local` there's none, and it proves nothing to one. `ui-app-lock` covers the app; it doesn't encrypt what's on the device.

## Device and hardware

These ask when they're used, not as the page loads, need https (or localhost), and say what's wrong and how to allow it. Pictures and recordings are kept as `data:` addresses, so a form, `ui-local-store` or `ui-offline-queue` can take them; keep videos short.

- Any recent browser: `ui-camera`, `ui-photo-annotate`, `ui-document-scanner`, `ui-code-scanner` (a decoder in WebAssembly where the browser has none), `ui-location`, `ui-field-map`, `ui-geo-checkin`, `ui-route-tracker`, `ui-audio-recorder`, `ui-video-recorder`, `ui-read-aloud`, `ui-swipe-actions`, `ui-wake-lock`, `ui-file-handler` (saves as a download where files can't be written).
- Phones and tablets with the sensors: `ui-compass`, `ui-level`. An iPhone asks first.
- Computers: `ui-screen-capture`.
- Chrome, Edge and Safari: `ui-voice-input`; `transcribeUrl` takes a recording in Firefox.
- Chrome on Android: `ui-nfc`, and the phone's own list in `ui-contact-picker` (a contact card file or typing elsewhere).
- Chrome and Edge: `ui-bluetooth`, `ui-serial`, `ui-usb`.

A web app can't follow a location, or tell that it's entered a place, while it's closed. `ui-field-map` is a light map without a library, to draw on; for Google Maps, popups of any components or a lot of data, use the [map kit](../irender-map-kit/README.md)'s `ui-map`. Its default tiles are OpenStreetMap's, for light use: an app names its own with `tiles`.

## Overlays and ids

Dialogs, sheets, popovers and menus open from their `trigger` prop's button. To open one from your own button instead, give the overlay an `id` and put `"attrs": { "command": "show-modal", "commandfor": "<id>" }` (dialogs) or `"attrs": { "popovertarget": "<id>" }` (popovers) on the button.

Every overlay needs an element id. The `id` prop gives it one, and so does `attrs: { id }` on a dialog, sheet, drawer, popover or menu, whose own element it is. (On components that wrap their popover, such as the date picker, tooltip or filters, `attrs.id` stays on the wrapper and the popover keeps its own.) A resizable or rail panel's `attrs.id` is the id its handle or tab points at. Without one, an id is derived from the node's place in the document. It stays unique for each item of a list rendered on the server, and inside a list the browser rebuilds from state, it ends with the item's index and follows it when the list changes. An `id` you pass can use the item too, such as `"confirm-{{ item.id }}"`. Either way, the trigger, close buttons and menu items inside follow the same id.

To show a toast, set the toaster's state key to it:

```yaml
- type: ui-button
  props: { label: Save }
  on:
    click: { action: set_state, key: toast, value: { title: Saved, description: Your changes are live. } }
- { type: ui-toaster, props: { bind: toast } }
```

The toaster adds it to a stack and clears the key, so the next one can follow. Each toast hides after its own `duration` (the toaster's, 4 seconds by default; `0` keeps it until dismissed), and at most `max` show at once, the oldest leaving first. Timers pause while the pointer is over the toasts or focus is in them. `variant: destructive` shows an error. Toasts need JavaScript: the toaster's behaviour builds them from a template the server renders, so the `toast` part still styles them.

## Browser support

The overlays rely on invoker commands (`command`/`commandfor`), `closedby`, the Popover API, CSS anchor positioning and exclusive `<details name>`. Chrome and Edge 135 and later support all of them, and the kit's browser tests run in Chrome, WebKit and Firefox. Check your other target browsers. Where a feature is missing:
- Without anchor positioning (Safari before 26, Firefox ESR), a small script places menus, popovers, filters, pickers and the combobox's list beside their buttons instead: on the side the kit asks for, flipped when there's no room, mirrored right to left, kept on screen and following the page as it scrolls. It reads the kit's own CSS, so it covers every anchored component, and it does nothing in browsers that position anchors themselves. Its test turns the browser's anchor positioning off (`<html data-anchor-fallback>` runs it anyway).
- Light dismiss falls back to Escape.
- Accordion items stop closing each other.

## Not included

The components still missing next to shadcn/ui (resizable panels, a draggable drawer, …) and the gaps in the ones here are listed in [PENDING.md](../../PENDING.md).
