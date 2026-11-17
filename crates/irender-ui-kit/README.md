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
| `ui-button` | `label`, `icon`, `variant` (default, secondary, destructive, outline, ghost, link), `size` (default, sm, lg, icon), `href`, `type`, `disabled` | With `href` it renders a link that looks like a button. Children follow the label. |
| `ui-badge` | `text`, `icon`, `variant` (default, secondary, destructive, outline) | |
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

As with the built-ins, a document can override any attribute with `attrs`, any style with `style`, and inner elements with `parts`. The kit only sets defaults.

Options, items and page numbers can come from `state` as well as server data: `ui-radio-group` and `ui-toggle-group` options, `ui-breadcrumb` items, and `ui-pagination`'s `page`, `pages` and `siblings`. The server renders them for the starting state and the browser redraws them as the state changes. A breadcrumb's `items` from state must be a path, like `"{{ crumbs }}"`, so it can tell which item is last. Pagination from state adds a small behaviour that redraws its links.

`icon` on `ui-button`, `ui-badge`, `ui-alert` and `ui-sidebar-item` takes an icon name, like `settings` or `it:database`, drawn by [irender-icon-kit](../irender-icon-kit/README.md) when it's registered. Otherwise, or for anything that isn't a name, it is shown as text, so an emoji or a character like `⚠` works too.

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

`apply_theme` merges a neutral, shadcn-style palette (zinc) with a dark mode under the document's theme. Tokens the document sets itself keep their values.

## Combobox

`ui-combobox` is a searchable select modelled on [react-select](https://react-select.com/). Its behaviour (about 4 KB gzipped) is added only to pages that use it.

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

Components that need focus management or keyboard navigation written in JavaScript are out of scope: command palette, date picker, data table with sorting, carousel and context menu. `ui-combobox` and `ui-dropdown-menu` have their own small behaviours.
