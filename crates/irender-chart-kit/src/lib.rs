//! Charts for iRender, drawn by [Apache ECharts](https://echarts.apache.org/).
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_chart_kit::register(&mut engine.registry);
//!
//! let mut doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-chart",
//!     "props": { "type": "bar", "data": [{ "day": "Mon", "orders": 120 }, { "day": "Tue", "orders": 200 }] } } }"#,
//!     Format::Json)?;
//! irender_chart_kit::apply_theme(&mut doc); // optional: the default series colours
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `ui-chart` takes either a shorthand (`type`, `data`, `x`, `y`, …) that builds an
//! ECharts option from rows, or a full ECharts `option`, or both: the option is
//! merged over what the shorthand builds. Any ECharts series or component works
//! through `option`. Every prop can use state, so a chart redraws when the state it
//! reads changes, and `bind` writes the clicked item back to state.
//!
//! Colours, text and grid lines come from the theme's tokens (`color.text`,
//! `color.muted`, `color.border`, `color.surface` and `color.chart-1` … `chart-8`),
//! so charts follow light and dark mode and sit naturally next to irender-ui-kit.
//!
//! ECharts itself (about 360 KB gzipped) is loaded once per page from
//! [`ChartKit::src`], only on pages that have a chart.

mod theme;

pub use theme::{apply_theme, theme};

use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val, expr};
use irender_html::{Component, RenderCx, RenderError, Registry};
use serde_json::{Map, Value as Json, json};

const SCRIPT: &str = include_str!("chart.js");

/// The ECharts release the kit is written and tested against.
pub const ECHARTS_VERSION: &str = "6.1.0";

/// Where pages load ECharts from by default.
pub const ECHARTS_CDN: &str = "https://cdn.jsdelivr.net/npm/echarts@6.1.0/dist/echarts.min.js";

/// How the kit's charts load and draw.
#[derive(Debug, Clone)]
pub struct ChartKit {
    /// The URL of `echarts.min.js`, loaded once per page that has a chart. Point it
    /// at your own copy to avoid the CDN, e.g. `/static/echarts.min.js`. ECharts
    /// extensions (echarts-gl, custom series) can be loaded by the host page.
    pub src: String,
    /// `canvas` (default) or `svg`; a chart's `renderer` prop overrides it.
    pub renderer: Renderer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    Canvas,
    Svg,
}

impl Renderer {
    fn as_str(self) -> &'static str {
        match self {
            Renderer::Canvas => "canvas",
            Renderer::Svg => "svg",
        }
    }
}

impl Default for ChartKit {
    fn default() -> Self {
        Self { src: ECHARTS_CDN.into(), renderer: Renderer::Canvas }
    }
}

/// Adds `ui-chart`, `ui-sparkline` and `ui-live-data` to `registry`, loading
/// ECharts from the CDN.
pub fn register(registry: &mut Registry) {
    register_with(registry, ChartKit::default());
}

/// Adds `ui-chart`, `ui-sparkline` and `ui-live-data` to `registry` with your own
/// settings.
pub fn register_with(registry: &mut Registry, kit: ChartKit) {
    registry
        .register(Chart { kit: kit.clone(), sparkline: false })
        .register(Chart { kit, sparkline: true })
        .register(LiveData);
}

/// Series types the shorthand can build. Anything else ECharts draws (maps, lines,
/// custom series, calendars, matrices, 3D with echarts-gl) goes in `option`.
pub const TYPES: &[&str] = &[
    "line", "area", "bar", "scatter", "effectScatter", "pictorialBar", "candlestick", "boxplot", "pie", "donut",
    "funnel", "radar", "heatmap", "gauge", "tree", "treemap", "sunburst", "sankey", "graph", "chord", "themeRiver",
    "parallel", "gantt",
];

