//! `cargo bench -p irender-html` (filter with e.g. `cargo bench -p irender-html -- render/table`).
//!
//! - `load/*`: parse + resolve + CSS validation (what happens once per document change)
//! - `render/*`: IR → HTML + CSS (what happens per request)
//! - `render/html_parallel/*`: with `--features parallel`, lists of 64+ items on all cores
//! - `expr/*`, `style/*`: the hot inner pieces

#[path = "../tests/support/mod.rs"]
mod support;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use irender_core::{Format, StyleBlock, StyleValue, Template, Theme, expr};
use irender_html::Engine;
use irender_style::{Layer, StyleCompiler};
use serde_json::{Value as Json, json};
use std::hint::black_box;

fn fixtures() -> Vec<(&'static str, (String, Json))> {
    vec![
        ("product", support::product()),
        ("table_100", support::table(100)),
        ("table_1000", support::table(1000)),
        ("unique_styles_1000", support::unique_styles(1000)),
        ("deep_50", support::deep(50)),
    ]
}

fn load(c: &mut Criterion) {
    let engine = Engine::new();
    let without_css_check = Engine::new();
    let mut group = c.benchmark_group("load");
    for (name, (src, _)) in fixtures() {
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse", name), &src, |b, src| {
            b.iter(|| irender_core::parse(black_box(src), Format::Json).unwrap())
        });
        group.bench_with_input(BenchmarkId::new("parse+resolve+validate", name), &src, |b, src| {
            b.iter(|| engine.load(black_box(src), Format::Json).unwrap())
        });
        let doc = irender_core::parse(&src, Format::Json).unwrap();
        group.bench_with_input(BenchmarkId::new("resolve_only", name), &doc, |b, doc| {
            b.iter(|| irender_core::resolve(black_box(doc), &without_css_check.registry, &without_css_check.policy).unwrap())
        });
    }
    group.finish();
}

fn render(c: &mut Criterion) {
    let engine = Engine::new();
    let minify = Engine { minify: true, ..Engine::new() };
    let mut group = c.benchmark_group("render");
    for (name, (src, data)) in fixtures() {
        let ir = engine.load(&src, Format::Json).unwrap();
        let elements = support::count_elements(&engine.render(&ir, &data).unwrap().body);
        group.throughput(Throughput::Elements(elements as u64));
        group.bench_with_input(BenchmarkId::new("html", name), &(&ir, &data), |b, (ir, data)| {
            b.iter(|| engine.render(black_box(ir), black_box(data)).unwrap())
        });
        group.bench_with_input(BenchmarkId::new("document", name), &(&ir, &data), |b, (ir, data)| {
            b.iter(|| engine.render(black_box(ir), black_box(data)).unwrap().document("bench"))
        });
        group.bench_with_input(BenchmarkId::new("html+minify", name), &(&ir, &data), |b, (ir, data)| {
            b.iter(|| minify.render(black_box(ir), black_box(data)).unwrap())
        });
        #[cfg(feature = "parallel")]
        group.bench_with_input(BenchmarkId::new("html_parallel", name), &(&ir, &data), |b, (ir, data)| {
            let parallel = Engine { parallel_min_items: Some(64), ..Engine::new() };
            b.iter(|| parallel.render(black_box(ir), black_box(data)).unwrap())
        });
    }
    group.finish();
}

fn expressions(c: &mut Criterion) {
    let src = "{{ qty >= 5 ? 100 : qty * 20 }}% of {{ product.name }}";
    let tpl = Template::parse(src).unwrap();
    let data = json!({ "qty": 3, "product": { "name": "Mug" } });
    let mut group = c.benchmark_group("expr");
    group.bench_function("parse", |b| b.iter(|| Template::parse(black_box(src)).unwrap()));
    group.bench_function("eval", |b| b.iter(|| tpl.eval(&|p| expr::lookup(black_box(&data), p))));
    group.bench_function("fold+to_js", |b| {
        b.iter(|| tpl.fold(&|r| r == "qty", &|p| expr::lookup(&data, p)).to_js())
    });
    group.finish();
}

fn styles(c: &mut Criterion) {
    let theme = Theme::default();
    let block = StyleBlock::from_pairs(&[("padding", "8px"), ("color", "var(--color-text)"), ("display", "flex")])
        .nest(":hover", StyleBlock::from_pairs(&[("color", "red")]))
        .nest("@md", StyleBlock::from_pairs(&[("padding", "16px")]));
    let mut group = c.benchmark_group("style");
    group.bench_function("compile_repeated", |b| {
        let mut compiler = StyleCompiler::new(&theme);
        b.iter(|| compiler.compile(black_box(&block), Layer::Instance))
    });
    group.bench_function("compile_unique", |b| {
        let mut compiler = StyleCompiler::new(&theme);
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let mut unique = block.clone();
            unique.push("z-index", StyleValue::Static(i.to_string()));
            compiler.compile(&unique, Layer::Instance)
        })
    });
    group.finish();
}

criterion_group!(benches, load, render, expressions, styles);
criterion_main!(benches);
