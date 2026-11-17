// irender-chart-kit: the `ui-chart` behaviour. Loads ECharts once per page, builds
// an option from the shorthand props, themes it from the page's CSS tokens, and
// redraws when the state it reads changes.
(() => {
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
    let surface = has("--color-surface") ? color("var(--color-surface)") : null;
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
    return { text, muted, border, surface, palette, seq: [low, palette[0]], dark, font: cs.fontFamily };
  }

  const compact = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });

  // Defaults for every chart, as an ECharts theme: recessive axes and grid lines,
  // text in text colours, a tooltip that looks like the page's popovers.
  function echartsTheme(t) {
    const axis = {
      axisLine: { lineStyle: { color: t.border } },
      axisTick: { show: false },
      axisLabel: { color: t.muted },
      splitLine: { lineStyle: { color: t.border } },
      splitArea: { show: false },
      nameTextStyle: { color: t.muted },
    };
    // Big numbers in short form (12K, 1.5M), and labels that would collide left out.
    const value = {
      ...axis,
      axisLine: { show: false, lineStyle: { color: t.border } },
      axisLabel: { color: t.muted, hideOverlap: true, formatter: (v) => (Math.abs(v) >= 1e4 ? compact.format(v) : v.toLocaleString()) },
    };
    return {
      color: t.palette,
      backgroundColor: "transparent",
      textStyle: { color: t.text, fontFamily: t.font },
      title: { textStyle: { color: t.text }, subtextStyle: { color: t.muted } },
      legend: {
        textStyle: { color: t.muted },
        pageTextStyle: { color: t.muted },
        pageIconColor: t.text,
        pageIconInactiveColor: t.border,
        inactiveColor: t.border,
      },
      tooltip: {
        backgroundColor: t.surface,
        borderColor: t.border,
        borderWidth: 1,
        padding: [8, 10],
        textStyle: { color: t.text, fontSize: 12 },
        extraCssText: "border-radius:8px;box-shadow:0 4px 12px rgb(0 0 0 / 0.1);",
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
    if (!blank(p.sort)) rows = sortRows(rows, p.sort);
    if (isNum(p.limit) && Number(p.limit) > 0) rows = rows.slice(0, Number(p.limit));
    return { ...out, rows, x, y };
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
        detail: { valueAnimation: true, color: t.text, fontSize: 28, fontWeight: 600, offsetCenter: [0, "0%"] },
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
          type, name: "All", data, top: 8, left: 0, right: 0, bottom: 32, roam: false, nodeClick: "zoomToNode",
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
    if (o.series.length) {
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
      let o = build(p, theme, cfg.sparkline, d);
      rowsBy = d.rowsBy;
      level = d;
      xField = d.x;

      // Zoom along the category (or time) axis: `zoom` shows the controls, and
      // `bindZoom` alone allows zooming from state.
      zoomAxis = o.xAxis ? (o.yAxis?.type === "category" && o.xAxis.type !== "category" ? "y" : "x") : null;
      const z = p.zoom === true ? "both" : p.zoom;
      if (zoomAxis && !cfg.sparkline && (z || cfg.bindZoom)) {
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
        const trail = ["All", ...path().slice(0, d.depth)].map(String).join("  ›  ");
        o.graphic = [{
          type: "text", id: "drill-up", left: 4, top: 2, z: 100, cursor: "pointer",
          style: { text: `‹  ${trail}`, fill: theme.muted, font: `12px ${theme.font}` },
          onclick: () => drillTo(path().slice(0, d.depth - 1)),
        }];
        if (o.grid) o.grid.top = (o.grid.top ?? 16) + 24;
        if (o.legend) o.legend.top = 24;
      }

      if (isObj(p.option)) o = merge(o, p.option);
      o = functions(o);
      if (cfg.safe) o = sanitize(o);
      if (o.series && !Array.isArray(o.series)) o.series = [o.series];
      // Ids let a redraw match series to the ones on screen, so they animate.
      (o.series || []).forEach((s, i) => isObj(s) && (s.id ??= `s${i}`));
      const about = cfg.title ? `This is a chart about “${String(cfg.title).replace(/[{}]/g, "")}”` : "This is a chart";
      o.aria = merge({ enabled: true, label: { general: { withoutTitle: about } } }, o.aria);
      if (reducedMotion.matches) o.animation = false;
      return o;
    }

    function draw() {
      drawn = shapeOf();
      const o = option();
      // A new type or a different set of components starts over; otherwise the
      // series are replaced by id and the rest merged, so values animate.
      const next = JSON.stringify([o.series?.map((s) => s?.type), Object.keys(o).sort(), level.depth]);
      const fresh = next !== layout;
      chart.setOption(o, fresh ? { notMerge: true } : { replaceMerge: ["series"] });
      layout = next;
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
    function onClick(e) {
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
      const dz = chart.getOption().dataZoom?.[0];
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
    function onZoom() {
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
        const n = chart.getOption().dataZoom?.length || 0;
        for (let i = 0; i < n; i++) {
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
        box.textContent = "Chart unavailable";
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
        if (chart) chart.resize();
        else start();
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