const CHART_PROPS: &[PropDef] = &[
    PropDef::new("type", PropKind::Enum(TYPES), "Series type the shorthand builds; leave it out to use `option` alone"),
    PropDef::new("data", PropKind::Any, "Rows (objects or arrays), numbers, or a tree / node list, depending on `type`"),
    PropDef::new("x", PropKind::Any, "Field (or column) for categories, names or the x value; default the first"),
    PropDef::new(
        "y",
        PropKind::Any,
        "Field(s) for values: a field, or a list of fields or `{ field, name, …series options }`; default every other numeric field",
    ),
    PropDef::new("value", PropKind::Any, "Third field: heatmap cell value, scatter bubble size"),
    PropDef::new("links", PropKind::Any, "`{ source, target, value }` for sankey, graph and chord"),
    PropDef::new("option", PropKind::Any, "An ECharts option, merged over what the shorthand builds. `{{ }}` works inside it"),
    PropDef::new(
        "filter",
        PropKind::Any,
        "Keep rows whose fields match: `{ region: \"{{ region }}\", date: { from, to }, product: [a, b] }`; blank conditions pass",
    ),
    PropDef::new("aggregate", PropKind::Enum(&["sum", "avg", "min", "max", "count"]), "Group rows by `x` (and `series`)"),
    PropDef::new("series", PropKind::Any, "Field whose values become separate series (long-format rows)"),
    PropDef::new("sort", PropKind::Any, "A field, `-field` for descending, or a list of them"),
    PropDef::new("limit", PropKind::Any, "Keep the first N rows after sorting"),
    PropDef::new("drill", PropKind::Any, "Fields to drill through on click, e.g. `[region, country, city]`"),
    PropDef::new("xType", PropKind::Enum(&["category", "time", "value", "log"]), "The x axis; default: time for dates on lines and points"),
    PropDef::new("zoom", PropKind::Any, "Zoom controls along the category or time axis: `inside`, `slider` or `both` (`true`)"),
    PropDef::new("brush", PropKind::Any, "Drag to select: `rect` (`true`), `polygon`, `lineX` or `lineY`"),
    PropDef::new(
        "live",
        PropKind::Any,
        "Live data: `{ url, every, mode: append, window, key, params, paused, stream, pick, into, status }`",
    ),
    PropDef::new(
        "columns",
        PropKind::Any,
        "Gantt: the table's columns, a field or `{ field, name, width, align }`; the first holds the tree of names (default `[name]`)",
    ),
    PropDef::new("markers", PropKind::Any, "Gantt: dated lines down the timeline, `[{ at, name, color }]`"),
    PropDef::new("today", PropKind::Bool, "Gantt: a line at today's date when it's on the timeline (default true)"),
    PropDef::new("bindData", PropKind::StateKey, "Gantt: the state key `data` reads; bars dragged (or moved with Alt and the arrows) write their new dates back to it"),
    PropDef::new("editable", PropKind::Bool, "Gantt: with `bindData`, bars can be moved and stretched, and links drawn and removed (default true)"),
    PropDef::new("cascade", PropKind::Bool, "Gantt: moving or stretching a task moves on the tasks that wait on it, so they start after it ends"),
    PropDef::new("stack", PropKind::Bool, "Stack the series"),
    PropDef::new("smooth", PropKind::Bool, "Smooth lines"),
    PropDef::new("horizontal", PropKind::Bool, "Bars along the x axis"),
    PropDef::new("labels", PropKind::Bool, "Show value labels on the marks"),
    PropDef::new("legend", PropKind::Bool, "Show the legend (default: with two or more series)"),
    PropDef::new("tooltip", PropKind::Bool, "Show a tooltip on hover (default true)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 320"),
    PropDef::new("renderer", PropKind::Enum(&["canvas", "svg"]), "Overrides the kit's renderer"),
    PropDef::new("loading", PropKind::Any, "Show the loading mask, e.g. `{{ fetching }}`"),
    PropDef::new("bind", PropKind::StateKey, "Two-way: the clicked item's name; clicking it again clears it"),
    PropDef::new("bindRow", PropKind::StateKey, "The clicked item's row; clicking it again clears it"),
    PropDef::new("bindZoom", PropKind::StateKey, "Two-way: the zoom window as `{ from, to }` axis values, or null"),
    PropDef::new("bindLegend", PropKind::StateKey, "Two-way: the names of the visible series"),
    PropDef::new("bindBrush", PropKind::StateKey, "The rows inside the brushed area"),
    PropDef::new("bindDrill", PropKind::StateKey, "Two-way: the drill path, e.g. `[Europe, Germany]`"),
    PropDef::new("maps", PropKind::Any, "GeoJSON maps to register first: `{ world: /geo/world.json }`"),
    PropDef::new("group", PropKind::String, "Charts in the same group share tooltips and zoom"),
    PropDef::new("title", PropKind::Any, "Accessible name, read before the generated description"),
];

