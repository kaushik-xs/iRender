//! Maps for iRender, on [OpenStreetMap](https://www.openstreetmap.org/) or Google Maps.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_map_kit::register(&mut engine.registry);
//! let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-map",
//!     "props": { "markers": [{ "id": "hq", "lat": 12.97, "lng": 77.59, "title": "Head office" }] } } }"#, Format::Json)?;
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `provider: osm` (the default) needs no key: [Leaflet](https://leafletjs.com/) draws
//! OpenStreetMap's tiles, or any other tile server's. `provider: google` draws Google Maps
//! through its JavaScript API, with a key from [`MapKit::google_key`] or the `apiKey` prop.
//! Everything on the map (markers, popups, clusters, shapes, the buttons) is the kit's own,
//! in the theme's colours, so a document looks and works the same on both.
//!
//! Neither library is inlined: a page loads the one it uses, once, when a map comes near
//! the view.

use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val};
use irender_html::{ClientList, Component, Element, Registry, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("map.js");

/// The Leaflet release the kit is written and tested against.
pub const LEAFLET_VERSION: &str = "1.9.4";

/// Where pages load Leaflet from by default: its `leaflet.js` and `leaflet.css`.
pub const LEAFLET_CDN: &str = "https://cdn.jsdelivr.net/npm/leaflet@1.9.4/dist/";

/// OpenStreetMap's own tiles. They're for light use (see its [tile usage
/// policy](https://operations.osmfoundation.org/policies/tiles/)); a busy site needs its
/// own tile server or a provider, set with [`MapKit::tiles`].
pub const OSM_TILES: &str = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";

/// Where maps come from.
#[derive(Debug, Clone)]
pub struct MapKit {
    /// The folder with `leaflet.js` and `leaflet.css`, ending in `/`. Point it at your own
    /// copy to avoid the CDN, e.g. `/static/leaflet/`.
    pub leaflet: String,
    /// The tiles of maps that don't set `tiles`.
    pub tiles: Tiles,
    /// The Google Maps API key, for maps that don't set `apiKey`. Browsers see it, so
    /// restrict it to your sites in Google's console.
    pub google_key: Option<String>,
}

/// A tile server.
#[derive(Debug, Clone)]
pub struct Tiles {
    /// With `{z}`, `{x}` and `{y}`, and optionally `{s}` (a, b or c) and `{r}` (`@2x`).
    pub url: String,
    /// Who the map data is from, shown in the corner.
    pub attribution: String,
    /// What the attribution links to.
    pub attribution_url: Option<String>,
    pub max_zoom: u8,
}

impl Default for Tiles {
    fn default() -> Self {
        Self {
            url: OSM_TILES.into(),
            attribution: "© OpenStreetMap contributors".into(),
            attribution_url: Some("https://www.openstreetmap.org/copyright".into()),
            max_zoom: 19,
        }
    }
}

impl Default for MapKit {
    fn default() -> Self {
        Self { leaflet: LEAFLET_CDN.into(), tiles: Tiles::default(), google_key: None }
    }
}

/// Adds `ui-map`, `ui-map-popup` and `ui-map-panel` to `registry`, with Leaflet from the
/// CDN and OpenStreetMap's tiles.
pub fn register(registry: &mut Registry) {
    register_with(registry, MapKit::default());
}

/// Adds `ui-map`, `ui-map-popup` and `ui-map-panel` to `registry` with your own settings.
pub fn register_with(registry: &mut Registry, kit: MapKit) {
    registry.register(MapView { kit }).register(Inside { name: "ui-map-popup" }).register(Inside { name: "ui-map-panel" });
}

/// The names the open marker and its index are in scope under, in a `ui-map-popup`.
const MARKER: &str = "marker";
const INDEX: &str = "index";

struct MapView {
    kit: MapKit,
}

const PROPS: &[PropDef] = &[
    PropDef::new("provider", PropKind::Enum(&["osm", "google"]), "`osm` (default): OpenStreetMap, no key needed. `google`: Google Maps, which needs an API key"),
    PropDef::new("markers", PropKind::Any, "Places, as rows: `{ id, lat, lng, title, description, label, tone, color, alarm, effect, href, draggable }`, usually from data or state. `label` is a letter, number or emoji in the pin"),
    PropDef::new("latField", PropKind::String, "The rows' latitude field (default `lat`, else `latitude`)"),
    PropDef::new("lngField", PropKind::String, "The rows' longitude field (default `lng`, else `lon`, `long` or `longitude`)"),
    PropDef::new("titleField", PropKind::String, "The rows' title field (default `title`, else `name`)"),
    PropDef::new("toneBy", PropKind::Any, "Colours markers by a field: `{ field: status, tones: { late: danger, done: success } }`"),
    PropDef::new("alarmBy", PropKind::Any, "Which markers are in alarm: `{ field: status, values: [offline, fault] }`, or `alarm: true` on a row. They pulse in red with a `!`, stay on top, and a new one is announced"),
    PropDef::new("effectBy", PropKind::Any, "Animates markers by a field: `{ field: status, effects: { late: glow } }`, or `effect` on a row: `pulse`, `glow`, `bounce` or `blink`. Still, where the viewer asked for less motion"),
    PropDef::new("filter", PropKind::Any, "Shows only the markers that pass: `{ field: condition }`, as `ui-data-table`'s and `ui-chart`'s `filter`, e.g. a `ui-filters` panel's `bind`"),
    PropDef::new("shapes", PropKind::Any, "Lines, areas and circles: `{ id, type: line | area | circle, points: [[lat, lng], …], center, radius (metres), title, tone, color, width, dashed, fill }`"),
    PropDef::new("geojson", PropKind::Any, "GeoJSON lines and areas, styled by each feature's `properties` (`tone`, `color`, `width`, `title`); its points become markers"),
    PropDef::new(
        "overlays",
        PropKind::Any,
        "Layers over the map: `{ id, type: image, url, bounds: [[south, west], [north, east]] }`, e.g. a floor plan; `{ type: tiles, url, attribution }`, e.g. sea marks; `{ type: heat, points, weightField, radius, max }`. Each with `opacity` and `show`, which may use state",
    ),
    PropDef::new("center", PropKind::Any, "`[lat, lng]` or `{ lat, lng }`. The map moves when it changes. Default: fitted to what's on it"),
    PropDef::new("zoom", PropKind::Any, "0 (the world) to 19 (a building); default 13 with a `center`"),
    PropDef::new("minZoom", PropKind::Number, "How far out it zooms"),
    PropDef::new("maxZoom", PropKind::Number, "How far in it zooms, and the closest it fits to"),
    PropDef::new("fit", PropKind::Any, "Fits the view to the markers and shapes when it opens (default: without a `center`); `always`: whenever they change"),
    PropDef::new("spread", PropKind::Any, "Markers on the same spot fan out round it, each on a line to where it is, so all can be seen and clicked (default true)"),
    PropDef::new(
        "draw",
        PropKind::Any,
        "With `bindShapes`: buttons to draw shapes by hand, `true` for all or a list of `line`, `area`, `circle`. Corners are clicked on the map, Enter or a double click finishes (an area also by its first corner); a circle is its middle then its edge. A drawn shape clicked is picked, and Delete takes it away",
    ),
    PropDef::new("bindShapes", PropKind::StateKey, "With `draw`: the state key the drawn shapes are in (give `shapes` the same, `{{ key }}`); each is added as `{ id, type, points }` or `{ id, type: circle, center, radius }`, and `draw` and `erase` fire"),
    PropDef::new("cluster", PropKind::Any, "Groups markers closer than this many px (`true`: 56), with how many there are; clicking a group zooms to it"),
    PropDef::new("bind", PropKind::StateKey, "State key with the selected marker's id (null when none). Setting it selects that marker and brings it into view"),
    PropDef::new("bindMarker", PropKind::StateKey, "State key with the selected marker itself, or null"),
    PropDef::new("bindShape", PropKind::StateKey, "State key with the id of the shape last clicked; shapes are clickable with it"),
    PropDef::new("bindView", PropKind::StateKey, "State key kept at `{ center, zoom, bounds: { north, south, east, west } }`"),
    PropDef::new("bindVisible", PropKind::StateKey, "State key with the ids of the markers in view, e.g. for a list beside the map"),
    PropDef::new("bindAlarms", PropKind::StateKey, "State key with the ids of the markers in alarm, e.g. for a count"),
    PropDef::new("bindPoint", PropKind::StateKey, "Picks a place: clicking the map puts a pin there, which can be dragged, and writes `{ lat, lng }`"),
    PropDef::new("bindMarkers", PropKind::StateKey, "State key the rows are in: a dragged marker's new place is written back to its row"),
    PropDef::new("bindLocation", PropKind::StateKey, "State key that gets `{ lat, lng, accuracy }` after My location"),
    PropDef::new("draggable", PropKind::Any, "Markers can be dragged, or moved with Alt and the arrow keys (needs `bindMarkers`)"),
    PropDef::new("popup", PropKind::Bool, "Opens a popup over the selected marker, with its title and description or the `ui-map-popup` (default true)"),
    PropDef::new("locate", PropKind::Bool, "A My location button: shows where the viewer is, once they allow it"),
    PropDef::new("controls", PropKind::Bool, "Zoom and fit buttons (default true)"),
    PropDef::new("gestures", PropKind::Enum(&["cooperative", "greedy", "none"]), "`cooperative` (default): the page scrolls past the map, and Ctrl / ⌘ + wheel or two fingers zoom it. `greedy`: the map takes the wheel and one finger. `none`: it stays put"),
    PropDef::new("tiles", PropKind::Any, "With `osm`, another tile server: `{ url, attribution, attributionUrl, maxZoom, darkUrl }`, `url` having `{z}`, `{x}` and `{y}`"),
    PropDef::new("darkTiles", PropKind::Bool, "With `osm`, darken the tiles in dark mode when there's no `darkUrl` (default true)"),
    PropDef::new("apiKey", PropKind::Any, "With `google`, the API key (default: the one the host set). Browsers see it, so restrict it to your sites"),
    PropDef::new("mapId", PropKind::Any, "With `google`, a map id from Google's console, for its cloud styles"),
    PropDef::new("mapType", PropKind::Enum(&["roadmap", "satellite", "hybrid", "terrain"]), "With `google` (default `roadmap`)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 400. The width is the container's"),
    PropDef::new("title", PropKind::Any, "Names the map for screen readers (default `Map`)"),
];

/// Props the browser resolves against state; the rest are read once on the server.
const LIVE: &[&str] = &["markers", "shapes", "geojson", "center", "zoom", "fit", "cluster", "spread", "toneBy", "draggable", "filter", "alarmBy", "effectBy", "overlays"];
const FIXED: &[&str] = &["latField", "lngField", "titleField", "minZoom", "maxZoom", "locate", "mapType", "draw"];
const BINDINGS: &[&str] =
    &["bind", "bindMarker", "bindShape", "bindView", "bindVisible", "bindAlarms", "bindPoint", "bindMarkers", "bindLocation", "bindShapes"];

impl Component for MapView {
    fn name(&self) -> &'static str {
        "ui-map"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A map, on OpenStreetMap or Google Maps, with markers, popups, shapes, clusters and picking a place. \
                  A `ui-map-popup` child is the popup's content; `ui-map-panel` children float above it.",
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
                "ui-map-popup" if popup.is_some() => return Err(RenderError::new(child, "a map takes one `ui-map-popup`")),
                "ui-map-popup" => popup = Some(child),
                "ui-map-panel" => panels.push(child),
                _ => return Err(RenderError::new(child, "a map's children are `ui-map-popup` and `ui-map-panel`")),
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
        // The ones whose address is known now; the browser checks the rest.
        for overlay in props.get("overlays").and_then(Json::as_array).into_iter().flatten() {
            for u in [overlay["url"].as_str(), overlay["attributionUrl"].as_str()].into_iter().flatten() {
                if !cx.policy().url_allowed(u) {
                    return Err(RenderError::new(node, "`overlays`: URL not allowed by the policy"));
                }
            }
        }
        for name in FIXED {
            if let Some(v) = cx.prop(node, name).filter(|v| !v.is_null()) {
                props.insert(name.to_string(), v);
            }
        }
        let provider = cx.prop_string(node, "provider").unwrap_or_else(|| "osm".into());
        let source = if provider == "google" { self.google(node, cx)? } else { self.osm(node, cx)? };
        let policy = cx.policy();
        let mut config = json!({
            "provider": provider,
            "source": source,
            "props": props,
            "reactive": reactive,
            "popup": cx.prop(node, "popup") != Some(Json::Bool(false)),
            "gestures": cx.prop_string(node, "gestures").unwrap_or_else(|| "cooperative".into()),
            // Documents the policy doesn't trust can't link a marker to a script.
            "safe": !(policy.allow_event_attributes && policy.allow_unsafe_urls),
        });
        for key in BINDINGS {
            config[key] = cx.prop_string(node, key).into();
        }
        cx.global_js("ui-map", SCRIPT);
        cx.global_css("ui-map", GLOBAL_CSS);

        let mut sizing = StyleBlock::new();
        let height = node.prop("height").cloned().unwrap_or(Val::Lit(json!(400)));
        if let Some(v) = cx.style_value(node, "height", &height)? {
            sizing.push("height", v);
        }
        let title = node.prop("title").cloned().unwrap_or_else(|| Val::Lit(cx.label("Map").into()));
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).base_style(sizing).behavior("ui-map", config).attr("role", "region").attr("aria-label", title);
        if cx.prop(node, "darkTiles") != Some(Json::Bool(false)) {
            root.attr("data-map-dark", true);
        }
        cx.open(&root, node)?;

        let mut canvas = Element::new("div");
        canvas.base_style_ref(&CANVAS).attr("data-map-canvas", true);
        cx.open(&canvas, node)?;
        cx.close("div");
        let mut note = Element::new("div");
        note.base_style_ref(&NOTE).attr("data-map-note", true).attr("role", "status");
        cx.open(&note, node)?;
        cx.close("div");

        let mut live = Element::new("div");
        live.base_style_ref(&LIVE_REGION).attr("data-map-live", true).attr("role", "alert");
        cx.open(&live, node)?;
        cx.close("div");

        if let Some(popup) = popup {
            // The popup's content: a hidden list the script gives the open marker.
            let none = Val::Lit(Json::Array(Vec::new()));
            let mut proto = cx.element("div", popup);
            proto
                .base_style_ref(&POPUP)
                .attr("data-map-proto", true)
                .attr("hidden", true)
                .list(ClientList { items: &none, item: MARKER.into(), index: INDEX.into(), key: None });
            cx.open(&proto, popup)?;
            cx.item_template_with(MARKER, INDEX, |cx| cx.children(popup))?;
            cx.close("div");
        }
        for panel in panels {
            let mut el = cx.element("div", panel);
            let position = cx.prop_string(panel, "position").unwrap_or_else(|| "top-left".into());
            el.base_style_ref(&PANEL).attr("data-position", position);
            cx.open(&el, panel)?;
            cx.children(panel)?;
            cx.close("div");
        }
        if cx.prop(node, "controls") != Some(Json::Bool(false)) {
            let locate = cx.prop(node, "locate") == Some(Json::Bool(true));
            let mut controls = cx.part("div", node, "controls");
            controls.base_style_ref(&CONTROLS).attr("data-map-controls", true);
            cx.open(&controls, node)?;
            for (action, label, icon) in CONTROL_BUTTONS.iter().filter(|(a, ..)| locate || *a != "locate") {
                let label = cx.label(label);
                let mut b = Element::new("button");
                b.base_style_ref(&CONTROL).attr("type", "button").attr("aria-label", label.as_str()).attr("title", label.as_str()).attr("data-map-action", *action);
                cx.open(&b, node)?;
                cx.markup(icon);
                cx.close("button");
            }
            // Drawing shapes by hand, with somewhere to keep them.
            if cx.prop_string(node, "bindShapes").is_some() {
                let kinds: Vec<String> = match cx.prop(node, "draw") {
                    Some(Json::Bool(true)) => vec!["line".into(), "area".into(), "circle".into()],
                    Some(Json::Array(l)) => l.iter().map(irender_core::expr::stringify).collect(),
                    None | Some(Json::Null) | Some(Json::Bool(false)) => Vec::new(),
                    Some(_) => return Err(RenderError::new(node, "`draw` is `true` or a list of `line`, `area`, `circle`")),
                };
                if let Some(bad) = kinds.iter().find(|k| !["line", "area", "circle"].contains(&k.as_str())) {
                    return Err(RenderError::new(node, format!("`draw` can't draw `{bad}`; it takes `line`, `area`, `circle`")));
                }
                for (action, label, icon) in DRAW_BUTTONS.iter().filter(|(a, ..)| kinds.iter().any(|k| a.ends_with(k.as_str()))) {
                    let label = cx.label(label);
                    let mut b = Element::new("button");
                    b.base_style_ref(&CONTROL)
                        .attr("type", "button")
                        .attr("aria-label", label.as_str())
                        .attr("title", label.as_str())
                        .attr("data-map-action", *action)
                        .attr("aria-pressed", "false");
                    cx.open(&b, node)?;
                    cx.markup(icon);
                    cx.close("button");
                }
            }
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

impl MapView {
    fn osm(&self, node: &Node, cx: &RenderCx<'_>) -> Result<Json, RenderError> {
        let tiles = match cx.prop(node, "tiles").filter(|t| !t.is_null()) {
            None => {
                let t = &self.kit.tiles;
                json!({ "url": t.url, "attribution": t.attribution, "attributionUrl": t.attribution_url, "maxZoom": t.max_zoom })
            }
            Some(t) => {
                let url = t["url"].as_str().ok_or_else(|| RenderError::new(node, "`tiles` needs a `url`"))?;
                for u in [Some(url), t["darkUrl"].as_str(), t["attributionUrl"].as_str()].into_iter().flatten() {
                    if !cx.policy().url_allowed(u) {
                        return Err(RenderError::new(node, "`tiles`: URL not allowed by the policy"));
                    }
                }
                t
            }
        };
        Ok(json!({ "leaflet": self.kit.leaflet, "tiles": tiles }))
    }

    fn google(&self, node: &Node, cx: &RenderCx<'_>) -> Result<Json, RenderError> {
        let key = cx
            .prop_string(node, "apiKey")
            .filter(|k| !k.is_empty())
            .or_else(|| self.kit.google_key.clone())
            .ok_or_else(|| RenderError::new(node, "`provider: google` needs an API key: the `apiKey` prop, or the kit's `google_key`"))?;
        Ok(json!({ "key": key, "mapId": cx.prop_string(node, "mapId"), "language": cx.labels().lang }))
    }
}

/// `ui-map-popup` and `ui-map-panel`, which their `ui-map` draws.
struct Inside {
    name: &'static str,
}

const PANEL_PROPS: &[PropDef] = &[PropDef::new(
    "position",
    PropKind::Enum(&["top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"]),
    "Corner or edge of the map (default top-left)",
)];

impl Component for Inside {
    fn name(&self) -> &'static str {
        self.name
    }

    fn info(&self) -> ComponentInfo {
        let panel = self.name == "ui-map-panel";
        ComponentInfo {
            doc: if panel {
                "Content that floats above a `ui-map`, e.g. a legend or a search box. It doesn't move with the map."
            } else {
                "What a `ui-map`'s popup shows, with the selected marker in scope as `marker`."
            },
            props: if panel { PANEL_PROPS } else { &[] },
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, format!("`{}` goes inside a `ui-map`", self.name)))
    }
}

