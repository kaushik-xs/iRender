//! `ui-data-source` and `ui-data-query`: records fetched by id, and the rows that name
//! them. A source says how one kind of record is fetched (many ids in one call, or one
//! at a time); a query fills a state key with rows from an API, or with records by id,
//! joined to the records they name (`expand`), as `ui-data-table`'s `expand` does.
//! `data.js` runs them: ids asked for together are fetched together, each once, and kept
//! in memory for a while.

use crate::widget;
use irender_core::{ComponentInfo, Node, PropDef, PropKind};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};

const SCRIPT: &str = include_str!("data.js");

/// Adds `data.js` (with `offline.js`, where records are kept on the device), for the
/// components that fetch or join records.
pub(crate) fn scripts(cx: &mut RenderCx<'_>) {
    crate::pwa::offline(cx);
    cx.global_js("ui-data", SCRIPT);
}

/// `persist`, `scope`, `private` and `retain`, checked: kept on the device needs whose
/// records they are.
fn check_keeping(cx: &RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    match cx.prop_string(node, "persist").as_deref() {
        None | Some("memory") => {}
        Some("device") if node.prop("scope").is_none() => {
            return Err(RenderError::new(node, "`persist: device` needs `scope`: whose records they are, e.g. `{{ user.id }}`, so one person's aren't shown to another"));
        }
        Some("device") => {}
        Some(p) => return Err(RenderError::new(node, format!("`persist` is memory or device, got `{p}`"))),
    }
    if let Some(p) = cx.prop(node, "private").filter(|p| !p.is_null()) {
        if !p.as_array().is_some_and(|a| a.iter().all(Json::is_string)) {
            return Err(RenderError::new(node, "`private` is a list of field names, e.g. `[email, phone]`"));
        }
    }
    Ok(())
}

/// The props that say how records are kept, shared by sources and queries.
const KEEPING: [PropDef; 4] = [
    PropDef::new("persist", PropKind::Enum(&["memory", "device"]), "`memory` (default): for the page's life; `device`: kept on the device too (IndexedDB), shown first and when offline"),
    PropDef::new("scope", PropKind::Any, "With `device`: whose records these are, e.g. `{{ user.id }}`; another one lets the others go from the page, and they're kept apart"),
    PropDef::new("private", PropKind::Any, "Fields kept on the device only encrypted, with the key a `ui-app-lock` unlocks (none: they aren't kept), e.g. `[email, phone]`"),
    PropDef::new("retain", PropKind::Number, "Days private fields may stay on the device without being fetched again (default 30)"),
];

/// A URL given in the document: a path or http(s), and allowed by the policy.
fn check(cx: &RenderCx<'_>, node: &Node, what: &str, url: &Json) -> Result<(), RenderError> {
    match url {
        Json::String(u) if irender_core::Template::is_template(u) => Ok(()),
        Json::String(u) if crate::combobox::safe_search_url(u) && cx.policy().url_allowed(u) => Ok(()),
        Json::String(u) => Err(RenderError::new(node, format!("`{what}` must be a path or an http(s) URL, got `{u}`"))),
        _ => Err(RenderError::new(node, format!("`{what}` must be a URL"))),
    }
}

/// `expand`, as `{ name: { source, from } }`: each entry an object, or the source's name
/// alone (its ids then in `<name>Id`).
pub(crate) fn expand(cx: &RenderCx<'_>, node: &Node) -> Result<Option<Json>, RenderError> {
    let Some(v) = cx.prop(node, "expand").filter(|v| !v.is_null()) else { return Ok(None) };
    let Json::Object(map) = v else {
        return Err(RenderError::new(node, "`expand` is `{ field: { source, from } }`, e.g. `{ owner: { source: people, from: ownerId } }`"));
    };
    let mut out = Map::new();
    for (name, e) in map {
        let (source, from) = match &e {
            Json::String(s) => (s.clone(), None),
            Json::Object(o) => (
                o.get("source").and_then(Json::as_str).map(String::from).unwrap_or_default(),
                o.get("from").and_then(Json::as_str).map(String::from),
            ),
            _ => (String::new(), None),
        };
        if source.is_empty() {
            return Err(RenderError::new(node, format!("`expand.{name}` needs `source`: the `name` of a `ui-data-source`")));
        }
        if name.contains('.') || name.starts_with('$') {
            return Err(RenderError::new(node, format!("`expand.{name}`: the field it fills is one name, without `.` or a leading `$`")));
        }
        out.insert(name.clone(), json!({ "source": source, "from": from.unwrap_or_else(|| format!("{name}Id")) }));
    }
    Ok(Some(Json::Object(out)))
}

