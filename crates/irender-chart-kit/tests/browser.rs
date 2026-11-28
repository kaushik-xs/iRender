//! Draws charts in headless Chrome and checks what ECharts ended up with.
//!
//! Ignored by default because it needs Chrome, and ECharts from the CDN:
//!   cargo test -p irender-chart-kit --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically, and
//! ECHARTS=/path/to/echarts.min.js to test offline.

use irender_chart_kit::{ChartKit, TYPES};
use irender_core::Format;
use irender_html::Engine;
use serde_json::{Value as Json, json};
use std::{path::PathBuf, process::Command};

fn chrome() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CHROME") {
        return Some(path.into());
    }
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "google-chrome", "google-chrome-stable", "chromium"]
        .iter()
        .map(PathBuf::from)
        .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

fn kit() -> ChartKit {
    match std::env::var("ECHARTS") {
        Ok(path) => ChartKit { src: format!("file://{}", std::fs::canonicalize(path).unwrap().display()), ..ChartKit::default() },
        Err(_) => ChartKit::default(),
    }
}

/// Renders `doc` with `driver` appended, runs it in Chrome and returns what the
/// driver wrote into `<pre id="result">`, one line per check.
fn run(name: &str, engine: &Engine, doc: Json, driver: &str) -> Option<Vec<String>> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    irender_chart_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let dir = std::env::temp_dir().join(format!("irender-chart-kit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let load = || {
        let out = Command::new(&chrome)
            .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--allow-file-access-from-files", "--window-size=1000,900"])
            // Headless virtual time runs one animation frame, so ECharts' animations would
            // never finish; this turns them off through the kit's reduced-motion path.
            .arg("--force-prefers-reduced-motion")
            .args(["--virtual-time-budget=15000", "--dump-dom"])
            .arg(format!("file://{}", file.display()))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    const RESULT: &str = "<pre id=\"result\">";
    // On a very busy machine Chrome now and then gives the page back before its driver is
    // done, so without a result it's loaded once more.
    let mut dom = load();
    if !dom.contains(RESULT) {
        dom = load();
    }
    let _ = std::fs::remove_dir_all(&dir);
    let start = dom.find(RESULT).unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + RESULT.len();
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    Some(result.lines().map(String::from).collect())
}

/// Shared by the drivers: waits for charts, logs, and reports errors.
const PRELUDE: &str = r##"
  const out = [], log = (...a) => out.push(a.join(" "));
  const errors = [];
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
  const el = (id) => document.getElementById(id), chart = (id) => el(id).__x.chart;
  const opt = (id) => chart(id).getOption();
  const ready = async (ids) => { for (let i = 0; i < 100 && !ids.every((id) => el(id).hasAttribute("data-ready")); i++) await wait(50); };
"##;

const POSTLUDE: &str = r##"
  const pre = document.createElement("pre"); pre.id = "result";
  pre.textContent = out.concat(errors.map((e) => "error " + e)).join("\n");
  document.body.append(pre);
"##;

fn driver(body: &str) -> String {
    format!(
        "<script>\n// Host functions are defined by an ordinary script, before the runtime starts.\n\
         window.irenderChartFunctions = {{ money: (v) => '$' + v }};\nconst __errors = []; window.addEventListener('error', (e) => __errors.push(e.message));\n\
         const __ce = console.error; console.error = (...a) => {{ __errors.push(a.join(' ')); __ce(...a); }};\n\
         window.addEventListener('load', async () => {{\n{PRELUDE}\n errors.push(...__errors);\n try {{\n{body}\n }} catch (e) {{ log('threw', e.message); }}\n\
         errors.push(...__errors.filter((e) => !errors.includes(e)));\n{POSTLUDE}\n}});\n</script>"
    )
}

/// Data the shorthand of each type understands.
fn sample(t: &str) -> Json {
    let rows = json!([{ "day": "Mon", "a": 3, "b": 5 }, { "day": "Tue", "a": 4, "b": 2 }, { "day": "Wed", "a": 6, "b": 1 }]);
    match t {
        "candlestick" => json!({ "data": [{ "d": "Mon", "open": 1, "high": 4, "low": 0, "close": 3 }, { "d": "Tue", "open": 3, "high": 5, "low": 2, "close": 2 }] }),
        "boxplot" => json!({ "data": [{ "g": "x", "v": 1 }, { "g": "x", "v": 5 }, { "g": "x", "v": 3 }, { "g": "y", "v": 2 }] }),
        "pie" | "donut" | "funnel" => json!({ "data": [{ "name": "A", "value": 3 }, { "name": "B", "value": 5 }] }),
        "heatmap" => json!({ "data": [{ "x": "a", "y": "p", "v": 1 }, { "x": "b", "y": "p", "v": 4 }, { "x": "a", "y": "q", "v": 2 }] }),
        "gauge" => json!({ "data": 42 }),
        "tree" => json!({ "data": { "name": "root", "children": [{ "name": "a" }, { "name": "b" }] } }),
        "treemap" | "sunburst" => json!({ "data": [{ "name": "a", "value": 3 }, { "name": "b", "children": [{ "name": "c", "value": 2 }] }] }),
        "sankey" | "graph" | "chord" => json!({ "links": [{ "source": "a", "target": "b", "value": 2 }, { "source": "b", "target": "c", "value": 1 }] }),
        "themeRiver" => json!({ "data": [{ "w": "2026-01-05", "a": 1, "b": 2 }, { "w": "2026-01-12", "a": 3, "b": 1 }] }),
        "gantt" => json!({ "data": [{ "name": "a", "start": "2026-01-05", "end": "2026-01-09" }, { "name": "b", "start": "2026-01-12" }] }),
        _ => json!({ "data": rows }),
    }
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn charts_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());

    let mut children: Vec<Json> = TYPES
        .iter()
        .map(|t| {
            let mut props = sample(t);
            props["type"] = json!(t);
            json!({ "type": "ui-chart", "attrs": { "id": format!("t-{t}") }, "props": props, "style": { "width": 300 } })
        })
        .collect();
    children.extend([
        json!({ "type": "ui-chart", "attrs": { "id": "live" },
            "props": { "type": "{{ kind }}", "data": "{{ rows }}", "x": "day", "bind": "picked",
                       "option": { "title": { "text": "{{ label }}" } } },
            "on": { "change": { "action": "set_state", "key": "changes", "value": "{{ changes + 1 }}" } } }),
        json!({ "type": "ui-chart", "attrs": { "id": "fn" },
            "props": { "type": "line", "data": [1, 2, 3], "option": { "yAxis": { "axisLabel": { "formatter": { "$fn": "money" } } } } } }),
        json!({ "type": "if", "props": { "when": "{{ show }}" }, "children": [
            { "type": "ui-chart", "attrs": { "id": "hidden" }, "props": { "type": "bar", "data": [1, 2] } }] }),
        json!({ "type": "ui-sparkline", "attrs": { "id": "spark" }, "props": { "data": [3, 1, 4, 1, 5] } }),
        json!({ "type": "ui-chart", "attrs": { "id": "loading" }, "props": { "type": "bar", "data": [1], "loading": "{{ busy }}" } }),
    ]);
    let doc = json!({ "version": 1,
        "state": { "kind": "bar", "rows": [{ "day": "Mon", "n": 120 }, { "day": "Tue", "n": 80 }, { "day": "Wed", "n": 50 }],
                   "picked": null, "changes": 0, "label": "First", "show": true, "busy": true },
        "root": { "type": "stack", "children": children } });

    fn expected_type(t: &str) -> &str {
        match t {
            "area" => "line",
            "donut" => "pie",
            "gantt" => "custom",
            "pareto" => "bar",
            t => t,
        }
    }
    let types: Vec<String> = TYPES.iter().map(|t| format!("[\"{t}\", \"{}\"]", expected_type(t))).collect();
    let body = format!(
        r##"
    const types = [{types}];
    await ready(types.map(([t]) => "t-" + t).concat(["live", "fn", "hidden", "spark", "loading"]));
    const wrong = types.filter(([t, want]) => {{
      const c = chart("t-" + t), s = c && c.getOption().series[0];
      return !s || s.type !== want || !((s.data && s.data.length) || (s.links && s.links.length));
    }});
    log("types", wrong.length ? JSON.stringify(wrong.map(([t]) => t)) : "ok");

    log("live", opt("live").series[0].type, opt("live").xAxis[0].data.join(","), opt("live").title[0].text);
    root.__irender.set("kind", "line");
    root.__irender.set("rows", S().rows.concat([{{ day: "Thu", n: 10 }}]));
    root.__irender.set("label", "Second");
    await wait(50);
    log("updated", opt("live").series[0].type, opt("live").xAxis[0].data.join(","), opt("live").title[0].text);
    root.__irender.set("kind", "bar");
    await wait(600);

    // A real click on Tuesday's bar.
    const [px, py] = chart("live").convertToPixel({{ seriesIndex: 0 }}, ["Tue", 40]);
    const canvas = el("live").querySelector("canvas"), r = canvas.getBoundingClientRect();
    const at = {{ bubbles: true, clientX: r.left + px, clientY: r.top + py }};
    for (const type of ["mousemove", "mousedown", "mouseup", "click"]) canvas.dispatchEvent(new MouseEvent(type, at));
    await wait(50);
    log("clicked", S().picked, S().changes);

    log("fn", typeof opt("fn").yAxis[0].axisLabel.formatter);
    log("spark", opt("spark").xAxis[0].show, opt("spark").yAxis[0].show, opt("spark").legend[0].show);
    log("aria", el("live").firstElementChild.getAttribute("aria-label").startsWith("This is a chart"));
    log("palette", opt("live").color[0]);

    // Changing a token restyles every chart.
    document.documentElement.style.setProperty("--color-chart-1", "rgb(255, 0, 0)");
    await wait(200);
    log("restyled", chart("live").getOption().color[0], chart("spark").getOption().color[0]);

    log("loading", !!el("loading").querySelector("canvas"));
    root.__irender.set("busy", false);

    // Hidden by an `if`: disposed of; shown again: drawn again.
    const hidden = el("hidden");
    root.__irender.set("show", false);
    await wait(200);
    log("hidden", hidden.isConnected, hidden.__x.chart === null);
    root.__irender.set("show", true);
    await wait(200);
    log("shown", hidden.isConnected, !!hidden.__x.chart, hidden.__x.chart && hidden.__x.chart.getOption().series[0].data.join(","));
"##,
        types = types.join(",")
    );
    let Some(lines) = run("charts", &engine, doc, &driver(&body)) else { return };
    let expected = [
        "types ok",
        "live bar Mon,Tue,Wed First",
        "updated line Mon,Tue,Wed,Thu Second",
        "clicked Tue 1",
        "fn function",
        "spark false false false",
        "aria true",
        "palette rgba(42,120,214,1)",
        "restyled rgba(255,0,0,1) rgba(255,0,0,1)",
        "loading true",
        "hidden false true",
        "shown true true 1,2",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt: the table's rows and labels, bars by series, arrows, groups that close,
/// the height that follows the rows, a click on a bar, zoom buttons and lines down.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1,
        "state": { "row": null, "toggles": 0, "tasks": [
            { "id": "p", "name": "Plan", "owner": "Asha" },
            { "id": "a", "group": "p", "name": "Brief", "team": "Product", "owner": "Asha", "start": "2026-03-02", "end": "2026-03-06", "progress": 100 },
            { "id": "b", "group": "p", "name": "Interviews", "team": "Design", "start": "2026-03-04", "end": "2026-03-13", "progress": 50 },
            { "id": "m", "group": "p", "name": "Signed off", "start": "2026-03-16", "milestone": true, "after": ["a", "b"] },
            { "id": "c", "name": "Build <b>it</b>", "team": "Engineering", "start": "2026-03-16", "end": "2026-03-27", "after": "m" },
            { "id": "q", "name": "Later", "collapsed": true },
            { "id": "d", "group": "q", "name": "Hidden", "team": "Product", "start": "2026-03-30", "end": "2026-04-03", "after": "c" } ] },
        "root": { "type": "stack", "children": [
            { "type": "ui-chart", "attrs": { "id": "g" },
              "props": { "type": "gantt", "data": "{{ tasks }}", "series": "team", "zoom": true, "bindRow": "row",
                         "columns": ["name", { "field": "owner", "name": "Owner", "width": 80 }],
                         "markers": [{ "at": "2026-03-20", "name": "Freeze" }] },
              "on": { "toggle": { "action": "set_state", "key": "toggles", "value": "{{ toggles + 1 }}" } } } ] } });
    let body = r##"
    await ready(["g"]);
    const o = () => opt("g"), rows = () => o().yAxis[0].data.length;
    // The table: HTML over the chart, a row per task.
    const T = () => el("g").querySelector("[data-gantt-table]"), trs = () => [...T().querySelectorAll("[role=row][data-key]")];
    const fmt = (i) => { const r = trs()[i]; return `${r.getAttribute("aria-level")}${r.hasAttribute("aria-expanded") ? (r.getAttribute("aria-expanded") === "true" ? "+" : "-") : ""}|${[...r.children].map((c) => c.textContent).join("|")}`; };
    log("rows", rows(), el("g").style.height);
    log("labels", fmt(0), fmt(1), fmt(5));
    log("table", T().getAttribute("role"), [...T().querySelectorAll("[role=columnheader]")].map((c) => c.textContent).join(","), trs()[1].getAttribute("aria-label"));
    // Each row sits on its bars' row.
    const g0 = el("g").getBoundingClientRect(), mid = (r) => Math.round(r.getBoundingClientRect().top + r.getBoundingClientRect().height / 2 - g0.top);
    log("aligned", trs().every((r, i) => Math.abs(mid(r) - Math.round(chart("g").convertToPixel({ yAxisIndex: 0 }, String(i)))) <= 1));
    const bars = o().series.filter((s) => s.type === "custom" && s.encode?.x?.length === 2 && s.id !== "links");
    log("series", bars.map((s) => s.name + ":" + s.data.length).join(" "));
    log("legend", o().legend[0].data.join(","));
    log("links", o().series.find((s) => s.id === "links").data.length);
    log("marks", o().series.find((s) => s.id === "rows").markLine.data.map((m) => m.name).join(","));
    const tip = o().tooltip[0].formatter({ data: bars.find((s) => s.name === "Engineering").data[0] });
    log("tip", tip);
    const brief = bars.find((s) => s.name === "Product").data[0].value;
    log("brief", brief[0], new Date(brief[1]).getDate(), new Date(brief[2]).getDate(), brief[3], brief[4]);
    const plan = o().series.find((s) => s.name === "Groups").data[0].value;
    log("plan", plan[0], new Date(plan[1]).getDate(), new Date(plan[2]).getDate(), plan[3].toFixed(2), plan[4]);

    // Real clicks on the canvas.
    const g = chart("g"), canvas = el("g").querySelector("canvas");
    const click = async (x, y) => {
      const r = canvas.getBoundingClientRect(), at = { bubbles: true, clientX: r.left + x, clientY: r.top + y };
      for (const type of ["mousemove", "mousedown", "mouseup", "click"]) canvas.dispatchEvent(new MouseEvent(type, at));
      await wait(50);
    };
    const rowY = (i) => g.convertToPixel({ yAxisIndex: 0 }, String(i));

    // Closing the first group by its name: its tasks go, and the arrow from them
    // starts at the group.
    trs()[0].click(); await wait(50);
    log("closed", rows(), fmt(0), S().toggles, el("g").style.height);
    log("links closed", JSON.stringify(o().series.find((s) => s.id === "links").data.map((l) => [l[0], l[2]])));

    // A click on Build's bar picks its row.
    const c = o().series.find((s) => s.name === "Engineering").data[0].value;
    await click(g.convertToPixel({ xAxisIndex: 0 }, (c[1] + c[2]) / 2), rowY(c[0]));
    log("picked", S().row && S().row.id);

    // The keys: Down, Right into Build's group… there's none, so a picked task; Left to
    // the group above; Right opens the closed one; Enter picks.
    const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
    trs()[0].focus();
    key("ArrowDown"); log("down", document.activeElement.getAttribute("data-key"), document.activeElement.tabIndex);
    key("Enter"); await wait(20); log("enter picks", S().row && S().row.id, document.activeElement.getAttribute("aria-selected"));
    key("ArrowUp"); key("ArrowRight"); await wait(50); log("right opens", rows(), document.activeElement.getAttribute("data-key"), document.activeElement.getAttribute("aria-expanded"));
    key("ArrowDown"); key("ArrowLeft"); log("left to group", document.activeElement.getAttribute("data-key"));
    key("Enter"); await wait(50); log("enter folds", rows(), document.activeElement.getAttribute("aria-expanded"));
    key("End"); log("end", document.activeElement.getAttribute("data-key"));

    // The zoom buttons halve and double the window, and show it all.
    const zoom = () => { const z = o().dataZoom[0]; return Math.round(z.end - z.start); };
    const w = el("g").clientWidth;
    for (const [id, right] of [["zoom-in", 42], ["zoom-in", 42], ["zoom-out", 72], ["zoom-all", 12]]) {
      await click(w - right - 13, 15);
      log(id, zoom());
    }
