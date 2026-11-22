//! `ui-push-opt-in`, `ui-app-badge`, `ui-share`, `ui-share-target`, `ui-contact-picker`,
//! `ui-passkey` and `ui-app-lock`. `share.js` runs them.

use crate::{
    BG_DIALOG, BORDER, MUTED, PRIMARY, SUBTLE, SURFACE, TEXT, lit,
    widget::{self, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("share.js");

fn scripts(cx: &mut RenderCx<'_>) {
    crate::pwa::offline(cx);
    cx.global_js("ui-share", SCRIPT);
}

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& [data-sh-menu]", pairs(&[("position", "relative"), ("display", "inline-block")]))
        .nest("& dialog[data-sh-list]", pairs(&[("padding", pad!(12))]))
        .nest("& [data-sh-list] button, & [data-sh-list] a", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(10)), ("box-sizing", "border-box"), ("width", "100%"), ("padding", pad!(11, 12)), ("border", "0"), ("border-radius", "var(--radius-md)"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("text-align", "start"), ("text-decoration", "none"), ("cursor", "pointer")]))
        .nest("& [data-sh-list] :is(button, a):hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-sh-thumb]", pairs(&[("flex", "none"), ("width", "40px"), ("height", "40px"), ("display", "grid"), ("place-items", "center"), ("object-fit", "cover"), ("border-radius", "var(--radius-md)"), ("background", SUBTLE), ("color", MUTED)]))
        .nest("& small", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& fieldset", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("margin", "0"), ("padding", "0"), ("border", "0")]))
        .nest("& legend", pairs(&[("padding", "0"), ("margin-bottom", gap!(4)), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& label[data-sh-check]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("cursor", "pointer")]))
        .nest("& label[data-sh-check] input", pairs(&[("width", "16px"), ("height", "16px"), ("margin", "0"), ("accent-color", PRIMARY)]))
        .nest("& label[data-w-btn] input", pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("opacity", "0")]))
        .nest("& label[data-w-btn]", pairs(&[("position", "relative")]))
        .nest("& label[data-w-btn]:focus-within", crate::forms::focus_ring())
        .nest("& [data-sh-fields]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fit, minmax(150px, 1fr))"), ("gap", gap!(8))]))
});

// ---------------------------------------------------------------- push

pub struct PushOptIn;

const PUSH_PROPS: &[PropDef] = &[
    PropDef::new("publicKey", PropKind::Any, "The server's VAPID public key; without it, only the app's own notifications are allowed for, with no pushes"),
    PropDef::new("saveUrl", PropKind::Any, "The subscription and topics are POSTed here as JSON, `{ subscription, topics }`, and again as they change; `subscription` is null once they're off"),
    PropDef::new("topics", PropKind::Any, "What can be chosen: `[{ value, label, description, on }]`"),
    PropDef::new("bind", PropKind::StateKey, "State key that gets `{ state, topics, endpoint }`; `state`: on, off, denied, install (an iPhone, until the app's on the home screen) or unsupported"),
    PropDef::new("title", PropKind::Any, "Default `Stay up to date`"),
    PropDef::new("text", PropKind::Any, "Why, before the browser asks"),
    PropDef::new("label", PropKind::Any, "The button (default `Turn on notifications`)"),
    PropDef::new("when", PropKind::Any, "Waits for a good moment, e.g. `{{ ordered }}`: shown once this is true"),
    PropDef::new("remember", PropKind::String, "A Not now button, which keeps it away for `days` in this browser, under this name"),
    PropDef::new("days", PropKind::Number, "Default 30"),
    PropDef::new("test", PropKind::Bool, "Once on, a button shows one"),
];

