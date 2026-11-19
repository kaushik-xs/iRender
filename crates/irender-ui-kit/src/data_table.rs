//! `ui-data-table`: a table over an array of rows with sorting, filters, search,
//! paging, row selection with bulk actions, expandable detail rows, column
//! visibility and summary rows. Columns are `ui-data-table-column` children (whose
//! own children, if any, draw the cell with the row in scope as `row`) or plain
//! objects in `columns`.
//!
//! The server renders every row, in the starting order and with the first page
//! shown, so the table reads without JavaScript. `data_table.js` (shipped once per
//! page through `global_js`) then sorts, filters and pages by moving and hiding the
//! rendered rows rather than drawing them again, so anything inside a cell (a badge,
//! a button, a chart) keeps working. Rows from state are rebuilt by the runtime when
//! the array changes, and the behaviour arranges them again.

use crate::{
    PRIMARY_FILL, BG_POPOVER, BG_UNDER, BORDER, MUTED, PRIMARY, RING, SUBTLE, SURFACE, TEXT, bin, cond, expr_val, forms, id_with, instance_id, lit,
    server_array, synthetic,
};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr},
};
use irender_html::{ClientList, Component, Element, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::{cmp::Ordering, sync::LazyLock};

const SCRIPT: &str = include_str!("data_table.js");

/// The names a row and its position are in scope under inside cell templates.
const ROW: &str = "row";
const ROW_INDEX: &str = "rowIndex";

const FILTERS: &[&str] = &["text", "select", "range", "date"];
const FORMATS: &[&str] = &["number", "currency", "percent", "date", "datetime"];
const SUMMARIES: &[&str] = &["sum", "avg", "min", "max", "count"];
const CELLS: &[&str] = &["text", "badge", "progress", "sparkline"];
const PAGING: &[&str] = &["pages", "more", "scroll", "none"];
const TONES: &[&str] = &["default", "secondary", "outline", "success", "warning", "destructive", "info"];

pub struct DataTable;

const PROPS: &[PropDef] = &[
    PropDef::new("rows", PropKind::Any, "The rows: an array of objects, from data or state (or use `url`)"),
    PropDef::new(
        "url",
        PropKind::Any,
        "Load rows from an API in batches: GET `<url>?offset=&limit=&sort=&q=&filters=`, answering `{ rows, total }` or an array",
    ),
    PropDef::new("params", PropKind::Any, "Extra query parameters for `url`; may use state, and a change reloads"),
    PropDef::new(
        "columns",
        PropKind::Any,
        "Columns as objects with the `ui-data-table-column` props, for tables with no cell templates",
    ),
    PropDef::new("rowKey", PropKind::String, "Field that identifies a row, for selection (default `id`; else its position)"),
    PropDef::new("selectable", PropKind::Bool, "A checkbox per row and a select-all box in the header"),
    PropDef::new("search", PropKind::Any, "A search box above the table: `true`, or its placeholder text"),
    PropDef::new("searchFields", PropKind::Any, "Fields the search looks in (default: every column's field)"),
    PropDef::new("sort", PropKind::Any, "Starting sort: `field`, `-field` for descending, or a list of them"),
    PropDef::new(
        "filter",
        PropKind::Any,
        "Filters from outside the table, e.g. a chart's `bind`: `{ field: condition }`, as `ui-chart`'s `filter`",
    ),
    PropDef::new(
        "paging",
        PropKind::Enum(PAGING),
        "`pages` (default): a pager; `more`: a Load more button; `scroll`: the next batch as you reach the bottom; `none`",
    ),
    PropDef::new("pageSize", PropKind::Number, "Rows per page, or per batch with `more` and `scroll` (default 10; 0 shows every row)"),
    PropDef::new("childrenField", PropKind::String, "Rows nest: the field holding a row's child rows (default `children`); `true` there means load them from `childrenUrl`"),
    PropDef::new("parentField", PropKind::String, "Rows nest: flat rows name their parent's key in this field, e.g. `parentId`"),
    PropDef::new(
        "childrenUrl",
        PropKind::Any,
        "Load a row's children when it's first opened: GET `<url>?parent=<key>`, answering an array or `{ rows }`",
    ),
    PropDef::new("expanded", PropKind::Bool, "Nested rows start open (default false)"),
    PropDef::new("virtual", PropKind::Bool, "Keep only the rows in view in the page, for long tables; scrolls inside `height` (default 480)"),
    PropDef::new("rowHeight", PropKind::Number, "With `virtual`, a row's height to start from before rows are measured (default 41)"),
    PropDef::new("pageSizes", PropKind::Any, "Choices for rows per page (default `[10, 20, 50, 100]`; `[]` hides it)"),
    PropDef::new("columnsMenu", PropKind::Bool, "A View menu that shows and hides columns (default true)"),
    PropDef::new("groupBy", PropKind::Any, "Groups the rows by this field, each group under a header row with its count and subtotals (may use state)"),
    PropDef::new("bindGroupBy", PropKind::StateKey, "State key holding the field the rows are grouped by (or null), set from the View menu"),
    PropDef::new("resizable", PropKind::Bool, "Columns can be made wider or narrower by dragging their header's edge"),
    PropDef::new("reorderable", PropKind::Bool, "Columns can be moved by dragging their header, or Alt and the arrow keys on it"),
    PropDef::new("bindColumns", PropKind::StateKey, "State key holding the columns' layout, `{ order, widths, hidden }`, to keep it"),
    PropDef::new("export", PropKind::Any, "An Export button that downloads the rows that pass the filters as CSV: `true`, or the file's name"),
    PropDef::new("title", PropKind::Any, "Accessible name of the table"),
    PropDef::new("emptyText", PropKind::Any, "Shown when no row matches (default `No results.`)"),
    PropDef::new("locale", PropKind::String, "Locale for formatted numbers and dates (default: the document's `lang`, else the browser's)"),
    PropDef::new("height", PropKind::Any, "Most the table grows to before it scrolls, with the header kept in view"),
    PropDef::new("striped", PropKind::Bool, "Shade every other row"),
    PropDef::new("loading", PropKind::Any, "Dim the rows and mark the table busy, e.g. `{{ fetching }}`"),
    PropDef::new("bindSelection", PropKind::StateKey, "State key holding the selected rows' keys (a list)"),
    PropDef::new("bindSort", PropKind::StateKey, "State key holding the sort: `field`, `-field`, a list of them, or null"),
    PropDef::new(
        "bindFilters",
        PropKind::StateKey,
        "State key holding the column filters, `{ field: condition }`: give it to a chart's `filter` to filter both",
    ),
    PropDef::new("bindSearch", PropKind::StateKey, "State key holding the search text"),
    PropDef::new("bindPage", PropKind::StateKey, "State key holding the page number, from 1"),
    PropDef::new(
        "bindView",
        PropKind::StateKey,
        "State key that gets the rows that pass the filters and search, sorted, every page: a chart's `data`",
    ),
    PropDef::new("bindRow", PropKind::StateKey, "State key that gets the row last clicked (or opened with Enter)"),
    PropDef::new(
        "views",
        PropKind::Any,
        "Ready-made views in a Views menu: `[{ id, name, sort, filters, search, columns, groupBy, pageSize }]`; they can't be changed or deleted",
    ),
    PropDef::new("bindViews", PropKind::StateKey, "State key for the views people save from the Views menu (the same shape as `views`)"),
    PropDef::new("bindCurrentView", PropKind::StateKey, "State key holding the id of the view shown (or null); the table opens with it"),
    PropDef::new("defaultView", PropKind::String, "The view the table opens with when `bindCurrentView` has none"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

const COLUMN_PROPS: &[PropDef] = &[
    PropDef::new("field", PropKind::String, "The row field shown, sorted and filtered, e.g. `amount` or `customer.name`"),
    PropDef::new("label", PropKind::Any, "Header text (default: the field)"),
    PropDef::new("sortable", PropKind::Bool, "Click the header to sort (default: when it has a `field` or `sortField`)"),
    PropDef::new("sortField", PropKind::String, "Sort by this field instead, e.g. a rank behind a label (default: `field`)"),
    PropDef::new("filter", PropKind::Enum(FILTERS), "A filter control in the toolbar (`date`: a range calendar with presets)"),
    PropDef::new("options", PropKind::Any, "A `select` filter's choices (default: the column's distinct values)"),
    PropDef::new(
        "presets",
        PropKind::Any,
        "A `date` filter's shortcuts, as `ui-calendar`'s: names such as `lastQuarter`, `yearToDate` or `quarters` (the last four), or `{ label, from, to }`",
    ),
    PropDef::new("fiscalStart", PropKind::Number, "A `date` filter's financial year: the month it starts, 1–12; quarters and years follow it"),
    PropDef::new("align", PropKind::Enum(&["left", "center", "right"]), "Default: right for numbers, left otherwise"),
    PropDef::new("format", PropKind::Enum(FORMATS), "How the browser shows the value (in `locale`)"),
    PropDef::new("currency", PropKind::String, "With `format: currency`, e.g. `EUR` (default `USD`)"),
    PropDef::new("decimals", PropKind::Number, "Digits after the point for `number`, `currency` and `percent`"),
    PropDef::new("cell", PropKind::Enum(CELLS), "Draw the value as `badge`, `progress` (0–100) or `sparkline` (needs the chart kit)"),
    PropDef::new("tones", PropKind::Any, "With `cell: badge`, a tone per value: `{ Paid: success, Failed: destructive }`"),
    PropDef::new("summary", PropKind::Enum(SUMMARIES), "A footer cell summing (or averaging, …) the rows that pass the filters"),
    PropDef::new("width", PropKind::Any, "Column width"),
    PropDef::new("pin", PropKind::Enum(&["left", "right"]), "Keep the column in view while the table scrolls sideways"),
    PropDef::new("tree", PropKind::Bool, "The column that shows nesting: indented, with the arrow that opens a row (default: the first)"),
    PropDef::new("hideable", PropKind::Bool, "Listed in the View menu (default true)"),
    PropDef::new("hidden", PropKind::Bool, "Starts hidden"),
    PropDef::new("group", PropKind::Any, "A header over this column and the ones next to it with the same `group`"),
    PropDef::new("groupable", PropKind::Bool, "Offered in the View menu's Group by"),
    PropDef::new("resizable", PropKind::Bool, "With the table's `resizable`, false keeps this column's width"),
    PropDef::new("id", PropKind::String, "The column's key when it has no field (default: its position)"),
];

// ---------------------------------------------------------------- styles

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const TINT: &str = "color-mix(in srgb, var(--color-primary) 8%, transparent)";

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(12)), ("min-width", "0"), ("font-size", fs!(14)), ("color", TEXT)])
        // Toolbar: search, filters, reset, then the document's own controls and View.
        .nest("& [data-dt-toolbar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", sp!(8))]))
        // Buttons set their own `display`, which would show them while `hidden`.
        .nest("& button[hidden]", pairs(&[("display", "none")]))
        .nest("& [data-dt-spacer]", pairs(&[("flex", "1"), ("min-width", "0")]))
        .nest(
            "& input[type=search], & [data-dt-pop] input",
            pairs(&[
                ("box-sizing", "border-box"),
                ("height", sp!(32)),
                ("padding", sp!(0, 10)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("font-size", fs!(14)),
            ]),
        )
        .nest("& [data-dt-search]", pairs(&[("width", "min(100%, 260px)")]))
        .nest(
            "& input:focus-visible",
            pairs(&[
                ("outline", "none"),
                ("border-color", RING),
                ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
            ]),
        )
        .nest("& [data-dt-trigger]", pairs(&[("border-style", "dashed")]))
        .nest("& [data-dt-trigger][data-active]", pairs(&[("border-style", "solid")]))
        .nest(
            "& [data-dt-chip]",
            pairs(&[
                ("padding", sp!(2, 6)),
                ("border-radius", "var(--radius-sm)"),
                ("background", SUBTLE),
                ("font-size", fs!(12)),
                ("font-weight", "400"),
            ]),
        )
        .nest("& [data-dt-chips]:empty", pairs(&[("display", "none")]))
        .nest(
            "& [data-dt-chips]",
            pairs(&[("display", "inline-flex"), ("gap", sp!(4)), ("padding-inline-start", sp!(8)), ("border-inline-start", "1px solid var(--color-border)")]),
        )
        // Popovers: a filter's control, or the View menu's columns.
        .nest(
            "& [data-dt-pop]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("inset", "auto"),
                ("margin", "6px 0"),
                ("min-width", sp!(200)),
                ("max-height", "min(360px, 70vh)"),
                ("overflow-y", "auto"),
                ("padding", sp!(4)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("--ui-bg", BG_POPOVER),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("font-size", fs!(14)),
                ("box-shadow", "0 6px 18px rgb(0 0 0 / 0.12)"),
                ("position-area", "bottom span-right"),
                ("position-try-fallbacks", "flip-block, flip-inline"),
            ])
            .nest("&:dir(rtl)", pairs(&[("position-area", "bottom span-left")])),
        )
        .nest("& [data-dt-pop]:popover-open", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(2))]))
        .nest(
            "& [data-dt-pop] label",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", sp!(8)),
                ("padding", sp!(6, 8)),
                ("border-radius", "var(--radius-sm)"),
                ("cursor", "pointer"),
                ("user-select", "none"),
            ]),
        )
        .nest("& [data-dt-pop] label:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-option] [data-dt-n]", pairs(&[("margin-inline-start", "auto"), ("padding-inline-start", sp!(16)), ("color", MUTED), ("font-size", fs!(12))]))
        .nest(
            "& [data-dt-fields]",
            pairs(&[("display", "grid"), ("grid-template-columns", "auto 1fr"), ("align-items", "center"), ("gap", sp!(6, 8)), ("padding", sp!(8))]),
        )
        .nest("& [data-dt-fields] label", pairs(&[("padding", "0"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-dt-fields] label:hover", pairs(&[("background", "none")]))
        .nest("& [data-dt-fields] input", pairs(&[("width", "100%"), ("min-width", "0")]))
        .nest(
            "& [data-dt-pop] [data-dt-clear]",
            pairs(&[
                ("margin-top", sp!(2)),
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-top", "1px solid var(--color-border)"),
                ("border-radius", "0"),
                ("background", "none"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("text-align", "center"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-dt-pop] [data-dt-clear]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-pop-title]", pairs(&[("padding", sp!(6, 8)), ("font-size", fs!(12)), ("font-weight", "600"), ("color", MUTED)]))
        .nest("& [data-dt-none]", pairs(&[("padding", sp!(8)), ("color", MUTED), ("text-align", "center")]))
        // The Views menu: a row per view, its name to pick it, and rename and delete.
        .nest("& [data-dt-views]", pairs(&[("min-width", sp!(240))]))
        .nest("& [data-dt-view]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(2)), ("border-radius", "var(--radius-sm)")]))
        .nest("& [data-dt-view]:hover", pairs(&[("background", SUBTLE)]))
        .nest(
            "& :is([data-dt-view] button, [data-dt-view-actions] button)",
            pairs(&[("border", "0"), ("background", "none"), ("color", "inherit"), ("font", "inherit"), ("cursor", "pointer"), ("border-radius", "var(--radius-sm)")]),
        )
        .nest(
            "& [data-dt-view-pick]",
            pairs(&[("flex", "1"), ("min-width", "0"), ("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding", sp!(6, 8)), ("text-align", "start"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]),
        )
        .nest("& [data-dt-view-pick] [data-dt-tick]", pairs(&[("flex", "none"), ("width", "14px"), ("display", "grid")]))
        .nest("& [data-dt-view-pick][aria-current=true]", pairs(&[("font-weight", "600")]))
        .nest("& :is([data-dt-view-rename], [data-dt-view-delete])", pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "28px"), ("height", "28px"), ("color", MUTED)]))
        .nest("& :is([data-dt-view-rename], [data-dt-view-delete]):hover", pairs(&[("color", TEXT), ("background", BG_POPOVER)]))
        .nest("& [data-dt-view] :focus-visible, & [data-dt-view-actions] :focus-visible", forms::focus_ring())
        .nest(
            "& [data-dt-view-actions]",
            pairs(&[("display", "flex"), ("flex-direction", "column"), ("margin-top", sp!(2)), ("padding-top", sp!(2)), ("border-top", "1px solid var(--color-border)")]),
        )
        .nest("& [data-dt-view-actions] button", pairs(&[("padding", sp!(6, 8)), ("text-align", "start")]))
        .nest("& [data-dt-view-actions] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-view-name]", pairs(&[("margin", sp!(4)), ("min-width", "0")]))
        .nest("& [data-dt-views-edited]", pairs(&[("color", MUTED), ("font-weight", "400")]))
        // Bulk actions, shown while rows are selected.
        .nest(
            "& [data-dt-bulk]",
            pairs(&[
                ("display", "flex"),
                ("flex-wrap", "wrap"),
                ("align-items", "center"),
                ("gap", sp!(8)),
                ("padding-block", sp!(6, 6)), ("padding-inline", sp!(12, 8)),
                ("border", "1px solid color-mix(in srgb, var(--color-primary) 30%, var(--color-border))"),
                ("border-radius", "var(--radius-md)"),
                ("background", TINT),
            ]),
        )
        .nest("& [data-dt-bulk][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-dt-count]", pairs(&[("font-weight", "500")]))
        .nest(
            "& [data-dt-link]",
            pairs(&[
                ("padding", "0"),
                ("border", "0"),
                ("background", "none"),
                ("color", PRIMARY),
                ("font", "inherit"),
                ("text-decoration", "underline"),
                ("text-underline-offset", "3px"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-dt-link][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-dt-bulk-actions]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", sp!(8)), ("margin-inline-start", "auto")]))
        // The table.
        .nest(
            "& [data-dt-scroll]",
            pairs(&[
                ("position", "relative"),
                ("overflow", "auto"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("transition", "opacity .15s"),
            ]),
        )
        .nest("&[aria-busy=true] [data-dt-scroll]", pairs(&[("opacity", "0.55"), ("pointer-events", "none")]))
        .nest("& table", pairs(&[("width", "100%"), ("border-collapse", "separate"), ("border-spacing", "0")]))
        .nest(
            "& th",
            pairs(&[
                ("position", "sticky"),
                ("top", "0"),
                ("z-index", "1"),
                ("height", sp!(40)),
                ("padding", sp!(0, 8)),
                ("background", "color-mix(in srgb, var(--color-text) 3%, var(--ui-bg, var(--color-background, var(--color-surface))))"),
                ("color", MUTED),
                ("font-weight", "500"),
                ("text-align", "start"),
                ("white-space", "nowrap"),
                ("border-bottom", "1px solid var(--color-border)"),
            ]),
        )
        .nest(
            "& td",
            pairs(&[("padding", sp!(8)), ("vertical-align", "middle"), ("white-space", "nowrap"), ("border-bottom", "1px solid var(--color-border)")]),
        )
        .nest("& [data-dt-detail] > td, & [data-dt-empty] > td", pairs(&[("white-space", "normal")]))
        // Bands over groups of columns, and the column row under them.
        .nest("& [data-dt-bands] > th", pairs(&[("height", sp!(30)), ("text-align", "center"), ("color", TEXT), ("font-size", fs!(12)), ("font-weight", "600")]))
        .nest("& [data-dt-bands] > th[data-dt-band]:not([data-dt-band=''])", pairs(&[("box-shadow", "inset 1px 0 var(--color-border), inset -1px 0 var(--color-border)")]))
        .nest("& thead > tr:not([data-dt-bands]) > th", pairs(&[("top", "var(--dt-band-h, 0px)")]))
        // Resizing and moving columns.
        .nest(
            "& [data-dt-resizer]",
            pairs(&[("position", "absolute"), ("z-index", "2"), ("top", "0"), ("bottom", "0"), ("inset-inline-end", "0"), ("width", "7px"), ("cursor", "col-resize"), ("touch-action", "none"), ("outline", "none")]),
        )
        .nest("& [data-dt-resizer]::after", pairs(&[("content", "''"), ("position", "absolute"), ("top", "8px"), ("bottom", "8px"), ("inset-inline-end", "3px"), ("width", "1px")]))
        .nest("& th:hover > [data-dt-resizer]::after", pairs(&[("background", "var(--color-border)")]))
        .nest(
            "& [data-dt-resizer]:is(:hover, :focus-visible)::after, &[data-dt-resizing] [data-dt-resizer]::after",
            pairs(&[("top", "0"), ("bottom", "0"), ("inset-inline-end", "2px"), ("width", "3px"), ("background", "var(--color-ring, var(--color-muted))")]),
        )
        .nest("&[data-dt-resizing]", pairs(&[("cursor", "col-resize"), ("user-select", "none")]))
        .nest("& th[data-dt-moving]", pairs(&[("opacity", "0.5")]))
        .nest("& th[data-dt-drop=before]", pairs(&[("box-shadow", "inset 3px 0 var(--color-primary)")]))
        .nest("& th[data-dt-drop=after]", pairs(&[("box-shadow", "inset -3px 0 var(--color-primary)")]))
        .nest("&:dir(rtl) th[data-dt-drop=before]", pairs(&[("box-shadow", "inset -3px 0 var(--color-primary)")]))
        .nest("&:dir(rtl) th[data-dt-drop=after]", pairs(&[("box-shadow", "inset 3px 0 var(--color-primary)")]))
        // Group rows: the value, a count, subtotals; a click folds the group.
        // Group rows from the API: shaded like a group's header, with nothing to select.
        .nest("& [data-dt-group-row] > td", pairs(&[("background", "color-mix(in srgb, var(--color-text) 4%, var(--ui-bg, var(--color-surface)))"), ("font-weight", "600")]))
        .nest("& :is([data-dt-group-row], [data-dt-more-row]) :is([data-dt-select], [data-dt-expand])", pairs(&[("visibility", "hidden")]))
        // After a group's rows, the button that loads the next batch.
        .nest(
            "& [data-dt-load-group]",
            pairs(&[("padding", sp!(2, 6)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", PRIMARY), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer"), ("white-space", "nowrap")]),
        )
        .nest("& [data-dt-load-group]:hover", pairs(&[("background", SUBTLE), ("text-decoration", "underline")]))
        .nest("& [data-dt-load-group]:disabled", pairs(&[("color", MUTED), ("cursor", "progress"), ("text-decoration", "none")]))
        .nest("& [data-dt-more-row] > td", pairs(&[("padding-block", sp!(4))]))
        // Its other cells are a row template's with no row: empty, and kept so.
        .nest("& [data-dt-more-row] > td:not([data-dt-tree]) > *", pairs(&[("visibility", "hidden")]))
        .nest("& [data-dt-group-row]", pairs(&[("cursor", "pointer")]))
        .nest("& [data-dt-group] > td", pairs(&[("padding", sp!(6, 8)), ("background", "color-mix(in srgb, var(--color-text) 4%, var(--ui-bg, var(--color-surface)))"), ("font-weight", "600")]))
        .nest(
            "& [data-dt-fold]",
            pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", sp!(6)), ("padding-block", sp!(2, 2)), ("padding-inline", sp!(2, 6)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("cursor", "pointer")]),
        )
        .nest("& [data-dt-fold]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 7%, transparent)")]))
        .nest("& [data-dt-fold]:focus-visible", forms::focus_ring())
        .nest("& [data-dt-fold] [data-dt-chevron]", pairs(&[("transition", "transform .15s"), ("color", MUTED)]))
        .nest("& [data-dt-fold][aria-expanded=true] [data-dt-chevron]", pairs(&[("transform", "rotate(90deg)")]))
        .nest(
            "& [data-dt-group-count]",
            pairs(&[("min-width", "20px"), ("padding", sp!(0, 6)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "500"), ("text-align", "center")]),
        )
        .nest("& [data-dt-group-title]", pairs(&[("margin-top", sp!(6)), ("padding-top", sp!(8)), ("border-top", "1px solid var(--color-border)")]))
        .nest("& tfoot td", pairs(&[("border-bottom", "0"), ("border-top", "1px solid var(--color-border)"), ("font-weight", "600"), ("background", SUBTLE)]))
        .nest("& [data-dt-body] > tr:last-of-type > td", pairs(&[("border-bottom", "0")]))
        .nest("& [data-dt-body]:has(> tr:not([hidden])) + [data-dt-empty-body] td", pairs(&[("border-top", "1px solid var(--color-border)")]))
        .nest("& [data-align=right]", pairs(&[("text-align", "end"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-align=center]", pairs(&[("text-align", "center")]))
        .nest("& th[data-align=right] [data-dt-sort]", pairs(&[("flex-direction", "row-reverse")]))
        .nest("& [data-dt-row]:hover", pairs(&[("background", SUBTLE)]))
        .nest("&[data-striped] [data-dt-row][data-odd]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 2.5%, transparent)")]))
        .nest("& [data-dt-row][aria-selected=true]", pairs(&[("background", TINT)]))
        .nest("&[data-clickable] [data-dt-row]", pairs(&[("cursor", "pointer")]))
        .nest(
            "& [data-dt-row]:focus-visible",
            pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]),
        )
        .nest("& [data-dt-detail] > td", pairs(&[("padding", sp!(12, 16)), ("background", SUBTLE)]))
        .nest("& [data-dt-box]", pairs(&[("width", sp!(36)), ("padding-inline-end", "0")]))
        .nest("& [data-dt-box] input", pairs(&[("display", "block")]))
        .nest("& [data-dt-empty] td", pairs(&[("height", sp!(96)), ("text-align", "center"), ("color", MUTED), ("border-bottom", "0")]))
        // Sortable headers.
        .nest(
            "& [data-dt-sort]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("gap", sp!(4)),
                ("margin", sp!(0, -8)),
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-dt-sort]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("color", TEXT)]))
        .nest(
            "& [data-dt-sort]:focus-visible",
            pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]),
        )
        .nest("& [data-dt-arrow]", pairs(&[("display", "inline-flex"), ("opacity", "0.45")]))
        .nest("& [aria-sort] [data-dt-arrow]", pairs(&[("opacity", "1"), ("color", TEXT)]))
        .nest("& [aria-sort=descending] [data-dt-arrow] svg", pairs(&[("transform", "rotate(180deg)")]))
        .nest("& [data-dt-rank]", pairs(&[("font-size", fs!(10)), ("color", MUTED)]))
        // The expander.
        .nest(
            "& [data-dt-expand]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "24px"),
                ("height", "24px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-dt-expand]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 8%, transparent)")]))
        .nest("& [data-dt-expand] svg", pairs(&[("transition", "transform .15s")]))
        .nest("& [data-dt-expand][aria-expanded=true] svg", pairs(&[("transform", "rotate(90deg)")]))
        // Cell shorthands.
        .nest(
            "& [data-dt-badge]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("padding", sp!(2, 8)),
                ("border", "1px solid transparent"),
                ("border-radius", "var(--radius-md)"),
                ("font-size", fs!(12)),
                ("font-weight", "500"),
                ("white-space", "nowrap"),
            ]),
        )
        .nest("& [data-dt-badge=default]", pairs(&[("background", PRIMARY_FILL), ("color", "var(--color-on-primary)")]))
        .nest("& [data-dt-badge=secondary]", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-badge=outline]", pairs(&[("border-color", BORDER)]))
        .nest(
            "& [data-dt-badge=success]",
            pairs(&[("background", "color-mix(in srgb, #16a34a 14%, transparent)"), ("color", "color-mix(in srgb, #16a34a 80%, var(--color-text))")]),
        )
        .nest(
            "& [data-dt-badge=warning]",
            pairs(&[("background", "color-mix(in srgb, #d97706 16%, transparent)"), ("color", "color-mix(in srgb, #d97706 80%, var(--color-text))")]),
        )
        .nest(
            "& [data-dt-badge=destructive]",
            pairs(&[
                ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 14%, transparent)"),
                ("color", "color-mix(in srgb, var(--color-destructive, #dc2626) 85%, var(--color-text))"),
            ]),
        )
        .nest(
            "& [data-dt-badge=info]",
            pairs(&[("background", "color-mix(in srgb, #2563eb 14%, transparent)"), ("color", "color-mix(in srgb, #2563eb 80%, var(--color-text))")]),
        )
        .nest("& [data-dt-progress]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("min-width", "120px")]))
        .nest(
            "& [data-dt-progress] progress",
            pairs(&[
                ("appearance", "none"),
                ("flex", "1"),
                ("height", "6px"),
                ("border", "0"),
                ("border-radius", "999px"),
                ("overflow", "hidden"),
                ("background", "color-mix(in srgb, var(--color-primary) 15%, transparent)"),
                ("color", PRIMARY),
            ]),
        )
        .nest("& [data-dt-progress] progress::-webkit-progress-bar", pairs(&[("background", "transparent")]))
        .nest("& [data-dt-progress] progress::-webkit-progress-value", pairs(&[("background", PRIMARY_FILL), ("border-radius", "999px")]))
        .nest("& [data-dt-progress] progress::-moz-progress-bar", pairs(&[("background", PRIMARY_FILL), ("border-radius", "999px")]))
        .nest("& [data-dt-progress] span", pairs(&[("min-width", "3ch"), ("color", MUTED), ("font-size", fs!(12)), ("text-align", "end")]))
        // The footer: selection count, rows per page, page and the page buttons.
        .nest(
            "& [data-dt-footer]",
            pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", sp!(8, 24)), ("color", MUTED)]),
        )
        .nest("& [data-dt-status]", pairs(&[("flex", "1"), ("min-width", "max-content")]))
        .nest("& [data-dt-sizes]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", TEXT), ("font-weight", "500")]))
        .nest(
            "& [data-dt-sizes] select",
            pairs(&[
                ("height", sp!(32)),
                ("padding", sp!(0, 6)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", SURFACE),
                ("color", TEXT),
                ("font", "inherit"),
            ]),
        )
        .nest("& [data-dt-page]", pairs(&[("color", TEXT), ("font-weight", "500"), ("white-space", "nowrap")]))
        .nest("& [data-dt-nav]", pairs(&[("display", "flex"), ("gap", sp!(4))]))
        .nest("& [data-dt-nav] button", pairs(&[("width", sp!(32)), ("padding", "0")]))
        // Pinned columns stay put while the rest scroll sideways under them; the browser
        // sets each one's offset. They need a background of their own.
        .nest("& [data-pin]", pairs(&[("position", "sticky"), ("z-index", "1"), ("background", BG_UNDER)]))
        .nest("& th[data-pin]", pairs(&[("z-index", "3"), ("background", "color-mix(in srgb, var(--color-text) 3%, var(--ui-bg, var(--color-background, var(--color-surface))))")]))
        .nest("& tfoot [data-pin]", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-row]:hover > [data-pin]", pairs(&[("background", SUBTLE)]))
        .nest("& [data-dt-row][aria-selected=true] > [data-pin]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 8%, var(--ui-bg, var(--color-background, var(--color-surface))))")]))
        .nest("&[data-striped] [data-dt-row][data-odd] > [data-pin]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 2.5%, var(--ui-bg, var(--color-background, var(--color-surface))))")]))
        .nest("& [data-pin-edge=left]", pairs(&[("box-shadow", "inset -1px 0 0 var(--color-border), 6px 0 6px -6px rgb(0 0 0 / 0.25)")]))
        .nest("& [data-pin-edge=right]", pairs(&[("box-shadow", "inset 1px 0 0 var(--color-border), -6px 0 6px -6px rgb(0 0 0 / 0.25)")]))
        // Pinned to the start (`left`) is on the right of a right-to-left table.
        .nest("&:dir(rtl) [data-pin-edge=left]", pairs(&[("box-shadow", "inset 1px 0 0 var(--color-border), -6px 0 6px -6px rgb(0 0 0 / 0.25)")]))
        .nest("&:dir(rtl) [data-pin-edge=right]", pairs(&[("box-shadow", "inset -1px 0 0 var(--color-border), 6px 0 6px -6px rgb(0 0 0 / 0.25)")]))
        // Nested rows: the tree column's content steps in by depth.
        .nest(
            "& [data-dt-node]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(4)), ("padding-inline-start", "calc(var(--dt-depth, 0) * 20px)")]),
        )
        .nest(
            "& [data-dt-twisty]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("flex-shrink", "0"),
                ("width", "20px"),
                ("height", "20px"),
                ("margin-inline-start", "-4px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        // Rows without children keep the arrow's space, so siblings line up.
        .nest("& [data-dt-twisty][hidden]", pairs(&[("display", "inline-flex"), ("visibility", "hidden")]))
        .nest("& [data-dt-twisty]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 8%, transparent)")]))
        .nest("& [data-dt-twisty] svg", pairs(&[("transition", "transform .15s")]))
        .nest("& [aria-expanded=true] > [data-dt-tree] [data-dt-twisty] svg", pairs(&[("transform", "rotate(90deg)")]))
        .nest("& [data-dt-twisty][data-loading] svg", pairs(&[("opacity", "0.4")]))
        .nest(
            "& [data-dt-more]",
            pairs(&[("display", "flex"), ("justify-content", "center"), ("align-items", "center"), ("min-height", sp!(40)), ("padding", sp!(8)), ("color", MUTED)]),
        )
        .nest("& [data-dt-more][hidden], & [data-dt-more] [hidden]", pairs(&[("display", "none")]))
        .nest("& [data-dt-pad] td", pairs(&[("padding", "0"), ("border", "0")]))
        .nest(
            "& [data-dt-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
});

