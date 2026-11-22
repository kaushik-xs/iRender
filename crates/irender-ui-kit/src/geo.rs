//! `ui-location`, `ui-field-map`, `ui-geo-checkin`, `ui-route-tracker`, `ui-compass` and
//! `ui-level`. `geo.js` runs them, and `field_map.js` the map, without a library: its tiles are
//! pictures from a tile server, and what's on them is SVG.

use crate::{
    BG_CARD, BG_POPOVER, BORDER, MUTED, PRIMARY, SUBTLE, SURFACE, TEXT, lit,
    widget::{self, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("geo.js");
const MAP: &str = include_str!("field_map.js");
const TILES: &str = "https://tile.openstreetmap.org/{z}/{x}/{y}.png";

fn scripts(cx: &mut RenderCx<'_>) {
    widget::device_kit(cx);
    cx.global_js("ui-geo", SCRIPT);
}

// ---------------------------------------------------------------- location

pub struct Location;

const LOCATION_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets `{ lat, lng, accuracy, altitude, heading, speed, at }`, the accuracy in metres").required(),
    PropDef::new("watch", PropKind::Bool, "Follows the device as it moves, while the page is open"),
    PropDef::new("auto", PropKind::Bool, "Found by itself, where it's been allowed"),
    PropDef::new("precise", PropKind::Bool, "With GPS rather than a quick guess (default true)"),
    PropDef::new("enough", PropKind::Number, "Metres: keeps trying for a while until it's as accurate as this"),
    PropDef::new("manual", PropKind::Bool, "Can be typed instead"),
    PropDef::new("link", PropKind::Any, "Where its Open in maps goes, with `{lat}` and `{lng}` (default OpenStreetMap); null for none"),
    PropDef::new("label", PropKind::Any, "The button (default `Use my location`)"),
];

static LOCATION: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& [data-geo-at]", pairs(&[("font-variant-numeric", "tabular-nums"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& a:not([data-w-btn])", pairs(&[("color", PRIMARY), ("text-underline-offset", "3px")]))
        .nest("& [data-w-field]", pairs(&[("width", "150px")]))
});