impl Component for PushOptIn {
    fn name(&self) -> &'static str {
        "ui-push-opt-in"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Offers notifications at a good moment, says why, then asks; subscribes to pushes and sends the subscription to a server, by topic. The app's service worker (`irender app`) shows them.", props: PUSH_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::check_url(cx, node, "saveUrl")?;
        let mut config = widget::config(cx, node, &["title", "text", "label", "when", "saveUrl", "publicKey"], &["topics", "remember", "days"], &["bind"])?;
        config.insert("gated".into(), json!(node.prop("when").is_some()));
        widget::flags(cx, node, &mut config, &[("test", false)]);
        scripts(cx);
        widget::drawn(cx, node, "section", &PANEL, "ui-push-opt-in", config, vec![("aria-label", lit(cx.label("Notifications")))])
    }
}

// ---------------------------------------------------------------- badge

pub struct AppBadge;

const BADGE_PROPS: &[PropDef] = &[
    PropDef::new("count", PropKind::Any, "The number; none at 0, and a dot for `true`; may use state").required(),
    PropDef::new("title", PropKind::Bool, "Before the page's title too, as `(3) Inbox` (default true)"),
];

impl Component for AppBadge {
    fn name(&self) -> &'static str {
        "ui-app-badge"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A number on the installed app's icon, and in the tab's title. Draws nothing.", props: BADGE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["count"], &[], &[])?;
        widget::flags(cx, node, &mut config, &[("title", true)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &HIDDEN, "ui-app-badge", config, vec![("hidden", lit(true))])
    }
}
static HIDDEN: LazyLock<StyleBlock> = LazyLock::new(StyleBlock::new);

// ---------------------------------------------------------------- share

pub struct Share;

const SHARE_PROPS: &[PropDef] = &[
    PropDef::new("url", PropKind::Any, "A link (default the page's when there's nothing else); may use state"),
    PropDef::new("title", PropKind::Any, "May use state"),
    PropDef::new("text", PropKind::Any, "May use state"),
    PropDef::new("files", PropKind::Any, "Files to share: `[{ url, name, type }]`, e.g. `ui-camera` photos; may use state"),
    PropDef::new("label", PropKind::Any, "Default `Share`"),
    PropDef::new("variant", PropKind::Enum(&["primary", "outline", "ghost"]), "Default `outline`"),
    PropDef::new("targets", PropKind::Any, "Besides copying and email, where there's no share sheet: `[{ label, href }]`, with `{url}`, `{title}` and `{text}` in `href`"),
    PropDef::new("sheet", PropKind::Enum(&["auto", "own"]), "`own` always shows the list, not the device's sheet (default `auto`)"),
];

impl Component for Share {
    fn name(&self) -> &'static str {
        "ui-share"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Shares a link, text or files with the device's own sheet; where there's none, a list: copy, email, download and others.", props: SHARE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["url", "title", "text", "files", "label"], &["variant", "targets", "sheet"], &[])?;
        scripts(cx);
        crate::sheets(cx);
        widget::drawn(cx, node, "div", &PANEL, "ui-share", config, vec![])
    }
}

// ---------------------------------------------------------------- share target

pub struct ShareTarget;

const TARGET_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets what was shared: `{ title, text, url, files: [{ name, type, size, url }], at }`").required(),
    PropDef::new("quiet", PropKind::Bool, "Draws nothing"),
    PropDef::new("acceptLabel", PropKind::Any, "Default `Add it`; runs `on: { accept: … }`"),
    PropDef::new("emptyText", PropKind::Any, "Shown while nothing has been"),
];

impl Component for ShareTarget {
    fn name(&self) -> &'static str {
        "ui-share-target"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Takes what other apps share with this one, once it's installed: put it on the page that the document's `app.share` names.", props: TARGET_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["acceptLabel", "emptyText"], &[], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("quiet", false)]);
        scripts(cx);
        widget::drawn(cx, node, "section", &PANEL, "ui-share-target", config, vec![("aria-label", lit(cx.label("Shared with this app")))])
    }
}

// ---------------------------------------------------------------- contacts

pub struct ContactPicker;

const CONTACT_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the contact, `{ name, tel: [], email: [] }`, or with `multiple` a list of them").required(),
    PropDef::new("multiple", PropKind::Bool, "Several"),
    PropDef::new("fields", PropKind::Any, "What's asked for: `[name, tel, email, address]` (default the first three)"),
    PropDef::new("manual", PropKind::Bool, "Can be typed (default true)"),
    PropDef::new("label", PropKind::Any, "Default `Choose a contact`"),
];

