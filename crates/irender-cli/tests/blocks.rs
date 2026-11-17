//! The blocks under `examples/blocks/` render with the kits they use, with no
//! problems or warnings, and the blocks index lists every one of them.

use std::process::Command;

const BLOCKS: &[(&str, Option<&str>)] = &[
    ("dashboard-01", Some("dashboard-01.data.json")),
    ("sidebar-07", None),
    ("sidebar-03", None),
    ("login-03", None),
    ("login-04", None),
    ("signup", None),
];

fn render(doc: &str, data: Option<&str>) -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/blocks");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_irender"));
    cmd.args(["render", &format!("{dir}/{doc}"), "--ui-kit", "--chart-kit", "--icon-kit"]);
    if let Some(d) = data {
        cmd.args(["--data", &format!("{dir}/{d}")]);
    }
    let out = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{doc}:\n{stderr}");
    assert!(stderr.trim().is_empty(), "{doc} has warnings:\n{stderr}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn blocks_render_without_problems() {
    for (block, data) in BLOCKS {
        let html = render(&format!("{block}.yaml"), *data);
        assert!(html.contains("data-i-root"), "{block}");
    }
}

#[test]
fn blocks_index_lists_every_block() {
    let html = render("index.yaml", None);
    for (block, _) in BLOCKS {
        assert!(html.contains(&format!(r#"src="{block}.html""#)), "{block} is missing from the index");
    }
}
