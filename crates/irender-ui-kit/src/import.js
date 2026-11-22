// irender-ui-kit: the `ui-import` behaviour. Reads the file (sheets.js) or pasted cells,
// matches its columns to the fields by their names, turns each cell into its field's value
// (cells.js) and puts the rows in state, where the review step's data table checks and
// edits them. Then hands over the rows that pass, or posts them in batches.
(window.__irenderBehaviors ||= {})["ui-import"] = (el, api) => {
  const cfg = api.config, C = window.__irenderCells, Z = window.__irenderSheets, L = (t, v) => window.__irender.label(t, v);
  const $ = (s, from = el) => from.querySelector(s);
  const steps = ["upload", "match", "review", "done"];
  const panel = (name) => $(`[data-imp-step=${name}]`);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const resolve = (v) => isObj(v) ? ("$irender" in v ? window.__irender.evaluate(v.$irender, { ...api.state, ...(api.scope() ?? {}) }) : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]))) : Array.isArray(v) ? v.map(resolve) : v;
  const n = (x) => Number(x).toLocaleString(cfg.locale || undefined);
  let cells = [], file = null, name = "", header = true, map = {}, at = "upload", sending = false, taken = [];

  function go(step, focus = true) {
    at = step;
    for (const s of steps) panel(s).hidden = s !== step;
    el.querySelectorAll(":scope > ol > li").forEach((li, i) => {
      li.toggleAttribute("data-done", i < steps.indexOf(step));
      if (i === steps.indexOf(step)) li.setAttribute("aria-current", "step");
      else li.removeAttribute("aria-current");
    });
    if (focus) panel(step).focus({ preventScroll: true });
    api.emit("step", { step });
  }
  const fail = (text) => {
    const p = $("[data-imp-error][role=alert]", panel("upload"));
    p.textContent = text ?? "";
    p.hidden = !text;
  };

  // ---- 1. reading
  async function read(f, sheet = 0) {
    fail(null);
    try {
      const x = /\.xlsx$/i.test(f.name);
      const rows = x ? await Z.xlsx(await f.arrayBuffer(), sheet) : Z.csv(await f.text(), /\.tsv$/i.test(f.name) ? "\t" : undefined);
      file = f;
      take(rows, f.name, rows.sheets ?? [], sheet);
    } catch (e) {
      fail(/\.xls$/i.test(f.name) ? L("Old Excel files (.xls) can't be read: save it as .xlsx or CSV.") : L("Couldn't read {name}: {error}", { name: f.name, error: e.message }));
    }
  }
  function take(rows, title, sheets = [], sheet = 0) {
    rows = rows.map((r) => r.map((c) => String(c ?? "").trim())).filter((r) => r.some((c) => c !== ""));
    if (!rows.length) return fail(L("There's nothing in it."));
    if (rows.length - 1 > cfg.maxRows) return fail(L("It has {n} rows; at most {max} can be imported at once.", { n: rows.length - 1, max: cfg.maxRows }));
    const width = Math.max(...rows.map((r) => r.length));
    cells = rows.map((r) => Array.from({ length: width }, (_, i) => r[i] ?? ""));
    name = title;
    taken = [];
    // A first row of names: some are the fields', or none of it is a number or a date.
    header = cells.length > 1 && (cfg.fields.some((f) => cells[0].some((h) => score(f, h) > 0.8)) || cells[0].every((h) => h !== "" && C.number(h, cfg.locale) === null && !/^\d{4}-\d{2}/.test(h)));
    match();
    const pick = $("[data-imp-sheets]");
    pick.hidden = sheets.length < 2;
    $("select", pick).replaceChildren(...sheets.map((s, i) => new Option(s, String(i), false, i === sheet)));
    go("match");
  }

  // ---- 2. matching
  const plain = (s) => C.fold(s).replace(/[^\p{L}\p{N}]+/gu, " ").trim();
  // How alike a field and a column's name are, 0 to 1.
  function score(f, h) {
    const head = plain(h);
    if (!head) return 0;
    let best = 0;
    for (const want of [f.label, f.field.replace(/([a-z])([A-Z])/g, "$1 $2"), ...(Array.isArray(f.aliases) ? f.aliases : [])].map(plain)) {
      if (!want) continue;
      if (want === head || want.replace(/ /g, "") === head.replace(/ /g, "")) return 1;
      const a = new Set(want.split(" ")), b = new Set(head.split(" "));
      const both = [...a].filter((w) => b.has(w)).length;
      best = Math.max(best, both / new Set([...a, ...b]).size, want.length > 3 && head.length > 3 && (head.includes(want) || want.includes(head)) ? 0.6 : 0);
    }
    return best;
  }
  function match() {
    map = {};
    if (!header) cfg.fields.forEach((f, i) => (map[f.field] = i < cells[0].length ? i : null));
    else {
      const pairs = cfg.fields.flatMap((f) => cells[0].map((h, i) => [f.field, i, score(f, h)])).filter((p) => p[2] >= 0.5).sort((a, b) => b[2] - a[2]);
      const used = new Set();
      for (const [field, i] of pairs) if (map[field] == null && !used.has(i)) (map[field] = i), used.add(i);
    }
    drawMap();
  }
  const letter = (i) => (i >= 26 ? String.fromCharCode(64 + Math.floor(i / 26)) : "") + String.fromCharCode(65 + (i % 26));
  const columnName = (i) => (header && cells[0][i]) || L("Column {name}", { name: letter(i) });
  function drawMap() {
    const box = $("[data-imp-map]"), make = (tag, text, attrs = {}) => {
      const e = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
      if (text != null) e.textContent = text;
      return e;
    };
    const out = [make("span", L("Field"), { "data-head": "" }), make("span", L("Column in the file"), { "data-head": "" }), make("span", L("Sample"), { "data-head": "" })];
    const rows = cells.slice(header ? 1 : 0);
    for (const f of cfg.fields) {
      const label = make("span"), s = make("select", null, { "aria-label": L("Column for {name}", { name: f.label }), "data-imp-field": f.field }), cell = make("span");
      label.append(make("b", f.label));
      if (f.required) label.append(make("span", "*", { "data-req": "", "aria-label": L("Required") }));
      s.append(new Option(L("Not in the file"), ""));
      cells[0].forEach((_, i) => s.append(new Option(columnName(i), String(i), false, map[f.field] === i)));
      s.value = map[f.field] == null ? "" : String(map[f.field]);
      cell.append(s);
      const sample = map[f.field] == null ? "" : rows.map((r) => r[map[f.field]]).filter(Boolean).slice(0, 3).join(" · ");
      out.push(label, cell, make("span", sample || "—", { "data-sample": "", title: sample }));
    }
    box.replaceChildren(...out);
    $("[data-imp-header] input").checked = header;
    $("[data-imp-file]").textContent = `${name} · ${L(["{n} row", "{n} rows"], { n: rows.length })}`;
    const missing = cfg.fields.filter((f) => f.required && map[f.field] == null).map((f) => f.label);
    const p = $("[data-imp-missing]");
    p.textContent = missing.length ? L("Required, with no column: {names}", { names: missing.join(", ") }) : "";
    $("[data-imp-review]").disabled = !rows.length || cfg.fields.every((f) => map[f.field] == null);
  }

  // ---- 3. the rows, as values
  function review() {
    const rows = cells.slice(header ? 1 : 0).map((r, i) => {
      const row = { _row: i + (header ? 2 : 1) };
      for (const f of cfg.fields) {
        const text = map[f.field] == null ? "" : r[map[f.field]] ?? "";
        row[f.field] = text === "" && f.default !== undefined ? f.default : C.parse(f, text, cfg.locale);
      }
      return row;
    });
    tableError({});
    api.set(cfg.bind, rows);
    go("review");
    counts();
  }
  const table = $("[data-i-x=ui-data-table]", panel("review"));
  // Messages for the table's cells, e.g. the server's: part of what it checks.
  function tableError(errors) {
    const c = table.__xc ?? (table.__xc = JSON.parse(table.getAttribute("data-i-xc")));
    c.errors = errors;
  }
  const given = () => table.__xc?.errors ?? {};
  function split() {
    const rows = Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind] : [], ok = [], bad = [];
    const uses = new Map(cfg.fields.filter((f) => f.unique).map((f) => [f.field, C.count(rows.map((r) => r[f.field]))]));
    for (const r of rows) (given()[r._row] || cfg.fields.some((f) => C.check(f, r[f.field], uses.get(f.field))) ? bad : ok).push(r);
    return { ok, bad };
  }
  function counts() {
    if (at !== "review") return;
    const { ok, bad } = split();
    const text = L(["{n} row is ready", "{n} rows are ready"], { n: ok.length }) + (bad.length ? ` · ${L(["{n} needs attention", "{n} need attention"], { n: bad.length })}${cfg.skip ? ` (${L("left out unless put right")})` : ""}` : "");
    const c = $("[data-imp-counts]"), go = $("[data-imp-go]");
    if (c.textContent !== text) c.textContent = text;
    go.disabled = !ok.length || (!cfg.skip && bad.length > 0);
    const t = L(["Import {n} row", "Import {n} rows"], { n: ok.length });
    if (go.textContent !== t) go.textContent = t;
  }

  // ---- 4. handing over
  const clean = ({ _row, ...r }) => r;
  async function send() {
    if (sending) return;
    const { ok, bad } = split(), url = resolve(cfg.url);
    const sent = [], errors = {};
    go("done");
    $("[data-imp-sending]").hidden = false;
    $("[data-imp-done]").hidden = true;
    sending = true;
    const progress = (done) => {
      $("[data-imp-progress]").textContent = L("{done} of {total} rows", { done: n(done), total: n(ok.length) });
      $("progress", panel("done")).value = ok.length ? (done / ok.length) * 100 : 100;
    };
    progress(0);
    let stopped = null;
    if (url) {
      for (let i = 0; i < ok.length; i += cfg.batch) {
        const part = ok.slice(i, i + cfg.batch);
        try {
          const res = await fetch(url, { method: "POST", headers: { "content-type": "application/json", accept: "application/json" }, body: JSON.stringify({ ...(resolve(cfg.params) ?? {}), rows: part.map(clean), offset: i, total: ok.length }) });
          const answer = (res.headers.get("content-type") ?? "").includes("json") ? await res.json() : null;
          if (!res.ok && !Array.isArray(answer?.errors)) throw new Error(answer?.error || answer?.message || `${res.status} ${res.statusText}`.trim());
          const back = new Set();
          for (const e of Array.isArray(answer?.errors) ? answer.errors : []) {
            const r = part[e.row];
            if (!r) continue;
            back.add(r._row);
            ((errors[r._row] ??= {})[e.field ?? cfg.fields[0].field] = String(e.message ?? L("Not accepted")));
          }
          // A refused batch with messages: the rows without one weren't taken either.
          if (res.ok) sent.push(...part.filter((r) => !back.has(r._row)));
        } catch (e) {
          stopped = e.message;
          break;
        }
        progress(Math.min(ok.length, i + cfg.batch));
      }
    } else sent.push(...ok);
    sending = false;
    const gone = new Set(sent.map((r) => r._row));
    const left = [...ok, ...bad].filter((r) => !gone.has(r._row)).sort((a, b) => a._row - b._row);
    if (cfg.imported && sent.length) api.set(cfg.imported, (taken = [...taken, ...sent.map(clean)]));
    api.emit("import", { rows: sent.map(clean), left: left.length });
    // What the server sent back, or didn't get, is put right and sent again.
    if (stopped || Object.keys(errors).length) {
      tableError(errors);
      api.set(cfg.bind, left);
      go("review");
      counts();
      const c = $("[data-imp-counts]");
      c.textContent = (sent.length ? L(["{n} row imported.", "{n} rows imported."], { n: sent.length }) + " " : "") + (stopped ? L("Sending stopped: {error}", { error: stopped }) : L(["{n} row was sent back.", "{n} rows were sent back."], { n: Object.keys(errors).length }));
      return;
    }
    $("[data-imp-sending]").hidden = true;
    $("[data-imp-done]").hidden = false;
    $("[data-imp-result]").textContent = L(["{n} row imported", "{n} rows imported"], { n: sent.length });
    $("[data-imp-muted]", panel("done")).textContent = left.length ? L(["{n} row was left out", "{n} rows were left out"], { n: left.length }) : "";
    $("[data-imp-again]").focus();
  }
  function template() {
    const head = cfg.fields.map((f) => f.label), sample = cfg.fields.map((f) => f.example ?? (C.choices(f)[0]?.label ?? ""));
    const rows = sample.some((s) => s !== "") ? [head, sample] : [head];
    if (/\.xlsx$/.test(cfg.template)) Z.save(Z.workbook(rows), cfg.template);
    else Z.save(new Blob(["﻿" + Z.toCsv(rows) + "\r\n"], { type: "text/csv;charset=utf-8" }), cfg.template);
  }

  // ---- events
  const zone = $("[data-imp-zone]"), input = $("input[type=file]", zone);
  input.addEventListener("change", () => {
    if (input.files[0]) read(input.files[0]);
    input.value = "";
  });
  let over = 0;
  zone.addEventListener("dragenter", (e) => (e.preventDefault(), over++, zone.toggleAttribute("data-over", true)));
  zone.addEventListener("dragleave", () => --over <= 0 && ((over = 0), zone.removeAttribute("data-over")));
  zone.addEventListener("dragover", (e) => e.preventDefault());
  zone.addEventListener("drop", (e) => {
    e.preventDefault();
    over = 0;
    zone.removeAttribute("data-over");
    if (e.dataTransfer.files[0]) read(e.dataTransfer.files[0]);
  });
  el.addEventListener("paste", (e) => {
    if (at !== "upload") return;
    const f = e.clipboardData.files[0], text = e.clipboardData.getData("text/plain");
    if (f) return e.preventDefault(), read(f);
    if (text.trim()) e.preventDefault(), (file = null), take(Z.csv(text, text.includes("\t") ? "\t" : undefined), L("Pasted cells"));
  });
  el.addEventListener("change", (e) => {
    const t = e.target;
    if (t.matches("[data-imp-field]")) {
      const field = t.getAttribute("data-imp-field"), i = t.value === "" ? null : Number(t.value);
      // A column goes to one field.
      if (i !== null) for (const k of Object.keys(map)) if (map[k] === i) map[k] = null;
      map[field] = i;
      drawMap();
      $(`[data-imp-field="${CSS.escape(field)}"]`).focus();
    } else if (t.closest("[data-imp-header]")) {
      header = t.checked;
      match();
    } else if (t.closest("[data-imp-sheets]")) read(file, Number(t.value));
  });
  // A cell the server sent back is right until it's sent again.
  table.addEventListener("celledit", (e) => {
    const { key, field } = e.detail, errors = { ...given() };
    if (!errors[key]?.[field]) return;
    const { [field]: _, ...rest } = errors[key];
    if (Object.keys(rest).length) errors[key] = rest;
    else delete errors[key];
    tableError(errors);
    api.set(cfg.bind, api.state[cfg.bind]);
  });
  el.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    if (b.matches("[data-imp-template]")) template();
    else if (b.matches("[data-imp-review]")) review();
    else if (b.matches("[data-imp-back]")) go(at === "review" ? (cells.length ? "match" : "upload") : "upload");
    else if (b.matches("[data-imp-go]")) send();
    else if (b.matches("[data-imp-again]")) {
      cells = [];
      api.set(cfg.bind, []);
      go("upload", false);
      input.focus();
    }
  });
  return { update: counts };
};