// Small inline icons, decorative: the buttons around them are labelled.
const CHEVRON: &str = r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#;
const SORT_ARROW: &str = r#"<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 13V3M4 7l4-4 4 4"/></svg>"#;
const SORT_BOTH: &str = r#"<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 6l3-3 3 3M5 10l3 3 3-3"/></svg>"#;
const PLUS: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><circle cx="8" cy="8" r="6"/><path d="M8 5.5v5M5.5 8h5"/></svg>"#;
const DOWNLOAD: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10"/></svg>"#;
const VIEWS: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.5 3.5h11M2.5 8h11M2.5 12.5h6"/><path d="m11 11 1.5 1.5L15 10"/></svg>"#;
const COLUMNS: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 4h12M2 8h12M2 12h12M5 2.5v11"/></svg>"#;
// Labels (looked up where they're used): "Go to first page", "Go to previous page", "Go to next page", "Go to last page"
const PAGE_BUTTONS: &[(&str, &str, &str)] = &[
    ("first", "Go to first page", r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8.5 3.5 4 8l4.5 4.5M12.5 3.5 8 8l4.5 4.5"/></svg>"#),
    ("prev", "Go to previous page", r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8l4.5 4.5"/></svg>"#),
    ("next", "Go to next page", r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#),
    ("last", "Go to last page", r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 3.5 8 8l-4.5 4.5M7.5 3.5 12 8l-4.5 4.5"/></svg>"#),
];

// ---------------------------------------------------------------- columns

/// A column, from a `ui-data-table-column` child or an object in `columns`.
struct Column<'n> {
    /// What the browser calls it: its field, `id`, or `c<position>`.
    key: String,
    field: Option<String>,
    sort_field: Option<String>,
    label: Val,
    label_text: String,
    sortable: bool,
    filter: Option<String>,
    options: Option<Json>,
    /// A date filter's calendar: its presets and the month its financial year starts.
    presets: Option<Json>,
    fiscal_start: Option<Json>,
    align: Option<&'static str>,
    format: Option<String>,
    currency: Option<String>,
    decimals: Option<f64>,
    cell: Option<String>,
    tones: Map<String, Json>,
    summary: Option<String>,
    width: Option<Val>,
    hideable: bool,
    hidden: bool,
    pin: Option<&'static str>,
    tree: bool,
    group: Option<String>,
    groupable: bool,
    resizable: bool,
    template: Option<&'n Node>,
}

/// Reads one column's props through `get`, which knows where they come from.
fn column<'n>(
    at: &Node,
    i: usize,
    get: &dyn Fn(&str) -> Option<Json>,
    label: Option<Val>,
    template: Option<&'n Node>,
) -> Result<Column<'n>, RenderError> {
    let string = |name: &str| get(name).filter(|v| !v.is_null()).map(|v| expr::stringify(&v));
    let flag = |name: &str, default: bool| get(name).filter(|v| !v.is_null()).map_or(default, |v| expr::truthy(&v));
    let one_of = |name: &str, allowed: &[&str]| -> Result<Option<String>, RenderError> {
        match string(name) {
            Some(v) if !allowed.contains(&v.as_str()) => {
                Err(RenderError::new(at, format!("column `{name}` must be one of {}, got `{v}`", allowed.join(", "))))
            }
            v => Ok(v),
        }
    };
    let field = string("field").filter(|f| !f.is_empty());
    let key = field.clone().or_else(|| string("id")).unwrap_or_else(|| format!("c{i}"));
    let sort_field = string("sortField").filter(|f| !f.is_empty()).or_else(|| field.clone());
    let label = label.or_else(|| get("label").filter(|v| !v.is_null()).map(Val::Lit)).unwrap_or_else(|| lit(field.clone().unwrap_or_default()));
    let format = one_of("format", FORMATS)?;
    let numeric = matches!(format.as_deref(), Some("number" | "currency" | "percent"));
    let align = match string("align").as_deref() {
        Some("right") => Some("right"),
        Some("center") => Some("center"),
        Some("left") => None,
        Some(other) => return Err(RenderError::new(at, format!("column `align` must be left, center or right, got `{other}`"))),
        None if numeric => Some("right"),
        None => None,
    };
    let filter = one_of("filter", FILTERS)?;
    let cell = one_of("cell", CELLS)?;
    let summary = one_of("summary", SUMMARIES)?;
    if field.is_none() && (filter.is_some() || summary.is_some() || cell.is_some()) {
        return Err(RenderError::new(at, "a column with `filter`, `summary` or `cell` needs a `field`"));
    }
    let tones = match get("tones") {
        Some(Json::Object(m)) => m,
        None | Some(Json::Null) => Map::new(),
        Some(_) => return Err(RenderError::new(at, "column `tones` must be an object of value → tone")),
    };
    for (value, tone) in &tones {
        let tone = expr::stringify(tone);
        if !TONES.contains(&tone.as_str()) {
            return Err(RenderError::new(at, format!("tone `{tone}` for `{value}` must be one of {}", TONES.join(", "))));
        }
    }
    Ok(Column {
        sort_field: sort_field.clone(),
        sortable: flag("sortable", sort_field.is_some()) && sort_field.is_some(),
        label_text: String::new(),
        key,
        field,
        label,
        filter,
        options: get("options").filter(|v| !v.is_null()),
        presets: get("presets").filter(|v| !v.is_null()),
        fiscal_start: get("fiscalStart").filter(|v| !v.is_null()),
        align,
        format,
        currency: string("currency"),
        decimals: get("decimals").filter(|v| !v.is_null()).map(|v| expr::to_num(&v)),
        cell,
        tones,
        summary,
        width: get("width").filter(|v| !v.is_null()).map(Val::Lit),
        hideable: flag("hideable", true),
        hidden: flag("hidden", false),
        pin: match string("pin").as_deref() {
            Some("left") => Some("left"),
            Some("right") => Some("right"),
            None => None,
            Some(other) => return Err(RenderError::new(at, format!("column `pin` must be left or right, got `{other}`"))),
        },
        tree: flag("tree", false),
        group: string("group").filter(|g| !g.is_empty()),
        groupable: flag("groupable", false),
        resizable: flag("resizable", true),
        template,
    })
}

/// The table's children, sorted by kind.
#[derive(Default)]
struct Parts<'n> {
    columns: Vec<&'n Node>,
    toolbar: Vec<&'n Node>,
    bulk: Vec<&'n Node>,
    detail: Option<&'n Node>,
}

