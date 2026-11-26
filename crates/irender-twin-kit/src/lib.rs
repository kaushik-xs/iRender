//! Digital twins for iRender: 3D worlds drawn with [three.js](https://threejs.org/), with
//! machine and sensor data on them.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_twin_kit::register(&mut engine.registry);
//! let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-twin", "props": {
//!     "models": { "pump": "models/pump.glb" },
//!     "objects": [{ "id": "p1", "model": "pump", "position": [2, 3], "title": "Pump 1", "temp": 71 }],
//!     "readouts": [{ "field": "temp", "label": "Temperature", "unit": "°C", "warn": 70, "alarm": 85 }] } } }"#, Format::Json)?;
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The whole world is configuration: the models it uses (glTF and GLB, OBJ, FBX, STL, PLY,
//! Collada, 3MF, 3DS, VRML and USD), the objects placed in it, the paths they follow, zones,
//! walls, the ground, the light and the camera's views. Each may use state, come as one
//! `world` object, or be fetched from `src`. Values from machines and sensors come from
//! state or the twin's own `live` feed; `readouts` show them on the objects and turn them
//! warning or alarm past their limits, and `colorBy` colours objects by a value.
//!
//! three.js isn't inlined: a page loads it, once, from the CDN (or [`TwinKit::three`])
//! when a twin comes near the view.

use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val};
use irender_html::{ClientList, Component, Element, Registry, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("twin.js");

/// The three.js release the kit is written and tested against.
pub const THREE_VERSION: &str = "0.183.2";

/// The meshoptimizer release whose simplifier makes models simpler far away.
pub const MESHOPT_VERSION: &str = "1.3.0";

/// Where three.js comes from.
#[derive(Debug, Clone)]
pub struct TwinKit {
    /// three.js as an ES module.
    pub three: String,
    /// Its add-ons (loaders, controls), with `{name}` for the path under `examples/jsm/`,
    /// e.g. `loaders/GLTFLoader.js`. They must import three.js from the same address as
    /// [`TwinKit::three`]: jsDelivr's `+esm` files do.
    pub addons: String,
    /// The folder with the Draco decoder, for compressed glTF meshes.
    pub draco: String,
    /// The folder with the Basis transcoder, for KTX2 textures.
    pub basis: String,
    /// meshoptimizer's simplifier as an ES module, for simpler models far away (`lod`).
    pub simplifier: String,
}

impl Default for TwinKit {
    fn default() -> Self {
        let base = format!("https://cdn.jsdelivr.net/npm/three@{THREE_VERSION}");
        Self {
            three: format!("{base}/+esm"),
            addons: format!("{base}/examples/jsm/{{name}}/+esm"),
            draco: format!("{base}/examples/jsm/libs/draco/gltf/"),
            basis: format!("{base}/examples/jsm/libs/basis/"),
            simplifier: format!("https://cdn.jsdelivr.net/npm/meshoptimizer@{MESHOPT_VERSION}/meshopt_simplifier.js"),
        }
    }
}

/// Adds `ui-twin`, `ui-twin-popup` and `ui-twin-panel` to `registry`, with three.js from the CDN.
pub fn register(registry: &mut Registry) {
    register_with(registry, TwinKit::default());
}

/// Adds `ui-twin`, `ui-twin-popup` and `ui-twin-panel` to `registry` with your own settings.
pub fn register_with(registry: &mut Registry, kit: TwinKit) {
    registry.register(Twin { kit }).register(Inside { name: "ui-twin-popup" }).register(Inside { name: "ui-twin-panel" });
}

/// The names the open object and its index are in scope under, in a `ui-twin-popup`.
const OBJECT: &str = "object";
const INDEX: &str = "index";

struct Twin {
    kit: TwinKit,
}

const PROPS: &[PropDef] = &[
    PropDef::new(
        "models",
        PropKind::Any,
        "The models by name: `{ forklift: models/forklift.glb, pump: { url, scale, units: m | cm | mm | in | ft, up: y | z, rotate, fit, origin: base | center | none, format, mtl, color } }`. \
         glTF / GLB (with Draco, meshopt and KTX2), OBJ (with `mtl`), FBX, STL, PLY, DAE, 3MF, 3DS, WRL and USD / USDZ. `fit`: the largest side in metres, or `[w, h, d]` to fit in. \
         Each is made to stand on the ground at its middle unless `origin` says otherwise. \
         `far`: how it's drawn far from the camera, a simpler model's url, `{ url, distance }`, `{ ratio, distance }` (made simpler in the browser, `ratio` of its triangles) or `false` (see `lod`)",
    ),
    PropDef::new(
        "objects",
        PropKind::Any,
        "What's in the world, as rows: `{ id, model | shape: box | cylinder | sphere | cone | sensor | point, size, color, position: [x, z] | [x, y, z], rotation (degrees, or [x, y, z]), scale, \
         title, description, label, tone, alarm, effect, path, speed, progress, loop, face, attach, animation, opacity, clickable, href, … }`, in metres with y up. \
         Any other fields are the object's data (a machine's speed, a sensor's value): `readouts` show them. Usually from data or state, so they move when it changes",
    ),
    PropDef::new("readouts", PropKind::Any, "Values shown on the objects that have them: `[{ field, label, unit, decimals, warn, alarm, below, tag, when, models, history }]`. Past `warn` an object turns warning, past `alarm` it's in alarm (`below: true`: under them); `when` (a `filter`) or `models` limit a readout to some objects, so each kind can have its own limits. The first two are on its tag (`tag` says otherwise); all are in its popup. `history: n` keeps an object's last n values (a point each time it changes, up to 500) and draws them as a sparkline in its popup"),
    PropDef::new("colorBy", PropKind::Any, "Colours objects by a value: `{ field, min, max, colors: [blue, green, yellow, red] }`, e.g. temperatures"),
    PropDef::new("live", PropKind::Any, "Machine and sensor values from a feed of the twin's own: `{ url, every: 5000 }` polled, or `{ stream: url }` (server-sent events). Each answer is rows with an `id`, or `{ <id>: { field: value } }`, merged into the objects with that id"),
    PropDef::new("paths", PropKind::Any, "Routes: `{ id, points: [[x, z], …], closed, width, tone, color, flow, height, show }`. `flow: true` (or a speed) moves stripes along it; an object with `path` follows it at `speed` m/s (default 1), or stands at `progress` (0 to 1)"),
    PropDef::new("zones", PropKind::Any, "Areas on the ground: `{ id, points: [[x, z], …] | center: [x, z] and size: [w, d], tone, color, label, title, description, height, opacity }`. With `bindZone` or a `title` they can be clicked"),
    PropDef::new("walls", PropKind::Any, "Walls: `{ points: [[x, z], …], closed, height (default 3), thickness (default 0.2), color, opacity }`"),
    PropDef::new("ground", PropKind::Any, "`{ size: [w, d], center, color, grid: 1 (metres a cell) | false }`, or false for none. Default: big enough for everything, with a 1 m grid"),
    PropDef::new("environment", PropKind::Any, "`{ preset: studio | day | night, light (default 1), shadows (default true), background, fog }`"),
    PropDef::new("camera", PropKind::Any, "`{ position: [x, y, z], target: [x, y, z], fov }`. The camera moves there when it changes. Default: fitted to the world"),
    PropDef::new("views", PropKind::Any, "Named views, shown as buttons: `[{ id, label, position, target }]`"),
    PropDef::new("world", PropKind::Any, "All of the above in one object (`{ models, objects, paths, zones, walls, ground, environment, camera, views, readouts, colorBy }`), e.g. a layout kept in state; a prop of its own wins"),
    PropDef::new("src", PropKind::Any, "The address of such a world as JSON, fetched in the browser"),
    PropDef::new("follow", PropKind::Any, "The id of an object the camera follows, keeping its distance"),
    PropDef::new("filter", PropKind::Any, "Shows only the objects that pass: `{ field: condition }`, as `ui-data-table`'s and `ui-chart`'s `filter`, e.g. a `ui-filters` panel's `bind`"),
    PropDef::new("toneBy", PropKind::Any, "Tones objects by a field: `{ field: status, tones: { running: success, idle: neutral, fault: danger } }`; a ring in it is drawn under each"),
    PropDef::new("alarmBy", PropKind::Any, "Which objects are in alarm: `{ field: status, values: [fault] }`, or `alarm: true` on a row, or a readout past `alarm`. They glow red, get a `!` tag, and a new one is announced"),
    PropDef::new("effectBy", PropKind::Any, "Animates objects by a field: `{ field: state, effects: { running: spin } }`, or `effect` on a row: `pulse`, `glow`, `blink`, `spin` (`spin`: degrees a second) or `bob`. Still, where the viewer asked for less motion"),
    PropDef::new("labels", PropKind::Enum(&["auto", "all", "none"]), "Tags over objects: `auto` (default) for those with a `label`, readouts or an alarm; `all`; `none` (alarms still)"),
    PropDef::new("editable", PropKind::Any, "Objects can be dragged on the ground, or moved with Alt and the arrow keys and turned with Alt+R (needs `bindObjects`)"),
    PropDef::new("snap", PropKind::Any, "Metres moves snap to, e.g. 0.5"),
    PropDef::new("palette", PropKind::Any, "While `editable`, things to add, as rows like `objects` without a place: `[{ label, model | shape, title, … }]`. Clicked, one is added in the middle of the view; dragged into it, where it's dropped. Delete removes the selected object"),
    PropDef::new("bind", PropKind::StateKey, "State key with the selected object's id (null when none). Setting it selects that object and brings it into view"),
    PropDef::new("bindObject", PropKind::StateKey, "State key with the selected object itself, with its latest values"),
    PropDef::new("bindZone", PropKind::StateKey, "State key with the id of the zone last clicked"),
    PropDef::new("bindView", PropKind::StateKey, "State key with the current view's id; setting it moves the camera there"),
    PropDef::new("bindAlarms", PropKind::StateKey, "State key with the ids of the objects in alarm, e.g. for a count"),
    PropDef::new("bindObjects", PropKind::StateKey, "State key the objects are in: a moved one's `position` and `rotation` are written back to its row"),
    PropDef::new("bindLive", PropKind::StateKey, "State key that gets the latest values from `live`, as `{ <id>: { field: value } }`, e.g. for a list or totals beside the twin"),
    PropDef::new("bindCamera", PropKind::StateKey, "State key kept at `{ position, target }` after the camera's moved, e.g. to copy into `views`"),
    PropDef::new("popup", PropKind::Bool, "Opens a popup over the selected object, with its title, description and readouts or the `ui-twin-popup` (default true)"),
    PropDef::new("controls", PropKind::Bool, "Zoom and fit buttons (default true)"),
    PropDef::new("gestures", PropKind::Enum(&["orbit", "none"]), "`orbit` (default): drag to turn, right-drag or two fingers to move, wheel or pinch to zoom. `none`: the camera stays put"),
    PropDef::new("transition", PropKind::Number, "Milliseconds an object takes to move to a new place from the data (default 600; 0 jumps)"),
    PropDef::new("quality", PropKind::Enum(&["high", "low"]), "`low`: no shadows or smoothing, for big worlds or slow devices"),
    PropDef::new(
        "lod",
        PropKind::Any,
        "Models drawn simpler far from the camera: `auto` (default) for a model used by 100 objects or more, `true` for every model, `false` (or `none`) for none, or `{ ratio, distance }` (default 0.15 of the triangles, past twelve times the model's size and at least 15 m) for every model. \
         Made in the browser with meshoptimizer's simplifier; a model's own `far` wins. Only for objects drawn together (not those in a look of their own)",
    ),
    PropDef::new("motionScale", PropKind::Number, "While the camera moves, the view is drawn at this share of its resolution each way (default 0.65), and sharp again 400 ms after it stops; 1 keeps it sharp"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 480. The width is the container's"),
    PropDef::new("title", PropKind::Any, "Names the view for screen readers (default `3D view`)"),
];

/// Props the browser resolves against state; the rest are read once on the server.
const LIVE: &[&str] = &[
    "models", "objects", "paths", "zones", "walls", "ground", "environment", "camera", "views", "readouts", "colorBy", "world", "src", "live", "follow", "filter", "toneBy", "alarmBy",
    "effectBy", "labels", "editable", "snap", "palette", "lod",
];
const BINDINGS: &[&str] = &["bind", "bindObject", "bindZone", "bindView", "bindAlarms", "bindObjects", "bindCamera", "bindLive"];

impl Component for Twin {
    fn name(&self) -> &'static str {
        "ui-twin"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A digital twin: a 3D world drawn with three.js from models of most formats, every part of it configurable and able to use state, \
                  with machine and sensor values shown on the objects, alarms, paths things follow, zones, views and editing the layout. \
                  A `ui-twin-popup` child is the popup's content; `ui-twin-panel` children float above it.",
            props: PROPS,
            parts: &["controls"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let (mut popup, mut panels) = (None, Vec::new());
        for child in &node.children {
            match child.kind.as_str() {
                "ui-twin-popup" if popup.is_some() => return Err(RenderError::new(child, "a twin takes one `ui-twin-popup`")),
                "ui-twin-popup" => popup = Some(child),
                "ui-twin-panel" => panels.push(child),
                _ => return Err(RenderError::new(child, "a twin's children are `ui-twin-popup` and `ui-twin-panel`")),
            }
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
        // The addresses known now; the browser checks the rest.
        let mut urls: Vec<&str> = Vec::new();
        let models = props.get("models").into_iter().chain(props.get("world").and_then(|w| w.get("models")));
        for m in models.filter_map(Json::as_object).flat_map(|m| m.values()) {
            urls.extend(m.as_str().or_else(|| m["url"].as_str()));
            urls.extend(m["mtl"].as_str());
            urls.extend(m["far"].as_str().or_else(|| m["far"]["url"].as_str()));
        }
        urls.extend(props.get("src").and_then(Json::as_str));
        if let Some(l) = props.get("live") {
            urls.extend(l.as_str().into_iter().chain(l["url"].as_str()).chain(l["stream"].as_str()));
        }
        if let Some(bad) = urls.iter().find(|u| !cx.policy().url_allowed(u)) {
            return Err(RenderError::new(node, format!("`{bad}`: URL not allowed by the policy")));
        }
        if let Some(Json::Object(m)) = props.get("models") {
            for (name, m) in m {
                if !(m.is_string() || m["url"].is_string() || m.get("$irender").is_some()) {
                    return Err(RenderError::new(node, format!("`models.{name}` must be a url or `{{ url, … }}`")));
                }
            }
        }

        let policy = cx.policy();
        let kit = &self.kit;
        let mut config = json!({
            "source": { "three": kit.three, "addons": kit.addons, "draco": kit.draco, "basis": kit.basis, "simplifier": kit.simplifier },
            "props": props,
            "reactive": reactive,
            "popup": cx.prop(node, "popup") != Some(Json::Bool(false)),
            "gestures": cx.prop_string(node, "gestures").unwrap_or_else(|| "orbit".into()),
            "quality": cx.prop_string(node, "quality").unwrap_or_else(|| "high".into()),
            "motionScale": cx.prop(node, "motionScale"),
            "transition": cx.prop(node, "transition"),
            // Documents the policy doesn't trust can't link an object to a script.
            "safe": !(policy.allow_event_attributes && policy.allow_unsafe_urls),
        });
        for key in BINDINGS {
            config[key] = cx.prop_string(node, key).into();
        }
        cx.global_js("ui-twin", SCRIPT);
        cx.global_css("ui-twin", GLOBAL_CSS);

        let mut sizing = StyleBlock::new();
        let height = node.prop("height").cloned().unwrap_or(Val::Lit(json!(480)));
        if let Some(v) = cx.style_value(node, "height", &height)? {
            sizing.push("height", v);
        }
        let title = node.prop("title").cloned().unwrap_or_else(|| Val::Lit(cx.label("3D view").into()));
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).base_style(sizing).behavior("ui-twin", config).attr("role", "region").attr("aria-label", title.clone());
        cx.open(&root, node)?;

        // Unique on the page: where the twin is in the document, and which list item it's in.
        let path: String = node.path.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
        let hint_id = format!("twin-{path}-{}-keys", cx.scope_key()).replace("--", "-");
        let mut canvas = Element::new("div");
        canvas
            .base_style_ref(&CANVAS)
            .attr("data-twin-canvas", true)
            .attr("tabindex", "0")
            .attr("role", "img")
            .attr("aria-label", title)
            .attr("aria-describedby", hint_id.as_str());
        cx.open(&canvas, node)?;
        cx.close("div");
        let mut hint = Element::new("p");
        hint.base_style_ref(&HIDDEN).attr("id", hint_id.as_str()).text(Val::Lit(
            cx.label("Arrow keys turn the view, Shift and the arrows move it, + and − zoom, Home fits it. The objects are listed after it.").into(),
        ));
        cx.open(&hint, node)?;
        cx.close("p");
        let mut tags = Element::new("div");
        tags.base_style_ref(&TAGS).attr("data-twin-tags", true);
        cx.open(&tags, node)?;
        cx.close("div");
        let mut list = Element::new("ul");
        list.base_style_ref(&LIST).attr("data-twin-list", true).attr("aria-label", cx.label("Objects"));
        cx.open(&list, node)?;
        cx.close("ul");
        let mut note = Element::new("div");
        note.base_style_ref(&NOTE).attr("data-twin-note", true).attr("role", "status");
        cx.open(&note, node)?;
        cx.close("div");
        let mut live = Element::new("div");
        live.base_style_ref(&HIDDEN).attr("data-twin-live", true).attr("role", "alert");
        cx.open(&live, node)?;
        cx.close("div");
        let mut palette = Element::new("div");
        palette.base_style_ref(&PALETTE).attr("data-twin-palette", true).attr("role", "group").attr("aria-label", cx.label("Add to the world")).attr("hidden", true);
        cx.open(&palette, node)?;
        cx.close("div");
        let mut views = Element::new("div");
        views.base_style_ref(&VIEWS).attr("data-twin-views", true).attr("role", "group").attr("aria-label", cx.label("Views")).attr("hidden", true);
        cx.open(&views, node)?;
        cx.close("div");

        if let Some(popup) = popup {
            // The popup's content: a hidden list the script gives the open object.
            let none = Val::Lit(Json::Array(Vec::new()));
            let mut proto = cx.element("div", popup);
            proto
                .base_style_ref(&POPUP)
                .attr("data-twin-proto", true)
                .attr("hidden", true)
                .list(ClientList { items: &none, item: OBJECT.into(), index: INDEX.into(), key: None });
            cx.open(&proto, popup)?;
            cx.item_template_with(OBJECT, INDEX, |cx| cx.children(popup))?;
            cx.close("div");
        }
        for panel in panels {
            let mut el = cx.element("div", panel);
            let position = cx.prop_string(panel, "position").unwrap_or_else(|| "top-left".into());
            el.base_style_ref(&PANEL).attr("data-position", position).attr("data-twin-panel", true);
            cx.open(&el, panel)?;
            cx.children(panel)?;
            cx.close("div");
        }
        if cx.prop(node, "controls") != Some(Json::Bool(false)) {
            let mut controls = cx.part("div", node, "controls");
            controls.base_style_ref(&CONTROLS).attr("data-twin-controls", true);
            cx.open(&controls, node)?;
            for (action, label, icon) in CONTROL_BUTTONS {
                let label = cx.label(label);
                let mut b = Element::new("button");
                b.base_style_ref(&CONTROL).attr("type", "button").attr("aria-label", label.as_str()).attr("title", label.as_str()).attr("data-twin-action", *action);
                cx.open(&b, node)?;
                cx.markup(icon);
                cx.close("button");
            }
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

/// `ui-twin-popup` and `ui-twin-panel`, which their `ui-twin` draws.
struct Inside {
    name: &'static str,
}

const PANEL_PROPS: &[PropDef] = &[PropDef::new(
    "position",
    PropKind::Enum(&["top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"]),
    "Corner or edge of the view (default top-left)",
)];

impl Component for Inside {
    fn name(&self) -> &'static str {
        self.name
    }

    fn info(&self) -> ComponentInfo {
        let panel = self.name == "ui-twin-panel";
        ComponentInfo {
            doc: if panel {
                "Content that floats above a `ui-twin`, e.g. a legend, counts or filters. It doesn't move with the camera."
            } else {
                "What a `ui-twin`'s popup shows, with the selected object (and its latest values) in scope as `object`."
            },
            props: if panel { PANEL_PROPS } else { &[] },
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, format!("`{}` goes inside a `ui-twin`", self.name)))
    }
}

// Labels (looked up where they're used): "Zoom in", "Zoom out", "Fit view"
const CONTROL_BUTTONS: &[(&str, &str, &str)] = &[
    ("zoom-in", "Zoom in", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#),
    ("zoom-out", "Zoom out", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M3 8h10"/></svg>"#),
    ("fit", "Fit view", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4"/></svg>"#),
];

const TEXT: &str = "var(--color-text, #18181b)";
const MUTED: &str = "var(--color-muted, #71717a)";
const SURFACE: &str = "var(--color-surface, #fff)";
const SUBTLE: &str = "var(--color-subtle, #f4f4f5)";
const BORDER: &str = "var(--border-width, 1px) solid var(--color-border, #e4e4e7)";
const RING: &str = "2px solid var(--color-ring, var(--color-primary, #18181b))";

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// The tones an object can have, as the UI kit's.
const TONES: &[(&str, &str)] = &[
    ("neutral", "var(--color-muted, #71717a)"),
    ("info", "var(--color-info, #2563eb)"),
    ("success", "var(--color-success, #16a34a)"),
    ("warning", "var(--color-warning, #d97706)"),
    ("danger", "var(--color-danger, #dc2626)"),
    ("primary", "var(--color-primary, #18181b)"),
];

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mut b = pairs(&[
        ("position", "relative"),
        ("isolation", "isolate"),
        ("overflow", "hidden"),
        ("min-width", "0"),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", "radial-gradient(120% 90% at 50% 0%, var(--color-surface, #fff), var(--color-subtle, #f4f4f5))"),
        ("color", TEXT),
        ("font-size", "calc(13px * var(--font-scale, 1))"),
        ("--tone", TONES[5].1),
    ]);
    for (tone, color) in TONES {
        b = b.nest(format!("& [data-tone={tone}]"), pairs(&[("--tone", color)]));
    }
    b.nest(
        "& [data-twin-tag]",
        pairs(&[
            ("position", "absolute"),
            ("left", "0"),
            ("top", "0"),
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("gap", "6px"),
            ("max-width", "220px"),
            ("margin", "0"),
            ("padding", "3px 8px 3px 6px"),
            ("border", "1px solid color-mix(in srgb, var(--tone) 45%, var(--color-border, #e4e4e7))"),
            ("border-radius", "999px"),
            ("background", "color-mix(in srgb, var(--color-surface, #fff) 92%, transparent)"),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", "12px"),
            ("line-height", "1.3"),
            ("white-space", "nowrap"),
            ("translate", "-50% -100%"),
            ("box-shadow", "0 1px 3px rgb(0 0 0 / 0.15)"),
            ("cursor", "pointer"),
            ("pointer-events", "auto"),
            // No `backdrop-filter`: each tag would be a blurred layer of its own, and some
            // thousands of them stall the browser's compositor (the view stops drawing).
        ]),
    )
    .nest("& [data-twin-tag] > span", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis")]))
    .nest("& [data-twin-tag] [data-dot]", pairs(&[("flex", "none"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
    .nest("& [data-twin-tag] [data-read]", pairs(&[("font-variant-numeric", "tabular-nums"), ("font-weight", "600")]))
    .nest("& [data-twin-tag] [data-read] + [data-read]", pairs(&[("padding-inline-start", "6px"), ("border-inline-start", BORDER)]))
    .nest("& [data-level=warn]", pairs(&[("color", "var(--color-warning, #b45309)")]))
    .nest("& [data-level=alarm]", pairs(&[("color", "var(--color-danger, #dc2626)")]))
    .nest(
        "& [data-twin-tag] [data-bang]",
        pairs(&[("display", "grid"), ("place-items", "center"), ("width", "16px"), ("height", "16px"), ("margin-inline-start", "-2px"), ("border-radius", "50%"), ("background", "var(--color-danger, #dc2626)"), ("color", "#fff"), ("font-size", "11px")]),
    )
    .nest("& [data-twin-tag][data-alarm] [data-dot]", pairs(&[("display", "none")]))
    .nest(
        "& [data-twin-tag][data-more]::after",
        pairs(&[("content", "attr(data-more)"), ("padding", "0 5px"), ("border-radius", "999px"), ("background", "var(--color-danger, #dc2626)"), ("color", "#fff"), ("font-size", "11px"), ("font-weight", "600"), ("line-height", "16px")]),
    )
    .nest("& [data-twin-tag][data-selected]", pairs(&[("border-color", "var(--color-primary, #18181b)"), ("box-shadow", "0 0 0 1px var(--color-primary, #18181b), 0 1px 3px rgb(0 0 0 / 0.15)")]))
    .nest("& [data-twin-tag]:focus-visible", pairs(&[("outline", RING), ("outline-offset", "2px")]))
    .nest(
        "& [data-twin-zone]",
        pairs(&[
            ("position", "absolute"),
            ("left", "0"),
            ("top", "0"),
            ("padding", "1px 6px"),
            ("border-radius", "var(--radius-sm, 4px)"),
            ("background", "color-mix(in srgb, var(--tone) 14%, var(--color-surface, #fff))"),
            ("color", "color-mix(in srgb, var(--tone) 70%, var(--color-text, #18181b))"),
            ("font-size", "11px"),
            ("font-weight", "600"),
            ("letter-spacing", "0.02em"),
            ("text-transform", "uppercase"),
            ("white-space", "nowrap"),
            ("translate", "-50% -50%"),
            ("pointer-events", "none"),
        ]),
    )
    .nest("@media (prefers-reduced-motion: no-preference)", pairs(&[]).nest("& [data-twin-tag][data-effect]", pairs(&[("animation", "ui-twin-alarm 1s ease-in-out infinite alternate")])))
    .nest(
        "& [data-twin-popup]",
        pairs(&[
            ("position", "absolute"),
            ("left", "0"),
            ("top", "0"),
            ("box-sizing", "border-box"),
            ("width", "max-content"),
            ("min-width", "180px"),
            ("max-width", "min(300px, calc(100% - 24px))"),
            ("max-height", "min(340px, calc(100% - 80px))"),
            ("padding", "12px 32px 12px 12px"),
            ("overflow", "auto"),
            ("border", BORDER),
            ("border-radius", "var(--radius-md, 8px)"),
            ("background", SURFACE),
            ("color", TEXT),
            ("translate", "calc(-50% + var(--shift, 0px)) calc(-100% - 12px)"),
            ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.15))"),
            ("line-height", "1.45"),
            ("pointer-events", "auto"),
            ("cursor", "default"),
            ("z-index", "2"),
        ]),
    )
    .nest("& [data-twin-popup][data-below]", pairs(&[("translate", "calc(-50% + var(--shift, 0px)) 24px")]))
    .nest("& [data-twin-popup] > strong", pairs(&[("display", "block"), ("font-weight", "600")]))
    .nest("& [data-twin-popup] > p", pairs(&[("margin", "2px 0 0"), ("color", MUTED)]))
    .nest("& [data-twin-popup] > dl", pairs(&[("display", "grid"), ("grid-template-columns", "auto auto"), ("gap", "2px 16px"), ("margin", "8px 0 0")]))
    .nest("& [data-twin-popup] dt", pairs(&[("color", MUTED)]))
    .nest("& [data-twin-popup] dd", pairs(&[("margin", "0"), ("text-align", "end"), ("font-variant-numeric", "tabular-nums"), ("font-weight", "600")]))
    .nest("& [data-twin-spark]", pairs(&[("display", "inline-block"), ("margin-inline-end", "8px"), ("vertical-align", "middle"), ("opacity", "0.85")]))
    .nest("& [data-twin-edit]", pairs(&[("display", "flex"), ("gap", "6px"), ("margin-top", "10px")]))
    .nest(
        "& [data-twin-edit] button",
        pairs(&[("padding", "3px 10px"), ("border", BORDER), ("border-radius", "var(--radius-sm, 6px)"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-size", "12px"), ("cursor", "pointer")]),
    )
    .nest("& [data-twin-edit] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-twin-edit] button:focus-visible", pairs(&[("outline", RING)]))
    .nest("& [data-twin-popup] > a", pairs(&[("display", "inline-block"), ("margin-top", "6px"), ("color", "var(--color-primary, currentColor)"), ("text-decoration", "underline"), ("text-underline-offset", "3px")]))
    .nest(
        "& [data-twin-close]",
        pairs(&[
            ("position", "absolute"),
            ("top", "6px"),
            ("right", "6px"),
            ("display", "grid"),
            ("place-items", "center"),
            ("width", "24px"),
            ("height", "24px"),
            ("padding", "0"),
            ("border", "0"),
            ("border-radius", "var(--radius-sm, 4px)"),
            ("background", "none"),
            ("color", MUTED),
            ("font", "inherit"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-twin-close]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
    .nest("& [data-twin-close]:focus-visible", pairs(&[("outline", RING)]))
    .nest("& [data-twin-failed]", pairs(&[("position", "absolute"), ("inset", "0"), ("overflow", "auto"), ("padding", "16px"), ("color", MUTED)]))
    .nest("& [data-twin-failed] ul", pairs(&[("margin", "8px 0 0"), ("padding-inline-start", "18px"), ("color", TEXT)]))
    .nest("&[data-stale] [data-twin-tag] [data-read]", pairs(&[("opacity", "0.55")]))
});

static CANVAS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "absolute"), ("inset", "0"), ("z-index", "0"), ("touch-action", "none")])
        .nest("& canvas", pairs(&[("display", "block"), ("outline", "none")]))
        .nest("&:focus-visible", pairs(&[("outline", RING), ("outline-offset", "-2px")]))
        .nest("&[data-dragging] canvas", pairs(&[("cursor", "grabbing !important")]))
});

/// Out of sight, for screen readers.
static HIDDEN: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("margin", "0"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]));

static TAGS: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("position", "absolute"), ("inset", "0"), ("z-index", "1"), ("overflow", "hidden"), ("pointer-events", "none")]));

/// The objects, for the keyboard: out of sight until one has the focus.
static LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("margin", "0"), ("padding", "0"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("list-style", "none")])
        .nest(
            "&:focus-within",
            pairs(&[
                ("z-index", "3"),
                ("left", "12px"),
                ("bottom", "12px"),
                ("width", "auto"),
                ("height", "auto"),
                ("max-width", "min(320px, calc(100% - 80px))"),
                ("max-height", "40%"),
                ("overflow", "auto"),
                ("clip-path", "none"),
                ("padding", "4px"),
                ("border", BORDER),
                ("border-radius", "var(--radius-md, 8px)"),
                ("background", SURFACE),
                ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.15))"),
            ]),
        )
        .nest(
            "& button",
            pairs(&[
                ("display", "block"),
                ("width", "100%"),
                ("padding", "4px 8px"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm, 4px)"),
                ("background", "none"),
                ("color", TEXT),
                ("font", "inherit"),
                ("text-align", "start"),
                ("white-space", "nowrap"),
                ("overflow", "hidden"),
                ("text-overflow", "ellipsis"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& button[aria-pressed=true]", pairs(&[("background", SUBTLE), ("font-weight", "600")]))
        .nest("& button:focus-visible", pairs(&[("outline", RING), ("outline-offset", "-2px")]))
});

/// What the view has to say: that it's loading, what went wrong.
static NOTE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("left", "50%"),
        ("top", "50%"),
        ("z-index", "3"),
        ("max-width", "calc(100% - 32px)"),
        ("box-sizing", "border-box"),
        ("padding", "8px 14px"),
        ("border-radius", "999px"),
        ("background", "rgb(0 0 0 / 0.72)"),
        ("color", "#fff"),
        ("transform", "translate(-50%, -50%)"),
        ("pointer-events", "none"),
        ("text-align", "center"),
        ("opacity", "0"),
        ("transition", "opacity .2s"),
    ])
    .nest("&[data-shown]", pairs(&[("opacity", "1")]))
});

