use clap::{Parser, Subcommand};
use irender_core::{Document, Format, FragmentDir, Fragments, PropKind};
use irender_html::Engine;
use irender_icon_kit::{IconKit, Pack};
use serde_json::Value as Json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(name = "irender", version, about = "Build UI from configuration")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a document and report every problem with its path.
    Validate {
        /// The document (.json, .yaml or .yml), or `-` for JSON on stdin.
        file: PathBuf,
        /// Apply the untrusted policy (for user- or LLM-written documents).
        #[arg(long)]
        untrusted: bool,
        /// Treat CSS warnings as errors.
        #[arg(long)]
        strict_css: bool,
        /// Directory that `include` nodes read fragments from (`<name>.json` / `.yaml`).
        /// Repeatable: the first directory that has a fragment wins.
        #[arg(long, value_name = "DIR")]
        fragments: Vec<PathBuf>,
        /// Enable the `ui-*` components and the kit's theme (irender-ui-kit).
        #[arg(long)]
        ui_kit: bool,
        /// Enable `ui-chart` and `ui-sparkline` and the chart colours (irender-chart-kit).
        #[arg(long)]
        chart_kit: bool,
        /// Enable `ui-flow` and its node, edge, port and panel templates (irender-flow-kit).
        #[arg(long)]
        flow_kit: bool,
        /// Enable `ui-markdown` and `ui-markdown-editor` (irender-markdown-kit).
        #[arg(long)]
        markdown_kit: bool,
        /// Enable `ui-map`, its popups and panels (irender-map-kit).
        #[arg(long)]
        map_kit: bool,
        #[command(flatten)]
        icons: IconArgs,
    },
    /// Render a document to HTML.
    Render {
        /// The document (.json, .yaml or .yml), or `-` for JSON on stdin.
        file: PathBuf,
        /// Server data (JSON or YAML) available to expressions.
        #[arg(long)]
        data: Option<PathBuf>,
        /// Write to this file instead of stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long)]
        untrusted: bool,
        /// Treat CSS warnings as errors.
        #[arg(long)]
        strict_css: bool,
        /// Directory that `include` nodes read fragments from (`<name>.json` / `.yaml`).
        /// Repeatable: the first directory that has a fragment wins.
        #[arg(long, value_name = "DIR")]
        fragments: Vec<PathBuf>,
        /// Enable the `ui-*` components and the kit's theme (irender-ui-kit).
        #[arg(long)]
        ui_kit: bool,
        /// Enable `ui-chart` and `ui-sparkline` and the chart colours (irender-chart-kit).
        #[arg(long)]
        chart_kit: bool,
        /// Enable `ui-flow` and its node, edge, port and panel templates (irender-flow-kit).
        #[arg(long)]
        flow_kit: bool,
        /// Enable `ui-markdown` and `ui-markdown-editor` (irender-markdown-kit).
        #[arg(long)]
        markdown_kit: bool,
        /// Enable `ui-map`, its popups and panels (irender-map-kit).
        #[arg(long)]
        map_kit: bool,
        /// The Google Maps API key for maps with `provider: google` (default: the
        /// GOOGLE_MAPS_API_KEY environment variable). Pages carry it, so restrict it to your sites.
        #[arg(long, value_name = "KEY")]
        google_maps_key: Option<String>,
        #[command(flatten)]
        icons: IconArgs,
        /// Emit an embeddable fragment instead of a full HTML document.
        #[arg(long)]
        fragment: bool,
        /// Emit a piece of another iRender page, such as a chat message's UI: no theme
        /// variables and no runtime, for `window.__irender.mount`.
        #[arg(long, conflicts_with = "fragment")]
        embed: bool,
        #[arg(long)]
        minify: bool,
        #[arg(long, default_value = "iRender")]
        title: String,
        /// Print how long each phase took (to stderr).
        #[arg(long)]
        timings: bool,
        /// Render lists with at least this many items on several threads.
        #[arg(long, value_name = "MIN_ITEMS")]
        parallel: Option<usize>,
        /// Render this many times and report min / median / mean (implies --timings).
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
        repeat: u32,
    },
    /// Print the JSON Schema for documents.
    Schema,
    /// List the available components and their props.
    Components {
        /// Include the `ui-*` components from irender-ui-kit.
        #[arg(long)]
        ui_kit: bool,
        /// Include `ui-chart` and `ui-sparkline` from irender-chart-kit.
        #[arg(long)]
        chart_kit: bool,
        /// Enable `ui-flow` and its node, edge, port and panel templates (irender-flow-kit).
        #[arg(long)]
        flow_kit: bool,
        /// Enable `ui-markdown` and `ui-markdown-editor` (irender-markdown-kit).
        #[arg(long)]
        markdown_kit: bool,
        /// Enable `ui-map`, its popups and panels (irender-map-kit).
        #[arg(long)]
        map_kit: bool,
        #[command(flatten)]
        icons: IconArgs,
        /// Print JSON: `{ "components": [{ "name", "doc", "props": [{ "name", "type", "values", "required", "doc" }], "parts", "children" }] }`.
        #[arg(long)]
        json: bool,
    },
    /// List the icon packs and their icons (irender-icon-kit).
    Icons {
        /// Only this pack.
        pack: Option<String>,
        /// Print JSON: `{ "packs": [{ "id", "title", "description", "icons": [...] }] }`.
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        icons: IconArgs,
    },
}