impl Component for Location {
    fn name(&self) -> &'static str {
        "ui-location"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Where the device is, once or as it moves, with how accurate that is; typed instead where it can't be found.", props: LOCATION_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["label"], &["enough"], &["bind"])?;
        if let Some(l) = cx.prop(node, "link") {
            config.insert("link".into(), l);
        }
        widget::flags(cx, node, &mut config, &[("watch", false), ("auto", false), ("precise", true), ("manual", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &LOCATION, "ui-location", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Location")))])
    }
}

// ---------------------------------------------------------------- map

pub struct FieldMap;

const DRAW: &[&str] = &["marker", "line", "polygon", "circle"];
const MAP_PROPS: &[PropDef] = &[
    PropDef::new("center", PropKind::Any, "`{ lat, lng }` or `[lat, lng]`; may use state, and the map follows it (default: what's on it)"),
    PropDef::new("zoom", PropKind::Any, "0 (the world) to 19 (a building); may use state"),
    PropDef::new("markers", PropKind::Any, "`[{ id, lat, lng, label, text, color, glyph }]`: `text` is shown when it's picked, `glyph` is a character in the pin"),
    PropDef::new("areas", PropKind::Any, "`[{ id, points: [[lat, lng]], label, color }]`, or a circle: `{ lat, lng, radius }`, in metres"),
    PropDef::new("lines", PropKind::Any, "`[{ id, points, color, width, dashed, label }]`"),
    PropDef::new("cluster", PropKind::Bool, "Markers near each other are one, with their number, until zoomed in"),
    PropDef::new("bindSelected", PropKind::StateKey, "State key for the id of the marker, area or line picked"),
    PropDef::new("bindView", PropKind::StateKey, "State key that gets `{ lat, lng, zoom, bounds: [[south, west], [north, east]] }` as it's moved"),
    PropDef::new("bindPoint", PropKind::StateKey, "State key that gets `{ lat, lng }` where the map was clicked"),
    PropDef::new("bindDrawn", PropKind::StateKey, "State key for what's drawn on it: `[{ id, type, points, radius }]`"),
    PropDef::new("draw", PropKind::Any, "What can be drawn: `[marker, line, polygon, circle]`"),
    PropDef::new("locate", PropKind::Bool, "A button shows where the device is"),
    PropDef::new("position", PropKind::Any, "A dot at `{ lat, lng, accuracy, heading }`, e.g. what `ui-location` gives"),
    PropDef::new("fit", PropKind::Bool, "Starts with everything on it in view (default true, without `center`)"),
    PropDef::new("height", PropKind::Any, "Default 360 px"),
    PropDef::new("tiles", PropKind::Any, "Tiles' address, with `{z}`, `{x}` and `{y}`; `none` for a plain ground (default OpenStreetMap's, which are for light use: name your own for an app)"),
    PropDef::new("attribution", PropKind::Any, "Whose map it is (default `© OpenStreetMap contributors`)"),
    PropDef::new("darkTiles", PropKind::Bool, "In dark mode the tiles are turned dark (default true)"),
    PropDef::new("wheel", PropKind::Bool, "The wheel zooms without Ctrl or ⌘"),
    PropDef::new("minZoom", PropKind::Number, "Default 1"),
    PropDef::new("maxZoom", PropKind::Number, "Default 19"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Map`)"),
];

static MAP_STYLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(pairs(&[
        ("position", "relative"),
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("height", "var(--map-h, 360px)"),
        ("overflow", "hidden"),
        ("border", "var(--border-width, 1px) solid"),
        ("border-color", BORDER),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("background", "color-mix(in srgb, var(--color-text) 7%, var(--color-surface))"),
        ("color", TEXT),
        ("font-size", fs!(13)),
        ("isolation", "isolate"),
    ]))
    .nest("& [data-map-pane]", pairs(&[("position", "absolute"), ("inset", "0"), ("touch-action", "none"), ("cursor", "grab"), ("user-select", "none"), ("-webkit-user-select", "none"), ("outline-offset", "-3px")]))
    .nest("& [data-map-pane][data-moving]", pairs(&[("cursor", "grabbing")]))
    .nest("&[data-tool] [data-map-pane]", pairs(&[("cursor", "crosshair")]))
    .nest("& [data-map-tiles]", pairs(&[("position", "absolute"), ("inset", "0"), ("pointer-events", "none")]))
    .nest("& [data-map-tiles] img", pairs(&[("position", "absolute"), ("width", "256px"), ("height", "256px"), ("max-width", "none"), ("transform-origin", "0 0")]))
    .nest("&[data-dark-tiles] [data-map-tiles]", pairs(&[]).nest("@dark", pairs(&[("filter", "invert(1) hue-rotate(180deg) brightness(0.95) contrast(0.9)")])))
    .nest("& svg[data-map-over]", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("overflow", "visible"), ("pointer-events", "none")]))
    .nest("& [data-map-shape]", pairs(&[("pointer-events", "auto"), ("cursor", "pointer")]))
    .nest("& [data-map-pins]", pairs(&[("position", "absolute"), ("inset", "0"), ("pointer-events", "none")]))
    .nest("& [data-map-pin]", pairs(&[("position", "absolute"), ("left", "0"), ("top", "0"), ("width", "30px"), ("height", "38px"), ("margin", "-37px 0 0 -15px"), ("padding", "0"), ("border", "0"), ("background", "none"), ("color", "var(--pin, var(--color-primary))"), ("cursor", "pointer"), ("pointer-events", "auto")]))
    .nest("& [data-map-pin] svg", pairs(&[("display", "block"), ("width", "30px"), ("height", "38px"), ("filter", "drop-shadow(0 1px 2px rgb(0 0 0 / 0.4))")]))
    .nest("& [data-map-pin] b", pairs(&[("position", "absolute"), ("inset-inline", "0"), ("top", "6px"), ("color", "#fff"), ("font-size", "12px"), ("font-weight", "600"), ("line-height", "18px"), ("text-align", "center")]))
    .nest("& [data-map-pin][aria-pressed=true]", pairs(&[("z-index", "2"), ("scale", "1.18"), ("transform-origin", "50% 100%")]))
    .nest("& [data-map-cluster]", pairs(&[("position", "absolute"), ("left", "0"), ("top", "0"), ("display", "grid"), ("place-items", "center"), ("width", "36px"), ("height", "36px"), ("margin", "-18px 0 0 -18px"), ("padding", "0"), ("border", "3px solid color-mix(in srgb, var(--color-primary) 40%, transparent)"), ("border-radius", "50%"), ("background", PRIMARY), ("background-clip", "padding-box"), ("color", crate::ON_PRIMARY), ("font", "inherit"), ("font-weight", "600"), ("cursor", "pointer"), ("pointer-events", "auto")]))
    .nest("& [data-map-me]", pairs(&[("position", "absolute"), ("left", "0"), ("top", "0"), ("width", "16px"), ("height", "16px"), ("margin", "-8px 0 0 -8px"), ("box-sizing", "border-box"), ("border", "3px solid #fff"), ("border-radius", "50%"), ("background", "#2563eb"), ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.25), 0 1px 4px rgb(0 0 0 / 0.4)")]))
    .nest("& [data-map-pop]", pairs(&[("position", "absolute"), ("z-index", "3"), ("left", "0"), ("top", "0"), ("width", "max-content"), ("max-width", "220px"), ("padding", pad!(8, 12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER), ("box-shadow", "0 6px 20px rgb(0 0 0 / 0.2)"), ("pointer-events", "auto")]))
    .nest("& [data-map-pop] strong", pairs(&[("display", "block"), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [data-map-pop] span", pairs(&[("color", MUTED), ("white-space", "pre-line")]))
    .nest("& [data-map-tools]", pairs(&[("position", "absolute"), ("z-index", "4"), ("top", "10px"), ("inset-inline-end", "10px"), ("display", "flex"), ("flex-direction", "column"), ("gap", "6px")]))
    .nest("& [data-map-draw]", pairs(&[("position", "absolute"), ("z-index", "4"), ("top", "10px"), ("inset-inline-start", "10px"), ("display", "flex"), ("flex-wrap", "wrap"), ("gap", "6px"), ("max-width", "calc(100% - 70px)")]))
    .nest("& :is([data-map-tools], [data-map-draw]) button", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("min-width", "36px"), ("height", "36px"), ("padding", "0 8px"), ("border", "1px solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("box-shadow", "0 1px 3px rgb(0 0 0 / 0.15)"), ("cursor", "pointer")]))
    .nest("& :is([data-map-tools], [data-map-draw]) button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& :is([data-map-tools], [data-map-draw]) button[aria-pressed=true]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 16%, var(--color-surface))")]))
    .nest("& :is([data-map-tools], [data-map-draw]) button:disabled", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
    .nest("& [data-map-by]", pairs(&[("position", "absolute"), ("z-index", "4"), ("bottom", "0"), ("inset-inline-end", "0"), ("padding", "1px 6px"), ("background", "color-mix(in srgb, var(--color-surface) 80%, transparent)"), ("color", MUTED), ("font-size", "11px")]))
    .nest("& [data-map-scale]", pairs(&[("position", "absolute"), ("z-index", "4"), ("bottom", "8px"), ("inset-inline-start", "10px"), ("padding", "0 4px"), ("border", "1.5px solid var(--color-text)"), ("border-top", "0"), ("background", "color-mix(in srgb, var(--color-surface) 70%, transparent)"), ("font-size", "11px"), ("line-height", "14px"), ("pointer-events", "none")]))
    .nest("& [data-map-hint]", pairs(&[("position", "absolute"), ("z-index", "4"), ("bottom", "30px"), ("inset-inline", "0"), ("margin-inline", "auto"), ("width", "fit-content"), ("max-width", "90%"), ("padding", pad!(6, 12)), ("border-radius", "999px"), ("background", "rgb(0 0 0 / 0.72)"), ("color", "#fff"), ("text-align", "center"), ("pointer-events", "none")]))
});

impl Component for FieldMap {
    fn name(&self) -> &'static str {
        "ui-field-map"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A light map to move and zoom, drawn without a library: markers (grouped when they're close), areas, lines, where the device is, and drawing on it.", props: MAP_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["center", "zoom", "markers", "areas", "lines", "position"], &["draw", "minZoom", "maxZoom", "attribution"], &["bindSelected", "bindView", "bindPoint", "bindDrawn"])?;
        if let Some(d) = config.get("draw").and_then(Json::as_array)
            && let Some(bad) = d.iter().find(|t| !DRAW.contains(&t.as_str().unwrap_or_default()))
        {
            return Err(RenderError::new(node, format!("{bad} can't be drawn: {}", DRAW.join(", "))));
        }
        if config.contains_key("draw") && !config.contains_key("bindDrawn") {
            return Err(RenderError::new(node, "`draw` needs `bindDrawn`"));
        }
        let tiles = cx.prop_string(node, "tiles").unwrap_or_else(|| TILES.into());
        if tiles != "none" && !crate::combobox::safe_search_url(&tiles) {
            return Err(RenderError::new(node, format!("`tiles` must be a path or an http(s) address with {{z}}, {{x}} and {{y}}, got `{tiles}`")));
        }
        if !config.contains_key("attribution") && tiles == TILES {
            config.insert("attribution".into(), json!(cx.label("© OpenStreetMap contributors")));
        }
        config.insert("tiles".into(), json!(tiles));
        let has_center = config.contains_key("center");
        widget::flags(cx, node, &mut config, &[("cluster", false), ("locate", false), ("fit", !has_center), ("wheel", false)]);
        scripts(cx);
        cx.global_js("ui-field-map", MAP);
        let mut style = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "height", h)?
        {
            style.push("--map-h", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&MAP_STYLE).base_style(style).behavior("ui-field-map", Json::Object(config));
        el.attr("role", "region").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Map"))));
        if widget::flag(cx, node, "darkTiles", true) {
            el.attr("data-dark-tiles", true);
        }
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- check-in

pub struct GeoCheckin;

const CHECKIN_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the visit that's open, `{ id, place, in: { at, lat, lng, accuracy, distance, inside, reason, photo } }` (null when there's none)").required(),
    PropDef::new("places", PropKind::Any, "Where one can check in: `[{ id, name, lat, lng, radius }]`, metres, or `points` for an area; may use state").required(),
    PropDef::new("bindLog", PropKind::StateKey, "State key that collects the visits that have ended, with `out` and `seconds`"),
    PropDef::new("outside", PropKind::Enum(&["block", "reason", "allow"]), "Away from every place: not at all (default), with a reason, or allowed"),
    PropDef::new("enough", PropKind::Number, "The least accuracy that counts, in metres (default 100)"),
    PropDef::new("ready", PropKind::Any, "The button waits for this, e.g. `{{ photo != null }}`"),
    PropDef::new("photo", PropKind::Any, "Kept with it, e.g. `{{ photo }}`"),
    PropDef::new("inLabel", PropKind::Any, "Default `Check in`"),
    PropDef::new("outLabel", PropKind::Any, "Default `Check out`"),
];

static CHECKIN: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(10), "100%"))
        .nest("& [data-ci-since]", pairs(&[("font-size", fs!(26)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-ci-body]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10))]))
});

impl Component for GeoCheckin {
    fn name(&self) -> &'static str {
        "ui-geo-checkin"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Checks in and out where someone has to be: how far they are from the place, the time, how long it's been, and a photo. Children (a `ui-camera`, say) go before the button. A device can be made to lie about where it is, so it's a record, not proof.", props: CHECKIN_PROPS, parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["places", "ready", "photo", "inLabel", "outLabel"], &["outside", "enough"], &["bind", "bindLog"])?;
        config.insert("gated".into(), json!(node.prop("ready").is_some()));
        scripts(cx);
        let mut el = cx.element("div", node);
        el.base_style_ref(&CHECKIN).behavior("ui-geo-checkin", Json::Object(config)).attr("role", "group").attr("aria-label", cx.label("Check in"));
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- route

pub struct RouteTracker;

const ROUTE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the route, `{ state: idle|recording|paused|done, points: [[lat, lng, time, accuracy]], distance, seconds, started, ended }`, metres").required(),
    PropDef::new("step", PropKind::Number, "Metres between points at least (default 10)"),
    PropDef::new("enough", PropKind::Number, "Points less accurate than this are left out, in metres (default 50)"),
    PropDef::new("units", PropKind::Enum(&["metric", "imperial"]), "Default `metric`"),
    PropDef::new("awake", PropKind::Bool, "Keeps the screen on while it records (default true): a phone stops saying where it is once it sleeps"),
    PropDef::new("download", PropKind::String, "A button saves it as GPX under this name, e.g. `route.gpx`"),
];

static ROUTE: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(12), "100%"))
        .nest("& dl", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fit, minmax(90px, 1fr))"), ("gap", gap!(10)), ("margin", "0")]))
        .nest("& dt", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& dd", pairs(&[("margin", "0"), ("font-size", fs!(22)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums")]))
});

impl Component for RouteTracker {
    fn name(&self) -> &'static str {
        "ui-route-tracker"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Records a path while the app is open and in front: distance, time and speed, with pause, and GPX to save. Draw it on a `ui-field-map`, and keep it with `ui-local-store`.", props: ROUTE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        if let Some(d) = cx.prop_string(node, "download")
            && !d.ends_with(".gpx")
        {
            return Err(RenderError::new(node, format!("`download` must end in .gpx, got `{d}`")));
        }
        let mut config = widget::config(cx, node, &[], &["step", "enough", "units", "download"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("awake", true)]);
        scripts(cx);
        crate::sheets(cx);
        widget::drawn(cx, node, "div", &ROUTE, "ui-route-tracker", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Route")))])
    }
}

// ---------------------------------------------------------------- compass and level

pub struct Compass;
pub struct Level;

const COMPASS_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets the heading, degrees from north"),
    PropDef::new("heading", PropKind::Any, "Or shown from this, e.g. a vehicle's or the wind's"),
    PropDef::new("target", PropKind::Any, "`{ lat, lng }` to point to, from `from`"),
    PropDef::new("from", PropKind::Any, "Where one is, e.g. what `ui-location` gives"),
    PropDef::new("size", PropKind::Number, "In px (default 220)"),
];
const LEVEL_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets `{ x, y, level }`, degrees off level"),
    PropDef::new("value", PropKind::Any, "Or shown from this `{ x, y }`"),
    PropDef::new("tolerance", PropKind::Number, "Degrees that count as level (default 0.5)"),
    PropDef::new("mode", PropKind::Enum(&["surface", "edge", "auto"]), "Lying on a surface, stood on an edge, or whichever it is (default `auto`)"),
    PropDef::new("size", PropKind::Number, "In px (default 220)"),
];

static DIAL: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(pairs(&[("display", "inline-flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(10)), ("max-width", "100%"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& svg[data-dial]", pairs(&[("display", "block"), ("max-width", "100%"), ("height", "auto"), ("overflow", "visible")]))
        .nest("& [data-dial-face]", pairs(&[("fill", BG_CARD), ("stroke", BORDER)]))
        .nest("& svg[data-dial] text", pairs(&[("fill", "currentColor"), ("font-size", "13px"), ("text-anchor", "middle"), ("dominant-baseline", "central")]))
        .nest("& [data-dial-value]", pairs(&[("font-size", fs!(24)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums"), ("min-height", "1.3em"), ("line-height", "1.3")]))
        .nest("&[data-level] [data-bubble]", pairs(&[("fill", "var(--color-success, #16a34a)")]))
});

impl Component for Compass {
    fn name(&self) -> &'static str {
        "ui-compass"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Which way the device faces, and the way to a place from where one is. A phone's magnet is easily put off by metal.", props: COMPASS_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["heading", "target", "from"], &["size"], &["bind"])?;
        scripts(cx);
        widget::drawn(cx, node, "div", &DIAL, "ui-compass", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Compass")))])
    }
}

impl Component for Level {
    fn name(&self) -> &'static str {
        "ui-level"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A spirit level from the device's own sensors, set to zero against a surface known to be level.", props: LEVEL_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["value"], &["tolerance", "mode", "size"], &["bind"])?;
        scripts(cx);
        widget::drawn(cx, node, "div", &DIAL, "ui-level", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Spirit level")))])
    }
}