// ---------------------------------------------------------------- source

pub struct DataSource;

const SOURCE_PROPS: &[PropDef] = &[
    PropDef::new("name", PropKind::String, "What queries and tables call it in `expand`, e.g. `people`").required(),
    PropDef::new("key", PropKind::String, "The field that holds a record's id (default `id`)"),
    PropDef::new(
        "batch",
        PropKind::Any,
        "Many ids in one call: `{ url, method, param, style, params, body, max, pick }`. GET (default) sends `<url>?ids=1,2,3` (`param` names it; `style: repeat` sends `ids=1&ids=2`); POST sends `{ ids: [...] }` with `body`'s fields. At most `max` ids a call (default 50). The answer is a list of records or a map by id, at `pick` (default the answer, or its `rows`, `data` or `items`)",
    ),
    PropDef::new("one", PropKind::String, "One id a call, where there's no `batch`: GET `<url>` with `{id}` in it, e.g. `/api/people/{id}`; 404 means there's none"),
    PropDef::new("pick", PropKind::String, "With `one`: the record's path in the answer, e.g. `data`"),
    PropDef::new("ttl", PropKind::Number, "Seconds a record is kept before it's fetched again (default 300); it's shown meanwhile"),
    PropDef::new("concurrency", PropKind::Number, "Calls at a time (default 4)"),
    PropDef::new("wait", PropKind::Number, "Milliseconds to gather ids before calling (default 0: what's asked for in the same turn)"),
    PropDef::new("headers", PropKind::Any, "Request headers, e.g. `{ x-tenant: acme }`"),
    KEEPING[0],
    KEEPING[1],
    KEEPING[2],
    KEEPING[3],
    PropDef::new("clear", PropKind::Any, "When this becomes true, its records (of `scope`, or every one without) are let go from the device and the page, e.g. `{{ !user }}`"),
];

