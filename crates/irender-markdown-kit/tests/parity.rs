//! `markdown.rs` (the server) and `markdown.js` (the browser's live preview) render
//! the same Markdown to the same HTML. Needs node; set NODE=/path/to/node.

use std::{io::Write, process::Command};

/// Markdown that exercises every rule, including the edges where the two could drift.
const CASES: &[&str] = &[
    "",
    "plain text",
    "# H1\n## H2 ##\n###### H6\n####### seven\n#nospace",
    "Para one\nstill one\n\nPara two",
    "trailing spaces  \nhard break\\\nbackslash break",
    "**bold** *em* ***both*** __b__ _e_ ~~del~~ ~single~",
    "snake_case_name and 2*3*4 and a * b * c",
    "**unclosed and *nested **inside** em*",
    "`code` ``with ` tick`` ` spaced ` `unclosed",
    "\\*not em\\* \\` \\[x\\] \\\\ \\a",
    "[link](https://example.com) [t](/rel \"Title\") [a](<with space.md>) [p](a(b)c)",
    "[nested [brackets]](#x) [**bold** label](http://x.y) [missing](",
    "![alt](img.png) ![bad](javascript:alert(1)) [js](JavaScript:x) [data](data:text/html,x)",
    "<https://a.b/c?d=e&f> <mailto:me@x.y> <not a link> <script>",
    "see https://a.b/c, then http://x.y/z). no:https://q.r",
    "- a\n- b\n- c",
    "* a\n+ b\n- c",
    "1. one\n2. two\n\n7) seven\n8) eight",
    "- a\n  - b\n    - c\n- d",
    "- a\n\n- b\n\n  para in b\n- c",
    "- [ ] todo\n- [x] done\n- [X] Done\n- [y] not",
    "1. first\n   continued\nlazy\n2. second",
    "> quote\n> > nested\n> - list\n\n> second",
    "```\nplain <code>\n```\n\n~~~python\ndef f(): pass\n~~~\n\n```js extra info\nlet x;\n```\n\n```\nunclosed",
    "  ```\n  indented\n    more\n  ```",
    "---\n***\n___\n- - -\n--",
    "| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n| x \\| y | `z` |",
    "| h |\n|---|\nno pipe ends it",
    "a | b\n--|--\n1 | 2",
    "| not | a table |\n| - x | - |",
    "Text\n# heading interrupts\n- list interrupts\n2. but not this",
    "& < > \" ' ©",
    "emoji 😀 **bold 😀** _and_ é",
    "\ttab indented\n\t- tab list",
    "line\r\nwindows\rold mac",
    "*a **b** c*",
    "**a *b* c**",
    "_intraword_underscore_ and *intra*word",
];

#[test]
fn rust_and_browser_render_the_same() {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".into());
    if Command::new(&node).arg("--version").output().is_err() {
        eprintln!("skipping markdown parity: `{node}` not found (set NODE=/path/to/node)");
        return;
    }
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/src/markdown.js");
    let runner = r#"
        const fs = require("fs");
        const { render } = require(process.argv[1]);
        const cases = JSON.parse(fs.readFileSync(0, "utf8"));
        process.stdout.write(JSON.stringify(cases.map(render)));
    "#;
    let mut child = Command::new(&node)
        .args(["-e", runner, script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(serde_json::to_string(CASES).unwrap().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let js: Vec<String> = serde_json::from_slice(&out.stdout).unwrap();
    let mut differ = Vec::new();
    for (case, browser) in CASES.iter().zip(&js) {
        let server = irender_markdown_kit::render(case);
        if &server != browser {
            differ.push(format!("{case:?}\n  server:  {server:?}\n  browser: {browser:?}"));
        }
    }
    assert!(differ.is_empty(), "{} of {} differ:\n{}", differ.len(), CASES.len(), differ.join("\n"));
}

/// Markdown still arriving, finished at its end (`markdown.js`'s `complete`).
#[test]
fn partial_markdown_is_completed() {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".into());
    if Command::new(&node).arg("--version").output().is_err() {
        eprintln!("skipping partial markdown: `{node}` not found (set NODE=/path/to/node)");
        return;
    }
    let cases = [
        // Marks are closed, innermost first; a mark with nothing after it yet is left out.
        ("Some **bold te", "Some **bold te**"),
        ("**bold _it", "**bold _it_**"),
        ("Text ***bo", "Text ***bo***"),
        ("~~stru", "~~stru~~"),
        ("Some **", "Some"),
        // Half of a closing mark.
        ("pick **Team*", "pick **Team**"),
        ("***both**", "***both***"),
        ("~~gone~", "~~gone~~"),
        ("***both*** and **mo", "***both*** and **mo**"),
        ("**done** and **mo", "**done** and **mo**"),
        // Code spans, and the marks inside them, which aren't marks.
        ("Use `co", "Use `co`"),
        ("Use `a **b", "Use `a **b`"),
        ("**run `npm i", "**run `npm i`**"),
        ("**run `", "**run**"),
        ("**`x` an", "**`x` an**"),
        // An open fence runs to the end; after it closes, the text is finished as usual.
        ("```js\nconst a **= 1", "```js\nconst a **= 1"),
        ("```js\nx\n```\nSome **bo", "```js\nx\n```\nSome **bo**"),
        // Only the last block is unfinished.
        ("Para **\n\nNext *it", "Para **\n\nNext *it*"),
        ("- one\n- **tw", "- one\n- **tw**"),
        // A link or image being written: its text, or nothing.
        ("See [the docs](https://exa", "See the docs"),
        ("See [the do", "See the do"),
        ("See [the docs]", "See the docs"),
        ("Look ![chart]", "Look"),
        ("Look ![chart](https://x", "Look"),
        // A table's header row, with however much of its `---` row.
        ("| a | b |", "| a | b |\n| --- | --- |"),
        ("| a | b |\n| --", "| a | b |\n| --- | --- |"),
        ("| a | b |\n| --- | --- |\n| 1 |", "| a | b |\n| --- | --- |\n| 1 |"),
        ("| St", "| St\n| --- |"),
        ("| Step | Ow", "| Step | Ow\n| --- | --- |"),
        ("|", ""),
        ("| a \\| b | c", "| a \\| b | c\n| --- | --- |"),
        ("| **St", "| **St**\n| --- |"),
        ("| a | b |\n| --- | --- |\n| 1 | **tw", "| a | b |\n| --- | --- |\n| 1 | **tw**"),
        // Not marks: arithmetic, names, escapes, and finished text.
        ("2*3", "2*3"),
        ("snake_case", "snake_case"),
        ("a * b", "a * b"),
        ("\\*not", "\\*not"),
        ("**done**", "**done**"),
        ("", ""),
    ];
    let runner = r#"
        const fs = require("fs");
        const { complete } = require(process.argv[1]);
        process.stdout.write(JSON.stringify(JSON.parse(fs.readFileSync(0, "utf8")).map(complete)));
    "#;
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/src/markdown.js");
    let mut child = Command::new(&node)
        .args(["-e", runner, script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let input: Vec<&str> = cases.iter().map(|c| c.0).collect();
    child.stdin.take().unwrap().write_all(serde_json::to_string(&input).unwrap().as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let got: Vec<String> = serde_json::from_slice(&out.stdout).unwrap();
    for ((text, want), got) in cases.iter().zip(&got) {
        assert_eq!(got, want, "{text:?}");
    }
}