static VIEWS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "2"),
        ("top", "12px"),
        ("right", "12px"),
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("justify-content", "flex-end"),
        ("gap", "2px"),
        ("max-width", "calc(100% - 24px)"),
        ("padding", "2px"),
        ("border", BORDER),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", SURFACE),
        ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.06))"),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest(
        "& button",
        pairs(&[("padding", "4px 10px"), ("border", "0"), ("border-radius", "var(--radius-sm, 6px)"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("font-size", "12px"), ("font-weight", "500"), ("cursor", "pointer")]),
    )
    .nest("& button:hover", pairs(&[("color", TEXT)]))
    .nest("& button[aria-pressed=true]", pairs(&[("background", SUBTLE), ("color", TEXT)]))
    .nest("& button:focus-visible", pairs(&[("outline", RING), ("outline-offset", "-2px")]))
});

static PALETTE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "2"),
        ("left", "12px"),
        ("bottom", "12px"),
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("gap", "4px"),
        ("max-width", "calc(100% - 80px)"),
        ("padding", "4px"),
        ("border", BORDER),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", SURFACE),
        ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.06))"),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest(
        "& button",
        pairs(&[
            ("padding", "5px 10px"),
            ("border", BORDER),
            ("border-radius", "var(--radius-sm, 6px)"),
            ("background", SURFACE),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", "12px"),
            ("cursor", "grab"),
            ("touch-action", "none"),
        ]),
    )
    .nest("& button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& button:focus-visible", pairs(&[("outline", RING), ("outline-offset", "1px")]))
});