const SPARKLINE_PROPS: &[PropDef] = &[
    PropDef::new("data", PropKind::Any, "Numbers, or rows with `y`").required(),
    PropDef::new("y", PropKind::Any, "Field holding the value, when `data` has rows"),
    PropDef::new("type", PropKind::Enum(&["line", "area", "bar"]), "Default `area`"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 40"),
    PropDef::new("width", PropKind::Any, "Number (px) or CSS length; default 100%"),
    PropDef::new("tooltip", PropKind::Bool, "Show the value on hover (default true)"),
    PropDef::new("option", PropKind::Any, "An ECharts option, merged over the sparkline's"),
    PropDef::new("live", PropKind::Any, "Live data, as for `ui-chart`"),
];

/// Props the browser resolves against state; the rest are read once on the server.
const LIVE: &[&str] = &[
    "type", "data", "x", "y", "value", "links", "option", "filter", "aggregate", "series", "sort", "limit", "drill",
    "xType", "zoom", "brush", "live", "stack", "smooth", "horizontal", "labels", "legend", "tooltip", "loading",
    "columns", "markers", "today", "editable", "cascade",
];

/// State keys a chart writes to, and for the two-way ones reads from.
const BINDINGS: &[&str] = &["bind", "bindRow", "bindZoom", "bindLegend", "bindBrush", "bindDrill", "bindData"];

/// `ui-chart`, or `ui-sparkline` when `sparkline` is set.
struct Chart {
    kit: ChartKit,
    sparkline: bool,
}

impl Component for Chart {
    fn name(&self) -> &'static str {
        if self.sparkline { "ui-sparkline" } else { "ui-chart" }
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: if self.sparkline {
                "A small chart without axes, for a trend inside text or a stat card."
            } else {
                "A chart drawn by Apache ECharts, from rows (`type`, `data`, `x`, `y`) or a full ECharts `option`."
            },
            props: if self.sparkline { SPARKLINE_PROPS } else { CHART_PROPS },
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if !self.sparkline && node.prop("type").is_none() && node.prop("option").is_none() {
            return Err(RenderError::new(node, "a chart needs a `type` or an `option`"));
        }
        let mut props = Map::new();
        let mut reactive = false;
        for name in LIVE {
            if let Some(v) = node.prop(name) {
                let (value, live) = dynamic(cx, node, name, v)?;
                reactive |= live;
                props.insert(name.to_string(), value);
            }
        }
        if props.get("option").is_some_and(|o| !(o.is_object() || o.is_null() || has_template(o))) {
            return Err(RenderError::new(node, "`option` must be an object"));
        }
        if let Some(live) = props.get("live") {
            check_source(cx, node, live.get("url"))?;
        }

        let renderer = match cx.prop_string(node, "renderer").as_deref() {
            Some("svg") => "svg",
            Some("canvas") => "canvas",
            _ => self.kit.renderer.as_str(),
        };
        let mut maps = Map::new();
        if let Some(m) = cx.prop(node, "maps").filter(|m| !m.is_null()) {
            let Json::Object(m) = m else {
                return Err(RenderError::new(node, "`maps` must map names to GeoJSON URLs"));
            };
            for (name, url) in m {
                let url = expr::stringify(&url);
                if !cx.policy().url_allowed(&url) {
                    return Err(RenderError::new(node, format!("map `{name}`: URL not allowed by the policy")));
                }
                maps.insert(name, url.into());
            }
        }
        let policy = cx.policy();
        let mut config = json!({
            "src": self.kit.src,
            "renderer": renderer,
            "sparkline": self.sparkline,
            // Documents the policy doesn't trust can't put markup in tooltips.
            "safe": !(policy.allow_event_attributes && policy.allow_unsafe_urls),
            "reactive": reactive,
            "props": props,
            "maps": maps,
            "group": cx.prop_string(node, "group"),
            "title": cx.prop_string(node, "title"),
            "autoHeight": !self.sparkline && node.prop("height").is_none(),
        });
        for key in BINDINGS {
            config[key] = cx.prop_string(node, key).into();
        }
        cx.global_js("ui-chart", SCRIPT);

        let (height, width) = if self.sparkline { (40, "100%") } else { (gantt_height(node, cx).unwrap_or(320), "100%") };
        // A chart's geometry runs left to right on any page; its text still reads its own way.
        let mut sizing = StyleBlock::from_pairs(&[("position", "relative"), ("min-width", "0"), ("direction", "ltr")]);
        for (property, default) in [("width", Val::Lit(width.into())), ("height", Val::Lit(height.into()))] {
            let v = node.prop(property).cloned().unwrap_or(default);
            if let Some(v) = cx.style_value(node, property, &v)? {
                sizing.push(property, v);
            }
        }
        if self.sparkline {
            sizing = sizing.set("display", "inline-block").set("vertical-align", "middle");
        }
        let mut el = cx.element("div", node);
        el.base_style(sizing).behavior("ui-chart", config);
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

/// The height a gantt starts at, before the script fits it to the rows it shows: a
/// heading band, a row per task in `data` (from the document or the starting
/// state), and room for a legend or zoom buttons. Matches `GANTT` in chart.js.
fn gantt_height(node: &Node, cx: &RenderCx<'_>) -> Option<usize> {
    let lit = |name| match node.prop(name) {
        Some(Val::Lit(v)) => Some(v),
        _ => None,
    };
    if lit("type")?.as_str()? != "gantt" {
        return None;
    }
    let rows = cx.prop(node, "data")?.as_array()?.len().max(1);
    let band = if node.prop("series").is_some() || node.prop("zoom").is_some() { 30 } else { 0 };
    let slider = if lit("zoom").and_then(Json::as_str).is_some_and(|z| z == "slider" || z == "both") { 36 } else { 0 };
    let foot = if node.prop("markers").is_some() { 28 } else { 12 };
    Some(band + 44 + rows * 34 + foot + slider)
}

/// `ui-live-data`: keeps a state key filled from a polled URL, a server-sent event
/// stream or a WebSocket. It renders a hidden element and nothing else.
struct LiveData;

const LIVE_DATA_PROPS: &[PropDef] = &[
    PropDef::new("url", PropKind::Any, "Polled with GET, or a `ws://` / `wss://` WebSocket; may use state").required(),
    PropDef::new("into", PropKind::StateKey, "State key that holds the data").required(),
    PropDef::new("every", PropKind::Any, "Milliseconds between polls (default 5000, at least 250)"),
    PropDef::new("mode", PropKind::Enum(&["replace", "append"]), "Replace the data with each message (default), or append to it"),
    PropDef::new("window", PropKind::Any, "With `append`: keep the last N rows (default 500)"),
    PropDef::new("key", PropKind::Any, "With `append`: a row with the same value in this field replaces the old one"),
    PropDef::new("params", PropKind::Any, "Query parameters; blank ones are left out, and a change reconnects"),
    PropDef::new("paused", PropKind::Any, "Stop while true, e.g. `{{ !liveMode }}`"),
    PropDef::new("stream", PropKind::Bool, "Listen to `url` as server-sent events instead of polling"),
    PropDef::new("pick", PropKind::String, "Path to the data in each message, e.g. `data.items`"),
    PropDef::new("status", PropKind::StateKey, "State key set to connecting, live, paused or error"),
];

/// The props `ui-live-data` resolves in the browser.
const LIVE_DATA_LIVE: &[&str] = &["url", "every", "mode", "window", "key", "params", "paused", "stream", "pick"];

impl Component for LiveData {
    fn name(&self) -> &'static str {
        "ui-live-data"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Keeps a state key filled from a polled URL, server-sent events or a WebSocket, for any component to read.",
            props: LIVE_DATA_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut props = Map::new();
        for name in LIVE_DATA_LIVE {
            if let Some(v) = node.prop(name) {
                props.insert(name.to_string(), dynamic(cx, node, name, v)?.0);
            }
        }
        check_source(cx, node, props.get("url"))?;
        let config = json!({
            "props": props,
            "into": cx.prop_string(node, "into"),
            "status": cx.prop_string(node, "status"),
        });
        // The chart script carries the behaviour; ECharts itself isn't loaded for it.
        cx.global_js("ui-chart", SCRIPT);
        let mut el = cx.element("div", node);
        el.attr("hidden", true).behavior("ui-live-data", config);
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

/// A live source's URL, when it is known on the server, must pass the policy. One
/// built from state is checked in the browser, which refuses script URLs.
fn check_source(cx: &RenderCx<'_>, node: &Node, url: Option<&Json>) -> Result<(), RenderError> {
    match url {
        Some(Json::String(u)) if !cx.policy().url_allowed(u) => {
            Err(RenderError::new(node, "live data: URL not allowed by the policy"))
        }
        _ => Ok(()),
    }
}

/// A prop as the browser needs it: its value, with every template that reads state
/// (at the top or anywhere inside an object literal) left as `{ "$irender": js }`
/// and every other template evaluated now. Also says whether any template was live.
fn dynamic(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Val) -> Result<(Json, bool), RenderError> {
    match v {
        Val::Tpl(_) if cx.is_reactive(v) => Ok((json!({ "$irender": cx.val_js(v) }), true)),
        Val::Tpl(_) => Ok((cx.eval(v), false)),
        Val::Lit(lit) => {
            let mut live = false;
            let out = nested(cx, node, name, lit, &mut live)?;
            Ok((out, live))
        }
    }
}

fn nested(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Json, live: &mut bool) -> Result<Json, RenderError> {
    Ok(match v {
        Json::String(s) if Template::is_template(s) => {
            let t = Template::parse(s).map_err(|e| RenderError::new(node, format!("`{name}`: {e}")))?;
            let (value, l) = dynamic(cx, node, name, &Val::Tpl(t))?;
            *live |= l;
            value
        }
        Json::Array(items) => Json::Array(items.iter().map(|i| nested(cx, node, name, i, live)).collect::<Result<_, _>>()?),
        Json::Object(o) => Json::Object(
            o.iter().map(|(k, v)| Ok((k.clone(), nested(cx, node, name, v, live)?))).collect::<Result<_, RenderError>>()?,
        ),
        other => other.clone(),
    })
}

fn has_template(v: &Json) -> bool {
    v.as_object().is_some_and(|o| o.len() == 1 && o.contains_key("$irender"))
}
