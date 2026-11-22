//! `ui-nfc`, `ui-bluetooth`, `ui-serial`, `ui-usb` and `ui-file-handler`. `hardware.js` runs
//! them. The first four are only in Chrome and its relatives, and say so anywhere else.

use crate::{
    BORDER, MONO, MUTED, SUBTLE, TEXT, lit,
    widget::{self, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Map, Value as Json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("hardware.js");

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& small", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& dl", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fit, minmax(110px, 1fr))"), ("gap", gap!(10)), ("margin", "0")]))
        .nest("& dt", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& dd", pairs(&[("margin", "0"), ("font-size", fs!(20)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-hw-log]", pairs(&[("box-sizing", "border-box"), ("height", "160px"), ("margin", "0"), ("padding", pad!(8, 10)), ("overflow", "auto"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", SUBTLE), ("color", TEXT), ("font-family", MONO), ("font-size", fs!(12)), ("line-height", "1.5"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-hw-log]:empty", pairs(&[("display", "none")]))
        .nest("& [data-hw-send]", pairs(&[("display", "flex"), ("gap", gap!(8)), ("margin", "0")]))
        .nest("& [data-hw-send] input", pairs(&[("flex", "1")]))
});

fn panel(cx: &mut RenderCx<'_>, node: &Node, name: &'static str, config: Map<String, Json>, label: &str) -> Result<(), RenderError> {
    widget::device_kit(cx);
    cx.global_js("ui-hardware", SCRIPT);
    let label = lit(cx.label(label));
    widget::drawn(cx, node, "div", &PANEL, name, config, vec![("role", lit("group")), ("aria-label", label)])
}

pub struct Nfc;
pub struct Bluetooth;
pub struct Serial;
pub struct Usb;
pub struct FileHandler;

const NFC_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets the last tag read, `{ serial, records: [{ type, text }], text, at }`").required(),
    PropDef::new("bindList", PropKind::StateKey, "State key that collects each different tag"),
    PropDef::new("write", PropKind::Any, "Text or an address to write, with a button for it; may use state"),
    PropDef::new("lock", PropKind::Bool, "What's written can't be changed again, ever"),
    PropDef::new("label", PropKind::Any, "Default `Read a tag`"),
];
const AS: &str = "`as`: uint8 (default), int8, uint16, int16, uint32, float32, text, hex or heart-rate; with `offset` and `scale`";
const BLE_PROPS: &[PropDef] = &[
    PropDef::new("read", PropKind::Any, "What's read: `[{ key, service, characteristic, as, notify, label, unit }]`, by name (`battery_service`) or number; `as`: uint8 (default), int8, uint16, int16, uint32, float32, text, hex or heart-rate; `scale`, `offset`").required(),
    PropDef::new("bind", PropKind::StateKey, "State key that gets the values by `key`"),
    PropDef::new("bindState", PropKind::StateKey, "State key that gets `disconnected`, `connecting` or `connected`"),
    PropDef::new("commands", PropKind::Any, "Buttons that write: `[{ label, service, characteristic, value, as }]`; `value` may use state"),
    PropDef::new("namePrefix", PropKind::String, "Only devices whose name starts with this are listed"),
    PropDef::new("label", PropKind::Any, "Default `Connect a device`"),
];
const SERIAL_PROPS: &[PropDef] = &[
    PropDef::new("baud", PropKind::Number, "Default 9600"),
    PropDef::new("bind", PropKind::StateKey, "State key that gets the last line, read by `parse`"),
    PropDef::new("bindLog", PropKind::StateKey, "State key that gets the last lines, `[{ at, text, out }]`"),
    PropDef::new("bindState", PropKind::StateKey, "State key that gets `disconnected`, `connecting` or `connected`"),
    PropDef::new("parse", PropKind::Enum(&["text", "number", "json", "csv"]), "What a line is (default `text`); `number` is the first one in it"),
    PropDef::new("end", PropKind::Enum(&["lf", "crlf", "cr", "none"]), "Ends what's sent (default `lf`)"),
    PropDef::new("filters", PropKind::Any, "Only these are listed: `[{ usbVendorId, usbProductId }]`"),
    PropDef::new("send", PropKind::Bool, "A field to send lines with"),
    PropDef::new("commands", PropKind::Any, "Buttons that send: `[{ label, text }]`; `text` may use state"),
    PropDef::new("log", PropKind::Bool, "Shows the lines (default true)"),
    PropDef::new("max", PropKind::Number, "Lines kept (default 200)"),
    PropDef::new("label", PropKind::Any, "Default `Connect a port`"),
];
const USB_PROPS: &[PropDef] = &[
    PropDef::new("filters", PropKind::Any, "Which devices are listed: `[{ vendorId, productId, classCode }]`").required(),
    PropDef::new("bind", PropKind::StateKey, "State key that gets what's read last, by `as`"),
    PropDef::new("bindState", PropKind::StateKey, "State key that gets `disconnected`, `connecting` or `connected`"),
    PropDef::new("bindDevice", PropKind::StateKey, "State key that gets `{ name, maker, serial, vendorId, productId }`"),
    PropDef::new("interface", PropKind::Number, "Its number (default the first that can be used)"),
    PropDef::new("as", PropKind::Enum(&["text", "hex", "bytes"]), "What's read and sent are (default `text`)"),
    PropDef::new("commands", PropKind::Any, "Buttons that send: `[{ label, value }]`; `value` may use state"),
    PropDef::new("label", PropKind::Any, "Default `Connect a device`"),
];
const FILE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets the file opened: `{ name, type, size, text }`, or `url` (a `data:` address) or `json` for another `as`").required(),
    PropDef::new("as", PropKind::Enum(&["text", "json", "data"]), "Default `text`"),
    PropDef::new("accept", PropKind::Any, "Types and their endings: `{ text/markdown: [.md] }`"),
    PropDef::new("save", PropKind::Any, "What Save writes, to the file that was opened where the browser can, else as a new one; may use state"),
    PropDef::new("name", PropKind::Any, "A new file's name"),
    PropDef::new("bindDirty", PropKind::StateKey, "State key that's true while `save` isn't what's in the file"),
    PropDef::new("label", PropKind::Any, "Default `Open a file`"),
];

