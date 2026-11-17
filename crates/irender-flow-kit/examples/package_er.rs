//! Turns a package — the JSON files the builder edits: `manifest.json`, `tables.json`
//! and `columns/`, `relationships/` and `indexes/` with a file per table — into the
//! data `examples/flow/er.yaml` draws: a node per table and an edge per relationship.
//!
//!   cargo run -p irender-flow-kit --example package_er -- examples/flow/er-package > er.data.json
//!
//! This is the host's half of an ER diagram: working out which columns are keys and
//! naming the edges is code, and the document only draws what it is given. It
//! follows the builder's `buildGraph` (web/src/components/ide/er-diagram).

use serde_json::{Value as Json, json};
use std::{collections::HashSet, fs, path::Path, process::ExitCode};

fn read(path: &Path) -> Result<Json, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Every array in `dir/*.json`, joined, in file-name order. A missing folder is empty.
fn read_all(dir: &Path) -> Result<Vec<Json>, String> {
    let Ok(entries) = fs::read_dir(dir) else { return Ok(Vec::new()) };
    let mut files: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        match read(&f)? {
            Json::Array(items) => out.extend(items),
            _ => return Err(format!("{}: expected an array", f.display())),
        }
    }
    Ok(out)
}

fn text(v: &Json, key: &str) -> String {
    v.get(key).and_then(Json::as_str).unwrap_or_default().to_string()
}

fn build(dir: &Path) -> Result<Json, String> {
    let manifest = read(&dir.join("manifest.json"))?;
    let Json::Array(tables) = read(&dir.join("tables.json"))? else {
        return Err("tables.json: expected an array".into());
    };
    let columns = read_all(&dir.join("columns"))?;
    let relationships = read_all(&dir.join("relationships"))?;
    let indexes = read_all(&dir.join("indexes"))?;

    let table_ids: HashSet<String> = tables.iter().map(|t| text(t, "id")).collect();
    let foreign: HashSet<String> = relationships.iter().map(|r| text(r, "from_column_id")).collect();
    let column_name = |id: &str| columns.iter().find(|c| text(c, "id") == id).map(|c| text(c, "name"));

    let mut nodes = Vec::new();
    for t in &tables {
        let id = text(t, "id");
        let groups: Vec<Vec<String>> = t.get("unique").and_then(Json::as_array).into_iter().flatten()
            .filter_map(|g| g.as_array().map(|g| g.iter().filter_map(Json::as_str).map(String::from).collect()))
            .collect();
        // Single-column uniques, from the table's groups and its unique indexes, become badges.
        let mut unique: HashSet<String> = groups.iter().filter(|g| g.len() == 1).map(|g| g[0].clone()).collect();
        for idx in indexes.iter().filter(|i| text(i, "table_id") == id && i["unique"] == true) {
            if let Some([col]) = idx["columns"].as_array().map(Vec::as_slice) {
                // An index column is a name, or `{ name }`.
                unique.extend(col.as_str().map(String::from).or_else(|| col.get("name").and_then(Json::as_str).map(String::from)));
            }
        }
        let pk = text(t, "primary_key");
        let cols: Vec<Json> = columns
            .iter()
            .filter(|c| text(c, "table_id") == id)
            .map(|c| {
                let name = text(c, "name");
                json!({
                    "name": name,
                    "type": text(c, "type"),
                    "pk": name == pk,
                    "fk": foreign.contains(&text(c, "id")),
                    "unique": unique.contains(&name),
                    "nullable": c["nullable"] != false,
                    "default": c.pointer("/default/expression").cloned().unwrap_or(Json::Null),
                    "comment": c.get("comment").cloned().unwrap_or(Json::Null),
                })
            })
            .collect();
        let keys = cols.iter().filter(|c| c["pk"] == true || c["fk"] == true).count();
        nodes.push(json!({
            "id": id,
            "type": "table",
            "data": {
                "label": text(t, "name"),
                "comment": t.get("comment").cloned().unwrap_or(Json::Null),
                "columns": cols,
                "keyCount": keys,
                "uniques": groups.iter().filter(|g| g.len() > 1).map(|g| g.join(", ")).collect::<Vec<_>>(),
                "checks": t.get("check").cloned().unwrap_or(json!([])),
            }
        }));
    }

    let mut edges = Vec::new();
    let mut external = Vec::new();
    for r in &relationships {
        let from = text(r, "from_table_id");
        let to = text(r, "to_table_id");
        if !table_ids.contains(&from) {
            continue;
        }
        let from_col = column_name(&text(r, "from_column_id")).unwrap_or_default();
        let package = text(r, "to_package_id");
        let (target, label) = if table_ids.contains(&to) {
            let to_col = column_name(&text(r, "to_column_id")).unwrap_or_default();
            (to, format!("{from_col} → {to_col}"))
        } else if !package.is_empty() && !to.is_empty() {
            // Into another package: a placeholder for its table.
            let id = format!("ext:{package}:{to}");
            if !external.iter().any(|n: &Json| n["id"] == id) {
                external.push(json!({ "id": id, "type": "external", "data": { "label": to, "ref": format!("{package}.{to}") } }));
            }
            (id, format!("{from_col} → {}.{}", package, text(r, "to_column_id")))
        } else {
            continue;
        };
        edges.push(json!({ "id": text(r, "id"), "source": from, "target": target, "label": label }));
    }
    nodes.extend(external);

    let mut names: Vec<Json> = tables.iter().map(|t| json!({ "value": text(t, "id"), "label": text(t, "name") })).collect();
    names.sort_by_key(|n| text(n, "label"));
    Ok(json!({
        "package": { "name": text(&manifest, "name"), "version": text(&manifest, "version"), "description": text(&manifest, "description") },
        "nodes": nodes,
        "edges": edges,
        "tables": names,
    }))
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else {
        eprintln!("usage: package_er <package directory>");
        return ExitCode::FAILURE;
    };
    match build(Path::new(&dir)) {
        Ok(data) => {
            println!("{}", serde_json::to_string_pretty(&json!({ "er": data })).expect("JSON serializes"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