impl<'n> Parts<'n> {
    fn collect(table: &'n Node) -> Result<Self, RenderError> {
        let mut parts = Parts::default();
        for child in &table.children {
            match child.kind.as_str() {
                "ui-data-table-column" => parts.columns.push(child),
                "ui-data-table-toolbar" => parts.toolbar.push(child),
                "ui-data-table-bulk" => parts.bulk.push(child),
                "ui-data-table-detail" if parts.detail.is_some() => {
                    return Err(RenderError::new(child, "a table takes one `ui-data-table-detail`"));
                }
                "ui-data-table-detail" => parts.detail = Some(child),
                other => {
                    return Err(RenderError::new(
                        child,
                        format!(
                            "`ui-data-table` takes `ui-data-table-column`, `ui-data-table-toolbar`, `ui-data-table-bulk` and `ui-data-table-detail` children, not `{other}`"
                        ),
                    ));
                }
            }
        }
        Ok(parts)
    }
}

/// `row.a.b` for a field `a.b`.
fn row_path(field: &str) -> Expr {
    let mut p = vec![ROW.to_string()];
    p.extend(field.split('.').map(String::from));
    Expr::Path(p)
}

fn lookup<'j>(row: &'j Json, field: &str) -> &'j Json {
    field.split('.').try_fold(row, |v, k| v.get(k)).unwrap_or(&Json::Null)
}