// Labels (looked up where they're used): "Zoom in", "Zoom out", "Fit view", "My location"
const CONTROL_BUTTONS: &[(&str, &str, &str)] = &[
    ("zoom-in", "Zoom in", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#),
    ("zoom-out", "Zoom out", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M3 8h10"/></svg>"#),
    ("fit", "Fit view", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4"/></svg>"#),
    ("locate", "My location", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><circle cx="8" cy="8" r="4"/><path d="M8 1.5V4M8 12v2.5M1.5 8H4M12 8h2.5"/></svg>"#),
];

// Labels (looked up where they're used): "Draw a line", "Draw an area", "Draw a circle"
const DRAW_BUTTONS: &[(&str, &str, &str)] = &[
    ("draw-line", "Draw a line", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.5 12.5 6 6l4 4 3.5-6.5"/></svg>"#),
    ("draw-area", "Draw an area", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" aria-hidden="true"><path d="M3 4.5 9 2.5l4.5 5-3 6h-7z"/></svg>"#),
    ("draw-circle", "Draw a circle", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><circle cx="8" cy="8" r="5.5"/><circle cx="8" cy="8" r="1" fill="currentColor"/></svg>"#),
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

/// The tones a marker or shape can have, as the UI kit's.
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
        // The libraries' layers stay inside, under the page's own menus and dialogs.
        ("isolation", "isolate"),
        ("overflow", "hidden"),
        ("min-width", "0"),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", SUBTLE),
        ("color", TEXT),
        ("font-size", "calc(13px * var(--font-scale, 1))"),
        // A map's geometry runs left to right on any page; its text still reads its own way.
        ("direction", "ltr"),
        ("--tone", TONES[5].1),
    ]);
    for (tone, color) in TONES {
        b = b.nest(format!("& [data-tone={tone}]"), pairs(&[("--tone", color)]));
    }
    // Leaflet's stylesheet is outside the cascade layers, so these have to insist.
    b.nest("& .leaflet-container", pairs(&[("background", "transparent !important"), ("font", "inherit !important")]))
        .nest("& .leaflet-control-attribution a", pairs(&[("color", "inherit !important")]))
        .nest(
            "& .leaflet-control-attribution",
            pairs(&[("padding", "1px 6px !important"), ("background", "color-mix(in srgb, var(--color-surface, #fff) 85%, transparent) !important"), ("color", "var(--color-muted, #71717a) !important"), ("font-size", "11px")]),
        )
        // A place's element is nothing but its position; what it holds hangs from that.
        .nest("& [data-map-at]", pairs(&[("width", "0"), ("height", "0"), ("border", "0"), ("background", "none")]))
        .nest(
            "& [data-map-pin]",
            pairs(&[
                ("position", "absolute"),
                ("left", "0"),
                ("top", "0"),
                ("display", "block"),
                ("width", "30px"),
                ("height", "38px"),
                ("margin", "0"),
                ("padding", "0"),
                ("border", "0"),
                ("background", "none"),
                ("transform", "translate(-50%, -100%)"),
                ("transform-origin", "50% 100%"),
                ("cursor", "pointer"),
                ("font", "inherit"),
                ("touch-action", "manipulation"),
            ]),
        )
        .nest("& [data-map-pin] svg", pairs(&[("display", "block"), ("width", "100%"), ("height", "100%"), ("overflow", "visible"), ("filter", "drop-shadow(0 1px 2px rgb(0 0 0 / 0.35))")]))
        .nest("& [data-map-pin] path", pairs(&[("fill", "var(--tone)"), ("stroke", SURFACE), ("stroke-width", "1.5")]))
        .nest("& [data-map-pin] circle", pairs(&[("fill", SURFACE)]))
        .nest(
            "& [data-map-pin] > span",
            pairs(&[
                ("position", "absolute"),
                ("left", "5px"),
                ("top", "5px"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "20px"),
                ("height", "20px"),
                ("border-radius", "50%"),
                ("overflow", "hidden"),
                ("background", SURFACE),
                ("color", TEXT),
                ("font-size", "11px"),
                ("font-weight", "600"),
                ("line-height", "1"),
                ("white-space", "nowrap"),
            ]),
        )
        .nest("& [data-map-pin][data-selected]", pairs(&[("transform", "translate(-50%, -100%) scale(1.2)")]))
        .nest("& [data-map-pin][data-draggable]", pairs(&[("cursor", "grab"), ("touch-action", "none")]))
        // Markers fanned out from one spot hang on a thin line to it.
        .nest("& [data-map-leg]", pairs(&[("position", "absolute"), ("left", "0"), ("top", "0"), ("width", "1px"), ("height", "1px"), ("overflow", "visible"), ("pointer-events", "none")]))
        .nest("& [data-map-leg] line", pairs(&[("stroke", TEXT), ("stroke-width", "1.5"), ("opacity", "0.6")]))
        // A corner of a shape being drawn.
        .nest(
            "& [data-map-vertex]",
            pairs(&[("position", "absolute"), ("left", "-5px"), ("top", "-5px"), ("width", "10px"), ("height", "10px"), ("border-radius", "50%"), ("background", SURFACE), ("border", "2px solid var(--color-primary, #18181b)"), ("pointer-events", "none")]),
        )
        .nest("&[data-sketching] [data-map-canvas]", pairs(&[("cursor", "crosshair")]))
        .nest("& [data-map-pin][data-dragging]", pairs(&[("cursor", "grabbing")]))
        .nest("& :is([data-map-pin], [data-map-cluster]):focus-visible", pairs(&[("outline", RING), ("outline-offset", "2px"), ("border-radius", "999px")]))
        .nest(
            "& [data-map-cluster]",
            pairs(&[
                ("position", "absolute"),
                ("display", "grid"),
                ("place-items", "center"),
                ("box-sizing", "border-box"),
                ("width", "var(--size, 36px)"),
                ("height", "var(--size, 36px)"),
                ("padding", "0"),
                ("border", "3px solid color-mix(in srgb, var(--color-surface, #fff) 70%, transparent)"),
                ("border-radius", "50%"),
                ("background", "var(--color-primary, #18181b)"),
                ("background-clip", "padding-box"),
                ("color", "var(--color-on-primary, #fff)"),
                ("transform", "translate(-50%, -50%)"),
                ("font", "inherit"),
                ("font-size", "12px"),
                ("font-weight", "600"),
                ("box-shadow", "0 1px 4px rgb(0 0 0 / 0.3)"),
                ("cursor", "pointer"),
            ]),
        )
        .nest(
            "& [data-map-me]",
            pairs(&[
                ("position", "absolute"),
                ("width", "14px"),
                ("height", "14px"),
                ("border", "3px solid #fff"),
                ("border-radius", "50%"),
                ("background", "var(--color-info, #2563eb)"),
                ("transform", "translate(-50%, -50%)"),
                ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.2), 0 1px 4px rgb(0 0 0 / 0.3)"),
            ]),
        )
        // Markers that move: a ring going out from the place, a halo, a hop, or slow blinking
        // (once a second, well under what can harm).
        .nest("& [data-map-cluster][data-alarm]", pairs(&[("background", "var(--tone)"), ("background-clip", "padding-box"), ("color", "#fff")]))
        .nest(
            "& [data-effect=pulse]::before, & [data-effect=pulse]::after",
            pairs(&[
                ("content", "\"\""),
                ("position", "absolute"),
                ("left", "50%"),
                ("top", "100%"),
                ("z-index", "-1"),
                ("width", "56px"),
                ("height", "56px"),
                ("margin", "-28px 0 0 -28px"),
                ("border-radius", "50%"),
                ("background", "var(--tone)"),
                ("opacity", "0.25"),
                ("pointer-events", "none"),
            ]),
        )
        .nest("& [data-map-cluster][data-effect=pulse]::before, & [data-map-cluster][data-effect=pulse]::after", pairs(&[("top", "50%"), ("width", "72px"), ("height", "72px"), ("margin", "-36px 0 0 -36px")]))
        .nest("& [data-effect=pulse]::after", pairs(&[("width", "28px"), ("height", "28px"), ("margin", "-14px 0 0 -14px")]))
        .nest("& [data-map-pin][data-effect=glow] svg", pairs(&[("filter", "drop-shadow(0 0 3px var(--tone)) drop-shadow(0 0 8px var(--tone))")]))
        .nest("& [data-map-cluster][data-effect=glow]", pairs(&[("box-shadow", "0 0 6px 3px var(--tone)")]))
        .nest(
            "@media (prefers-reduced-motion: no-preference)",
            pairs(&[])
                .nest("& [data-effect=pulse]::before", pairs(&[("animation", "ui-map-pulse 1.8s ease-out infinite")]))
                .nest("& [data-effect=pulse]::after", pairs(&[("animation", "ui-map-pulse 1.8s ease-out .6s infinite backwards")]))
                .nest("& [data-map-pin][data-effect=glow] svg", pairs(&[("animation", "ui-map-glow 1.4s ease-in-out infinite alternate")]))
                .nest("& [data-map-cluster][data-effect=glow]", pairs(&[("animation", "ui-map-halo 1.4s ease-in-out infinite alternate")]))
                .nest("& [data-map-pin][data-effect=bounce] > *", pairs(&[("animation", "ui-map-bounce 1s cubic-bezier(.3,.6,.4,1) infinite alternate")]))
                .nest("& [data-effect=blink]", pairs(&[("animation", "ui-map-blink 1s ease-in-out infinite alternate")])),
        )
        .nest("& [data-map-heat], & [data-map-heat] canvas", pairs(&[("pointer-events", "none")]))
        .nest("&[data-zooming] [data-map-heat] canvas", pairs(&[("visibility", "hidden")]))
        .nest(
            "& [data-map-credit]",
            pairs(&[("position", "absolute"), ("right", "0"), ("bottom", "16px"), ("z-index", "1"), ("padding", "1px 6px"), ("background", "color-mix(in srgb, var(--color-surface, #fff) 85%, transparent)"), ("color", MUTED), ("font-size", "11px")]),
        )
        // The popup, above its pin (or under it, near the top).
        .nest(
            "& [data-map-popup]",
            pairs(&[
                ("position", "absolute"),
                ("left", "0"),
                ("bottom", "50px"),
                ("box-sizing", "border-box"),
                ("width", "max-content"),
                ("max-width", "min(280px, var(--map-width, 280px))"),
                ("max-height", "min(320px, var(--map-height, 320px))"),
                ("padding", "12px 32px 12px 12px"),
                ("overflow", "auto"),
                ("border", BORDER),
                ("border-radius", "var(--radius-md, 8px)"),
                ("background", SURFACE),
                ("color", TEXT),
                ("transform", "translateX(calc(-50% + var(--shift, 0px)))"),
                ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.15))"),
                ("font-size", "calc(13px * var(--font-scale, 1))"),
                ("line-height", "1.45"),
                ("cursor", "default"),
            ]),
        )
        .nest("& [data-map-popup][data-below]", pairs(&[("bottom", "auto"), ("top", "10px")]))
        .nest("& [data-map-popup] > strong", pairs(&[("display", "block"), ("font-weight", "600")]))
        .nest("& [data-map-popup] > p", pairs(&[("margin", "2px 0 0"), ("color", MUTED)]))
        .nest("& [data-map-popup] > a", pairs(&[("display", "inline-block"), ("margin-top", "6px"), ("color", "var(--color-primary, currentColor)"), ("text-decoration", "underline"), ("text-underline-offset", "3px")]))
        .nest(
            "& [data-map-close]",
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
        .nest("& [data-map-close]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-map-close]:focus-visible", pairs(&[("outline", RING)]))
        .nest("& [data-map-failed]", pairs(&[("position", "absolute"), ("inset", "0"), ("overflow", "auto"), ("padding", "16px"), ("color", MUTED)]))
        .nest("& [data-map-failed] ul", pairs(&[("margin", "8px 0 0"), ("padding-inline-start", "18px"), ("color", TEXT)]))
        .nest("&[data-dark][data-map-dark] .leaflet-tile-pane[data-filter]", pairs(&[("filter", "invert(1) hue-rotate(180deg) brightness(0.95) contrast(0.9)")]))
        .nest("@media (prefers-reduced-motion: no-preference)", pairs(&[]).nest("& [data-map-pin]", pairs(&[("transition", "transform .12s")])))
});

static CANVAS: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("position", "absolute"), ("inset", "0"), ("z-index", "0")]));

