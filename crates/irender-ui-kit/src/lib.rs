//! A component kit for iRender, styled after shadcn/ui.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_ui_kit::register(&mut engine.registry);
//!
//! let mut doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-button",
//!     "props": { "label": "Save", "variant": "outline" } } }"#, Format::Json)?;
//! irender_ui_kit::apply_theme(&mut doc); // optional: the kit's neutral palette
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Every component is named `ui-…` so it never collides with the built-ins or your
//! own components. Like the built-ins, each only sets defaults: a document can
//! still override any attribute through `attrs`, any style through `style`, and
//! inner elements through `parts`.
//!
//! Colours come from theme tokens. The ones the default theme lacks
//! (`color.secondary`, `color.accent`, `color.destructive`, …) fall back to values
//! derived from `color.text` and `color.surface`, so the kit looks right with any
//! theme, dark mode included. [`apply_theme`] adds the kit's own palette.

/// A length that follows the theme's `space.scale` (1 unless a theme option sets
/// it) times `$s`, a further factor: `scaled!(""; 4, 12)` is
/// `calc(4px * var(--space-scale, 1)) calc(12px * …)`. `0` stays `0`.
macro_rules! scaled {
    ($s:literal; 0) => { "0" };
    ($s:literal; 0, $($rest:literal),+) => { concat!("0 ", scaled!($s; $($rest),+)) };
    ($s:literal; $n:literal) => { concat!("calc(", $n, "px * var(--space-scale, 1)", $s, ")") };
    ($s:literal; $n:literal, $($rest:literal),+) => { concat!(scaled!($s; $n), " ", scaled!($s; $($rest),+)) };
}

/// A spacing value that follows the density (`space.scale`) alone: `sp!(4, 12)`.
/// Layout sizes use it, so a density option makes every component more compact or
/// roomier. The finer options scale one kind of spacing further:
macro_rules! sp {
    ($($t:tt)*) => { scaled!(""; $($t)*) };
}

/// Paddings, and offsets inside a component's padding: also `space.inset`.
macro_rules! pad {
    ($($t:tt)*) => { scaled!(" * var(--space-inset, 1)"; $($t)*) };
}

/// Gaps and margins between things: also `space.gap`.
macro_rules! gap {
    ($($t:tt)*) => { scaled!(" * var(--space-gap, 1)"; $($t)*) };
}

/// The heights and widths of controls, rows and cells: also `size.control`.
macro_rules! ctl {
    ($($t:tt)*) => { scaled!(" * var(--size-control, 1)"; $($t)*) };
}

/// A font size that follows the theme's `font.scale`: `fs!(14)`.
macro_rules! fs {
    ($n:literal) => { concat!("calc(", $n, "px * var(--font-scale, 1))") };
}

mod agent;
mod calendar;
mod color;
mod gauge;
mod sortable;
mod form_builder;
mod countdown;
mod camera;
mod carousel;
mod chat;
mod code;
mod codes;
mod combobox;
mod comments;
mod command;
mod dashboard_grid;
mod data_table;
mod devtools;
mod disclosure;
mod display;
mod events;
mod feedback;
mod filters;
mod form;
mod form_layout;
mod forms;
mod geo;
mod guide;
mod kanban;
mod lightbox;
mod hardware;
mod illustration;
mod import;
mod inputs;
mod media;
mod menus;
mod navigation;
mod notifications;
mod number;
mod otp;
mod rail;
mod rating;
mod record;
mod repeater;
mod resizable;
mod overlay;
mod pickers;
mod pivot;
mod pwa;
mod scheduler;
mod scroll_spy;
mod share;
mod sidebar;
mod stat;
mod status;
mod stepper;
mod theme;
mod tree;
mod upload;
mod widget;

pub use chat::{render_message_ui, render_ui, ui_document};
pub use pwa::service_worker;
pub use theme::{apply_theme, theme};

