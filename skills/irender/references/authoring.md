# Writing iRender documents

iRender renders UI from a JSON or YAML document. This page is the compact set of rules for writing one that validates on the first try. Component props are in `components.md`; each kit's details are in its own file.

## Document

```yaml
version: 1
lang: es                                                # optional: the page's language
labels: { "Rows per page": "Filas por página" }         # optional: the kits' interface text, by its English
theme: { tokens: { color: { primary: "#4f46e5" } } }   # optional
state: { count: 0, tab: overview, results: [] }         # optional: browser state, every key you read or set
fragments: { card: { type: stack, children: [] } }      # optional: reusable subtrees
root: { type: stack, children: [] }                     # required: one node
```

Only these seven top-level keys exist. Unknown keys are errors. For a page in another language, set `lang` and give `labels` for the kits' fixed words: [labels.md](labels.md) lists every key, and `{ one, other }` gives a count's plural forms.

## Node

```yaml
type: button                 # required: a component name
props: { label: Save }       # the component's props (see components.md); typed and validated
attrs: { id: save, aria-label: Save }   # any HTML attribute; wins over the component's own
class: [cta]                 # extra class names
style: { padding: 12px 20px, ":hover": { opacity: 0.9 }, "@md": { padding: 16 } }
parts: { label: { style: { font-weight: 600 } } }   # style a component's inner elements (names in components.md)
on: { click: { action: set_state, key: count, value: "{{ count + 1 }}" } }
children: []
```

Only these eight keys exist on a node. Components that say `children: no` take none.

## Built-in components (always available)

`element` (any tag via `props.tag`), `text`, `stack`, `button`, `input`, `select`, `textarea`, `list`, `if`, plus `else` and `include`, which the loader resolves.

| kit | flag | adds |
|---|---|---|
| ui-kit | `--ui-kit` | `ui-*` components in the style of shadcn/ui, and a theme with dark mode and theme options |
| chart-kit | `--chart-kit` | `ui-chart`, `ui-sparkline`, `ui-live-data` (Apache ECharts) |
| flow-kit | `--flow-kit` | `ui-flow` and its `ui-flow-*` templates: node-and-edge diagrams |
| icon-kit | `--icon-kit` | `ui-icon`, and icon names in `icon` props of ui-kit components |

Prefer ui-kit components over hand-styled `element`s whenever a kit is in use. They are accessible, themed and dark-mode aware.

## Expressions

Any string may contain `{{ … }}`.

- **Grammar:** paths (`user.name`, `items[0]`, `row['first-name']`, `cart.length`), string, number, boolean and `null` literals, arithmetic `+ - * / %`, comparisons `== != < <= > >=`, `&& || !`, `a ? b : c` and parentheses.
- **Functions:** only `range(n)` / `range(start, end)`, `includes(list, value)` and `toggle(list, value)`. There are no others: no `map`, `filter`, `join`, `toUpperCase`, `Math.*`, template literals or arrow functions.
- **No array or object literals inside `{{ }}`:** write the structure in YAML or JSON instead, with expressions in its leaves: `value: { title: Saved, description: "Hi {{ name }}", tags: ["{{ tag }}"] }`. An action's object or list is filled in when the action runs.
- **Paths only:** `.field` and `[i]` follow paths, not call results (`range(n).length` is invalid).
- **Live or server-side:** a path whose first name is a `state` key updates live in the browser. Any other path is filled from server `data` at render time. Inside a `list`, `item` and `index` (or the `as` / `index` names) are in scope.
- **Exact values:** a string that is only one expression keeps the value's type (`"{{ count }}"` is a number). Mixed text makes a string: `"Buy {{ qty }}"`.

## State and actions

- **Declare state:** every state key an action sets, a control `bind`s or an expression reads live must be declared in `state` with its starting value. Undeclared keys are errors.
- **Top-level keys only:** `bind` and `set_state` `key` take a top-level key, never a path like `draft.customer`. A form needs one flat key per field (`customer`, `product`), which `submit` then gathers in its `body`.
- **Event names:** `on` maps a DOM event (`click`, `input`, `change`, `submit`, `keydown`, …) or `mount` (when the element first appears) to one action or a list. A list runs in order, and `submit` waits for its response.

| action | fields |
|---|---|
| `set_state` | `key`, `value` (usually an expression) |
| `navigate` | `url` |
| `submit` | `endpoint`, `method` (default POST), `body` (object; a query string for GET), `into` (state key for the JSON response) |
| `call` | `name`, `args`: dispatches an `irender:call` DOM event for the host app |
| `wait` | `ms` (at most 60000) |

There is no other action and no inline JavaScript. Anything custom goes through `call`.