/// Where new alarms are announced, out of sight.
static LIVE_REGION: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]));

/// What the map has to say: that it's loading, how to zoom it, what went wrong.
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

static POPUP: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "8px")]).nest("&[hidden]", pairs(&[("display", "none")])));

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    let at = |p: &[(&str, &str)]| pairs(p);
    pairs(&[("position", "absolute"), ("z-index", "2"), ("max-width", "calc(100% - 24px)")])
        .nest("&[data-position=top-left]", at(&[("top", "12px"), ("left", "12px")]))
        .nest("&[data-position=top-center]", at(&[("top", "12px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=top-right]", at(&[("top", "12px"), ("right", "12px")]))
        .nest("&[data-position=bottom-left]", at(&[("bottom", "24px"), ("left", "12px")]))
        .nest("&[data-position=bottom-center]", at(&[("bottom", "24px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=bottom-right]", at(&[("bottom", "24px"), ("right", "12px")]))
});

static CONTROLS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "2"),
        ("right", "12px"),
        ("bottom", "28px"),
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
    .nest("&[aria-pressed=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary, #18181b) 14%, transparent)"), ("color", "var(--color-primary, #18181b)")]))
});

const GLOBAL_CSS: &str = "@keyframes ui-map-pulse{from{transform:scale(.2);opacity:.55}to{transform:scale(1);opacity:0}}\
@keyframes ui-map-glow{from{filter:drop-shadow(0 0 1px var(--tone))}to{filter:drop-shadow(0 0 4px var(--tone)) drop-shadow(0 0 12px var(--tone))}}\
@keyframes ui-map-halo{from{box-shadow:0 0 0 0 var(--tone)}to{box-shadow:0 0 12px 6px var(--tone)}}\
@keyframes ui-map-bounce{from{transform:none}to{transform:translateY(-9px)}}\
@keyframes ui-map-blink{from{opacity:1}to{opacity:.35}}";

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
