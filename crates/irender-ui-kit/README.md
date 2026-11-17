# irender-ui-kit

Ready-made components for [iRender](../../README.md) documents, styled after [shadcn/ui](https://ui.shadcn.com/). Every component is plain config, and nothing needs JavaScript beyond iRender's own runtime.

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
| `ui-skeleton` | `width`, `height`, `round` | A static placeholder (no pulse animation yet). |
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

## Not included

Components that need focus trapping, keyboard navigation or positioning logic (combobox, command palette, date picker, data table, carousel, menus, context menu) are out of scope for now. So are interactive overlays (dialog, popover, tooltip, tabs, accordion, toast), which are planned next.