#[derive(clap::Args, Clone, Default)]
struct IconArgs {
    /// Enable `ui-icon` with the built-in icon packs (irender-icon-kit).
    #[arg(long)]
    icon_kit: bool,
    /// Add an icon pack from a directory of SVG files, as `DIR` (the pack is named
    /// after the directory) or `NAME=DIR`. Repeatable; implies --icon-kit.
    #[arg(long = "icons", value_name = "[NAME=]DIR")]
    packs: Vec<String>,
}

impl IconArgs {
    fn enabled(&self) -> bool {
        self.icon_kit || !self.packs.is_empty()
    }

    /// The built-in packs, then the ones from `--icons`.
    fn kit(&self) -> Result<IconKit, String> {
        let mut kit = IconKit::new();
        for arg in &self.packs {
            let (name, dir) = match arg.split_once('=') {
                Some((name, dir)) => (name.to_string(), PathBuf::from(dir)),
                None => {
                    let dir = PathBuf::from(arg);
                    let name = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                    (name, dir)
                }
            };
            kit = kit.with(Pack::from_dir(&name, &dir).map_err(|e| format!("--icons {arg}: {e}"))?);
        }
        Ok(kit)
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &Path) -> Result<(String, Format), String> {
    // `-` is a JSON document on stdin, e.g. a chat reply's UI from a server.
    if path == Path::new("-") {
        let mut src = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut src).map_err(|e| format!("stdin: {e}"))?;
        return Ok((src, Format::Json));
    }
    let format = Format::from_path(path)
        .ok_or_else(|| format!("{}: unsupported file type (use .json, .yaml or .yml)", path.display()))?;
    let src = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((src, format))
}

/// The optional component kits a command enables.
#[derive(Clone)]
struct Kits {
    ui: bool,
    chart: bool,
    flow: bool,
    markdown: bool,
    map: bool,
    google_maps_key: Option<String>,
    icons: IconArgs,
}

impl Kits {
    /// The kits' theme tokens, so their styles resolve.
    fn apply_themes(&self, doc: &mut Document) {
        if self.ui {
            irender_ui_kit::apply_theme(doc);
        }
        if self.chart {
            irender_chart_kit::apply_theme(doc);
        }
    }
}

/// Fragments from several directories, the first that has one winning.
struct Dirs(Vec<FragmentDir>);

impl Fragments for Dirs {
    fn fragment(&self, name: &str) -> Result<Option<irender_core::NodeSpec>, String> {
        for dir in &self.0 {
            if let Some(f) = dir.fragment(name)? {
                return Ok(Some(f));
            }
        }
        Ok(None)
    }
}