/// Text safe in an element id.
fn slug(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect()
}

/// A prop as the behaviour needs it, as the chart kit passes them: templates that read
/// state, at the top or anywhere inside an object or list, left as `{ "$irender": js }`
/// for the browser, and every other template evaluated now.
pub(crate) fn dynamic(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Val) -> Result<Json, RenderError> {
    match v {
        Val::Tpl(_) if cx.is_reactive(v) => Ok(json!({ "$irender": cx.val_js(v) })),
        Val::Tpl(_) => Ok(cx.eval(v)),
        Val::Lit(Json::String(s)) if irender_core::Template::is_template(s) => {
            let t = irender_core::Template::parse(s).map_err(|e| RenderError::new(node, format!("`{name}`: {e}")))?;
            dynamic(cx, node, name, &Val::Tpl(t))
        }
        Val::Lit(Json::Array(items)) => {
            Ok(Json::Array(items.iter().map(|i| dynamic(cx, node, name, &Val::Lit(i.clone()))).collect::<Result<_, _>>()?))
        }
        Val::Lit(Json::Object(o)) => Ok(Json::Object(
            o.iter().map(|(k, v)| Ok((k.clone(), dynamic(cx, node, name, &Val::Lit(v.clone()))?))).collect::<Result<_, RenderError>>()?,
        )),
        Val::Lit(other) => Ok(other.clone()),
    }
}

// ---------------------------------------------------------------- sorting on the server

/// `field`, `-field`, `{ field, order }` or a list of them, as (field, descending).
fn sort_keys(v: &Json) -> Vec<(String, bool)> {
    let one = |k: &Json| match k {
        Json::String(s) if s.starts_with('-') => Some((s[1..].to_string(), true)),
        Json::String(s) if !s.is_empty() => Some((s.clone(), false)),
        Json::Object(m) => m.get("field").map(|f| (expr::stringify(f), m.get("order").and_then(Json::as_str) == Some("desc"))),
        _ => None,
    };
    match v {
        Json::Array(list) => list.iter().filter_map(one).collect(),
        other => one(other).into_iter().collect(),
    }
}

/// As `cmp` in data_table.js: numbers as numbers, other values as text, and blanks last
/// whichever way the column sorts. The browser compares text by locale, so the
/// server's order of mixed-case text can differ until it runs.
fn compare(a: &Json, b: &Json, desc: bool) -> Ordering {
    let blank = |v: &Json| v.is_null() || v.as_str() == Some("");
    match (blank(a), blank(b)) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    let o = match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => expr::stringify(a).to_lowercase().cmp(&expr::stringify(b).to_lowercase()),
    };
    if desc { o.reverse() } else { o }
}

// ---------------------------------------------------------------- the component

