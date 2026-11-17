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