fn engine(untrusted: bool, strict_css: bool, fragments: Vec<PathBuf>, kits: &Kits) -> Result<Engine, String> {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    if kits.ui {
        irender_ui_kit::register(&mut engine.registry);
    }
    if kits.chart {
        irender_chart_kit::register(&mut engine.registry);
    }
    if kits.flow {
        irender_flow_kit::register(&mut engine.registry);
    }
    if kits.markdown {
        irender_markdown_kit::register(&mut engine.registry);
    }
    if kits.map {
        let google_key = kits.google_maps_key.clone().or_else(|| std::env::var("GOOGLE_MAPS_API_KEY").ok()).filter(|k| !k.is_empty());
        irender_map_kit::register_with(&mut engine.registry, irender_map_kit::MapKit { google_key, ..Default::default() });
    }
    if kits.icons.enabled() {
        irender_icon_kit::register_with(&mut engine.registry, kits.icons.kit()?);
    }
    engine.strict_css = strict_css;
    if !fragments.is_empty() {
        engine.fragments = Some(Arc::new(Dirs(fragments.into_iter().map(FragmentDir).collect())) as Arc<dyn Fragments>);
    }
    Ok(engine)
}

struct Loaded {
    ir: irender_core::Ir,
    parse: Duration,
    resolve: Duration,
}

fn load(engine: &Engine, file: &Path, kits: &Kits) -> Result<Loaded, String> {
    let (src, format) = read(file)?;
    let problems = |d: irender_core::Diagnostics| {
        let n = d.len();
        format!("{}: {n} problem{}\n{d}", file.display(), if n == 1 { "" } else { "s" })
    };
    let start = Instant::now();
    let mut doc = irender_core::parse(&src, format).map_err(problems)?;
    kits.apply_themes(&mut doc);
    if kits.ui {
        // Messages whose `ui` is a document or node show it, rendered now.
        irender_ui_kit::render_message_ui(&mut doc, |ui| embed(engine, kits, ui))
            .map_err(|e| format!("{}: {e}", file.display()))?;
    }
    let parse = start.elapsed();
    let start = Instant::now();
    let ir = engine.resolve(&doc).map_err(problems)?;
    let resolve = start.elapsed();
    for w in ir.warnings.iter() {
        eprintln!("{}: {w}", file.display());
    }
    Ok(Loaded { ir, parse, resolve })
}

/// A message's UI as a piece of the page, rendered with the same kits.
fn embed(engine: &Engine, kits: &Kits, ui: &Json) -> Result<String, String> {
    irender_ui_kit::render_ui(engine, ui, |doc| kits.apply_themes(doc))
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
}

fn kb(bytes: usize) -> String {
    format!("{:.1} KB", bytes as f64 / 1024.0)
}