impl Component for ContactPicker {
    fn name(&self) -> &'static str {
        "ui-contact-picker"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Contacts picked from the phone's own list (Chrome on Android); anywhere else, from a contact card file (.vcf) or typed.", props: CONTACT_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["label"], &["fields"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("multiple", false), ("manual", true)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &PANEL, "ui-contact-picker", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Contacts")))])
    }
}

// ---------------------------------------------------------------- passkey

pub struct Passkey;

const PASSKEY_PROPS: &[PropDef] = &[
    PropDef::new("mode", PropKind::Enum(&["signin", "register"]), "Default `signin`"),
    PropDef::new("optionsUrl", PropKind::Any, "POSTed `body` for the server's options (WebAuthn's, as JSON)"),
    PropDef::new("verifyUrl", PropKind::Any, "POSTed the passkey's answer as JSON; what it replies goes to `bind`"),
    PropDef::new("body", PropKind::Any, "Sent for the options, e.g. `{ email: \"{{ email }}\" }`"),
    PropDef::new("bind", PropKind::StateKey, "State key that gets the server's reply"),
    PropDef::new("autofill", PropKind::Bool, "Offered among the browser's suggestions for a field with `autocomplete: username webauthn`"),
    PropDef::new("local", PropKind::Bool, "Without a server, for this device only: it shows someone can unlock the device, and proves nothing to a server"),
    PropDef::new("name", PropKind::Any, "With `local`, what it's called"),
    PropDef::new("label", PropKind::Any, "Default `Sign in with a passkey`, or `Create a passkey`"),
    PropDef::new("variant", PropKind::Enum(&["primary", "outline"]), "Default `outline`"),
];

