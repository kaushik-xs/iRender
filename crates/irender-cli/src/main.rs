use clap::{Parser, Subcommand};
use irender_core::{Document, Format, FragmentDir, Fragments, PropKind};
use irender_html::Engine;
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
    Components,
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

fn engine(untrusted: bool, strict_css: bool, fragments: Option<PathBuf>) -> Engine {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    engine.strict_css = strict_css;
    engine.fragments = fragments.map(|dir| Arc::new(FragmentDir(dir)) as Arc<dyn Fragments>);
    engine
}

struct Loaded {
    ir: irender_core::Ir,
    parse: Duration,
    resolve: Duration,
}

fn load(engine: &Engine, file: &Path) -> Result<Loaded, String> {
    let (src, format) = read(file)?;
    let problems = |d: irender_core::Diagnostics| {
        let n = d.len();
        format!("{}: {n} problem{}\n{d}", file.display(), if n == 1 { "" } else { "s" })
    };
    let start = Instant::now();
    let doc = irender_core::parse(&src, format).map_err(problems)?;
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
        Command::Validate { file, untrusted, strict_css, fragments } => {
            load(&engine(untrusted, strict_css, fragments), &file)?;
            println!("{}: ok", file.display());
        }
        Command::Render {
            file,
            data,
            out,
            untrusted,
            strict_css,
            fragments,
            fragment,
            minify,
            title,
            timings,
            parallel,
            repeat,
        } => {
            let mut engine = engine(untrusted, strict_css, fragments);
            engine.minify = minify;
            engine.parallel_min_items = parallel;
            let Loaded { ir, parse, resolve } = load(&engine, &file)?;
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
        Command::Components => {
            for c in Engine::new().registry.iter() {
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
    }
    Ok(())
}