"##;
    let Some(lines) = run("gantt", &engine, doc, &driver(body)) else { return };
    let expected = [
        "rows 6 306px",
        // Rows in the table: level, open (+) or closed (-), and the cells.
        "labels 1+|▾ Plan|Asha 2|Brief|Asha 1-|▸ Later|",
        "table treegrid Task,Owner Brief, Mar 2, 2026 – Mar 6, 2026 · 5 days, 100% done, Product",
        "aligned true",
        "series Product:1 Design:1 :1 Engineering:1 Groups:2",
        "legend Product,Design,Engineering",
        "links 4",
        "marks Freeze",
        "tip <b>Build &lt;b&gt;it&lt;/b&gt;</b><br/>Mar 16, 2026 – Mar 27, 2026 · 12 days<br/>Engineering",
        "brief 1 2 7 1 0",
        "plan 0 2 16 0.67 1",
        "closed 3 1-|▸ Plan|Asha 1 204px",
        "links closed [[0,1],[1,2]]",
        "picked c",
        "down c 0",
        // The task was picked by a click on its bar: Enter picks it again, which clears it.
        "enter picks  false",
        // Right opens the closed group; Left goes back to it; Enter folds it.
        "right opens 6 p true",
        "left to group p",
        "enter folds 3 false",
        "end q",
        "zoom-in 50",
        "zoom-in 25",
        "zoom-out 50",
        "zoom-all 100",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt with `visibleRows`: as tall as that many rows, the rest scrolled to inside it
/// (the wheel, a bar on the right, or the keys on the table), the table following its
/// window; zooming the timeline leaves the rows where they are.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_scrolls_its_rows_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let tasks: Vec<Json> = (0..12)
        .map(|i| json!({ "id": format!("t{i}"), "name": format!("Task {i}"), "start": format!("2026-03-{:02}", i + 2), "end": format!("2026-03-{:02}", i + 5) }))
        .collect();
    let doc = json!({ "version": 1, "state": { "tasks": tasks },
        "root": { "type": "ui-chart", "attrs": { "id": "g" }, "props": { "type": "gantt", "data": "{{ tasks }}", "zoom": true, "visibleRows": 4 } } });
    let body = r##"
    await ready(["g"]);
    const g = chart("g"), o = () => opt("g");
    const T = () => el("g").querySelector("[data-gantt-table]");
    const shownRows = () => [...T().querySelectorAll("[role=row][data-key]")].filter((r) => r.style.display !== "none");
    const keys = () => shownRows().map((r) => r.getAttribute("data-key")).join(",");
    const g0 = () => el("g").getBoundingClientRect(), mid = (r) => r.getBoundingClientRect().top + r.getBoundingClientRect().height / 2 - g0().top;
    const aligned = () => shownRows().every((r) => Math.abs(mid(r) - g.convertToPixel({ yAxisIndex: 0 }, r.getAttribute("data-key").slice(1))) <= 1);
    const bars = () => o().series.filter((s) => s.type === "custom" && s.id !== "rows" && s.id !== "links").flatMap((s) => s.data).length;
    log("start", el("g").style.height, keys(), aligned(), o().dataZoom.map((z) => z.id || z.type).join(","));
    g.dispatchAction({ type: "dataZoom", dataZoomId: "rows", startValue: 5, endValue: 8 }); await wait(250);
    log("scrolled", keys(), aligned());
    // The wheel: a row for each row's height (34px) scrolled; at the end, the page's.
    const wheel = (dy) => { const ev = new WheelEvent("wheel", { deltaY: dy, bubbles: true, cancelable: true }); el("g").querySelector("canvas").dispatchEvent(ev); return ev.defaultPrevented; };
    log("wheel", wheel(68), keys(), wheel(34 * 10), keys(), wheel(34), keys());
    // The keys: End goes to the last task, which scrolls into view; Home back to the first.
    shownRows()[0].focus();
    const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
    key("End"); await wait(250);
    log("end", document.activeElement.getAttribute("data-key"), keys(), aligned());
    key("Home"); await wait(250);
    log("home", document.activeElement.getAttribute("data-key"), keys());
    key("ArrowDown"); key("ArrowDown"); key("ArrowDown"); key("ArrowDown"); await wait(250);
    log("down past the window", document.activeElement.getAttribute("data-key"), keys());
    // Zooming the timeline in leaves the rows' window alone.
    const w = el("g").clientWidth, canvas = el("g").querySelector("canvas");
    const click = async (x, y) => {
      const r = canvas.getBoundingClientRect(), at = { bubbles: true, clientX: r.left + x, clientY: r.top + y };
      for (const type of ["mousemove", "mousedown", "mouseup", "click"]) canvas.dispatchEvent(new MouseEvent(type, at));
      await wait(250);
    };
    await click(w - 42 - 13, 15);
    const time = o().dataZoom.find((z) => z.id !== "rows" && z.id !== "rowsBar");
    log("zoomed", Math.round(time.end - time.start), keys());
"##;
    let Some(lines) = run("gantt-scroll", &engine, doc, &driver(body)) else { return };
    let expected = [
        // Four rows under the dates (and the zoom buttons' band): 74 + 4 × 34 + 12.
        "start 222px t0,t1,t2,t3 true inside,rows,rowsBar",
        "scrolled t5,t6,t7,t8 true",
        // Down two rows; then to the end; past it, the wheel is left to the page.
        "wheel true t7,t8,t9,t10 true t8,t9,t10,t11 false t8,t9,t10,t11",
        "end t11 t8,t9,t10,t11 true",
        "home t0 t0,t1,t2,t3",
        "down past the window t4 t1,t2,t3,t4",
        "zoomed 50 t1,t2,t3,t4",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt's `critical` path: the tasks with no slack, ringed, and the links between
/// them; slack counts working days with `workdays`, so a weekend before the next task
/// isn't slack.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_critical_path_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    // 2 March 2026 is a Monday; M, on Monday the 9th, waits on B and C.
    let tasks = json!([
        { "id": "a", "name": "A", "start": "2026-03-02", "end": "2026-03-04" },
        { "id": "b", "name": "B", "start": "2026-03-05", "end": "2026-03-06", "after": "a" },
        { "id": "c", "name": "C", "start": "2026-03-05", "end": "2026-03-05", "after": "a" },
        { "id": "m", "name": "M", "start": "2026-03-09", "milestone": true, "after": ["b", "c"] },
        { "id": "d", "name": "D", "start": "2026-03-02", "end": "2026-03-03" } ]);
    let chart = |id: &str, workdays: bool| json!({ "type": "ui-chart", "attrs": { "id": id }, "props": { "type": "gantt", "data": tasks, "critical": true, "workdays": workdays, "today": false } });
    let doc = json!({ "version": 1, "root": { "type": "stack", "children": [chart("days", false), chart("work", true)] } });
    let body = r##"
    await ready(["days", "work"]);
    const critical = (id) => [...el(id).querySelectorAll("[data-gantt-table] [role=row][data-key]")]
      .filter((r) => r.getAttribute("aria-label").includes("On the critical path")).map((r) => r.getAttribute("data-key")).join(",") || "none";
    const flags = (id) => opt(id).series.filter((s) => s.type === "custom" && s.encode?.x?.length === 2 && s.id !== "links").flatMap((s) => s.data).filter((d) => d.value[5] === 1).length;
    const links = (id) => opt(id).series.find((s) => s.id === "links").data.filter((l) => l[5] === 1).map((l) => `${l[0]}>${l[2]}`).join(",") || "none";
    log("calendar days", critical("days"), flags("days"), links("days"));
    log("working days", critical("work"), flags("work"), links("work"));
"##;
    let Some(lines) = run("gantt-critical", &engine, doc, &driver(body)) else { return };
    let expected = [
        // The weekend before M is slack: only M sets the end.
        "calendar days m 1 none",
        // On working days it isn't: A, B and M, joined A → B → M; C has a day (Friday) to spare.
        "working days a,b,m 3 0>1,1>3",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt's baselines: a thin bar from each task's planned dates, a group's from its
/// tasks', how late or early each one is, the timeline widened to show them, and
/// `baselines: false` from state hiding them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_baselines_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1, "state": { "show": true, "tasks": [
            { "id": "g", "name": "G" },
            { "id": "a", "group": "g", "name": "A", "start": "2026-03-02", "end": "2026-03-04", "baselineStart": "2026-03-02", "baselineEnd": "2026-03-03" },
            { "id": "b", "group": "g", "name": "B", "start": "2026-03-05", "end": "2026-03-06", "baselineStart": "2026-03-05", "baselineEnd": "2026-03-09" },
            { "id": "c", "name": "C", "start": "2026-03-05", "end": "2026-03-05" },
            { "id": "m", "name": "M", "start": "2026-03-10", "milestone": true, "baselineStart": "2026-03-09" },
            { "id": "e", "name": "E", "start": "2026-03-03", "end": "2026-03-04", "baselineStart": "2026-02-23", "baselineEnd": "2026-02-24" } ] },
        "root": { "type": "ui-chart", "attrs": { "id": "g" }, "props": { "type": "gantt", "data": "{{ tasks }}", "baselines": "{{ show }}", "today": false } } });
    let body = r##"
    await ready(["g"]);
    const rows = () => [...el("g").querySelectorAll("[data-gantt-table] [role=row][data-key]")];
    const said = () => rows().map((r) => `${r.getAttribute("data-key")}: ${(r.getAttribute("aria-label").match(/Planned .*?(\d+ days? (late|early)|$)/) ?? ["-"])[0]}`).join(" | ");
    const planned = () => opt("g").series.filter((s) => s.type === "custom" && s.encode?.x?.length === 2 && s.id !== "links").flatMap((s) => s.data).filter((d) => Number.isFinite(d.value[6])).length;
    const from = () => new Date(opt("g").xAxis[0].min).getDate();
    log("shown", planned(), "from", from());
    log(said());
    root.__irender.set("show", false);
    await wait(200);
    log("hidden", planned(), "from", from());
    log(said());
