//! `ui-install-prompt`, `ui-update-prompt`, `ui-offline-indicator`, `ui-offline-queue`,
//! `ui-local-store` and `ui-app-shell` (with `ui-app-screen` and `ui-app-actions`): what an
//! installed app needs around its pages. `pwa.js` runs them, and `offline.js` keeps things
//! on the device; the document's `app` makes the pages installable.

use crate::{BG_CARD, BORDER, MUTED, PRIMARY, SUBTLE, SURFACE, TEXT, bin, cond, expr_val, icon, lit, synthetic, val_expr, widget::{self, pairs}};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock,
    expr::{BinOp, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("pwa.js");
pub(crate) const OFFLINE: &str = include_str!("offline.js");
const WORKER: &str = include_str!("worker.js");

/// The service worker of an app: `told` is [`irender_core::AppSpec::worker`].
pub fn service_worker(told: &Json) -> String {
    format!("const APP = {told};\n{OFFLINE}\n{WORKER}")
}

/// Adds what keeps things on the device.
pub(crate) fn offline(cx: &mut RenderCx<'_>) {
    widget::device_kit(cx);
    cx.global_js("ui-offline", OFFLINE);
}

fn scripts(cx: &mut RenderCx<'_>) {
    offline(cx);
    cx.global_js("ui-pwa", SCRIPT);
}

/// A message that keeps to the foot of the window, or sits in the page.
fn notice() -> StyleBlock {
    widget::device(pairs(&[("display", "block"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest(
            "& [data-pwa-bar]",
            pairs(&[
                ("display", "flex"),
                ("flex-wrap", "wrap"),
                ("align-items", "center"),
                ("gap", gap!(10, 14)),
                ("box-sizing", "border-box"),
                ("padding", pad!(12, 14)),
                ("border", "var(--border-width, 1px) solid"),
                ("border-color", BORDER),
                ("border-radius", "var(--radius-card, var(--radius-lg))"),
                ("background", BG_CARD),
                ("color", TEXT),
            ]),
        )
        .nest("& [data-pwa-bar][data-floating]", pairs(&[("position", "fixed"), ("z-index", "80"), ("inset-inline", "12px"), ("bottom", "calc(12px + env(safe-area-inset-bottom, 0px))"), ("max-width", "460px"), ("margin-inline", "auto"), ("box-shadow", "var(--elevation-dialog, 0 12px 32px rgb(0 0 0 / 0.2))")]))
        .nest("& [data-pwa-bar] img", pairs(&[("flex", "none"), ("width", "40px"), ("height", "40px"), ("border-radius", "9px")]))
        .nest("& [data-pwa-text]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("flex", "1 1 180px"), ("min-width", "0")]))
        .nest("& [data-pwa-text] strong", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-pwa-text] span", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
}

static NOTICE: LazyLock<StyleBlock> = LazyLock::new(notice);

// ---------------------------------------------------------------- install

pub struct InstallPrompt;

const INSTALL_PROPS: &[PropDef] = &[
    PropDef::new("variant", PropKind::Enum(&["button", "card", "floating"]), "Default `button`; `floating` is a card at the foot of the window"),
    PropDef::new("label", PropKind::Any, "The button (default `Install the app`)"),
    PropDef::new("title", PropKind::Any, "On the card (default the app's name)"),
    PropDef::new("text", PropKind::Any, "Under it (default `Opens from your home screen, and works offline.`)"),
    PropDef::new("icon", PropKind::Any, "A picture's address, on the card"),
    PropDef::new("bindState", PropKind::StateKey, "State key that gets `available` (the browser can install it), `steps` (it's added from a menu, which the button explains), `installed` or `unavailable`"),
    PropDef::new("remember", PropKind::String, "A card has a Not now button, and once that's pressed stays away for `days` in this browser, under this name"),
    PropDef::new("days", PropKind::Number, "Default 30"),
    PropDef::new("always", PropKind::Bool, "Shown wherever it is, with the steps where there's no prompt"),
];

impl Component for InstallPrompt {
    fn name(&self) -> &'static str {
        "ui-install-prompt"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Offers to install the app: the browser's own prompt where there is one, else the steps for this browser (on an iPhone, Add to Home Screen). Hidden in the installed app. Needs the document's `app`.", props: INSTALL_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["label", "title", "text", "icon"], &["variant", "remember", "days"], &["bindState"])?;
        widget::flags(cx, node, &mut config, &[("always", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &NOTICE, "ui-install-prompt", config, vec![])
    }
}

// ---------------------------------------------------------------- update

pub struct UpdatePrompt;

const UPDATE_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Default `A new version is ready`"),
    PropDef::new("text", PropKind::Any, "Under it, e.g. what's new"),
    PropDef::new("label", PropKind::Any, "The button (default `Reload`)"),
    PropDef::new("laterLabel", PropKind::Any, "Default `Later`; null for none"),
    PropDef::new("every", PropKind::Number, "Minutes between looking for one, besides each time the app's opened (default 60)"),
    PropDef::new("inline", PropKind::Bool, "In the page rather than at the foot of the window"),
    PropDef::new("show", PropKind::Any, "Shown whether there's one or not, e.g. when the host says"),
    PropDef::new("bind", PropKind::StateKey, "State key that's true while a new version is waiting"),
];

impl Component for UpdatePrompt {
    fn name(&self) -> &'static str {
        "ui-update-prompt"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Says when a new version of the app has been fetched, and reloads into it when asked. `reload` comes first, for saving what's open.", props: UPDATE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["title", "text", "label", "show"], &["every"], &["bind"])?;
        if let Some(l) = cx.prop(node, "laterLabel") {
            config.insert("laterLabel".into(), l);
        }
        widget::flags(cx, node, &mut config, &[("inline", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &NOTICE, "ui-update-prompt", config, vec![])
    }
}

// ---------------------------------------------------------------- offline indicator

pub struct OfflineIndicator;

const INDICATOR_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets `online`, `offline` or `reconnecting`"),
    PropDef::new("variant", PropKind::Enum(&["badge", "banner", "dot", "none"]), "Default `badge`; `none` only keeps the state"),
    PropDef::new("showOnline", PropKind::Bool, "Shown when there's a connection too"),
    PropDef::new("checkUrl", PropKind::Any, "Asked while there's doubt, as a device can be on a network that goes nowhere"),
    PropDef::new("every", PropKind::Number, "Seconds between those (default 15)"),
    PropDef::new("offlineText", PropKind::Any, "Default `Offline`"),
    PropDef::new("onlineText", PropKind::Any, "Default `Online`"),
    PropDef::new("reconnectingText", PropKind::Any, "Default `Reconnecting…`"),
    PropDef::new("detail", PropKind::Any, "After it in a banner, e.g. `What you do is kept and sent later.`"),
];

static INDICATOR: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(pairs(&[("display", "inline-flex"), ("max-width", "100%"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("&[data-variant=banner]", pairs(&[("display", "block")]))
        .nest("&[data-variant=banner] [data-pwa-state]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("justify-content", "center"), ("gap", gap!(6)), ("padding", pad!(8, 14)), ("border-radius", "0"), ("font-size", fs!(13)), ("white-space", "normal"), ("text-align", "center")]))
        .nest("& [data-pwa-state] i", pairs(&[("flex", "none"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--w-tone)")]))
        .nest("&[data-variant=dot] [data-pwa-state]", pairs(&[("padding", "4px"), ("background", "none")]))
        .nest("& [data-pwa-state=reconnecting] i", pairs(&[("animation", "ui-blink 1s infinite")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& i", pairs(&[("animation", "none !important")])))
});

const BLINK: &str = "@keyframes ui-blink { 50% { opacity: 0.3; } }";

impl Component for OfflineIndicator {
    fn name(&self) -> &'static str {
        "ui-offline-indicator"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Whether there's a connection: a badge, a banner or a dot, and state for the rest of the page.", props: INDICATOR_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::check_url(cx, node, "checkUrl")?;
        let mut config = widget::config(cx, node, &["checkUrl", "offlineText", "onlineText", "reconnectingText", "detail"], &["every"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("showOnline", false)]);
        let variant = cx.prop_string(node, "variant").unwrap_or_else(|| "badge".into());
        config.insert("variant".into(), json!(variant));
        scripts(cx);
        cx.global_css("ui-blink", BLINK);
        widget::drawn(cx, node, "div", &INDICATOR, "ui-offline-indicator", config, vec![("data-variant", lit(variant)), ("role", lit("status"))])
    }
}

// ---------------------------------------------------------------- offline queue

pub struct OfflineQueue;

const QUEUE_PROPS: &[PropDef] = &[
    PropDef::new("paths", PropKind::Any, "Addresses whose requests are kept, e.g. `[/api/inspections]`: any under them (default every one on the site)"),
    PropDef::new("methods", PropKind::Any, "Default `[POST, PUT, PATCH, DELETE]`"),
    PropDef::new("labels", PropKind::Any, "What to call them: `{ /api/inspections: Inspection }`"),
    PropDef::new("labelField", PropKind::String, "A field of what's sent that says which, e.g. `title`"),
    PropDef::new("variant", PropKind::Enum(&["list", "badge", "none"]), "Default `list`"),
    PropDef::new("bind", PropKind::StateKey, "State key that gets what's waiting: `[{ id, label, url, method, at, status, error, tries }]`"),
    PropDef::new("bindCount", PropKind::StateKey, "State key that gets how many"),
    PropDef::new("retryEvery", PropKind::Number, "Seconds between tries while the page is open (default 60)"),
    PropDef::new("title", PropKind::Any, "Over the list (default `Waiting to be sent`)"),
    PropDef::new("emptyText", PropKind::Any, "Default `Everything has been sent`"),
    PropDef::new("hideEmpty", PropKind::Bool, "Nothing is shown when there's nothing waiting"),
];

static QUEUE: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("&[data-variant=badge]", pairs(&[("display", "inline-flex"), ("width", "auto")]))
        .nest("& [data-w-list] small", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
});

impl Component for OfflineQueue {
    fn name(&self) -> &'static str {
        "ui-offline-queue"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Keeps what's sent without a connection (submits, forms and uploads) and sends it once there is one, in order, even after the app's been closed. Shows what's waiting and what was refused. Each is sent with an `x-queued-id` header, so a server can tell one it has had.", props: QUEUE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["title", "emptyText"], &["paths", "methods", "labels", "labelField", "retryEvery"], &["bind", "bindCount"])?;
        let variant = cx.prop_string(node, "variant").unwrap_or_else(|| "list".into());
        config.insert("variant".into(), json!(variant));
        widget::flags(cx, node, &mut config, &[("hideEmpty", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &QUEUE, "ui-offline-queue", config, vec![("data-variant", lit(variant))])
    }
}

// ---------------------------------------------------------------- local store

pub struct LocalStore;

const STORE_PROPS: &[PropDef] = &[
    PropDef::new("keys", PropKind::Any, "State keys to keep, e.g. `[draft, filters]`").required(),
    PropDef::new("name", PropKind::String, "What they're kept under (default the page's address)"),
    PropDef::new("where", PropKind::Enum(&["device", "session"]), "`device` (default) until cleared, `session` until the tab's closed"),
    PropDef::new("maxAge", PropKind::Number, "Days after which they're let go (default never)"),
    PropDef::new("version", PropKind::Any, "What's kept under another one is let go"),
    PropDef::new("sync", PropKind::Bool, "Other tabs follow (default true)"),
    PropDef::new("bindReady", PropKind::StateKey, "State key that's true once what was kept has been put back"),
    PropDef::new("bindSaved", PropKind::StateKey, "State key that gets the time they were last kept"),
    PropDef::new("clear", PropKind::Any, "When this becomes true, what's kept is let go"),
];

impl Component for LocalStore {
    fn name(&self) -> &'static str {
        "ui-local-store"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Keeps state keys on the device between visits. Draws nothing. Not for secrets: anyone with the device can read them.", props: STORE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["clear"], &["keys", "name", "where", "maxAge", "version"], &["bindReady", "bindSaved"])?;
        let ok = config.get("keys").and_then(Json::as_array).is_some_and(|k| !k.is_empty() && k.iter().all(Json::is_string));
        if !ok {
            return Err(RenderError::new(node, "`keys` is a list of state keys"));
        }
        widget::flags(cx, node, &mut config, &[("sync", true)]);
        scripts(cx);
        let mut el = cx.element("div", node);
        el.attr("hidden", true).behavior("ui-local-store", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- app shell

pub struct AppShell;
pub struct AppScreen;
pub struct AppActions;

const SHELL_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the screen shown, a `ui-app-screen`'s `value`").required(),
    PropDef::new("title", PropKind::Any, "In the bar (default the screen's own)"),
    PropDef::new("height", PropKind::Any, "Default the window's; px or a CSS length"),
    PropDef::new("rail", PropKind::Bool, "With room for it, the tabs go down the side (default true)"),
    PropDef::new("bar", PropKind::Bool, "The bar at the top (default true)"),
    PropDef::new("pullToRefresh", PropKind::Bool, "Pulling the screen down runs `on: { refresh: … }`"),
    PropDef::new("bindRefreshing", PropKind::StateKey, "State key that pulling sets to true; set it to false once it's done (default: it stops by itself)"),
    PropDef::new("history", PropKind::Bool, "The browser's and the phone's back button go back through the screens"),
    PropDef::new("label", PropKind::Any, "Names the tabs for screen readers (default `Main`)"),
];
const SCREEN_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::String, "What `bind` holds at this screen").required(),
    PropDef::new("label", PropKind::Any, "Its tab, and the bar's title").required(),
    PropDef::new("title", PropKind::Any, "Or the bar's title, where it's another"),
    PropDef::new("icon", PropKind::Any, "Its tab's: an icon name (with the icon kit) or a character"),
    PropDef::new("badge", PropKind::Any, "On its tab, e.g. a count; nothing at 0"),
    PropDef::new("parent", PropKind::String, "The screen its Back button goes to; it has no tab of its own"),
    PropDef::new("flush", PropKind::Bool, "No space around its content, for a list or a map"),
];

static SHELL: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(pairs(&[
        ("--app-height", "100dvh"),
        ("container-type", "inline-size"),
        ("box-sizing", "border-box"),
        ("height", "var(--app-height)"),
        ("overflow", "hidden"),
        ("color", TEXT),
        ("background", "var(--color-background, var(--color-surface))"),
        ("font-family", "var(--font-body, system-ui, sans-serif)"),
        ("font-size", fs!(14)),
        ("-webkit-tap-highlight-color", "transparent"),
    ]))
    .nest("& > [data-app-in]", pairs(&[("display", "grid"), ("grid-template", "'bar' auto 'main' minmax(0, 1fr) 'tabs' auto / minmax(0, 1fr)"), ("height", "100%")]))
    .nest("& [data-app-bar]", pairs(&[("grid-area", "bar"), ("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("box-sizing", "border-box"), ("min-height", "calc(52px + env(safe-area-inset-top, 0px))"), ("padding", "env(safe-area-inset-top, 0px) calc(12px + env(safe-area-inset-right, 0px)) 0 calc(12px + env(safe-area-inset-left, 0px))"), ("border-bottom", "1px solid var(--color-border)"), ("background", SURFACE)]))
    .nest("& [data-app-title]", pairs(&[("flex", "1"), ("min-width", "0"), ("margin", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-size", fs!(17)), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [data-app-back]", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("flex", "none"), ("width", "40px"), ("height", "40px"), ("margin-inline-start", "-8px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "none"), ("color", "inherit"), ("cursor", "pointer")]))
    .nest("& [data-app-back]:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-app-main]", pairs(&[("grid-area", "main"), ("position", "relative"), ("min-width", "0"), ("min-height", "0"), ("overflow", "hidden")]))
    .nest("& [data-app-screen]", pairs(&[("position", "absolute"), ("inset", "0"), ("box-sizing", "border-box"), ("overflow", "auto"), ("overscroll-behavior-y", "contain"), ("padding", "16px calc(16px + env(safe-area-inset-right, 0px)) 24px calc(16px + env(safe-area-inset-left, 0px))"), ("outline", "none")]))
    .nest("& [data-app-screen][data-flush]", pairs(&[("padding", "0")]))
    .nest("& [data-app-pull]", pairs(&[("position", "absolute"), ("z-index", "2"), ("top", "8px"), ("inset-inline-start", "50%"), ("width", "34px"), ("height", "34px"), ("margin-inline-start", "-17px"), ("display", "grid"), ("place-items", "center"), ("border-radius", "50%"), ("background", BG_CARD), ("box-shadow", "0 2px 8px rgb(0 0 0 / 0.2)"), ("opacity", "0"), ("pointer-events", "none")]))
    .nest("& [data-app-pull][data-on] svg", pairs(&[("animation", "ui-spin 0.8s linear infinite")]))
    .nest("& [data-app-tabs]", pairs(&[("grid-area", "tabs"), ("display", "flex"), ("padding", "0 env(safe-area-inset-right, 0px) env(safe-area-inset-bottom, 0px) env(safe-area-inset-left, 0px)"), ("border-top", "1px solid var(--color-border)"), ("background", SURFACE)]))
    .nest("& [data-app-tab]", pairs(&[("position", "relative"), ("display", "flex"), ("flex", "1 1 0"), ("flex-direction", "column"), ("align-items", "center"), ("justify-content", "center"), ("gap", "3px"), ("min-width", "0"), ("min-height", "54px"), ("padding", "6px 4px"), ("border", "0"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(11)), ("cursor", "pointer")]))
    .nest("& [data-app-tab] > span:last-child", pairs(&[("max-width", "100%"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
    .nest("& [data-app-tab] [data-icon]", pairs(&[("font-size", "20px"), ("line-height", "1")]))
    .nest("& [data-app-tab][aria-current]", pairs(&[("color", PRIMARY), ("font-weight", "var(--font-weight-medium, 500)")]))
    .nest("& [data-app-tab]:focus-visible", pairs(&[("outline-offset", "-3px")]))
    .nest("& [data-app-badge]", pairs(&[("position", "absolute"), ("top", "4px"), ("inset-inline-start", "calc(50% + 6px)"), ("min-width", "16px"), ("box-sizing", "border-box"), ("padding", "0 4px"), ("border-radius", "8px"), ("background", crate::DESTRUCTIVE), ("color", crate::ON_DESTRUCTIVE), ("font-size", "10px"), ("font-weight", "600"), ("line-height", "16px"), ("text-align", "center")]))
    .nest(
        "@container (min-width: 720px)",
        pairs(&[])
            .nest("& > [data-app-in][data-rail]", pairs(&[("grid-template", "'tabs bar' auto 'tabs main' minmax(0, 1fr) / auto minmax(0, 1fr)")]))
            .nest("& > [data-app-in][data-rail] > [data-app-tabs]", pairs(&[("flex-direction", "column"), ("gap", "4px"), ("padding", "calc(10px + env(safe-area-inset-top, 0px)) 8px 10px calc(8px + env(safe-area-inset-left, 0px))"), ("border-top", "0"), ("border-inline-end", "1px solid var(--color-border)")]))
            .nest("& > [data-app-in][data-rail] > [data-app-tabs] > [data-app-tab]", pairs(&[("flex", "none"), ("width", "76px"), ("border-radius", "var(--radius-md)")]))
            .nest("& > [data-app-in][data-rail] > [data-app-tabs] > [data-app-tab][aria-current]", pairs(&[("background", SUBTLE)])),
    )
});
static ACTIONS: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("display", "flex"), ("align-items", "center"), ("flex", "none"), ("gap", gap!(4))]));

struct At<'n> {
    node: &'n Node,
    value: String,
    parent: Option<String>,
}

impl Component for AppShell {
    fn name(&self) -> &'static str {
        "ui-app-shell"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "An app's frame: a bar with the screen's title and a Back button, the screen, and tabs at the foot (down the side with room for them), clear of a phone's notch. Children are `ui-app-screen`s and one `ui-app-actions`, at the end of the bar.", props: SHELL_PROPS, parts: &["bar", "main", "tabs"], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = widget::need(cx, node, "bind")?;
        let mut screens: Vec<At<'_>> = Vec::new();
        for c in &node.children {
            match c.kind.as_str() {
                "ui-app-screen" => {
                    let value = cx.prop_string(c, "value").ok_or_else(|| RenderError::new(c, "`ui-app-screen` needs `value`"))?;
                    if screens.iter().any(|s| s.value == value) {
                        return Err(RenderError::new(c, format!("two screens have the value `{value}`")));
                    }
                    screens.push(At { node: c, value, parent: cx.prop_string(c, "parent") });
                }
                "ui-app-actions" => {}
                _ => return Err(RenderError::new(c, "children of `ui-app-shell` must be `ui-app-screen`s or a `ui-app-actions`")),
            }
        }
        if screens.is_empty() {
            return Err(RenderError::new(node, "`ui-app-shell` needs `ui-app-screen` children"));
        }
        if let Some(s) = screens.iter().find(|s| s.parent.as_ref().is_some_and(|p| *p == s.value || !screens.iter().any(|o| o.value == *p))) {
            return Err(RenderError::new(s.node, "`parent` is another screen's `value`"));
        }
        // The screen shown; the first when the state names none.
        let at = || screens.iter().rev().fold(Expr::Lit(json!(screens[0].value)), |rest, s| cond(bin(BinOp::Eq, crate::state(&key), Expr::Lit(json!(s.value))), Expr::Lit(json!(s.value)), rest));
        let is = |v: &str| bin(BinOp::Eq, at(), Expr::Lit(json!(v)));
        // The tab a screen is under: itself, or its parent's.
        let tab_of = |v: &str| {
            let mut v = v.to_string();
            for _ in 0..screens.len() {
                match screens.iter().find(|s| s.value == v).and_then(|s| s.parent.clone()) {
                    Some(p) => v = p,
                    None => break,
                }
            }
            v
        };
        let mut config = widget::config(cx, node, &[], &[], &["bind", "bindRefreshing"])?;
        widget::flags(cx, node, &mut config, &[("pullToRefresh", false), ("history", false)]);
        config.insert("first".into(), json!(screens[0].value));
        config.insert("parents".into(), json!(screens.iter().filter_map(|s| s.parent.as_ref().map(|p| (s.value.clone(), json!(p)))).collect::<serde_json::Map<_, _>>()));
        scripts(cx);
        cx.global_css("ui-spin", crate::SPIN);
        let mut style = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "height", h)?
        {
            style.push("--app-height", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&SHELL).base_style(style).behavior("ui-app-shell", Json::Object(config));
        cx.open(&el, node)?;
        let mut inner = Element::new("div");
        inner.attr("data-app-in", true);
        if widget::flag(cx, node, "rail", true) {
            inner.attr("data-rail", true);
        }
        cx.open(&inner, node)?;
        if widget::flag(cx, node, "bar", true) {
            let mut bar = cx.part("header", node, "bar");
            bar.attr("data-app-bar", true);
            cx.open(&bar, node)?;
            let back = screens.iter().filter(|s| s.parent.is_some()).fold(Expr::Lit(Json::Null), |rest, s| cond(is(&s.value), Expr::Lit(json!(s.parent)), rest));
            if screens.iter().any(|s| s.parent.is_some()) {
                let b = synthetic(
                    node,
                    "button",
                    Vec::new(),
                    vec![("type", lit("button")), ("data-app-back", lit(true)), ("aria-label", lit(cx.label("Back"))), ("hidden", expr_val(bin(BinOp::Eq, back.clone(), Expr::Lit(Json::Null))))],
                    vec![("click", vec![Action::SetState { key: key.clone(), value: expr_val(back) }])],
                );
                let e = cx.element("button", &b);
                cx.open(&e, &b)?;
                cx.markup(r#"<svg data-flip width="20" height="20" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8l4.5 4.5"/></svg>"#);
                cx.close("button");
            }
            let title = match node.prop("title") {
                Some(t) => t.clone(),
                None => expr_val(screens.iter().rev().fold(Expr::Lit(json!("")), |rest, s| cond(is(&s.value), s.node.prop("title").or_else(|| s.node.prop("label")).map(val_expr).unwrap_or(Expr::Lit(json!(""))), rest))),
            };
            let mut h = Element::new("h1");
            h.attr("data-app-title", true).text(title);
            cx.open(&h, node)?;
            cx.close("h1");
            for c in node.children.iter().filter(|c| c.kind == "ui-app-actions") {
                cx.render(c)?;
            }
            cx.close("header");
        }
        let mut main = cx.part("main", node, "main");
        main.attr("data-app-main", true);
        cx.open(&main, node)?;
        for s in &screens {
            let mut e = cx.element("section", s.node);
            e.attr("data-app-screen", s.value.clone())
                .attr("tabindex", "-1")
                .attr("aria-label", s.node.prop("label").cloned().unwrap_or_else(|| lit("")))
                .attr("hidden", expr_val(bin(BinOp::Ne, at(), Expr::Lit(json!(s.value)))));
            if widget::flag(cx, s.node, "flush", false) {
                e.attr("data-flush", true);
            }
            cx.open(&e, s.node)?;
            cx.children(s.node)?;
            cx.close("section");
        }
        cx.close("main");
        if screens.iter().filter(|s| s.parent.is_none()).count() > 1 {
            let mut nav = cx.part("nav", node, "tabs");
            nav.attr("data-app-tabs", true).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Main"))));
            cx.open(&nav, node)?;
            for s in screens.iter().filter(|s| s.parent.is_none()) {
                let under = screens.iter().filter(|o| tab_of(&o.value) == s.value).fold(Expr::Lit(json!(false)), |rest, o| bin(BinOp::Or, is(&o.value), rest));
                let b = synthetic(
                    s.node,
                    "button",
                    Vec::new(),
                    vec![("type", lit("button")), ("data-app-tab", lit(s.value.clone())), ("aria-current", expr_val(cond(under, Expr::Lit(json!("page")), Expr::Lit(Json::Null))))],
                    vec![("click", vec![Action::SetState { key: key.clone(), value: lit(s.value.clone()) }])],
                );
                let e = cx.element("button", &b);
                cx.open(&e, &b)?;
                if let Some(i) = s.node.prop("icon") {
                    icon(cx, s.node, i, 22)?;
                }
                if let Some(v) = s.node.prop("badge") {
                    let mut n = Element::new("span");
                    n.attr("data-app-badge", true).attr("hidden", expr_val(cond(val_expr(v), Expr::Lit(Json::Null), Expr::Lit(json!(true))))).text(v.clone());
                    cx.open(&n, s.node)?;
                    cx.close("span");
                }
                let mut t = Element::new("span");
                t.text(s.node.prop("label").cloned().unwrap_or_else(|| lit("")));
                cx.open(&t, s.node)?;
                cx.close("span");
                cx.close("button");
            }
            cx.close("nav");
        }
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

impl Component for AppScreen {
    fn name(&self) -> &'static str {
        "ui-app-screen"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A screen of a `ui-app-shell`.", props: SCREEN_PROPS, parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-app-screen` goes directly in a `ui-app-shell`"))
    }
}

impl Component for AppActions {
    fn name(&self) -> &'static str {
        "ui-app-actions"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "What's at the end of a `ui-app-shell`'s bar: buttons, badges, an offline indicator.", props: &[], parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&ACTIONS);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}