static POPUP: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "8px")]).nest("&[hidden]", pairs(&[("display", "none")])));

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    let at = |p: &[(&str, &str)]| pairs(p);
    pairs(&[("position", "absolute"), ("z-index", "2"), ("max-width", "calc(100% - 24px)")])
        .nest("&[data-position=top-left]", at(&[("top", "12px"), ("left", "12px")]))
        .nest("&[data-position=top-center]", at(&[("top", "12px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=top-right]", at(&[("top", "56px"), ("right", "12px")]))
        .nest("&[data-position=bottom-left]", at(&[("bottom", "12px"), ("left", "12px")]))
        .nest("&[data-position=bottom-center]", at(&[("bottom", "12px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=bottom-right]", at(&[("bottom", "12px"), ("right", "56px")]))
});

static CONTROLS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "2"),
        ("right", "12px"),
        ("bottom", "12px"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("border", BORDER),
        ("border-radius", "var(--radius-md, 8px)"),
        ("overflow", "hidden"),
        ("background", SURFACE),
        ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.06))"),
    ])
});

static CONTROL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "grid"),
        ("place-items", "center"),
        ("width", "32px"),
        ("height", "32px"),
        ("padding", "0"),
        ("border", "none"),
        ("border-bottom", BORDER),
        ("background", "transparent"),
        ("color", TEXT),
        ("cursor", "pointer"),
    ])
    .nest("&:last-child", pairs(&[("border-bottom", "none")]))
    .nest("&:hover", pairs(&[("background", SUBTLE)]))
    .nest("&:focus-visible", pairs(&[("outline", RING), ("outline-offset", "-2px")]))
});

const GLOBAL_CSS: &str = "@keyframes ui-twin-alarm{from{box-shadow:0 0 0 0 var(--color-danger,#dc2626)}to{box-shadow:0 0 0 5px color-mix(in srgb,var(--color-danger,#dc2626) 25%,transparent)}}";

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
        Json::Object(o) => Json::Object(o.iter().map(|(k, v)| Ok((k.clone(), nested(cx, node, name, v, live)?))).collect::<Result<_, RenderError>>()?),
        other => other.clone(),
    })
}
