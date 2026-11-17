//! SVG icons for iRender: `ui-icon`, with packs of original icons for everyday
//! interfaces and for industries (logistics, warehouse, retail, manufacturing, …),
//! each in an outline and a filled style, and your own packs from SVG files.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_icon_kit::register(&mut engine.registry);
//!
//! let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-icon",
//!     "props": { "name": "logistics:truck", "size": 20, "label": "In transit" } } }"#, Format::Json)?;
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Icons are inlined as `<svg>` and drawn in `currentColor`, so they take the
//! colour of the text around them; only the icons a page uses are sent. A name is
//! either `pack:icon`, or a bare `icon` looked up in each pack in turn.
//!
//! Every built-in icon is drawn for this kit to one standard (`GUIDELINES.md`): a
//! 24 px grid with 2 px padding, 1.4 px round strokes and shared keylines. Its filled
//! style (`variant: filled`) is drawn on the same lines, so the two swap in place. Your own
//! packs ([`Pack::from_dir`], [`Pack::add_svg`]) can be any SVG; they are reduced
//! to shapes on load, so a pack file can't put scripts or links into a page.

mod svg;

pub use svg::Icon;

use indexmap::IndexMap;
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val};
use irender_html::{Component, RenderCx, RenderError, Registry};
use std::{
    collections::HashSet,
    fmt, fs,
    path::Path,
    sync::{Arc, LazyLock},
};

struct BuiltinPack {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icons: &'static [(&'static str, &'static str)],
    filled: &'static [(&'static str, &'static str)],
    same: &'static [&'static str],
}

static BUILTIN: &[BuiltinPack] = include!(concat!(env!("OUT_DIR"), "/packs.rs"));

static BUILTIN_PACKS: LazyLock<Vec<Arc<Pack>>> = LazyLock::new(|| {
    BUILTIN
        .iter()
        .map(|b| {
            let mut pack = Pack::new(b.id, b.title).expect("built-in pack id");
            pack.description = b.description.into();
            for (name, src) in b.icons {
                pack.add_svg(name, src).unwrap_or_else(|e| panic!("built-in icon {}:{name}: {e}", b.id));
            }
            for (name, src) in b.filled {
                pack.add_svg_filled(name, src).unwrap_or_else(|e| panic!("built-in icon {}:{name} (filled): {e}", b.id));
            }
            for name in b.same {
                pack.same_when_filled(name).unwrap_or_else(|e| panic!("built-in pack {}: {e}", b.id));
            }
            Arc::new(pack)
        })
        .collect()
});

/// A problem loading an icon or a pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconError(pub String);

impl fmt::Display for IconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for IconError {}

/// The two styles every built-in icon is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// 1.4 px strokes.
    #[default]
    Outline,
    /// Solid shapes with cut-out details, on the same lines as the outline.
    Filled,
}

impl Variant {
    pub fn as_str(self) -> &'static str {
        match self {
            Variant::Outline => "outline",
            Variant::Filled => "filled",
        }
    }
}

/// A named set of icons.
#[derive(Debug, Clone)]
pub struct Pack {
    id: String,
    title: String,
    description: String,
    icons: IndexMap<String, Icon>,
    filled: IndexMap<String, Icon>,
    /// Icons with nothing to fill (an arrow, a check): the outline serves both styles.
    same: HashSet<String>,
}

