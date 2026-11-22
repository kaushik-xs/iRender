//! `ui-tour` (steps that point at parts of the page), `ui-banner` (a message across the
//! page) and `ui-session-timeout` (asks before a session ends). `guide.js` runs them.

use crate::{
    BG_DIALOG, BG_POPOVER, BORDER, MUTED, PRIMARY_FILL, SUBTLE, SURFACE, TEXT, expr_val, forms, icon, lit, status, synthetic,
    widget::{self, flag, pairs},
};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::Expr,
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("guide.js");

// ---------------------------------------------------------------- tour

pub struct Tour;

const TOUR_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the step shown, from 0 (null: closed); set it to start").required(),
    PropDef::new("steps", PropKind::Any, "`[{ target, title, text, side, image, set }]`: `target`, a CSS selector (none: in the middle); `side`: top, bottom, left or right; `set`: state to set first, e.g. `{ tab: billing }`").required(),
    PropDef::new("autoStart", PropKind::Bool, "Starts by itself, for someone who hasn't seen it (needs `id`)"),
    PropDef::new("id", PropKind::String, "Remembered under this name in the browser once it's finished or skipped"),
    PropDef::new("backdrop", PropKind::Bool, "The rest of the page is dimmed (default true)"),
    PropDef::new("interactive", PropKind::Bool, "The part pointed at can be used while it's shown"),
    PropDef::new("doneLabel", PropKind::Any, "The last step's button (default `Done`)"),
];

