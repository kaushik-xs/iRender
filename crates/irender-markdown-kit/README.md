# irender-markdown-kit

Markdown for [iRender](../../README.md) documents. `ui-markdown` shows Markdown as formatted text, and `ui-markdown-editor` is for writing it: by default the text is edited formatted, as readers will see it, so anyone can use it, and it's kept as Markdown. A Markdown tab, or `mode: tabs` / `split`, edits the Markdown itself with a live preview. The look comes from the theme's tokens, so it matches [irender-ui-kit](../irender-ui-kit/README.md) and follows its theme options.

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

The server renders the Markdown into the page, so it shows without a script. When `text` uses state, the kit adds its renderer (about 5 KB gzipped; the editor adds 9 KB more) and the text re-renders when the state changes.

| Prop | |
|---|---|
| `text` | The Markdown (required). |
| `tag` | The HTML tag (default `div`). |

## ui-markdown-editor

```yaml
- { type: ui-markdown-editor, props: { bind: post, maxLength: 4000, placeholder: Start writing… } }
- { type: ui-markdown-editor, props: { bind: post, mode: split } }
```

### Rich text (`mode: rich`, the default)

The text shows formatted, the way `ui-markdown` and readers see it, and is edited there; nobody needs to know Markdown. Each edit writes Markdown to `bind`.

- **Toolbar and shortcuts:** the same buttons and keys as below format the selection; a button lights up where the caret is in that format, and pressing it again takes it off. The link button opens a bar under the toolbar for the address (a bare `example.com` becomes `https://example.com`), with Remove link when the caret is on one.
- **Formatting as you type:** start a line with `#` to `######`, `-`, `1.`, `>`, `[ ]` or ` ``` ` and a space, and it becomes a heading, a list, a numbered list, a quote, a task or a code block.
- **Tasks:** tick a task's box with a click.
- **Lists, quotes and code:** Enter on an empty list item, quote line or last code line leaves it. Tab and Shift+Tab nest list items.
- **Paste:** a page or document keeps its headings, lists, links, bold, italic, code and tables, and drops the rest (colours, fonts, scripts). Pasted plain text is read as Markdown.
- **Markdown tab:** shows the Markdown in a text area, with everything below.

Markdown it can't show the same way is written back in a standard form on the first edit: `*a*` becomes `_a_`, lines of one paragraph are joined, and a table's delimiter row becomes `| --- |`. Text that would read as Markdown (`*`, `_`, a line starting `1.`) is escaped with `\`.

### Markdown (`mode: tabs` or `split`, and the Markdown tab)

- **Toolbar:** heading, bold, italic, strikethrough, quote, code, link, and bulleted, numbered and task lists. A button wraps the selection, or a placeholder word to type over. Heading, quote and the lists apply to every selected line, and pressing the button again takes them off.
- **Shortcuts:** ⌘B bold, ⌘I italic, ⌘⇧X strikethrough, ⌘E code, ⌘K link (Ctrl on Windows and Linux).
- **Lists:** Enter continues a list, a numbered list or a quote, and Enter on an empty item ends it. Tab and Shift+Tab indent list lines; elsewhere Tab moves focus as usual.
- **Undo:** edits go through the browser's own editing, so ⌘Z undoes them.
- **Preview:** Write and Preview tabs (`mode: tabs`), or the preview beside the text (`mode: split`).

| Prop | |
|---|---|
| `bind` | The state key that holds the Markdown (required). |
| `mode` | `rich` (default), `tabs` or `split`. |
| `placeholder` | Shown while it's empty. |
| `label` | The editor's name for screen readers (default `Text` in rich text, `Markdown` in the text area). |
| `rows` | Visible lines (default 10); in rich text, its least height. |
| `maxLength` | A limit, shown as a count under the editor. Rich text can go over it (the count turns red); the text area stops at it. |
| `toolbar` | `false` hides the toolbar. |
| `name`, `disabled` | As on `textarea`. |

Parts: `toolbar`, `rich` (the rich text), `input` (the text area), `preview` and `footer`.

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