"##;
    let Some(lines) = run("gantt-baselines", &engine, doc, &driver(body)) else { return };
    let expected = [
        // G, A, B, M and E; the timeline starts before E's planned 23 Feb.
        "shown 5 from 22",
        // G's from its tasks'; A and M a day late, B three days early, E eight late.
        "g: Planned Mar 2, 2026 – Mar 9, 2026, 3 days early | a: Planned Mar 2, 2026 – Mar 3, 2026, 1 day late | b: Planned Mar 5, 2026 – Mar 9, 2026, 3 days early | c: - | m: Planned Mar 9, 2026, 1 day late | e: Planned Feb 23, 2026 – Feb 24, 2026, 8 days late",
        "hidden 0 from 1",
        "g: - | a: - | b: - | c: - | m: - | e: -",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt's `snap`: tasks with times of day move and stretch by the step, landing on its
/// grid, written as they were (local times stay local, epoch numbers numbers), and
/// `cascade` pushing on by steps; a task in whole days still moves by days.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_snap_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1, "state": { "tasks": [
            { "id": "a", "name": "A", "start": "2026-03-02T09:00", "end": "2026-03-02T10:30" },
            { "id": "b", "name": "B", "start": "2026-03-02T10:30", "end": "2026-03-02T11:00", "after": "a" },
            { "id": "c", "name": "C", "start": "2026-03-02T09:07", "end": "2026-03-02T09:37" },
            { "id": "d", "name": "D", "start": "2026-03-02", "end": "2026-03-03" } ] },
        "root": { "type": "ui-chart", "attrs": { "id": "g" }, "props": { "type": "gantt", "data": "{{ tasks }}", "bindData": "tasks",
            "snap": "15m", "cascade": true, "today": false } } });
    let body = r##"
    await ready(["g"]);
    const rows = () => [...el("g").querySelectorAll("[data-gantt-table] [role=row][data-key]")];
    const dates = () => S().tasks.map((t) => `${t.id} ${String(t.start).slice(5)}–${String(t.end).slice(5)}`).join(", ");
    const key = (n, k, mods) => { rows()[n].focus(); document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, altKey: true, ...mods, bubbles: true, cancelable: true })); };
    key(0, "ArrowRight"); await wait(100);
    log("moved", dates());
    key(0, "ArrowRight", { shiftKey: true }); await wait(100);
    log("stretched", dates());
    key(2, "ArrowLeft"); await wait(100);
    log("aligned", dates());
    key(3, "ArrowRight"); await wait(100);
    log("days", dates());
