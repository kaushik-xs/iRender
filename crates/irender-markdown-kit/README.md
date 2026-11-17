# irender-markdown-kit

Markdown for [iRender](../../README.md) documents. `ui-markdown` shows Markdown as formatted text, and `ui-markdown-editor` is for writing it: a text area with a formatting toolbar, keyboard shortcuts and a live preview. The look comes from the theme's tokens, so it matches [irender-ui-kit](../irender-ui-kit/README.md) and follows its theme options.

```rust
let mut engine = irender_html::Engine::new();
irender_ui_kit::register(&mut engine.registry);      // optional
irender_markdown_kit::register(&mut engine.registry);
```

From the CLI, add `--markdown-kit` to `validate`, `render` or `components`:

```sh
cargo run -p irender-cli -- render examples/blocks/markdown.yaml --ui-kit --markdown-kit -o markdown.html
```

## ui-markdown

```yaml
- { type: ui-markdown, props: { text: "**Hello**, _world_." } }
- { type: ui-markdown, props: { text: "{{ post }}" } }   # follows state
```

The server renders the Markdown into the page, so it shows without a script. When `text` uses state, the kit adds its renderer (about 5 KB gzipped; the editor adds 3 KB more) and the text re-renders when the state changes.

| Prop | |
|---|---|
| `text` | The Markdown (required). |
| `tag` | The HTML tag (default `div`). |

## ui-markdown-editor

```yaml
- { type: ui-markdown-editor, props: { bind: post, maxLength: 4000, placeholder: Start writing… } }
- { type: ui-markdown-editor, props: { bind: post, mode: split } }
```

- **Toolbar:** heading, bold, italic, strikethrough, quote, code, link, and bulleted, numbered and task lists. A button wraps the selection, or a placeholder word to type over. Heading, quote and the lists apply to every selected line, and pressing the button again takes them off.
- **Shortcuts:** ⌘B bold, ⌘I italic, ⌘⇧X strikethrough, ⌘E code, ⌘K link (Ctrl on Windows and Linux).
- **Lists:** Enter continues a list, a numbered list or a quote, and Enter on an empty item ends it. Tab and Shift+Tab indent list lines; elsewhere Tab moves focus as usual.
- **Undo:** edits go through the browser's own editing, so ⌘Z undoes them.
- **Preview:** Write and Preview tabs (`mode: tabs`, the default), or the preview beside the text (`mode: split`).

| Prop | |
|---|---|
| `bind` | The state key that holds the Markdown (required). |
| `mode` | `tabs` (default) or `split`. |
| `placeholder` | Shown while it's empty. |
| `label` | The text area's name for screen readers (default `Markdown`). |
| `rows` | Visible lines (default 10). |
| `maxLength` | A limit, shown as a count under the editor. |
| `toolbar` | `false` hides the toolbar. |
| `name`, `disabled` | As on `textarea`. |

Parts: `toolbar`, `input`, `preview` and `footer`.

## The Markdown it supports

CommonMark's common parts, plus GitHub's tables, task lists, strikethrough and bare links:

- headings (`#` to `######`), paragraphs, hard line breaks (two trailing spaces or `\`);
- `**bold**`, `*italic*` / `_italic_`, `***both***`, `~~strikethrough~~`, `` `code` ``;
- `[links](url "title")`, `![images](url)`, `<https://autolinks>`, and bare `https://` links;
- bulleted, numbered (`1.` or `1)`, with any start number), nested and task lists;
- block quotes, fenced code blocks (with a `language-*` class), rules and tables with alignment;
- backslash escapes.

Not included: setext headings (underlined with `===`), indented code blocks, reference links, footnotes and HTML.

## Safe for text from anyone

Markdown from users, a chat or an API can be shown as it is:

- Every piece of text is escaped, so HTML in the Markdown shows as text.
- A link or image keeps its URL only if it is `http:`, `https:`, `mailto:`, a `#fragment` or relative. Any other (`javascript:`, `data:` …) renders as its text alone.

## One renderer, twice

The server renders with Rust (`irender_markdown_kit::render`), and the browser with `markdown.js`, the same rules written line for line in JavaScript. A parity test runs both on the same Markdown and requires identical HTML, so the preview shows what the page will.

## Tests

```sh
cargo test -p irender-markdown-kit                          # unit and parity (parity needs node)
cargo test -p irender-markdown-kit --test browser -- --ignored   # the editor in headless Chrome
```
