// irender-ui-kit: the `ui-data-table` behaviour. The server renders every row; this
// sorts, filters, searches and pages them by moving and hiding the rendered rows, so
// what's inside a cell keeps its DOM. It also keeps the selection, the expanded rows,
// hidden columns, formatted values, summaries, the toolbar and the footer up to date.
// Everything it writes is textContent, never HTML, apart from its own icons.
(window.__irenderBehaviors ||= {})["ui-data-table"] = (el, api) => {
  const cfg = api.config, B = cfg.bind;
  // Elements of this table, not of a table inside one of its rows.
  const $$ = (s) => [...el.querySelectorAll(s)].filter((n) => n.closest("[data-i-x=ui-data-table]") === el);
  const $ = (s) => $$(s)[0] ?? null;
  const body = $("[data-dt-body]");
  const cols = cfg.columns, byKey = new Map(cols.map((c) => [c.key, c]));
  const lead = (cfg.selectable ? 1 : 0) + (cfg.detail ? 1 : 0);

  // ---- values
  const isNum = (v) => typeof v === "number" || (typeof v === "string" && v.trim() !== "" && !isNaN(v));
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const blank = (v) => v === null || v === undefined || v === "";
  const looksLikeDate = (v) => typeof v === "string" && /^\d{4}-\d{2}/.test(v) && !isNaN(Date.parse(v));
  const dateOnly = (v) => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v);
  const at = (r, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), r);
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const keyOf = (v) => JSON.stringify(v ?? null);
  const num = (n) => Number(n).toLocaleString(cfg.locale || undefined);
  const text = (v) => (blank(v) ? "" : typeof v === "object" ? JSON.stringify(v) : String(v));

  // Filters use ui-chart's `filter` conditions (`matches` in chart.js), so the same
  // object filters a table and a chart. Two additions: a list-valued field (tags)
  // matches when any of its values does, and a date-only `to` takes in the whole day.
  const same = (a, b) =>
    a === b || (!blank(a) && !blank(b) && typeof a !== "object" && typeof b !== "object" && String(a) === String(b));
  const cmp = (a, b) => {
    if (isNum(a) && isNum(b)) return Number(a) - Number(b);
    if (looksLikeDate(a) && looksLikeDate(b)) return Date.parse(a) - Date.parse(b);
    return String(a ?? "").localeCompare(String(b ?? ""), cfg.locale || undefined, { numeric: true, sensitivity: "base" });
  };
  const endOfDay = (b) => (dateOnly(b) ? b + "T23:59:59.999" : b);
  const OPS = {
    from: (v, b) => cmp(v, b) >= 0,
    to: (v, b) => cmp(v, looksLikeDate(v) ? endOfDay(b) : b) <= 0,
    gt: (v, b) => cmp(v, b) > 0,
    gte: (v, b) => cmp(v, b) >= 0,
    lt: (v, b) => cmp(v, b) < 0,
    lte: (v, b) => cmp(v, b) <= 0,
    contains: (v, b) => fold(v).includes(fold(b)),
  };
  function matches(v, c, field) {
    if (blank(c)) return true;
    if (Array.isArray(v)) return !v.length ? matches(null, c, field) || (Array.isArray(c) && !c.length) : v.some((x) => matches(x, c, field));
    if (Array.isArray(c)) return !c.length || c.some((x) => same(v, isObj(x) ? x[field] : x));
    if (!isObj(c)) return same(v, c);
    if (Object.hasOwn(c, "not")) return blank(c.not) || !matches(v, c.not, field);
    const ops = Object.keys(c).filter((k) => Object.hasOwn(OPS, k));
    if (!ops.length) return same(v, c[field]);
    return ops.every((k) => blank(c[k]) || (!blank(v) && OPS[k](v, c[k])));
  }
  const active = (c) => !blank(c) && !(Array.isArray(c) && !c.length) && !(isObj(c) && Object.values(c).every(blank));

  // ---- props that use state, resolved against it (and the list item the table is in)
  const env = () => {
    const s = api.scope();
    return s ? { ...api.state, ...s } : api.state;
  };
  // Values with nothing to resolve come back as they are, so static rows keep their
  // identity and `bindView` settles.
  const live = (v) => (Array.isArray(v) ? v.some(live) : isObj(v) && (("$irender" in v && Object.keys(v).length === 1) || Object.values(v).some(live)));
  const resolve = (v) => {
    if (!live(v)) return v;
    if (Array.isArray(v)) return v.map(resolve);
    if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
  };
  const rows = () => {
    const r = resolve(cfg.rows);
    return Array.isArray(r) ? r : [];
  };
  const rowKey = (r, i) => {
    const k = isObj(r) ? at(r, cfg.rowKey) : undefined;
    return blank(k) ? i : k;
  };

  // ---- view state: in state when bound, here otherwise
  const local = {
    sort: cfg.sort ?? null,
    filters: cfg.filters ?? {},
    search: cfg.searchText ?? "",
    page: 1,
    selection: [],
  };
  let size = cfg.pageSize;
  // `pages`, `more`, `scroll` or `none`; with `more` and `scroll`, how many rows are
  // revealed so far.
  const mode = cfg.paging;
  let loaded = size;
  const remote = cfg.url != null;
  const hidden = new Set(cols.filter((c) => c.hidden).map((c) => c.key));
  const expanded = new Set();
  const get = (name) => {
    const v = B[name] ? api.state[B[name]] : undefined;
    return v ?? local[name];
  };
  function put(name, v) {
    local[name] = v;
    if (B[name]) api.set(B[name], v);
    else apply();
  }
  const selection = () => {
    const s = get("selection");
    return Array.isArray(s) ? s : [];
  };
  const filters = () => (isObj(get("filters")) ? get("filters") : {});

  // `field`, `-field`, `{ field, order }` or a list, as [{ field, desc }].
  const sortList = (s) =>
    (Array.isArray(s) ? s : blank(s) ? [] : [s]).map((k) =>
      isObj(k) ? { field: String(k.field), desc: k.order === "desc" }
      : String(k).startsWith("-") ? { field: String(k).slice(1), desc: true }
      : { field: String(k), desc: false });
  const sortValue = (list) => {
    const out = list.map((k) => (k.desc ? "-" : "") + k.field);
    return out.length === 0 ? null : out.length === 1 ? out[0] : out;
  };

  // ---- formatting
  const formats = new Map();
  function fmt(c, v) {
    if (blank(v) || !c.format) return text(v);
    const loc = cfg.locale || undefined;
    if (c.format === "date" || c.format === "datetime") {
      const d = new Date(v);
      if (isNaN(d)) return text(v);
      const key = c.key + (dateOnly(v) ? ":utc" : "");
      if (!formats.has(key)) {
        const o = c.format === "date" ? { dateStyle: "medium" } : { dateStyle: "medium", timeStyle: "short" };
        // A plain date is midnight UTC; shown in UTC it stays the same day everywhere.
        if (dateOnly(v)) o.timeZone = "UTC";
        formats.set(key, new Intl.DateTimeFormat(loc, o));
      }
      return formats.get(key).format(d);
    }
    if (!isNum(v)) return text(v);
    if (!formats.has(c.key)) {
      const o = c.format === "currency" ? { style: "currency", currency: c.currency || "USD" } : c.format === "percent" ? { style: "percent" } : {};
      if (c.decimals != null) o.minimumFractionDigits = o.maximumFractionDigits = Math.max(0, Math.min(20, c.decimals));
      formats.set(c.key, new Intl.NumberFormat(loc, o));
    }
    return formats.get(c.key).format(Number(v));
  }
  const SUM = {
    sum: (vs) => vs.reduce((a, b) => a + b, 0),
    avg: (vs) => (vs.length ? vs.reduce((a, b) => a + b, 0) / vs.length : null),
    min: (vs) => (vs.length ? Math.min(...vs) : null),
    max: (vs) => (vs.length ? Math.max(...vs) : null),
  };

  // ---- the rows that pass, in order
  const searchFields = Array.isArray(cfg.searchFields) ? cfg.searchFields : cols.map((c) => c.field).filter(Boolean);
  // Every condition but the one on `skip` (a select filter counts its options this way).
  let outside = null; // `filter`, resolved once per apply
  function passes(r, skip) {
    if (isObj(outside)) for (const [f, c] of Object.entries(outside)) if (!matches(at(r, f), c, f)) return false;
    for (const [k, c] of Object.entries(filters())) {
      if (k === skip) continue;
      const f = byKey.get(k)?.field ?? k;
      if (!matches(at(r, f), c, f)) return false;
    }
    const q = fold(get("search")).trim();
    if (q && !q.split(/\s+/).every((w) => searchFields.some((f) => fold(text(at(r, f))).includes(w)))) return false;
    return true;
  }
  // Two rows by the sort keys; blanks go last either way.
  function cmpRows(keys, r, s) {
    for (const { field, desc } of keys) {
      const x = at(r, field), y = at(s, field);
      if (blank(x) || blank(y)) {
        if (blank(x) !== blank(y)) return blank(x) ? 1 : -1;
        continue;
      }
      const c = cmp(x, y);
      if (c) return desc ? -c : c;
    }
    return 0;
  }
  function view(data) {
    const idx = [];
    data.forEach((r, i) => passes(r) && idx.push(i));
    const keys = sortList(get("sort"));
    if (keys.length) idx.sort((a, b) => cmpRows(keys, data[a], data[b]) || a - b);
    return idx;
  }

  // ---- nested rows: the open ones flattened into the order shown, with each row's
  // depth, whether it has children and is open, and its parent's index.
  const tree = cfg.tree;
  let meta = [], treeVersion = 0, treeCache = null, byParent = new Map();
  const kids = new Map(), kidsLoading = new Set(), kidsFailed = new Set();
  // Rows opened (or, with `expanded`, closed) by hand.
  const toggled = new Set();
  const isOpen = (k) => (tree.all ? !toggled.has(k) : toggled.has(k));
  function childrenOf(r, i) {
    const k = keyOf(rowKey(r, i));
    if (kids.has(k)) return kids.get(k);
    if (tree.parent) return byParent.get(k) ?? null;
    const c = isObj(r) ? at(r, tree.field) : null;
    return Array.isArray(c) ? c : null;
  }
  // Children still to load: the row says it has some (`children: true`, or
  // `hasChildren: true`) and `childrenUrl` will give them.
  const lazy = (r, i) => tree.url != null && isObj(r) && !kids.has(keyOf(rowKey(r, i))) && (at(r, tree.field) === true || r.hasChildren === true);
  function flatTree(base, q) {
    const sig = [base, q, treeVersion];
    if (treeCache && treeCache.sig.every((x, n) => x === sig[n])) return treeCache;
    let roots = base;
    if (tree.parent) {
      // Flat rows: a row whose parent is missing is a root.
      const known = new Set(base.map((r, i) => keyOf(rowKey(r, i))));
      byParent = new Map();
      roots = [];
      for (const r of base) {
        const p = isObj(r) ? at(r, tree.parent) : null;
        if (blank(p) || !known.has(keyOf(p))) roots.push(r);
        else byParent.has(keyOf(p)) ? byParent.get(keyOf(p)).push(r) : byParent.set(keyOf(p), [r]);
      }
    }
    // While filtering, a row shows when it or a row under it passes, and rows with
    // matches under them open.
    const filtering = !remote && (Object.values(filters()).some(active) || !!String(get("search") ?? "").trim() || (isObj(outside) && Object.values(outside).some(active)));
    const keep = new Map();
    const kept = (r) => {
      if (!filtering) return true;
      if (keep.has(r)) return keep.get(r);
      keep.set(r, false);
      const v = passes(r) || (childrenOf(r) ?? []).some(kept);
      keep.set(r, v);
      return v;
    };
    const keys = sortList(get("sort"));
    const out = [], info = [];
    const walk = (list, depth, parent) => {
      let rows = list.filter(kept);
      // Siblings sort among themselves; the server already sorted remote top rows.
      if (keys.length && !(remote && depth === 0)) rows = rows.map((r, n) => [r, n]).sort(([a, n], [b, m]) => cmpRows(keys, a, b) || n - m).map(([r]) => r);
      for (const r of rows) {
        const i = out.length;
        out.push(r);
        const k = keyOf(rowKey(r, i)), ch = childrenOf(r, i);
        const has = (ch?.length ?? 0) > 0 || lazy(r, i);
        const open = has && (isOpen(k) || (filtering && (ch ?? []).some(kept)));
        info.push({ depth, has, open, parent, loading: kidsLoading.has(k), failed: kidsFailed.has(k) });
        if (open && ch) walk(ch, depth + 1, i);
      }
    };
    walk(roots, 0, -1);
    treeCache = { sig, rows: out, meta: info };
    return treeCache;
  }
  function toggleNode(i, want) {
    const r = data[i], m = meta[i];
    if (!m?.has) return;
    want ??= !m.open;
    if (want === m.open) return;
    const k = keyOf(rowKey(r, i));
    if (tree.all !== want) toggled.add(k);
    else toggled.delete(k);
    treeVersion++;
    if (want && lazy(r, i) && !kidsLoading.has(k)) loadChildren(r, i, k);
    apply();
  }
  async function loadChildren(r, i, k) {
    kidsLoading.add(k);
    kidsFailed.delete(k);
    try {
      const u = text(resolve(tree.url));
      if (!okUrl(u)) throw new Error("url");
      const p = new URLSearchParams({ parent: text(rowKey(r, i)) });
      const sort = sortValue(sortList(get("sort")));
      if (sort) p.set("sort", Array.isArray(sort) ? sort.join(",") : sort);
      const res = await fetch(u + (u.includes("?") ? "&" : "?") + p, { headers: { accept: "application/json" } });
      if (!res.ok) throw new Error(String(res.status));
      const answer = await res.json();
      kids.set(k, Array.isArray(answer) ? answer : Array.isArray(answer?.rows) ? answer.rows : []);
    } catch {
      // Closed again, so opening it retries.
      kidsFailed.add(k);
      if (tree.all) toggled.add(k);
      else toggled.delete(k);
    }
    kidsLoading.delete(k);
    treeVersion++;
    apply();
  }

  // ---- the DOM's rows: each row's nodes (its <tr> and detail row), by row index.
  // A listed table (virtual or remote) numbers its rows by their place in `shown`.
  const indexOf = (tr) => {
    const n = Number(tr.getAttribute("data-dt-row"));
    return cfg.listed ? shown[n] : n;
  };
  function groups() {
    const out = new Map();
    let cur = null;
    for (const n of [...body.childNodes]) {
      if (n.nodeType === 1 && n.hasAttribute("data-dt-row")) {
        cur = [n];
        out.set(indexOf(n), cur);
      } else if (cur) cur.push(n);
    }
    return out;
  }
  const rowEl = (g) => g[0];
  const detailEl = (g) => g.find((n) => n.nodeType === 1 && n.hasAttribute("data-dt-detail"));

  // ---- apply: the whole view, from the state
  let shown = [], filtered = [], data = [], lastView = null, lastCount = null, lastClicked = null;
  const ARROW = '<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 13V3M4 7l4-4 4 4"/></svg>';
  const BOTH = '<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 6l3-3 3 3M5 10l3 3 3-3"/></svg>';

  let applies = 0, page = 1, pages = 1, query = null, viewOf = null, listSig = null, listData = null, listStart = 0;
  function apply() {
    applies++;
    outside = resolve(cfg.filter);
    page = Math.max(1, Math.trunc(Number(get("page")) || 1));
    // New filters, search or sort start again from the first batch, at the top.
    const q = JSON.stringify([get("sort"), filters(), get("search"), outside]);
    if (q !== query) {
      if (query !== null && mode !== "pages") {
        loaded = size;
        scroller.scrollTop = 0;
      }
      query = q;
    }
    if (remote) sync();
    const base = remote ? R.rows : rows();
    if (tree) {
      // Nested rows come flattened, already filtered and sorted.
      const t = flatTree(base, q);
      data = t.rows;
      meta = t.meta;
      if (viewOf?.data !== data) {
        filtered = data.map((_, i) => i);
        viewOf = { data, q };
      }
    } else {
      data = base;
      // The server sorted and filtered remote rows; local ones are sorted and filtered
      // here, again only when the rows or the query change.
      if (viewOf?.data !== data || viewOf.q !== q) {
        filtered = remote ? data.map((_, i) => i) : view(data);
        viewOf = { data, q };
      }
    }
    pages = mode !== "pages" ? 1
      : !remote ? Math.max(1, Math.ceil(filtered.length / size))
      : R.total != null ? Math.max(1, Math.ceil(R.total / size))
      : R.done ? Math.max(1, R.page) : Math.max(page, R.page + 1);
    if (page > pages) {
      page = pages;
      local.page = page;
    }
    shown = remote || mode === "none" ? filtered
      : mode === "pages" ? filtered.slice((page - 1) * size, page * size)
      : filtered.slice(0, loaded);
    const showing = new Set(shown);

    const g = groups();
    if (cfg.listed) {
      // Only the rows in view (or every shown row, when not virtual), handed to the
      // runtime, which keeps the nodes of rows that stay.
      const [s, e] = windowOf(shown.length);
      pad("top", s * est);
      pad("bottom", (shown.length - e) * est);
      const sig = s + ":" + shown.slice(s, e).join(",");
      if (sig !== listSig || data !== listData) {
        listSig = sig;
        listData = data;
        listStart = s;
        const n = applies;
        api.list(body, shown.slice(s, e).map((i) => data[i]), s);
        if (applies !== n) return; // the redraw applied everything again
      }
    } else {
      // Rows: in order, the shown ones first, every other one hidden.
      const passing = new Set(filtered);
      const order = [...filtered, ...[...g.keys()].filter((i) => !passing.has(i))];
      const current = [...g.keys()];
      if (order.some((i, n) => current[n] !== i)) {
        const focused = body.contains(document.activeElement) ? document.activeElement : null;
        for (const i of order) if (g.has(i)) body.append(...g.get(i));
        if (focused?.isConnected && document.activeElement !== focused) focused.focus({ preventScroll: true });
      }
    }
    const chosen = new Set(selection().map(keyOf));
    let n = 0;
    for (const [i, nodes] of g) {
      const tr = rowEl(nodes), on = showing.has(i);
      tr.hidden = !on;
      const d = detailEl(nodes);
      const key = keyOf(rowKey(data[i], i));
      const open = on && expanded.has(key);
      if (d) d.hidden = !open;
      tr.querySelector("[data-dt-expand]")?.setAttribute("aria-expanded", String(open));
      if (on) tr.toggleAttribute("data-odd", n++ % 2 === 1);
      if (cfg.selectable) {
        const sel = chosen.has(key);
        tr.setAttribute("aria-selected", String(sel));
        const box = tr.querySelector("[data-dt-select]");
        if (box) box.checked = sel;
      }
      if (on) for (const x of tr.querySelectorAll(":scope > [data-dt-fmt], :scope > td > [data-dt-node] > [data-dt-fmt]")) {
        const c = byKey.get(x.closest("[data-dt-col]").getAttribute("data-dt-col"));
        x.textContent = fmt(c, at(data[i], c.field));
      }
      if (tree && meta[i]) {
        const m = meta[i];
        tr.setAttribute("aria-level", String(m.depth + 1));
        if (m.has) tr.setAttribute("aria-expanded", String(m.open));
        else tr.removeAttribute("aria-expanded");
        const cell = tr.querySelector(":scope > [data-dt-tree]");
        cell?.style.setProperty("--dt-depth", String(m.depth));
        const twisty = cell?.querySelector("[data-dt-twisty]");
        if (twisty) {
          twisty.hidden = !m.has;
          twisty.toggleAttribute("data-loading", m.loading);
          twisty.title = m.failed ? "Couldn't load the rows under this one" : "";
        }
      }
    }
    roving();
    if (cfg.virtual) measure(g);

    // Columns shown and the cells that span them.
    for (const cell of $$("[data-dt-col]")) cell.hidden = hidden.has(cell.getAttribute("data-dt-col"));
    const span = String(cols.length - hidden.size + lead);
    for (const td of $$("[data-dt-detail] > td, [data-dt-empty] > td")) td.colSpan = span;
    for (const box of $$("[data-dt-show]")) box.checked = !hidden.has(box.getAttribute("data-dt-show"));
    const empty = $("[data-dt-empty]");
    if (empty) {
      empty.hidden = filtered.length > 0;
      empty.firstElementChild.textContent = remote && R.loading ? "Loading…" : remote && R.error ? "Couldn't load rows." : emptyText;
    }

    // Sorting.
    const keys = sortList(get("sort"));
    for (const b of $$("[data-dt-sort]")) {
      const c = byKey.get(b.getAttribute("data-dt-sort"));
      const k = keys.findIndex((s) => s.field === c.sortField);
      const th = b.closest("th");
      if (k < 0) th.removeAttribute("aria-sort");
      else th.setAttribute("aria-sort", keys[k].desc ? "descending" : "ascending");
      const arrow = b.querySelector("[data-dt-arrow]");
      const html = (k < 0 ? BOTH : ARROW) + (k >= 0 && keys.length > 1 ? `<span data-dt-rank>${k + 1}</span>` : "");
      if (arrow.innerHTML !== html) arrow.innerHTML = html;
    }

    // Filters: the triggers' chips, the controls' values, Reset.
    const fs = filters();
    for (const t of $$("[data-dt-trigger]")) {
      const key = t.getAttribute("data-dt-trigger"), c = fs[key];
      t.toggleAttribute("data-active", active(c));
      const chips = t.querySelector("[data-dt-chips]");
      const words = !active(c) ? []
        : Array.isArray(c) ? (c.length > 2 ? [`${c.length} selected`] : c.map(text))
        : isObj(c) ? [c.contains != null ? `“${c.contains}”` : range(byKey.get(key), c)]
        : [text(c)];
      const now = words.join("\u0000");
      if (chips.__w !== now) {
        chips.__w = now;
        chips.replaceChildren(...words.map((w) => Object.assign(document.createElement("span"), { textContent: w })));
        chips.querySelectorAll("span").forEach((s) => s.setAttribute("data-dt-chip", ""));
      }
    }
    for (const p of $$("[data-dt-filter]")) {
      const c = fs[p.getAttribute("data-dt-filter")];
      for (const i of p.querySelectorAll("[data-dt-op]")) {
        if (i === document.activeElement) continue;
        const v = isObj(c) ? c[i.getAttribute("data-dt-op")] : null;
        i.value = blank(v) ? "" : String(v);
      }
      if (p.matches(":popover-open")) options(p);
    }
    const search = $("[data-dt-search]");
    if (search && search !== document.activeElement) search.value = get("search") ?? "";
    const reset = $("[data-dt-reset]");
    if (reset) reset.hidden = !Object.values(fs).some(active) && !String(get("search") ?? "").trim();

    // Selection: the header box, the bulk bar, the count.
    const pageKeys = shown.map((i) => keyOf(rowKey(data[i], i)));
    const onPage = pageKeys.filter((k) => chosen.has(k)).length;
    const all = $("[data-dt-select-page]");
    if (all) {
      all.checked = shown.length > 0 && onPage === shown.length;
      all.indeterminate = onPage > 0 && onPage < shown.length;
    }
    const count = selection().length;
    const bulk = $("[data-dt-bulk]");
    if (bulk) {
      bulk.hidden = count === 0;
      $("[data-dt-count]").textContent = `${count} selected`;
      const allOf = $("[data-dt-select-all]");
      const inView = filtered.filter((i) => chosen.has(keyOf(rowKey(data[i], i)))).length;
      allOf.hidden = !(shown.length && onPage === shown.length && inView < filtered.length);
      allOf.textContent = `Select all ${filtered.length}`;
    }
    const status = $("[data-dt-status]");
    const picked = cfg.selectable ? ` · ${count} selected` : "";
    if (remote) {
      const total = R.total ?? (R.done ? data.length : null);
      const first = mode === "pages" ? (R.page - 1) * size : 0;
      status.textContent = R.loading && !data.length ? "Loading…"
        : !data.length ? "0 rows" + picked
        : mode === "pages" ? `Rows ${num(first + 1)}–${num(first + data.length)}${total != null ? ` of ${num(total)}` : ""}${picked}`
        : `Showing ${num(data.length)}${total != null ? ` of ${num(total)}` : ""}${picked}`;
    } else if (mode === "more" || mode === "scroll") {
      status.textContent = `Showing ${num(shown.length)} of ${num(filtered.length)}${filtered.length < data.length ? ` (of ${num(data.length)})` : ""}${picked}`;
    } else {
      const of = filtered.length === data.length ? num(data.length) : `${num(filtered.length)} (of ${num(data.length)})`;
      status.textContent = cfg.selectable ? `${count} of ${of} row(s) selected.` : `${of} row(s)`;
    }
    // The next batch: a button, or a note while it loads, or a retry.
    if (more) {
      const busy = remote && R.loading, failed = remote && R.error;
      more.hidden = !(hasMore() || busy || failed) || (busy && !data.length);
      more.querySelector("[data-dt-loading]").hidden = failed || !(busy || (mode === "scroll" && hasMore()));
      const load = more.querySelector("[data-dt-load]");
      if (load) load.hidden = busy || failed || !hasMore();
      more.querySelector("[data-dt-retry]").hidden = !failed;
    }
    el.toggleAttribute("data-loading", remote && R.loading);

    // Paging.
    const pageText = $("[data-dt-page]");
    if (pageText) pageText.textContent = `Page ${num(page)} of ${num(pages)}`;
    for (const b of $$("[data-dt-go]")) {
      const go = b.getAttribute("data-dt-go");
      // Without a total, the last page is known once a short page comes back.
      b.disabled = go === "first" || go === "prev" ? page <= 1 : go === "last" && remote && R.total == null ? true : page >= pages;
    }
    if (pageText && remote && R.total == null && !R.done) pageText.textContent = `Page ${page}`;
    const sizes = $("[data-dt-size]");
    if (sizes) sizes.value = String(size);

    // Summaries, over every row that passes.
    for (const td of $$("[data-dt-sum]")) {
      const c = byKey.get(td.getAttribute("data-dt-sum"));
      if (c.summary === "count") {
        td.textContent = String(filtered.length);
        continue;
      }
      // Nested rows count once: only the top ones are added up.
      const vs = filtered.filter((i) => !tree || meta[i]?.depth === 0).map((i) => at(data[i], c.field)).filter(isNum).map(Number);
      const v = SUM[c.summary](vs);
      td.textContent = v === null ? "" : fmt({ ...c, format: c.format || "number" }, Math.round(v * 1e9) / 1e9);
    }

    // The result count, for screen readers, when the filters change it.
    if (lastCount !== null && lastCount !== filtered.length) $("[data-dt-footer] [data-dt-live]").textContent = `${filtered.length} result${filtered.length === 1 ? "" : "s"}`;
    lastCount = filtered.length;
    if (mode === "scroll") setTimeout(nearEnd);
    pinLayout();

    // The rows that pass, for charts. Only written when they change, so this settles.
    if (B.view) {
      const sig = filtered.join(",");
      if (lastView?.data !== data || lastView.sig !== sig) {
        lastView = { data, sig };
        const out = filtered.map((i) => data[i]);
        queueMicrotask(() => api.set(B.view, out));
      }
    }
  }

  // ---- more rows: revealed here, or loaded from `url`
  const scroller = $("[data-dt-scroll]"), more = $("[data-dt-more]");
  const emptyText = $("[data-dt-empty]")?.textContent ?? "";
  const hasMore = () => (remote ? !R.done && mode !== "pages" : (mode === "more" || mode === "scroll") && loaded < filtered.length);
  function loadMore() {
    if (remote) {
      if (!R.loading && !R.done && !R.error) {
        fetchBatch();
        apply();
      }
      return;
    }
    if (!hasMore()) return;
    loaded += size;
    apply();
  }
  // With `scroll`, the next batch comes when the end of the rows is near the bottom of
  // the table's own scrolling area, or of the window.
  function nearEnd() {
    if (mode !== "scroll" || !more || more.hidden || !hasMore() || (remote && (R.loading || R.error))) return;
    const r = more.getBoundingClientRect();
    if (!r.width && !r.height) return;
    const own = scroller.scrollHeight > scroller.clientHeight + 1;
    const bottom = own ? Math.min(innerHeight, scroller.getBoundingClientRect().bottom) : innerHeight;
    if (r.top < bottom + 200) loadMore();
  }

  // Remote rows: one query (sort, search, filters, params) at a time; a new one starts
  // over, and answers to an old one are dropped.
  const R = { rows: [], total: null, done: false, loading: false, error: false, page: 0, key: null, seq: 0 };
  let timer = null;
  const okUrl = (u) => {
    const l = u.trim().toLowerCase();
    if (!l || l.includes("\\") || l.startsWith("//") || /[\u0000-\u001f]/.test(l)) return false;
    if (/^https?:\/\//.test(l)) return true;
    const end = l.search(/[/?#]/);
    return !(end < 0 ? l : l.slice(0, end)).includes(":");
  };
  function remoteQuery() {
    const fs = isObj(outside) ? { ...outside } : {};
    for (const [k, c] of Object.entries(filters())) if (active(c)) fs[byKey.get(k)?.field ?? k] = c;
    const sort = sortValue(sortList(get("sort")));
    return {
      url: text(resolve(cfg.url)),
      sort: Array.isArray(sort) ? sort.join(",") : sort,
      q: String(get("search") ?? "").trim(),
      filters: Object.keys(fs).length ? JSON.stringify(fs) : null,
      params: resolve(cfg.params),
      size,
    };
  }
  function sync() {
    const key = JSON.stringify(remoteQuery());
    if (key !== R.key) {
      const first = R.key === null;
      Object.assign(R, { key, rows: [], total: null, done: false, error: false, page: 0, loading: true });
      R.seq++;
      clearTimeout(timer);
      // Typing waits for a pause; the first load goes at once.
      timer = setTimeout(() => fetchBatch(), first ? 0 : 250);
    } else if (mode === "pages" && page !== R.page && !R.loading && !R.error) {
      fetchBatch();
    }
  }
  async function fetchBatch() {
    const q = JSON.parse(R.key), at = mode === "pages" ? page : R.page + 1;
    const offset = mode === "pages" ? (at - 1) * size : R.rows.length;
    const id = ++R.seq;
    Object.assign(R, { loading: true, error: false });
    const p = new URLSearchParams();
    if (mode !== "none") {
      p.set("offset", String(offset));
      p.set("limit", String(size));
    }
    if (q.sort) p.set("sort", q.sort);
    if (q.q) p.set("q", q.q);
    if (q.filters) p.set("filters", q.filters);
    if (isObj(q.params)) for (const [k, v] of Object.entries(q.params)) if (!blank(v)) p.set(k, typeof v === "object" ? JSON.stringify(v) : String(v));
    try {
      if (!okUrl(q.url)) throw new Error("url");
      const res = await fetch(q.url + (q.url.includes("?") ? "&" : "?") + p, { headers: { accept: "application/json" } });
      if (!res.ok) throw new Error(String(res.status));
      const answer = await res.json();
      if (id !== R.seq) return;
      const batch = Array.isArray(answer) ? answer : Array.isArray(answer?.rows) ? answer.rows : [];
      const total = Number.isFinite(answer?.total) ? answer.total : null;
      Object.assign(R, {
        rows: mode === "pages" || mode === "none" ? batch : R.rows.concat(batch),
        page: at,
        total,
        done: mode === "none" || (total != null ? offset + batch.length >= total : batch.length < size),
        loading: false,
      });
    } catch {
      if (id !== R.seq) return;
      Object.assign(R, { loading: false, error: true });
    }
    apply();
  }

  // ---- virtual: only the rows in view, with space above and below for the rest
  const OVERSCAN = 6;
  let est = cfg.rowHeight;
  function windowOf(n) {
    if (!cfg.virtual || !n) return [0, n];
    const head = $("thead")?.offsetHeight ?? 0;
    const top = Math.max(0, scroller.scrollTop - head);
    const s = Math.min(Math.max(0, Math.floor(top / est) - OVERSCAN), n - 1);
    return [s, Math.min(n, s + Math.ceil((scroller.clientHeight || 480) / est) + 2 * OVERSCAN)];
  }
  function pad(at, h) {
    const td = $(`[data-dt-pad=${at}] td`);
    if (td) td.style.height = Math.max(0, Math.round(h)) + "px";
  }
  // Rows' real height, averaged over the ones drawn, open details included.
  function measure(g) {
    let h = 0, n = 0;
    for (const nodes of g.values()) {
      for (const x of nodes) if (x.nodeType === 1 && !x.hidden) h += x.getBoundingClientRect().height;
      n++;
    }
    if (n && h) est = Math.max(8, h / n);
  }
  // At most one redraw per 16 ms while scrolling, and one after the last event.
  let last = 0, trailing = null;
  const onScroll = () => {
    const run = () => {
      last = Date.now();
      trailing = null;
      if (cfg.virtual) apply();
      nearEnd();
    };
    clearTimeout(trailing);
    if (Date.now() - last >= 16) run();
    else trailing = setTimeout(run, 16);
  };
  scroller.addEventListener("scroll", onScroll, { passive: true });
  if (mode === "scroll") {
    addEventListener("scroll", onScroll, { passive: true });
    addEventListener("resize", onScroll, { passive: true });
  }

  // ---- pinned columns: each one's offset is the width of the pinned ones before it
  const table = $("table"), pinned = !!table.querySelector("thead [data-pin]");
  function pinLayout() {
    if (!pinned) return;
    const head = [...table.tHead.rows[0].cells];
    const offsets = (side, cells) => {
      const out = new Map();
      let x = 0, edge = -1;
      for (const [n, c] of cells) {
        if (c.getAttribute("data-pin") !== side || c.hidden) continue;
        out.set(n, x);
        x += c.getBoundingClientRect().width;
        edge = n;
      }
      return { out, edge };
    };
    const L = offsets("left", head.map((c, n) => [n, c]));
    const R2 = offsets("right", head.map((c, n) => [n, c]).reverse());
    for (const tr of table.rows) {
      // Detail, empty and spacer rows span the table.
      if (tr.cells.length !== head.length) continue;
      for (const [{ out, edge }, prop, side] of [[L, "left", "left"], [R2, "right", "right"]]) {
        for (const [n, x] of out) {
          const c = tr.cells[n];
          c.style[prop] = x + "px";
          if (n === edge) c.setAttribute("data-pin-edge", side);
          else c.removeAttribute("data-pin-edge");
        }
      }
    }
  }
  if (pinned) new ResizeObserver(pinLayout).observe(table);

  // A range filter's chip: `a – b`, `≥ a` or `≤ b`.
  function range(c, { from, to }) {
    if (!blank(from) && !blank(to)) return `${fmt(c, from)} – ${fmt(c, to)}`;
    return blank(to) ? `≥ ${fmt(c, from)}` : `≤ ${fmt(c, to)}`;
  }

  // ---- a select filter's choices, with how many rows have each
  function options(pop) {
    const key = pop.getAttribute("data-dt-filter"), c = byKey.get(key);
    const list = pop.querySelector("[data-dt-options]");
    if (!list) return;
    const counts = new Map();
    for (const r of data) {
      if (!remote && !passes(r, key)) continue;
      const v = at(r, c.field);
      for (const x of Array.isArray(v) ? v : [v]) {
        if (blank(x)) continue;
        const k = text(x);
        counts.set(k, (counts.get(k) ?? 0) + 1);
      }
    }
    const given = Array.isArray(c.options) ? c.options.map((o) => (isObj(o) ? { value: o.value, label: text(o.label ?? o.value) } : { value: o, label: text(o) })) : null;
    const choices = given ?? [...new Set(data.flatMap((r) => [at(r, c.field)].flat()).filter((v) => !blank(v)).map(text))]
      .sort((a, b) => cmp(a, b))
      .map((v) => ({ value: v, label: v }));
    const picked = filters()[key];
    const on = (v) => Array.isArray(picked) && picked.some((p) => same(p, v));
    const sig = JSON.stringify([choices, [...counts], picked]);
    if (list.__sig === sig) return;
    list.__sig = sig;
    if (!choices.length) {
      list.replaceChildren(Object.assign(document.createElement("div"), { textContent: "No values" }));
      list.firstChild.setAttribute("data-dt-none", "");
      return;
    }
    // Each choice is a copy of the server's `<label><input><span><span data-dt-n>`.
    const tpl = pop.querySelector("template[data-dt-choice]");
    list.replaceChildren(
      ...choices.map((o) => {
        const label = tpl.content.firstElementChild.cloneNode(true);
        const [box, name, n] = label.children;
        box.checked = on(o.value);
        box.__v = o.value;
        name.textContent = o.label;
        n.textContent = String(counts.get(text(o.value)) ?? 0);
        return label;
      }),
    );
  }

  // ---- keyboard: one row in the tab order, arrows move between shown rows
  const navigable = cfg.selectable || cfg.detail || !!B.row || !!cfg.tree;
  let focusKey = null;
  function roving() {
    if (!navigable) return;
    const g = groups();
    const trs = shown.map((i) => g.get(i)?.[0]).filter(Boolean);
    const target = trs.find((tr) => tr.__k === focusKey) ?? trs[0];
    for (const [i, nodes] of g) {
      const tr = nodes[0];
      tr.__k = keyOf(rowKey(data[i], i));
      tr.tabIndex = tr === target ? 0 : -1;
    }
  }
  const visibleRows = () => $$("[data-dt-body] > [data-dt-row]:not([hidden])");
  const rowOf = (tr) => {
    const i = indexOf(tr);
    return { i, row: data[i], key: rowKey(data[i], i) };
  };

  // ---- changes people make
  const setFilter = (key, c) => {
    const next = { ...filters() };
    if (active(c)) next[key] = c;
    else delete next[key];
    local.page = 1;
    if (B.page) api.set(B.page, 1);
    put("filters", next);
  };
  const goTo = (p) => put("page", p);
  function toggleRows(keys, on) {
    const ks = new Set(keys.map(keyOf));
    const rest = selection().filter((k) => !ks.has(keyOf(k)));
    put("selection", on ? [...rest, ...keys] : rest);
    api.emit("select", { selection: get("selection") });
  }
  function clickRow(tr) {
    const { row } = rowOf(tr);
    if (B.row) api.set(B.row, row);
    api.emit("rowclick", { row });
  }
  function toggleDetail(tr) {
    const k = keyOf(rowOf(tr).key);
    expanded.has(k) ? expanded.delete(k) : expanded.add(k);
    apply();
  }

  el.addEventListener("click", (e) => {
    const t = e.target.closest("button, [data-dt-row]");
    if (!t || !el.contains(t)) return;
    if (t.matches("[data-dt-sort]")) {
      const field = byKey.get(t.getAttribute("data-dt-sort")).sortField;
      let keys = sortList(get("sort"));
      const k = keys.find((s) => s.field === field);
      // Ascending, descending, then off. Shift adds the column to the sort.
      const next = !k ? { field, desc: false } : !k.desc ? { field, desc: true } : null;
      keys = e.shiftKey ? keys.filter((s) => s.field !== field) : [];
      if (next) keys.push(next);
      put("sort", sortValue(keys));
    } else if (t.matches("[data-dt-go]")) {
      const go = t.getAttribute("data-dt-go"), page = Math.trunc(Number(get("page")) || 1);
      goTo(go === "first" ? 1 : go === "last" ? pages : go === "prev" ? Math.max(1, page - 1) : Math.min(pages, page + 1));
    } else if (t.matches("[data-dt-twisty]")) {
      toggleNode(indexOf(t.closest("[data-dt-row]")));
    } else if (t.matches("[data-dt-load]")) {
      loadMore();
    } else if (t.matches("[data-dt-retry]")) {
      R.error = false;
      fetchBatch();
      apply();
    } else if (t.matches("[data-dt-reset]")) {
      local.page = 1;
      if (B.page) api.set(B.page, 1);
      if (B.search) api.set(B.search, "");
      local.search = "";
      put("filters", {});
    } else if (t.matches("[data-dt-clear]")) {
      setFilter(t.closest("[data-dt-filter]").getAttribute("data-dt-filter"), null);
    } else if (t.matches("[data-dt-select-all]")) {
      toggleRows(filtered.map((i) => rowKey(data[i], i)), true);
    } else if (t.matches("[data-dt-clear-selection]")) {
      put("selection", []);
      api.emit("select", { selection: [] });
    } else if (t.matches("[data-dt-expand]")) {
      toggleDetail(t.closest("[data-dt-row]"));
    } else if (t.matches("[data-dt-row]")) {
      // A click on the row itself, not on something inside it that acts (a control,
      // a link, a chart).
      const inner = e.target.closest("a, button, input, select, textarea, label, summary, [data-i-on], [data-i-x]");
      if (inner && inner !== t && t.contains(inner)) return;
      focusKey = t.__k;
      roving();
      clickRow(t);
    }
  });

  el.addEventListener("change", (e) => {
    const t = e.target;
    if (t.matches("[data-dt-select]")) {
      const tr = t.closest("[data-dt-row]"), { key } = rowOf(tr);
      // Shift selects every shown row between this one and the last one clicked.
      const rowsShown = visibleRows();
      const from = lastClicked ? rowsShown.indexOf(lastClicked) : -1, to = rowsShown.indexOf(tr);
      if (t.__shift && from >= 0 && to >= 0) {
        const span = rowsShown.slice(Math.min(from, to), Math.max(from, to) + 1).map((r) => rowOf(r).key);
        toggleRows(span, t.checked);
      } else toggleRows([key], t.checked);
      lastClicked = tr;
    } else if (t.matches("[data-dt-select-page]")) {
      toggleRows(shown.map((i) => rowKey(data[i], i)), t.checked);
    } else if (t.matches("[data-dt-show]")) {
      t.checked ? hidden.delete(t.getAttribute("data-dt-show")) : hidden.add(t.getAttribute("data-dt-show"));
      apply();
    } else if (t.matches("[data-dt-size]")) {
      size = Math.max(1, Number(t.value) || cfg.pageSize);
      local.page = 1;
      if (B.page) api.set(B.page, 1);
      else apply();
    } else if (t.matches("[data-dt-pick]")) {
      const pop = t.closest("[data-dt-filter]");
      const values = [...pop.querySelectorAll("[data-dt-pick]")].filter((b) => b.checked).map((b) => b.__v);
      setFilter(pop.getAttribute("data-dt-filter"), values);
    }
  });
  // Checkbox clicks carry Shift; `change` doesn't.
  el.addEventListener("click", (e) => {
    if (e.target.matches?.("[data-dt-select]")) e.target.__shift = e.shiftKey;
  }, true);

  el.addEventListener("input", (e) => {
    const t = e.target;
    if (t.matches("[data-dt-search]")) {
      local.page = 1;
      if (B.page) api.set(B.page, 1);
      put("search", t.value);
    } else if (t.matches("[data-dt-op]")) {
      const pop = t.closest("[data-dt-filter]"), key = pop.getAttribute("data-dt-filter");
      const c = {};
      for (const i of pop.querySelectorAll("[data-dt-op]")) {
        if (i.value === "") continue;
        c[i.getAttribute("data-dt-op")] = i.type === "number" ? Number(i.value) : i.value;
      }
      setFilter(key, Object.keys(c).length ? c : null);
    }
  });

  // A select filter lists its choices when it opens.
  for (const p of $$("[data-dt-filter]")) p.addEventListener("toggle", (e) => e.newState === "open" && options(p));

  el.addEventListener("keydown", (e) => {
    const t = e.target;
    if (t.matches("[data-dt-search]") && e.key === "Escape" && t.value) {
      e.preventDefault();
      t.value = "";
      t.dispatchEvent(new Event("input", { bubbles: true }));
      return;
    }
    if (!navigable || !t.matches("[data-dt-row]")) return;
    // Positions count over every shown row: a virtual table draws the target first.
    const list = visibleRows();
    const n = cfg.listed ? Number(t.getAttribute("data-dt-row")) : list.indexOf(t);
    const last = (cfg.listed ? shown.length : list.length) - 1;
    const rowAt = (to) => (cfg.listed ? $(`[data-dt-body] > [data-dt-row="${to}"]`) : list[to]);
    const move = (to) => {
      e.preventDefault();
      to = Math.max(0, Math.min(last, to));
      let next = rowAt(to);
      if (!next && cfg.virtual) {
        scroller.scrollTop = Math.max(0, ($("thead")?.offsetHeight ?? 0) + to * est - scroller.clientHeight / 2);
        apply();
        next = rowAt(to);
      }
      if (!next) return;
      focusKey = next.__k;
      roving();
      next.focus();
    };
    switch (e.key) {
      case "ArrowDown": return move(n + 1);
      case "ArrowUp": return move(n - 1);
      case "Home": return move(0);
      case "End": return move(last);
      case "PageDown":
      case "PageUp": {
        if (mode !== "pages") return move(n + (e.key === "PageDown" ? 10 : -10));
        e.preventDefault();
        const p = e.key === "PageDown" ? Math.min(pages, page + 1) : Math.max(1, page - 1);
        if (p === page) return move(e.key === "PageDown" ? last : 0);
        focusKey = null;
        goTo(p);
        return visibleRows()[0]?.focus();
      }
      case " ":
        if (!cfg.selectable) return;
        e.preventDefault();
        return toggleRows([rowOf(t).key], !selection().some((k) => keyOf(k) === keyOf(rowOf(t).key)));
      case "Enter":
        e.preventDefault();
        if (B.row) return clickRow(t);
        if (cfg.detail) return toggleDetail(t);
        return;
      case "ArrowRight":
      case "ArrowLeft":
        if (tree) {
          // Right opens a row, then steps into it; Left closes it, then steps out.
          e.preventDefault();
          const i = indexOf(t), m = meta[i];
          if (!m) return;
          if (e.key === "ArrowRight") return m.has && !m.open ? toggleNode(i, true) : m.open ? move(n + 1) : undefined;
          if (m.open) return toggleNode(i, false);
          if (m.parent >= 0) return move(cfg.listed ? shown.indexOf(m.parent) : n);
          return;
        }
        if (!cfg.detail) return;
        e.preventDefault();
        if (expanded.has(t.__k) !== (e.key === "ArrowRight")) toggleDetail(t);
    }
  });

  apply();
  return { update: apply };
};