impl Component for DataSource {
    fn name(&self) -> &'static str {
        "ui-data-source"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Says how one kind of record is fetched by id (people, items, accounts), for `expand` on tables and queries. Ids asked for together are fetched together, each once, and kept in memory for `ttl`. Draws nothing.",
            props: SOURCE_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["scope", "clear"], &["name", "key", "batch", "one", "pick", "ttl", "concurrency", "wait", "headers", "persist", "private", "retain"], &[])?;
        check_keeping(cx, node)?;
        if !config.get("name").and_then(Json::as_str).is_some_and(|n| !n.is_empty()) {
            return Err(RenderError::new(node, "`ui-data-source` needs `name`"));
        }
        match (config.get("batch"), config.get("one")) {
            (None, None) => return Err(RenderError::new(node, "`ui-data-source` needs `batch` (many ids a call) or `one` (an id a call)")),
            (Some(Json::Object(b)), _) => {
                check(cx, node, "batch.url", b.get("url").unwrap_or(&Json::Null))?;
                if let Some(m) = b.get("method").and_then(Json::as_str).filter(|m| !m.eq_ignore_ascii_case("GET") && !m.eq_ignore_ascii_case("POST")) {
                    return Err(RenderError::new(node, format!("`batch.method` is GET or POST, got `{m}`")));
                }
                if let Some(s) = b.get("style").and_then(Json::as_str).filter(|s| *s != "comma" && *s != "repeat") {
                    return Err(RenderError::new(node, format!("`batch.style` is comma or repeat, got `{s}`")));
                }
            }
            (Some(_), _) => return Err(RenderError::new(node, "`batch` is `{ url, method, param, max, pick }`")),
            _ => {}
        }
        if let Some(one) = config.get("one") {
            check(cx, node, "one", one)?;
            if !one.as_str().is_some_and(|u| u.contains("{id}")) {
                return Err(RenderError::new(node, "`one` needs `{id}` where the id goes, e.g. `/api/people/{id}`"));
            }
        }
        scripts(cx);
        let mut el = cx.element("div", node);
        el.attr("hidden", true).behavior("ui-data-source", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- query

pub struct DataQuery;

const QUERY_PROPS: &[PropDef] = &[
    PropDef::new("into", PropKind::StateKey, "State key that gets the rows (or the record, for one id)").required(),
    PropDef::new("url", PropKind::Any, "GET this for the rows; may use state, and a change loads again. The answer is a list, `{ rows, total }`, or one record"),
    PropDef::new("params", PropKind::Any, "Query parameters for `url`; blank ones are left out, and a change loads again"),
    PropDef::new("pick", PropKind::String, "Path to the rows in the answer (default the answer, or its `rows`, `data` or `items`)"),
    PropDef::new("source", PropKind::String, "Instead of `url`: the records of this `ui-data-source` for `ids`"),
    PropDef::new("ids", PropKind::Any, "With `source`: an id (into gets the record) or a list (the records); may use state"),
    PropDef::new(
        "expand",
        PropKind::Any,
        "Joins records into each row: `{ owner: { source: people, from: ownerId } }` puts the `people` record whose id is in `ownerId` in `owner` (`from` may hold a list of ids). While it's on its way the row's `$loading.owner` is true; `$failed.owner` when it couldn't be fetched",
    ),
    PropDef::new("status", PropKind::StateKey, "State key that gets `{ state, pending, total, error, at }`: state is idle, loading, joining (rows here, joined records on their way), ready or error"),
    PropDef::new("refresh", PropKind::Any, "Loads again when this changes, e.g. `{{ saved }}`"),
    PropDef::new("paused", PropKind::Any, "Nothing is loaded while true"),
    PropDef::new("headers", PropKind::Any, "Request headers for `url`"),
    KEEPING[0],
    KEEPING[1],
    KEEPING[2],
    KEEPING[3],
    PropDef::new(
        "sync",
        PropKind::Any,
        "Asks only for what changed since the last answer: `true` or `{ param, cursor, deleted, tombstone, key, full }`. The answer's `cursor` (default `cursor`) is sent back as `?since=` (`param`); its rows replace those with the same `key` (default `id`) or are added, and rows whose `tombstone` field (default `deleted`) is true, or whose ids are in its `deleted` list, go. Everything is loaded again every `full` hours (default 24)",
    ),
    PropDef::new("every", PropKind::Number, "Seconds between loads while the page is in sight (at least 5); it also loads when the connection comes back"),
    PropDef::new("clear", PropKind::Any, "When this becomes true, the answers kept for `scope` are let go and the rows emptied, and nothing loads while it stays true, e.g. `{{ !user }}`"),
];

impl Component for DataQuery {
    fn name(&self) -> &'static str {
        "ui-data-query"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Fills a state key with rows from an API, or with records by id, joined to the records they name (`expand`), for a chart, a list or any component. The rows come first; joined fields follow as their records arrive. Draws nothing.",
            props: QUERY_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["url", "params", "ids", "refresh", "paused", "scope", "clear"], &["pick", "source", "headers", "persist", "private", "retain", "sync", "every"], &["into", "status"])?;
        check_keeping(cx, node)?;
        match config.get("sync") {
            None | Some(Json::Bool(_) | Json::Object(_)) => {}
            Some(_) => return Err(RenderError::new(node, "`sync` is `true` or `{ param, cursor, deleted, tombstone, key, full }`")),
        }
        if config.contains_key("sync") && config.contains_key("source") {
            return Err(RenderError::new(node, "`sync` is for rows from `url`; records from a `source` follow its `ttl`"));
        }
        widget::need(cx, node, "into")?;
        match (node.prop("url").is_some(), config.contains_key("source")) {
            (false, false) => return Err(RenderError::new(node, "`ui-data-query` needs `url`, or `source` and `ids`")),
            (true, true) => return Err(RenderError::new(node, "`ui-data-query` takes `url` or `source`, not both")),
            (false, true) if node.prop("ids").is_none() => return Err(RenderError::new(node, "`source` needs `ids`: an id or a list of them")),
            _ => {}
        }
        if let Some(u) = node.prop("url").filter(|u| !cx.is_reactive(u)) {
            check(cx, node, "url", &cx.eval(u))?;
        }
        if let Some(e) = expand(cx, node)? {
            config.insert("expand".into(), e);
        }
        scripts(cx);
        let mut el = cx.element("div", node);
        el.attr("hidden", true).behavior("ui-data-query", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}