use indexmap::IndexMap;
use irender_core::{
    Action, Node, Part, Template, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{ClientList, Element, RenderCx, RenderError, Registry};
use serde_json::{Value as Json, json};

/// Adds every kit component to `registry`, and the script that applies the colour
/// scheme chosen with `ui-theme-toggle` to every page it renders.
pub fn register(registry: &mut Registry) {
    registry
        .register(forms::Button)
        .register(camera::Camera)
        .register(camera::PermissionManager)
        .register(camera::PhotoAnnotate)
        .register(geo::Location)
        .register(geo::FieldMap)
        .register(geo::GeoCheckin)
        .register(geo::RouteTracker)
        .register(geo::Compass)
        .register(geo::Level)
        .register(share::PushOptIn)
        .register(share::AppBadge)
        .register(share::Share)
        .register(share::ShareTarget)
        .register(share::ContactPicker)
        .register(share::Passkey)
        .register(share::AppLock)
        .register(record::AudioRecorder)
        .register(record::VideoRecorder)
        .register(record::ScreenCapture)
        .register(record::VoiceInput)
        .register(record::ReadAloud)
        .register(record::WakeLock)
        .register(record::SwipeActions)
        .register(record::DocumentScanner)
        .register(hardware::Nfc)
        .register(hardware::Bluetooth)
        .register(hardware::Serial)
        .register(hardware::Usb)
        .register(hardware::FileHandler)
        .register(pwa::InstallPrompt)
        .register(pwa::UpdatePrompt)
        .register(pwa::OfflineIndicator)
        .register(pwa::OfflineQueue)
        .register(pwa::LocalStore)
        .register(pwa::AppShell)
        .register(pwa::AppScreen)
        .register(pwa::AppActions)
        .register(display::Badge)
        .register(display::Card)
        .register(display::CardFooter)
        .register(display::Alert)
        .register(display::Avatar)
        .register(display::Separator)
        .register(display::Skeleton)
        .register(display::Kbd)
        .register(display::Progress)
        .register(display::AspectRatio)
        .register(display::Table)
        .register(display::Heading)
        .register(display::Text)
        .register(display::Spinner)
        .register(display::Empty)
        .register(illustration::Illustration)
        .register(display::ScrollArea)
        .register(forms::Label)
        .register(forms::Input)
        .register(forms::Textarea)
        .register(forms::Select)
        .register(forms::Checkbox)
        .register(forms::Switch)
        .register(forms::RadioGroup)
        .register(navigation::Breadcrumb)
        .register(navigation::Pagination)
        .register(forms::Slider)
        .register(otp::InputOtp)
        .register(scroll_spy::ScrollSpy)
        .register(filters::Filters)
        .register(filters::Filter)
        .register(tree::Tree)
        .register(upload::FileUpload)
        .register(status::StatusBadge)
        .register(status::ProgressRing)
        .register(gauge::Gauge)
        .register(sortable::SortableList)
        .register(form_builder::FormBuilder)
        .register(countdown::Countdown)
        .register(status::Timeline)
        .register(status::TimelineItem)
        .register(lightbox::Lightbox)
        .register(pickers::QueryBuilder)
        .register(pickers::TransferList)
        .register(pickers::TreeSelect)
        .register(agent::Attachment)
        .register(agent::Sources)
        .register(agent::ToolCall)
        .register(agent::Trace)
        .register(agent::Questionnaire)
        .register(guide::Tour)
        .register(guide::Banner)
        .register(guide::SessionTimeout)
        .register(media::Signature)
        .register(media::ImageCropper)
        .register(media::Plan)
        .register(media::Pdf)
        .register(devtools::Diff)
        .register(devtools::Log)
        .register(devtools::CodeEditor)
        .register(codes::QrCode)
        .register(codes::Barcode)
        .register(codes::CodeScanner)
        .register(pivot::Pivot)
        .register(scheduler::Scheduler)
        .register(comments::Comments)
        .register(comments::MentionInput)
        .register(inputs::TagsInput)
        .register(inputs::MaskedInput)
        .register(inputs::PhoneInput)
        .register(inputs::TimePicker)
        .register(stat::Stat)
        .register(stat::DescriptionList)
        .register(stat::DescriptionItem)
        .register(stat::ItemGroup)
        .register(stat::Item)
        .register(import::Import)
        .register(form::Form)
        .register(form::ErrorSummary)
        .register(form::SaveBar)
        .register(form_layout::FormSection)
        .register(repeater::Repeater)
        .register(stepper::Stepper)
        .register(stepper::Step)
        .register(rating::Rating)
        .register(rating::PasswordChecklist)
        .register(dashboard_grid::DashboardGrid)
        .register(dashboard_grid::Tile)
        .register(kanban::Kanban)
        .register(events::EventCalendar)
        .register(number::NumberInput)
        .register(notifications::Notifications)
        .register(rail::RailLayout)
        .register(rail::RailPanel)
        .register(resizable::Resizable)
        .register(resizable::ResizablePanel)
        .register(code::CodeBlock)
        .register(code::JsonEditor)
        .register(forms::ButtonGroup)
        .register(forms::InputGroup)
        .register(forms::Field)
        .register(disclosure::Accordion)
        .register(disclosure::AccordionItem)
        .register(disclosure::Collapsible)
        .register(disclosure::Tabs)
        .register(disclosure::TabPanel)
        .register(disclosure::Toggle)
        .register(disclosure::ToggleGroup)
        .register(disclosure::ThemeToggle)
        .register(disclosure::ThemeOption)
        .register(disclosure::ThemePanel)
        .register(disclosure::ThemeColors)
        .register(overlay::Dialog::DIALOG)
        .register(overlay::Dialog::ALERT)
        .register(overlay::Dialog::SHEET)
        .register(overlay::Dialog::DRAWER)
        .register(overlay::DialogClose)
        .register(overlay::DialogFooter)
        .register(overlay::Popover::POPOVER)
        .register(overlay::Popover::MENU)
        .register(overlay::SplitButton)
        .register(overlay::ContextMenu)
        .register(menus::MenuSub)
        .register(menus::MenuCheckbox)
        .register(menus::MenuRadioGroup)
        .register(menus::Menubar)
        .register(menus::MenubarMenu)
        .register(menus::NavigationMenu)
        .register(menus::NavigationItem)
        .register(menus::NavigationLink)
        .register(overlay::MenuItem)
        .register(overlay::MenuSeparator)
        .register(overlay::MenuLabel)
        .register(overlay::Tooltip)
        .register(overlay::HoverCard)
        .register(feedback::Toaster)
        .register(sidebar::SidebarLayout)
        .register(sidebar::Sidebar)
        .register(sidebar::SidebarHeader)
        .register(sidebar::SidebarContent)
        .register(sidebar::SidebarFooter)
        .register(sidebar::SidebarGroup)
        .register(sidebar::SidebarItem)
        .register(sidebar::SidebarAction)
        .register(sidebar::SidebarTrigger)
        .register(combobox::Combobox)
        .register(command::Command)
        .register(command::CommandDialog)
        .register(command::CommandGroup)
        .register(command::CommandItem)
        .register(command::CommandPage)
        .register(command::CommandSeparator)
        .register(chat::Conversation)
        .register(chat::Message)
        .register(chat::TypingIndicator)
        .register(chat::ChatInput)
        .register(carousel::Carousel)
        .register(calendar::Calendar)
        .register(calendar::DatePicker)
        .register(color::ColorPicker)
        .register(data_table::DataTable)
        .register(data_table::Child::COLUMN)
        .register(data_table::Child::TOOLBAR)
        .register(data_table::Child::BULK)
        .register(data_table::Child::DETAIL)
        // A Light or Dark choice made with `ui-theme-toggle` holds on every page, with a
        // toggle on it or not.
        .early_js("ui-theme", disclosure::THEME_EARLY_SCRIPT)
        .global_css("ui-body", BODY);
}

/// The page's text: the theme's font, and its line height and letter spacing once a
/// theme option sets them (until then `var()` without a fallback leaves the page's
/// own). Also on an element that picks one of them for its part of the page, as an
/// inherited value isn't worked out again below the body. No specificity, so a
/// document's styles always win.
const BODY: &str = ":where(body, [data-theme-font], [data-theme-line-height], [data-theme-letter-spacing]) { font-family: var(--font-body, system-ui, sans-serif); line-height: var(--font-leading); letter-spacing: var(--font-tracking); }";

// Colours. Tokens the default theme has are used directly; the others fall back
// to mixes of text and surface so they follow light and dark mode on their own.
const PRIMARY: &str = "var(--color-primary)";
const ON_PRIMARY: &str = "var(--color-on-primary)";
const SURFACE: &str = "var(--color-surface)";
const TEXT: &str = "var(--color-text)";
const MUTED: &str = "var(--color-muted)";
const BORDER: &str = "var(--color-border)";
const SECONDARY: &str = "var(--color-secondary, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const ON_SECONDARY: &str = "var(--color-on-secondary, var(--color-text))";
const ACCENT: &str = "var(--color-accent, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const SUBTLE: &str = "var(--color-subtle, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const ON_BRAND_2: &str = "var(--color-on-brand-2, var(--color-on-primary))";
// Fills in a brand colour (buttons, badges, progress, the chat's own messages): the
// theme's `gradient.primary` / `gradient.brand-2` when it has one, else the colour.
// The second brand colour is for the `brand` variants; the primary when a theme has none.
const PRIMARY_FILL: &str = "var(--gradient-primary, var(--color-primary))";
const PRIMARY_FILL_HOVER: &str = "var(--gradient-primary-hover, var(--color-primary-hover))";
const BRAND_2_FILL: &str = "var(--gradient-brand-2, var(--color-brand-2, var(--gradient-primary, var(--color-primary))))";
// Backgrounds, each the surface colour unless the theme sets its own. A card, popover,
// dialog or sheet also sets `--ui-bg` to its background, so what's drawn inside it
// (pinned table columns, tints) can match whatever it sits on.
const BG_PAGE: &str = "var(--color-background, var(--color-surface))";
const BG_CARD: &str = "var(--color-card, var(--color-surface))";
const BG_POPOVER: &str = "var(--color-popover, var(--color-surface))";
const BG_DIALOG: &str = "var(--color-dialog, var(--color-surface))";
const BG_SHEET: &str = "var(--color-sheet, var(--color-dialog, var(--color-surface)))";
/// The background an element sits on: the nearest card's, popover's, dialog's or
/// sheet's, otherwise the page's.
const BG_UNDER: &str = "var(--ui-bg, var(--color-background, var(--color-surface)))";
const DESTRUCTIVE: &str = "var(--color-destructive, #dc2626)";
const ON_DESTRUCTIVE: &str = "var(--color-on-destructive, #ffffff)";
/// The message variants alerts and toasts share: name, colour and icon (the inside of
/// a 24×24 stroked SVG, drawn by `notice_icon`).
const NOTICES: &[(&str, &str, &str)] = &[
    ("info", "var(--color-info, #2563eb)", r#"<circle cx="12" cy="12" r="9"/><path d="M12 16v-4.5M12 8h.01"/>"#),
    ("success", "var(--color-success, #16a34a)", r#"<circle cx="12" cy="12" r="9"/><path d="m8.5 12.5 2.5 2.5 4.5-5"/>"#),
    (
        "warning",
        "var(--color-warning, #d97706)",
        r#"<path d="M10.3 4.2 2.8 17.5A2 2 0 0 0 4.5 20.5h15a2 2 0 0 0 1.7-3L13.7 4.2a2 2 0 0 0-3.4 0Z"/><path d="M12 9.5v4M12 17h.01"/>"#,
    ),
    ("destructive", DESTRUCTIVE, r#"<circle cx="12" cy="12" r="9"/><path d="M12 7.5v5M12 16h.01"/>"#),
];

/// One of `NOTICES`' icons as an SVG, `attrs` going on the `<svg>`.
fn notice_icon(paths: &str, attrs: &str) -> String {
    format!(
        r#"<svg{attrs} width="18" height="18" style="width:calc(18px * var(--size-icon, 1));height:calc(18px * var(--size-icon, 1))" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{paths}</svg>"#
    )
}

const RING: &str = "var(--color-ring, var(--color-muted))";
const MONO: &str = "ui-monospace, SFMono-Regular, Menlo, monospace";

/// One turn, for spinners; added to the page once by whichever needs it first.
const SPIN: &str = "@keyframes ui-spin {\n  to { transform: rotate(1turn); }\n}";

/// The index of the prop's value in `options`, or 0 (the default) when it's
/// missing or unknown.
fn choice(cx: &RenderCx<'_>, node: &Node, prop: &str, options: &[&str]) -> usize {
    cx.prop_string(node, prop).and_then(|v| options.iter().position(|o| *o == v)).unwrap_or(0)
}

/// An array prop that the server expands into markup. It can't follow state, so
/// a prop that reads state is an error rather than silently frozen.
fn server_array(cx: &RenderCx<'_>, node: &Node, prop: &str) -> Result<Vec<Json>, RenderError> {
    if node.prop(prop).is_some_and(|v| cx.is_reactive(v)) {
        return Err(RenderError::new(node, format!("`{prop}` can't use state; it is expanded on the server")));
    }
    match cx.prop(node, prop) {
        Some(Json::Array(items)) => Ok(items),
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(_) => Err(RenderError::new(node, format!("`{prop}` must evaluate to an array"))),
    }
}

/// A field of an object item, as text.
fn field(item: &Json, key: &str) -> Option<String> {
    item.get(key).filter(|v| !v.is_null()).map(expr::stringify)
}

fn lit(v: impl Into<Json>) -> Val {
    Val::Lit(v.into())
}

/// Right to left: icons that point along the line (previous and next, a submenu's
/// chevron, a panel's side) carry `data-flip` and point the other way (`scale` rather
/// than `transform`, so an icon's own rotation still applies), and `--ui-dir` is -1, for
/// offsets along the line (`translateX(calc(16px * var(--ui-dir, 1)))`).
const RTL: &str = ":dir(rtl) { --ui-dir: -1; }\nsvg[data-flip]:dir(rtl) { scale: -1 1; }";

/// Dragging by touch, for components that drag with the browser's drag and drop
/// (`window.__irenderTouchDrag`). Added before the component's own script.
fn touch_drag(cx: &mut RenderCx<'_>) {
    cx.global_js("ui-touch-drag", include_str!("touch_drag.js"));
}

/// Anchor positioning for browsers without it, for components that place popovers
/// beside an anchor (`anchors.js`; it does nothing where the browser does it).
fn anchors(cx: &mut RenderCx<'_>) {
    cx.global_js("ui-anchors", include_str!("anchors.js"));
}

/// What typed cells take and how they're checked (`window.__irenderCells`), for
/// components whose cells or fields are read from text. Added before their own script.
fn cells(cx: &mut RenderCx<'_>) {
    cx.global_js("ui-cells", include_str!("cells.js"));
}

/// Reading CSV and .xlsx, and writing .xlsx and zips (`window.__irenderSheets`), without
/// libraries. Added before the script that uses it.
fn sheets(cx: &mut RenderCx<'_>) {
    cx.global_js("ui-sheets", include_str!("sheets.js"));
}

/// Adds the right-to-left rules, for components that use them.
fn flips(cx: &mut RenderCx<'_>) {
    cx.global_css("ui-rtl", RTL);
}

/// `position-area` with left and right swapped.
fn mirrored_area(area: &str) -> String {
    area.split(' ')
        .map(|w| match w {
            "left" => "right",
            "right" => "left",
            "span-left" => "span-right",
            "span-right" => "span-left",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `block` placed at `area` (a `position-area`), mirrored on a right-to-left page.
fn with_area(block: irender_core::StyleBlock, area: &str) -> irender_core::StyleBlock {
    block.set("position-area", area.to_string()).nest("&:dir(rtl)", irender_core::StyleBlock::new().set("position-area", mirrored_area(area)))
}

/// The `locale` for a component's numbers and dates: its own, else the document's `lang`
/// (else the browser's, decided there).
fn locale(cx: &RenderCx<'_>, node: &Node) -> Option<String> {
    cx.prop_string(node, "locale").filter(|l| !l.is_empty()).or_else(|| cx.labels().lang.clone())
}

/// `instance_id` for a component whose own element carries its id (a dialog, a popover,
/// a menu): `attrs: { id }` is on that element, so it's the id the component's buttons
/// and labels point at too.
fn own_id(cx: &RenderCx<'_>, node: &Node, prefix: &str) -> Val {
    match (node.prop("id"), node.attrs.get("id")) {
        (None, Some(id)) => id.clone(),
        _ => instance_id(cx, node, prefix),
    }
}

/// The element id for this instance of a component: the `id` prop (or `attrs.id`) if
/// given (templates allowed, e.g. `confirm-{{ item.id }}` inside a list), otherwise one
/// derived from the node's place in the document and the list item it is in.
fn instance_id(cx: &RenderCx<'_>, node: &Node, prefix: &str) -> Val {
    if let Some(id) = node.prop("id") {
        return id.clone();
    }
    let path: String = node.path.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let key = cx.scope_key();
    let id = if key.is_empty() { format!("{prefix}-{path}") } else { format!("{prefix}-{path}-{key}") };
    let id = id.replace("--", "-").trim_end_matches('-').to_string();
    // Items of a list the browser rebuilds share a scope key; their index tells them apart.
    match cx.client_key() {
        Some(mut t) => {
            t.parts.insert(0, Part::Lit(format!("{id}-")));
            Val::Tpl(t)
        }
        None => lit(id),
    }
}

/// `id` followed by `suffix`, as a value that follows a templated id.
fn id_with(id: &Val, suffix: &str) -> Val {
    match id {
        Val::Lit(v) => lit(format!("{}{suffix}", expr::stringify(v))),
        Val::Tpl(t) => {
            let mut t = t.clone();
            t.parts.push(Part::Lit(suffix.into()));
            Val::Tpl(t)
        }
    }
}

/// `prefix` followed by `id`, e.g. a CSS anchor name.
fn id_after(prefix: &str, id: &Val) -> Val {
    match id {
        Val::Lit(v) => lit(format!("{prefix}{}", expr::stringify(v))),
        Val::Tpl(t) => {
            let mut t = t.clone();
            t.parts.insert(0, Part::Lit(prefix.into()));
            Val::Tpl(t)
        }
    }
}

fn expr_val(e: Expr) -> Val {
    Val::Tpl(Template { parts: vec![Part::Expr(e)] })
}

fn state(key: &str) -> Expr {
    Expr::Path(vec![key.to_string()])
}

/// `{{ key == value }}`: live in the browser because it reads state.
fn state_eq(key: &str, value: Json) -> Val {
    expr_val(Expr::Binary(BinOp::Eq, Box::new(state(key)), Box::new(Expr::Lit(value))))
}

/// `{{ !key }}`
fn state_not(key: &str) -> Val {
    expr_val(Expr::Unary(UnOp::Not, Box::new(state(key))))
}

/// A node the kit renders itself, carrying its own attributes and actions. The
/// path stays the original node's so errors point at the document.
fn synthetic(node: &Node, kind: &str, props: Vec<(&str, Val)>, attrs: Vec<(&str, Val)>, on: Vec<(&str, Vec<Action>)>) -> Node {
    Node {
        kind: kind.into(),
        path: node.path.clone(),
        props: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        class: Vec::new(),
        style: Default::default(),
        parts: IndexMap::new(),
        on: on.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        children: Vec::new(),
    }
}

/// The value a parent component passed to its children (the dialog a close button
/// belongs to, the accordion an item is in).
fn context(cx: &RenderCx<'_>, name: &str) -> Option<Val> {
    cx.context(name).cloned()
}

/// The kit's and the built-ins' form controls, which a `ui-field` labels and describes.
const CONTROLS: &[&str] = &[
    "input", "textarea", "select", "ui-input", "ui-textarea", "ui-select", "ui-checkbox", "ui-switch", "ui-slider",
    "ui-radio-group", "ui-combobox", "ui-date-picker", "ui-input-otp", "ui-number-input", "ui-json-editor",
    "ui-tags-input", "ui-masked-input", "ui-phone-input", "ui-time-picker", "ui-mention-input", "ui-tree-select", "ui-code-editor",
    "ui-color-picker",
];

/// The `attrs` that reach the element a control focuses: the inner `<input>` for kit
/// controls that wrap one, otherwise the node's own.
fn focus_attrs(node: &mut Node) -> &mut IndexMap<String, Val> {
    match node.kind.as_str() {
        "ui-checkbox" | "ui-switch" => &mut node.parts.entry("control".into()).or_default().attrs,
        "ui-combobox" | "ui-input-otp" | "ui-number-input" | "ui-tags-input" | "ui-masked-input" | "ui-phone-input" | "ui-time-picker" | "ui-mention-input" => &mut node.parts.entry("input".into()).or_default().attrs,
        "ui-tree-select" => &mut node.parts.entry("trigger".into()).or_default().attrs,
        "ui-json-editor" | "ui-code-editor" => &mut node.parts.entry("textarea".into()).or_default().attrs,
        // With `input`, the text field; otherwise the button that opens the calendar.
        "ui-date-picker" if typed_date(node) => &mut node.parts.entry("input".into()).or_default().attrs,
        "ui-date-picker" => &mut node.parts.entry("trigger".into()).or_default().attrs,
        // Inline, its text field; otherwise the button that opens it.
        "ui-color-picker" if inline_color(node) => &mut node.parts.entry("input".into()).or_default().attrs,
        "ui-color-picker" => &mut node.parts.entry("trigger".into()).or_default().attrs,
        _ => &mut node.attrs,
    }
}

fn focus_attr<'a>(node: &'a Node, name: &str) -> Option<&'a Val> {
    match node.kind.as_str() {
        "ui-checkbox" | "ui-switch" => node.parts.get("control")?.attrs.get(name),
        "ui-combobox" | "ui-input-otp" | "ui-number-input" | "ui-tags-input" | "ui-masked-input" | "ui-phone-input" | "ui-time-picker" | "ui-mention-input" => node.parts.get("input")?.attrs.get(name),
        "ui-tree-select" => node.parts.get("trigger")?.attrs.get(name),
        "ui-json-editor" | "ui-code-editor" => node.parts.get("textarea")?.attrs.get(name),
        "ui-date-picker" if typed_date(node) => node.parts.get("input")?.attrs.get(name),
        "ui-date-picker" => node.parts.get("trigger")?.attrs.get(name),
        "ui-color-picker" if inline_color(node) => node.parts.get("input")?.attrs.get(name),
        "ui-color-picker" => node.parts.get("trigger")?.attrs.get(name),
        _ => node.attrs.get(name),
    }
}

/// A colour picker shown on the page, without its button.
fn inline_color(node: &Node) -> bool {
    matches!(node.prop("inline"), Some(Val::Lit(Json::Bool(true))))
}

/// A date picker with a text field to type in.
fn typed_date(node: &Node) -> bool {
    matches!(node.prop("input"), Some(Val::Lit(Json::Bool(true))))
}

/// Adds `ids` to the `aria-describedby` of the element `node` focuses, after any it has.
fn describe(node: &mut Node, ids: &[Val]) {
    let attrs = focus_attrs(node);
    let mut all: Vec<Val> = attrs.get("aria-describedby").cloned().into_iter().collect();
    all.extend_from_slice(ids);
    attrs.insert("aria-describedby".into(), join(&all));
}

/// Values joined with spaces: a template if any of them is one.
fn join(vals: &[Val]) -> Val {
    let mut parts = Vec::new();
    for (i, v) in vals.iter().enumerate() {
        if i > 0 {
            parts.push(Part::Lit(" ".into()));
        }
        match v {
            Val::Lit(j) => parts.push(Part::Lit(expr::stringify(j))),
            Val::Tpl(t) => parts.extend(t.parts.iter().cloned()),
        }
    }
    if parts.iter().all(|p| matches!(p, Part::Lit(_))) {
        return lit(parts.into_iter().map(|p| if let Part::Lit(s) = p { s } else { unreachable!() }).collect::<String>());
    }
    Val::Tpl(Template { parts })
}

/// The array prop `prop` as items to render: its value now, and the prop itself when
/// it uses state, so the browser can rebuild the items as it changes.
fn items<'n>(cx: &RenderCx<'_>, node: &'n Node, prop: &str) -> Result<(Vec<Json>, Option<&'n Val>), RenderError> {
    let values = match cx.prop(node, prop) {
        Some(Json::Array(items)) => items,
        None | Some(Json::Null) => Vec::new(),
        Some(_) => return Err(RenderError::new(node, format!("`{prop}` must evaluate to an array"))),
    };
    Ok((values, node.prop(prop).filter(|v| cx.is_reactive(v))))
}

/// Opens `el` and renders `render` once per item with the item and its index in
/// scope as `item` and `index`. With `live`, the browser rebuilds the items whenever
/// it changes, reusing those whose `key` it has seen. The caller closes `el`.
#[allow(clippy::too_many_arguments)]
fn each_item<'n>(
    cx: &mut RenderCx<'_>,
    node: &'n Node,
    mut el: Element<'n>,
    (values, live): (Vec<Json>, Option<&'n Val>),
    item: &str,
    index: &str,
    key: &Val,
    render: impl Fn(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    if let Some(items) = live {
        let key = cx.item_val_js(key, item, index);
        el.list(ClientList { items, item: item.into(), index: index.into(), key: Some(key) });
    }
    cx.open(&el, node)?;
    if live.is_some() {
        cx.item_template_with(item, index, &render)?;
    }
    for (i, value) in values.into_iter().enumerate() {
        cx.with_scope(vec![(item.into(), value), (index.into(), json!(i))], &render)?;
    }
    Ok(())
}

/// Renders `f` while `when` holds: decided on the server, or by the browser when
/// `when` uses state (or an item of a list the browser rebuilds).
fn branch(
    cx: &mut RenderCx<'_>,
    when: &Val,
    f: impl FnOnce(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    if cx.is_reactive(when) {
        cx.conditional_with(when, f)
    } else if expr::truthy(&cx.eval(when)) {
        f(cx)
    } else {
        Ok(())
    }
}

/// A value as an expression: a literal, a template's one expression, or the parts of
/// a longer template joined as text.
fn val_expr(v: &Val) -> Expr {
    match v {
        Val::Lit(j) => Expr::Lit(j.clone()),
        Val::Tpl(t) => match t.parts.as_slice() {
            [Part::Expr(e)] => e.clone(),
            parts => parts.iter().fold(Expr::Lit(json!("")), |text, p| {
                let p = match p {
                    Part::Lit(s) => Expr::Lit(json!(s)),
                    Part::Expr(e) => e.clone(),
                };
                bin(BinOp::Add, text, p)
            }),
        },
    }
}

fn bin(op: BinOp, a: Expr, b: Expr) -> Expr {
    Expr::Binary(op, Box::new(a), Box::new(b))
}

fn cond(c: Expr, a: Expr, b: Expr) -> Expr {
    Expr::Cond(Box::new(c), Box::new(a), Box::new(b))
}

fn path(segments: &[&str]) -> Expr {
    Expr::Path(segments.iter().map(|s| s.to_string()).collect())
}

fn num(n: impl Into<Json>) -> Expr {
    Expr::Lit(n.into())
}

/// `var.field` when the item is an object that has it, otherwise `fallback`: options
/// and items may be plain values or objects.
fn field_or(var: &str, field: &str, fallback: Expr) -> Expr {
    let f = path(&[var, field]);
    cond(bin(BinOp::Ne, f.clone(), Expr::Lit(Json::Null)), f, fallback)
}

/// The icon name `value` holds, when the icon kit draws it: the kit is registered and
/// the value is a name like `settings` or `it:database`. Known on the server, written
/// in the document or from server data (a list item's, say), not from state.
fn icon_name(cx: &RenderCx<'_>, value: &Val) -> Option<Val> {
    if cx.is_reactive(value) || !cx.has_component("ui-icon") {
        return None;
    }
    let Json::String(n) = cx.eval(value) else { return None };
    let name = !n.is_empty() && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':');
    name.then(|| lit(n))
}

/// The icon kit's `ui-icon` for `name`, decorative.
fn icon_kit(cx: &mut RenderCx<'_>, node: &Node, name: &Val, size: u32) -> Result<(), RenderError> {
    let size = format!("calc({size}px * var(--size-icon, 1))");
    let mut i = synthetic(node, "ui-icon", vec![("name", name.clone()), ("size", lit(size))], Vec::new(), Vec::new());
    i.style = irender_core::StyleBlock::from_pairs(&[("flex-shrink", "0")]);
    cx.render(&i)
}

/// An icon in front of a label: a `ui-icon` when the icon kit is registered and the
/// value is an icon name (`settings`, `it:database`), otherwise the value as text,
/// e.g. an emoji. Decorative either way: the label next to it names the thing.
fn icon(cx: &mut RenderCx<'_>, node: &Node, value: &Val, size: u32) -> Result<(), RenderError> {
    if let Some(name) = icon_name(cx, value) {
        return icon_kit(cx, node, &name, size);
    }
    let mut span = Element::new("span");
    span.attr("aria-hidden", "true").attr("data-icon", true).text(value.clone());
    cx.open(&span, node)?;
    cx.close("span");
    Ok(())
}