impl Component for Passkey {
    fn name(&self) -> &'static str {
        "ui-passkey"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Signs in, or makes the passkey to, with a fingerprint, a face or the device's PIN. The server makes the challenge and checks the answer.", props: PASSKEY_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let local = widget::flag(cx, node, "local", false);
        for u in ["optionsUrl", "verifyUrl"] {
            widget::check_url(cx, node, u)?;
            if !local && node.prop(u).is_none() {
                return Err(RenderError::new(node, format!("`ui-passkey` needs `{u}`, or `local`")));
            }
        }
        let mut config = widget::config(cx, node, &["optionsUrl", "verifyUrl", "body", "label", "name"], &["mode", "variant"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("autofill", false), ("local", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &PANEL, "ui-passkey", config, vec![])
    }
}

// ---------------------------------------------------------------- lock

pub struct AppLock;

const LOCK_PROPS: &[PropDef] = &[
    PropDef::new("methods", PropKind::Any, "How it's opened: `[passkey, pin]` (default both, as the device can)"),
    PropDef::new("after", PropKind::Number, "Minutes without anything done until it locks (default 5; 0: never)"),
    PropDef::new("hidden", PropKind::Number, "Seconds out of sight until it locks (default 30; 0: at once)"),
    PropDef::new("digits", PropKind::Number, "Of the PIN, 4 to 8 (default 4)"),
    PropDef::new("tries", PropKind::Number, "Wrong PINs before it makes one wait, longer each time (default 5)"),
    PropDef::new("name", PropKind::String, "Kept under this name on the device (default `app`)"),
    PropDef::new("bindLocked", PropKind::StateKey, "State key that's true while it's locked, e.g. to take things off the page"),
    PropDef::new("title", PropKind::Any, "Default `Locked`"),
    PropDef::new("logo", PropKind::Any, "A picture's address, over the title"),
    PropDef::new("signOutUrl", PropKind::Any, "Where `Forgot your PIN?` goes, after the `forgot` event; it stays locked until `reset`"),
    PropDef::new("reset", PropKind::Any, "Takes the lock off, e.g. once they've signed in again: `{{ signedInNow }}`"),
    PropDef::new("settings", PropKind::Bool, "Draws turning it on and off, changing the PIN and locking now (default true)"),
];

static LOCK: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& [data-lk-panel]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10))]))
        .nest("& small", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest(
            "& dialog[data-lk]",
            pairs(&[("display", "none"), ("width", "100vw"), ("max-width", "none"), ("height", "100dvh"), ("max-height", "none"), ("margin", "0"), ("padding", "24px"), ("border", "0"), ("border-radius", "0"), ("background", BG_DIALOG)]),
        )
        .nest("& dialog[data-lk][open]", pairs(&[("display", "grid"), ("place-items", "center")]))
        .nest("& dialog[data-lk]::backdrop", pairs(&[("background", BG_DIALOG)]))
        .nest("& [data-lk-in]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(14)), ("width", "min(300px, 100%)"), ("text-align", "center")]))
        .nest("& [data-lk-in] h2", pairs(&[("margin", "0")]))
        .nest("& [data-lk-in] img", pairs(&[("width", "64px"), ("height", "64px"), ("border-radius", "14px")]))
        .nest("& [data-lk-dots]", pairs(&[("display", "flex"), ("gap", "14px"), ("min-height", "16px")]))
        .nest("& [data-lk-dots] i", pairs(&[("width", "14px"), ("height", "14px"), ("box-sizing", "border-box"), ("border", "2px solid var(--color-muted)"), ("border-radius", "50%")]))
        .nest("& [data-lk-dots] i[data-on]", pairs(&[("border-color", PRIMARY), ("background", PRIMARY)]))
        .nest("& [data-lk-keys]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(3, 68px)"), ("gap", "12px"), ("direction", "ltr")]))
        .nest("& [data-lk-keys] button", pairs(&[("width", "68px"), ("height", "68px"), ("padding", "0"), ("border", "1px solid"), ("border-color", BORDER), ("border-radius", "50%"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-size", "24px"), ("cursor", "pointer"), ("display", "grid"), ("place-items", "center"), ("touch-action", "manipulation")]))
        .nest("& [data-lk-keys] button:is(:hover, :active)", pairs(&[("background", SUBTLE)]))
        .nest("& [data-lk-keys] button:disabled", pairs(&[("opacity", "0.5")]))
        .nest("& [data-lk-keys] button[data-plain]", pairs(&[("border-color", "transparent"), ("background", "none")]))
        .nest("& [data-lk-in][data-wrong] [data-lk-dots]", pairs(&[("animation", "ui-shake 0.35s")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-lk-dots]", pairs(&[("animation", "none !important")])))
});
const SHAKE: &str = "@keyframes ui-shake { 20%, 60% { transform: translateX(-8px); } 40%, 80% { transform: translateX(8px); } }";

impl Component for AppLock {
    fn name(&self) -> &'static str {
        "ui-app-lock"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Covers the app after a while, or once it's been out of sight, until it's opened with a passkey or a PIN. It keeps someone who picks up the device out; it doesn't encrypt anything, and a server still has to check who's asking.", props: LOCK_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::check_url(cx, node, "signOutUrl")?;
        let digits = widget::number(cx, node, "digits").unwrap_or(4.0);
        if !(4.0..=8.0).contains(&digits) {
            return Err(RenderError::new(node, "`digits` is from 4 to 8"));
        }
        let mut config = widget::config(cx, node, &["title", "logo", "signOutUrl", "reset"], &["methods", "after", "hidden", "tries", "name"], &["bindLocked"])?;
        if let Some(m) = config.get("methods")
            && !m.as_array().is_some_and(|m| !m.is_empty() && m.iter().all(|x| x == "passkey" || x == "pin"))
        {
            return Err(RenderError::new(node, "`methods` is a list of `passkey` and `pin`"));
        }
        config.insert("digits".into(), Json::from(digits as u64));
        widget::flags(cx, node, &mut config, &[("settings", true)]);
        scripts(cx);
        cx.global_css("ui-shake", SHAKE);
        widget::drawn(cx, node, "section", &LOCK, "ui-app-lock", config, vec![("aria-label", lit(cx.label("App lock")))])
    }
}