impl Pack {
    /// An empty pack. `id` is what documents write before the colon
    /// (`brand:logo`): lowercase letters, digits and dashes.
    pub fn new(id: &str, title: &str) -> Result<Self, IconError> {
        check_name("pack id", id)?;
        Ok(Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            icons: IndexMap::new(),
            filled: IndexMap::new(),
            same: HashSet::new(),
        })
    }

    /// A pack of every `*.svg` file in `dir`, named after its file name: `Box Truck.svg`
    /// becomes `box-truck`. Filled versions, if any, are read from `dir/filled/`
    /// (same file names), with `dir/filled/_same.txt` listing icons whose outline
    /// serves both styles.
    pub fn from_dir(id: &str, dir: impl AsRef<Path>) -> Result<Self, IconError> {
        let dir = dir.as_ref();
        let mut pack = Self::new(id, id)?;
        for (name, src) in read_svgs(dir)? {
            pack.add_svg(&name, &src.1).map_err(|e| IconError(format!("{}: {e}", src.0)))?;
        }
        let filled = dir.join("filled");
        if filled.is_dir() {
            for (name, src) in read_svgs(&filled)? {
                pack.add_svg_filled(&name, &src.1).map_err(|e| IconError(format!("{}: {e}", src.0)))?;
            }
            if let Ok(same) = fs::read_to_string(filled.join("_same.txt")) {
                for name in same.split_whitespace() {
                    pack.same_when_filled(name)?;
                }
            }
        }
        Ok(pack)
    }

    /// Adds (or replaces) an icon from SVG source. Only shapes and drawing
    /// attributes are kept.
    pub fn add_svg(&mut self, name: &str, svg: &str) -> Result<&mut Self, IconError> {
        check_name("icon name", name)?;
        let icon = svg::parse(svg).map_err(IconError)?;
        self.icons.insert(name.into(), icon);
        Ok(self)
    }

    /// Adds (or replaces) an icon's filled version. The outline version must be
    /// added too, before or after.
    pub fn add_svg_filled(&mut self, name: &str, svg: &str) -> Result<&mut Self, IconError> {
        check_name("icon name", name)?;
        let icon = svg::parse(svg).map_err(IconError)?;
        self.filled.insert(name.into(), icon);
        Ok(self)
    }

    /// Uses the outline drawing for the filled style too, for an icon with nothing
    /// to fill (an arrow, a check).
    pub fn same_when_filled(&mut self, name: &str) -> Result<&mut Self, IconError> {
        check_name("icon name", name)?;
        self.same.insert(name.into());
        Ok(self)
    }

    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.into();
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn get(&self, name: &str) -> Option<&Icon> {
        self.icons.get(name)
    }

    /// The icon in `variant`. `None` when the pack lacks it, or has it only in
    /// the other style.
    pub fn get_variant(&self, name: &str, variant: Variant) -> Option<&Icon> {
        match variant {
            Variant::Outline => self.icons.get(name),
            Variant::Filled if self.same.contains(name) => self.icons.get(name),
            Variant::Filled => self.filled.get(name).filter(|_| self.icons.contains_key(name)),
        }
    }

    /// Whether `name` can be drawn filled (it has a filled version, or is the same in both styles).
    pub fn has_filled(&self, name: &str) -> bool {
        self.get_variant(name, Variant::Filled).is_some()
    }

    /// Icon names, in the order they were added (alphabetical for built-in packs).
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.icons.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.icons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }
}

/// An icon file: its name, then its path (for errors) and source.
type SvgFile = (String, (String, String));

/// Every `*.svg` file in `dir`, sorted, named after the file: `Box Truck.svg` →
/// `box-truck`.
fn read_svgs(dir: &Path) -> Result<Vec<SvgFile>, IconError> {
    let entries = fs::read_dir(dir).map_err(|e| IconError(format!("{}: {e}", dir.display())))?;
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg")))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|file| {
            let stem = file.file_stem().unwrap_or_default().to_string_lossy();
            let name = stem.trim().chars().map(|c| if c == '_' || c == ' ' { '-' } else { c.to_ascii_lowercase() }).collect();
            let src = fs::read_to_string(&file).map_err(|e| IconError(format!("{}: {e}", file.display())))?;
            Ok((name, (file.display().to_string(), src)))
        })
        .collect()
}

fn check_name(what: &str, name: &str) -> Result<(), IconError> {
    let valid = !name.is_empty()
        && name.split('-').all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    if valid { Ok(()) } else { Err(IconError(format!("{what} `{name}` must be lowercase letters and digits joined by `-`"))) }
}

/// The packs `ui-icon` draws from.
#[derive(Debug, Clone)]
pub struct IconKit {
    packs: Vec<Arc<Pack>>,
}

impl Default for IconKit {
    fn default() -> Self {
        Self::new()
    }
}

impl IconKit {
    /// Every built-in pack: `core`, `arrows` and `status` first, then the industry
    /// packs alphabetically.
    pub fn new() -> Self {
        Self { packs: BUILTIN_PACKS.clone() }
    }

    /// No packs; add the ones you want with [`IconKit::with`] or [`IconKit::with_builtin`].
    pub fn empty() -> Self {
        Self { packs: Vec::new() }
    }