static TOUR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "contents")])
        .nest("& [data-tour-hole]", pairs(&[("position", "fixed"), ("z-index", "90"), ("border-radius", "10px"), ("box-shadow", "0 0 0 100vmax rgb(0 0 0 / 0.5), 0 0 0 3px var(--color-ring, #fff)"), ("pointer-events", "none"), ("transition", "all .25s")]))
        .nest("& [data-tour-hole][data-plain]", pairs(&[("box-shadow", "0 0 0 3px var(--color-primary), 0 0 0 7px color-mix(in srgb, var(--color-primary) 30%, transparent)")]))
        .nest("& [data-tour-cover]", pairs(&[("position", "fixed"), ("z-index", "89"), ("inset", "0")]))
        .nest("& [data-tour-cover][data-dim]", pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
        .nest(
            "& [data-tour-card]",
            pairs(&[
                ("position", "fixed"),
                ("z-index", "91"),
                ("box-sizing", "border-box"),
                ("width", "min(340px, calc(100vw - 24px))"),
                ("padding", pad!(16)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-card, var(--radius-lg))"),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("font-family", "var(--font-body, system-ui, sans-serif)"),
                ("font-size", fs!(14)),
                ("line-height", "1.5"),
                ("box-shadow", "var(--elevation-dialog, 0 16px 40px rgb(0 0 0 / 0.25))"),
                ("outline", "none"),
            ]),
        )
        .nest("& [data-tour-card] img", pairs(&[("display", "block"), ("width", "100%"), ("margin-bottom", gap!(10)), ("border-radius", "var(--radius-md)")]))
        .nest("& [data-tour-card] h2", pairs(&[("margin", "0 24px 4px 0"), ("font-size", fs!(15)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-tour-card] p", pairs(&[("margin", "0"), ("color", MUTED)]))
        .nest("& [data-tour-foot]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("margin-top", gap!(14))]))
        .nest("& [data-tour-foot] small", pairs(&[("margin-inline-end", "auto"), ("color", MUTED), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-tour-card] button", pairs(&[("height", ctl!(32)), ("padding", pad!(0, 12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-tour-card] button[data-next]", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& [data-tour-card] button[data-skip]", widget::tool().set("position", "absolute").set("top", "10px").set("inset-inline-end", "10px").set("height", "28px"))
        .nest("& [data-tour-card] button:focus-visible", forms::focus_ring())
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-tour-hole]", pairs(&[("transition", "none")])))
});

impl Component for Tour {
    fn name(&self) -> &'static str {
        "ui-tour"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Steps that point at parts of the page one after another, for someone new to it. Draws nothing until it starts.", props: TOUR_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-tour` needs `bind`"));
        }
        let mut config = widget::config(cx, node, &["steps", "doneLabel"], &["id"], &["bind"])?;
        for (f, d) in [("autoStart", false), ("backdrop", true), ("interactive", false)] {
            config.insert(f.into(), json!(flag(cx, node, f, d)));
        }
        if flag(cx, node, "autoStart", false) && cx.prop_string(node, "id").is_none() {
            return Err(RenderError::new(node, "`autoStart` needs an `id`, to remember who has seen it"));
        }
        widget::kit(cx);
        cx.global_js("ui-guide", SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&TOUR).behavior("ui-tour", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- banner

pub struct Banner;

const BANNER_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "The message"),
    PropDef::new("title", PropKind::Any, "In bold before it"),
    PropDef::new("tone", PropKind::Enum(status::TONES), "Its colour (default `info`)"),
    PropDef::new("variant", PropKind::Enum(&["soft", "solid"]), "Default `soft`"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character"),
    PropDef::new("bind", PropKind::StateKey, "State key that's true while it's closed; with it, there's a close button"),
    PropDef::new("remember", PropKind::String, "Once closed, stays closed in this browser, under this name (change it to show again)"),
    PropDef::new("sticky", PropKind::Enum(&["top", "bottom"]), "Keeps to an edge of the window as the page scrolls"),
    PropDef::new("align", PropKind::Enum(&["start", "center"]), "Default `center`"),
    PropDef::new("until", PropKind::Any, "Goes by itself at this time, e.g. `2026-12-01T06:00`"),
    PropDef::new("from", PropKind::Any, "Shown from this time"),
    PropDef::new("countdown", PropKind::Any, "Time left to this, after the message"),
];

static BANNER: LazyLock<StyleBlock> = LazyLock::new(|| {
    status::tone_rules(
        pairs(&[
            ("--tone", "var(--color-info, #2563eb)"),
            ("display", "flex"),
            ("flex-wrap", "wrap"),
            ("align-items", "center"),
            ("justify-content", "center"),
            ("gap", gap!(6, 12)),
            ("box-sizing", "border-box"),
            ("width", "100%"),
            ("padding", pad!(10, 48, 10, 16)),
            ("position", "relative"),
            ("background", "color-mix(in srgb, var(--tone) 13%, var(--color-surface))"),
            ("border-bottom", "1px solid color-mix(in srgb, var(--tone) 35%, transparent)"),
            ("color", TEXT),
            ("font-size", fs!(14)),
            ("line-height", "1.45"),
        ]),
        "&",
    )
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest("&:dir(rtl)", pairs(&[("padding", pad!(10, 16, 10, 48))]))
    .nest("&[data-align=start]", pairs(&[("justify-content", "flex-start")]))
    .nest("&[data-variant=solid]", pairs(&[("background", "var(--tone)"), ("border-color", "transparent"), ("color", "#fff")]))
    .nest("&[data-sticky]", pairs(&[("position", "sticky"), ("z-index", "30")]))
    .nest("&[data-sticky=top]", pairs(&[("top", "var(--site-nav, 0px)")]))
    .nest("&[data-sticky=bottom]", pairs(&[("bottom", "0"), ("border-bottom", "0"), ("border-top", "1px solid color-mix(in srgb, var(--tone) 35%, transparent)")]))
    .nest("& > svg, & > [data-icon]", pairs(&[("color", "var(--tone)")]))
    .nest("&[data-variant=solid] > svg", pairs(&[("color", "inherit")]))
    .nest("& [data-banner-title]", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [data-banner-left]", pairs(&[("font-variant-numeric", "tabular-nums"), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& a", pairs(&[("color", "inherit"), ("text-underline-offset", "3px")]))
    .nest("& [data-banner-close]", widget::tool().set("position", "absolute").set("inset-inline-end", "10px").set("top", "50%").set("translate", "0 -50%").set("color", "inherit").set("opacity", "0.7"))
    .nest("& [data-banner-close]:hover", pairs(&[("opacity", "1"), ("background", "color-mix(in srgb, currentColor 12%, transparent)")]))
    .nest("& [data-banner-close]:focus-visible", forms::focus_ring())
});

impl Component for Banner {
    fn name(&self) -> &'static str {
        "ui-banner"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A message across the page: maintenance, an offer, a trial's end, a setting to finish. Children (links, buttons) go after it.", props: BANNER_PROPS, parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let bind = cx.prop_string(node, "bind");
        let remember = cx.prop_string(node, "remember");
        if remember.is_some() && bind.is_none() {
            return Err(RenderError::new(node, "`remember` needs `bind`, for the close button"));
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&BANNER)
            .attr("role", "region")
            .attr("aria-label", cx.label("Announcement"))
            .attr("data-tone", cx.prop_string(node, "tone").unwrap_or_else(|| "info".into()))
            .attr("data-variant", cx.prop_string(node, "variant").unwrap_or_else(|| "soft".into()));
        for p in ["sticky", "align"] {
            if let Some(v) = cx.prop_string(node, p) {
                el.attr(format!("data-{p}").as_str(), v);
            }
        }
        if let Some(b) = &bind {
            el.attr("hidden", expr_val(Expr::Path(vec![b.clone()])));
        }
        let timed = ["until", "from", "countdown"].iter().any(|p| node.prop(p).is_some());
        if timed || remember.is_some() {
            let config = widget::config(cx, node, &["until", "from", "countdown"], &["remember"], &["bind"])?;
            widget::kit(cx);
            cx.global_js("ui-guide", SCRIPT);
            el.behavior("ui-banner", Json::Object(config));
        }
        cx.open(&el, node)?;
        if let Some(i) = node.prop("icon") {
            icon(cx, node, i, 18)?;
        }
        if node.prop("title").is_some() || node.prop("text").is_some() || node.prop("countdown").is_some() {
            cx.open(&Element::new("span"), node)?;
            if let Some(t) = node.prop("title") {
                let mut s = Element::new("span");
                s.attr("data-banner-title", true).text(t.clone());
                cx.open(&s, node)?;
                cx.close("span");
                cx.text(" ");
            }
            if let Some(t) = node.prop("text") {
                let mut s = Element::new("span");
                s.text(t.clone());
                cx.open(&s, node)?;
                cx.close("span");
            }
            if node.prop("countdown").is_some() {
                cx.text(" ");
                let mut s = Element::new("span");
                s.attr("data-banner-left", true);
                cx.open(&s, node)?;
                cx.close("span");
            }
            cx.close("span");
        }
        cx.children(node)?;
        if let Some(b) = bind {
            let close = synthetic(node, "button", Vec::new(), vec![("type", lit("button")), ("data-banner-close", lit(true)), ("aria-label", lit(cx.label("Close")))], vec![("click", vec![Action::SetState { key: b, value: Val::Lit(json!(true)) }])]);
            let e = cx.element("button", &close);
            cx.open(&e, &close)?;
            cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8"/></svg>"#);
            cx.close("button");
        }
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- session timeout

pub struct SessionTimeout;

const SESSION_PROPS: &[PropDef] = &[
    PropDef::new("after", PropKind::Number, "Minutes without anyone doing anything until the session ends").required(),
    PropDef::new("warn", PropKind::Number, "Minutes before the end that it asks (default 2)"),
    PropDef::new("expires", PropKind::Any, "Or the time it ends whatever's done, e.g. `{{ session.expires }}`"),
    PropDef::new("bindLeft", PropKind::StateKey, "State key that gets the seconds left while it's asking (null otherwise)"),
    PropDef::new("extendUrl", PropKind::Any, "Posted to when they stay; otherwise handle the `extend` event"),
    PropDef::new("signOutUrl", PropKind::Any, "Where Sign out, and the end, go; the `expire` event comes first"),
    PropDef::new("title", PropKind::Any, "Default `Are you still there?`"),
    PropDef::new("text", PropKind::Any, "Default `You'll be signed out to keep your account safe.`"),
    PropDef::new("stayLabel", PropKind::Any, "Default `Stay signed in`"),
    PropDef::new("signOutLabel", PropKind::Any, "Default `Sign out`"),
    PropDef::new("channel", PropKind::String, "Tabs of the same name count as one: being used in one keeps the others (default `session`)"),
];

static SESSION: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "contents")])
        .nest(
            "& dialog",
            pairs(&[("box-sizing", "border-box"), ("width", "min(400px, calc(100vw - 32px))"), ("padding", pad!(24)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", BG_DIALOG), ("color", TEXT), ("font-family", "var(--font-body, system-ui, sans-serif)"), ("font-size", fs!(14)), ("text-align", "center"), ("box-shadow", "var(--elevation-dialog, 0 16px 40px rgb(0 0 0 / 0.25))")]),
        )
        .nest("& dialog::backdrop", pairs(&[("background", "rgb(0 0 0 / 0.55)")]))
        .nest("& h2", pairs(&[("margin", "0 0 6px"), ("font-size", fs!(18)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& p", pairs(&[("margin", "0"), ("color", MUTED)]))
        .nest("& [data-left]", pairs(&[("margin", "14px 0"), ("font-size", fs!(34)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-left][data-soon]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-choices]", pairs(&[("display", "flex"), ("justify-content", "center"), ("flex-wrap", "wrap"), ("gap", gap!(8))]))
        .nest("& progress", pairs(&[("width", "100%"), ("height", "4px"), ("margin-bottom", "16px"), ("accent-color", "var(--color-primary)"), ("background", SUBTLE)]))
});

impl Component for SessionTimeout {
    fn name(&self) -> &'static str {
        "ui-session-timeout"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Asks whether someone's still there before their session ends, counting down, across the tabs they have open.", props: SESSION_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        for u in ["extendUrl", "signOutUrl"] {
            widget::check_url(cx, node, u)?;
        }
        let after = widget::number(cx, node, "after").unwrap_or(0.0);
        let warn = widget::number(cx, node, "warn").unwrap_or(2.0);
        if after <= 0.0 || warn <= 0.0 || warn >= after {
            return Err(RenderError::new(node, "`after` must be more than `warn`, and both more than 0 minutes"));
        }
        let mut config = widget::config(cx, node, &["expires", "extendUrl", "signOutUrl"], &["channel"], &["bindLeft"])?;
        config.insert("after".into(), json!(after));
        config.insert("warn".into(), json!(warn));
        widget::kit(cx);
        cx.global_js("ui-guide", SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&SESSION).behavior("ui-session-timeout", Json::Object(config));
        cx.open(&el, node)?;
        let title = node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Are you still there?")));
        let mut d = Element::new("dialog");
        d.attr("role", "alertdialog").attr("aria-label", title.clone());
        cx.open(&d, node)?;
        let mut h = Element::new("h2");
        h.text(title);
        cx.open(&h, node)?;
        cx.close("h2");
        let mut p = Element::new("p");
        p.text(node.prop("text").cloned().unwrap_or_else(|| lit(cx.label("You'll be signed out to keep your account safe."))));
        cx.open(&p, node)?;
        cx.close("p");
        let mut left = Element::new("div");
        left.attr("data-left", true).attr("role", "timer").attr("aria-live", "off");
        cx.open(&left, node)?;
        cx.close("div");
        let mut pr = Element::new("progress");
        pr.attr("max", "1").attr("value", "1").attr("aria-hidden", "true");
        cx.open(&pr, node)?;
        cx.close("progress");
        let mut row = Element::new("div");
        row.attr("data-choices", true);
        cx.open(&row, node)?;
        // Labels: "Sign out", "Stay signed in"
        for (prop, text, variant, attr) in [("signOutLabel", "Sign out", "outline", "data-out"), ("stayLabel", "Stay signed in", "default", "data-stay")] {
            let label = node.prop(prop).cloned().unwrap_or_else(|| lit(cx.label(text)));
            cx.render(&synthetic(node, "ui-button", vec![("label", label), ("variant", lit(variant))], vec![(attr, lit(true))], Vec::new()))?;
        }
        cx.close("div");
        cx.close("dialog");
        cx.close("div");
        Ok(())
    }
}