impl Component for DataTable {
    fn name(&self) -> &'static str {
        "ui-data-table"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A data table: sorting, column filters, search, paging, row selection with bulk actions, \
                  expandable rows, a View menu and summaries. Children are `ui-data-table-column`s (a cell \
                  template, with the row as `row`), `ui-data-table-toolbar`, `ui-data-table-bulk` and \
                  `ui-data-table-detail`.",
            props: PROPS,
            parts: &["toolbar", "table", "footer"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        let parts = Parts::collect(node)?;
        let flag = |cx: &RenderCx<'_>, name: &str, default: bool| cx.prop(node, name).filter(|v| !v.is_null()).map_or(default, |v| expr::truthy(&v));

        // Columns: the children, then any in `columns`.
        let mut columns = Vec::new();
        for (i, c) in parts.columns.iter().enumerate() {
            let get = |name: &str| cx.prop(c, name);
            columns.push(column(c, i, &get, c.prop("label").cloned(), (!c.children.is_empty()).then_some(*c))?);
        }
        for (i, c) in server_array(cx, node, "columns")?.into_iter().enumerate() {
            let Json::Object(m) = c else { return Err(RenderError::new(node, "each of `columns` must be an object")) };
            let get = |name: &str| m.get(name).cloned();
            columns.push(column(node, columns.len() + i, &get, None, None)?);
        }
        if columns.is_empty() {
            return Err(RenderError::new(node, "a data table needs columns: `ui-data-table-column` children or `columns`"));
        }
        for i in 0..columns.len() {
            if columns[..i].iter().any(|c| c.key == columns[i].key) {
                return Err(RenderError::new(node, format!("two columns are called `{}`; give one an `id`", columns[i].key)));
            }
            columns[i].label_text = expr::stringify(&cx.eval(&columns[i].label));
        }
        if columns.iter().any(|c| c.cell.as_deref() == Some("sparkline")) && !cx.has_component("ui-sparkline") {
            return Err(RenderError::new(node, "`cell: sparkline` needs the chart kit"));
        }

        let url = node.prop("url");
        for name in ["url", "childrenUrl"] {
            if let Some(u) = node.prop(name).filter(|u| !cx.is_reactive(u)) {
                let u = expr::stringify(&cx.eval(u));
                if !crate::combobox::safe_search_url(&u) {
                    return Err(RenderError::new(node, format!("`{name}` must be a path or an http(s) URL, got `{u}`")));
                }
            }
        }
        let no_rows = Val::Lit(Json::Array(Vec::new()));
        let rows_val = match (node.prop("rows"), url) {
            (Some(r), None) => r,
            (None, Some(_)) => &no_rows,
            (Some(_), Some(_)) => return Err(RenderError::new(node, "give a table `rows` or `url`, not both")),
            (None, None) => return Err(RenderError::new(node, "a data table needs `rows` (or `url`)")),
        };
        let live = cx.is_reactive(rows_val);
        let rows = match cx.eval(rows_val) {
            Json::Array(r) => r,
            Json::Null => Vec::new(),
            _ => return Err(RenderError::new(node, "`rows` must evaluate to an array")),
        };
        // Nested rows: asked for, or rows that start out with `children`. The tree column
        // is the one marked `tree`, else the first.
        let own_nesting = columns.iter().any(|c| c.tree)
            || ["childrenField", "parentField", "childrenUrl"].iter().any(|p| node.prop(p).is_some())
            || rows.iter().any(|r| matches!(r.get("children"), Some(Json::Array(_) | Json::Bool(true))));
        let grouped = node.prop("groupBy").is_some() || cx.prop_string(node, "bindGroupBy").is_some() || columns.iter().any(|c| c.groupable);
        // Rows from `url` grouped: the API answers a row per group, and a group's rows load
        // when it's opened, as nested rows do.
        let group_remote = grouped && url.is_some() && !own_nesting;
        let nested = own_nesting || group_remote;
        if nested && !columns.iter().any(|c| c.tree) {
            columns[0].tree = true;
        }
        if columns.iter().filter(|c| c.tree).count() > 1 {
            return Err(RenderError::new(node, "only one column can be the `tree` column"));
        }
        let selectable = flag(cx, "selectable", false) || !parts.bulk.is_empty();
        let row_key = cx.prop_string(node, "rowKey").unwrap_or_else(|| "id".into());
        let bind = |name: &str| cx.prop_string(node, name);
        let state_of = |key: Option<String>| key.map(|k| cx.lookup(&[k])).filter(|v| !v.is_null());

        // Where the table starts: the bound state if it has a value, else the props.
        let page_size = cx.prop(node, "pageSize").map_or(10.0, |v| expr::to_num(&v)).max(0.0) as usize;
        let page_sizes = match cx.prop(node, "pageSizes") {
            None | Some(Json::Null) => vec![json!(10), json!(20), json!(50), json!(100)],
            Some(Json::Array(a)) => a,
            Some(_) => return Err(RenderError::new(node, "`pageSizes` must be a list of numbers")),
        };
        let sort = state_of(bind("bindSort")).or_else(|| cx.prop(node, "sort")).unwrap_or(Json::Null);
        let selection: Vec<Json> = match state_of(bind("bindSelection")) {
            Some(Json::Array(a)) => a,
            _ => Vec::new(),
        };
        let page = state_of(bind("bindPage")).map_or(1.0, |v| expr::to_num(&v)).max(1.0) as usize;
        let paging = match cx.prop_string(node, "paging") {
            Some(p) if !PAGING.contains(&p.as_str()) => {
                return Err(RenderError::new(node, format!("`paging` must be one of {}, got `{p}`", PAGING.join(", "))));
            }
            Some(p) => p,
            None if page_size == 0 => "none".into(),
            None => "pages".into(),
        };
        let page_size = if page_size == 0 && paging != "none" { 10 } else { page_size };
        let virtual_ = flag(cx, "virtual", false);
        // Virtual and remote tables have their rows supplied by the behaviour (the ones in
        // view, or the ones loaded) instead of arranging rows the server rendered.
        let listed = virtual_ || url.is_some() || nested;

        // The order and page the server shows, for server rows; the browser takes over.
        let mut order: Vec<usize> = (0..rows.len()).collect();
        let keys = sort_keys(&sort);
        if !keys.is_empty() {
            order.sort_by(|&a, &b| {
                keys.iter().map(|(f, d)| compare(lookup(&rows[a], f), lookup(&rows[b], f), *d)).find(|o| o.is_ne()).unwrap_or(Ordering::Equal)
            });
        }
        let pages = if paging != "pages" { 1 } else { rows.len().div_ceil(page_size).max(1) };
        let page = page.min(pages);
        let shown = |pos: usize| match paging.as_str() {
            "pages" => pos >= (page - 1) * page_size && pos < page * page_size,
            "none" => true,
            _ => pos < page_size,
        };

        let children_url = node.prop("childrenUrl").map(|u| dynamic(cx, node, "childrenUrl", u)).transpose()?;
        if grouped && listed && !(group_remote && !virtual_) {
            return Err(RenderError::new(node, "grouping rows doesn't work with `virtual` or nested rows"));
        }
        let resizable = flag(cx, "resizable", false);
        let reorderable = flag(cx, "reorderable", false);
        // The file name, from `export: true` (the title's, or `rows`) or `export: "name.csv"`.
        let export = match cx.prop(node, "export") {
            None | Some(Json::Null) | Some(Json::Bool(false)) => None,
            Some(Json::String(name)) if !name.is_empty() => Some(if name.ends_with(".csv") { name } else { format!("{name}.csv") }),
            Some(_) => Some(format!("{}.csv", cx.prop_string(node, "title").map(|t| slug(&t)).filter(|t| !t.is_empty()).unwrap_or_else(|| "rows".into()))),
        };
        let id = instance_id(cx, node, "data-table");
        let search = cx.prop(node, "search").filter(expr::truthy);
        // Saved views: the ready-made ones, each with an id.
        let views: Vec<Json> = match cx.prop(node, "views") {
            None | Some(Json::Null) => Vec::new(),
            Some(Json::Array(a)) => a
                .into_iter()
                .enumerate()
                .map(|(i, v)| match v {
                    Json::Object(mut o) => {
                        let id = o.get("id").filter(|x| !x.is_null()).map(expr::stringify).unwrap_or_else(|| format!("preset-{}", i + 1));
                        let name = o.get("name").map(expr::stringify).unwrap_or_else(|| id.clone());
                        o.insert("id".into(), json!(id));
                        o.insert("name".into(), json!(name));
                        Ok(Json::Object(o))
                    }
                    _ => Err(RenderError::new(node, "each of `views` must be an object: `{ id, name, sort, filters, … }`")),
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(RenderError::new(node, "`views` must be a list of views")),
        };
        let has_views = !views.is_empty() || bind("bindViews").is_some();
        // The Views button names the view shown, when it's a ready-made one.
        let view_name = state_of(bind("bindCurrentView"))
            .map(|v| expr::stringify(&v))
            .or_else(|| cx.prop_string(node, "defaultView"))
            .and_then(|c| views.iter().find(|v| v["id"] == json!(c)).map(|v| expr::stringify(&v["name"])))
            .unwrap_or_else(|| cx.label("Views"));
        let search_fields = cx.prop(node, "searchFields").filter(|v| !v.is_null());
        let config = json!({
            "rows": dynamic(cx, node, "rows", rows_val)?,
            "url": url.map(|u| dynamic(cx, node, "url", u)).transpose()?,
            "params": node.prop("params").map(|p| dynamic(cx, node, "params", p)).transpose()?,
            "paging": paging,
            "virtual": virtual_,
            "listed": listed,
            "rowHeight": cx.prop(node, "rowHeight").map_or(41.0, |v| expr::to_num(&v)).max(8.0),
            "rowKey": row_key,
            "tree": nested.then(|| json!({
                "field": cx.prop_string(node, "childrenField").unwrap_or_else(|| "children".into()),
                "parent": cx.prop_string(node, "parentField"),
                "url": children_url,
                "all": flag(cx, "expanded", false),
            })),
            "groupRemote": group_remote,
            "columns": columns.iter().map(|c| json!({
                "key": c.key, "field": c.field, "sortField": c.sort_field, "label": c.label_text, "sortable": c.sortable, "filter": c.filter,
                "options": c.options, "format": c.format, "currency": c.currency, "decimals": c.decimals,
                "summary": c.summary, "hidden": c.hidden, "group": c.group, "groupable": c.groupable, "pin": c.pin, "align": c.align,
            })).collect::<Vec<_>>(),
            "groupBy": node.prop("groupBy").map(|g| dynamic(cx, node, "groupBy", g)).transpose()?,
            "resizable": resizable,
            "reorderable": reorderable,
            "export": export.clone(),
            "selectable": selectable,
            "search": search.is_some(),
            "searchFields": search_fields,
            "sort": sort,
            "filter": node.prop("filter").map(|f| dynamic(cx, node, "filter", f)).transpose()?,
            "pageSize": page_size,
            "locale": crate::locale(cx, node),
            "detail": parts.detail.is_some(),
            "bind": {
                "selection": bind("bindSelection"), "sort": bind("bindSort"), "filters": bind("bindFilters"),
                "search": bind("bindSearch"), "page": bind("bindPage"), "view": bind("bindView"), "row": bind("bindRow"),
                "groupBy": bind("bindGroupBy"), "columns": bind("bindColumns"),
                "views": bind("bindViews"), "currentView": bind("bindCurrentView"),
            },
            "views": views,
            "defaultView": cx.prop_string(node, "defaultView"),
            "filters": state_of(bind("bindFilters")),
            "searchText": state_of(bind("bindSearch")),
        });
        let clickable = bind("bindRow").is_some();
        let locale = crate::locale(cx, node);
        let search_text = state_of(bind("bindSearch")).map(|v| expr::stringify(&v)).unwrap_or_default();
        cx.global_js("ui-data-table", SCRIPT);

        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-data-table", config).attr("id", id.clone());
        if flag(cx, "striped", false) {
            root.attr("data-striped", true);
        }
        if clickable {
            root.attr("data-clickable", true);
        }
        if let Some(l) = node.prop("loading") {
            root.attr("aria-busy", l.clone());
        }
        cx.open(&root, node)?;

        // ---- toolbar
        let filtered: Vec<&Column> = columns.iter().filter(|c| c.filter.is_some()).collect();
        let groupable: Vec<&Column> = columns.iter().filter(|c| c.groupable && c.field.is_some()).collect();
        let menu = (flag(cx, "columnsMenu", true) && columns.iter().any(|c| c.hideable)) || !groupable.is_empty();
        if search.is_some() || !filtered.is_empty() || menu || !parts.toolbar.is_empty() || export.is_some() || has_views {
            let mut bar = cx.part("div", node, "toolbar");
            bar.attr("data-dt-toolbar", true);
            cx.open(&bar, node)?;
            if has_views {
                // The Views menu: its list is drawn by the script, from `views` and `bindViews`.
                let pop = id_with(&id, "-views");
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                b.attr("type", "button").attr("popovertarget", pop.clone()).attr("aria-haspopup", "dialog").attr("data-dt-views-button", true);
                let anchor = format!("--{}-views", expr::stringify(&cx.eval(&id)));
                b.base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
                cx.open(&b, node)?;
                cx.markup(VIEWS);
                let mut l = Element::new("span");
                l.attr("data-dt-views-label", true).text(lit(view_name.clone()));
                cx.open(&l, node)?;
                cx.close("span");
                cx.close("button");
                let mut p = Element::new("div");
                p.attr("popover", "auto").attr("id", pop).attr("data-dt-pop", true).attr("data-dt-views", true).attr("role", "dialog").attr("aria-label", cx.label("Views"));
                p.base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor), "bottom span-right"));
                cx.open(&p, node)?;
                cx.close("div");
            }
            if let Some(s) = &search {
                let placeholder = match s {
                    Json::String(p) => p.clone(),
                    _ => cx.label("Search…"),
                };
                let mut input = Element::new("input");
                input
                    .attr("type", "search")
                    .attr("data-dt-search", true)
                    .attr("placeholder", placeholder.clone())
                    .attr("aria-label", placeholder)
                    .attr("value", search_text.clone());
                cx.open(&input, node)?;
            }
            for c in &filtered {
                filter_control(cx, node, &id, c, locale.as_deref())?;
            }
            if !filtered.is_empty() || search.is_some() {
                let mut reset = Element::new("button");
                forms::button_look(&mut reset, "ghost", "sm");
                reset.attr("type", "button").attr("data-dt-reset", true).attr("hidden", true).text(lit(format!("{} ✕", cx.label("Reset"))));
                cx.open(&reset, node)?;
                cx.close("button");
            }
            let mut spacer = Element::new("span");
            spacer.attr("data-dt-spacer", true);
            cx.open(&spacer, node)?;
            cx.close("span");
            for t in &parts.toolbar {
                cx.children(t)?;
            }
            if export.is_some() {
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                b.attr("type", "button").attr("data-dt-export", true).attr("title", cx.label("Download the rows that pass the filters as CSV"));
                cx.open(&b, node)?;
                cx.markup(DOWNLOAD);
                let t = cx.label("Export");
                cx.text(&t);
                cx.close("button");
            }
            if menu {
                let pop = id_with(&id, "-columns");
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                b.attr("type", "button").attr("popovertarget", pop.clone()).attr("aria-haspopup", "dialog");
                let anchor = format!("--{}-columns", expr::stringify(&cx.eval(&id)));
                b.base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
                cx.open(&b, node)?;
                cx.markup(COLUMNS);
                let t = cx.label("View");
                cx.text(&t);
                cx.close("button");
                let mut p = Element::new("div");
                p.attr("popover", "auto").attr("id", pop).attr("data-dt-pop", true).attr("data-dt-columns", true);
                p.base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor), "bottom span-left"));
                cx.open(&p, node)?;
                let mut title = Element::new("div");
                title.attr("data-dt-pop-title", true).text(lit(cx.label("Columns")));
                cx.open(&title, node)?;
                cx.close("div");
                for c in columns.iter().filter(|c| c.hideable) {
                    let mut label = Element::new("label");
                    cx.open(label.attr("data-dt-option", true), node)?;
                    let mut cb = Element::new("input");
                    cb.base_style_ref(forms::checkbox_look()).attr("type", "checkbox").attr("data-dt-show", c.key.clone()).attr("checked", !c.hidden);
                    cx.open(&cb, node)?;
                    cx.text(&c.label_text);
                    cx.close("label");
                }
                if !groupable.is_empty() {
                    let mut title = Element::new("div");
                    title.attr("data-dt-pop-title", true).attr("data-dt-group-title", true).text(lit(cx.label("Group by")));
                    cx.open(&title, node)?;
                    cx.close("div");
                    let name = format!("{}-group-by", expr::stringify(&cx.eval(&id)));
                    let none = (String::new(), cx.label("None"));
                    for (value, text) in std::iter::once(none).chain(groupable.iter().map(|c| (c.field.clone().unwrap_or_default(), c.label_text.clone()))) {
                        let mut label = Element::new("label");
                        cx.open(label.attr("data-dt-option", true), node)?;
                        let mut r = Element::new("input");
                        r.attr("type", "radio").attr("name", name.clone()).attr("value", value).attr("data-dt-group-by", true);
                        cx.open(&r, node)?;
                        cx.text(&text);
                        cx.close("label");
                    }
                }
                cx.close("div");
            }
            cx.close("div");
        }

        // ---- bulk actions
        if !parts.bulk.is_empty() {
            let mut bulk = Element::new("div");
            bulk.attr("data-dt-bulk", true).attr("role", "region").attr("aria-label", cx.label("Bulk actions")).attr("hidden", selection.is_empty());
            cx.open(&bulk, node)?;
            let mut count = Element::new("span");
            count.attr("data-dt-count", true).text(lit(cx.label_with("{n} selected", &[("n", &selection.len().to_string())])));
            cx.open(&count, node)?;
            cx.close("span");
            // Labels: "Select all", "Clear selection"
            for (attr, text) in [("data-dt-select-all", "Select all"), ("data-dt-clear-selection", "Clear selection")] {
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-dt-link", true).attr(attr, true).text(lit(cx.label(text)));
                if attr == "data-dt-select-all" {
                    b.attr("hidden", true);
                }
                cx.open(&b, node)?;
                cx.close("button");
            }
            let mut actions = Element::new("div");
            actions.attr("data-dt-bulk-actions", true);
            cx.open(&actions, node)?;
            for b in &parts.bulk {
                cx.children(b)?;
            }
            cx.close("div");
            cx.close("div");
        }

        // ---- table
        let mut scroll = Element::new("div");
        scroll.attr("data-dt-scroll", true);
        let default_height = lit(480);
        if let Some(h) = node.prop("height").or(virtual_.then_some(&default_height)) {
            let mut s = StyleBlock::new();
            if let Some(v) = cx.style_value(node, "max-height", h)? {
                s.push("max-height", v);
            }
            scroll.base_style(s);
        }
        cx.open(&scroll, node)?;
        let mut table = cx.part("table", node, "table");
        if nested {
            table.attr("role", "treegrid");
        }
        if let Some(t) = node.prop("title") {
            table.attr("aria-label", t.clone());
        }
        cx.open(&table, node)?;

        let lead = selectable as usize + parts.detail.is_some() as usize;
        let span = lit(columns.len() + lead);
        // The checkbox and arrow columns stay with the columns pinned to the left.
        let lead_pin = if columns.iter().any(|c| c.pin == Some("left")) { lit("left") } else { Val::Lit(Json::Null) };
        cx.open(&Element::new("thead"), node)?;
        if columns.iter().any(|c| c.group.is_some()) {
            // Headers over runs of columns with the same `group`; the browser redraws it
            // when columns move or hide.
            let mut tr = Element::new("tr");
            tr.attr("data-dt-bands", true);
            cx.open(&tr, node)?;
            for _ in 0..lead {
                let mut th = Element::new("th");
                th.attr("data-dt-box", true).attr("data-pin", lead_pin.clone()).attr("aria-hidden", "true");
                cx.open(&th, node)?;
                cx.close("th");
            }
            let shown: Vec<&Column> = columns.iter().filter(|c| !c.hidden).collect();
            let mut i = 0;
            while i < shown.len() {
                let g = shown[i].group.clone();
                let n = shown[i..].iter().take_while(|c| c.group == g).count();
                let mut th = Element::new("th");
                th.attr("data-dt-band", g.clone().unwrap_or_default()).attr("colspan", lit(n));
                match &g {
                    Some(g) => {
                        th.attr("scope", "colgroup").text(lit(g.clone()));
                    }
                    None => {
                        th.attr("aria-hidden", "true");
                    }
                }
                cx.open(&th, node)?;
                cx.close("th");
                i += n;
            }
            cx.close("tr");
        }
        cx.open(&Element::new("tr"), node)?;
        if selectable {
            let mut th = Element::new("th");
            th.attr("data-dt-box", true).attr("data-pin", lead_pin.clone());
            cx.open(&th, node)?;
            let mut all = Element::new("input");
            all.base_style_ref(forms::checkbox_look()).attr("type", "checkbox").attr("data-dt-select-page", true).attr("aria-label", cx.label("Select all rows on this page"));
            cx.open(&all, node)?;
            cx.close("th");
        }
        if parts.detail.is_some() {
            let mut th = Element::new("th");
            th.attr("data-dt-box", true).attr("data-pin", lead_pin.clone());
            cx.open(&th, node)?;
            let mut hidden = Element::new("span");
            hidden.attr("data-dt-live", true).text(lit(cx.label("Details")));
            cx.open(&hidden, node)?;
            cx.close("span");
            cx.close("th");
        }
        for c in &columns {
            let mut th = Element::new("th");
            th.attr("data-dt-col", c.key.clone()).attr("scope", "col");
            if let Some(p) = c.pin {
                th.attr("data-pin", p);
            }
            if let Some(a) = c.align {
                th.attr("data-align", a);
            }
            if c.hidden {
                th.attr("hidden", true);
            }
            if let Some((_, desc)) = keys.iter().find(|(f, _)| Some(f) == c.sort_field.as_ref()) {
                th.attr("aria-sort", if *desc { "descending" } else { "ascending" });
            }
            if let Some(w) = &c.width {
                let mut s = StyleBlock::new();
                if let Some(v) = cx.style_value(node, "width", w)? {
                    s.push("width", v);
                }
                th.base_style(s);
            }
            if reorderable && c.pin.is_none() {
                th.attr("data-dt-movable", true);
            }
            cx.open(&th, node)?;
            if c.sortable {
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-dt-sort", c.key.clone());
                cx.open(&b, node)?;
                let mut label = Element::new("span");
                label.text(c.label.clone());
                cx.open(&label, node)?;
                cx.close("span");
                let mut arrow = Element::new("span");
                arrow.attr("data-dt-arrow", true);
                cx.open(&arrow, node)?;
                cx.markup(if keys.iter().any(|(f, _)| Some(f) == c.sort_field.as_ref()) { SORT_ARROW } else { SORT_BOTH });
                cx.close("span");
                cx.close("button");
            } else {
                let mut label = Element::new("span");
                label.text(c.label.clone());
                cx.open(&label, node)?;
                cx.close("span");
            }
            if resizable && c.resizable {
                let mut r = Element::new("span");
                r.attr("data-dt-resizer", c.key.clone())
                    .attr("role", "separator")
                    .attr("aria-orientation", "vertical")
                    .attr("tabindex", "0")
                    .attr("aria-label", cx.label_with("Resize {name}", &[("name", &c.label_text)]));
                cx.open(&r, node)?;
                cx.close("span");
            }
            cx.close("th");
        }
        cx.close("tr");
        cx.close("thead");

        // One row: its cells, then its detail row if there is one.
        let first_field = columns.iter().find_map(|c| c.field.clone());
        let render_row = |cx: &mut RenderCx<'_>, hidden: bool| -> Result<(), RenderError> {
            let mut tr = Element::new("tr");
            tr.attr("data-dt-row", expr_val(Expr::Path(vec![ROW_INDEX.into()])));
            if hidden {
                tr.attr("hidden", true);
            }
            cx.open(&tr, node)?;
            if selectable {
                let mut td = Element::new("td");
                td.attr("data-dt-box", true).attr("data-pin", lead_pin.clone());
                cx.open(&td, node)?;
                let mut cb = Element::new("input");
                let name = match &first_field {
                    Some(f) => crate::val_expr(&expr_val(row_path(f))),
                    None => Expr::Lit(json!("row")),
                };
                cb.base_style_ref(forms::checkbox_look())
                    .attr("type", "checkbox")
                    .attr("data-dt-select", true)
                    .attr("aria-label", expr_val(bin(BinOp::Add, Expr::Lit(json!("Select ")), name)));
                if !selection.is_empty() {
                    let key = cond(
                        bin(BinOp::Ne, row_path(&row_key), Expr::Lit(Json::Null)),
                        row_path(&row_key),
                        Expr::Path(vec![ROW_INDEX.into()]),
                    );
                    if selection.contains(&cx.eval(&expr_val(key))) {
                        cb.attr("checked", true);
                    }
                }
                cx.open(&cb, node)?;
                cx.close("td");
            }
            if parts.detail.is_some() {
                let mut td = Element::new("td");
                td.attr("data-dt-box", true).attr("data-pin", lead_pin.clone());
                cx.open(&td, node)?;
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-dt-expand", true).attr("aria-expanded", "false").attr("aria-label", cx.label("Show details"));
                cx.open(&b, node)?;
                cx.markup(CHEVRON);
                cx.close("button");
                cx.close("td");
            }
            for c in &columns {
                cell(cx, node, c)?;
            }
            cx.close("tr");
            if let Some(d) = parts.detail {
                let mut tr = Element::new("tr");
                tr.attr("data-dt-detail", true).attr("hidden", true);
                cx.open(&tr, node)?;
                let mut td = cx.element("td", d);
                td.attr("colspan", span.clone());
                cx.open(&td, d)?;
                cx.children(d)?;
                cx.close("td");
                cx.close("tr");
            }
            Ok(())
        };

        // Space for the rows above and below the ones in view, in a virtual table.
        let pad = |cx: &mut RenderCx<'_>, at: &str| -> Result<(), RenderError> {
            let mut b = Element::new("tbody");
            b.attr("data-dt-pad", at).attr("aria-hidden", "true");
            cx.open(&b, node)?;
            cx.open(&Element::new("tr"), node)?;
            let mut td = Element::new("td");
            td.attr("colspan", span.clone());
            cx.open(&td, node)?;
            cx.close("td");
            cx.close("tr");
            cx.close("tbody");
            Ok(())
        };
        if virtual_ {
            pad(cx, "top")?;
        }
        let mut body = Element::new("tbody");
        body.attr("data-dt-body", true);
        // The behaviour supplies a listed table's rows, so its own `items` stay empty.
        let none = Val::Lit(Json::Array(Vec::new()));
        if listed {
            let key = cond(bin(BinOp::Ne, row_path(&row_key), Expr::Lit(Json::Null)), row_path(&row_key), Expr::Path(vec![ROW_INDEX.into()]));
            let key = cx.item_val_js(&expr_val(key), ROW, ROW_INDEX);
            body.list(ClientList { items: &none, item: ROW.into(), index: ROW_INDEX.into(), key: Some(key) });
        } else if live {
            body.list(ClientList { items: rows_val, item: ROW.into(), index: ROW_INDEX.into(), key: None });
        }
        cx.open(&body, node)?;
        if listed {
            // The starting rows, for pages read without JavaScript; the browser replaces
            // them. Their index is their place in the order shown.
            cx.item_template_with(ROW, ROW_INDEX, |cx| render_row(cx, false))?;
            let first: Vec<usize> = order.iter().copied().enumerate().filter(|(pos, _)| shown(*pos)).map(|(_, i)| i).take(if virtual_ { 40 } else { usize::MAX }).collect();
            for (pos, i) in first.into_iter().enumerate() {
                cx.with_scope(vec![(ROW.into(), rows[i].clone()), (ROW_INDEX.into(), json!(pos))], |cx| render_row(cx, false))?;
            }
        } else if live {
            cx.item_template_with(ROW, ROW_INDEX, |cx| render_row(cx, false))?;
            for (i, r) in rows.iter().enumerate() {
                cx.with_scope(vec![(ROW.into(), r.clone()), (ROW_INDEX.into(), json!(i))], |cx| render_row(cx, false))?;
            }
        } else {
            for (pos, &i) in order.iter().enumerate() {
                cx.with_scope(vec![(ROW.into(), rows[i].clone()), (ROW_INDEX.into(), json!(i))], |cx| render_row(cx, !shown(pos)))?;
            }
        }
        cx.close("tbody");
        if virtual_ {
            pad(cx, "bottom")?;
        }

        let mut empty_body = Element::new("tbody");
        empty_body.attr("data-dt-empty-body", true);
        cx.open(&empty_body, node)?;
        let mut tr = Element::new("tr");
        tr.attr("data-dt-empty", true).attr("hidden", !rows.is_empty() || url.is_some());
        cx.open(&tr, node)?;
        let mut td = Element::new("td");
        td.attr("colspan", span.clone()).text(node.prop("emptyText").cloned().unwrap_or_else(|| lit(cx.label("No results."))));
        cx.open(&td, node)?;
        cx.close("td");
        cx.close("tr");
        cx.close("tbody");

        if columns.iter().any(|c| c.summary.is_some()) {
            cx.open(&Element::new("tfoot"), node)?;
            cx.open(&Element::new("tr"), node)?;
            for _ in 0..lead {
                let mut td = Element::new("td");
                cx.open(td.attr("data-pin", lead_pin.clone()), node)?;
                cx.close("td");
            }
            for c in &columns {
                let mut td = Element::new("td");
                td.attr("data-dt-col", c.key.clone());
                if let Some(a) = c.align {
                    td.attr("data-align", a);
                }
                if let Some(p) = c.pin {
                    td.attr("data-pin", p);
                }
                if c.hidden {
                    td.attr("hidden", true);
                }
                if c.summary.is_some() {
                    td.attr("data-dt-sum", c.key.clone());
                }
                cx.open(&td, node)?;
                cx.close("td");
            }
            cx.close("tr");
            cx.close("tfoot");
        }
        cx.close("table");
        if paging == "more" || paging == "scroll" || url.is_some() {
            // Where the next batch is asked for: scrolled into view, or its button.
            let mut more = Element::new("div");
            more.attr("data-dt-more", true).attr("data-paging", paging.clone());
            if url.is_none() && rows.len() <= page_size {
                more.attr("hidden", true);
            }
            cx.open(&more, node)?;
            let mut loading = Element::new("span");
            loading.attr("data-dt-loading", true).attr("hidden", true).text(lit(cx.label("Loading more…")));
            cx.open(&loading, node)?;
            cx.close("span");
            if paging == "more" {
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                b.attr("type", "button").attr("data-dt-load", true).text(lit(cx.label("Load more")));
                cx.open(&b, node)?;
                cx.close("button");
            }
            let mut retry = Element::new("button");
            forms::button_look(&mut retry, "outline", "sm");
            retry.attr("type", "button").attr("data-dt-retry", true).attr("hidden", true).text(lit(cx.label("Couldn't load rows. Retry")));
            cx.open(&retry, node)?;
            cx.close("button");
            cx.close("div");
        }
        cx.close("div");

        // ---- footer
        let mut footer = cx.part("div", node, "footer");
        footer.attr("data-dt-footer", true);
        cx.open(&footer, node)?;
        let mut status = Element::new("div");
        let status_text = if url.is_some() {
            cx.label("Loading…")
        } else if paging == "more" || paging == "scroll" {
            cx.label_with("Showing {shown} of {total}", &[("shown", &rows.len().min(page_size).to_string()), ("total", &rows.len().to_string())])
        } else if selectable {
            cx.label_with("{count} of {total} row(s) selected.", &[("count", &selection.len().to_string()), ("total", &rows.len().to_string())])
        } else {
            cx.label_with("{total} row(s)", &[("total", &rows.len().to_string())])
        };
        status.attr("data-dt-status", true).text(lit(status_text));
        cx.open(&status, node)?;
        cx.close("div");
        if paging == "pages" {
            if !page_sizes.is_empty() {
                let mut sizes = Element::new("label");
                sizes.attr("data-dt-sizes", true);
                cx.open(&sizes, node)?;
                let t = cx.label("Rows per page");
                cx.text(&t);
                let mut select = Element::new("select");
                select.attr("data-dt-size", true);
                cx.open(&select, node)?;
                let mut sizes: Vec<usize> = page_sizes.iter().map(|v| expr::to_num(v).max(1.0) as usize).collect();
                if !sizes.contains(&page_size) {
                    sizes.push(page_size);
                    sizes.sort_unstable();
                }
                for s in sizes {
                    let mut o = Element::new("option");
                    o.attr("value", s.to_string()).attr("selected", s == page_size).text(lit(s.to_string()));
                    cx.open(&o, node)?;
                    cx.close("option");
                }
                cx.close("select");
                cx.close("label");
            }
            let mut at = Element::new("div");
            at.attr("data-dt-page", true).text(lit(cx.label_with("Page {page} of {pages}", &[("page", &page.to_string()), ("pages", &pages.to_string())])));
            cx.open(&at, node)?;
            cx.close("div");
            let mut nav = Element::new("div");
            nav.attr("data-dt-nav", true);
            cx.open(&nav, node)?;
            for (go, label, icon) in PAGE_BUTTONS {
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                let off = if matches!(*go, "first" | "prev") { page <= 1 } else { page >= pages };
                let label = cx.label(label);
                b.attr("type", "button").attr("data-dt-go", *go).attr("aria-label", label.clone()).attr("title", label).attr("disabled", off);
                cx.open(&b, node)?;
                cx.markup(icon);
                cx.close("button");
            }
            cx.close("div");
        }
        let mut live_region = Element::new("div");
        live_region.attr("data-dt-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live_region, node)?;
        cx.close("div");
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

/// A filter's button in the toolbar, and the popover it opens.
fn filter_control(cx: &mut RenderCx<'_>, node: &Node, id: &Val, c: &Column, locale: Option<&str>) -> Result<(), RenderError> {
    let pop = id_with(id, &format!("-filter-{}", slug(&c.key)));
    let anchor = format!("--{}-filter-{}", expr::stringify(&cx.eval(id)), slug(&c.key));
    let mut b = Element::new("button");
    forms::button_look(&mut b, "outline", "sm");
    b.attr("type", "button")
        .attr("popovertarget", pop.clone())
        .attr("aria-haspopup", "dialog")
        .attr("data-dt-trigger", c.key.clone())
        .base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
    cx.open(&b, node)?;
    cx.markup(PLUS);
    cx.text(&c.label_text);
    let mut chips = Element::new("span");
    chips.attr("data-dt-chips", true);
    cx.open(&chips, node)?;
    cx.close("span");
    cx.close("button");

    let kind = c.filter.as_deref().unwrap_or("text");
    let mut p = Element::new("div");
    p.attr("popover", "auto")
        .attr("id", pop)
        .attr("data-dt-pop", true)
        .attr("data-dt-filter", c.key.clone())
        .attr("data-kind", kind)
        .attr("role", "dialog")
        .attr("aria-label", cx.label_with("Filter by {name}", &[("name", &c.label_text)]))
        .base_style(StyleBlock::new().set("position-anchor", anchor));
    cx.open(&p, node)?;
    // Labels: "Contains", "Min", "Max"
    let input = |cx: &mut RenderCx<'_>, ty: &str, op: &str, label: &str| -> Result<(), RenderError> {
        let label = cx.label(label);
        let mut l = Element::new("label");
        l.text(lit(label.clone()));
        cx.open(&l, node)?;
        cx.close("label");
        let mut i = Element::new("input");
        i.attr("type", ty).attr("data-dt-op", op).attr("aria-label", format!("{} {}", c.label_text, label.to_lowercase()));
        if ty == "number" {
            i.attr("step", "any");
        }
        cx.open(&i, node)
    };
    match kind {
        "select" => {
            let mut list = Element::new("div");
            list.attr("data-dt-options", true).attr("role", "group").attr("aria-label", c.label_text.clone());
            cx.open(&list, node)?;
            cx.close("div");
            // What the browser copies for each choice.
            let mut tpl = Element::new("template");
            tpl.attr("data-dt-choice", true);
            cx.open(&tpl, node)?;
            let mut label = Element::new("label");
            cx.open(label.attr("data-dt-option", true), node)?;
            let mut cb = Element::new("input");
            cb.base_style_ref(forms::checkbox_look()).attr("type", "checkbox").attr("data-dt-pick", true);
            cx.open(&cb, node)?;
            cx.open(&Element::new("span"), node)?;
            cx.close("span");
            let mut n = Element::new("span");
            cx.open(n.attr("data-dt-n", true), node)?;
            cx.close("span");
            cx.close("label");
            cx.close("template");
        }
        "text" => {
            let mut f = Element::new("div");
            f.attr("data-dt-fields", true);
            cx.open(&f, node)?;
            input(cx, "search", "contains", "Contains")?;
            cx.close("div");
        }
        "date" => {
            // A range calendar with presets; the behaviour turns its picks into the filter.
            let mut cal = synthetic(
                node,
                "ui-calendar",
                vec![
                    ("mode", lit("range")),
                    ("label", lit(format!("{} range", c.label_text))),
                    ("presets", lit(c.presets.clone().unwrap_or_else(|| json!(["last7", "last30", "thisMonth", "lastMonth", "thisQuarter", "lastQuarter", "thisYear"])))),
                ],
                vec![("data-dt-calendar", lit(true))],
                Vec::new(),
            );
            if let Some(f) = &c.fiscal_start {
                cal.props.insert("fiscalStart".into(), lit(f.clone()));
            }
            if let Some(l) = locale {
                cal.props.insert("locale".into(), lit(l.to_string()));
            }
            cal.style = StyleBlock::from_pairs(&[("border", "0"), ("padding", sp!(4))]);
            cx.render(&cal)?;
        }
        _ => {
            let mut f = Element::new("div");
            f.attr("data-dt-fields", true);
            cx.open(&f, node)?;
            input(cx, "number", "from", "Min")?;
            input(cx, "number", "to", "Max")?;
            cx.close("div");
        }
    }
    let mut clear = Element::new("button");
    clear.attr("type", "button").attr("data-dt-clear", true).text(lit(cx.label("Clear filter")));
    cx.open(&clear, node)?;
    cx.close("button");
    cx.close("div");
    Ok(())
}

