---
name: irender
description: Write, edit, validate and render iRender documents, which are UI pages, dashboards, forms, charts and diagrams described in JSON or YAML and rendered to HTML by the `irender` CLI or the irender Rust crates. Use whenever the user asks to build, change, fix or convert a page, screen, component, form, dashboard, chart or flow diagram as an iRender, irender or "UI config" document, works on `.yaml` / `.json` files with `version: 1` and a `root:` node, mentions `ui-*` components (ui-card, ui-chart, ui-flow, ui-icon …), or wants a UI generated from a description, mockup, screenshot or existing HTML into iRender config.
---

# iRender documents

An iRender document is JSON or YAML: `version`, optional `theme`, `state` and `fragments`, and a `root` node tree. The `irender` CLI validates it, with every problem reported at its path, and renders it to HTML.

## Before writing

1. **Read the rules:** read [references/authoring.md](references/authoring.md) first. It covers the document and node shape, the small expression language, state and actions, YAML pitfalls and common mistakes. Its rules are strict: the expression language is not JavaScript, and every state key must be declared.
2. **Pick kits:** use the core built-ins only for minimal pages. For anything app-like, use `--ui-kit`. Add `--chart-kit` for charts, `--flow-kit` for node/edge diagrams, `--map-kit` for maps and `--icon-kit` for icons. Use the same flags on every command.
3. **Look up props:** never guess them.
   - **Components:** [references/components.md](references/components.md) lists every component, its props, their types and its parts. Search it for the component (`ui-chart:`) instead of reading it whole.
   - **Kit features:** for behaviour beyond props, read that kit's file: [ui-kit.md](references/ui-kit.md) (theme options, sidebar, combobox, dialogs), [chart-kit.md](references/chart-kit.md) (filters, grouping, drill-downs, live data), [flow-kit.md](references/flow-kit.md) (node templates, ports, editing), [map-kit.md](references/map-kit.md) (markers, popups, shapes, Google Maps) or [icon-kit.md](references/icon-kit.md).
   - **Core features:** [core.md](references/core.md) covers `include` parameters, list keys, theme options and the untrusted policy.
   - **Icon names:** they are in [references/icons.md](references/icons.md). Search it, since names are packed per line.
   - **Another language:** [references/labels.md](references/labels.md) lists the kits' interface text a document's `labels` can translate.
4. **Copy patterns:** for a whole page, start from the closest example in [examples/](examples/):
   - `counter.yaml`: state and actions
   - `wizard.yaml`: a multi-step form with validation
   - `login.yaml`: a ui-kit form card
   - `dashboard-01.yaml` with `dashboard-01.data.json`: sidebar, cards, chart, table, command palette

## Write → validate → render

```sh
irender validate page.yaml --ui-kit            # repeat until "ok"; fix every reported path
irender render page.yaml --ui-kit --data page.data.json -o page.html
```

- **Validate after every edit,** not just at the end. A fix can surface new problems at other paths.
- **Fix warnings too:** an unknown CSS property or value is usually a typo.
- **Server data:** if the document reads data that isn't in `state`, such as `{{ user.name }}`, write sample data to `page.data.json` and pass `--data`.
- **Untrusted documents:** for a document produced for an end user or another system, also validate with `--untrusted`.
- **Check it in a browser:** after rendering, open the HTML (or serve the folder when it fetches APIs) and check it. Clicks, `if`/`else` and lists rebuilt from state only show their behaviour in a browser.

### Getting the CLI

- **In the iRender repository:** `cargo run -q -p irender-cli -- <command>`.
- **Elsewhere:** `cargo install irender-cli` (binary `irender`).
- **From the CLI itself:** if the reference files here look out of date, `irender components --ui-kit --chart-kit --flow-kit --icon-kit`, `irender icons` and `irender schema` print the current truth.

## Style of the output

- **Format:** write YAML unless the user or the surrounding files use JSON. Use block style for nodes with children and flow style (`{ type: text, props: { value: Hi } }`) for leaves.
- **Components:** prefer kit components (`ui-card`, `ui-field`, `ui-button`, `ui-table` …) over hand-styled `element`s. Use `stack` for rows and columns. A CSS grid on an `element` is fine for page-level layout, as the dashboard example does.
- **Colours and spacing:** use theme tokens (`$color.muted`, `$space.md`) so dark mode and theme options work. Don't use literal colours.
- **Repetition:** factor repeated blocks into `fragments` with `include` + `with`, and render repeated data with `list`, not copy-paste.
- **Behaviour:** keep it declarative: `set_state`, `submit … into`, `if`/`else` and `list`. Hand anything else to the host with `call`, and tell the user which `call` names their app must handle.
- **Structure:** when converting HTML or a screenshot, map its structure onto kit components rather than reproducing every `div`.
