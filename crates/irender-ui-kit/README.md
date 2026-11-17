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
| `ui-button` | `label`, `variant` (default, secondary, destructive, outline, ghost, link), `size` (default, sm, lg, icon), `href`, `type`, `disabled` | With `href` it renders a link that looks like a button. Children follow the label. |
| `ui-badge` | `text`, `variant` (default, secondary, destructive, outline) | |
| `ui-card` | `title`, `description`, `tag` | Parts: `header`, `title`, `description`. |
| `ui-card-footer` | | A row of actions; put it last in a card. |
| `ui-alert` | `title`, `description`, `icon`, `variant` (default, destructive) | `role="alert"`. Parts: `icon`, `title`, `description`. |
| `ui-avatar` | `src`, `alt`, `fallback`, `size` | Initials when there's no `src`. |
| `ui-separator` | `orientation`, `decorative` | |
| `ui-skeleton` | `width`, `height`, `round` | Pulses gently; still for people who ask their system for reduced motion. |
| `ui-kbd` | `text` | |
| `ui-progress` | `value` (0–100) | Updates live when `value` uses state, and is clamped to 0–100 in CSS. |
| `ui-aspect-ratio` | `ratio` (`1.5` or `"16 / 9"`) | Its children fill it. |
| `ui-table` | `caption` | Children are the table's rows and sections, so `list` with `tag: tbody` works. Part: `table`. |
| `ui-heading` | `text`, `level` (1–4) | |
| `ui-text` | `text`, `variant` (p, lead, large, small, muted, code, blockquote), `tag` | Takes children, e.g. inline `code`. |
| `ui-label` | `text`, `for` | Takes children, so it can wrap a control. |
| `ui-input`, `ui-textarea`, `ui-select` | same as `input`, `textarea`, `select` | Same binding behaviour as the built-ins, restyled. |
| `ui-checkbox`, `ui-switch` | `label`, `bind`, `checked`, `name`, `disabled` | The switch is a checkbox with `role="switch"`. Parts: `control`, `label`. |
| `ui-radio-group` | `options`, `bind`, `value`, `name`, `orientation`, `disabled` | Options are values or `{ value, label, disabled }`. |
| `ui-breadcrumb` | `items` (`{ label, href }`), `separator` | The last item is the current page. |
| `ui-pagination` | `page`, `pages`, `href`, `siblings` | `href` is evaluated per page with `p` in scope: `"/orders?page={{ p }}"`. |
| `ui-slider` | `bind`, `min`, `max`, `step`, `name`, `disabled` | A range input; `bind` holds a number. |
| `ui-accordion` + `ui-accordion-item` | accordion: `multiple`, `id`; item: `title`, `open` | Native `<details>`. One item open at a time unless `multiple`. |
| `ui-collapsible` | `trigger`, `open` | Native `<details>`. |
| `ui-tabs` + `ui-tab-panel` | tabs: `bind` (required), `id`; panel: `value`, `label` | `bind` holds the open panel's `value`. Tabs carry `role=tab`, `aria-selected`, `aria-controls`. |
| `ui-toggle` | `bind`, `label`, `variant` (default, outline), `size`, `disabled` | `aria-pressed` follows `bind`. |
| `ui-toggle-group` | `bind`, `options`, `variant`, `size`, `disabled` | One option pressed at most; pressing it again sets `bind` to null. |
| `ui-theme-toggle` | `labels` (`{ system, light, dark }`), `variant` (default `outline`), `size` (default `sm`) | System / Light / Dark. Light and Dark override `prefers-color-scheme` for the theme's dark mode and `@dark` styles; the choice is kept in `localStorage` (`irender-theme`). Its behaviour is added only to pages that use it. |
| `ui-dialog` | `trigger`, `triggerVariant`, `title`, `description`, `id` | A modal `<dialog>`. Escape, its ✕ or a click outside closes it. |
| `ui-alert-dialog` | same as `ui-dialog` | `role=alertdialog`; only its own buttons close it. |
| `ui-sheet` | same, plus `side` (right, left, top, bottom) | A dialog along one edge. |
| `ui-dialog-close` | `label`, `variant` | Closes the dialog it is in; its own `on.click` actions run too. |
| `ui-dialog-footer` | | A right-aligned row of buttons. |
| `ui-popover` | `trigger`, `triggerVariant`, `side` (bottom, top, right, left), `align` (center, start, end), `id` | Native popover, placed next to its trigger and flipped when there's no room. |
| `ui-dropdown-menu` | same as `ui-popover` | Children: `ui-menu-item` (`label`, `shortcut`, `variant`, `href`, `disabled`; choosing it runs its `on.click` and closes the menu), `ui-menu-label`, `ui-menu-separator`. |
| `ui-tooltip` | `text`, `side` | Shown while its children are hovered or focused. CSS only. |
| `ui-hover-card` | `trigger`, `href` | Children are the card, shown while the trigger is hovered or focused. CSS only. |
| `ui-toaster` | `bind` | Shows the toast in `bind` (`{ title, description, variant }`) at the bottom right. |
| `ui-combobox` | see [Combobox](#combobox) | A searchable select in the style of react-select: multi-select, groups, async, creatable. |

As with the built-ins, a document can override any attribute with `attrs`, any style with `style`, and inner elements with `parts`. The kit only sets defaults.

`ui-radio-group` options, `ui-breadcrumb` items and `ui-pagination` are expanded on the server, so they can come from server data but not from `state`. Using state there is a render error, not a page that silently stops updating. For a choice list that changes in the browser, use `ui-select`, whose options can come from state.

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
- Escape closes the menu, then clears the text.
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

A toast is a state value. Set it to show one and to null to hide it, with a `wait` between the two to hide it automatically:

```yaml
- type: ui-button
  props: { label: Save }
  on:
    click:
      - { action: set_state, key: toast, value: { title: Saved, description: Your changes are live. } }
      - { action: wait, ms: 3000 }
      - { action: set_state, key: toast, value: null }
- { type: ui-toaster, props: { bind: toast } }
```

There is one toast at a time. A new toast replaces the current one, and the first toast's `wait` can still hide its replacement early.

## Browser support

The overlays rely on invoker commands (`command`/`commandfor`), `closedby`, the Popover API, CSS anchor positioning and exclusive `<details name>`. Chrome and Edge 135 and later support all of them, and the kit's browser tests run in Chrome. Check your other target browsers. Where a feature is missing:
- A popover still opens, but in the middle of the screen.
- Light dismiss falls back to Escape.
- Accordion items stop closing each other.

## Not included

Components that need focus management or keyboard navigation written in JavaScript are out of scope: combobox, command palette, date picker, data table with sorting, carousel, context menu, and menus with arrow-key navigation. Menu items are reachable with Tab.