fn commands(cx: &RenderCx<'_>, node: &Node, config: &mut Map<String, Json>) -> Result<(), RenderError> {
    let more = widget::config(cx, node, &["commands", "label"], &[], &[])?;
    config.extend(more);
    Ok(())
}

impl Component for Nfc {
    fn name(&self) -> &'static str {
        "ui-nfc"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Reads NFC tags held to the phone, and writes to them (Chrome on Android).", props: NFC_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["write", "label"], &[], &["bind", "bindList"])?;
        widget::flags(cx, node, &mut config, &[("lock", false)]);
        panel(cx, node, "ui-nfc", config, "NFC")
    }
}

impl Component for Bluetooth {
    fn name(&self) -> &'static str {
        "ui-bluetooth"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Connects to a Bluetooth Low Energy device, reads and follows its values, and sends to it (Chrome and Edge; not iPhones).", props: BLE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &[], &["read", "namePrefix"], &["bind", "bindState"])?;
        let ok = config.get("read").and_then(Json::as_array).is_some_and(|r| !r.is_empty() && r.iter().all(|x| ["key", "service", "characteristic"].iter().all(|k| !x[*k].is_null())));
        if !ok {
            return Err(RenderError::new(node, format!("each of `read` has `key`, `service` and `characteristic`; {AS}")));
        }
        commands(cx, node, &mut config)?;
        panel(cx, node, "ui-bluetooth", config, "Bluetooth")
    }
}

impl Component for Serial {
    fn name(&self) -> &'static str {
        "ui-serial"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Talks to a serial port, a line at a time: scales, meters, microcontrollers (Chrome and Edge on a computer).", props: SERIAL_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &[], &["baud", "parse", "end", "filters", "max"], &["bind", "bindLog", "bindState"])?;
        widget::flags(cx, node, &mut config, &[("send", false), ("log", true)]);
        commands(cx, node, &mut config)?;
        panel(cx, node, "ui-serial", config, "Serial port")
    }
}

impl Component for Usb {
    fn name(&self) -> &'static str {
        "ui-usb"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Talks to a USB device that no driver has taken: label printers, scanners, instruments (Chrome and Edge).", props: USB_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &[], &["filters", "interface", "as"], &["bind", "bindState", "bindDevice"])?;
        if !config.get("filters").is_some_and(Json::is_array) {
            return Err(RenderError::new(node, "`filters` is a list, such as `[{ vendorId: 1208 }]`"));
        }
        commands(cx, node, &mut config)?;
        panel(cx, node, "ui-usb", config, "USB")
    }
}

impl Component for FileHandler {
    fn name(&self) -> &'static str {
        "ui-file-handler"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Opens files, from a button or, installed, when one's opened with the app (the document's `app.files`), and saves back to them.", props: FILE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let config = widget::config(cx, node, &["save", "name", "label"], &["as", "accept"], &["bind", "bindDirty"])?;
        crate::sheets(cx);
        panel(cx, node, "ui-file-handler", config, "File")
    }
}