/// One cell: the column's template, a shorthand (`badge`, `progress`, `sparkline`),
/// or the value as text, which the browser formats when the column has a `format`.
fn cell(cx: &mut RenderCx<'_>, table: &Node, c: &Column) -> Result<(), RenderError> {
    let mut td = Element::new("td");
    td.attr("data-dt-col", c.key.clone());
    if let Some(a) = c.align {
        td.attr("data-align", a);
    }
    if let Some(p) = c.pin {
        td.attr("data-pin", p);
    }
    if c.hidden {
        td.attr("hidden", true);
    }
    let value = c.field.as_deref().map(|f| expr_val(row_path(f)));
    let plain = matches!((&value, c.template, c.cell.as_deref()), (Some(_), None, None | Some("text")));
    // The tree column indents its content behind the arrow that opens the row.
    let mut text = if c.tree { Element::new("span") } else { std::mem::replace(&mut td, Element::new("td")) };
    if let (true, Some(v)) = (plain, &value) {
        if c.format.is_some() {
            text.attr("data-dt-fmt", true);
        }
        text.text(v.clone());
    }
    if c.tree {
        td.attr("data-dt-tree", true);
        cx.open(&td, table)?;
        let mut node = Element::new("div");
        cx.open(node.attr("data-dt-node", true), table)?;
        let mut b = Element::new("button");
        b.attr("type", "button").attr("data-dt-twisty", true).attr("tabindex", lit(-1)).attr("aria-hidden", "true").attr("hidden", true);
        cx.open(&b, table)?;
        cx.markup(CHEVRON);
        cx.close("button");
        if plain {
            cx.open(&text, table)?;
            cx.close("span");
        }
    } else {
        td = text;
        cx.open(&td, table)?;
    }
    if let Some(t) = c.template {
        cx.children(t)?;
    } else if let (Some(v), Some(f)) = (&value, &c.field) {
        match c.cell.as_deref() {
            Some("badge") => {
                // The tone follows the value, in the browser too for rows from state.
                let fallback = Expr::Lit(json!("secondary"));
                let tone = c.tones.iter().rev().fold(fallback, |rest, (when, tone)| {
                    cond(bin(BinOp::Eq, row_path(f), Expr::Lit(json!(when))), Expr::Lit(tone.clone()), rest)
                });
                let mut s = Element::new("span");
                s.attr("data-dt-badge", expr_val(tone)).text(v.clone());
                cx.open(&s, table)?;
                cx.close("span");
            }
            Some("progress") => {
                let mut wrap = Element::new("span");
                wrap.attr("data-dt-progress", true);
                cx.open(&wrap, table)?;
                let mut p = Element::new("progress");
                p.attr("max", "100").attr("value", v.clone()).attr("aria-label", c.label_text.clone());
                cx.open(&p, table)?;
                cx.close("progress");
                let mut n = Element::new("span");
                n.text(expr_val(bin(BinOp::Add, row_path(f), Expr::Lit(json!("%")))));
                cx.open(&n, table)?;
                cx.close("span");
                cx.close("span");
            }
            Some("sparkline") => {
                let s = synthetic(table, "ui-sparkline", vec![("data", v.clone()), ("height", lit(28)), ("width", lit(96))], Vec::new(), Vec::new());
                cx.render(&s)?;
            }
            _ => {}
        }
    }
    if c.tree {
        cx.close("div");
    }
    cx.close("td");
    Ok(())
}