"##;
    let Some(lines) = run("gantt-snap", &engine, doc, &driver(body)) else { return };
    let expected = [
        // A a quarter of an hour on; B, waiting on it, pushed to its new end.
        "moved a 03-02T09:15–03-02T10:45, b 03-02T10:45–03-02T11:15, c 03-02T09:07–03-02T09:37, d 03-02–03-03",
        "stretched a 03-02T09:15–03-02T11:00, b 03-02T11:00–03-02T11:30, c 03-02T09:07–03-02T09:37, d 03-02–03-03",
        // C, off the grid, lands on it (8:52 rounds to 8:45) and keeps its half hour.
        "aligned a 03-02T09:15–03-02T11:00, b 03-02T11:00–03-02T11:30, c 03-02T08:45–03-02T09:15, d 03-02–03-03",
        "days a 03-02T09:15–03-02T11:00, b 03-02T11:00–03-02T11:30, c 03-02T08:45–03-02T09:15, d 03-03–03-04",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt's `cascade: "both"`: moving a task back pulls the tasks waiting on it back to
/// start when the last thing they wait for ends; `cascade: true` only pushes.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_cascade_pull_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let tasks = json!([
        { "id": "a", "name": "A", "start": "2026-03-02", "end": "2026-03-04" },
        { "id": "e", "name": "E", "start": "2026-03-02", "end": "2026-03-06" },
        { "id": "b", "name": "B", "start": "2026-03-09", "end": "2026-03-10", "after": "a" },
        { "id": "d", "name": "D", "start": "2026-03-09", "end": "2026-03-09", "after": ["a", "e"] } ]);
    let chart = |id: &str, key: &str, cascade: Json| json!({ "type": "ui-chart", "attrs": { "id": id }, "props": { "type": "gantt", "data": format!("{{{{ {key} }}}}"), "bindData": key, "cascade": cascade, "today": false } });
    let doc = json!({ "version": 1, "state": { "both": tasks, "push": tasks },
        "root": { "type": "stack", "children": [chart("both", "both", json!("both")), chart("push", "push", json!(true))] } });
    let body = r##"
    await ready(["both", "push"]);
    const dates = (k) => S()[k].map((t) => `${t.id} ${t.start.slice(8)}–${t.end.slice(8)}`).join(", ");
    const back = (id) => { el(id).querySelector("[data-gantt-table] [role=row][data-key]").focus(); document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", altKey: true, bubbles: true, cancelable: true })); };
    back("both"); await wait(100);
    log("both", dates("both"));
    back("push"); await wait(100);
    log("push", dates("push"));
"##;
    let Some(lines) = run("gantt-cascade-pull", &engine, doc, &driver(body)) else { return };
    let expected = [
        // B comes back to start as A ends; D only as far as E, which ends later, lets it.
        "both a 01–03, e 02–06, b 04–05, d 07–07",
        "push a 01–03, e 02–06, b 09–10, d 09–09",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// A gantt with `workdays` and `holidays`: lengths in working days, days off shaded, and
/// a moved task (and, with `cascade`, those that wait on it) landing on working days and
/// keeping its working days.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_workdays_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    // 4–6 Nov 2026 is Wednesday to Friday; 10 Nov, a Tuesday, is a holiday.
    let doc = json!({ "version": 1, "state": { "tasks": [
            { "id": "a", "name": "A", "start": "2026-11-04", "end": "2026-11-06" },
            { "id": "b", "name": "B", "start": "2026-11-09", "end": "2026-11-12", "after": "a" } ] },
        "root": { "type": "ui-chart", "attrs": { "id": "g" }, "props": { "type": "gantt", "data": "{{ tasks }}", "bindData": "tasks",
            "workdays": true, "holidays": ["2026-11-10"], "cascade": true, "today": false } } });
    let body = r##"
    await ready(["g"]);
    const o = () => opt("g");
    const rows = () => [...el("g").querySelectorAll("[data-gantt-table] [role=row][data-key]")];
    const dates = () => S().tasks.map((t) => `${t.id} ${t.start.slice(8)}–${t.end.slice(8)}`).join(", ");
    const day = (t) => new Date(t).getDate();
    log("lengths", rows().map((r) => r.getAttribute("aria-label").match(/· ([^,]*)/)[1]).join(" | "));
    log("off", o().series.find((s) => s.id === "rows").markArea.data.map(([a, b]) => `${day(a.xAxis)}-${day(b.xAxis)}`).join(" "));
    const key = (k, mods) => { rows()[0].focus(); document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, altKey: true, ...mods, bubbles: true, cancelable: true })); };
    key("ArrowRight"); await wait(100);
    log("moved", dates());
    key("ArrowRight", { shiftKey: true }); await wait(100);
    log("stretched", dates());
    key("ArrowLeft"); await wait(100);
    log("back", dates());