    /// The built-in packs, for listing or picking from.
    pub fn builtin() -> impl Iterator<Item = &'static Pack> {
        BUILTIN_PACKS.iter().map(|p| &**p)
    }

    /// Adds a built-in pack by id.
    pub fn with_builtin(mut self, id: &str) -> Result<Self, IconError> {
        let pack = BUILTIN_PACKS
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| IconError(format!("no built-in pack `{id}`")))?;
        self.packs.retain(|p| p.id != id);
        self.packs.push(pack.clone());
        Ok(self)
    }

    /// Adds a pack, replacing one with the same id in place. Bare names are looked
    /// up in the order packs were added, so add your own pack to an
    /// [`IconKit::empty`] kit first if its names should win.
    pub fn with(mut self, pack: Pack) -> Self {
        match self.packs.iter().position(|p| p.id == pack.id) {
            Some(i) => self.packs[i] = Arc::new(pack),
            None => self.packs.push(Arc::new(pack)),
        }
        self
    }

    pub fn packs(&self) -> impl Iterator<Item = &Pack> {
        self.packs.iter().map(|p| &**p)
    }

    pub fn pack(&self, id: &str) -> Option<&Pack> {
        self.packs().find(|p| p.id == id)
    }

    /// The icon for `pack:name` or a bare `name`, in `variant`.
    pub fn find(&self, name: &str, variant: Variant) -> Result<(&Pack, &Icon), IconError> {
        let (pack, icon) = match name.split_once(':') {
            Some((id, icon)) => {
                let pack = self.pack(id).ok_or_else(|| {
                    let ids: Vec<_> = self.packs().map(|p| p.id.as_str()).collect();
                    IconError(format!("no icon pack `{id}`; packs: {}", ids.join(", ")))
                })?;
                if pack.get(icon).is_none() {
                    return Err(self.unknown(name, Some(pack)));
                }
                (pack, icon)
            }
            None => self.packs().find(|p| p.get(name).is_some()).map(|p| (p, name)).ok_or_else(|| self.unknown(name, None))?,
        };
        pack.get_variant(icon, variant)
            .map(|i| (pack, i))
            .ok_or_else(|| IconError(format!("`{}:{icon}` has no {} style", pack.id, variant.as_str())))
    }

    fn unknown(&self, name: &str, pack: Option<&Pack>) -> IconError {
        let wanted = name.rsplit(':').next().unwrap_or(name);
        let mut close: Vec<(usize, String)> = self
            .packs()
            .filter(|p| pack.is_none_or(|w| w.id == p.id))
            .flat_map(|p| p.names().map(move |n| (distance(wanted, n), format!("{}:{n}", p.id))))
            .filter(|(d, _)| *d <= 2.max(wanted.len() / 4))
            .collect();
        close.sort();
        let hint = match close.len() {
            0 => String::new(),
            _ => format!("; did you mean {}?", close.iter().take(3).map(|(_, n)| format!("`{n}`")).collect::<Vec<_>>().join(", ")),
        };
        IconError(format!("unknown icon `{name}`{hint}"))
    }
}

/// Edit distance, for suggestions.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (row[j + 1] + 1).min(row[j] + 1).min(prev + usize::from(ca != *cb));
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

/// Adds `ui-icon` with every built-in pack to `registry`.
pub fn register(registry: &mut Registry) {
    register_with(registry, IconKit::new());
}

/// Adds `ui-icon` drawing from `kit`'s packs.
pub fn register_with(registry: &mut Registry, kit: IconKit) {
    registry.register(UiIcon { kit });
}

const VARIANTS: &[&str] = &["outline", "filled"];

const PROPS: &[PropDef] = &[
    PropDef::new("name", PropKind::String, "`pack:icon` (e.g. `logistics:truck`) or a bare `icon`, found in the first pack that has it")
        .required(),
    PropDef::new("variant", PropKind::Enum(VARIANTS), "`outline` (default) or `filled`"),
    PropDef::new("size", PropKind::Any, "Number (px) or CSS length; default `1em`, the size of the text around it"),
    PropDef::new("label", PropKind::Any, "Accessible name. Without one the icon is decorative and hidden from screen readers"),
];

/// `ui-icon`
struct UiIcon {
    kit: IconKit,
}

impl Component for UiIcon {
    fn name(&self) -> &'static str {
        "ui-icon"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An SVG icon from the icon kit's packs, drawn in the current text colour.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        for prop in ["name", "variant"] {
            if node.prop(prop).is_some_and(|v| cx.is_reactive(v)) {
                return Err(RenderError::new(
                    node,
                    format!("`{prop}` can't use state; icons are drawn on the server. Switch between icons with `if` / `else`"),
                ));
            }
        }
        let name = cx.prop_string(node, "name").ok_or_else(|| RenderError::new(node, "`name` is required"))?;
        let variant = match cx.prop_string(node, "variant").as_deref() {
            Some("filled") => Variant::Filled,
            _ => Variant::Outline,
        };
        let (_, icon) = self.kit.find(&name, variant).map_err(|e| RenderError::new(node, e.0))?;

        let mut sizing = StyleBlock::from_pairs(&[
            ("display", "inline-block"),
            ("flex-shrink", "0"),
            ("vertical-align", "-0.125em"),
        ]);
        let size = node.prop("size").cloned().unwrap_or_else(|| Val::from("1em"));
        for property in ["width", "height"] {
            if let Some(v) = cx.style_value(node, property, &size)? {
                sizing.push(property, v);
            }
        }

        let mut el = cx.element("svg", node);
        el.base_style(sizing)
            .attr("xmlns", "http://www.w3.org/2000/svg")
            .attr("viewBox", icon.view_box.as_str());
        // The stroke width is the icon's own (1.4 for the built-in packs): the
        // drawings' spacing is designed for it, so documents can't change it.
        for (k, v) in &icon.attrs {
            el.attr(k, v.as_str());
        }
        match node.prop("label") {
            Some(label) => el.attr("role", "img").attr("aria-label", label.clone()),
            None => el.attr("aria-hidden", "true"),
        };
        cx.open(&el, node)?;
        cx.markup(&icon.body);
        cx.close("svg");
        Ok(())
    }
}
