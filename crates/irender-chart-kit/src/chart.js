// irender-chart-kit: the `ui-chart` behaviour. Loads ECharts once per page, builds
// an option from the shorthand props, themes it from the page's CSS tokens, and
// redraws when the state it reads changes.
(() => {
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);

  // ---------------------------------------------------------------- loading
  const loads = {};
  const load = (src) =>
    window.echarts ? Promise.resolve(window.echarts)
    : (loads[src] ||= new Promise((ok, fail) => {
        const s = document.createElement("script");
        s.src = src;
        s.async = true;
        s.onload = () => (window.echarts ? ok(window.echarts) : fail(new Error(`${src} did not define echarts`)));
        s.onerror = () => {
          delete loads[src];
          fail(new Error(`couldn't load ${src}`));
        };
        document.head.append(s);
      }));
  const maps = {};
  const loadMap = (ec, name, url) =>
    (maps[name] ||= fetch(url, { headers: { accept: "application/json" } })
      .then((r) => {
        if (!r.ok) throw new Error(`map ${name}: ${r.status}`);
        return r.json();
      })
      .then((geo) => ec.registerMap(name, geo)));

  // ---------------------------------------------------------------- colours
  // The reference palette, for pages whose theme has no `color.chart-*` tokens.
  const FALLBACK = {
    light: ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"],
    dark: ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"],
  };
  let ctx;
  // Any colour the browser understands (color-mix, oklch, …) as rgba(), which
  // ECharts can parse and fade.
  const rgba = (css) => {
    ctx ||= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true });
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = "#000";
    ctx.fillStyle = css;
    ctx.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data;
    return `rgba(${r},${g},${b},${+(a / 255).toFixed(3)})`;
  };
  const channels = (c) => c.match(/[\d.]+/g).map(Number);
  const fade = (c, a) => {
    const [r, g, b, a0] = channels(c);
    return `rgba(${r},${g},${b},${+(a0 * a).toFixed(3)})`;
  };
  const luminance = (c) => {
    const [r, g, b] = channels(c);
    return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
  };

  // The page's tokens as resolved colours. Tokens the theme lacks fall back to the
  // element's own text colour and the first opaque background above it.
  function readTheme(el) {
    const cs = getComputedStyle(el);
    const probe = document.createElement("i");
    probe.style.display = "none";
    el.append(probe);
    const color = (css) => {
      probe.style.color = "";
      probe.style.color = css;
      return rgba(getComputedStyle(probe).color);
    };
    const has = (name) => cs.getPropertyValue(name).trim() !== "";
    const text = color(has("--color-text") ? "var(--color-text)" : cs.color);
    // What the chart sits on: the card's, popover's, … (irender-ui-kit sets `--ui-bg`) or the page's.
    let surface = has("--color-surface") ? color("var(--ui-bg, var(--color-background, var(--color-surface)))") : null;
    for (let n = el; !surface && n; n = n.parentElement) {
      const bg = rgba(getComputedStyle(n).backgroundColor);
      if (channels(bg)[3] > 0) surface = bg;
    }
    surface ||= "rgba(255,255,255,1)";
    const dark = luminance(surface) < 0.4;
    const muted = has("--color-muted") ? color("var(--color-muted)") : fade(text, 0.65);
    const border = has("--color-border") ? color("var(--color-border)") : fade(text, 0.15);
    const palette = FALLBACK[dark ? "dark" : "light"].map((c, i) => color(`var(--color-chart-${i + 1}, ${c})`));
    // Sequential: the first hue, receding into the surface at the low end.
    const low = color(`color-mix(in oklab, ${palette[0]} 12%, ${surface})`);
    probe.remove();
    // The theme's text size and padding (`font.scale`, `space.scale` × `space.inset`), as at `el`.
    const scale = (name) => parseFloat(cs.getPropertyValue(name)) || 1;
    return { text, muted, border, surface, palette, seq: [low, palette[0]], dark, font: cs.fontFamily, fontScale: scale("--font-scale"), inset: scale("--space-scale") * scale("--space-inset") };
  }

  // A font size in px that follows the theme's `font.scale`.
  const px = (t) => (n) => Math.round(n * t.fontScale * 10) / 10;

  const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });

  // Defaults for every chart, as an ECharts theme: recessive axes and grid lines,
  // text in text colours, a tooltip that looks like the page's popovers.
  function echartsTheme(t) {
    const fs = px(t);
    const axis = {
      axisLine: { lineStyle: { color: t.border } },
      axisTick: { show: false },
      axisLabel: { color: t.muted, fontSize: fs(12) },
      splitLine: { lineStyle: { color: t.border } },
      splitArea: { show: false },
      nameTextStyle: { color: t.muted },
    };
    // Big numbers in short form (12K, 1.5M), and labels that would collide left out.
    const value = {
      ...axis,
      axisLine: { show: false, lineStyle: { color: t.border } },
      axisLabel: { color: t.muted, fontSize: fs(12), hideOverlap: true, formatter: (v) => (Math.abs(v) >= 1e4 ? compact.format(v) : v.toLocaleString()) },
    };
    return {
      color: t.palette,
      backgroundColor: "transparent",
      textStyle: { color: t.text, fontFamily: t.font, fontSize: fs(12) },
      title: { textStyle: { color: t.text, fontSize: fs(18) }, subtextStyle: { color: t.muted, fontSize: fs(12) } },
      legend: {
        textStyle: { color: t.muted, fontSize: fs(12) },
        pageTextStyle: { color: t.muted },
        pageIconColor: t.text,
        pageIconInactiveColor: t.border,
        inactiveColor: t.border,
      },
      tooltip: {
        backgroundColor: t.surface,
        borderColor: t.border,
        borderWidth: 1,
        padding: [8 * t.inset, 10 * t.inset],
        textStyle: { color: t.text, fontSize: fs(12) },
        // Like the page's popovers: its card radius, popover shadow and border width.
        extraCssText: "border-radius:var(--radius-card, 8px);box-shadow:var(--elevation-popover, 0 4px 12px rgb(0 0 0 / 0.1));border-width:var(--border-width, 1px);",
        axisPointer: {
          lineStyle: { color: t.muted },
          crossStyle: { color: t.muted },
          shadowStyle: { color: fade(t.text, 0.05) },
          label: { backgroundColor: t.text, color: t.surface },
        },
      },
      // Every category label that fits, rather than ECharts' cautious guess.
      categoryAxis: { ...axis, axisLabel: { color: t.muted, interval: 0, hideOverlap: true }, splitLine: { show: false, lineStyle: { color: t.border } } },
      valueAxis: value,
      logAxis: value,
      timeAxis: { ...axis, splitLine: { show: false, lineStyle: { color: t.border } } },
      line: { lineStyle: { width: 2 }, symbol: "circle", symbolSize: 8 },
      visualMap: { color: [t.seq[1], t.seq[0]], textStyle: { color: t.muted } },
      dataZoom: {
        textStyle: { color: t.muted },
        borderColor: t.border,
        fillerColor: fade(t.palette[0], 0.12),
        handleStyle: { color: t.surface, borderColor: t.muted },
        moveHandleStyle: { color: t.border },
        dataBackground: { lineStyle: { color: t.border }, areaStyle: { color: fade(t.text, 0.06) } },
        selectedDataBackground: { lineStyle: { color: t.palette[0] }, areaStyle: { color: fade(t.palette[0], 0.15) } },
      },
      toolbox: { iconStyle: { borderColor: t.muted }, emphasis: { iconStyle: { borderColor: t.text } } },
      markLine: { lineStyle: { color: t.muted }, label: { color: t.muted } },
    };
  }

  // ---------------------------------------------------------------- shorthand
  const isNum = (v) => typeof v === "number" || (typeof v === "string" && v.trim() !== "" && !isNaN(v));
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const uniq = (vs) => [...new Set(vs)];
  const looksLikeDate = (v) => typeof v === "string" && /^\d{4}-\d{2}/.test(v) && !isNaN(Date.parse(v));
  const fieldsOf = (rows) => {
    const r = rows.find((r) => r !== null && typeof r === "object");
    return !r ? [] : Array.isArray(r) ? r.map((_, i) => i) : Object.keys(r);
  };
  const ylist = (y) => (Array.isArray(y) ? y : [y]).map((f) => (isObj(f) ? f : { field: f }));
  const named = (y) => String(y.name ?? y.field);
  const quantiles = (vs) => {
    const s = vs.filter(isNum).map(Number).sort((a, b) => a - b);
    const q = (p) => {
      const i = (s.length - 1) * p, lo = Math.floor(i);
      return s[lo] + (s[Math.min(lo + 1, s.length - 1)] - s[lo]) * (i - lo);
    };
    return s.length ? [s[0], q(0.25), q(0.5), q(0.75), s[s.length - 1]] : [];
  };

  const CARTESIAN = new Set(["line", "area", "bar", "scatter", "effectScatter", "pictorialBar"]);
  const NESTED = new Set(["tree", "treemap", "sunburst", "sankey", "graph", "chord"]);
  // Types whose `data` is rows (rather than a tree, nodes or a single value).
  const tabular = (type) => !NESTED.has(type) && type !== "gauge";

  // ---------------------------------------------------------------- rows
  const at = (r, f) => (r === null || r === undefined ? undefined : r[f]);
  const blank = (v) => v === null || v === undefined || v === "";
  const isTime = (v) => looksLikeDate(v) || (typeof v === "number" && v > 1e11);
  // Loose equality for plain values: state from form controls is text, data may be numbers.
  const same = (a, b) =>
    a === b || (!blank(a) && !blank(b) && typeof a !== "object" && typeof b !== "object" && String(a) === String(b));
  // Orders numbers as numbers, dates as dates and everything else as text.
  const cmp = (a, b) => {
    if (isNum(a) && isNum(b)) return Number(a) - Number(b);
    if (looksLikeDate(a) && looksLikeDate(b)) return Date.parse(a) - Date.parse(b);
    return String(a ?? "").localeCompare(String(b ?? ""));
  };
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const OPS = {
    from: (v, b) => cmp(v, b) >= 0,
    to: (v, b) => cmp(v, b) <= 0,
    gt: (v, b) => cmp(v, b) > 0,
    gte: (v, b) => cmp(v, b) >= 0,
    lt: (v, b) => cmp(v, b) < 0,
    lte: (v, b) => cmp(v, b) <= 0,
    contains: (v, b) => fold(v).includes(fold(b)),
  };
  // Whether a row's `field` value passes one `filter` condition: a value, a list of
  // values (or of rows), a row, or operators such as `{ from, to }`. Blank conditions
  // and empty lists pass, so a filter bound to an empty control shows everything.
  function matches(v, c, field) {
    if (blank(c)) return true;
    if (Array.isArray(c)) return !c.length || c.some((x) => same(v, isObj(x) ? x[field] : x));
    if (!isObj(c)) return same(v, c);
    if (Object.hasOwn(c, "not")) return blank(c.not) || !matches(v, c.not, field);
    // `{ empty: true }`: no value; `{ empty: false }`: any value.
    if (Object.hasOwn(c, "empty")) return blank(c.empty) || (c.empty ? blank(v) : !blank(v));
    const ops = Object.keys(c).filter((k) => Object.hasOwn(OPS, k));
    // A row, e.g. one `bindRow` wrote: compare the same field.
    if (!ops.length) return same(v, c[field]);
    return ops.every((k) => blank(c[k]) || OPS[k](v, c[k]));
  }

  const tidy = (n) => (n === null || !Number.isFinite(n) ? null : Math.round(n * 1e9) / 1e9);
  const AGGREGATES = {
    sum: (vs) => tidy(vs.reduce((a, b) => a + b, 0)),
    avg: (vs) => (vs.length ? tidy(vs.reduce((a, b) => a + b, 0) / vs.length) : null),
    min: (vs) => (vs.length ? Math.min(...vs) : null),
    max: (vs) => (vs.length ? Math.max(...vs) : null),
    count: (vs) => vs.length,
  };
  // One row per distinct `keys`, with `fields` aggregated and the group's `count`.
  function group(rows, keys, fields, how) {
    const groups = new Map();
    for (const r of rows) {
      const k = JSON.stringify(keys.map((f) => at(r, f) ?? null));
      groups.has(k) ? groups.get(k).push(r) : groups.set(k, [r]);
    }
    return [...groups.values()].map((g) => {
      const out = { count: g.length };
      for (const f of keys) out[f] = at(g[0], f);
      for (const f of fields) out[f] = how === "count" ? g.length : AGGREGATES[how](g.map((r) => at(r, f)).filter(isNum).map(Number));
      return out;
    });
  }
  // `sort`: a field, `-field` for descending, `{ field, order }`, or a list of them.
  function sortRows(rows, by) {
    const keys = (Array.isArray(by) ? by : [by]).map((k) =>
      isObj(k) ? { field: k.field, desc: k.order === "desc" }
      : typeof k === "string" && k.startsWith("-") ? { field: k.slice(1), desc: true }
      : { field: k, desc: false });
    return [...rows].sort((a, b) => {
      for (const { field, desc } of keys) {
        const c = cmp(at(a, field), at(b, field));
        if (c) return desc ? -c : c;
      }
      return 0;
    });
  }

  // The rows a chart draws: `data` at the current drill level, filtered, grouped,
  // sorted and cut to `limit`, with the `x` and `y` to draw them by.
  function prepare(p, path) {
    const out = { rows: p.data, x: p.x, y: p.y, depth: 0, levels: 0 };
    let rows = p.data;
    if (!Array.isArray(rows) || !tabular(p.type)) return out;
    // Plain numbers: their position is the category.
    if (rows.length && rows.every((r) => r === null || typeof r !== "object")) rows = rows.map((v, i) => [i, v]);
    const fields = fieldsOf(rows);
    let x = p.x ?? fields[0], y = p.y, how = AGGREGATES[p.aggregate] ? p.aggregate : null;
    const drill = Array.isArray(p.drill) && p.drill.length ? p.drill : null;
    if (drill) {
      out.levels = drill.length;
      out.depth = Math.min(path.length, drill.length - 1);
      rows = rows.filter((r) => path.slice(0, out.depth).every((v, i) => same(at(r, drill[i]), v)));
      x = drill[out.depth];
      how ||= "sum";
    }
    if (isObj(p.filter)) {
      const conds = Object.entries(p.filter);
      rows = rows.filter((r) => conds.every(([f, c]) => matches(at(r, f), c, f)));
    }
    if (how) {
      const keys = blank(p.series) ? [x] : [x, p.series];
      const skip = new Set([...keys, ...(drill || [])].map(String));
      const ys = !blank(y) ? ylist(y).map((f) => f.field)
        : how === "count" ? []
        : fields.filter((f) => !skip.has(String(f)) && rows.some((r) => isNum(at(r, f))));
      rows = group(rows, keys, ys, how);
      if (blank(y)) y = how === "count" ? "count" : ys;
    }
    const limit = isNum(p.limit) && Number(p.limit) > 0 ? Number(p.limit) : null;
    if (p.type === "pareto") {
      // Largest first, whatever `sort` says; past `limit`, the rest add up into one
      // Other bar, kept last. Values that aren't numbers, or are below zero, are left out.
      const f = blank(y) ? fieldsOf(rows).find((f) => f !== x && rows.some((r) => isNum(at(r, f)))) : ylist(y)[0].field;
      rows = sortRows(rows.filter((r) => isNum(at(r, f)) && Number(at(r, f)) >= 0), { field: f, order: "desc" });
      if (limit && rows.length > limit) {
        const rest = rows.slice(limit);
        // Other holds what went into it, for `bindRow` and `change`.
        rows = [...rows.slice(0, limit), { [x]: L("Other"), [f]: tidy(rest.reduce((a, r) => a + Number(at(r, f)), 0)), rows: rest }];
      }
      return { ...out, rows, x, y: blank(y) ? f : y };
    }
    if (!blank(p.series) && (!blank(p.sort) || limit)) {
      // Split into series, the rows are (category, series) pairs: sort and limit the
      // categories, each by its total over its series, and keep its rows together.
      const cats = new Map();
      for (const r of rows) {
        const k = JSON.stringify(at(r, x) ?? null);
        cats.has(k) ? cats.get(k).push(r) : cats.set(k, [r]);
      }
      let order = [...cats.values()].map((list) => {
        const sum = { [x]: at(list[0], x), __rows: list };
        for (const f of fieldsOf(list)) {
          if (f === x) continue;
          sum[f] = list.every((r) => blank(at(r, f)) || isNum(at(r, f))) && list.some((r) => isNum(at(r, f))) ? tidy(list.reduce((a, r) => a + (isNum(at(r, f)) ? Number(at(r, f)) : 0), 0)) : at(list[0], f);
        }
        return sum;
      });
      if (!blank(p.sort)) order = sortRows(order, p.sort);
      if (limit) order = order.slice(0, limit);
      rows = order.flatMap((c) => c.__rows);
    } else {
      if (!blank(p.sort)) rows = sortRows(rows, p.sort);
      if (limit) rows = rows.slice(0, limit);
    }
    return { ...out, rows, x, y };
  }

  // ---------------------------------------------------------------- gantt
  // Rows are tasks: `{ id, name, start, end, group, progress, after, milestone, color }`.
  // A row whose `id` is another's `group` is a group: its bar spans its tasks, and
  // its line in the table opens and closes. The table on the left is drawn as the
  // category axis' labels, one `columns` entry per column.
  const GANTT = { row: 34, head: 44, band: 30, foot: 12, marks: 28, slider: 36, indent: 16, edge: 7 };
  const DAY = 864e5;
  const dateOnly = (v) => typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v);
  // Epoch ms; a plain day is local midnight, as ECharts reads it.
  const ms = (v) => {
    if (blank(v)) return NaN;
    if (typeof v === "number") return v;
    if (dateOnly(v)) {
      const [y, m, d] = v.split("-").map(Number);
      return new Date(y, m - 1, d).getTime();
    }
    return Date.parse(v);
  };
  // Local midnight `n` days from `t` (a day changing clocks is still one day).
  const addDays = (t, n) => {
    const d = new Date(t);
    return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n).getTime();
  };
  // A gantt's working days: `workdays` (true for Monday to Friday, or the weekdays
  // that are, 0 for Sunday) less its `holidays`. Null when every day is one.
  function workCalendar(p) {
    if (!p.workdays) return null;
    const week = new Set(Array.isArray(p.workdays) ? p.workdays.map(Number) : [1, 2, 3, 4, 5]);
    if (!week.size) return null;
    const off = new Set((Array.isArray(p.holidays) ? p.holidays : []).filter(dateOnly));
    const key = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const works = (t) => {
      const d = new Date(t);
      return week.has(d.getDay()) && !off.has(key(d));
    };
    // The working day at `t`, or the next (`dir` 1) or last (-1) one.
    const snap = (t, dir = 1) => {
      for (let i = 0; i < 400 && !works(t); i++) t = addDays(t, dir < 0 ? -1 : 1);
      return t;
    };
    return {
      works,
      snap,
      // The working days from `a` up to, not including, `b`.
      count: (a, b) => {
        let n = 0;
        for (let t = a, i = 0; t < b && i < 20000; t = addDays(t, 1), i++) if (works(t)) n++;
        return n;
      },
      // The working day `n` working days after working day `t`.
      add: (t, n) => {
        for (let i = 0; i < n; i++) t = snap(addDays(t, 1));
        return t;
      },
    };
  }
  const escapeHtml = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  // The rows a gantt shows, in order, with their depth and span. `collapsed` holds
  // the ids of closed groups; a group's own `collapsed: true` closes it the first time
  // it's seen.
  function ganttRows(rows, collapsed, seen, drafts) {
    const key = (r) => String(at(r, "id") ?? at(r, "name"));
    const byId = new Map(rows.map((r) => [key(r), r]));
    const kids = new Map(), tops = [];
    for (const r of rows) {
      const g = at(r, "group");
      if (!blank(g) && byId.has(String(g)) && String(g) !== key(r)) {
        if (!kids.has(String(g))) kids.set(String(g), []);
        kids.get(String(g)).push(r);
      } else tops.push(r);
    }
    // A row's span and progress: its own, or for a group what its tasks add up to.
    const spans = new Map();
    const span = (r, trail) => {
      const k = key(r);
      if (spans.has(k)) return spans.get(k);
      if (trail.has(k)) return { a: NaN, b: NaN, done: null };
      trail.add(k);
      let a = ms(at(r, "start")), b = ms(at(r, "end"));
      const milestone = !!at(r, "milestone") || (!Number.isNaN(a) && blank(at(r, "end")));
      // A plain day as the end means the whole of that day.
      if (dateOnly(at(r, "end"))) b += DAY;
      if (milestone) b = a;
      // A task being dragged: where it would go.
      const draft = drafts?.get(k);
      if (draft) [a, b] = [draft.a, milestone ? draft.a : draft.b];
      // The baseline, the planned dates: `baselineStart` and `baselineEnd`, read as
      // `start` and `end` are.
      let pa = ms(at(r, "baselineStart")), pb = ms(at(r, "baselineEnd"));
      if (dateOnly(at(r, "baselineEnd"))) pb += DAY;
      if (milestone || (!Number.isNaN(pa) && blank(at(r, "baselineEnd")))) pb = pa;
      let done = isNum(at(r, "progress")) ? Math.max(0, Math.min(100, Number(at(r, "progress")))) / 100 : null;
      const children = kids.get(k) || [];
      if (children.length) {
        let weight = 0, sum = 0;
        for (const c of children) {
          const s = span(c, trail);
          if (Number.isNaN(a) || s.a < a) a = Number.isNaN(s.a) ? a : s.a;
          if (Number.isNaN(b) || s.b > b) b = Number.isNaN(s.b) ? b : s.b;
          // A group's baseline: its tasks', unless it has its own.
          if (blank(at(r, "baselineStart"))) {
            if (!Number.isNaN(s.pa) && (Number.isNaN(pa) || s.pa < pa)) pa = s.pa;
            if (!Number.isNaN(s.pb) && (Number.isNaN(pb) || s.pb > pb)) pb = s.pb;
          }
          const w = Math.max(s.b - s.a, DAY / 24);
          if (s.done !== null && Number.isFinite(w)) (weight += w), (sum += s.done * w);
        }
        if (done === null && weight) done = sum / weight;
      }
      const out = { a, b, pa, pb, done, milestone: milestone && !children.length };
      spans.set(k, out);
      return out;
    };
    const shown = [], all = [], seenNow = new Set();
    const walk = (list, depth, open) => {
      for (const r of list) {
        const k = key(r);
        if (seenNow.has(k)) continue;
        seenNow.add(k);
        const group = kids.has(k);
        if (group && !seen.has(k)) {
          seen.add(k);
          if (at(r, "collapsed")) collapsed.add(k);
        }
        const item = { row: r, key: k, depth, group, open: group && !collapsed.has(k), ...span(r, new Set()) };
        all.push(item);
        if (open) item.index = shown.push(item) - 1;
        if (group) walk(kids.get(k), depth + 1, open && item.open);
      }
    };
    walk(tops, 0, true);
    // Hidden rows stand in for by their nearest shown group, for arrows.
    const parent = new Map();
    for (const [g, cs] of kids) for (const c of cs) parent.set(key(c), g);
    const byKey = new Map(all.map((i) => [i.key, i]));
    const showing = (k) => {
      let i = byKey.get(k);
      while (i && i.index === undefined) i = byKey.get(parent.get(i.key));
      return i;
    };
    return { shown, all, showing };
  }

  function gantt(p, t, d, o, rows) {
    const g = ganttRows(rows, d.collapsed || new Set(), d.seen || new Set(), d.drafts);
    const editable = !!d.editable;
    // Each task's `after`: the tasks it waits for, and those it starts before the end of.
    const byKey = new Map(g.all.map((i) => [i.key, i]));
    const afterOf = (i) => {
      const v = at(i.row, "after");
      return (blank(v) ? [] : Array.isArray(v) ? v : String(v).split(",")).map((k) => byKey.get(String(k).trim())).filter(Boolean);
    };
    for (const i of g.all) i.late = afterOf(i).filter((f) => f !== i && !Number.isNaN(f.b) && !Number.isNaN(i.a) && i.a < f.b);
    const { shown } = g;
    d.gantt = g;
    const lang = document.documentElement.lang || undefined;
    const day = new Intl.DateTimeFormat(lang, { dateStyle: "medium" });
    const stamp = new Intl.DateTimeFormat(lang, { dateStyle: "medium", timeStyle: "short" });
    const when = (v) => (blank(v) ? "" : dateOnly(v) ? day.format(ms(v)) : Number.isNaN(ms(v)) ? String(v) : stamp.format(ms(v)));
    // The axis's dates in the document's language: a year at its start, a month at its
    // first day, else the day and month; times of day when zoomed in that far.
    const yearF = new Intl.DateTimeFormat(lang, { year: "numeric" }), monthF = new Intl.DateTimeFormat(lang, { month: "short" });
    const dayF = new Intl.DateTimeFormat(lang, { day: "numeric", month: "short" }), timeF = new Intl.DateTimeFormat(lang, { hour: "numeric", minute: "2-digit" });
    const axisDate = (v) => {
      const d = new Date(v);
      if (d.getHours() || d.getMinutes()) return timeF.format(d);
      if (d.getDate() === 1 && d.getMonth() === 0) return yearF.format(d);
      return d.getDate() === 1 ? monthF.format(d) : dayF.format(d);
    };

    // The table: the first column is the tree of names.
    const all = (Array.isArray(p.columns) && p.columns.length ? p.columns : ["name"]).map((c, i) => {
      const c0 = isObj(c) ? c : { field: c };
      const name = c0.name ?? (c0.field === "name" ? L("Task") : String(c0.field));
      return { field: c0.field, name, width: Number(c0.width) || (i ? 96 : 200), align: c0.align ?? "left" };
    });
    // On a narrow chart the table takes at most half: the last columns go first,
    // then the names get narrower.
    const width = d.width || 960, budget = Math.max(120, width * 0.5 - 16);
    const cols = [all[0]];
    for (const c of all.slice(1)) if (cols.reduce((n, x) => n + x.width, 0) + c.width <= budget) cols.push(c);
    cols[0] = { ...all[0], width: Math.max(96, Math.min(all[0].width, budget - cols.slice(1).reduce((n, x) => n + x.width, 0))) };
    const tableWidth = cols.reduce((n, c) => n + c.width, 0) + 8;
    const cell = (v) => (blank(v) ? "" : looksLikeDate(v) ? when(v) : isNum(v) && typeof v === "number" ? v.toLocaleString(lang) : String(v));
    // Series: one per `series` value (with a legend), groups and milestones apart.
    const split = blank(p.series) ? null : p.series;
    // From every row, so closing a group keeps each team's colour.
    const names = split ? uniq(g.all.filter((i) => !i.group).map((i) => String(at(i.row, split) ?? ""))) : [""];
    // The critical path's colour: red, as plans usually draw it.
    const CRIT = t.palette[7] ?? "#dc2626";
    // Its length: working days with `workdays` (for tasks dated in whole days).
    const cal = workCalendar(p);
    const working = (i) => cal && dateOnly(at(i.row, "start")) && (blank(at(i.row, "end")) || dateOnly(at(i.row, "end")));
    const days = (i) => Math.max(1, working(i) ? cal.count(i.a, i.b) : Math.round((i.b - i.a) / DAY));
    // `critical`: the tasks with no slack, the chain that sets the plan's end. A task's
    // slack is the least of the gap to a task that waits on it plus that task's slack;
    // with nothing waiting, the gap to the plan's end. Gaps are working days with
    // `workdays`, else days.
    for (const i of g.all) i.critical = false;
    if (p.critical) {
      const nodes = g.all.filter((i) => !Number.isNaN(i.a) && !Number.isNaN(i.b));
      const end = Math.max(...nodes.map((i) => i.b));
      const next = new Map(nodes.map((i) => [i, []]));
      for (const i of nodes) for (const f of afterOf(i)) if (f !== i && next.has(f)) next.get(f).push(i);
      const gap = (i, t) => (working(i) ? cal.count(i.b, t) : (t - i.b) / DAY);
      const memo = new Map();
      const slackOf = (i, path = new Set()) => {
        if (memo.has(i)) return memo.get(i);
        // A loop of `after`s: no further on.
        if (path.has(i)) return 0;
        path.add(i);
        let v = next.get(i).length ? Infinity : gap(i, end);
        for (const s of next.get(i)) v = Math.min(v, Math.max(0, gap(i, s.a)) + slackOf(s, path));
        path.delete(i);
        memo.set(i, v);
        return v;
      };
      for (const i of nodes) i.critical = !i.group && slackOf(i) < 0.5;
    }
    // Baselines: drawn under the bars when a task has one, unless `baselines` is false.
    const planned = (i) => !Number.isNaN(i.pa) && !Number.isNaN(i.pb);
    const baselines = p.baselines !== false && g.all.some(planned);
    // How far a task's end is from its planned end, in days (working days with
    // `workdays`): more than 0 is late.
    const slip = (i) => {
      if (!planned(i) || Number.isNaN(i.b)) return 0;
      const [x, y] = i.b >= i.pb ? [i.pb, i.b] : [i.b, i.pb];
      // A group's in working days too, with `workdays`.
      const n = working(i) || (cal && i.group) ? cal.count(x, y) : Math.round((y - x) / DAY);
      return i.b >= i.pb ? n : -n;
    };
    const nameOf = (i) => String(at(i.row, cols[0].field) ?? at(i.row, "name") ?? i.key);
    // What a task is, line by line: its name, dates and length, progress and series.
    const about = (i) => {
      const r = i.row, lines = [nameOf(i)];
      if (i.milestone) lines.push(when(at(r, "start")));
      else if (!Number.isNaN(i.a)) {
        const a = blank(at(r, "start")) ? when(new Date(i.a).toISOString()) : when(at(r, "start"));
        const b = blank(at(r, "end")) ? when(new Date(dateOnly(at(r, "start")) ? i.b - DAY : i.b).toISOString()) : when(at(r, "end"));
        lines.push(`${a} – ${b} · ${working(i) ? L(["{n} working day", "{n} working days"], { n: days(i) }) : L(["{n} day", "{n} days"], { n: days(i) })}`);
      }
      if (baselines && planned(i)) {
        // A group's are its tasks': days.
        const a = blank(at(r, "baselineStart")) ? day.format(i.pa) : when(at(r, "baselineStart"));
        const b = blank(at(r, "baselineEnd")) ? day.format(i.pb - DAY) : when(at(r, "baselineEnd"));
        lines.push(i.pa === i.pb ? L("Planned {date}", { date: a }) : L("Planned {start} – {end}", { start: a, end: b }));
        const n = slip(i);
        if (n > 0) lines.push(L(["{n} day late", "{n} days late"], { n }));
        else if (n < 0) lines.push(L(["{n} day early", "{n} days early"], { n: -n }));
      }
      if (i.done !== null) lines.push(L("{percent}% done", { percent: Math.round(i.done * 100) }));
      if (split && !i.group && !blank(at(r, split))) lines.push(String(at(r, split)));
      for (const f of i.late ?? []) lines.push(L("Starts before {name} ends", { name: nameOf(f) }));
      if (i.critical) lines.push(L("On the critical path"));
      return lines;
    };
    const tip = (i) => about(i).map((l, n) => (n ? escapeHtml(l) : `<b>${escapeHtml(l)}</b>`)).join("<br/>");
    const item = (i) => ({
      name: String(at(i.row, "name") ?? i.key),
      value: [i.index, i.a, i.b, i.done === null ? -1 : i.done, i.group ? 1 : i.milestone ? 2 : 0, i.critical ? 1 : 0, baselines && planned(i) ? i.pa : null, baselines && planned(i) ? i.pb : null],
      tip: tip(i),
      // A milestone outside every `series` is the text's colour.
      ...(!blank(at(i.row, "color")) ? { itemStyle: { color: String(at(i.row, "color")) } } : i.milestone && split && blank(at(i.row, split)) ? { itemStyle: { color: t.text } } : {}),
    });
    const drawn = shown.filter((i) => !Number.isNaN(i.a) && !Number.isNaN(i.b));

    const bar = (params, api) => {
      const at0 = api.coord([api.value(1), api.value(0)]), at1 = api.coord([api.value(2), api.value(0)]);
      const h = api.size([0, 1])[1];
      const kind = api.value(4), done = api.value(3), critical = api.value(5) === 1;
      const color = api.visual("color");
      // On the critical path: a ring in the critical colour.
      const ring = (shape, type = "rect") => ({ type, shape, style: { fill: "none", stroke: CRIT, lineWidth: 2 }, silent: true });
      const y = at0[1];
      // The baseline: a thin grey bar under the bar from its planned dates (a hollow
      // diamond for a milestone's), behind it.
      const pa = api.value(6), pb = api.value(7);
      const base = Number.isFinite(pa) && Number.isFinite(pb) ? [api.coord([pa, api.value(0)])[0], api.coord([pb, api.value(0)])[0]] : null;
      const baseStyle = { fill: fade(t.muted, 0.55) };
      const baseBar = (by) => ({ type: "rect", shape: { x: base[0], y: by, width: Math.max(base[1] - base[0], 2), height: Math.max(3, h * 0.1), r: 1.5 }, style: baseStyle, silent: true });
      // Editable: a dot after the bar, shown on hover, to drag to a task that waits on it.
      const connector = (x) => [
        { type: "circle", shape: { cx: x + 10, cy: y, r: 9 }, style: { fill: "transparent" }, cursor: "crosshair" },
        { type: "circle", shape: { cx: x + 10, cy: y, r: 4 }, style: { fill: t.surface, stroke: color, lineWidth: 1.5, opacity: 0 }, emphasis: { style: { opacity: 1 } }, cursor: "crosshair" },
      ];
      if (kind === 2) {
        const s = Math.min(h * 0.32, 9);
        const diamond = { type: "polygon", shape: { points: [[at0[0], y - s], [at0[0] + s, y], [at0[0], y + s], [at0[0] - s, y]] }, style: { fill: color, stroke: critical ? CRIT : t.surface, lineWidth: critical ? 2 : 1 } };
        const ps = s * 0.8, px = base?.[0];
        const planned = base ? [{ type: "polygon", shape: { points: [[px, y - ps], [px + ps, y], [px, y + ps], [px - ps, y]] }, style: { fill: "none", stroke: t.muted, lineWidth: 1.5 }, silent: true }] : [];
        if (!editable && !planned.length) return diamond;
        return { type: "group", children: [...planned, editable ? { ...diamond, cursor: "grab" } : diamond, ...(editable ? connector(at0[0] + s) : [])] };
      }
      const w = Math.max(at1[0] - at0[0], 2);
      if (kind === 1) {
        // A group: a thin bar with a notch down at either end.
        const bh = Math.max(4, h * 0.2), top = y - bh / 2, tip = Math.min(6, w / 2);
        const children = [{ type: "polygon", shape: { points: [[at0[0], top], [at0[0] + w, top], [at0[0] + w, top + bh + tip], [at0[0] + w - tip, top + bh], [at0[0] + tip, top + bh], [at0[0], top + bh + tip]] }, style: { fill: color } }];
        if (base) children.unshift(baseBar(top + bh + tip + 2));
        return { type: "group", children };
      }
      // With baselines the bars move up a little, to make room under them.
      const bh = baselines ? Math.max(8, h * 0.44) : Math.max(10, h * 0.56), top = y - bh / 2 - (baselines ? h * 0.08 : 0);
      const r = Math.min(4, bh / 2, w / 2);
      const children = done < 0
        ? [{ type: "rect", shape: { x: at0[0], y: top, width: w, height: bh, r }, style: { fill: color } }]
        : [
            { type: "rect", shape: { x: at0[0], y: top, width: w, height: bh, r }, style: { fill: fade(rgba(color), 0.35) } },
            { type: "rect", shape: { x: at0[0], y: top, width: w * done, height: bh, r }, style: { fill: color } },
          ];
      if (base) children.unshift(baseBar(top + bh + 2));
      if (critical) children.push(ring({ x: at0[0] - 1.5, y: top - 1.5, width: w + 3, height: bh + 3, r: r + 1.5 }));
      // Editable: the bar is grabbed to move it, its ends to stretch it.
      if (editable) {
        for (const c of children) c.cursor = "grab";
        const edge = Math.min(GANTT.edge, w / 3);
        children.push(
          { type: "rect", shape: { x: at0[0] - 2, y: top, width: edge + 2, height: bh }, style: { fill: "transparent" }, cursor: "ew-resize" },
          { type: "rect", shape: { x: at0[0] + w - edge, y: top, width: edge + 2, height: bh }, style: { fill: "transparent" }, cursor: "ew-resize" },
          ...connector(at0[0] + w),
        );
      }
      return { type: "group", children };
    };
    const custom = (name, items, extra) => ({ type: "custom", name, renderItem: bar, encode: { x: [1, 2], y: 0 }, data: items, clip: true, z: 3, ...extra });
    const tasks = drawn.filter((i) => !i.group);
    const groupSeries = drawn.filter((i) => i.group);
    for (const name of names) {
      const mine = split ? tasks.filter((i) => String(at(i.row, split) ?? "") === name) : tasks;
      o.series.push(custom(name, mine.map(item)));
      d.rowsBy.push(mine.map((i) => i.row));
    }
    if (groupSeries.length) {
      o.series.push(custom(L("Groups"), groupSeries.map(item), { itemStyle: { color: t.muted } }));
      d.rowsBy.push(groupSeries.map((i) => i.row));
    }

    // Arrows from each task in `after` to the task that waits on it.
    const links = [], linkKeys = [], seenLinks = new Set();
    for (const i of g.all) {
      for (const f of afterOf(i)) {
        const from = g.showing(f.key), to = g.showing(i.key);
        if (!from || !to || from === to || Number.isNaN(from.b) || Number.isNaN(to.a)) continue;
        // A task that starts before the one it waits for ends: its arrow in red.
        const l = [from.index, from.b, to.index, to.a, i.late.includes(f) ? 1 : 0, f.critical && i.critical ? 1 : 0];
        if (seenLinks.has(JSON.stringify(l))) continue;
        seenLinks.add(JSON.stringify(l));
        links.push(l);
        // Which link it is, when both ends are shown (not a closed group's).
        linkKeys.push(from === f && to === i ? [f.key, i.key] : null);
      }
    }
    g.linkKeys = linkKeys;
    if (links.length) {
      o.series.push({
        // Editable, an arrow is clicked to remove it.
        type: "custom", id: "links", silent: !editable, clip: true, z: 2, tooltip: { show: false }, encode: { x: [1, 3], y: [0, 2] }, data: links,
        renderItem: (params, api) => {
          const [x1, y1] = api.coord([api.value(1), api.value(0)]), [x2, y2] = api.coord([api.value(3), api.value(2)]);
          const h = api.size([0, 1])[1], gap = 8;
          const ym = y1 + (y2 >= y1 ? h / 2 : -h / 2);
          const points = x2 - gap >= x1 + gap ? [[x1, y1], [x2 - gap, y1], [x2 - gap, y2], [x2 - 5, y2]] : [[x1, y1], [x1 + gap, y1], [x1 + gap, ym], [x2 - gap, ym], [x2 - gap, y2], [x2 - 5, y2]];
          // Late (starting before what it waits for ends): dashed red. On the critical
          // path: solid, thicker, in the critical colour.
          const late = api.value(4) === 1, crit = api.value(5) === 1;
          const stroke = late ? (t.palette[7] ?? "#dc2626") : crit ? CRIT : t.muted;
          return {
            type: "group",
            children: [
              { type: "polyline", shape: { points }, style: { stroke, fill: "none", lineWidth: crit ? 2 : 1, lineDash: late ? [4, 3] : null }, emphasis: { style: { lineWidth: 2.5 } }, cursor: editable ? "pointer" : "default" },
              { type: "polygon", shape: { points: [[x2, y2], [x2 - 6, y2 - 4], [x2 - 6, y2 + 4]] }, style: { fill: stroke } },
              // A wider line to hit.
              ...(editable ? [{ type: "polyline", shape: { points }, style: { stroke: "transparent", fill: "none", lineWidth: 8 }, cursor: "pointer" }] : []),
            ],
          };
        },
      });
      d.rowsBy.push([]);
    }

    // The ruled lines of the table and the chart, and the lines down: today and `markers`.
    const head = (split || p.zoom ? GANTT.band : 0) + GANTT.head;
    const marks = [];
    // The dates shown: the bars', and their baselines'.
    const both = (i) => baselines && planned(i);
    const lo = Math.min(...drawn.flatMap((i) => (both(i) ? [i.a, i.pa] : [i.a]))), hi = Math.max(...drawn.flatMap((i) => (both(i) ? [i.b, i.pb] : [i.b])));
    const now = Date.now();
    // Days off shaded, runs of them as one band, on plans up to three years long.
    const offDays = [];
    if (cal && Number.isFinite(hi - lo) && hi - lo < 3 * 365 * DAY) {
      let from = null;
      for (let day = addDays(lo - 2 * DAY, 0), end = hi + 2 * DAY; day <= end; day = addDays(day, 1)) {
        if (!cal.works(day)) from ??= day;
        else if (from !== null) offDays.push([{ xAxis: from }, { xAxis: day }]), (from = null);
      }
      if (from !== null) offDays.push([{ xAxis: from }, { xAxis: addDays(hi + 2 * DAY, 1) }]);
    }
    if (p.today !== false && drawn.length && now >= lo && now <= hi)
      marks.push({ xAxis: now, name: L("Today"), lineStyle: { color: t.palette[7] ?? t.text, type: "solid", width: 1.5 }, label: { color: t.palette[7] ?? t.text } });
    for (const m of Array.isArray(p.markers) ? p.markers : []) {
      const x = ms(isObj(m) ? m.at : m);
      if (Number.isNaN(x)) continue;
      const color = isObj(m) && !blank(m.color) ? String(m.color) : t.muted;
      marks.push({ xAxis: x, name: isObj(m) ? String(m.name ?? "") : "", lineStyle: { color, type: "dashed", width: 1 }, label: { color } });
    }
    o.series.unshift({
      type: "custom", id: "rows", silent: true, z: 1, tooltip: { show: false }, encode: { y: 0 }, data: shown.map((i) => [i.index]),
      // A line above and below each row, from the axis, so a scrolled window lines up.
      clip: true,
      renderItem: (params, api) => {
        const mid = api.coord([0, api.value(0)])[1], h = api.size([0, 1])[1];
        const line = (yy) => ({ type: "line", shape: { x1: 0, y1: Math.round(yy) + 0.5, x2: api.getWidth(), y2: Math.round(yy) + 0.5 }, style: { stroke: t.border, lineWidth: 1 } });
        return { type: "group", children: [line(mid - h / 2), line(mid + h / 2)] };
      },
      ...(marks.length ? { markLine: { silent: true, symbol: "none", animation: false, label: { show: true, position: "end", distance: 6, formatter: "{b}", fontSize: px(t)(11) }, data: marks } } : {}),
      ...(offDays.length ? { markArea: { silent: true, animation: false, itemStyle: { color: fade(t.text, 0.05) }, data: offDays } } : {}),
    });
    d.rowsBy.unshift(shown.map((i) => i.row));

    const pad = Number.isFinite(hi - lo) ? Math.max(DAY / 2, (hi - lo) * 0.02) : 0;
    o.xAxis = {
      type: "time", position: "top", min: Number.isFinite(lo) ? lo - pad : undefined, max: Number.isFinite(hi) ? hi + pad : undefined,
      axisLine: { show: false }, axisTick: { show: false }, axisLabel: { color: t.muted, hideOverlap: true, fontSize: px(t)(11), formatter: axisDate },
      splitLine: { show: true, lineStyle: { color: fade(t.border, 0.6) } },
    };
    o.yAxis = {
      type: "category", inverse: true, data: shown.map((i) => String(i.index)), triggerEvent: true,
      axisLine: { show: false }, axisTick: { show: false }, splitLine: { show: false },
      // The table beside the bars is HTML, laid over this space by the behaviour.
      axisLabel: { show: false },
    };
    const slider = p.zoom === "slider" || p.zoom === "both";
    // Room under the rows for the lines' names, and the slider.
    o.grid = { left: tableWidth + 8, right: 16, top: head, bottom: (marks.length ? GANTT.marks : GANTT.foot) + (slider ? GANTT.slider : 0) };
    if (p.tooltip !== false)
      o.tooltip = { trigger: "item", confine: true, formatter: (e) => e.data?.tip ?? "" };
    else o.tooltip = { show: false };
    if (split) o.legend = { show: p.legend !== false, top: 4, left: 8, type: "scroll", icon: "roundRect", itemWidth: 12, itemHeight: 12, data: names.filter((n) => n !== ""), width: width - (p.zoom ? 136 : 16) };
    else o.legend = { show: false };
    // `visibleRows`: at most that many rows, the rest scrolled to.
    const visible = Number(p.visibleRows) >= 1 && shown.length > Number(p.visibleRows) ? Math.floor(Number(p.visibleRows)) : 0;
    if (visible) o.grid.right += 12;
    d.height = head + (visible || Math.max(1, shown.length)) * GANTT.row + o.grid.bottom;
    // For the table: its columns, where its rows go, and each row's cells and description.
    Object.assign(g, {
      cols, head, bottom: o.grid.bottom, tableWidth, visible,
      cells: (i) => cols.map((c, n) => (n ? cell(at(i.row, c.field)) : nameOf(i))),
      about: (i) => about(i).join(", "),
    });
  }

  // How wide `text` is in the charts' 12px labels.
  let ruler;
  const textWidth = (text) => {
    ruler ??= document.createElement("canvas").getContext("2d");
    const scale = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--font-scale")) || 1;
    ruler.font = `${12 * scale}px ${getComputedStyle(document.body).fontFamily}`;
    return ruler.measureText(text).width;
  };

  // Values in value labels, tooltips and (with a `format`) the value axis, written as
  // `format` says in the document's language: `number` (the default), `currency` (in
  // `currency`) or `percent` (of 1), with `decimals` digits after the point (default: up
  // to 2, so an average doesn't show ten).
  function numbers(o, p) {
    if (!o || p.type === "gantt") return;
    const lang = document.documentElement.lang || undefined;
    const style = p.format === "currency" ? { style: "currency", currency: p.currency || "USD" } : p.format === "percent" ? { style: "percent" } : {};
    const digits = isNum(p.decimals) ? Math.max(0, Math.min(20, Number(p.decimals))) : null;
    const exact = new Intl.NumberFormat(lang, { ...style, ...(digits !== null ? { minimumFractionDigits: digits, maximumFractionDigits: digits } : { maximumFractionDigits: 2 }) });
    const fmt = (v) => (isNum(v) ? exact.format(Number(v)) : v);
    // A label's value: the number, or the value in an [x, y] pair.
    const valueOf = (v) => (Array.isArray(v) ? v[1] : v?.value ?? v);
    if (o.tooltip && o.tooltip.show !== false && !o.tooltip.formatter) o.tooltip.valueFormatter ??= (v) => (Array.isArray(v) ? v.map(fmt).join(", ") : fmt(v));
    let past = 0;
    for (const s of [o.series ?? []].flat()) {
      if (!s?.label?.show || s.label.formatter || !["bar", "line", "pictorialBar", "scatter", "effectScatter"].includes(s.type)) continue;
      s.label.formatter = (e) => String(fmt(valueOf(e.value)) ?? "");
      // Labels after the ends of bars along x: room for the longest on the right.
      if (s.label.position === "right") for (const v of s.data ?? []) past = Math.max(past, textWidth(String(fmt(valueOf(v)) ?? "")));
    }
    if (past) o.grid = { ...o.grid, right: Math.max(o.grid?.right ?? 0, Math.ceil(past) + 12) };
    // The value axis follows a `format`; big values stay short (12K, $1.5M).
    if (!p.format && digits === null) return;
    const short = new Intl.NumberFormat(lang, { ...style, notation: "compact", maximumFractionDigits: 1 });
    for (const a of [o.xAxis, o.yAxis].flat()) {
      if (a && a.type === "value" && !a.axisLabel?.formatter) a.axisLabel = { ...a.axisLabel, formatter: (v) => (Math.abs(v) >= 1e4 ? short.format(v) : exact.format(v)) };
    }
  }

  // An ECharts option from the shorthand props and the rows `prepare` gave (`d`),
  // with `x` naming the category (or name) field and `y` the value field(s). The
  // rows behind each series' data go in `d.rowsBy`, so clicks can find them.
  function build(p, t, sparkline, d) {
    const type = p.type;
    if (!type) return {};
    const rows = tabular(type) ? (Array.isArray(d.rows) ? d.rows : []) : d.rows;
    const fields = Array.isArray(rows) ? fieldsOf(rows) : [];
    const x = d.x ?? fields[0];
    const split = blank(p.series) ? null : p.series;
    const numeric = fields.filter((f) => f !== x && f !== split && rows.some((r) => r && isNum(r[f])));
    const ys = ylist(blank(d.y) ? numeric : d.y);
    const y0 = ys[0]?.field;
    const rowsBy = d.rowsBy;
    const o = { series: [] };
    const tooltip = (trigger, extra) => (o.tooltip = p.tooltip === false ? { show: false } : { trigger, confine: true, ...extra });
    let legend = false;

    if (CARTESIAN.has(type) || type === "candlestick" || type === "boxplot") {
      const cats = rows.map((r) => at(r, x));
      const scatter = type === "scatter" || type === "effectScatter";
      const bars = type === "bar" || type === "pictorialBar";
      const across = !!p.horizontal && !scatter;
      const valueX = scatter && cats.every(isNum);
      const cat = { type: "category", data: cats, boundaryGap: bars || type === "candlestick" || type === "boxplot", inverse: across };
      const val = { type: "value", scale: scatter || type === "candlestick" };
      o.xAxis = valueX ? { type: "value", scale: true } : across ? val : cat;
      o.yAxis = across ? cat : val;
      o.grid = { left: 8, right: 16, top: 16, bottom: 8 };

      if (type === "candlestick") {
        // ECharts wants open, close, low, high; fields with those names are found.
        const ohlc = ["open", "close", "low", "high"];
        const cols = p.y ? ys.map((y) => y.field) : ohlc.every((f) => fields.includes(f)) ? ohlc : numeric.slice(0, 4);
        const [up, down] = [t.palette[2], t.palette[7]];
        o.series = [{
          type, name: p.y && ys[0]?.name, data: rows.map((r) => cols.map((f) => at(r, f))),
          itemStyle: { color: up, color0: down, borderColor: up, borderColor0: down },
        }];
        rowsBy.push(rows);
        tooltip("axis", { axisPointer: { type: "cross" } });
      } else if (type === "boxplot") {
        // Five fields are min, Q1, median, Q3, max; one field is raw samples, grouped by `x`.
        let data;
        if (ys.length >= 5) {
          data = rows.map((r) => ys.slice(0, 5).map((y) => at(r, y.field)));
          rowsBy.push(rows);
        } else {
          const groups = new Map();
          for (const r of rows) {
            const k = at(r, x);
            groups.has(k) || groups.set(k, []);
            groups.get(k).push(at(r, y0));
          }
          cat.data = [...groups.keys()];
          data = [...groups.values()].map(quantiles);
          rowsBy.push(cat.data.map((k) => ({ [x]: k })));
        }
        o.series = [{
          type, data, name: ys.length === 1 ? named(ys[0]) : undefined,
          itemStyle: { color: fade(t.palette[0], 0.15), borderColor: t.palette[0], borderWidth: 2 },
        }];
        tooltip("item");
      } else {
        const sizeField = scatter ? p.value : undefined;
        let sizes;
        if (sizeField !== undefined) {
          const vs = rows.map((r) => Number(at(r, sizeField))).filter(Number.isFinite);
          const [lo, hi] = [Math.min(...vs), Math.max(...vs)];
          sizes = (d) => 8 + (32 * (Number(d[2]) - lo)) / (hi - lo || 1);
        }
        // Dates (or epoch milliseconds) along x make a time axis for lines and points.
        const time = !across && (p.xType === "time" || (blank(p.xType) && (type === "line" || type === "area" || scatter) && cats.length && cats.every(isTime)));
        if (time) {
          // Within a day or two the labels are times, which collide when frequent
          // (live data); over longer spans ECharts' own day and month levels are kept.
          const ms = cats.map((c) => (typeof c === "number" ? c : Date.parse(c))).filter(Number.isFinite);
          const short = ms.length < 2 || Math.max(...ms) - Math.min(...ms) < 2 * 864e5;
          // Bars centred on their time need room at the ends.
          o.xAxis = { type: "time", axisLabel: short ? { hideOverlap: true } : undefined, boundaryGap: bars ? ["4%", "4%"] : undefined };
        }
        else if (!across && (p.xType === "value" || p.xType === "log")) o.xAxis = { type: p.xType, scale: true };
        const pairs = scatter || (!across && o.xAxis.type !== "category");
        // `series`: one series per value of that field (long-format rows).
        if (split && !pairs) cat.data = uniq(cats);
        const specs = split
          ? uniq(rows.map((r) => at(r, split))).map((v) => {
              const mine = rows.filter((r) => same(at(r, split), v));
              const { name: _, ...y } = ys[0] || {};
              return { ...y, name: String(v), rows: pairs ? mine : cat.data.map((c) => mine.find((r) => same(at(r, x), c)) ?? null) };
            })
          : ys.map((y) => ({ ...y, rows }));
        o.series = specs.map((spec, i) => {
          const { field, name, rows: list, ...extra } = spec;
          rowsBy.push(list);
          const s = {
            id: `s${i}`,
            type: type === "area" ? "line" : type,
            name: name ?? String(field),
            data: pairs
              ? list.map((r) => [at(r, x), at(r, field), ...(sizeField !== undefined ? [at(r, sizeField)] : [])])
              : list.map((r) => (r === null ? null : at(r, field))),
          };
          if (p.stack) s.stack = "total";
          if (type === "line" || type === "area") {
            s.smooth = !!p.smooth;
            s.showSymbol = list.length <= 12;
            if (specs.length > 1) s.emphasis = { focus: "series" };
            if (type === "area") s.areaStyle = { opacity: p.stack ? 0.5 : 0.15 };
          }
          if (type === "bar") {
            s.barMaxWidth = 40;
            // Rounded at the data end; stacked segments are split by a surface gap instead.
            s.itemStyle = p.stack ? { borderColor: t.surface, borderWidth: 1 } : { borderRadius: across ? [0, 4, 4, 0] : [4, 4, 0, 0] };
          }
          if (type === "pictorialBar") Object.assign(s, { symbol: "roundRect", symbolRepeat: true, symbolMargin: 2, symbolClip: true, symbolSize: across ? [8, 16] : [16, 8] });
          if (scatter) Object.assign(s, { symbolSize: sizes || 10, itemStyle: { opacity: 0.8 } });
          if (p.labels) s.label = { show: true, position: p.stack ? "inside" : across ? "right" : "top", color: p.stack ? undefined : t.muted };
          return merge(s, extra);
        });
        tooltip(scatter ? "item" : "axis", scatter ? {} : { axisPointer: { type: bars ? "shadow" : "line" } });
        legend = o.series.length > 1;
      }
    } else if (type === "pareto") {
      // Bars largest first, and a line of the running total as a percent of the whole on
      // a second axis. A threshold (80% by default) is a dashed line on that axis; the
      // bars after the one that takes the total past it are faded, leaving the vital few.
      // `horizontal` lays the bars down the side, largest at the top, the percent along the top.
      const { field, name, ...extra } = ys[0] || {};
      const across = !!p.horizontal;
      const vals = rows.map((r) => Number(at(r, field)));
      const total = vals.reduce((a, v) => a + v, 0);
      let run = 0;
      const cum = vals.map((v) => ((run += v), total ? Math.round((run * 1000) / total) / 10 : 0));
      const limit = p.threshold === false || p.threshold === "false" ? null : isNum(p.threshold) ? Math.max(0, Math.min(100, Number(p.threshold))) : 80;
      const past = limit === null ? -1 : cum.findIndex((c) => c >= limit);
      const few = past < 0 ? rows.length : past + 1;
      const cat = { type: "category", data: rows.map((r) => at(r, x)), boundaryGap: true, inverse: across };
      const vals2 = [
        { type: "value" },
        { type: "value", min: 0, max: 100, interval: 20, axisLabel: { formatter: "{value}%" }, splitLine: { show: false } },
      ];
      o.xAxis = across ? vals2 : cat;
      o.yAxis = across ? cat : vals2;
      o.grid = { left: 8, right: across ? 24 : 16, top: 16, bottom: 8 };
      const second = across ? { xAxisIndex: 1 } : { yAxisIndex: 1 };
      const percent = (v) => (isNum(v) ? `${v}%` : v);
      o.series = [
        merge({
          id: "s0", type: "bar", name: name ?? String(field), barMaxWidth: 40, itemStyle: { borderRadius: across ? [0, 4, 4, 0] : [4, 4, 0, 0] },
          data: vals.map((v, i) => (i < few ? v : { value: v, itemStyle: { color: fade(t.palette[0], 0.4) } })),
          label: p.labels ? { show: true, position: across ? "right" : "top", color: t.muted } : undefined,
          labelLayout: p.labels ? { hideOverlap: true } : undefined,
        }, extra),
        {
          id: "s1", type: "line", name: L("Cumulative %"), ...second, data: cum, color: t.palette[1],
          symbolSize: 6, showSymbol: rows.length <= 24, tooltip: { valueFormatter: percent },
          // Above the points, giving way to the bars' labels where they'd overlap.
          label: p.labels ? { show: true, position: across ? "right" : "top", color: t.palette[1], formatter: (e) => percent(e.value) } : undefined,
          labelLayout: p.labels ? { hideOverlap: true } : undefined,
          z: 3,
          markLine: limit === null ? undefined : {
            silent: true, symbol: "none", data: [across ? { xAxis: limit } : { yAxis: limit }],
            lineStyle: { type: "dashed", color: t.muted }, label: { position: across ? "end" : "insideEndTop", formatter: `${limit}%`, color: t.muted },
          },
        },
      ];
      rowsBy.push(rows, rows);
      tooltip("axis", { axisPointer: { type: "shadow" } });
      legend = true;
    } else if (type === "pie" || type === "donut" || type === "funnel") {
      const data = rows.map((r) => ({ name: at(r, x), value: at(r, y0) }));
      const labels = p.labels !== false;
      rowsBy.push(rows);
      if (type === "funnel") {
        o.series = [{
          type, data, sort: "descending", gap: 2, left: "10%", width: "80%", top: 16, bottom: 16,
          label: { show: labels, position: "inside" }, itemStyle: { borderColor: t.surface, borderWidth: 1 },
        }];
      } else {
        o.series = [{
          // Smaller with labels, which need room outside.
          type: "pie", data, radius: type === "donut" ? (labels ? ["38%", "58%"] : ["55%", "80%"]) : [0, labels ? "58%" : "80%"],
          itemStyle: { borderColor: t.surface, borderWidth: 2, borderRadius: 4 },
          label: { show: labels, color: t.text, overflow: "none", minMargin: 4 }, labelLine: { show: labels, length: 8, length2: 8 },
        }];
      }
      tooltip("item");
      legend = !labels;
    } else if (type === "radar") {
      o.radar = {
        indicator: ys.map((y) => ({ name: named(y) })),
        radius: "65%",
        axisName: { color: t.muted },
        splitLine: { lineStyle: { color: t.border } },
        splitArea: { show: false },
        axisLine: { lineStyle: { color: t.border } },
      };
      o.series = [{
        type, symbol: "circle", symbolSize: 6, lineStyle: { width: 2 }, areaStyle: { opacity: 0.1 },
        data: rows.map((r) => ({ name: at(r, x), value: ys.map((y) => at(r, y.field)) })),
      }];
      rowsBy.push(rows);
      tooltip("item");
      legend = rows.length > 1;
    } else if (type === "heatmap") {
      // x and y are both categories; `value` (default the third field) colours the cell.
      const yf = blank(d.y) ? fields[1] : y0;
      const vf = p.value ?? fields.find((f) => f !== x && f !== yf);
      const data = rows.map((r) => [at(r, x), at(r, yf), at(r, vf)]);
      rowsBy.push(rows);
      const vs = data.map((d) => Number(d[2])).filter(Number.isFinite);
      o.xAxis = { type: "category", data: uniq(rows.map((r) => at(r, x))) };
      o.yAxis = { type: "category", data: uniq(rows.map((r) => at(r, yf))) };
      o.grid = { left: 8, right: 16, top: 8, bottom: 72 };
      o.visualMap = {
        min: vs.length ? Math.min(...vs) : 0, max: vs.length ? Math.max(...vs) : 1, calculable: true,
        orient: "horizontal", left: "center", bottom: 0, itemHeight: 160, itemWidth: 12, inRange: { color: t.seq },
      };
      o.series = [{
        type, data, label: { show: !!p.labels },
        itemStyle: { borderColor: t.surface, borderWidth: 2, borderRadius: 4 },
        emphasis: { itemStyle: { borderColor: t.text, borderWidth: 1 } },
      }];
      tooltip("item");
    } else if (type === "gauge") {
      // A number, `{ name, value }`, one object with the value in `y` (e.g. the
      // latest live reading), or rows.
      const d = p.data;
      const field = typeof p.y === "string" ? p.y : null;
      const data = Array.isArray(d) ? d.map((r) => (isObj(r) ? { name: r[p.x ?? "name"], value: r[p.y ?? "value"] } : { value: r }))
        : isObj(d) && field ? [{ name: p.x ? d[p.x] : field, value: d[field] }]
        : isObj(d) ? [d] : [{ value: d ?? 0 }];
      o.series = [{
        type, data, startAngle: 210, endAngle: -30,
        progress: { show: true, width: 12, roundCap: true },
        axisLine: { roundCap: true, lineStyle: { width: 12, color: [[1, t.border]] } },
        pointer: { show: false }, axisTick: { show: false }, splitLine: { show: false }, axisLabel: { show: false },
        anchor: { show: false }, title: { color: t.muted, offsetCenter: [0, "35%"] },
        detail: { valueAnimation: true, color: t.text, fontSize: px(t)(28), fontWeight: 600, offsetCenter: [0, "0%"] },
      }];
      tooltip("item");
    } else if (type === "tree" || type === "treemap" || type === "sunburst") {
      const d = p.data;
      const data = Array.isArray(d) ? d : isObj(d) ? [d] : [];
      if (type === "tree") {
        o.series = [{
          type, data, top: 8, bottom: 8, left: 80, right: 120, symbol: "circle", symbolSize: 8, expandAndCollapse: true,
          label: { position: "left", verticalAlign: "middle", align: "right", color: t.text },
          leaves: { label: { position: "right", align: "left" } },
          lineStyle: { color: t.border, width: 1.5 }, itemStyle: { color: t.palette[0], borderColor: t.palette[0] },
        }];
      } else if (type === "treemap") {
        o.series = [{
          // The name labels the breadcrumb's root, which zooms back out.
          type, name: L("All"), data, top: 8, left: 0, right: 0, bottom: 32, roam: false, nodeClick: "zoomToNode",
          itemStyle: { borderColor: t.surface, borderWidth: 2, gapWidth: 2, borderRadius: 4 },
          breadcrumb: { itemStyle: { color: t.surface, borderColor: t.border, textStyle: { color: t.text } } },
        }];
      } else {
        o.series = [{
          type, data, radius: [0, "92%"], itemStyle: { borderColor: t.surface, borderWidth: 2, borderRadius: 4 },
          label: { rotate: "radial" }, emphasis: { focus: "ancestor" },
        }];
      }
      tooltip("item");
    } else if (type === "sankey" || type === "graph" || type === "chord") {
      const links = Array.isArray(p.links) ? p.links : [];
      let nodes = (Array.isArray(p.data) ? p.data : []).map((n) => (isObj(n) ? { ...n } : { name: String(n) }));
      if (!nodes.length) nodes = uniq(links.flatMap((l) => [l.source, l.target])).map((n) => ({ name: String(n) }));
      if (type === "sankey") {
        o.series = [{
          type, data: nodes, links, top: 8, bottom: 8, left: 8, right: 96, nodeGap: 12, nodeWidth: 12,
          emphasis: { focus: "adjacency" }, label: { color: t.text },
          lineStyle: { color: "gradient", curveness: 0.5, opacity: 0.35 },
        }];
      } else if (type === "graph") {
        // Nodes with a `category` name are coloured, and listed in the legend, by it.
        const cats = uniq(nodes.map((n) => n.category).filter((c) => c !== undefined && typeof c !== "number"));
        for (const n of nodes) if (cats.length && typeof n.category !== "number") n.category = cats.indexOf(n.category);
        o.series = [{
          type, data: nodes, links, categories: cats.length ? cats.map((name) => ({ name })) : undefined,
          layout: nodes.some((n) => n.x !== undefined) ? "none" : "force", roam: true, draggable: true, symbolSize: 12,
          top: cats.length > 1 ? 40 : 16, bottom: 16, left: 16, right: 64,
          force: { repulsion: 160, edgeLength: 80 }, label: { show: true, position: "right", color: t.text },
          lineStyle: { color: "source", width: 1.5, opacity: 0.6 }, emphasis: { focus: "adjacency" },
        }];
        legend = cats.length > 1;
      } else {
        o.series = [{
          type, data: nodes, links, padAngle: 2, radius: ["56%", "62%"], itemStyle: { borderColor: t.surface, borderWidth: 1 },
          label: { color: t.text }, lineStyle: { color: "source", opacity: 0.35 }, emphasis: { focus: "adjacency" },
        }];
      }
      tooltip("item");
    } else if (type === "gantt") {
      gantt(p, t, d, o, rows);
    } else if (type === "themeRiver") {
      o.singleAxis = {
        type: rows.length && looksLikeDate(at(rows[0], x)) ? "time" : "value", top: 40, bottom: 32, left: 24, right: 24,
        axisLabel: { color: t.muted }, axisLine: { lineStyle: { color: t.border } }, axisTick: { show: false },
        splitLine: { show: false }, axisPointer: { show: true, label: { show: false }, lineStyle: { color: t.muted } },
      };
      o.series = [{ type, data: ys.flatMap((y) => rows.map((r) => [at(r, x), at(r, y.field), named(y)])), label: { show: false }, emphasis: { focus: "self" } }];
      rowsBy.push(ys.flatMap(() => rows));
      tooltip("axis", { axisPointer: { type: "line" } });
      legend = true;
    } else if (type === "parallel") {
      // Every `y` field is an axis and every row a line.
      o.parallelAxis = ys.map((y, i) => {
        const vs = rows.map((r) => at(r, y.field));
        return vs.every(isNum) ? { dim: i, name: named(y) } : { dim: i, name: named(y), type: "category", data: uniq(vs) };
      });
      o.parallel = {
        left: 40, right: 56, top: 32, bottom: 24,
        parallelAxisDefault: {
          nameTextStyle: { color: t.muted }, axisLabel: { color: t.muted }, axisLine: { lineStyle: { color: t.border } },
          axisTick: { lineStyle: { color: t.border } }, splitLine: { show: false },
        },
      };
      o.series = [{ type, smooth: !!p.smooth, lineStyle: { width: 2, opacity: 0.5 }, data: rows.map((r) => ys.map((y) => at(r, y.field))) }];
      rowsBy.push(rows);
    }

    for (const s of o.series) {
      // Clicks select an item, which shows until another is chosen.
      if (d.selectable) Object.assign(s, { selectedMode: "single", select: { itemStyle: { borderColor: t.text, borderWidth: 2 } } });
      // Going down or up a level morphs the old marks into the new ones.
      if (d.levels) s.universalTransition = { enabled: true, divideShape: "clone" };
    }
    if (o.series.length && type !== "gantt") {
      const show = p.legend ?? legend;
      o.legend = { show, top: 0, type: "scroll", icon: "roundRect", itemWidth: 12, itemHeight: 12 };
      if (show && o.grid) o.grid.top = 40;
      if (show && o.radar) o.radar.center = ["50%", "55%"];
      if (show && o.series[0].type === "pie") o.series[0].center = ["50%", "55%"];
    }
    if (sparkline) {
      o.grid = { left: 2, right: 2, top: 4, bottom: 4 };
      o.xAxis = { ...o.xAxis, show: false };
      o.yAxis = { ...o.yAxis, show: false, scale: type !== "bar" };
      o.legend = { show: false };
      for (const s of o.series) {
        s.showSymbol = false;
        if (s.type === "bar") s.itemStyle = { borderRadius: 2 };
      }
      if (p.tooltip !== false) o.tooltip = { trigger: "axis", confine: true, axisPointer: { type: type === "bar" ? "shadow" : "line" }, formatter: "{c}" };
    }
    return o;
  }

  // ---------------------------------------------------------------- option helpers
  const REPLACE = new Set(["data", "links", "edges", "nodes", "source", "indicator", "categories"]);
  // `over` merged into `base`. Arrays of objects (series, axes, …) merge by position,
  // like ECharts' own merge; data arrays and everything else are replaced.
  function merge(base, over, key) {
    if (isObj(base) && isObj(over)) {
      const out = { ...base };
      for (const [k, v] of Object.entries(over)) out[k] = merge(base[k], v, k);
      return out;
    }
    if (!REPLACE.has(key) && Array.isArray(over) && (Array.isArray(base) || isObj(base)) && over.every(isObj)) {
      const list = Array.isArray(base) ? base : [base];
      return over.map((v, i) => (isObj(list[i]) ? merge(list[i], v) : v)).concat(list.slice(over.length));
    }
    return over === undefined ? base : over;
  }

  // `{ "$fn": "name" }` is a function the host page provides in
  // `window.irenderChartFunctions`: formatters, `renderItem` for custom series, …
  function functions(v) {
    if (Array.isArray(v)) return v.map(functions);
    if (!isObj(v)) return v;
    const keys = Object.keys(v);
    if (keys.length === 1 && keys[0] === "$fn") {
      const f = (window.irenderChartFunctions || {})[v.$fn];
      if (typeof f !== "function") console.warn(`ui-chart: no function \`${v.$fn}\` in window.irenderChartFunctions`);
      return typeof f === "function" ? f : undefined;
    }
    return Object.fromEntries(keys.map((k) => [k, functions(v[k])]));
  }

  // For documents the policy doesn't trust: tooltips are HTML, so their template
  // text is escaped (line breaks survive), and links must not run script.
  const escapeTip = (s) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/&lt;br\s*\/?&gt;/gi, "<br/>");
  const safeUrl = (u) => typeof u === "string" && !/^\s*(javascript|vbscript|data):/i.test(u);
  function sanitize(v, tip) {
    if (Array.isArray(v)) return v.map((x) => sanitize(x, tip));
    if (!isObj(v)) return v;
    const out = {};
    for (const [k, x] of Object.entries(v)) {
      const inTip = tip || k === "tooltip";
      if (inTip && k === "extraCssText") continue;
      if ((k === "link" || k === "sublink") && !safeUrl(x)) continue;
      out[k] = inTip && k === "formatter" && typeof x === "string" ? escapeTip(x) : sanitize(x, inTip);
    }
    return out;
  }

  const hash = (s) => {
    let h = 5381;
    for (let i = 0; i < s.length; i++) h = (h * 33) ^ s.charCodeAt(i);
    return (h >>> 0).toString(36);
  };
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

  // ---------------------------------------------------------------- live data
  // Gets `c.url` every `c.every` ms, or listens to it (`stream: true` for server-sent
  // events, `ws://` / `wss://` for a WebSocket), and passes each message to `onData`.
  // `onStatus` hears connecting, live, paused or error. Polling waits while the tab
  // is hidden and catches up when it's shown.
  function feed(onData, onStatus) {
    let c = null, key = null, seq = 0, timer, source, retry = 1000;
    const every = () => Math.max(250, Number(c.every) || 5000);
    const urlOf = () => {
      let u = String(c.url);
      const q = Object.entries(isObj(c.params) ? c.params : {}).filter(([, v]) => !blank(v));
      if (q.length) u += (u.includes("?") ? "&" : "?") + new URLSearchParams(q.map(([k, v]) => [k, typeof v === "object" ? JSON.stringify(v) : String(v)]));
      return u;
    };
    function stop() {
      seq++;
      clearTimeout(timer);
      source?.close();
      source = null;
    }
    function start() {
      stop();
      if (!c || blank(c.url) || !safeUrl(String(c.url))) return;
      if (c.paused) return onStatus("paused");
      onStatus("connecting");
      const url = urlOf(), id = seq;
      if (/^wss?:/i.test(url)) socket(url, id);
      else if (c.stream) {
        source = new EventSource(url);
        source.onopen = () => id === seq && onStatus("live");
        source.onmessage = (m) => id === seq && message(m.data);
        // EventSource reconnects by itself.
        source.onerror = () => id === seq && onStatus("error");
      } else poll(url, id);
    }
    function message(text) {
      try {
        onData(JSON.parse(text));
        onStatus("live");
      } catch {
        onStatus("error");
      }
    }
    async function poll(url, id) {
      try {
        const res = await fetch(url, { headers: { accept: "application/json" }, cache: "no-store" });
        if (!res.ok) throw new Error(String(res.status));
        const body = await res.json();
        if (id !== seq) return;
        onData(body);
        onStatus("live");
      } catch {
        if (id === seq) onStatus("error");
      }
      if (id !== seq) return;
      if (document.hidden) document.addEventListener("visibilitychange", () => id === seq && poll(url, id), { once: true });
      else timer = setTimeout(() => poll(url, id), every());
    }
    function socket(url, id) {
      const ws = (source = new WebSocket(url));
      ws.onopen = () => id === seq && ((retry = 1000), onStatus("live"));
      ws.onmessage = (m) => id === seq && message(m.data);
      ws.onclose = () => {
        if (id !== seq) return;
        onStatus("error");
        timer = setTimeout(() => id === seq && socket(url, id), (retry = Math.min(retry * 2, 30000)));
      };
    }
    return {
      // New settings; a new source, params or pause state reconnects.
      set(next) {
        c = isObj(next) ? next : null;
        const k = c && JSON.stringify([c.url, c.params, !!c.stream, !!c.paused]);
        if (k !== key) {
          key = k;
          start();
        }
      },
      stop() {
        key = null;
        stop();
      },
    };
  }
  // A message applied to `rows`: it replaces them, or with `mode: append` is added
  // (rows with the same `key` replaced) and cut to the last `window` rows.
  function receive(rows, body, c) {
    const v = blank(c.pick) ? body : String(c.pick).split(".").reduce((x, k) => at(x, k), body);
    if (c.mode !== "append") return v ?? null;
    const out = Array.isArray(rows) ? rows.slice() : [];
    const add = Array.isArray(v) ? v : blank(v) ? [] : [v];
    if (blank(c.key)) out.push(...add);
    else {
      const index = new Map(out.map((r, i) => [String(at(r, c.key)), i]));
      for (const r of add) {
        const k = String(at(r, c.key));
        if (index.has(k)) out[index.get(k)] = r;
        else index.set(k, out.push(r) - 1);
      }
    }
    const n = Math.max(1, Number(c.window) || 500);
    return out.length > n ? out.slice(out.length - n) : out;
  }

  // Resolves a behaviour's `{ "$irender": template }` values against the state
  // (and the list item it is in).
  const resolver = (api) => {
    const env = () => {
      const s = api.scope();
      return s ? { ...api.state, ...s } : api.state;
    };
    const resolve = (v) => {
      if (Array.isArray(v)) return v.map(resolve);
      if (!isObj(v)) return v;
      if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
    };
    return resolve;
  };

  // ---------------------------------------------------------------- the page
  // Charts on the page, as `{ el, restyle, detach }`. They restyle when the colour
  // scheme changes, a theme attribute does (on <html> and <body>, or `data-theme`
  // anywhere) or a theme option is picked (a palette's `color.chart-*`), and are
  // disposed of when they leave the page.
  const live = new Set();
  const restyleAll = () => live.forEach((c) => c.restyle());
  let watching = false;
  const watchPage = () => {
    if (watching) return;
    watching = true;
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", restyleAll);
    addEventListener("irender:theme-option", restyleAll);
    const themes = new MutationObserver(restyleAll);
    for (const n of [document.documentElement, document.body])
      themes.observe(n, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-mode"] });
    new MutationObserver((records) => {
      if (records.some((r) => r.type === "attributes")) restyleAll();
      if (records.some((r) => r.removedNodes?.length)) live.forEach((c) => c.el.isConnected || c.detach());
    }).observe(document.documentElement, { childList: true, subtree: true, attributes: true, attributeFilter: ["data-theme", "data-mode"] });
  };

  // The tables over gantts: rows placed by the script, a ring on the focused one.
  let tableCss = false;
  const styleTables = () => {
    if (tableCss) return;
    tableCss = true;
    const css = document.createElement("style");
    css.textContent = [
      "[data-gantt-table] [role=row] { position: absolute; left: 0; right: 0; display: flex; align-items: center; box-sizing: border-box; outline: none; cursor: default; border-radius: 4px; }",
      "[data-gantt-table] [role=row][data-group] { cursor: pointer; font-weight: 600; }",
      "[data-gantt-table] [role=row][data-key]:hover { background: color-mix(in srgb, currentColor 5%, transparent); }",
      "[data-gantt-table] [role=row][aria-selected=true] { background: color-mix(in srgb, currentColor 9%, transparent); }",
      "[data-gantt-table] [role=row]:focus-visible { box-shadow: inset 0 0 0 2px var(--color-ring, currentColor); }",
      "[data-gantt-table] [role=row][data-linking] { box-shadow: inset 0 0 0 2px var(--color-primary, currentColor); background: color-mix(in srgb, currentColor 6%, transparent); }",
      "[data-gantt-table] :is([role=gridcell], [role=columnheader]) { flex: none; box-sizing: border-box; padding: 0 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }",
    ].join("\n");
    document.head.append(css);
  };

  // ---------------------------------------------------------------- the chart
  (window.__irenderBehaviors ||= {})["ui-chart"] = (el, api) => {
    const cfg = api.config;
    const resolve = resolver(api);
    let props = cfg.reactive ? resolve(cfg.props) : cfg.props;

    // Live data, when `live` is set: rows kept here, starting from `data`.
    let rows = null, version = 0, liveFeed = null;
    const current = () => (props.live && rows !== null ? { ...props, data: rows } : props);
    function syncLive() {
      const c = isObj(props.live) ? props.live : null;
      if (!c) {
        liveFeed?.stop();
        liveFeed = null;
        return;
      }
      if (rows === null) rows = props.data ?? null;
      liveFeed ||= feed(
        (body) => {
          const c = props.live || {};
          rows = receive(rows, body, c);
          version++;
          if (!blank(c.into)) api.set(c.into, rows);
          if (!blank(c.into)) return; // the state change redraws
          refresh();
        },
        (s) => {
          el.setAttribute("data-live", s);
          const key = props.live?.status;
          if (!blank(key) && api.state[key] !== s) api.set(key, s);
        },
      );
      liveFeed.set(c);
    }

    // The drill path: in state with `bindDrill`, here otherwise.
    let ownPath = [];
    const path = () => {
      const v = cfg.bindDrill ? api.state[cfg.bindDrill] : ownPath;
      return Array.isArray(v) ? v : [];
    };
    function drillTo(next) {
      if (cfg.bindDrill) api.set(cfg.bindDrill, next);
      else {
        ownPath = next;
        refresh();
      }
      api.emit("drill", { path: next });
    }

    // What a redraw depends on: the props, live rows and the drill path.
    const shapeOf = () => JSON.stringify([{ ...props, loading: undefined, data: props.live ? version : props.data }, path()]);
    let drawn = null;

    const box = document.createElement("div");
    box.style.cssText = "position:absolute;inset:0";
    el.append(box);

    let ec, chart, theme, themeKey, layout, ro, starting;
    // From the last build: the rows behind each series, the drill level, the axis
    // `bindZoom` follows, and the last values written to or read from state.
    let rowsBy = [], level = { depth: 0, levels: 0 }, zoomAxis = null, xField;
    const last = {};

    function option() {
      const p0 = current();
      const p = { ...p0, type: p0.type || (cfg.sparkline ? "area" : null) };
      const d = prepare(p, path());
      d.rowsBy = [];
      d.selectable = !!(cfg.bind || cfg.bindRow);
      Object.assign(d, { collapsed, seen, width: el.clientWidth, drafts, editable: canEdit() });
      let o = build(p, theme, cfg.sparkline, d);
      numbers(o, p);
      rowsBy = d.rowsBy;
      level = d;
      xField = d.x;

      // Zoom along the category (or time) axis: `zoom` shows the controls, and
      // `bindZoom` alone allows zooming from state.
      zoomAxis = o.xAxis ? (o.yAxis?.type === "category" && o.xAxis.type !== "category" && p.type !== "gantt" ? "y" : "x") : null;
      const z = p.zoom === true ? "both" : p.zoom;
      if (p.type === "gantt" && (z || cfg.bindZoom)) {
        // Time zooms with ⌘/Ctrl and the wheel or the buttons, and pans by dragging,
        // so the wheel still scrolls the page.
        const along = { xAxisIndex: 0, filterMode: "none" };
        // Editable, dragging moves bars: Shift and the wheel pan instead.
        o.dataZoom = [{ type: "inside", ...along, zoomOnMouseWheel: "ctrl", moveOnMouseWheel: "shift", moveOnMouseMove: !!z && !canEdit() }];
        if (p.zoom === "slider" || p.zoom === "both") o.dataZoom.push({ type: "slider", ...along, height: 20, bottom: 8 });
        if (z) o.graphic = [...(o.graphic || []), ...zoomButtons()];
      } else if (zoomAxis && !cfg.sparkline && (z || cfg.bindZoom)) {
        const along = { [`${zoomAxis}AxisIndex`]: 0, filterMode: "filter" };
        const dz = [];
        if (z === "inside" || z === "both") dz.push({ type: "inside", ...along });
        if (z === "slider" || z === "both") {
          dz.push(zoomAxis === "x" ? { type: "slider", ...along, height: 20, bottom: 4 } : { type: "slider", ...along, width: 16, right: 4 });
          if (o.grid) o.grid[zoomAxis === "x" ? "bottom" : "right"] = 40;
        }
        if (!dz.length) dz.push({ type: "inside", ...along, zoomOnMouseWheel: false, moveOnMouseMove: false, moveOnMouseWheel: false });
        o.dataZoom = dz;
      }
      // A gantt's `visibleRows`: a window of rows, moved by the wheel (or a drag when bars
      // don't move), or by a thin bar on the right. It's kept as the chart redraws.
      const win = p.type === "gantt" ? d.gantt?.visible : 0;
      if (win) {
        const n = d.gantt.shown.length, start = Math.max(0, Math.min(n - win, rowStart));
        // Its size held by the spans: `zoomLock` would leave ECharts' wheel handling off.
        const range = { yAxisIndex: 0, filterMode: "weakFilter", minValueSpan: win - 1, maxValueSpan: win - 1, startValue: start, endValue: start + win - 1 };
        o.dataZoom = [
          ...(o.dataZoom || []),
          // The wheel is the behaviour's (a row at a time); a drag moves it when bars don't.
          { type: "inside", id: "rows", ...range, zoomOnMouseWheel: false, moveOnMouseWheel: false, moveOnMouseMove: !canEdit(), preventDefaultMouseMove: !canEdit() },
          {
            type: "slider", id: "rowsBar", ...range, right: 2, width: 8, top: o.grid.top, bottom: o.grid.bottom, showDetail: false, brushSelect: false,
            showDataShadow: false, handleSize: 0, moveHandleSize: 0, borderColor: "transparent", backgroundColor: fade(theme.border, 0.35), fillerColor: fade(theme.muted, 0.45),
          },
        ];
      }
      // `brush`: drag to select points; `bindBrush` gets their rows.
      if (p.brush && o.series?.length) {
        o.brush = {
          xAxisIndex: "all", throttleType: "debounce", throttleDelay: 150,
          brushStyle: { borderWidth: 1, color: fade(theme.palette[0], 0.1), borderColor: fade(theme.palette[0], 0.6) },
          outOfBrush: { colorAlpha: 0.25 },
        };
      }
      // Below the top level of a drill: a way back up, through each level.
      if (d.levels && d.depth > 0) {
        const trail = [L("All"), ...path().slice(0, d.depth)].map(String).join("  ›  ");
        o.graphic = [{
          type: "text", id: "drill-up", left: 4, top: 2, z: 100, cursor: "pointer",
          style: { text: `‹  ${trail}`, fill: theme.muted, font: `${px(theme)(12)}px ${theme.font}` },
          onclick: () => drillTo(path().slice(0, d.depth - 1)),
        }];
        if (o.grid) o.grid.top = (o.grid.top ?? 16) + 24;
        if (o.legend) o.legend.top = 24;
      }

      if (isObj(p.option)) o = merge(o, p.option);
      // A null at the top (`color: null` from state) leaves that to the defaults: it's
      // dropped, so the chart starts over without it rather than keeping the last value.
      for (const k of Object.keys(o)) if (o[k] === null) delete o[k];
      o = functions(o);
      if (cfg.safe) o = sanitize(o);
      if (o.series && !Array.isArray(o.series)) o.series = [o.series];
      // Ids let a redraw match series to the ones on screen, so they animate.
      (o.series || []).forEach((s, i) => isObj(s) && (s.id ??= `s${i}`));
      const about = cfg.title ? L("This is a chart about “{title}”", { title: String(cfg.title).replace(/[{}]/g, "") }) : L("This is a chart");
      o.aria = merge({ enabled: true, label: { general: { withoutTitle: about } } }, o.aria);
      if (reducedMotion.matches) o.animation = false;
      return o;
    }

    // A gantt's closed groups, and the groups it has seen (so a row's own
    // `collapsed` applies once).
    const collapsed = new Set(), seen = new Set();
    // The first row a gantt's `visibleRows` window shows, kept across redraws.
    let rowStart = 0;
    // Zoom in (factor < 1) or out about the middle of the window; 0 shows everything.
    // The controls that zoom the chart's axis, not a gantt's window of rows.
    const axisZooms = () => (chart.getOption().dataZoom || []).map((z, i) => (z.id === "rows" || z.id === "rowsBar" ? -1 : i)).filter((i) => i >= 0);
    function zoomBy(factor) {
      const dz = chart.getOption().dataZoom?.[axisZooms()[0]];
      if (!dz) return;
      let a = 0, b = 100;
      if (factor) {
        const mid = (dz.start + dz.end) / 2, half = Math.min(50, Math.max(0.5, ((dz.end - dz.start) / 2) * factor));
        a = Math.max(0, mid - half);
        b = Math.min(100, a + half * 2);
        a = Math.max(0, b - half * 2);
      }
      chart.dispatchAction({ type: "dataZoom", batch: axisZooms().map((i) => ({ dataZoomIndex: i, start: a, end: b })) });
    }
    function zoomButtons() {
      const button = (id, text, right, factor) => ({
        type: "group", id, right, top: 4, cursor: "pointer", onclick: () => zoomBy(factor),
        children: [
          { type: "rect", shape: { width: 26, height: 22, r: 6 }, style: { fill: theme.surface, stroke: theme.border, lineWidth: 1 } },
          { type: "text", x: 13, y: 11, style: { text, fill: theme.text, font: `14px ${theme.font}`, align: "center", verticalAlign: "middle" } },
        ],
      });
      return [button("zoom-out", "−", 72, 2), button("zoom-in", "+", 42, 0.5), button("zoom-all", "⤢", 12, 0)];
    }

    function draw() {
      drawn = shapeOf();
      const o = option();
      // A gantt without a set height grows to fit its rows.
      const tall = cfg.autoHeight && level.height ? `${level.height}px` : null;
      const grow = tall && el.style.height !== tall;
      if (grow) el.style.height = tall;
      // A new type or a different set of components starts over; otherwise the
      // series are replaced by id and the rest merged, so values animate.
      const next = JSON.stringify([o.series?.map((s) => s?.type), Object.keys(o).sort(), level.depth]);
      const fresh = next !== layout;
      chart.setOption(o, fresh ? { notMerge: true } : { replaceMerge: ["series"] });
      if (grow) chart.resize();
      layout = next;
      ganttTable();
      // A fresh chart has forgotten its zoom, legend, selection and brush tool.
      if (fresh) for (const k of ["zoom", "legend", "select"]) delete last[k];
      syncState();
      if (fresh && current().brush) {
        const type = current().brush === true ? "rect" : current().brush;
        chart.dispatchAction({ type: "takeGlobalCursor", key: "brush", brushOption: { brushType: type, brushMode: "single" } });
      }
    }
    const refresh = () => chart && shapeOf() !== drawn && draw();

    // ---- events → state
    const rowAt = (si, di) => rowsBy[si]?.[di] ?? chart.getOption().series?.[si]?.data?.[di] ?? null;
    // Opens or closes a gantt group.
    function toggleGroup(item) {
      if (collapsed.has(item.key)) collapsed.delete(item.key);
      else collapsed.add(item.key);
      draw();
      api.emit("toggle", { id: item.key, open: !collapsed.has(item.key), row: item.row });
    }
    function onClick(e) {
      // The click that ends a drag isn't one.
      if (dragged) return;
      // An arrow, editable: the link goes.
      if (e.seriesId === "links" && canEdit()) {
        const k = level.gantt?.linkKeys?.[e.dataIndex];
        if (k) relink(k[0], k[1], false);
        return;
      }
      // A gantt group's bar opens and closes it.
      const item = e.componentType === "series" ? level.gantt?.shown[e.value?.[0]] : null;
      if (item?.group && e.value?.[4] === 1) toggleGroup(item);
      if (e.componentType !== "series") return;
      const row = rowAt(e.seriesIndex, e.dataIndex);
      const name = !blank(e.name) ? e.name : isObj(row) && !blank(at(row, xField)) ? at(row, xField) : e.value;
      if (level.levels && level.depth < level.levels - 1) drillTo([...path().slice(0, level.depth), name]);
      // Clicking the chosen item again clears the choice, so a click filter toggles.
      else if (cfg.bind) api.set(cfg.bind, same(api.state[cfg.bind], name) ? null : name ?? null);
      if (cfg.bindRow) api.set(cfg.bindRow, JSON.stringify(api.state[cfg.bindRow]) === JSON.stringify(row) ? null : row);
      // A plain `change`, so documents can react with `on: { change: … }`.
      api.emit("change", { name, value: e.value, row, seriesName: e.seriesName, seriesIndex: e.seriesIndex, dataIndex: e.dataIndex });
    }

    // The axis values at either end of the zoom window, or null when it shows everything.
    function zoomWindow() {
      const dz = chart.getOption().dataZoom?.find((z) => z.id !== "rows" && z.id !== "rowsBar");
      if (!dz || (dz.start <= 0 && dz.end >= 100)) return null;
      const axis = chart.getOption()[`${zoomAxis}Axis`]?.[0];
      if (axis?.type === "category") {
        const cats = (axis.data || []).map((c) => (isObj(c) ? c.value : c));
        const pick = (pct) => cats[Math.round((pct / 100) * (cats.length - 1))];
        return { from: pick(dz.start), to: pick(dz.end) };
      }
      let [a, b] = [dz.startValue, dz.endValue];
      try {
        [a, b] = chart.getModel().getComponent(`${zoomAxis}Axis`, 0).axis.scale.getExtent();
      } catch {}
      if (axis?.type === "time") {
        // Dates come back in the data's own form: epoch milliseconds, a day (which
        // ECharts reads as local midnight), or a full timestamp.
        const sample = rowsBy[0]?.find((r) => r)?.[xField];
        const pad = (n) => String(n).padStart(2, "0");
        const fmt = (v) => {
          const t = new Date(Math.round(v));
          if (typeof sample === "number") return t.getTime();
          if (typeof sample === "string" && sample.length === 10) return `${t.getFullYear()}-${pad(t.getMonth() + 1)}-${pad(t.getDate())}`;
          return t.toISOString();
        };
        return { from: fmt(a), to: fmt(b) };
      }
      return { from: tidy(a), to: tidy(b) };
    }
    let zoomTimer;
    // The first row a gantt's window of rows shows.
    function firstRow() {
      try {
        return Math.round(chart.getModel().getComponent("yAxis", 0).axis.scale.getExtent()[0]);
      } catch {
        return 0;
      }
    }
    // Moves a gantt's window of rows to start at row `start`; false when it can't move.
    function scrollRows(start) {
      const win = level.gantt?.visible;
      if (!win) return false;
      start = Math.max(0, Math.min(level.gantt.shown.length - win, start));
      if (start === firstRow()) return false;
      rowStart = start;
      chart.dispatchAction({ type: "dataZoom", dataZoomId: "rows", startValue: start, endValue: start + win - 1 });
      ganttTable();
      return true;
    }
    // Scrolls a gantt's window of rows so row `k` is in it.
    function showRow(k) {
      const win = level.gantt?.visible, first = firstRow();
      if (win && (k < first || k >= first + win)) scrollRows(k < first ? k : k - win + 1);
    }
    // The wheel over a gantt with `visibleRows` (bars or table): a row per row's height
    // scrolled, so a mouse's notches and a trackpad's glide both feel right. At either
    // end the page scrolls on. With ⌘/Ctrl or Shift it's the timeline's.
    let wheeled = 0;
    el.addEventListener("wheel", (e) => {
      if (!level.gantt?.visible || e.ctrlKey || e.metaKey || e.shiftKey || !e.deltaY) return;
      wheeled += e.deltaMode === 1 ? e.deltaY * GANTT.row : e.deltaY;
      const step = Math.trunc(wheeled / GANTT.row);
      const first = firstRow(), last = level.gantt.shown.length - level.gantt.visible;
      // Already at the end it's going to: the page has it.
      if ((e.deltaY < 0 && first <= 0) || (e.deltaY > 0 && first >= last)) return void (wheeled = 0);
      e.preventDefault();
      if (!step) return;
      wheeled -= step * GANTT.row;
      scrollRows(first + step);
      // Before ECharts', which stops the wheel on the bars.
    }, { passive: false, capture: true });
    function onZoom(e) {
      // A gantt's rows scrolled: the table follows; it isn't a zoom of the timeline.
      const id = e?.batch?.[0]?.dataZoomId ?? e?.dataZoomId;
      if (id === "rows" || id === "rowsBar") {
        rowStart = firstRow();
        ganttTable();
        return;
      }
      clearTimeout(zoomTimer);
      zoomTimer = setTimeout(() => {
        if (!chart) return;
        const v = zoomWindow();
        last.zoom = JSON.stringify(v);
        if (cfg.bindZoom) api.set(cfg.bindZoom, v);
        api.emit("zoom", v);
      }, 120);
    }
    function onLegend(e) {
      const visible = Object.entries(e.selected).filter(([, on]) => on).map(([n]) => n);
      last.legend = JSON.stringify(visible);
      if (cfg.bindLegend) api.set(cfg.bindLegend, visible);
      api.emit("legend", { visible, name: e.name });
    }
    function onBrush(e) {
      const picked = (e.batch?.[0]?.selected || []).flatMap((s) => (s.dataIndex || []).map((i) => rowAt(s.seriesIndex, i)));
      const v = [...new Set(picked.filter((r) => r !== null))];
      const k = JSON.stringify(v);
      if (k === last.brush) return;
      last.brush = k;
      if (cfg.bindBrush) api.set(cfg.bindBrush, v);
      api.emit("brush", { rows: v });
    }

    // ---- state → chart
    function syncState() {
      if (!chart) return;
      const s = api.state;
      const changed = (k, v) => {
        const j = JSON.stringify(v ?? null);
        if (last[k] === j) return false;
        last[k] = j;
        return true;
      };
      const all = (chart.getOption().series || []).map((_, i) => i);
      if (cfg.bind) {
        const before = last.select === undefined ? null : JSON.parse(last.select);
        if (changed("select", s[cfg.bind])) {
          if (!blank(before)) chart.dispatchAction({ type: "unselect", seriesIndex: all, name: String(before) });
          if (!blank(s[cfg.bind])) chart.dispatchAction({ type: "select", seriesIndex: all, name: String(s[cfg.bind]) });
        }
      }
      if (cfg.bindZoom && zoomAxis && changed("zoom", s[cfg.bindZoom])) {
        const v = s[cfg.bindZoom];
        const axis = chart.getOption()[`${zoomAxis}Axis`]?.[0];
        const cats = axis?.type === "category" ? (axis.data || []).map((c) => (isObj(c) ? c.value : c)) : null;
        // Dates are read the way ECharts reads them, so a day is local midnight.
        const value = (x) => (cats ? cats.findIndex((c) => same(c, x)) : axis?.type === "time" ? +ec.time.parse(x) : Number(x));
        for (const i of axisZooms()) {
          const action = { type: "dataZoom", dataZoomIndex: i };
          const [a, b] = isObj(v) ? [v.from, v.to] : [null, null];
          if (blank(a) || value(a) < 0) action.start = 0;
          else action.startValue = value(a);
          if (blank(b) || value(b) < 0) action.end = 100;
          else action.endValue = value(b);
          chart.dispatchAction(action);
        }
      }
      if (cfg.bindLegend && changed("legend", s[cfg.bindLegend])) {
        const v = s[cfg.bindLegend];
        const names = (chart.getOption().series || []).flatMap((x) => (x.type === "pie" || x.type === "funnel" ? (x.data || []).map((d) => d?.name) : [x.name]));
        for (const name of uniq(names.filter((n) => !blank(n))))
          chart.dispatchAction({ type: Array.isArray(v) && !v.some((x) => same(x, name)) ? "legendUnSelect" : "legendSelect", name });
      }
      // Clearing the state clears the brushed area.
      if (cfg.bindBrush && last.brush && last.brush !== "[]" && !(s[cfg.bindBrush] || []).length) {
        last.brush = "[]";
        chart.dispatchAction({ type: "brush", areas: [] });
      }
    }

    function showLoading() {
      if (current().loading) chart.showLoading("default", { text: "", color: theme.palette[0], maskColor: fade(theme.surface, 0.7), lineWidth: 2, spinnerRadius: 10 });
      else chart.hideLoading();
    }

    function init() {
      theme = readTheme(el);
      themeKey = JSON.stringify(theme);
      const name = `irender-${hash(themeKey)}`;
      ec.registerTheme(name, echartsTheme(theme));
      chart = ec.init(box, name, { renderer: cfg.renderer });
      if (cfg.group) {
        chart.group = cfg.group;
        ec.connect(cfg.group);
      }
      chart.on("click", onClick);
      chart.on("mousedown", onDown);
      chart.on("datazoom", onZoom);
      chart.on("legendselectchanged", onLegend);
      chart.on("brushselected", onBrush);
      layout = null;
      for (const k of Object.keys(last)) delete last[k];
      draw();
      showLoading();
      el.setAttribute("data-ready", "");
    }

    async function start() {
      if (starting) return;
      starting = true;
      try {
        ec = await load(cfg.src);
        await Promise.all(Object.entries(cfg.maps || {}).map(([n, url]) => (ec.getMap(n) ? null : loadMap(ec, n, url))));
      } catch (e) {
        console.error("ui-chart:", e);
        el.setAttribute("data-error", "");
        box.textContent = L("Chart unavailable");
        box.style.cssText += ";display:grid;place-items:center;font-size:12px;opacity:.6";
        return;
      } finally {
        starting = false;
      }
      if (!chart && el.isConnected && el.clientWidth && el.clientHeight) init();
    }

    function restyle() {
      if (!chart || !el.isConnected) return;
      if (JSON.stringify(readTheme(el)) === themeKey) return;
      chart.dispose();
      init();
    }

    // ---- editing a gantt (with `bindData`): drag a bar to move its task, or an end to
    // stretch it, a day at a time (or by `snap` with times of day); by touch, hold it first. On its row in the table,
    // Alt with Left or Right moves it a day, and with Shift too moves its end. The new
    // dates go back to the tasks in `bindData`, written as they were (a day stays a day).
    const drafts = new Map();
    let editing = null, dragged = false;
    const canEdit = () => !!cfg.bindData && current().type === "gantt" && current().editable !== false;
    const pad2 = (n) => String(n).padStart(2, "0");
    const dayShift = (v, n) => {
      if (blank(v) || !n) return v;
      if (typeof v === "number") return v + n * DAY;
      if (dateOnly(v)) {
        const [y, m, dd] = v.split("-").map(Number), x = new Date(y, m - 1, dd + n);
        return `${x.getFullYear()}-${pad2(x.getMonth() + 1)}-${pad2(x.getDate())}`;
      }
      const t = Date.parse(v);
      return Number.isNaN(t) ? v : new Date(t + n * DAY).toISOString();
    };
    // `snap`: the step a task with times of day moves by, in minutes or as "15m", "1h"
    // or "1d"; a day without it. Tasks dated in whole days always move by days.
    const snapStep = () => {
      const v = current().snap;
      if (isNum(v) && Number(v) > 0) return Number(v) * 6e4;
      const m = /^\s*(\d+(?:\.\d+)?)\s*(m|min|h|d)\s*$/i.exec(String(v ?? ""));
      return m && Number(m[1]) > 0 ? Number(m[1]) * { m: 6e4, min: 6e4, h: 36e5, d: DAY }[m[2].toLowerCase()] : DAY;
    };
    const stepOf = (row) => (dateOnly(at(row, "start")) ? DAY : snapStep());
    // A time moved `d` ms, written as it was: a local "2026-03-02T09:30" stays local.
    const timeShift = (v, d) => {
      if (blank(v) || !d) return v;
      if (typeof v === "number") return v + d;
      const t = ms(v);
      if (Number.isNaN(t)) return v;
      const m = /^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(:\d{2})?$/.exec(v);
      if (!m) return new Date(t + d).toISOString();
      const x = new Date(t + d);
      return `${x.getFullYear()}-${pad2(x.getMonth() + 1)}-${pad2(x.getDate())}T${pad2(x.getHours())}:${pad2(x.getMinutes())}${m[1] ? ":" + pad2(x.getSeconds()) : ""}`;
    };
    // A time on the step's grid, counted from its day's local midnight.
    const align = (t, step) => {
      const mid = new Date(t).setHours(0, 0, 0, 0);
      return mid + Math.round((t - mid) / step) * step;
    };
    // A task's start and end after moving it (`move`) or its start or end `n` steps (days,
    // or `snap` for times of day), or null when that would turn it inside out.
    function shifted(item, how, n) {
      const s0 = at(item.row, "start"), e0 = at(item.row, "end");
      const cal = workCalendar(current());
      if (cal && dateOnly(s0) && (blank(e0) || dateOnly(e0))) return workShifted(cal, s0, e0, how, n);
      const step = stepOf(item.row);
      if (step < DAY) {
        // Times of day: the moved edge lands on the step's grid; a move keeps the length.
        const a = ms(s0), b = blank(e0) ? NaN : ms(e0);
        let start = s0, end = e0;
        if (how === "end") end = timeShift(e0, align(b + n * step, step) - b);
        else {
          const d = align(a + n * step, step) - a;
          start = timeShift(s0, d);
          if (how === "move") end = timeShift(e0, d);
        }
        return !blank(end) && ms(end) <= ms(start) ? null : { start, end };
      }
      const start = how === "end" ? s0 : dayShift(s0, n);
      const end = how === "start" || blank(e0) ? e0 : dayShift(e0, n);
      return !blank(end) && ms(end) < ms(start) ? null : { start, end };
    }
    // `shifted` on working days: what moves lands on one (onward the way it moved), and
    // a moved task keeps its length in working days.
    const daysFrom = (v, t) => dayShift(v, Math.round((t - ms(v)) / DAY));
    function workShifted(cal, s0, e0, how, n) {
      const dir = n < 0 ? -1 : 1, a = ms(s0), b = blank(e0) ? null : ms(e0);
      let start = s0, end = e0;
      if (how === "move") {
        const a2 = cal.snap(addDays(a, n), dir);
        start = daysFrom(s0, a2);
        if (b !== null) end = daysFrom(e0, cal.add(a2, Math.max(0, cal.count(a, addDays(b, 1)) - 1)));
      } else if (how === "start") start = daysFrom(s0, cal.snap(addDays(a, n), dir));
      else if (b !== null) end = daysFrom(e0, cal.snap(addDays(b, n), dir));
      return !blank(end) && ms(end) < ms(start) ? null : { start, end };
    }
    const span = (v) => {
      const a = ms(v.start);
      return { a, b: blank(v.end) ? a : ms(v.end) + (dateOnly(v.end) ? DAY : 0) };
    };
    const keyOf = (r) => String(at(r, "id") ?? at(r, "name"));
    const afterKeys = (r) => {
      const v = at(r, "after");
      return (blank(v) ? [] : Array.isArray(v) ? v : String(v).split(",")).map((k) => String(k).trim());
    };
    // The new dates of task `key` (`v`), and with `cascade` those of the tasks that wait on
    // it, each moved on as far as it must to start when what it waits for ends.
    function pushed(list, key, v) {
      const out = new Map([[key, v]]);
      const mode = current().cascade;
      if (!mode) return out;
      // `cascade: "both"` also pulls a task back, to start when the last thing it waits
      // for ends.
      const pull = mode === "both";
      const rows = new Map(list.map((r) => [keyOf(r), r]));
      const now = (k) => out.get(k) ?? { start: at(rows.get(k), "start"), end: at(rows.get(k), "end") };
      const queue = [key];
      for (let guard = 0; queue.length && guard < 1000; guard++) {
        const k = queue.shift();
        for (const r of list) {
          if (!afterKeys(r).includes(k)) continue;
          const rk = keyOf(r), mine = now(rk), a = ms(mine.start);
          // When it can start: once all it waits for have ended.
          const ends = afterKeys(r).filter((x) => rows.has(x) && x !== rk).map((x) => span(now(x)).b).filter((x) => !Number.isNaN(x));
          if (Number.isNaN(a) || !ends.length) continue;
          const ready = Math.max(...ends);
          if (a >= ready && !pull) continue;
          let next;
          const cal = workCalendar(current());
          if (cal && dateOnly(mine.start) && (blank(mine.end) || dateOnly(mine.end))) {
            // The first working day once it ends, keeping the task's working days.
            const a2 = cal.snap(addDays(ready, 0)), b = blank(mine.end) ? null : ms(mine.end);
            if (a2 === a) continue;
            next = { start: daysFrom(mine.start, a2), end: b === null ? mine.end : daysFrom(mine.end, cal.add(a2, Math.max(0, cal.count(a, addDays(b, 1)) - 1))) };
          } else if (stepOf(r) < DAY) {
            // Times of day with `snap`: the step after it ends.
            const step = stepOf(r), d = Math.ceil((ready - a) / step - 1e-6) * step;
            if (!d) continue;
            next = { start: timeShift(mine.start, d), end: blank(mine.end) ? mine.end : timeShift(mine.end, d) };
          } else {
            // Whole days (a day changing clocks is an hour short or long).
            const n = Math.ceil((ready - a) / DAY - 0.1);
            if (!n) continue;
            next = { start: dayShift(mine.start, n), end: blank(mine.end) ? mine.end : dayShift(mine.end, n) };
          }
          out.set(rk, next);
          queue.push(rk);
        }
      }
      return out;
    }
    function commit(item, v) {
      const list = api.state[cfg.bindData];
      if (!Array.isArray(list)) return;
      const changes = pushed(list, item.key, v);
      const next = list.map((r) => {
        const c = changes.get(keyOf(r));
        return c ? { ...r, start: c.start, ...(blank(at(r, "end")) ? {} : { end: c.end }) } : r;
      });
      api.set(cfg.bindData, next);
      const moved = [...changes.keys()].filter((k) => k !== item.key);
      api.emit("edit", { id: item.key, start: v.start, end: blank(v.end) ? null : v.end, row: next.find((r) => keyOf(r) === item.key), pushed: moved });
      // Said once the chart has the new dates.
      setTimeout(() => {
        const now = level.gantt?.shown.find((i) => i.key === item.key);
        if (now) say(level.gantt.about(now) + (moved.length ? ". " + L(["{n} task after it moved too", "{n} tasks after it moved too"], { n: moved.length }) : ""));
      });
    }
    // Links: `to` waits for `from` (added to its `after`), or no longer does.
    function relink(from, to, add) {
      const list = api.state[cfg.bindData];
      if (!Array.isArray(list)) return;
      let next = list.map((r) => {
        if (keyOf(r) !== to) return r;
        const keys = afterKeys(r).filter((k) => k !== from);
        if (add) keys.push(from);
        const { after, ...rest } = r;
        return keys.length ? { ...r, after: keys.length === 1 ? keys[0] : keys } : rest;
      });
      // A new link can push the task that now waits.
      if (add) {
        const r = next.find((x) => keyOf(x) === from);
        const changes = r ? pushed(next, from, { start: at(r, "start"), end: at(r, "end") }) : new Map();
        changes.delete(from);
        next = next.map((x) => {
          const c = changes.get(keyOf(x));
          return c ? { ...x, start: c.start, ...(blank(at(x, "end")) ? {} : { end: c.end }) } : x;
        });
      }
      api.set(cfg.bindData, next);
      api.emit("link", { from, to, added: add });
      const name = (k) => String(at(list.find((x) => keyOf(x) === k), "name") ?? k);
      say(add ? L("{to} now waits for {from}", { to: name(to), from: name(from) }) : L("{to} no longer waits for {from}", { to: name(to), from: name(from) }));
    }
    // Whether `to` waiting for `from` would go round in a circle: `from` waits for `to`.
    function circular(from, to) {
      const list = api.state[cfg.bindData] || [];
      const rows = new Map(list.map((r) => [keyOf(r), r]));
      const seen = new Set(), stack = [from];
      while (stack.length) {
        const k = stack.pop();
        if (k === to) return true;
        if (seen.has(k)) continue;
        seen.add(k);
        stack.push(...(rows.has(k) ? afterKeys(rows.get(k)) : []));
      }
      return false;
    }
    function onDown(e) {
      // A task's bar (not an arrow, or the rows' lines).
      if (!canEdit() || e.componentType !== "series" || !level.gantt || e.seriesId === "links" || e.seriesId === "rows") return;
      const v = e.value, ev = e.event?.event;
      if (!Array.isArray(v) || v.length < 5 || v[4] === 1 || !ev) return;
      const item = level.gantt.shown[v[0]];
      if (!item || item.group) return;
      const p0 = chart.convertToPixel({ xAxisIndex: 0 }, v[1]), p1 = chart.convertToPixel({ xAxisIndex: 0 }, v[2]), x = e.event.offsetX;
      // Past the end of the bar: its connector, to draw a link from.
      const end = v[4] === 2 ? p0 + 9 : p1;
      if (x > end + 3) return startLink(item, end + 10, chart.convertToPixel({ yAxisIndex: 0 }, String(v[0])), ev);
      const edge = Math.min(GANTT.edge, (p1 - p0) / 3);
      const how = item.milestone ? "move" : x - p0 <= edge ? "start" : p1 - x <= edge ? "end" : "move";
      const touch = ev.pointerType === "touch" || String(ev.type).startsWith("touch");
      const pt = ev.touches?.[0] ?? ev;
      editing = { item, how, x0: pt.clientX, y0: pt.clientY, perStep: chart.convertToPixel({ xAxisIndex: 0 }, v[1] + stepOf(item.row)) - p0, n: 0, live: !touch };
      if (touch) editing.timer = setTimeout(() => editing && ((editing.live = true), navigator.vibrate?.(8)), 300);
      addEventListener("pointermove", onDragMove);
      addEventListener("pointerup", onDragEnd);
      addEventListener("pointercancel", onDragCancel);
    }
    function onDragMove(e) {
      const g = editing;
      if (!g) return;
      if (!g.live) {
        // Moved before the hold: a scroll, not a drag.
        if (Math.hypot(e.clientX - g.x0, e.clientY - g.y0) > 8) onDragCancel();
        return;
      }
      const n = Math.round((e.clientX - g.x0) / (g.perStep || 1));
      if (n === g.n) return;
      const v = shifted(g.item, g.how, n);
      if (!v) return;
      g.n = n;
      g.value = v;
      // The tasks it pushes move with it.
      drafts.clear();
      for (const [k, x] of pushed(api.state[cfg.bindData] || [], g.item.key, v)) drafts.set(k, span(x));
      chart.dispatchAction({ type: "hideTip" });
      draw();
    }
    function stopDrag() {
      clearTimeout(editing?.timer);
      removeEventListener("pointermove", onDragMove);
      removeEventListener("pointerup", onDragEnd);
      removeEventListener("pointercancel", onDragCancel);
      const g = editing;
      editing = null;
      if (g) drafts.clear();
      return g;
    }
    function onDragEnd() {
      const g = stopDrag();
      if (!g?.n || !g.value) return;
      dragged = true;
      setTimeout(() => (dragged = false));
      commit(g.item, g.value);
    }
    function onDragCancel() {
      const g = stopDrag();
      if (g?.n) draw();
    }
    // ---- drawing a link: from a bar's connector to the task that waits on it, with a
    // dashed line following the pointer
    let linking = null;
    function startLink(item, x, y, ev) {
      const box = el.getBoundingClientRect();
      const line = new ec.graphic.Line({ shape: { x1: x, y1: y, x2: x, y2: y }, style: { stroke: theme.text, lineWidth: 1.5, lineDash: [4, 3] }, z: 1000, silent: true });
      chart.getZr().add(line);
      linking = { item, line, box, x0: (ev.touches?.[0] ?? ev).clientX, y0: (ev.touches?.[0] ?? ev).clientY };
      chart.dispatchAction({ type: "hideTip" });
      addEventListener("pointermove", onLinkMove);
      addEventListener("pointerup", onLinkEnd);
      addEventListener("pointercancel", stopLink);
    }
    // The task under a point on the page, if a link can go to it.
    function linkTarget(clientX, clientY) {
      const g = level.gantt, x = clientX - linking.box.left, y = clientY - linking.box.top;
      if (!g || !chart.containPixel({ gridIndex: 0 }, [x, y])) return null;
      const to = g.shown[Math.round(chart.convertFromPixel({ yAxisIndex: 0 }, y))];
      return to && !to.group && to.key !== linking.item.key ? to : null;
    }
    function onLinkMove(e) {
      if (!linking) return;
      linking.line.setShape({ x2: e.clientX - linking.box.left, y2: e.clientY - linking.box.top });
      const to = linkTarget(e.clientX, e.clientY);
      linking.line.setStyle({ stroke: to ? (circular(linking.item.key, to.key) ? theme.palette[7] ?? "#dc2626" : theme.palette[0]) : theme.text });
    }
    function stopLink() {
      removeEventListener("pointermove", onLinkMove);
      removeEventListener("pointerup", onLinkEnd);
      removeEventListener("pointercancel", stopLink);
      if (linking) chart.getZr().remove(linking.line);
      const l = linking;
      linking = null;
      return l;
    }
    function onLinkEnd(e) {
      const to = linking && linkTarget(e.clientX, e.clientY), l = stopLink();
      if (!l || !to) return;
      dragged = true;
      setTimeout(() => (dragged = false));
      // Not already linked, and not round in a circle.
      const r = (api.state[cfg.bindData] || []).find((x) => keyOf(x) === to.key);
      if (r && afterKeys(r).includes(l.item.key)) return;
      if (circular(l.item.key, to.key)) return say(L("{to} can't wait for {from}: {from} waits for it", { to: to.row.name ?? to.key, from: l.item.row.name ?? l.item.key }));
      relink(l.item.key, to.key, true);
    }

    // While a bar is dragged by touch, the page doesn't scroll; Escape puts it back.
    el.addEventListener("touchmove", (e) => editing?.live && e.cancelable && e.preventDefault(), { passive: false });
    addEventListener("keydown", (e) => e.key === "Escape" && (editing ? onDragCancel() : linking && stopLink()));
    // Edits are announced here.
    let status;
    const say = (text) => {
      if (!status) {
        status = document.createElement("div");
        status.setAttribute("role", "status");
        status.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap";
        el.append(status);
      }
      status.textContent = "";
      setTimeout(() => (status.textContent = text), 30);
    };

    // ---- a gantt's table: HTML laid over the space left of the bars, a row per task, so
    // it can be read and used from the keyboard. Up and Down move between rows, Right
    // and Left open and close a group (or step into and out of it), and Enter or Space
    // opens a group or picks a task, as a click does. A focused task shows its tooltip.
    let table = null, focusKey = null, pickedKey = null, linkFrom = null;
    function stopKeyLink(announce) {
      linkFrom = null;
      table?.querySelectorAll("[data-linking]").forEach((r) => r.removeAttribute("data-linking"));
      if (announce) say(L("Stopped linking"));
    }
    // The picked task follows `bindRow`, so a click on a bar marks its row too.
    function syncPicked() {
      const g = level.gantt;
      if (!g || !cfg.bindRow) return;
      const v = JSON.stringify(api.state[cfg.bindRow] ?? null);
      pickedKey = g.shown.find((i) => JSON.stringify(i.row) === v)?.key ?? null;
      for (const r of table?.querySelectorAll("[role=row][data-key]") ?? []) r.setAttribute("aria-selected", String(r.getAttribute("data-key") === pickedKey));
    }
    function ganttTable() {
      const g = level.gantt;
      if (!g) {
        table?.remove();
        table = null;
        return;
      }
      styleTables();
      if (!table) {
        table = document.createElement("div");
        table.setAttribute("role", "treegrid");
        table.setAttribute("data-gantt-table", "");
        table.addEventListener("click", onTableClick);
        table.addEventListener("keydown", onTableKey);
        table.addEventListener("focusin", (e) => showTip(e.target.closest?.("[role=row][data-key]")));
        table.addEventListener("focusout", () => chart?.dispatchAction({ type: "hideTip" }));
        el.append(table);
      }
      table.setAttribute("aria-label", cfg.title ? String(cfg.title) : L("Tasks"));
      // With `visibleRows`, a window of them: each row where the axis puts it, the rest hidden.
      const n = g.visible || Math.max(1, g.shown.length), h = (el.clientHeight - g.head - g.bottom) / n;
      const first = g.visible ? firstRow() : 0;
      Object.assign(table.style, { position: "absolute", left: "0", top: "0", width: `${g.tableWidth}px`, height: "0", font: `${px(theme)(12)}px ${theme.font}`, color: theme.text });
      const had = table.contains(document.activeElement) ? document.activeElement.getAttribute("data-key") : null;
      const make = (role, attrs, style) => {
        const x = document.createElement("div");
        x.setAttribute("role", role);
        for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined) x.setAttribute(k, v);
        Object.assign(x.style, style);
        return x;
      };
      const cellsOf = (texts, head, depth, mark) =>
        g.cols.map((c, k) => {
          const x = make(head ? "columnheader" : "gridcell", {}, { width: `${c.width}px`, textAlign: c.align === "right" && k ? "right" : "left", color: k || head ? theme.muted : "" });
          if (!k && !head) {
            x.style.paddingInlineStart = `${8 + depth * GANTT.indent}px`;
            if (mark) {
              const m = document.createElement("span");
              m.setAttribute("aria-hidden", "true");
              m.textContent = mark;
              x.append(m);
            }
          }
          x.append(texts[k] ?? "");
          return x;
        });
      const header = make("row", { "data-head": "" }, { top: `${g.head - 30}px`, height: "28px", fontWeight: "600", borderBottom: "0" });
      header.append(...cellsOf(g.cols.map((c) => c.name), true));
      if (!g.shown.some((i) => i.key === focusKey)) focusKey = g.shown[0]?.key ?? null;
      const rows = g.shown.map((i, k) => {
        const r = make("row", {
          "data-key": i.key, "aria-level": String(i.depth + 1), "aria-expanded": i.group ? String(i.open) : null,
          "aria-selected": String(i.key === pickedKey), "aria-label": g.about(i), tabindex: i.key === focusKey ? "0" : "-1",
          "data-group": i.group ? "" : null, "data-linking": i.key === linkFrom ? "" : null,
        }, { top: `${g.head + (k - first) * h}px`, height: `${h}px`, display: k < first || k >= first + n ? "none" : "" });
        r.append(...cellsOf(g.cells(i), false, i.depth, i.group ? (i.open ? "▾ " : "▸ ") : ""));
        return r;
      });
      table.replaceChildren(header, ...rows);
      syncPicked();
      if (had) table.querySelector(`[data-key="${CSS.escape(had)}"]`)?.focus();
    }
    const itemOf = (row) => level.gantt?.shown.find((i) => i.key === row?.getAttribute("data-key"));
    // A task picked from the table, as a click on its bar picks it.
    function pick(item) {
      const again = pickedKey === item.key;
      pickedKey = again ? null : item.key;
      for (const r of table.querySelectorAll("[role=row][data-key]")) r.setAttribute("aria-selected", String(r.getAttribute("data-key") === pickedKey));
      const name = at(item.row, "name") ?? item.key;
      if (cfg.bind) api.set(cfg.bind, again ? null : name);
      if (cfg.bindRow) api.set(cfg.bindRow, again ? null : item.row);
      api.emit("change", { name, row: item.row });
    }
    function onTableClick(e) {
      const row = e.target.closest("[role=row][data-key]"), item = itemOf(row);
      if (!item) return;
      focusKey = item.key;
      if (item.group) toggleGroup(item);
      else pick(item);
    }
    function onTableKey(e) {
      const rows = [...table.querySelectorAll("[role=row][data-key]")], row = e.target.closest("[role=row][data-key]");
      const k = rows.indexOf(row), item = itemOf(row);
      if (!item) return;
      const go = (to) => {
        const r = rows[Math.max(0, Math.min(rows.length - 1, to))];
        if (!r) return;
        e.preventDefault();
        focusKey = r.getAttribute("data-key");
        // Out of a gantt's window of rows: scrolled to first (which draws the rows again).
        showRow(level.gantt.shown.findIndex((i) => i.key === focusKey));
        const now = table.querySelector(`[role=row][data-key="${CSS.escape(focusKey)}"]`) ?? r;
        for (const x of table.querySelectorAll("[role=row][data-key]")) x.tabIndex = x === now ? 0 : -1;
        now.focus();
      };
      // Links from the keyboard: L on a task starts one, Enter on another makes it wait
      // for the first (or, if it already does, no longer), Escape stops.
      if (canEdit() && !e.altKey && !e.ctrlKey && !e.metaKey) {
        if ((e.key === "l" || e.key === "L") && !item.group) {
          e.preventDefault();
          if (linkFrom === item.key) return stopKeyLink(true);
          linkFrom = item.key;
          table.querySelectorAll("[data-linking]").forEach((r) => r.removeAttribute("data-linking"));
          row.setAttribute("data-linking", "");
          return say(L("Linking from {name}: go to the task that waits for it and press Enter, or Escape to stop", { name: String(at(item.row, "name") ?? item.key) }));
        }
        if (linkFrom && e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          return stopKeyLink(true);
        }
        if (linkFrom && (e.key === "Enter" || e.key === " ") && !item.group && item.key !== linkFrom) {
          e.preventDefault();
          const from = linkFrom, r = (api.state[cfg.bindData] || []).find((x) => keyOf(x) === item.key);
          stopKeyLink(false);
          if (r && afterKeys(r).includes(from)) return relink(from, item.key, false);
          const fromRow = (api.state[cfg.bindData] || []).find((x) => keyOf(x) === from);
          if (circular(from, item.key)) return say(L("{to} can't wait for {from}: {from} waits for it", { to: String(at(item.row, "name") ?? item.key), from: String(at(fromRow, "name") ?? from) }));
          return relink(from, item.key, true);
        }
      }
      // Alt with Left or Right moves the task a day (or a `snap`); with Shift too, its end.
      if (e.altKey && (e.key === "ArrowLeft" || e.key === "ArrowRight") && canEdit() && !item.group && !(e.shiftKey && item.milestone)) {
        e.preventDefault();
        const v = shifted(item, e.shiftKey ? "end" : "move", e.key === "ArrowRight" ? 1 : -1);
        if (v) commit(item, v);
        return;
      }
      switch (e.key) {
        case "ArrowDown": return go(k + 1);
        case "ArrowUp": return go(k - 1);
        case "Home": return go(0);
        case "End": return go(rows.length - 1);
        case "ArrowRight":
          e.preventDefault();
          if (item.group && !item.open) return toggleGroup(item);
          if (item.group) return go(k + 1);
          return;
        case "ArrowLeft": {
          e.preventDefault();
          if (item.group && item.open) return toggleGroup(item);
          // To the group it's in.
          for (let j = k - 1; j >= 0; j--) if (Number(rows[j].getAttribute("aria-level")) === item.depth) return go(j);
          return;
        }
        case "Enter":
        case " ":
          e.preventDefault();
          return item.group ? toggleGroup(item) : pick(item);
      }
    }
    // The focused task's tooltip, from its bar.
    function showTip(row) {
      const item = itemOf(row);
      if (!item || !chart) return;
      const series = chart.getOption().series || [];
      for (let si = 0; si < series.length; si++) {
        const di = (series[si].data || []).findIndex((x) => isObj(x) && Array.isArray(x.value) && x.value[0] === item.index && x.tip);
        if (di >= 0) return chart.dispatchAction({ type: "showTip", seriesIndex: si, dataIndex: di });
      }
    }

    // Taken out of the page (an `if`, a rebuilt list): let go of everything.
    const self = {
      el,
      restyle,
      detach() {
        ro?.disconnect();
        ro = null;
        live.delete(self);
        liveFeed?.stop();
        liveFeed = null;
        chart?.dispose();
        chart = null;
      },
    };

    // Charts start once they have a size (so hidden tabs and dialogs work) and
    // follow it.
    function watch() {
      ro = new ResizeObserver(() => {
        if (!el.isConnected || !el.clientWidth || !el.clientHeight) return;
        if (!chart) return start();
        chart.resize();
        // A gantt's table fits the width it has, and its rows the height.
        if (level.gantt && level.width !== el.clientWidth) draw();
        else if (level.gantt) ganttTable();
      });
      ro.observe(el);
      live.add(self);
      watchPage();
      syncLive();
      if (el.clientWidth && el.clientHeight) start();
    }
    watch();

    return {
      update() {
        // Shown again by an `if`: start over.
        if (!ro && el.isConnected) watch();
        if (cfg.reactive) {
          props = resolve(cfg.props);
          syncLive();
        }
        if (!chart) return;
        if (shapeOf() !== drawn) draw();
        else syncState();
        syncPicked();
        showLoading();
      },
      get chart() {
        return chart;
      },
    };
  };

  // ---------------------------------------------------------------- ui-live-data
  // Keeps a state key filled from a live source, for any component to read.
  (window.__irenderBehaviors ||= {})["ui-live-data"] = (el, api) => {
    const cfg = api.config;
    const resolve = resolver(api);
    let c = resolve(cfg.props);
    const source = feed(
      (body) => api.set(cfg.into, receive(api.state[cfg.into], body, c)),
      (s) => {
        el.setAttribute("data-status", s);
        if (cfg.status && api.state[cfg.status] !== s) api.set(cfg.status, s);
      },
    );
    let attached = true;
    const self = {
      el,
      restyle() {},
      detach() {
        attached = false;
        live.delete(self);
        source.stop();
      },
    };
    live.add(self);
    watchPage();
    source.set(c);
    return {
      update() {
        if (!attached && el.isConnected) {
          attached = true;
          live.add(self);
        }
        if (!attached) return;
        c = resolve(cfg.props);
        source.set(c);
      },
    };
  };
})();
