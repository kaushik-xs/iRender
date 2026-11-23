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
    ("settings", None),
    ("pricing", None),
    ("theme", None),
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

#[test]
fn twin_blocks_and_gallery_render() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    for doc in ["blocks/twin-01.yaml", "blocks/twin-02.yaml", "blocks/twin-03.yaml", "twin/gallery.yaml"] {
        let out = Command::new(env!("CARGO_BIN_EXE_irender")).args(["render", &format!("{root}/{doc}"), "--ui-kit", "--icon-kit", "--twin-kit"]).output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success() && stderr.trim().is_empty(), "{doc}:\n{stderr}");
        assert!(String::from_utf8_lossy(&out.stdout).contains("data-twin-canvas"), "{doc}");
    }
}

#[test]
fn map_block_renders_on_either_provider() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/blocks");
    let page = |doc: &str, more: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_irender"))
            .args(["render", doc, "--ui-kit", "--icon-kit", "--map-kit"])
            .args(more)
            .env_remove("GOOGLE_MAPS_API_KEY")
            .output()
            .unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let (ok, html, err) = page(&format!("{dir}/map-01.yaml"), &[]);
    assert!(ok && err.trim().is_empty(), "{err}");
    assert!(html.contains("tile.openstreetmap.org"));

    let google = std::env::temp_dir().join(format!("irender-map-{}.json", std::process::id()));
    std::fs::write(&google, r#"{ "version": 1, "root": { "type": "ui-map", "props": { "provider": "google" } } }"#).unwrap();
    let doc = google.to_str().unwrap();
    let without = page(doc, &[]);
    let with = page(doc, &["--google-maps-key", "a-key"]);
    let _ = std::fs::remove_file(&google);
    assert!(!without.0 && without.2.contains("needs an API key"), "{}", without.2);
    assert!(with.0 && with.1.contains("a-key"), "{}", with.2);
}