fn summary(times: &mut [Duration]) -> String {
    times.sort();
    if times.len() == 1 {
        return ms(times[0]);
    }
    let mean = times.iter().sum::<Duration>() / times.len() as u32;
    format!("min {}  median {}  mean {}", ms(times[0]), ms(times[times.len() / 2]), ms(mean))
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Validate { file, untrusted, strict_css, fragments, ui_kit, chart_kit, flow_kit, markdown_kit, map_kit, icons } => {
            let kits = Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, markdown: markdown_kit, map: map_kit, google_maps_key: None, icons };
            load(&engine(untrusted, strict_css, fragments, &kits)?, &file, &kits)?;
            println!("{}: ok", file.display());
        }
        Command::Render {
            file,
            data,
            out,
            untrusted,
            strict_css,
            fragments,
            ui_kit,
            chart_kit,
            flow_kit,
            markdown_kit,
            map_kit,
            google_maps_key,
            icons,
            fragment,
            embed,
            minify,
            title,
            timings,
            parallel,
            repeat,
        } => {
            let kits = Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, markdown: markdown_kit, map: map_kit, google_maps_key, icons };
            let mut engine = engine(untrusted, strict_css, fragments, &kits)?;
            engine.minify = minify;
            engine.parallel_min_items = parallel;
            let Loaded { ir, parse, resolve } = load(&engine, &file, &kits)?;
            let data = match data {
                Some(path) => {
                    let (src, format) = read(&path)?;
                    irender_core::parse_as::<Json>(&src, format).map_err(|d| format!("{}: {d}", path.display()))?
                }
                None => Json::Object(Default::default()),
            };
            let mut render_times = Vec::new();
            let mut serialize_times = Vec::new();
            let mut rendered = None;
            for _ in 0..repeat {
                let start = Instant::now();
                let page = if embed { engine.render_embedded(&ir, &data) } else { engine.render(&ir, &data) };
                let page = page.map_err(|e| e.to_string())?;
                render_times.push(start.elapsed());
                let start = Instant::now();
                let html = if embed {
                    page.embedded()
                } else if fragment {
                    page.fragment()
                } else {
                    page.document(&title)
                };
                serialize_times.push(start.elapsed());
                rendered = Some((page, html));
            }
            let (page, html) = rendered.expect("repeat is at least 1");
            if timings || repeat > 1 {
                let css_note = if cfg!(debug_assertions) { "  (debug build: use --release)" } else { "" };
                eprintln!("{}: render ×{repeat}{css_note}", file.display());
                eprintln!("  parse      {}", ms(parse));
                eprintln!("  resolve    {}  (incl. CSS validation)", ms(resolve));
                eprintln!("  render     {}", summary(&mut render_times));
                eprintln!("  serialize  {}", summary(&mut serialize_times));
                eprintln!(
                    "  output     {} html ({} css), {} elements{}",
                    kb(html.len()),
                    kb(page.css.len()),
                    page.body.match_indices('<').filter(|(i, _)| !page.body[i + 1..].starts_with('/')).count(),
                    if page.interactive { ", interactive" } else { "" },
                );
            }
            match out {
                Some(path) => fs::write(&path, html).map_err(|e| format!("{}: {e}", path.display()))?,
                None => print!("{html}"),
            }
        }
        Command::Schema => {
            let schema = schemars::schema_for!(Document);
            println!("{}", serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())?);
        }
        Command::Components { ui_kit, chart_kit, flow_kit, markdown_kit, map_kit, icons, json } => {
            let engine = engine(false, false, Vec::new(), &Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, markdown: markdown_kit, map: map_kit, google_maps_key: None, icons })?;
            if json {
                let components: Vec<Json> = engine
                    .registry
                    .iter()
                    .map(|c| {
                        let info = c.info();
                        let props: Vec<Json> = info
                            .props
                            .iter()
                            .map(|p| {
                                let (kind, values) = match p.kind {
                                    PropKind::Enum(v) => ("enum".to_string(), Some(v)),
                                    k => (format!("{k:?}").to_lowercase(), None),
                                };
                                serde_json::json!({ "name": p.name, "type": kind, "values": values, "required": p.required, "doc": p.doc })
                            })
                            .collect();
                        serde_json::json!({ "name": c.name(), "doc": info.doc, "props": props, "parts": info.parts, "children": info.children })
                    })
                    .collect();
                println!("{}", serde_json::json!({ "components": components }));
                return Ok(());
            }
            for c in engine.registry.iter() {
                let info = c.info();
                println!("{}: {}", c.name(), info.doc);
                for p in info.props {
                    let kind = match p.kind {
                        PropKind::Enum(_) => p.kind.describe(),
                        k => format!("{k:?}").to_lowercase(),
                    };
                    let required = if p.required { " (required)" } else { "" };
                    println!("  {:<12} {kind}{required}  {}", p.name, p.doc);
                }
                if !info.parts.is_empty() {
                    println!("  parts: {}", info.parts.join(", "));
                }
                println!("  children: {}", if info.children { "yes" } else { "no" });
            }
        }
        Command::Icons { pack, json, icons } => {
            let kit = icons.kit()?;
            let packs: Vec<_> = match &pack {
                Some(id) => vec![kit.pack(id).ok_or_else(|| format!("no icon pack `{id}`"))?],
                None => kit.packs().collect(),
            };
            if json {
                let packs: Vec<Json> = packs
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id(), "title": p.title(), "description": p.description(),
                            "icons": p.names().collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                println!("{}", serde_json::json!({ "packs": packs }));
            } else {
                for p in packs {
                    println!("{} ({}, {} icons): {}", p.id(), p.title(), p.len(), p.description());
                    println!("  {}", p.names().collect::<Vec<_>>().join(" "));
                }
            }
        }
    }
    Ok(())
}
