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
        file: PathBuf,
        /// Apply the untrusted policy (for user- or LLM-written documents).
        #[arg(long)]
        untrusted: bool,
        /// Treat CSS warnings as errors.
        #[arg(long)]
        strict_css: bool,
        /// Directory that `include` nodes read fragments from (`<name>.json` / `.yaml`).
        #[arg(long, value_name = "DIR")]
        fragments: Option<PathBuf>,
        /// Enable the `ui-*` components and the kit's theme (irender-ui-kit).
        #[arg(long)]
        ui_kit: bool,
        /// Enable `ui-chart` and `ui-sparkline` and the chart colours (irender-chart-kit).
        #[arg(long)]
        chart_kit: bool,
        /// Enable `ui-flow` and its node, edge, port and panel templates (irender-flow-kit).
        #[arg(long)]
        flow_kit: bool,
        #[command(flatten)]
        icons: IconArgs,
    },
    /// Render a document to HTML.
    Render {
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
        #[arg(long, value_name = "DIR")]
        fragments: Option<PathBuf>,
        /// Enable the `ui-*` components and the kit's theme (irender-ui-kit).
        #[arg(long)]
        ui_kit: bool,
        /// Enable `ui-chart` and `ui-sparkline` and the chart colours (irender-chart-kit).
        #[arg(long)]
        chart_kit: bool,
        /// Enable `ui-flow` and its node, edge, port and panel templates (irender-flow-kit).
        #[arg(long)]
        flow_kit: bool,
        #[command(flatten)]
        icons: IconArgs,
        /// Emit an embeddable fragment instead of a full HTML document.
        #[arg(long)]
        fragment: bool,
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
        #[command(flatten)]
        icons: IconArgs,
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
    icons: IconArgs,
}

fn engine(untrusted: bool, strict_css: bool, fragments: Option<PathBuf>, kits: &Kits) -> Result<Engine, String> {
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
    if kits.icons.enabled() {
        irender_icon_kit::register_with(&mut engine.registry, kits.icons.kit()?);
    }
    engine.strict_css = strict_css;
    engine.fragments = fragments.map(|dir| Arc::new(FragmentDir(dir)) as Arc<dyn Fragments>);
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
    if kits.ui {
        irender_ui_kit::apply_theme(&mut doc);
    }
    if kits.chart {
        irender_chart_kit::apply_theme(&mut doc);
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
        Command::Validate { file, untrusted, strict_css, fragments, ui_kit, chart_kit, flow_kit, icons } => {
            let kits = Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, icons };
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
            icons,
            fragment,
            minify,
            title,
            timings,
            parallel,
            repeat,
        } => {
            let kits = Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, icons };
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
                let page = engine.render(&ir, &data).map_err(|e| e.to_string())?;
                render_times.push(start.elapsed());
                let start = Instant::now();
                let html = if fragment { page.fragment() } else { page.document(&title) };
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
        Command::Components { ui_kit, chart_kit, flow_kit, icons } => {
            for c in engine(false, false, None, &Kits { ui: ui_kit, chart: chart_kit, flow: flow_kit, icons })?.registry.iter() {
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