"##;
    let Some(lines) = run("gantt-workdays", &engine, doc, &driver(body)) else { return };
    let expected = [
        "lengths 3 working days | 3 working days",
        // The weekends and the holiday, each run of days off one band.
        "off 7-9 10-11 14-16",
        // A moves a working day and keeps three; B, waiting on it, starts after the holiday.
        "moved a 05–09, b 11–13",
        // A's end steps over the holiday; B moves on, past the weekend.
        "stretched a 05–11, b 12–16",
        // Back a working day, keeping its four; B stays (cascade only pushes on).
        "back a 04–09, b 12–16",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// Editing a gantt: dragging a bar moves its task and an end stretches it, a day at a
/// time, written back to `bindData` in the dates' own form; Escape puts a drag back; a
/// task that starts before the one it waits for ends is marked; Alt and the arrows on
/// a row move it and its end.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_editing_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1,
        "state": { "row": null, "edits": 0, "tasks": [
            { "id": "a", "name": "A", "start": "2026-03-02", "end": "2026-03-06" },
            { "id": "b", "name": "B", "start": "2026-03-09", "end": "2026-03-13", "after": "a" },
            { "id": "m", "name": "M", "start": "2026-03-20T09:00:00.000Z", "milestone": true, "after": "b" } ] },
        "root": { "type": "ui-chart", "attrs": { "id": "g" },
            "props": { "type": "gantt", "data": "{{ tasks }}", "bindData": "tasks", "bindRow": "row", "today": false },
            "on": { "edit": { "action": "set_state", "key": "edits", "value": "{{ edits + 1 }}" } } } });
    let body = r##"
    await ready(["g"]);
    const g = chart("g"), canvas = el("g").querySelector("canvas");
    const tasks = () => S().tasks.map((t) => `${t.id}:${t.start.slice(5, 16)}${t.end ? ".." + t.end.slice(5) : ""}`).join(" ");
    const ms = (v) => { const [y, m, d] = v.split("-").map(Number); return new Date(y, m - 1, d).getTime(); };
    const x = (t) => g.convertToPixel({ xAxisIndex: 0 }, t), row = (i) => g.convertToPixel({ yAxisIndex: 0 }, String(i));
    const day = x(ms("2026-03-03")) - x(ms("2026-03-02"));
    const ptr = (target, type, px, py) => {
      const r = canvas.getBoundingClientRect(), o = { bubbles: true, cancelable: true, pointerId: 3, pointerType: "mouse", isPrimary: true, button: 0, clientX: r.left + px, clientY: r.top + py };
      target.dispatchEvent(new PointerEvent(type, o));
      target.dispatchEvent(new MouseEvent(type.replace("pointer", "mouse"), o));
    };
    const drag = async (px, py, dx, end = "pointerup") => {
      ptr(canvas, "pointermove", px, py); ptr(canvas, "pointerdown", px, py);
      ptr(window, "pointermove", px + dx / 2, py); ptr(window, "pointermove", px + dx, py);
      if (end) { ptr(window, end, px + dx, py); canvas.dispatchEvent(new MouseEvent("click", { bubbles: true })); }
      await wait(50);
    };
    log("start", tasks());
    // B's middle, two days on.
    await drag((x(ms("2026-03-09")) + x(ms("2026-03-14"))) / 2, row(1), day * 2);
    log("moved", tasks(), S().edits, String(S().row));
    // A's end, six days on: B now starts before A ends.
    await drag(x(ms("2026-03-07")) - 2, row(0), day * 6);
    const links = g.getOption().series.find((s) => s.id === "links").data;
    log("stretched", tasks(), JSON.stringify(links.map((l) => l[4])));
    const rowB = el("g").querySelector('[data-gantt-table] [data-key="b"]');
    log("late", rowB.getAttribute("aria-label"));
    // A's start can't pass its end.
    await drag(x(ms("2026-03-02")) + 2, row(0), day * 30);
    log("not inside out", tasks());
    // Escape puts a drag back.
    await drag((x(ms("2026-03-11")) + x(ms("2026-03-16"))) / 2, row(1), day * 3, null);
    const mid = tasks();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })); await wait(20);
    ptr(window, "pointerup", 0, 0); await wait(20);
    log("escape", mid, "|", tasks(), S().edits);
    // The milestone moves its start, a timestamp stays one.
    await drag(x(Date.parse("2026-03-20T09:00:00.000Z")), row(2), -day);
    log("milestone", tasks());
    // The keys on a row: Alt+Left moves B a day back, Alt+Shift+Right its end a day on.
    el("g").querySelector('[data-gantt-table] [data-key="b"]').focus();
    const key = (k, shift) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, altKey: true, shiftKey: !!shift, bubbles: true, cancelable: true }));
    key("ArrowLeft"); await wait(20); key("ArrowRight", true); await wait(60);
    log("keys", tasks(), document.activeElement.getAttribute("data-key"));
    await wait(100);
    log("said", el("g").querySelector("[role=status]").textContent);
"##;
    let Some(lines) = run("gantt-editing", &engine, doc, &driver(body)) else { return };
    let expected = [
        "start a:03-02..03-06 b:03-09..03-13 m:03-20T09:00",
        // Two days on; the click that ended the drag picked nothing.
        "moved a:03-02..03-06 b:03-11..03-15 m:03-20T09:00 1 null",
        // A ends after B starts: B's arrow is red, and its row says so.
        "stretched a:03-02..03-12 b:03-11..03-15 m:03-20T09:00 [1,0]",
        "late B, Mar 11, 2026 – Mar 15, 2026 · 5 days, Starts before A ends",
        // A's start can't go past its end.
        "not inside out a:03-02..03-12 b:03-11..03-15 m:03-20T09:00",
        // While dragging, only the chart moved; Escape left the tasks as they were.
        "escape a:03-02..03-12 b:03-11..03-15 m:03-20T09:00 | a:03-02..03-12 b:03-11..03-15 m:03-20T09:00 2",
        // A timestamp stays a timestamp.
        "milestone a:03-02..03-12 b:03-11..03-15 m:03-19T09:00",
        // Alt+Left: B a day back; Alt+Shift+Right: its end a day on.
        "keys a:03-02..03-12 b:03-10..03-15 m:03-19T09:00 b",
        "said B, Mar 10, 2026 – Mar 15, 2026 · 6 days, Starts before A ends",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// Gantt links: drawn by dragging from a bar's dot onto another task, refused when they'd
/// go round in a circle, removed by clicking the arrow; and with `cascade`, the tasks that
/// wait on a moved one move on so they start when it ends.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gantt_links_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1,
        "state": { "links": 0, "tasks": [
            { "id": "a", "name": "A", "start": "2026-03-02", "end": "2026-03-06" },
            { "id": "b", "name": "B", "start": "2026-03-09", "end": "2026-03-13", "after": "a" },
            { "id": "c", "name": "C", "start": "2026-03-16", "end": "2026-03-20", "after": "b" },
            { "id": "d", "name": "D", "start": "2026-03-02", "end": "2026-03-04" } ] },
        "root": { "type": "ui-chart", "attrs": { "id": "g" },
            "props": { "type": "gantt", "data": "{{ tasks }}", "bindData": "tasks", "cascade": true, "today": false },
            "on": { "link": { "action": "set_state", "key": "links", "value": "{{ links + 1 }}" } } } });
    let body = r##"
    await ready(["g"]);
    const g = chart("g"), canvas = el("g").querySelector("canvas");
    const tasks = () => S().tasks.map((t) => `${t.id}:${t.start.slice(5)}..${t.end.slice(5)}${t.after ? "<" + [].concat(t.after).join("+") : ""}`).join(" ");
    const ms = (v) => { const [y, m, d] = v.split("-").map(Number); return new Date(y, m - 1, d).getTime(); };
    const x = (v) => g.convertToPixel({ xAxisIndex: 0 }, ms(v)), row = (i) => g.convertToPixel({ yAxisIndex: 0 }, String(i));
    const day = x("2026-03-03") - x("2026-03-02");
    const ptr = (target, type, px, py) => {
      const r = canvas.getBoundingClientRect(), o = { bubbles: true, cancelable: true, pointerId: 4, pointerType: "mouse", isPrimary: true, button: 0, clientX: r.left + px, clientY: r.top + py };
      target.dispatchEvent(new PointerEvent(type, o));
      target.dispatchEvent(new MouseEvent(type.replace("pointer", "mouse"), o));
    };
    const drag = async (px, py, qx, qy) => {
      ptr(canvas, "pointermove", px, py); ptr(canvas, "pointerdown", px, py);
      ptr(window, "pointermove", (px + qx) / 2, (py + qy) / 2); ptr(window, "pointermove", qx, qy); ptr(window, "pointerup", qx, qy);
      canvas.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await wait(60);
    };
    log("start", tasks());
    // A's end three days on: B (after A) and C (after B) move on with it.
    await drag(x("2026-03-07") - 2, row(0), x("2026-03-07") - 2 + day * 6, row(0));
    log("cascade", tasks());
    // From D's dot onto C: C waits for D too.
    await drag(x("2026-03-05") + 10, row(3), x("2026-03-20"), row(2));
    log("linked", tasks(), S().links);
    // From C's dot onto A: A would wait for C, which waits for A. Refused.
    await drag(x("2026-03-23") + 10, row(2), x("2026-03-04"), row(0)); await wait(60);
    log("circle", tasks(), S().links, el("g").querySelector("[role=status]")?.textContent ?? "");
    // The arrow into B, clicked: B no longer waits for A.
    const bStart = x(S().tasks[1].start);
    ptr(canvas, "pointermove", bStart - 6, row(1)); ptr(canvas, "pointerdown", bStart - 6, row(1)); ptr(canvas, "pointerup", bStart - 6, row(1));
    canvas.dispatchEvent(new MouseEvent("click", { bubbles: true, clientX: canvas.getBoundingClientRect().left + bStart - 6, clientY: canvas.getBoundingClientRect().top + row(1) }));
    await wait(60);
    log("unlinked", tasks(), S().links);
    // From the keyboard: L on D, then Enter on B: B waits for D. Again: it no longer does.
    const row_ = (k) => el("g").querySelector(`[data-gantt-table] [data-key="${k}"]`);
    const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
    const said = async () => { await wait(60); return el("g").querySelector("[role=status]").textContent; };
    row_("d").focus(); key("l");
    log("linking", row_("d").hasAttribute("data-linking"), await said());
    row_("b").focus(); key("Enter"); await wait(60);
    log("key linked", tasks(), S().links, await said());
    row_("d").focus(); key("L"); row_("b").focus(); key("Enter"); await wait(60);
    log("key unlinked", tasks(), S().links);
    // A circle is refused (C waits for B, so B can't wait for C); Escape stops.
    row_("c").focus(); key("l"); row_("b").focus(); key("Enter"); await wait(60);
    log("key circle", tasks(), await said());
    row_("a").focus(); key("l"); key("Escape");
    log("escape", !!el("g").querySelector("[data-linking]"), await said());