- **What gets evaluated:** in actions, every `{{ }}` is, however deep: a `set_state` value, each field of `submit`'s `body` (`body: { customer: "{{ customer }}" }`) and `call`'s `args`, objects and lists in them included. In component props, the component decides (a column's `{{ row.x }}` is filled per row).
- **`submit` failures:** they don't stop the chain. A non-2xx response is still stored in `into`, and a network error fires an `irender:error` DOM event. Either way, the following actions run. Success fires `irender:submitted` with `{ ok, status, data }`. To act only on success, show the outcome with an `if` on the response stored in `into` (e.g. `{{ saved.id }}`), or let the host handle the request through `call`.
- **Forms:** `on.submit` on an `element` with `tag: form` whose actions include `submit` prevents the browser's own navigation.

## Form controls

`input`, `select` and `textarea`, and their `ui-*` versions, take `bind: <state key>` for two-way binding. A checkbox binds a boolean. A radio needs a `value`. `select` / `ui-select` `options` are values or `{ value, label, disabled }`.

## Conditionals and lists

```yaml
- { type: if, props: { when: "{{ step == 1 }}" }, children: [ … ] }
- { type: else, props: { when: "{{ step == 2 }}" }, children: [ … ] }   # else-if; must directly follow an if/else
- { type: else, children: [ … ] }

- type: list
  props: { items: "{{ results }}", as: r, key: "{{ r.id }}", tag: ul }   # key keeps DOM (inputs, focus) across updates
  children: [ { type: text, props: { tag: li, value: "{{ r.title }}" } } ]
```

- `if` adds no element of its own. `list` wraps its items in a `div` unless you give it a `tag` such as `ul` or `tbody`.
- **Mixed conditions:** an `if` inside a server-rendered list may mix the item and state, as in `"{{ filter == null || row.status == filter }}"`. The item part is filled in per row on the server, and the state part stays live.
- Repeat something n times: `items: "{{ range(n) }}"`.
- Keep a set of choices in state: `includes(tags, 'x')` and `toggle(tags, 'x')`.

## Fragments

```yaml
fragments:
  stat:
    type: stack
    children: [ { type: text, props: { value: "{{ label }}" } }, { type: text, props: { value: "{{ value }}" } } ]
root:
  type: stack
  children:
    - { type: include, props: { name: stat, with: { label: Revenue, value: "{{ totals.revenue }}" } } }
```

`name` must be a fixed string. Use fragments for anything repeated more than twice with different values.

## Styles and theme

- **Properties:** `style` accepts any CSS property, in kebab-case. Numbers get `px` where it makes sense.
- **Tokens:** `$group.name` compiles to `var(--group-name)`, and an unknown token is an error. Default tokens:
  - `color.primary`, `primary-hover`, `on-primary`, `surface`, `text`, `muted`, `border`
  - `radius.sm` / `md` / `lg`
  - `space.xs` / `sm` / `md` / `lg` / `xl`
  - `font.body`, `font.size.sm` / `md` / `lg`
- **ui-kit tokens:** the kit adds `color.secondary`, `accent`, `subtle`, `destructive`, `ring` and others. Add your own under `theme.tokens`.
- **Nested blocks:** `":hover"`, `"&.active"`, `"& > li"`, `"@sm"` / `"@md"` / `"@lg"` / `"@xl"` (min-width breakpoints), `"@dark"`, `"@print"`, `"@motion-safe"`, `"@media (…)"`.
- **Dynamic values:** `color: "{{ ok ? 'green' : 'crimson' }}"` updates live. `null` leaves the property unset.
- **Dark mode:** use tokens, not literal colours, so dark mode works. `theme.modes.dark` overrides tokens.

## YAML pitfalls

- **Expressions:** quote any value that starts with `{{` or `{`, or YAML reads it as a mapping. Write `value: "{{ count }}"`, never `value: {{ count }}`.
- **Hex colours:** quote them (`"#4f46e5"`), or `#` starts a comment.
- **Style keys:** quote any key starting with `:`, `@` or `&`, such as `":hover"`, `"@md"` and `"&.active"`.
- **Flow maps:** they are fine for short nodes (`{ type: text, props: { value: Hi } }`). Use block style once a node has children.

## Workflow

1. **Kits:** decide which ones the page needs, and pass the same flags to every command.
2. **Props:** look up each component's props in `components.md` (or run `irender components --ui-kit …`). Don't guess prop names.
3. **Write:** write the document. Put sample server data in a separate `*.data.json` if it reads any.
4. **Validate:** run `irender validate page.yaml --ui-kit …`. It lists every problem with its path, such as `root.children[1].on.click[0].key: 'n' is not declared in 'state'`. Fix them all and repeat until it passes. Treat warnings (unknown CSS) as worth fixing.
5. **Render:** run `irender render page.yaml --data page.data.json --ui-kit … -o page.html`, then open it in a browser to check it.
6. **Untrusted input:** for documents from users or other models, validate and render with `--untrusted`.

## Common mistakes

- **Undeclared state:** setting or binding a key missing from `state`.
- **Unsupported expressions:** JavaScript-only syntax such as `.map(`, `.toFixed(`, `${}`, `===`, `=>`, or `[…]` and `{…}` literals.
- **Missing kit flag:** a `ui-*` component without `--ui-kit` is "unknown component type".
- **Wrong place for a value:** a prop under `attrs`, or an attribute under `props`. Component props go in `props`, and raw HTML attributes such as `id`, `aria-*` and `href` on an `element` go in `attrs`.
- **Children where none are allowed:** giving children to `text`, `input`, `ui-input`, `ui-badge` or other components marked `children: no`. Use `element` or `stack` to group.
- **Bad `else`:** an `else` that doesn't directly follow an `if`, or one that follows a plain `else` (only `else` with `when` can be followed).
- **Layout by hand:** hand-writing flex layout with `element` instead of `stack` (`direction`, `gap`, `align`, `justify`, `wrap`).
- **Literal colours:** they break dark mode. Use `$color.*`.
- **Toggle groups reset to `null`:** pressing a selected `ui-toggle-group` option again sets its key to `null`, so a filter must treat `null` as "all".
- **Dialogs that stay open:** a submit button inside a `ui-dialog` doesn't close it. Use a `ui-dialog-close` whose `on.click` runs the `submit` (and any toast), so it sends and closes in one click.
- **Missing table components:** `ui-table` has no row or cell components. Its children are `element`s with `tag: thead`, `tbody`, `tr`, `th` and `td`, or a `list` with `tag: tbody`.
