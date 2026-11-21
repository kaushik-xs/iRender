//! `ui-kanban`: a board of columns with cards to move between them. The server draws the
//! starting board from `columns` and `cards`; `kanban.js` draws it again whenever they
//! change, and moves cards by dragging or with Alt and the arrow keys, writing the new
//! order (and each card's `column`) to `bindCards`.

use crate::data_table::dynamic;
use crate::status::tone_rules;
use crate::{ACCENT, BORDER, MUTED, PRIMARY, TEXT, forms, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("kanban.js");

pub struct Kanban;

const PROPS: &[PropDef] = &[
    PropDef::new("columns", PropKind::Any, "The columns, in order: `{ id, title, tone, limit }` (or plain names). `limit` marks a column over it").required(),
    PropDef::new(
        "cards",
        PropKind::Any,
        "The cards, in order: `{ id, column, title, description, tags, assignee, due, priority }`, usually from state, e.g. `{{ tasks }}`",
    )
    .required(),
    PropDef::new("bindCards", PropKind::StateKey, "State key the cards are in: moves and new cards are written back to it"),
    PropDef::new("bindCard", PropKind::StateKey, "State key that gets the card clicked or opened with Enter"),
    PropDef::new("movable", PropKind::Bool, "Drag cards, or move the focused one with Alt and the arrow keys (default true)"),
    PropDef::new("addable", PropKind::Bool, "An Add card button under each column, which asks for a title (needs `bindCards`)"),
    PropDef::new("collapsible", PropKind::Bool, "A button on each column to fold it away"),
    PropDef::new("collapsed", PropKind::Any, "Columns folded to start with, by id"),
    PropDef::new("addLabel", PropKind::Any, "The add button's text (default `Add card`)"),
    PropDef::new("emptyText", PropKind::String, "In a column with no cards (default `No cards`)"),
    PropDef::new("columnWidth", PropKind::Number, "Each column's width in px (default 280)"),
    PropDef::new("label", PropKind::Any, "Names the board for screen readers (default `Board`)"),
    PropDef::new("swimlanes", PropKind::String, "A card field to split the board by: a row of columns per value, e.g. `assignee`; moving a card to a lane writes it. May use state (null: no lanes)"),
    PropDef::new("lanes", PropKind::Any, "With `swimlanes`, the lanes in order: values or `{ value, label }` (default: the values as cards have them)"),
    PropDef::new(
        "fields",
        PropKind::Any,
        "More of each card, a line each: `{ field, label, cell }`, `cell` being `text` (default), `badge` (with `tones`), `progress` (0–100) or `date`. May use state, e.g. a list picked by a setting",
    ),
    PropDef::new(
        "reorderable",
        PropKind::Bool,
        "A grip on each column's header moves the column: dragged, or Left and Right on it (with `bindColumns`); with lanes, one on each lane's header moves the lane, or Up and Down (with `bindLanes`)",
    ),
    PropDef::new("bindColumns", PropKind::StateKey, "State key the columns are in: a moved column's new order is written back to it"),
    PropDef::new("bindLanes", PropKind::StateKey, "State key `lanes` reads: a moved lane's new order is written back to it"),
    PropDef::new("locale", PropKind::String, "Locale for `date` fields (default: the document's `lang`)"),
    PropDef::new(
        "filter",
        PropKind::Any,
        "Shows only the cards that pass: `{ field: condition }`, as `ui-data-table`'s and `ui-chart`'s `filter`, e.g. a `ui-filters` panel's `bind`. Applied in the browser",
    ),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static BOARD: LazyLock<StyleBlock> = LazyLock::new(|| {
    let tag = pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("padding", sp!(1, 8)),
        ("border-radius", "999px"),
        ("background", "color-mix(in srgb, var(--tone) 12%, transparent)"),
        ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"),
        ("font-size", fs!(11)),
        ("font-weight", "500"),
        ("line-height", "1.6"),
    ]);
    let b = pairs(&[("color", TEXT), ("font-size", fs!(14))])
        .nest(
            "& [data-kb-board]",
            pairs(&[("display", "flex"), ("align-items", "flex-start"), ("gap", sp!(12)), ("overflow-x", "auto"), ("padding-bottom", sp!(8)), ("scrollbar-width", "thin")]),
        )
        .nest(
            "& [data-kb-column]",
            pairs(&[
                ("--tone", MUTED),
                ("flex", "0 0 var(--kb-width, 280px)"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", sp!(8)),
                ("box-sizing", "border-box"),
                ("padding", sp!(10)),
                ("border-radius", "var(--radius-lg)"),
                ("background", "color-mix(in srgb, var(--color-text) 3.5%, var(--color-surface, #fff))"),
                ("transition", "background .15s, box-shadow .15s"),
            ]),
        )
        // A long press picks a card up, rather than selecting its text.
        .nest("@media (pointer: coarse)", pairs(&[]).nest("& [data-kb-card]", pairs(&[("user-select", "none"), ("-webkit-touch-callout", "none")])))
        .nest("& [data-kb-column][data-over-drop]", pairs(&[("box-shadow", "inset 0 0 0 1.5px color-mix(in srgb, var(--color-primary) 45%, transparent)")]))
        .nest("& [data-kb-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding", sp!(2, 4))]))
        .nest("& [data-kb-head] h3", pairs(&[("margin", "0"), ("font-size", fs!(13)), ("font-weight", "600"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-kb-dot]", pairs(&[("flex", "none"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest(
            "& [data-kb-count]",
            // "2 / 3" reads left to right on any page.
            pairs(&[("direction", "ltr"), ("unicode-bidi", "isolate"), ("padding", sp!(0, 7)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]),
        )
        .nest("& [data-over] [data-kb-count]", pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 14%, transparent)"), ("color", "var(--color-destructive, #dc2626)"), ("font-weight", "600")]))
        .nest(
            "& [data-kb-fold]",
            pairs(&[("margin-inline-start", "auto"), ("display", "inline-flex"), ("padding", sp!(4)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("cursor", "pointer")]),
        )
        .nest("& [data-kb-fold]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
        .nest("& [data-kb-fold]:focus-visible", forms::focus_ring())
        // Folded: a narrow strip with the title down its side.
        .nest("& [data-kb-column][data-collapsed]", pairs(&[("flex-basis", "44px"), ("align-items", "center")]))
        // Swimlanes: the columns' headers in a row, then a row of cells per lane.
        .nest("& [data-kb-board][data-kb-lanes]", pairs(&[("flex-direction", "column"), ("align-items", "stretch"), ("gap", sp!(8))]))
        .nest("& :is([data-kb-heads], [data-kb-cells])", pairs(&[("display", "flex"), ("align-items", "stretch"), ("gap", sp!(12)), ("width", "max-content")]))
        .nest(
            "& [data-kb-colhead]",
            pairs(&[
                ("--tone", MUTED),
                ("flex", "0 0 var(--kb-width, 280px)"),
                ("box-sizing", "border-box"),
                ("padding", sp!(6, 10)),
                ("border-radius", "var(--radius-lg)"),
                ("background", "color-mix(in srgb, var(--color-text) 3.5%, var(--color-surface, #fff))"),
            ]),
        )
        .nest("& [data-kb-colhead][data-collapsed]", pairs(&[("flex-basis", "44px")]))
        .nest("& [data-kb-lanes] [data-kb-column]", pairs(&[("border-radius", "var(--radius-md)"), ("min-height", "56px")]))
        .nest(
            "& [data-kb-lane-head]",
            pairs(&[("position", "sticky"), ("inset-inline-start", "0"), ("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("width", "fit-content"), ("padding", sp!(6, 2, 4))]),
        )
        .nest(
            "& [data-kb-lane-fold]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("gap", sp!(6)),
                ("padding", sp!(2, 6)),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", TEXT),
                ("font", "inherit"),
                ("font-weight", "600"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-kb-lane-fold]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-kb-lane-fold]:focus-visible", forms::focus_ring())
        .nest("& [data-kb-lane-row][data-collapsed] [data-kb-lane-fold] svg", pairs(&[("transform", "rotate(-90deg)")]))
        .nest("&:dir(rtl) [data-kb-lane-row][data-collapsed] [data-kb-lane-fold] svg", pairs(&[("transform", "rotate(90deg)")]))
        .nest("& [data-kb-lane-head] [data-kb-count]", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        // A column's grip, to move it.
        .nest(
            "& :is([data-kb-grip], [data-kb-lane-grip])",
            pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "18px"), ("height", "22px"), ("padding", "0"), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", MUTED), ("cursor", "grab"), ("opacity", ".7")]),
        )
        .nest("& :is([data-kb-grip], [data-kb-lane-grip]):hover", pairs(&[("background", ACCENT), ("opacity", "1")]))
        .nest("& :is([data-kb-grip], [data-kb-lane-grip]):focus-visible", forms::focus_ring())
        // A lane dragged over a row: a line above or below it, where it goes.
        .nest("& [data-kb-lane-row][data-over-drop=before]", pairs(&[("box-shadow", "0 -3px 0 var(--color-primary)")]))
        .nest("& [data-kb-lane-row][data-over-drop=after]", pairs(&[("box-shadow", "0 3px 0 var(--color-primary)")]))
        .nest("& [data-kb-head][data-over-drop]", pairs(&[("box-shadow", "inset 3px 0 0 var(--color-primary)"), ("border-radius", "var(--radius-sm)")]))
        // `fields`: a line each.
        .nest("& [data-kb-fields]", pairs(&[("display", "grid"), ("gap", sp!(4)), ("font-size", fs!(12))]))
        .nest("& [data-kb-field]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(6)), ("min-width", "0")]))
        .nest("& [data-kb-field-label]", pairs(&[("color", MUTED)]))
        .nest("& [data-kb-field-value]", pairs(&[("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest(
            "& [data-kb-progress]",
            pairs(&[("flex", "1"), ("height", "6px"), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 10%, transparent)"), ("overflow", "hidden")]),
        )
        .nest("& [data-kb-progress] > span", pairs(&[("display", "block"), ("height", "100%"), ("background", "var(--color-primary)")]))
        .nest("& [data-collapsed] [data-kb-head]", pairs(&[("flex-direction", "column"), ("padding", "2px 0")]))
        .nest("& [data-collapsed] [data-kb-head] h3", pairs(&[("writing-mode", "vertical-rl"), ("overflow", "visible")]))
        .nest("& [data-collapsed] [data-kb-fold]", pairs(&[("margin", "0"), ("order", "-1"), ("transform", "rotate(180deg)")]))
        .nest("& [data-collapsed] :is([data-kb-cards], [data-kb-add], [data-kb-name])", pairs(&[("display", "none")]))
        .nest("& [data-kb-cards]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(8)), ("min-height", "48px")]))
        .nest("& [data-kb-empty]", pairs(&[("margin", "0"), ("padding", sp!(12, 4)), ("color", MUTED), ("font-size", fs!(13)), ("text-align", "center")]))
        .nest(
            "& [data-kb-card]",
            pairs(&[
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", sp!(8)),
                ("padding", sp!(10, 12)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "var(--color-card, var(--color-surface, #fff))"),
                ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
                ("cursor", "pointer"),
                ("outline", "none"),
                ("transition", "box-shadow .15s, border-color .15s, opacity .15s"),
            ]),
        )
        .nest("& [data-kb-card][draggable=true]", pairs(&[("cursor", "grab")]))
        .nest("& [data-kb-card]:hover", pairs(&[("border-color", "color-mix(in srgb, var(--color-text) 22%, transparent)"), ("box-shadow", "0 2px 6px rgb(0 0 0 / 0.08)")]))
        .nest("& [data-kb-card]:focus-visible", forms::focus_ring().set("outline-offset", "1px"))
        .nest("& [data-kb-card][data-dragging]", pairs(&[("opacity", "0.35")]))
        .nest("& [data-kb-title]", pairs(&[("font-weight", "500"), ("line-height", "1.35"), ("overflow-wrap", "anywhere")]))
        .nest(
            "& [data-kb-desc]",
            pairs(&[("margin", "0"), ("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.4"), ("display", "-webkit-box"), ("-webkit-line-clamp", "2"), ("-webkit-box-orient", "vertical"), ("overflow", "hidden")]),
        )
        .nest("& [data-kb-tags]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", sp!(4))]))
        .nest("& [data-kb-tags] span, & [data-kb-priority], & [data-kb-badge]", tag)
        .nest("& [data-kb-foot]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", MUTED), ("font-size", fs!(12))]))
        .nest(
            "& [data-kb-avatar]",
            pairs(&[
                ("margin-inline-start", "auto"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "24px"),
                ("height", "24px"),
                ("border-radius", "50%"),
                ("overflow", "hidden"),
                ("background", "color-mix(in srgb, var(--color-primary) 14%, var(--color-surface, #fff))"),
                ("color", TEXT),
                ("font-size", fs!(10)),
                ("font-weight", "600"),
            ]),
        )
        .nest("& [data-kb-avatar] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-kb-drop]", pairs(&[("height", "2px"), ("margin", sp!(-5, 2)), ("border-radius", "2px"), ("background", PRIMARY)]))
        .nest(
            "& [data-kb-add]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", sp!(6)),
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("font", "inherit"),
                ("font-size", fs!(13)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-kb-add]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("color", TEXT)]))
        .nest("& [data-kb-add]:focus-visible", forms::focus_ring())
        .nest(
            "& [data-kb-name]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("width", "100%"),
                ("padding", sp!(8, 10)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "var(--color-card, var(--color-surface, #fff))"),
                ("color", TEXT),
                ("font", "inherit"),
                ("resize", "vertical"),
            ]),
        )
        .nest("& [data-kb-name]:focus-visible", forms::focus_ring().set("border-color", BORDER))
        .nest(
            "& [data-kb-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& *", pairs(&[("transition", "none !important")])));
    // Column dots, tags and priorities take their tone.
    tone_rules(tone_rules(tone_rules(b, "& [data-kb-column]"), "& [data-kb-colhead]"), "& :is([data-kb-tags] span, [data-kb-priority], [data-kb-badge])")
});

const PRIORITY: &[(&str, &str)] = &[("urgent", "danger"), ("critical", "danger"), ("high", "warning"), ("medium", "info"), ("normal", "info"), ("low", "neutral")];

fn text(v: Option<&Json>) -> String {
    v.map(expr::stringify).unwrap_or_default()
}

fn initials_of(name: &str) -> String {
    name.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect()
}

impl Component for Kanban {
    fn name(&self) -> &'static str {
        "ui-kanban"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A board of columns with cards to drag between them (or move with Alt and the arrow keys), add to, and open. \
                  The new order goes to `bindCards`, with `move`, `add` and `open` events.",
            props: PROPS,
            parts: &["board"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let flag = |cx: &RenderCx<'_>, name: &str, default: bool| cx.prop(node, name).map_or(default, |v| expr::truthy(&v));
        let (movable, addable, collapsible) = (flag(cx, "movable", true), flag(cx, "addable", false), flag(cx, "collapsible", false));
        let list = |cx: &RenderCx<'_>, name: &str| -> Result<Vec<Json>, RenderError> {
            match cx.prop(node, name) {
                Some(Json::Array(a)) => Ok(a),
                None | Some(Json::Null) => Ok(Vec::new()),
                Some(_) => Err(RenderError::new(node, format!("`{name}` must be a list"))),
            }
        };
        let columns = list(cx, "columns")?;
        let cards = list(cx, "cards")?;
        let collapsed: Vec<String> = list(cx, "collapsed")?.iter().map(expr::stringify).collect();
        let bind_cards = cx.prop_string(node, "bindCards");
        if addable && bind_cards.is_none() {
            return Err(RenderError::new(node, "`addable` needs `bindCards`, to add the new card to"));
        }
        let width = cx.prop(node, "columnWidth").map_or(280.0, |v| expr::to_num(&v));
        let lane_field = cx.prop_string(node, "swimlanes").filter(|f| !f.is_empty());
        let reorderable = flag(cx, "reorderable", false);
        let fields: Vec<Json> = match cx.prop(node, "fields") {
            None | Some(Json::Null) => Vec::new(),
            Some(Json::Array(a)) => a
                .into_iter()
                .map(|f| match f {
                    Json::String(name) => Ok(json!({ "field": name })),
                    Json::Object(ref m) if m.contains_key("field") => {
                        let cell = m.get("cell").map(expr::stringify).unwrap_or_else(|| "text".into());
                        if !["text", "badge", "progress", "date"].contains(&cell.as_str()) {
                            return Err(RenderError::new(node, format!("a field's `cell` is text, badge, progress or date, not `{cell}`")));
                        }
                        Ok(f)
                    }
                    _ => Err(RenderError::new(node, "each of `fields` is a field name or `{ field, label, cell }`")),
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(RenderError::new(node, "`fields` must be a list")),
        };

        crate::touch_drag(cx);
        cx.global_js("ui-kanban", SCRIPT);
        let config = json!({
            "columns": dynamic(cx, node, "columns", node.prop("columns").expect("required"))?,
            "cards": dynamic(cx, node, "cards", node.prop("cards").expect("required"))?,
            "bindCards": bind_cards, "bindCard": cx.prop_string(node, "bindCard"),
            "movable": movable, "addable": addable, "collapsible": collapsible, "collapsed": collapsed,
            "addLabel": node.prop("addLabel").map(|v| expr::stringify(&cx.eval(v))),
            "emptyText": cx.prop_string(node, "emptyText"),
            "swimlanes": node.prop("swimlanes").map(|v| dynamic(cx, node, "swimlanes", v)).transpose()?,
            "lanes": node.prop("lanes").map(|l| dynamic(cx, node, "lanes", l)).transpose()?,
            "fields": node.prop("fields").map(|v| dynamic(cx, node, "fields", v)).transpose()?,
            "filter": node.prop("filter").map(|f| dynamic(cx, node, "filter", f)).transpose()?,
            "reorderable": reorderable,
            "bindColumns": cx.prop_string(node, "bindColumns"),
            "bindLanes": cx.prop_string(node, "bindLanes"),
            "locale": crate::locale(cx, node),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&BOARD)
            .base_style(StyleBlock::new().set("--kb-width", format!("{width}px")))
            .behavior("ui-kanban", config)
            .attr("role", "region")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Board"))))
            .attr("aria-roledescription", cx.label("board"));
        cx.open(&root, node)?;
        let mut board = cx.part("div", node, "board");
        board.attr("data-kb-board", true);
        if lane_field.is_some() {
            board.attr("data-kb-lanes", true);
        }
        cx.open(&board, node)?;
        let look = Look {
            empty: cx.prop_string(node, "emptyText").unwrap_or_else(|| cx.label("No cards")),
            add: node.prop("addLabel").cloned().unwrap_or_else(|| lit(cx.label("Add card"))),
            movable,
            addable,
            collapsible,
            reorderable,
            fields: &fields,
        };
        let mut first = true;
        let column_cards = |id: &str| cards.iter().filter(|c| text(c.get("column")) == id).collect::<Vec<_>>();
        match &lane_field {
            None => {
                for col in &columns {
                    let id = col_id(col);
                    let mine = column_cards(&id);
                    column_cell(cx, node, &look, col, None, &mine, mine.len(), &collapsed, true, &mut first)?;
                }
            }
            Some(field) => {
                // A row of the columns' headers, then a row of cells per lane under its header.
                let mut heads = Element::new("div");
                heads.attr("data-kb-heads", true);
                cx.open(&heads, node)?;
                for col in &columns {
                    let id = col_id(col);
                    let mut h = Element::new("div");
                    h.attr("data-kb-colhead", id.clone());
                    if let Some(t) = col.get("tone").filter(|t| !t.is_null()) {
                        h.attr("data-tone", expr::stringify(t));
                    }
                    if collapsed.contains(&id) {
                        h.attr("data-collapsed", true);
                    }
                    cx.open(&h, node)?;
                    column_head(cx, node, &look, col, column_cards(&id).len(), &collapsed)?;
                    cx.close("div");
                }
                cx.close("div");
                for (key, name) in lanes_of(cx, node, field, &cards)? {
                    let mine: Vec<&Json> = cards.iter().filter(|c| lane_key(lookup(c, field)) == key).collect();
                    let mut row = Element::new("section");
                    row.attr("data-kb-lane-row", key.clone()).attr("aria-label", format!("{name}, {}", cx.label_count("{n} card", "{n} cards", mine.len() as f64)));
                    cx.open(&row, node)?;
                    let mut head = Element::new("header");
                    head.attr("data-kb-lane-head", true);
                    cx.open(&head, node)?;
                    let mut b = Element::new("button");
                    b.attr("type", "button").attr("data-kb-lane-fold", key.clone()).attr("aria-expanded", "true");
                    cx.open(&b, node)?;
                    cx.markup(LANE_ICON);
                    let mut l = Element::new("span");
                    l.text(lit(name.clone()));
                    cx.open(&l, node)?;
                    cx.close("span");
                    cx.close("button");
                    let mut n = Element::new("span");
                    n.attr("data-kb-count", true).text(lit(mine.len().to_string()));
                    cx.open(&n, node)?;
                    cx.close("span");
                    cx.close("header");
                    let mut cells = Element::new("div");
                    cells.attr("data-kb-cells", true);
                    cx.open(&cells, node)?;
                    for col in &columns {
                        let id = col_id(col);
                        let here: Vec<&Json> = mine.iter().copied().filter(|c| text(c.get("column")) == id).collect();
                        column_cell(cx, node, &look, col, Some((&key, &name)), &here, column_cards(&id).len(), &collapsed, false, &mut first)?;
                    }
                    cx.close("div");
                    cx.close("section");
                }
            }
        }
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-kb-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

const LANE_ICON: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>"#;
const GRIP: &str = r#"<svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" aria-hidden="true"><circle cx="2.5" cy="2.5" r="1.2"/><circle cx="7.5" cy="2.5" r="1.2"/><circle cx="2.5" cy="7" r="1.2"/><circle cx="7.5" cy="7" r="1.2"/><circle cx="2.5" cy="11.5" r="1.2"/><circle cx="7.5" cy="11.5" r="1.2"/></svg>"#;

/// How the board draws its columns and cards.
struct Look<'a> {
    empty: String,
    add: irender_core::Val,
    movable: bool,
    addable: bool,
    collapsible: bool,
    reorderable: bool,
    fields: &'a [Json],
}

fn col_id(col: &Json) -> String {
    match col {
        Json::Object(m) => text(m.get("id")),
        other => expr::stringify(other),
    }
}
fn col_title(col: &Json) -> String {
    match col {
        Json::Object(m) => m.get("title").map(expr::stringify).unwrap_or_else(|| text(m.get("id"))),
        other => expr::stringify(other),
    }
}
/// A dotted field of a card.
fn lookup<'a>(c: &'a Json, field: &str) -> Option<&'a Json> {
    field.split('.').try_fold(c, |v, k| v.get(k))
}
/// A lane's key from a card's value: a name (or label, id) for an object, else the text.
fn lane_key(v: Option<&Json>) -> String {
    match v {
        None | Some(Json::Null) => String::new(),
        Some(Json::Object(m)) => text(m.get("name").or(m.get("label")).or(m.get("id"))),
        Some(other) => expr::stringify(other),
    }
}
/// The lanes, as `kanban.js` finds them: `lanes` in order, then the cards' values as they
/// come, the cards without a value last. Each is its key and its name.
fn lanes_of(cx: &RenderCx<'_>, node: &Node, field: &str, cards: &[Json]) -> Result<Vec<(String, String)>, RenderError> {
    let mut out: Vec<(String, String)> = Vec::new();
    let named = |k: &str, v: Option<&Json>| -> String {
        if k.is_empty() {
            return cx.label_with("No {field}", &[("field", field)]);
        }
        match v {
            Some(Json::Object(m)) => text(m.get("name").or(m.get("label")).or(m.get("id"))),
            _ => k.to_string(),
        }
    };
    match cx.prop(node, "lanes") {
        None | Some(Json::Null) => {}
        Some(Json::Array(a)) => {
            for l in &a {
                let (v, label) = match l {
                    Json::Object(m) if m.contains_key("value") => (m.get("value"), m.get("label").map(expr::stringify)),
                    other => (Some(other), None),
                };
                let k = lane_key(v);
                if !out.iter().any(|(x, _)| *x == k) {
                    out.push((k.clone(), label.unwrap_or_else(|| named(&k, v))));
                }
            }
        }
        Some(_) => return Err(RenderError::new(node, "`lanes` must be a list")),
    }
    for c in cards {
        let v = lookup(c, field);
        let k = lane_key(v);
        if !out.iter().any(|(x, _)| *x == k) {
            out.push((k.clone(), named(&k, v)));
        }
    }
    let (some, none): (Vec<_>, Vec<_>) = out.into_iter().partition(|(k, _)| !k.is_empty());
    Ok(some.into_iter().chain(none).collect())
}

/// A column's header: its grip (`reorderable`), dot, title, count of every card in it,
/// and fold button.
fn column_head(cx: &mut RenderCx<'_>, node: &Node, look: &Look<'_>, col: &Json, count: usize, collapsed: &[String]) -> Result<(), RenderError> {
    let (id, title) = (col_id(col), col_title(col));
    let limit = col.get("limit").filter(|l| !l.is_null()).map(expr::to_num);
    let folded = collapsed.contains(&id);
    let mut head = Element::new("header");
    head.attr("data-kb-head", true);
    cx.open(&head, node)?;
    if look.reorderable {
        let mut g = Element::new("button");
        g.attr("type", "button")
            .attr("data-kb-grip", id.clone())
            .attr("draggable", "true")
            .attr("aria-label", cx.label_with("Move column {name}", &[("name", &title)]))
            .attr("title", cx.label("Drag, or press Left or Right, to move the column"));
        cx.open(&g, node)?;
        cx.markup(GRIP);
        cx.close("button");
    }
    let mut dot = Element::new("span");
    dot.attr("data-kb-dot", true).attr("aria-hidden", "true");
    cx.open(&dot, node)?;
    cx.close("span");
    let mut h = Element::new("h3");
    h.text(lit(title.clone()));
    cx.open(&h, node)?;
    cx.close("h3");
    let mut n = Element::new("span");
    n.attr("data-kb-count", true).text(lit(match limit {
        Some(l) => format!("{count} / {l}"),
        None => count.to_string(),
    }));
    cx.open(&n, node)?;
    cx.close("span");
    if look.collapsible {
        let mut b = Element::new("button");
        b.attr("type", "button")
            .attr("data-kb-fold", true)
            .attr("aria-expanded", if folded { "false" } else { "true" })
            .attr("aria-label", cx.label_with(if folded { "Show {name}" } else { "Hide {name}" }, &[("name", &title)]));
        cx.open(&b, node)?;
        cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8 10 12.5"/></svg>"#);
        cx.close("button");
    }
    cx.close("header");
    Ok(())
}

/// A column's cards in one lane (or all of them, `lane` none): its header too when
/// `with_head`, the list, the empty note and Add card.
#[allow(clippy::too_many_arguments)]
fn column_cell(
    cx: &mut RenderCx<'_>,
    node: &Node,
    look: &Look<'_>,
    col: &Json,
    lane: Option<(&String, &String)>,
    mine: &[&Json],
    in_column: usize,
    collapsed: &[String],
    with_head: bool,
    first: &mut bool,
) -> Result<(), RenderError> {
    let (id, title) = (col_id(col), col_title(col));
    let limit = col.get("limit").filter(|l| !l.is_null()).map(expr::to_num);
    let name = lane.map_or(title.clone(), |(_, n)| format!("{title}, {n}"));
    let mut section = Element::new("section");
    section.attr("data-kb-column", id.clone()).attr("aria-label", format!("{name}, {}", cx.label_count("{n} card", "{n} cards", mine.len() as f64)));
    if let Some((k, _)) = lane {
        section.attr("data-kb-lane", k.clone());
    }
    if let Some(t) = col.get("tone").filter(|t| !t.is_null()) {
        section.attr("data-tone", expr::stringify(t));
    }
    if limit.is_some_and(|l| in_column as f64 > l) {
        section.attr("data-over", true);
    }
    if collapsed.contains(&id) {
        section.attr("data-collapsed", true);
    }
    cx.open(&section, node)?;
    if with_head {
        column_head(cx, node, look, col, in_column, collapsed)?;
    }
    let mut list = Element::new("div");
    list.attr("data-kb-cards", true).attr("role", "list");
    cx.open(&list, node)?;
    for c in mine {
        card(cx, node, c, look.movable, *first, look.fields)?;
        *first = false;
    }
    if mine.is_empty() {
        let mut p = Element::new("p");
        p.attr("data-kb-empty", true).text(lit(look.empty.clone()));
        cx.open(&p, node)?;
        cx.close("p");
    }
    cx.close("div");
    if look.addable {
        let mut b = Element::new("button");
        b.attr("type", "button").attr("data-kb-add", true);
        cx.open(&b, node)?;
        cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#);
        let mut l = Element::new("span");
        l.text(look.add.clone());
        cx.open(&l, node)?;
        cx.close("span");
        cx.close("button");
    }
    cx.close("section");
    Ok(())
}

/// One card, as `kanban.js` draws it.
fn card(cx: &mut RenderCx<'_>, node: &Node, c: &Json, movable: bool, first: bool, fields: &[Json]) -> Result<(), RenderError> {
    let title = text(c.get("title"));
    let who = c.get("assignee").filter(|a| !a.is_null() && *a != &json!(""));
    let who_name = who.map(|w| if w.is_object() { text(w.get("name")) } else { expr::stringify(w) });
    let priority = c.get("priority").filter(|p| !p.is_null()).map(expr::stringify);
    let due = c.get("due").filter(|p| !p.is_null()).map(expr::stringify);
    let label: Vec<String> = [
        Some(title.clone()),
        priority.as_ref().map(|p| cx.label_with("{priority} priority", &[("priority", p)])),
        due.as_ref().map(|d| cx.label_with("due {date}", &[("date", d)])),
        who_name.as_ref().filter(|n| !n.is_empty()).map(|n| cx.label_with("assigned to {name}", &[("name", n)])),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut a = Element::new("article");
    a.attr("data-kb-card", text(c.get("id")))
        .attr("tabindex", lit(if first { 0 } else { -1 }))
        .attr("aria-roledescription", cx.label("card"))
        .attr("aria-label", label.join(", "))
        .attr("role", "listitem");
    if movable {
        a.attr("draggable", "true");
    }
    cx.open(&a, node)?;
    let mut t = Element::new("div");
    t.attr("data-kb-title", true).text(lit(title));
    cx.open(&t, node)?;
    cx.close("div");
    if let Some(d) = c.get("description").filter(|d| !d.is_null() && *d != &json!("")) {
        let mut p = Element::new("p");
        p.attr("data-kb-desc", true).text(lit(expr::stringify(d)));
        cx.open(&p, node)?;
        cx.close("p");
    }
    // `fields`: a line each (dates as written; the script writes them in the locale).
    let shown: Vec<(&Json, &Json)> = fields.iter().filter_map(|f| lookup(c, &text(f.get("field"))).filter(|v| !v.is_null() && **v != json!("")).map(|v| (f, v))).collect();
    if !shown.is_empty() {
        let mut box_ = Element::new("div");
        box_.attr("data-kb-fields", true);
        cx.open(&box_, node)?;
        for (f, v) in shown {
            let field = text(f.get("field"));
            let mut row = Element::new("div");
            row.attr("data-kb-field", field.clone());
            cx.open(&row, node)?;
            if let Some(l) = f.get("label").filter(|l| !l.is_null()) {
                let mut s = Element::new("span");
                s.attr("data-kb-field-label", true).text(lit(expr::stringify(l)));
                cx.open(&s, node)?;
                cx.close("span");
            }
            let value = expr::stringify(v);
            match f.get("cell").map(expr::stringify).as_deref() {
                Some("badge") => {
                    let tone = f.get("tones").and_then(|t| t.get(&value)).map(expr::stringify).unwrap_or_else(|| "neutral".into());
                    let mut s = Element::new("span");
                    s.attr("data-kb-badge", true).attr("data-tone", tone).text(lit(value));
                    cx.open(&s, node)?;
                    cx.close("span");
                }
                Some("progress") => {
                    let n = expr::to_num(v).clamp(0.0, 100.0);
                    let mut bar = Element::new("span");
                    bar.attr("data-kb-progress", true)
                        .attr("role", "progressbar")
                        .attr("aria-valuenow", lit(n))
                        .attr("aria-valuemin", "0")
                        .attr("aria-valuemax", "100")
                        .attr("aria-label", f.get("label").map(expr::stringify).unwrap_or_else(|| field.clone()));
                    cx.open(&bar, node)?;
                    let mut fill = Element::new("span");
                    fill.attr("style", format!("width: {n}%"));
                    cx.open(&fill, node)?;
                    cx.close("span");
                    cx.close("span");
                    let mut s = Element::new("span");
                    s.attr("data-kb-field-value", true).text(lit(format!("{n}%")));
                    cx.open(&s, node)?;
                    cx.close("span");
                }
                _ => {
                    let mut s = Element::new("span");
                    s.attr("data-kb-field-value", true).text(lit(value));
                    cx.open(&s, node)?;
                    cx.close("span");
                }
            }
            cx.close("div");
        }
        cx.close("div");
    }
    if let Some(Json::Array(tags)) = c.get("tags").filter(|t| t.as_array().is_some_and(|a| !a.is_empty())) {
        let mut box_ = Element::new("div");
        box_.attr("data-kb-tags", true);
        cx.open(&box_, node)?;
        for tag in tags {
            let (label, tone) = match tag {
                Json::Object(m) => (text(m.get("label").or(m.get("text"))), m.get("tone").map(expr::stringify).unwrap_or_else(|| "neutral".into())),
                other => (expr::stringify(other), "neutral".into()),
            };
            let mut s = Element::new("span");
            s.attr("data-tone", tone).text(lit(label));
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("div");
    }
    if priority.is_some() || due.is_some() || who.is_some() {
        let mut f = Element::new("div");
        f.attr("data-kb-foot", true);
        cx.open(&f, node)?;
        if let Some(p) = &priority {
            let tone = PRIORITY.iter().find(|(k, _)| *k == p.to_lowercase()).map_or("neutral", |(_, t)| t);
            let mut s = Element::new("span");
            s.attr("data-kb-priority", true).attr("data-tone", tone).text(lit(p.clone()));
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(d) = &due {
            let mut s = Element::new("span");
            s.attr("data-kb-due", true).text(lit(d.clone()));
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(w) = who {
            let mut s = Element::new("span");
            s.attr("data-kb-avatar", true).attr("title", who_name.clone().unwrap_or_default());
            let avatar = w.get("avatar").filter(|a| !a.is_null()).map(expr::stringify);
            if avatar.is_none() {
                let initials = w.get("initials").map(expr::stringify).unwrap_or_else(|| initials_of(who_name.as_deref().unwrap_or("")));
                s.text(lit(initials));
            }
            cx.open(&s, node)?;
            if let Some(src) = avatar {
                let mut img = Element::new("img");
                img.attr("src", src).attr("alt", "");
                cx.open(&img, node)?;
            }
            cx.close("span");
        }
        cx.close("div");
    }
    cx.close("article");
    Ok(())
}
