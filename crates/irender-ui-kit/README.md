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

| component | props | notes |
|---|---|---|
| `ui-button` | `label`, `icon`, `variant` (default, secondary, destructive, outline, ghost, link, brand), `size` (default, sm, lg, icon), `href`, `type`, `disabled` | With `href` it renders a link that looks like a button. Children follow the label. |
| `ui-badge` | `text`, `icon`, `variant` (default, secondary, destructive, outline, brand) | |
| `ui-card` | `title`, `description`, `tag` | Parts: `header`, `title`, `description`. |
| `ui-card-footer` | | A row of actions; put it last in a card. |
| `ui-alert` | `title`, `description`, `icon`, `variant` (default, destructive) | `role="alert"`. Parts: `icon`, `title`, `description`. |
| `ui-empty` | `title`, `description`, `icon` | A placeholder for a page or list with nothing in it yet; children (usually buttons) follow the text. Parts: `icon`, `title`, `description`. |
| `ui-avatar` | `src`, `alt`, `fallback`, `size` | Initials when there's no `src`. |
| `ui-separator` | `orientation`, `decorative`, `label` | With `label`, a horizontal line with text in the middle, like "Or continue with". |
| `ui-skeleton` | `width`, `height`, `round` | Pulses gently; still for people who ask their system for reduced motion. |
| `ui-spinner` | `size` (default 16), `label` (default `Loading`) | A spinning ring in the text colour, `role="status"`. Slows down for people who ask for reduced motion. |
| `ui-kbd` | `text` | |
| `ui-progress` | `value` (0–100) | Updates live when `value` uses state, and is clamped to 0–100 in CSS. |
| `ui-aspect-ratio` | `ratio` (`1.5` or `"16 / 9"`) | Its children fill it. |
| `ui-table` | `caption` | Children are the table's rows and sections, so `list` with `tag: tbody` works. Part: `table`. |
| `ui-data-table` | see [Data table](#data-table) | Sorting, column filters, search, paging, row selection with bulk actions, expandable rows, a View menu and summaries, over an array of rows. |
| `ui-scroll-area` | `height`, `maxHeight`, `label` | Scrolls its children with a thin scrollbar in the theme's colours. Focusable, so the keyboard scrolls it; `label` makes it a named region. |
| `ui-heading` | `text`, `level` (1–4) | |
| `ui-text` | `text`, `variant` (p, lead, large, small, muted, code, blockquote), `tag` | Takes children, e.g. inline `code`. |
| `ui-label` | `text`, `for` | Takes children, so it can wrap a control. |
| `ui-field` | `label`, `for`, `description`, `error`, `orientation` (vertical, horizontal) | A label, the control (its children), help text and an error. The label names the control, and the control's `aria-describedby` points at the help text and error, so screen readers read them with it. The control is the one whose id is `for`, or else the first control inside, which gets an id if it has none. The error is hidden while empty, so `"{{ … ? 'Too short' : '' }}"` shows it only when needed, and it is announced when it appears. Parts: `label`, `description`, `error`. |
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
| `ui-tabs` + `ui-tab-panel` | tabs: `bind` (required), `id`; panel: `value`, `label` | `bind` holds the open panel's `value`. Tabs carry `role=tab`, `aria-selected`, `aria-controls`. |
| `ui-toggle` | `bind`, `label`, `variant` (default, outline), `size`, `disabled` | `aria-pressed` follows `bind`. |
| `ui-toggle-group` | `bind`, `options`, `multiple`, `variant`, `size`, `disabled` | One option pressed at most; pressing it again sets `bind` to null. With `multiple`, `bind` holds a list and each press adds or removes its option. |
| `ui-theme-toggle` | `labels` (`{ system, light, dark }`), `variant` (default `outline`), `size` (default `sm`) | System / Light / Dark. Light and Dark override `prefers-color-scheme` for the theme's dark mode and `@dark` styles; the choice is kept in `localStorage` (`irender-theme`) and applied before the page is shown, so a reload doesn't flash. Every page rendered with the kit applies it, with a toggle on it or not. Its behaviour is added only to pages that use it. |
| `ui-theme-option` | `option` (required), `swatch` (a token, or a list for several dots), `label`, `variant` (default `outline`), `size` (default `sm`) | Picks a choice of a theme option for every page; see [Theme options](#theme-options). |
| `ui-theme-panel` | `options` (default: every theme option), `mode` (default true) | A labelled `ui-theme-option` per option, palette first and chart colours last, and the mode. Put it in a `ui-popover` for a Theme button. |
| `ui-theme-colors` | `option` (default `palette`) | Colour inputs (picked, or typed as hex) for a custom primary and second brand colour; changing one picks the `custom` choice. `ui-theme-panel` shows it under the palette. |
| `ui-dialog` | `trigger`, `triggerVariant`, `title`, `description`, `id` | A modal `<dialog>`. Escape, its ✕ or a click outside closes it. |
| `ui-alert-dialog` | same as `ui-dialog` | `role=alertdialog`; only its own buttons close it. |
| `ui-sheet` | same, plus `side` (right, left, top, bottom) | A dialog along one edge. |
| `ui-dialog-close` | `label`, `variant` | Closes the dialog it is in; its own `on.click` actions run too. |
| `ui-dialog-footer` | | A right-aligned row of buttons. |
| `ui-popover` | `trigger`, `triggerVariant`, `side` (bottom, top, right, left), `align` (center, start, end), `id` | Native popover, placed next to its trigger and flipped when there's no room. |
| `ui-dropdown-menu` | same as `ui-popover`, plus `triggerSize` | Children: `ui-menu-item` (`label`, `shortcut`, `variant`, `href`, `disabled`; choosing it runs its `on.click` and closes the menu), `ui-menu-label`, `ui-menu-separator`. Keyboard: opening focuses the first item; Up/Down, Home/End and typing move between items; Down or Up on the button opens it; Tab closes it. |
| `ui-split-button` | `label`, `variant`, `size`, `disabled`, `menuLabel` (default `More actions`), `side`, `align` (default `end`), `id` | A primary action joined to an arrow that opens a menu of secondary ones, e.g. Download PDF with Excel and CSV in the menu. Its `on.click` is the primary action; children are the menu's items, as in `ui-dropdown-menu`, keyboard included. Parts: `primary`, `trigger`. |
| `ui-tooltip` | `text`, `side`, `id` | Shown while its children are hovered or focused. CSS only. The first child (usually a button) gets `aria-describedby` pointing at the tip. |
| `ui-hover-card` | `trigger`, `href` | Children are the card, shown while the trigger is hovered or focused. CSS only. |
| `ui-toaster` | `bind`, `duration` (default 4000 ms), `max` (default 3) | Toasts at the bottom right. Setting `bind` to `{ title, description, variant, duration }` adds one to the stack. |
| `ui-sidebar-layout` + `ui-sidebar` | layout: `bind`, `collapsible` (offcanvas, icon, none), `variant` (sidebar, inset, floating), `side`, `width`; sidebar: `label` | See [Sidebar](#sidebar). |
| `ui-sidebar-header`, `ui-sidebar-content`, `ui-sidebar-footer`, `ui-sidebar-group`, `ui-sidebar-item`, `ui-sidebar-trigger` | group: `label`; item: `label`, `icon`, `href`, `active`, `badge`, `open`; trigger: `label` | The parts of a sidebar. An item with children opens a sub-menu. |
| `ui-combobox` | see [Combobox](#combobox) | A searchable select in the style of react-select: multi-select, groups, async, creatable. |
| `ui-date-picker`, `ui-calendar` | see [Date picker](#date-picker) | A day (with a time), month, quarter or year; one, a range or several; presets, limits and financial years. The picker opens the calendar from a button; `ui-calendar` shows it inline. |
| `ui-command` | `placeholder`, `label`, `empty` (default `No results found.`), `id` | A command palette: a search box over a list of actions, filtered as you type (every word must match the label or `keywords`). Up and Down move between items, skipping disabled ones; Home and End jump while the box is empty; Enter runs the active item; Escape clears the search. Focus stays in the search box. Parts: `search`, `input`, `list`, `empty`. |
| `ui-command-dialog` | as `ui-command`, plus `trigger`, `triggerVariant`, `shortcut` (e.g. `mod+k`: ⌘K on Apple devices, Ctrl+K elsewhere), `title` (default `Command palette`) | The palette in a modal, opened by its trigger, the shortcut, or a button with `commandfor` its id. Running an item closes it, and it opens with an empty search. |
| `ui-command-group`, `ui-command-item`, `ui-command-separator` | group: `heading`; item: `label`, `keywords` (a string or a list), `icon`, `shortcut`, `href`, `disabled` | The palette's contents. An item runs its `on.click`, or follows `href`. A group hides while none of its items match, and separators hide while searching. |

| `ui-conversation` | `messages`, `typing`, `assistant`, `render`, `empty`, `label`, `height`, `maxHeight` | A chat log; see [Chat](#chat). |
| `ui-message` | `role` (assistant, user, system), `content`, `name`, `time`, `avatar`, `initials`, `html`, `ui`, `render`, `replies`, `open` | One message bubble, with optional UI under it (see [UI in messages](#ui-in-messages)) and a thread. Children go in the bubble. |
| `ui-chat-input` | `messages` (required), `endpoint`, `bind`, `waiting`, `threads`, `thread`, `placeholder`, `label`, `suggestions`, `disabled` | The box a message is written in; sends to `endpoint` and streams the reply in. Children go in its toolbar. |
| `ui-typing-indicator` | `label` (default `Typing`) | Three bouncing dots. |
| `ui-carousel` | `label` (default `Carousel`), `size` (each slide's width; default `min(280px, 85%)`), `gap` (default 12) | Slides in a row that scrolls sideways and snaps to each; children are the slides, and a `list` with `style: { display: contents }` works. The arrow buttons, Left and Right on the focused track, and swiping move a slide at a time; a button hides at its end. Parts: `track`, `previous`, `next`. |
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
- **Phones:** below 768px wide the sidebar is a sheet over the page instead. It starts hidden, the trigger opens it whatever `bind` says, and a tap on the backdrop, Escape or following one of its links closes it. While it's open the page behind is `inert` and focus is in the panel; closing it returns focus to the trigger.
- **Keyboard:** ⌘B (Ctrl+B) toggles the sidebar, or the sheet on a phone. With several layouts on a page, the first takes the shortcut.
- **Labels when collapsed:** narrowed to icons, an item shows its label beside it on hover or keyboard focus.
- **Variants:** `sidebar` puts a border between the panel and the page, `inset` makes the main area a raised panel on the sidebar's colour, and `floating` floats the panel itself.
- **Items:** a link with `href`, otherwise a button that runs its `on.click`. `active` marks the current page (`aria-current="page"`). Children make a sub-menu, a native `<details>` the item opens and closes; `open` starts it open.
- **Colour:** the panel uses `color.sidebar` when the theme has it, otherwise a step off the surface colour, in light and dark.

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

`apply_theme` merges a neutral, shadcn-style palette (zinc) with a dark mode under the document's theme. Tokens the document sets itself keep their values.

### Theme options

`apply_theme` also adds theme options, and `ui-theme-option` picks one:

```yaml
- { type: ui-theme-option, props: { option: palette, swatch: color.primary } }
- { type: ui-theme-option, props: { option: density } }
```

| option | choices | sets |
|---|---|---|
| `palette` | zinc (default), slate, stone, rose, blue, green, orange, violet, custom | every colour (neutrals) or the primary and ring (colours), each with its dark mode; custom: the colours from `ui-theme-colors` |
| `radius` | 0, 0.3, 0.5 (default), 0.75, 1.0 | `radius.sm`, `radius.md`, `radius.lg` |
| `density` | compact, default, comfortable | `space.scale`: 0.8, 1, 1.2 |
| `text-size` | small, default, large | `font.scale`: 0.9, 1, 1.125 |
| `font` | sans (default), serif, mono | `font.body` |

- **Picking:** `ui-theme-option` shows one button per choice; `swatch` names a token shown as a colour dot, or a list of them: `ui-theme-panel` shows a chart scheme's first four colours. A choice is kept in `localStorage` (`irender-theme-options`) and applied before the page shows on every page rendered with the kit, like `ui-theme-toggle`'s mode. Picking the default clears it. Pages already open follow at once, other tabs and pages in iframes included, so a picker on one page restyles the rest.
- **A Theme button:** `{ type: ui-popover, props: { trigger: Theme }, children: [ { type: ui-theme-panel } ] }` gives every option and the mode, including ones the document adds. The examples index, the blocks page and the dashboard block have one.
- **Scales:** the kit's paddings, gaps, margins and control heights are `calc(Npx * var(--space-scale, 1))`, and its font sizes `calc(Npx * var(--font-scale, 1))`, so density and text size reach every component. Use the same tokens in your own styles to follow them: `padding: "calc(16px * var(--space-scale, 1))"`.
- **Custom colours:** `ui-theme-colors` takes a primary and a second brand colour as hex (`#rgb` or `#rrggbb`, nothing else), typed or picked. They're kept in `localStorage` (`irender-theme-custom`) and set on `<html>` while the palette's `custom` choice is picked, with a hover shade, a ring and dark or light text on them (whichever reads better) derived from each. The chart kit's `custom` chart colours are built from the same two, and setting a colour picks them too unless a chart scheme was picked by hand; Reset puts both back. In script: `window.__irenderTheme.custom()` and `.setCustom("primary", "#1e3a8a")`.
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
- { type: ui-date-picker, props: { bind: meeting, time: true, step: 15 } }            # 2026-03-10T14:30
- { type: ui-date-picker, props: { bind: quarter, picker: quarter, fiscalStart: 4 } }  # 2026-Q1: April to June 2026
- { type: ui-calendar, props: { bind: shifts, mode: multiple, disabledDays: [0, 6] } }
```

| prop | |
|---|---|
| `bind`, `value` | The state key holding the value, or the starting value when not bound. A day is `2026-03-01` (with `time`, `2026-03-01T14:30`), a month `2026-03`, a quarter `2026-Q1`, a year `2026`; a range is `{ from, to }` and `multiple` a list. Partial ranges keep `to: null` while the end is being picked. |
| `picker` | `day` (default), `month`, `quarter` or `year`. |
| `mode` | `single` (default), `range` or `multiple`. |
| `time`, `step` | A time under the calendar, for single days; `step` is the minutes between times (default 1). |
| `min`, `max` | The earliest and latest dates: `2026-01-01`, or `today`, `today-30d`, `today+2w`, `today+1m`, `today+1y`. They may use state, and the arrows don't page past them. |
| `disabledDates`, `disabledDays` | Dates that can't be picked (`2026-12-25`, or `{ from, to }`), and weekdays (0 Sunday … 6 Saturday). |
| `months` | Months side by side (default 1); two suit ranges. |
| `weekStart`, `locale` | The first day of the week (default: the locale's) and the locale for names and formats (default: the browser's). |
| `fiscalStart` | The month a financial year starts (1–12, default 1). Quarters and years follow it: with `fiscalStart: 4`, `2026-Q1` is April to June 2026 and shows as `Q1 FY2026–27`, and the year `2026` is April 2026 to March 2027. |
| `presets` | Shortcuts beside the calendar: `today`, `yesterday`, `tomorrow`, `last7`, `last30`, `last90`, `next7`, `next30`, `thisWeek`, `lastWeek`, `thisMonth`, `lastMonth`, `nextMonth`, `thisQuarter`, `lastQuarter`, `thisYear`, `lastYear`, `yearToDate`, `monthToDate`, or `{ label, from, to }` with dates or `today±N`. Quarters and years follow `fiscalStart`; the preset that matches the value shows pressed. |
| `dropdowns` | Month and year menus in place of the caption, for dates far away such as a birthday. |
| `bindRange` | A state key that gets the dates the value covers as `{ from, to }`: a quarter's first and last days, say. Give it to a chart's or a data table's `filter` to filter by the pick. |
| `name` | Hidden inputs carry the value in a form: `name`, or `name[from]` and `name[to]` for a range, or one per date. |
| `label` | The accessible name. In a `ui-field`, its label names the picker's button. |
| `clearable` | A ✕ in the picker's button (a Clear link under `ui-calendar`). |
| `placeholder`, `closeOnSelect`, `align`, `disabled` | The picker only: the text while empty (default `Pick a date`, `Pick a range`, …), closing once a value is complete (default true; not with `time` or `multiple`), which edge the popover lines up with (`start` or `end`), and turning it off. |

The calendar follows the WAI-ARIA date picker pattern: a grid with one cell in the tab order.
- Arrows move a day (or a month, quarter or year), up and down a week (or a row).
- Page Up and Page Down change month (Shift: year), or year for the other pickers.
- Home and End go to the start and end of the week (or row).
- Enter or Space picks. A range takes two picks and shows the span under the pointer while the end is chosen.
- Escape closes the picker, and focus goes back to its button.
- A live region says what was picked.

Days outside the month show muted (left out when two months show), today is marked, and the value's text in the button follows the locale. Each change fires a bubbling `change` event with `{ value, range }`.

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
| `filter` | A filter button in the toolbar: `select` (checkboxes with counts; choices from `options` or the column's values; list-valued fields such as tags match on any value), `text` (contains), `range` (min and max) or `date` (a range calendar with presets). |
| `format`, `currency`, `decimals` | `number`, `currency`, `percent` (0.25 is 25%), `date` or `datetime`, formatted in the browser with `Intl` in `locale`. Plain dates (`2026-03-01`) show the same day in every time zone. |
| `cell`, `tones` | Draw the value as a `badge` (with a tone per value: default, secondary, outline, success, warning, destructive, info), a `progress` bar (0–100) or a `sparkline` of a list of numbers (needs the chart kit). |
| `summary` | A footer cell: `sum`, `avg`, `min`, `max` or `count` of the rows that pass the filters. |
| `pin` | `left` or `right`: the column stays in view while the rest scroll sideways. The checkbox and arrow columns stay with the left ones, and a shadow marks where pinned columns end. |
| `tree` | The column that shows nesting, indented with the arrow that opens a row (default: the first column, in a table with nested rows). |
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
- A select filter's choices come from `options`, or the rows loaded so far; summaries and `bindView` cover the rows loaded so far.
- `url` must be a path or an http(s) URL, as `ui-combobox`'s `searchUrl`.

**Virtual.** With `virtual`, only the rows in view (and a few either side) are in the page, with space above and below standing in for the rest, so a table of thousands of rows scrolls smoothly. It scrolls inside `height` (default 480). Rows of different heights work: the table measures the rows it draws, starting from `rowHeight` (default 41). Arrow keys, Home and End reach rows that aren't drawn yet. It works with every `paging`, rows from `rows` or `url`, and nested rows; with `scroll` it's the usual infinite list.

### Nested rows

Rows can hold rows. The tree column (`tree: true`, or the first) indents each row by its depth, behind an arrow that opens it, and the table is a WAI-ARIA tree grid (`aria-level`, `aria-expanded`).
- **Children in the row:** a `children` array (another field with `childrenField`). Rows from state that start out empty need `childrenField` or a `tree` column to say they nest.
- **Flat rows:** `parentField: parentId` builds the tree from rows that name their parent's key. A row whose parent isn't there is a top row.
- **Loaded when opened:** a row with `children: true` (or `hasChildren: true`) gets its children from `childrenUrl` the first time it's opened: `GET <childrenUrl>?parent=<key>&sort=…`, answering an array or `{ rows }`. The arrow dims while they load; if they don't, the row closes again and opening it retries.

`expanded: true` opens every row to start with. Siblings sort among themselves under their parent. A search or filter shows the rows that match with the rows above them, opened. Summaries add up the top rows only, so nothing counts twice. Give rows a `rowKey`: it identifies them when they open, load and are selected.

On a focused row, Right opens it, then steps to its first child; Left closes it, then steps to its parent. The other keys work as in any table.

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

**How it works.** The server renders the rows, in the starting order and with the first page shown, so the table reads without JavaScript. The browser then sorts, filters and pages by moving and hiding the rendered rows rather than drawing them again, so what's inside a cell keeps its state. Virtual, remote and nested tables instead hand the runtime the rows to show, as a list keyed by `rowKey` (`api.list` in the [runtime](../irender-html/src/runtime.js)); a row that stays keeps its nodes, charts included.

## Chat

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

## Overlays and ids

Dialogs, sheets, popovers and menus open from their `trigger` prop's button. To open one from your own button instead, give the overlay an `id` and put `"attrs": { "command": "show-modal", "commandfor": "<id>" }` (dialogs) or `"attrs": { "popovertarget": "<id>" }` (popovers) on the button.

Every overlay needs an element id. Without the `id` prop, one is derived from the node's place in the document. It stays unique for each item of a list rendered on the server, and inside a list the browser rebuilds from state, it ends with the item's index and follows it when the list changes. An `id` you pass can use the item too, such as `"confirm-{{ item.id }}"`. Either way, the trigger, close buttons and menu items inside follow the same id.

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

The overlays rely on invoker commands (`command`/`commandfor`), `closedby`, the Popover API, CSS anchor positioning and exclusive `<details name>`. Chrome and Edge 135 and later support all of them, and the kit's browser tests run in Chrome. Check your other target browsers. Where a feature is missing:
- A popover still opens, but in the middle of the screen.
- Light dismiss falls back to Escape.
- Accordion items stop closing each other.

## Not included

Components that need focus management or keyboard navigation written in JavaScript are out of scope for now: date picker and context menu. `ui-combobox` and `ui-dropdown-menu` have their own small behaviours.