"##;
    let Some(lines) = run("gantt-links", &engine, doc, &driver(body)) else { return };
    let expected = [
        "start a:03-02..03-06 b:03-09..03-13<a c:03-16..03-20<b d:03-02..03-04",
        // A ends on the 12th: B starts the 13th, C after B, each as long as it was.
        "cascade a:03-02..03-12 b:03-13..03-17<a c:03-18..03-22<b d:03-02..03-04",
        "linked a:03-02..03-12 b:03-13..03-17<a c:03-18..03-22<b+d d:03-02..03-04 1",
        "circle a:03-02..03-12 b:03-13..03-17<a c:03-18..03-22<b+d d:03-02..03-04 1 A can't wait for C: C waits for it",
        "unlinked a:03-02..03-12 b:03-13..03-17 c:03-18..03-22<b+d d:03-02..03-04 2",
        // The keyboard: L on D, Enter on B.
        "linking true Linking from D: go to the task that waits for it and press Enter, or Escape to stop",
        "key linked a:03-02..03-12 b:03-13..03-17<d c:03-18..03-22<b+d d:03-02..03-04 3 B now waits for D",
        "key unlinked a:03-02..03-12 b:03-13..03-17 c:03-18..03-22<b+d d:03-02..03-04 4",
        "key circle a:03-02..03-12 b:03-13..03-17 c:03-18..03-22<b+d d:03-02..03-04 B can't wait for C: C waits for it",
        "escape false Stopped linking",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// Picking a chart colour scheme (the `chart` theme option) recolours the charts on
/// the page; picking the default goes back to the kit's hues.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn chart_colour_schemes_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1, "root": { "type": "stack", "children": [
        { "type": "ui-theme-option", "props": { "option": "chart" } },
        { "type": "ui-chart", "attrs": { "id": "c" }, "props": { "type": "bar", "data": [{ "d": "a", "x": 1, "y": 2 }] } } ] } });
    let body = r##"
    await ready(["c"]);
    log("default", opt("c").color[0]);
    document.querySelector("[data-choice=blue]").click();
    await wait(100);
    log("blue", opt("c").color[0], opt("c").color[1]);
    document.querySelector("[data-choice=default]").click();
    await wait(100);
    log("back", opt("c").color[0]);
