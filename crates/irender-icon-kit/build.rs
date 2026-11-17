//! Embeds the packs under `icons/`: one directory per pack, holding `_pack.txt`
//! (title, then description), one `<name>.svg` per icon, and `filled/` with the
//! filled style: `<name>.svg` files and `_same.txt` (icons drawn the same in both).

use std::{env, fmt::Write, fs, path::Path};

/// Packs that come first when a bare icon name is looked up; the rest follow
/// alphabetically.
const FIRST: &[&str] = &["core", "arrows", "status"];

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("icons");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut packs: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    packs.sort_by_key(|p| {
        let id = p.file_name().unwrap().to_string_lossy().into_owned();
        (FIRST.iter().position(|f| *f == id).unwrap_or(FIRST.len()), id)
    });

    let mut out = String::from("&[\n");
    for pack in packs {
        println!("cargo:rerun-if-changed={}", pack.display());
        let id = pack.file_name().unwrap().to_string_lossy().into_owned();
        let meta = fs::read_to_string(pack.join("_pack.txt")).unwrap_or_else(|_| panic!("{id}: missing _pack.txt"));
        let mut lines = meta.lines();
        let (title, description) = (lines.next().unwrap_or(&id).trim(), lines.next().unwrap_or("").trim());
        let filled = pack.join("filled");
        println!("cargo:rerun-if-changed={}", filled.display());
        let same = fs::read_to_string(filled.join("_same.txt")).unwrap_or_default();
        let same: Vec<_> = same.split_whitespace().collect();
        let _ = writeln!(out, "    BuiltinPack {{ id: {id:?}, title: {title:?}, description: {description:?},");
        let _ = writeln!(out, "        icons: {},", svgs(&pack));
        let _ = writeln!(out, "        filled: {},", svgs(&filled));
        let _ = writeln!(out, "        same: &{same:?} }},");
    }
    out.push(']');
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("packs.rs"), out).unwrap();
}

/// `&[(name, include_str!(path)), …]` for the `*.svg` files in `dir`.
fn svgs(dir: &Path) -> String {
    let mut files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "svg")).collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    let mut out = String::from("&[");
    for file in files {
        let name = file.file_stem().unwrap().to_string_lossy();
        let _ = write!(out, "({name:?}, include_str!({:?})), ", file.display().to_string());
    }
    out.push(']');
    out
}