// ---------------------------------------------------------------- children

/// `ui-data-table-column`, `-toolbar`, `-bulk` and `-detail`: read by the table, never
/// rendered on their own.
pub struct Child(pub &'static str);

impl Child {
    pub const COLUMN: Child = Child("ui-data-table-column");
    pub const TOOLBAR: Child = Child("ui-data-table-toolbar");
    pub const BULK: Child = Child("ui-data-table-bulk");
    pub const DETAIL: Child = Child("ui-data-table-detail");
}

impl Component for Child {
    fn name(&self) -> &'static str {
        self.0
    }

    fn info(&self) -> ComponentInfo {
        let (doc, props): (&'static str, &'static [PropDef]) = match self.0 {
            "ui-data-table-column" => (
                "A column of the `ui-data-table` it is in. Children, if any, draw each cell, with the row as `row` and its position as `rowIndex`.",
                COLUMN_PROPS,
            ),
            "ui-data-table-toolbar" => ("Controls added to the table's toolbar, after its filters (e.g. an Add or Export button).", &[]),
            "ui-data-table-bulk" => (
                "Actions shown while rows are selected, next to the count. Read the selection from the table's `bindSelection` key.",
                &[],
            ),
            _ => ("A row's details, shown under it when its expander is opened; the row is in scope as `row`.", &[]),
        };
        ComponentInfo { doc, props, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, format!("`{}` must be a direct child of `ui-data-table`", self.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_sort_puts_blanks_last() {
        let rows = [json!({ "n": 2 }), json!({ "n": null }), json!({ "n": 10 }), json!({ "n": 1 })];
        let mut order: Vec<usize> = (0..rows.len()).collect();
        order.sort_by(|&a, &b| compare(lookup(&rows[a], "n"), lookup(&rows[b], "n"), false));
        assert_eq!(order, [3, 0, 2, 1]);
        order.sort_by(|&a, &b| compare(lookup(&rows[a], "n"), lookup(&rows[b], "n"), true));
        assert_eq!(order, [2, 0, 3, 1]);
        assert_eq!(sort_keys(&json!(["-a", "b", { "field": "c", "order": "desc" }])), [
            ("a".into(), true),
            ("b".into(), false),
            ("c".into(), true)
        ]);
        assert_eq!(lookup(&json!({ "a": { "b": 3 } }), "a.b"), &json!(3));
    }
}