"##;
    let Some(lines) = run("schemes", &engine, doc, &driver(body)) else { return };
    assert_eq!(lines, ["default rgba(42,120,214,1)", "blue rgba(37,99,235,1) rgba(96,165,250,1)", "back rgba(42,120,214,1)"], "\n{}", lines.join("\n"));
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn untrusted_documents_in_chrome() {
    let mut engine = Engine::untrusted();
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let doc = json!({ "version": 1, "root": { "type": "ui-chart", "attrs": { "id": "c" }, "props": {
        "type": "bar", "data": [{ "n": "<img src=x onerror=alert(1)>", "v": 1 }],
        "option": {
            "title": { "text": "t", "link": "javascript:alert(1)" },
            "tooltip": { "formatter": "<b onmouseover=alert(1)>{b}</b><br/>{c}", "extraCssText": "background:url(https://evil)" } } } } });
    let body = r##"
    await ready(["c"]);
    const o = opt("c");
    log("formatter", o.tooltip[0].formatter);
    log("link", o.title[0].link === undefined || o.title[0].link === "");
    log("css", !String(o.tooltip[0].extraCssText).includes("evil"));
    // Show the tooltip: the category name is data, and ECharts escapes it.
    chart("c").dispatchAction({ type: "showTip", seriesIndex: 0, dataIndex: 0 });
    await wait(100);
    log("tooltip", document.querySelectorAll("img").length, document.body.innerHTML.includes("&lt;img src=x"));
"##;
    let Some(lines) = run("untrusted", &engine, doc, &driver(body)) else { return };
    let expected = [
        "formatter &lt;b onmouseover=alert(1)&gt;{b}&lt;/b&gt;<br/>{c}",
        "link true",
        "css true",
        "tooltip 0 true",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// Answers `GET /tick` with the next of an endless series of `{ t, v }` rows, and
/// `GET /events` with three server-sent events and a long `retry`, so the stream
/// ends. CORS headers let a file:// page call it.
fn tick_server() -> u16 {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let len = stream.read(&mut buf).unwrap_or(0);
            if String::from_utf8_lossy(&buf[..len]).starts_with("GET /events") {
                let events: String = (1..=3).map(|i| format!("data: {}\n\n", json!({ "id": i, "at": i, "amount": i * 5 }))).collect();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\nretry: 600000\n\n{events}"
                );
                continue;
            }
            let body = json!({ "t": n, "v": n * 10 }).to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    port
}

/// Sales rows: two regions, their countries and cities, two products, four days.
fn sales() -> Json {
    let places = [("Europe", "Germany", "Berlin"), ("Europe", "Germany", "Munich"), ("Europe", "France", "Paris"), ("Asia", "Japan", "Tokyo"), ("Asia", "India", "Pune")];
    let mut rows = Vec::new();
    for (d, day) in ["2026-11-01", "2026-11-02", "2026-11-03", "2026-11-04"].iter().enumerate() {
        for (i, (region, country, city)) in places.iter().enumerate() {
            for (k, product) in ["A", "B"].iter().enumerate() {
                let amount = 10 * (i + 1) + d + k * 100;
                rows.push(json!({ "date": day, "region": region, "country": country, "city": city, "product": product, "amount": amount, "qty": i + k }));
            }
        }
    }
    Json::Array(rows)
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn drill_filters_and_live_data_in_chrome() {
    let port = tick_server();
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let tick = format!("http://127.0.0.1:{port}/tick");
    let chart = |id: &str, props: Json| json!({ "type": "ui-chart", "attrs": { "id": id }, "props": props, "style": { "width": 480 } });
    let doc = json!({ "version": 1,
        "state": { "sales": sales(), "path": [], "region": null, "row": null, "products": [], "range": null,
                   "visible": null, "picked": [], "feed": [], "status": null, "paused": false, "drills": 0 },
        "root": { "type": "stack", "children": [
            json!({ "type": "ui-chart", "attrs": { "id": "drill" }, "style": { "width": 480 },
                "props": { "type": "bar", "data": "{{ sales }}", "drill": ["region", "country", "city"], "y": "amount", "bindDrill": "path" },
                "on": { "drill": { "action": "set_state", "key": "drills", "value": "{{ drills + 1 }}" } } }),
            chart("regions", json!({ "type": "bar", "data": "{{ sales }}", "x": "region", "y": "amount", "aggregate": "sum",
                "sort": "-amount", "bind": "region", "bindRow": "row" })),
            chart("filtered", json!({ "type": "pie", "data": "{{ sales }}", "x": "product", "y": "amount", "aggregate": "sum",
                "filter": { "region": "{{ region }}", "product": "{{ products }}", "date": "{{ range }}" } })),
            chart("time", json!({ "type": "line", "data": "{{ sales }}", "x": "date", "y": "amount", "aggregate": "sum",
                "zoom": "slider", "bindZoom": "range" })),
            chart("split", json!({ "type": "area", "data": "{{ sales }}", "x": "date", "y": "amount", "series": "product",
                "aggregate": "sum", "stack": true, "bindLegend": "visible" })),
            chart("brush", json!({ "type": "scatter", "data": "{{ sales }}", "x": "qty", "y": "amount", "brush": true, "bindBrush": "picked" })),
            json!({ "type": "ui-live-data", "props": { "url": tick, "every": 300, "mode": "append", "window": 5, "key": "t",
                "into": "feed", "status": "status", "paused": "{{ paused }}" } }),
            chart("live", json!({ "type": "line", "data": "{{ feed }}", "x": "t", "y": "v" })),
            chart("own", json!({ "type": "bar", "data": [], "x": "t", "y": "v", "live": { "url": tick, "every": 300, "mode": "append", "window": 3 } })),
            chart("sse", json!({ "type": "bar", "x": "at", "y": "amount",
                "live": { "url": format!("http://127.0.0.1:{port}/events"), "stream": true, "mode": "append", "key": "id" } })),
        ] } });
    let body = r##"
    const cats = (id, axis = "xAxis") => opt(id)[axis][0].data.join(",");
    // A real click on a category's bar.
    const click = (id, category, value) => {
      const c = chart(id), [px, py] = c.convertToPixel({ seriesIndex: 0 }, [category, value]);
      const canvas = el(id).querySelector("canvas"), r = canvas.getBoundingClientRect();
      for (const type of ["mousemove", "mousedown", "mouseup", "click"])
        canvas.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: r.left + px, clientY: r.top + py }));
    };
    const pie = () => opt("filtered").series[0].data.map((d) => d.name + "=" + d.value).join(",");
    await ready(["drill", "regions", "filtered", "time", "split", "brush", "live", "own", "sse"]);

    // Drill down two levels by clicking, then back up from state.
    log("drill", cats("drill"), opt("drill").series[0].data.join(","));
    click("drill", "Europe", 20); await wait(50);
    log("drill", JSON.stringify(S().path), cats("drill"), !!opt("drill").graphic?.[0]?.elements?.length);
    click("drill", "Germany", 20); await wait(50);
    log("drill", JSON.stringify(S().path), cats("drill"));
    click("drill", "Berlin", 20); await wait(50);
    log("leaf", JSON.stringify(S().path), S().drills);
    root.__irender.set("path", []); await wait(50);
    log("up", cats("drill"));

    // Click a region to filter the pie; again to clear it.
    log("regions", cats("regions"), pie());
    click("regions", "Europe", 50); await wait(50);
    log("region", S().region, S().row && S().row.amount, pie(), opt("regions").series[0].selectedMode);
    click("regions", "Europe", 50); await wait(50);
    log("cleared", S().region, S().row, pie());
    root.__irender.set("products", ["A"]); await wait(50);
    log("products", pie());
    // Every row has a product: `{ empty: true }` keeps none, `{ empty: false }` all.
    root.__irender.set("products", { empty: true }); await wait(50);
    log("empty", pie());
    root.__irender.set("products", { empty: false }); await wait(50);
    log("not empty", pie());
    root.__irender.set("products", []);

    // A date range from state zooms the time chart and filters the pie.
    root.__irender.set("range", { from: "2026-11-02", to: "2026-11-03" }); await wait(50);
    const dz = opt("time").dataZoom[0];
    log("range", dz.startValue === +echarts.time.parse("2026-11-02"), dz.endValue === +echarts.time.parse("2026-11-03"), pie());
    // Zooming the chart writes the range back.
    chart("time").dispatchAction({ type: "dataZoom", start: 0, end: 100 }); await wait(300);
    log("unzoomed", JSON.stringify(S().range), pie());
    const day = (d) => +echarts.time.parse(d);
    chart("time").dispatchAction({ type: "dataZoom", startValue: day("2026-11-03"), endValue: day("2026-11-04") }); await wait(300);
    log("zoomed", JSON.stringify(S().range));

    // Series from a field, and the legend in state both ways.
    log("split", opt("split").series.map((s) => s.name + ":" + s.data.length).join(","));
    root.__irender.set("visible", ["B"]); await wait(50);
    log("legend", JSON.stringify(opt("split").legend[0].selected));
    chart("split").dispatchAction({ type: "legendToggleSelect", name: "A" }); await wait(50);
    log("legend", JSON.stringify(S().visible));

    // Brush the points with qty 0..1 and amount 10..60.
    chart("brush").dispatchAction({ type: "brush", areas: [{ brushType: "rect", xAxisIndex: 0, coordRange: [[-0.5, 1.5], [0, 60]] }] });
    await wait(400);
    log("brush", S().picked.length, S().picked.every((r) => r.qty <= 1 && r.amount <= 60));
    root.__irender.set("picked", []); await wait(400);
    log("unbrushed", S().picked.length);

    // Live data: appended, keyed, windowed; paused from state.
    for (let i = 0; i < 60 && S().feed.length < 5; i++) await wait(100);
    await wait(700);
    log("live", S().status, S().feed.length, opt("live").xAxis[0].data.length, opt("own").xAxis[0].data.length);
    root.__irender.set("paused", true); await wait(50);
    const n = S().feed.map((r) => r.t).join(",");
    await wait(1000);
    log("paused", S().status, S().feed.map((r) => r.t).join(",") === n);
    root.__irender.set("paused", false);
    for (let i = 0; i < 30 && S().status !== "live"; i++) await wait(100);
    log("resumed", S().status);
    log("sse", el("sse").getAttribute("data-live") !== null, opt("sse").series[0].data.join(","));
"##;
    let Some(lines) = run("drill", &engine, doc, &driver(body)) else { return };
    let expected = [
        "drill Europe,Asia 1716,1544",
        "drill [\"Europe\"] Germany,France true",
        "drill [\"Europe\",\"Germany\"] Berlin,Munich",
        "leaf [\"Europe\",\"Germany\"] 2",
        "up Europe,Asia",
        "regions Europe,Asia A=630,B=2630",
        // Europe's rows only, and the clicked row is the region's total.
        "region Europe 1716 A=258,B=1458 single",
        "cleared   A=630,B=2630",
        "products A=630",
        "empty ",
        "not empty A=630,B=2630",
        // A time axis zooms in milliseconds: local midnight on 2 and 3 November.
        "range true true A=315,B=1315",
        "unzoomed null A=630,B=2630",
        "zoomed {\"from\":\"2026-11-03\",\"to\":\"2026-11-04\"}",
        "split A:4,B:4",
        "legend {\"A\":false,\"B\":true}",
        "legend [\"A\",\"B\"]",
        "brush 8 true",
        "unbrushed 0",
        "live live 5 5 3",
        "paused paused true",
        "resumed live",
        "sse true 5,10,15",
    ];
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

/// Value labels, tooltips and the value axis written as `format`, `currency` and
/// `decimals` say, in the document's language; without them, at most two decimals.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn value_formats_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let rows = json!([{ "d": "a", "v": 3580.045454545 }, { "d": "b", "v": 12500 }]);
    let doc = json!({ "version": 1, "lang": "de", "root": { "type": "stack", "children": [
        { "type": "ui-chart", "attrs": { "id": "plain" }, "props": { "type": "bar", "data": rows, "labels": true } },
        { "type": "ui-chart", "attrs": { "id": "money" }, "props": { "type": "bar", "data": rows, "labels": true, "format": "currency", "currency": "EUR", "decimals": 0 } },
        { "type": "ui-chart", "attrs": { "id": "share" }, "props": { "type": "line", "data": [{ "d": "a", "v": 0.256 }], "labels": true, "format": "percent", "decimals": 1 } },
        { "type": "ui-chart", "attrs": { "id": "pie" }, "props": { "type": "pie", "data": [{ "name": "a", "value": 1.23456 }], "decimals": 2 } } ] } });
    let body = r##"
    await ready(["plain", "money", "share", "pie"]);
    const label = (id, v) => opt(id).series[0].label.formatter({ value: v });
    const tip = (id, v) => opt(id).tooltip[0].valueFormatter(v);
    const axis = (id, v) => opt(id).yAxis[0].axisLabel.formatter(v);
    log("plain", label("plain", 3580.045454545), tip("plain", 3580.045454545), typeof opt("plain").yAxis[0].axisLabel.formatter);
    log("money", label("money", 3580.4), tip("money", 12500), axis("money", 2000), axis("money", 1500000));
    log("share", label("share", ["a", 0.256]), axis("share", 0.5));
    log("pie", tip("pie", 1.23456), tip("pie", 2));
"##;
    let Some(lines) = run("formats", &engine, doc, &driver(body)) else { return };
    // `&nbsp;`: the no-break spaces Intl puts before units, as the DOM dump writes them.
    assert_eq!(
        lines,
        [
            // No format: grouped in German, two decimals at most; the axis keeps its own.
            "plain 3.580,05 3.580,05 function",
            "money 3.580&nbsp;€ 12.500&nbsp;€ 2.000&nbsp;€ 1,5&nbsp;Mio.&nbsp;€",
            "share 25,6&nbsp;% 50,0&nbsp;%",
            "pie 1,23 2,00",
        ],
        "\n{}",
        lines.join("\n")
    );
}

/// With `series`, `sort` and `limit` rank the categories by their totals over the
/// series, and keep each category's series together.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn series_sort_and_limit_by_category_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    // North has the biggest single row (9) but the smallest total (9 + 0 = 9);
    // West's total is 12, South's 10.
    let rows = json!([
        { "region": "North", "channel": "web", "amount": 9 }, { "region": "North", "channel": "shop", "amount": 0 },
        { "region": "South", "channel": "web", "amount": 4 }, { "region": "South", "channel": "shop", "amount": 3 }, { "region": "South", "channel": "shop", "amount": 3 },
        { "region": "West", "channel": "web", "amount": 6 }, { "region": "West", "channel": "shop", "amount": 6 } ]);
    let doc = json!({ "version": 1, "root": { "type": "stack", "children": [
        { "type": "ui-chart", "attrs": { "id": "top" }, "props": { "type": "bar", "stack": true, "data": rows, "x": "region", "y": "amount", "series": "channel", "aggregate": "sum", "sort": "-amount", "limit": 2 } },
        { "type": "ui-chart", "attrs": { "id": "az" }, "props": { "type": "bar", "data": rows, "x": "region", "y": "amount", "series": "channel", "aggregate": "sum", "sort": "region" } } ] } });
    let body = r##"
    await ready(["top", "az"]);
    const show = (id) => opt(id).xAxis[0].data.join(",") + " | " + opt(id).series.map((s) => s.name + ":" + s.data.map((v) => v?.value ?? v).join(",")).join(" ");
    log("top", show("top"));
    log("az", show("az"));
"##;
    let Some(lines) = run("series-sort", &engine, doc, &driver(body)) else { return };
    assert_eq!(lines, ["top West,South | web:6,4 shop:6,6", "az North,South,West | web:9,4,6 shop:0,6,6"], "\n{}", lines.join("\n"));
}

/// `pareto`: bars largest first and the running percent on a second axis; past `limit`
/// the rest add up into Other, kept last; the bars after the threshold are faded; and
/// the percent follows state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn pareto_in_chrome() {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register_with(&mut engine.registry, kit());
    let defects = json!([
        { "cause": "Dents", "n": 12 }, { "cause": "Scratches", "n": 42 }, { "cause": "Gaps", "n": 8 },
        { "cause": "Labels", "n": 18 }, { "cause": "Misaligned", "n": 30 }, { "cause": "Missing", "n": 10 }, { "cause": "Bad", "n": "x" } ]);
    // Stacked by shift: Scratches 30 (day 20, night 10), Dents 25 (5, 20), Gaps 6, Labels 4.
    let by_shift = json!([
        { "cause": "Scratches", "shift": "Day", "n": 20 }, { "cause": "Scratches", "shift": "Night", "n": 10 },
        { "cause": "Dents", "shift": "Day", "n": 5 }, { "cause": "Dents", "shift": "Night", "n": 20 },
        { "cause": "Gaps", "shift": "Night", "n": 6 }, { "cause": "Labels", "shift": "Day", "n": 4 } ]);
    let tickets: Vec<Json> = ["Login", "Billing", "Login", "Bug", "Login", "Billing", "Slow", "Export"].iter().map(|r| json!({ "reason": r })).collect();
    let doc = json!({ "version": 1, "state": { "defects": defects, "limit": 80, "row": null }, "root": { "type": "stack", "children": [
        { "type": "ui-chart", "attrs": { "id": "p" }, "props": { "type": "pareto", "data": "{{ defects }}", "x": "cause", "y": "n", "threshold": "{{ limit }}" } },
        { "type": "ui-chart", "attrs": { "id": "c" }, "props": { "type": "pareto", "data": tickets, "x": "reason", "aggregate": "count", "limit": 2, "threshold": false, "bindRow": "row" } },
        { "type": "ui-chart", "attrs": { "id": "h" }, "props": { "type": "pareto", "data": "{{ defects }}", "x": "cause", "y": "n", "horizontal": true, "labels": true } },
        { "type": "ui-chart", "attrs": { "id": "s" }, "props": { "type": "pareto", "data": by_shift, "x": "cause", "y": "n", "series": "shift", "limit": 2 } },
        { "type": "ui-chart", "attrs": { "id": "o" }, "props": { "type": "pareto", "data": tickets, "x": "reason", "aggregate": "count", "limit": 1, "other": "sorted", "targets": [50, { "value": 95, "label": "Goal" }, 120] } } ] } });
    let body = r##"
    await ready(["p", "c", "h", "s", "o"]);
    const faded = (id) => opt(id).series[0].data.map((v) => (v?.itemStyle ? "f" : "-")).join("");
    const show = (id) => [opt(id).xAxis[0].data.join(","), opt(id).series[0].data.map((v) => v?.value ?? v).join(","), opt(id).series[1].data.join(","), faded(id),
      opt(id).series[1].yAxisIndex, opt(id).yAxis[1].max, JSON.stringify(opt(id).series[1].markLine?.data?.[0]?.yAxis ?? null)].join(" | ");
    log("defects", show("p"));
    log("count", show("c"));
    log("legend", opt("p").legend[0].data?.join(",") ?? opt("p").series.map((s) => s.name).join(","));
    root.__irender.set("limit", 60); await wait(100);
    log("60%", faded("p"), opt("p").series[1].markLine.data[0].yAxis);
    // Across: categories down the side, largest at the top; the percent along the top.
    const h = opt("h");
    log("across", h.yAxis[0].type, h.yAxis[0].inverse, h.yAxis[0].data[0], h.xAxis.map((a) => a.max ?? "-").join(","), h.series[1].xAxisIndex, h.series[1].markLine.data[0].xAxis, h.series[0].label.position, faded("h"));
    // Drawn: the largest at the top, its bar the longest.
    const px = (c, v) => chart("h").convertToPixel({ yAxisIndex: 0, xAxisIndex: 0 }, [v, c]);
    log("drawn", px("Scratches", 42)[1] < px("Gaps", 8)[1], px("Scratches", 42)[0] > px("Gaps", 8)[0]);
    // A real click on Other: its row holds the rows that went into it.
    const [ox, oy] = chart("c").convertToPixel({ seriesIndex: 0 }, ["Other", 1]), canvas = el("c").querySelector("canvas"), cr = canvas.getBoundingClientRect();
    for (const type of ["mousemove", "mousedown", "mouseup", "click"]) canvas.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: cr.left + ox, clientY: cr.top + oy }));
    await wait(50);
    log("other", S().row?.reason, S().row?.count, (S().row?.rows ?? []).map((r) => r.reason + ":" + r.count).join(","));
    // Stacked: a bar a part (Night's total is the larger), the line on the whole bars.
    const st = opt("s");
    log("stacked", st.xAxis[0].data.join(","), st.series.map((x) => x.name + ":" + (x.stack ?? "-") + ":" + x.data.map((v) => v?.value ?? v).join("/")).join(" "));
    // Other sorted in by its size; the targets as more lines, each naming the bar it's reached at.
    const o = opt("o");
    log("sorted", o.xAxis[0].data.join(","), o.series[1].markLine.data.map((m) => m.yAxis + ":" + (m.label?.formatter ?? "-")).join(" / "));
"##;
    let Some(lines) = run("pareto", &engine, doc, &driver(body)) else { return };
    assert_eq!(
        lines,
        [
            // 120 in all; 80% is passed at Labels (75%) → Dents (85%), so Missing and Gaps fade.
            "defects Scratches,Misaligned,Labels,Dents,Missing,Gaps | 42,30,18,12,10,8 | 35,60,75,85,93.3,100 | ----ff | 1 | 100 | 80",
            // Counted, the top two kept and the other three tickets in Other.
            "count Login,Billing,Other | 3,2,3 | 37.5,62.5,100 | --- | 1 | 100 | null",
            "legend n,Cumulative %",
            "60% --ffff 60",
            "across category true Scratches -,100 1 80 right ----ff",
            "drawn true true",
            "other Other 3 Bug:1,Slow:1,Export:1",
            // 65 in all: Scratches 30, Dents 25, and Gaps and Labels in Other (10).
            "stacked Scratches,Dents,Other Night:pareto:10/20/6 Day:pareto:20/5/4 Cumulative %:-:46.2/84.6/100",
            // Other (5) before Login (3); 120 is off the axis and left out.
            "sorted Other,Login 80:- / 50:50% at Other / 95:Goal 95% at Login",
        ],
        "\n{}",
        lines.join("\n")
    );
}
