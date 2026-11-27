// irender-ui-kit: the `ui-file-browser` behaviour. The files are a flat list in state, each
// with its folder as `parent`; this draws the open folder (a grid or a list, searched and
// sorted), the breadcrumb back up and the counts, and writes every change back: the
// folder opened, the selection, renaming, new folders, deleting (with undo) and moving
// (dragged with a mouse onto a folder or the breadcrumb, held and dragged by touch, or
// from the Move to menu). The keys: arrows move (by rows in the grid), Shift and ⌘/Ctrl
// select several, ⌘/Ctrl+A all, Enter opens, Backspace or Alt+↑ goes up, F2 renames,
// Delete deletes. With `url`, a folder's contents are fetched the first time it's opened;
// with `uploadUrl`, files dropped from the desktop are sent up into the folder they land
// on, shown as they go. Everything it writes is text, never HTML.
(window.__irenderBehaviors ||= {})["ui-file-browser"] = (el, api) => {
  const cfg = api.config, L = (t, v) => window.__irender.label(t, v);
  const $ = (s) => el.querySelector(s);
  const bar = $("[data-fm-bar]"), area = $("[data-fm-area]"), foot = $("[data-fm-foot]"), live = $("[data-fm-live]");
  const say = (msg) => live && (live.textContent = msg);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const h = (tag, attrs = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else e.setAttribute(k, v === true ? "" : v);
    }
    e.append(...kids.filter((k) => k !== null && k !== undefined && k !== false));
    return e;
  };
  const SVG = "http://www.w3.org/2000/svg";
  // A file's kind, from its name: what its icon shows and how it sorts.
  const KINDS = [
    ["image", /\.(png|jpe?g|gif|webp|svg|avif|heic)$/i], ["pdf", /\.pdf$/i], ["sheet", /\.(xlsx?|csv|ods|numbers)$/i],
    ["doc", /\.(docx?|odt|rtf|txt|md|pages)$/i], ["slides", /\.(pptx?|odp|key)$/i], ["archive", /\.(zip|rar|7z|tar|gz)$/i],
    ["video", /\.(mp4|mov|webm|mkv|avi)$/i], ["audio", /\.(mp3|wav|m4a|ogg|flac)$/i], ["code", /\.(js|ts|rs|py|json|ya?ml|html|css|sh)$/i],
  ];
  const kindOf = (f) => (f.folder ? "folder" : KINDS.find(([, re]) => re.test(f.name))?.[0] ?? "file");
  const COLORS = { folder: "#e0a526", image: "#16a34a", pdf: "#dc2626", sheet: "#15803d", doc: "#2563eb", slides: "#ea580c", archive: "#7c3aed", video: "#db2777", audio: "#0891b2", code: "#475569", file: "#71717a" };
  function icon(kind) {
    const s = document.createElementNS(SVG, "svg");
    s.setAttribute("viewBox", "0 0 48 48");
    s.setAttribute("data-fm-icon", "");
    s.setAttribute("aria-hidden", "true");
    const p = (d, fill, extra = {}) => {
      const e = document.createElementNS(SVG, "path");
      e.setAttribute("d", d);
      e.setAttribute("fill", fill);
      for (const [k, v] of Object.entries(extra)) e.setAttribute(k, v);
      s.append(e);
    };
    const c = COLORS[kind];
    if (kind === "folder") {
      p("M4 12a4 4 0 0 1 4-4h11l4 5h17a4 4 0 0 1 4 4v19a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4z", c);
      p("M4 18h40v18a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4z", c, { opacity: "0.75" });
    } else {
      p("M11 4h18l10 10v28a2 2 0 0 1-2 2H11a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z", `color-mix(in srgb, ${c} 14%, white)`, { stroke: c, "stroke-width": "2" });
      p("M29 4v8a2 2 0 0 0 2 2h8", "none", { stroke: c, "stroke-width": "2" });
      const tag = { image: "IMG", pdf: "PDF", sheet: "XLS", doc: "DOC", slides: "PPT", archive: "ZIP", video: "VID", audio: "AUD", code: "</>", file: "" }[kind];
      if (tag) {
        const t = document.createElementNS(SVG, "text");
        Object.entries({ x: "24", y: "34", "text-anchor": "middle", "font-size": "9", "font-weight": "700", "font-family": "system-ui, sans-serif", fill: c }).forEach(([k, v]) => t.setAttribute(k, v));
        t.textContent = tag;
        s.append(t);
      }
    }
    return s;
  }

  // ---- state
  const files = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind].filter((f) => isObj(f) && f.id != null) : []);
  const key = (v) => (v === null || v === undefined || v === "" ? null : String(v));
  const byId = () => new Map(files().map((f) => [String(f.id), f]));
  let localFolder = null;
  const folder = () => {
    const v = key(cfg.bindFolder ? api.state[cfg.bindFolder] : localFolder);
    return v !== null && byId().get(v)?.folder ? v : null;
  };
  let selected = new Set(), focus = null, anchor = null, view = cfg.view ?? "grid", sortBy = "name", query = "", renaming = null, undo = null;
  // ---- folders from a server: each fetched once, the first time it's opened
  const loaded = new Set(), loading = new Set(), failed = new Map();
  function fetchFolder(id) {
    const k = id ?? "";
    if (!cfg.url || loaded.has(k) || loading.has(k)) return;
    loading.add(k);
    failed.delete(k);
    const u = new URL(cfg.url, location.href);
    if (id !== null) u.searchParams.set("folder", id);
    fetch(u, { headers: { accept: "application/json" } })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((got) => {
        const list = (Array.isArray(got) ? got : Array.isArray(got?.items) ? got.items : []).filter((f) => isObj(f) && f.id != null);
        // What came is this folder's: its own copies of anything already there replace them.
        const ids = new Set(list.map((f) => String(f.id)));
        const next = [...files().filter((f) => !ids.has(String(f.id))), ...list.map((f) => ({ ...f, parent: f.parent ?? id }))];
        loaded.add(k);
        loading.delete(k);
        api.set(cfg.bind, next);
        say(L(["{n} item loaded", "{n} items loaded"], { n: list.length }));
      })
      .catch((e) => {
        loading.delete(k);
        failed.set(k, String(e.message ?? e));
        draw();
      });
    draw();
  }
  function setFolder(id) {
    const v = key(id);
    fetchFolder(v);
    if (cfg.bindFolder) api.set(cfg.bindFolder, v);
    else localFolder = v;
    selected = new Set();
    focus = null;
    anchor = null;
    query = "";
    publishSelection();
    draw();
    say(L(["{name}, {n} item", "{name}, {n} items"], { name: v === null ? cfg.rootLabel : byId().get(v)?.name ?? "", n: shown().length }));
    el.querySelector("[data-fm-files]")?.focus();
  }
  function write(next, msg) {
    api.set(cfg.bind, next);
    api.emit("change", { value: next });
    if (msg) say(msg);
  }
  const publishSelection = () => cfg.bindSelection && api.set(cfg.bindSelection, [...selected]);
  const children = (id) => files().filter((f) => key(f.parent) === id);
  // Every folder from the top to `id`.
  function trail(id) {
    const map = byId(), out = [];
    for (let at = id, guard = 0; at !== null && guard < 200; guard++) {
      const f = map.get(at);
      if (!f) break;
      out.unshift(f);
      at = key(f.parent);
    }
    return out;
  }
  // `id` and everything inside it.
  function within(id) {
    const out = new Set([id]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const f of files()) if (!out.has(String(f.id)) && out.has(key(f.parent))) out.add(String(f.id)), (grew = true);
    }
    return out;
  }
  // A name no other item in `parent` has: "New folder", "New folder (2)", …
  function free(name, parent, except) {
    const taken = new Set(children(parent).filter((f) => String(f.id) !== except).map((f) => f.name.toLowerCase()));
    if (!taken.has(name.toLowerCase())) return name;
    const dot = name.lastIndexOf("."), [stem, ext] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ""];
    for (let n = 2; ; n++) if (!taken.has(`${stem} (${n})${ext}`.toLowerCase())) return `${stem} (${n})${ext}`;
  }

  // ---- what the open folder shows
  const loc = cfg.locale || document.documentElement.lang || undefined;
  const dateFmt = new Intl.DateTimeFormat(loc, { dateStyle: "medium", timeStyle: "short" });
  const date = (v) => {
    const t = v ? Date.parse(v) : NaN;
    return Number.isFinite(t) ? dateFmt.format(t) : "";
  };
  function size(n) {
    if (n === null || n === undefined || n === "" || !Number.isFinite(Number(n))) return "";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let v = Number(n), i = 0;
    while (v >= 1000 && i < units.length - 1) (v /= 1000), i++;
    return `${new Intl.NumberFormat(loc, { maximumFractionDigits: v < 10 && i ? 1 : 0 }).format(v)} ${units[i]}`;
  }
  const collator = new Intl.Collator(loc, { numeric: true, sensitivity: "base" });
  function shown() {
    const q = query.trim().toLowerCase();
    const list = children(folder()).filter((f) => !q || f.name.toLowerCase().includes(q));
    const cmp = {
      name: (a, b) => collator.compare(a.name, b.name),
      modified: (a, b) => (Date.parse(b.modified) || 0) - (Date.parse(a.modified) || 0) || collator.compare(a.name, b.name),
      size: (a, b) => (Number(b.size) || 0) - (Number(a.size) || 0) || collator.compare(a.name, b.name),
      kind: (a, b) => collator.compare(kindOf(a), kindOf(b)) || collator.compare(a.name, b.name),
    }[sortBy];
    // Folders first, as most file browsers have them.
    return list.sort((a, b) => (b.folder ? 1 : 0) - (a.folder ? 1 : 0) || cmp(a, b));
  }

  // ---- drawing
  function draw() {
    const here = folder(), items = shown();
    const ids = new Set(items.map((f) => String(f.id)));
    selected = new Set([...selected].filter((id) => ids.has(id)));
    if (focus !== null && !ids.has(focus)) focus = null;
    drawBar(here);
    const box = h("div", {
      "data-fm-files": true, "data-view": view, role: "listbox", tabindex: "0", "aria-multiselectable": "true",
      "aria-label": here === null ? cfg.rootLabel : byId().get(here)?.name, "aria-activedescendant": focus !== null ? itemId(focus) : null,
    });
    // Labels: "Loading…", "Couldn't load this folder", "Try again"
    const k = here ?? "";
    if (loading.has(k) && !items.length) box.append(h("p", { "data-fm-empty": true, role: "status" }, L("Loading…")));
    else if (failed.has(k)) box.append(h("p", { "data-fm-empty": true, role: "alert" }, L("Couldn't load this folder"), " ", h("button", { type: "button", "data-fm-retry": true, onclick: () => fetchFolder(here) }, L("Try again"))));
    else if (!items.length) box.append(h("p", { "data-fm-empty": true }, query ? L("Nothing here matches “{q}”", { q: query }) : L("This folder is empty")));
    for (const f of items) box.append(drawItem(f));
    const head = view === "list" && items.length
      ? h("div", { "data-fm-head": true, "aria-hidden": "true" }, h("span"), h("span", {}, L("Name")), h("span", { "data-fm-date": true }, L("Modified")), h("span", {}, L("Size")))
      : null;
    const hadFocus = area.contains(document.activeElement) && !document.activeElement.matches("[data-fm-rename]");
    area.replaceChildren(...[head, box].filter(Boolean));
    if (hadFocus) box.focus({ preventScroll: true });
    if (renaming !== null) startRename(renaming);
    drawFoot(items);
    seen = JSON.stringify([files(), folder()]);
  }
  const itemId = (id) => `${cfg.id}-item-${String(id).replace(/[^\w-]/g, "_")}`;
  function drawItem(f) {
    const id = String(f.id), kind = kindOf(f);
    const pic = f.thumb && !f.folder ? h("img", { src: f.thumb, alt: "", "data-fm-thumb": true, loading: "lazy", draggable: "false" }) : icon(kind);
    // Labels: "Uploading…", "Upload failed"
    const up = f.uploading ? L("Uploading…") : f.uploadError ? L("Upload failed") : null;
    return h("div", {
      id: itemId(id), role: "option", "data-fm-item": id, "data-folder": f.folder || null, "data-fm-up": f.uploading ? "going" : f.uploadError ? "failed" : null,
      "aria-selected": selected.has(id) ? "true" : "false", "data-focus": focus === id || null,
      draggable: cfg.editable ? "true" : null,
      "aria-description": f.folder ? L("Folder") : [size(f.size), date(f.modified)].filter(Boolean).join(", ") || null,
    }, pic, h("span", { "data-fm-name": true }, f.name), h("span", { "data-fm-meta": true, "data-fm-date": true }, date(f.modified)), h("span", { "data-fm-meta": true, "data-fm-size": true }, f.folder ? L(["{n} item", "{n} items"], { n: children(id).length }) : up ?? size(f.size)));
  }
  function drawBar(here) {
    const crumbs = h("ol", { "data-fm-crumbs": true, "aria-label": L("Folders") });
    const path = [{ id: null, name: cfg.rootLabel }, ...trail(here).map((f) => ({ id: String(f.id), name: f.name }))];
    path.forEach((c, i) => crumbs.append(h("li", { "data-fm-crumb": c.id ?? "" }, h("button", { type: "button", "aria-current": i === path.length - 1 ? "location" : null, onclick: () => setFolder(c.id) }, c.name))));
    const search = h("input", { type: "search", placeholder: L("Search this folder"), "aria-label": L("Search this folder"), value: query });
    search.addEventListener("input", () => {
      query = search.value;
      draw();
      bar.querySelector("input[type=search]").focus();
      const field = bar.querySelector("input[type=search]");
      field.setSelectionRange(field.value.length, field.value.length);
    });
    const sort = h("select", { "aria-label": L("Sort by") },
      ...[["name", L("Name")], ["modified", L("Newest")], ["size", L("Largest")], ["kind", L("Kind")]].map(([v, t]) => h("option", { value: v, selected: sortBy === v }, t)));
    sort.addEventListener("change", () => {
      sortBy = sort.value;
      draw();
      bar.querySelector("select").focus();
    });
    const views = [["grid", L("Grid")], ["list", L("List")]].map(([v, t]) => h("button", { type: "button", "aria-pressed": view === v ? "true" : "false", onclick: () => { view = v; draw(); bar.querySelector(`[data-fm-view=${v}]`)?.focus(); }, "data-fm-view": v }, t));
    const tools = cfg.editable ? [
      h("button", { type: "button", "data-fm-new": true, onclick: newFolder }, L("New folder")),
      h("button", { type: "button", "data-fm-move": true, disabled: selected.size ? null : true, "aria-haspopup": "menu", onclick: (e) => openMenu(e.currentTarget) }, L("Move to…")),
      h("button", { type: "button", "data-fm-delete": true, disabled: selected.size ? null : true, onclick: () => remove([...selected]) }, L("Delete")),
    ] : [];
    bar.replaceChildren(crumbs, search, sort, ...views, ...tools);
  }
  function drawFoot(items) {
    const count = h("span", {}, selected.size ? L("{n} of {total} selected", { n: selected.size, total: items.length }) : L(["{n} item", "{n} items"], { n: items.length }));
    const undoBar = h("span", { "data-fm-undo": true, hidden: undo ? null : true }, undo ? undo.text : "", undo ? h("button", { type: "button", onclick: doUndo }, L("Undo")) : null);
    foot.replaceChildren(count, undoBar);
  }

  // ---- selecting and opening
  function pick(id, { add = false, range = false } = {}) {
    const items = shown().map((f) => String(f.id));
    if (range && anchor !== null && items.includes(anchor)) {
      const [a, b] = [items.indexOf(anchor), items.indexOf(id)].sort((x, y) => x - y);
      if (!add) selected = new Set();
      for (const x of items.slice(a, b + 1)) selected.add(x);
    } else if (add) {
      selected.has(id) ? selected.delete(id) : selected.add(id);
      anchor = id;
    } else {
      selected = new Set([id]);
      anchor = id;
    }
    focus = id;
    publishSelection();
    draw();
  }
  function open(id) {
    const f = byId().get(id);
    if (!f) return;
    if (f.folder) return setFolder(id);
    if (cfg.bindOpen) api.set(cfg.bindOpen, f);
    api.emit("open", { file: f });
    say(L("Opened {name}", { name: f.name }));
  }
  function up() {
    const here = folder();
    if (here === null) return;
    const parent = key(byId().get(here)?.parent);
    const from = here;
    setFolder(parent);
    // Back where it came from: the folder just left is focused.
    focus = from;
    selected = new Set([from]);
    anchor = from;
    publishSelection();
    draw();
  }

  // ---- editing
  function newFolder() {
    const here = folder();
    const id = `folder-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    const f = { id, name: free(L("New folder"), here), folder: true, parent: here, modified: new Date().toISOString() };
    write([...files(), f], L("{name} added", { name: f.name }));
    selected = new Set([id]);
    focus = anchor = id;
    renaming = id;
    publishSelection();
    draw();
  }
  function startRename(id) {
    const node = area.querySelector(`[data-fm-item="${CSS.escape(id)}"] [data-fm-name]`), f = byId().get(id);
    if (!node || !f) return (renaming = null);
    const input = h("input", { type: "text", "data-fm-rename": true, value: f.name, "aria-label": L("Rename {name}", { name: f.name }), spellcheck: "false" });
    node.replaceWith(input);
    input.focus();
    // The name without its extension, ready to type over.
    const dot = f.folder ? -1 : f.name.lastIndexOf(".");
    input.setSelectionRange(0, dot > 0 ? dot : f.name.length);
    let done = false;
    const finish = (keep) => {
      if (done) return;
      done = true;
      renaming = null;
      const name = input.value.trim().replace(/[\\/]/g, "-");
      if (keep && name && name !== f.name) {
        const clash = children(key(f.parent)).some((x) => String(x.id) !== id && x.name.toLowerCase() === name.toLowerCase());
        if (clash) {
          say(L("There's already a {name} here", { name }));
          renaming = id;
          done = false;
          input.setAttribute("aria-invalid", "true");
          input.focus();
          return;
        }
        write(files().map((x) => (String(x.id) === id ? { ...x, name } : x)), L("Renamed to {name}", { name }));
      }
      draw();
      area.querySelector("[data-fm-files]")?.focus();
    };
    input.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Enter") finish(true);
      else if (e.key === "Escape") finish(false);
    });
    input.addEventListener("blur", () => finish(true));
  }
  function remove(ids) {
    if (!cfg.editable || !ids.length) return;
    const gone = new Set(ids.flatMap((id) => [...within(id)]));
    const before = files();
    const names = ids.map((id) => byId().get(id)?.name).filter(Boolean);
    clearTimeout(undo?.timer);
    undo = { before, text: names.length === 1 ? L("{name} deleted", { name: names[0] }) : L("{n} items deleted", { n: names.length }), timer: setTimeout(() => ((undo = null), draw()), 10000) };
    selected = new Set();
    write(before.filter((f) => !gone.has(String(f.id))), `${undo.text}. ${L("Undo is at the foot.")}`);
    publishSelection();
    draw();
  }
  function doUndo() {
    if (!undo) return;
    clearTimeout(undo.timer);
    const before = undo.before;
    undo = null;
    write(before, L("Undone"));
    draw();
  }
  // Moves `ids` into `target` (null: the top). A folder can't go into itself.
  function move(ids, target) {
    if (!cfg.editable) return;
    const map = byId(), ok = ids.filter((id) => map.has(id) && key(map.get(id).parent) !== target && !(target !== null && within(id).has(target)));
    if (!ok.length) return;
    let list = files();
    for (const id of ok) list = list.map((x) => (String(x.id) === id ? { ...x, parent: target, name: free(x.name, target, id) } : x));
    const where = target === null ? cfg.rootLabel : map.get(target)?.name;
    selected = new Set();
    write(list, ok.length === 1 ? L("{name} moved to {folder}", { name: map.get(ok[0]).name, folder: where }) : L("{n} items moved to {folder}", { n: ok.length, folder: where }));
    publishSelection();
    draw();
  }

  // ---- the Move to menu: every folder the selection can go into
  let menu = null;
  function openMenu(button) {
    menu?.remove();
    const ids = [...selected], blocked = new Set(ids.flatMap((id) => [...within(id)]));
    const folders = files().filter((f) => f.folder && !blocked.has(String(f.id)));
    const depth = (f) => trail(String(f.id)).length - 1;
    const order = (parent) => folders.filter((f) => key(f.parent) === parent).sort((a, b) => collator.compare(a.name, b.name)).flatMap((f) => [f, ...order(String(f.id))]);
    menu = h("div", { popover: "auto", role: "menu", "data-fm-menu": true, "aria-label": L("Move to") },
      h("button", { type: "button", role: "menuitem", onclick: () => { menu.hidePopover(); move(ids, null); } }, cfg.rootLabel),
      ...order(null).map((f) => h("button", { type: "button", role: "menuitem", style: null, "data-depth": depth(f), onclick: () => { menu.hidePopover(); move(ids, String(f.id)); } }, f.name)));
    for (const b of menu.querySelectorAll("button")) b.style.paddingInlineStart = `${10 + 14 * Number(b.dataset.depth ?? -1) + 14}px`;
    el.append(menu);
    const r = button.getBoundingClientRect();
    menu.showPopover();
    Object.assign(menu.style, { position: "fixed", left: `${Math.min(r.left, innerWidth - menu.offsetWidth - 8)}px`, top: `${r.bottom + 4}px` });
    menu.querySelector("button")?.focus();
    menu.addEventListener("keydown", (e) => {
      const items = [...menu.querySelectorAll("button")], i = items.indexOf(document.activeElement);
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        items[(i + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length].focus();
      }
    });
    menu.addEventListener("toggle", (e) => {
      if (e.newState === "closed") {
        menu.remove();
        menu = null;
        bar.querySelector("[data-fm-move]")?.focus();
      }
    });
  }

  // ---- the mouse and the keys
  area.addEventListener("click", (e) => {
    const item = e.target.closest?.("[data-fm-item]");
    if (e.target.matches?.("[data-fm-rename]")) return;
    if (!item) {
      if (e.target.closest("[data-fm-files]") && !e.shiftKey && !e.metaKey && !e.ctrlKey) {
        selected = new Set();
        publishSelection();
        draw();
      }
      return;
    }
    pick(item.getAttribute("data-fm-item"), { add: e.metaKey || e.ctrlKey, range: e.shiftKey });
  });
  area.addEventListener("dblclick", (e) => {
    const item = e.target.closest?.("[data-fm-item]");
    if (item) open(item.getAttribute("data-fm-item"));
  });
  // How many items a row of the grid holds.
  function perRow() {
    const items = [...area.querySelectorAll("[data-fm-item]")];
    if (view !== "grid" || items.length < 2) return 1;
    const top = items[0].offsetTop;
    const n = items.findIndex((x) => x.offsetTop !== top);
    return n < 0 ? items.length : n;
  }
  area.addEventListener("keydown", (e) => {
    if (!e.target.matches?.("[data-fm-files]")) return;
    const items = shown().map((f) => String(f.id)), i = focus === null ? -1 : items.indexOf(focus), row = perRow();
    const step = { ArrowRight: view === "grid" ? 1 : 0, ArrowLeft: view === "grid" ? -1 : 0, ArrowDown: row, ArrowUp: -row, Home: -Infinity, End: Infinity }[e.key];
    const mod = e.metaKey || e.ctrlKey;
    if (e.key === "ArrowUp" && e.altKey) {
      e.preventDefault();
      return up();
    }
    if (step !== undefined && step !== 0 && items.length) {
      e.preventDefault();
      const next = items[Math.max(0, Math.min(items.length - 1, i < 0 ? 0 : i + (Number.isFinite(step) ? step : step > 0 ? items.length : -items.length)))];
      if (e.shiftKey) pick(next, { range: true, add: mod });
      else if (mod) {
        focus = next;
        draw();
      } else pick(next);
      area.querySelector(`[data-fm-item="${CSS.escape(next)}"]`)?.scrollIntoView({ block: "nearest" });
      return;
    }
    if (e.key === " " && focus !== null) {
      e.preventDefault();
      return pick(focus, { add: true });
    }
    if (e.key === "a" && mod) {
      e.preventDefault();
      selected = new Set(items);
      publishSelection();
      say(L("{n} selected", { n: items.length }));
      return draw();
    }
    if (e.key === "Enter" && focus !== null) {
      e.preventDefault();
      return open(focus);
    }
    if (e.key === "Backspace") {
      e.preventDefault();
      return up();
    }
    if (e.key === "F2" && focus !== null && cfg.editable) {
      e.preventDefault();
      renaming = focus;
      return startRename(focus);
    }
    if ((e.key === "Delete" || (e.key === "Backspace" && mod)) && selected.size) {
      e.preventDefault();
      return remove([...selected]);
    }
    if (e.key === "Escape" && selected.size) {
      selected = new Set();
      publishSelection();
      draw();
    }
  });
  area.addEventListener("focusin", (e) => {
    if (e.target.matches?.("[data-fm-files]") && focus === null) {
      const first = shown()[0];
      if (first) {
        focus = String(first.id);
        e.target.setAttribute("aria-activedescendant", itemId(focus));
        area.querySelector(`[data-fm-item="${CSS.escape(focus)}"]`)?.setAttribute("data-focus", "");
      }
    }
  });

  // ---- dragging onto a folder or the breadcrumb, with a mouse
  let dragging = null;
  const targetOf = (node) => {
    const crumb = node?.closest?.("[data-fm-crumb]");
    if (crumb) return { el: crumb, id: key(crumb.getAttribute("data-fm-crumb")) };
    const item = node?.closest?.("[data-fm-item][data-folder]");
    return item ? { el: item, id: item.getAttribute("data-fm-item") } : null;
  };
  const unmark = () => el.querySelectorAll("[data-drop]").forEach((x) => x.removeAttribute("data-drop"));
  const allowed = (t) => t && dragging && !dragging.some((id) => within(id).has(t.id ?? "")) && !(t.id === null && dragging.every((id) => key(byId().get(id)?.parent) === null));
  el.addEventListener("dragstart", (e) => {
    const item = e.target.closest?.("[data-fm-item]");
    if (!item || !cfg.editable) return;
    const id = item.getAttribute("data-fm-item");
    if (!selected.has(id)) pick(id);
    dragging = [...selected];
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", dragging.map((x) => byId().get(x)?.name).join("\n"));
    requestAnimationFrame(() => dragging?.forEach((x) => area.querySelector(`[data-fm-item="${CSS.escape(x)}"]`)?.setAttribute("data-cut", "")));
  });
  // ---- files dropped from the desktop, with `uploadUrl`
  const fromDesktop = (e) => !dragging && !!cfg.uploadUrl && [...(e.dataTransfer?.types ?? [])].includes("Files");
  let ups = 0;
  function upload(list, parent) {
    const items = files();
    const added = [...list].map((file) => ({ file, item: { id: `upload-${Date.now()}-${++ups}`, name: free(file.name, parent), parent, size: file.size, modified: new Date().toISOString(), uploading: true } }));
    write([...items, ...added.map((a) => a.item)], L(["Uploading {n} file", "Uploading {n} files"], { n: added.length }));
    for (const { file, item } of added) {
      const body = new FormData();
      body.append("file", file, item.name);
      if (parent !== null) body.append("folder", parent);
      fetch(cfg.uploadUrl, { method: "POST", body, headers: { accept: "application/json" } })
        .then((r) => (r.ok ? r.json().catch(() => null) : Promise.reject(new Error(`HTTP ${r.status}`))))
        .then((got) => {
          // An item back takes the file's place; otherwise it's kept as it is, uploaded.
          const done = isObj(got) && got.id != null ? { ...got, parent: got.parent ?? parent } : (({ uploading, ...rest }) => rest)(item);
          write(files().map((f) => (String(f.id) === item.id ? done : f)), L("{name} uploaded", { name: done.name ?? item.name }));
          api.emit("upload", { file: done });
        })
        .catch(() => write(files().map((f) => (String(f.id) === item.id ? { ...(({ uploading, ...rest }) => rest)(f), uploadError: true } : f)), L("{name} didn't upload", { name: item.name })));
    }
  }
  el.addEventListener("dragover", (e) => {
    if (fromDesktop(e)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      unmark();
      // Onto a folder, or the open one.
      const t = targetOf(e.target);
      (t?.el && t.id !== null && byId().get(t.id)?.folder ? t.el : area).setAttribute("data-drop", "");
      return;
    }
    const t = targetOf(e.target);
    if (!allowed(t)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    unmark();
    t.el.setAttribute("data-drop", "");
  });
  el.addEventListener("dragleave", (e) => {
    if (!el.contains(e.relatedTarget)) unmark();
  });
  el.addEventListener("drop", (e) => {
    if (fromDesktop(e)) {
      e.preventDefault();
      unmark();
      const t = targetOf(e.target), into = t && t.id !== null && byId().get(t.id)?.folder ? t.id : folder();
      if (e.dataTransfer.files?.length) upload(e.dataTransfer.files, into);
      return;
    }
    const t = targetOf(e.target);
    if (!allowed(t)) return;
    e.preventDefault();
    const ids = dragging;
    dragging = null;
    unmark();
    move(ids, t.id);
  });
  el.addEventListener("dragend", () => {
    dragging = null;
    unmark();
    el.querySelectorAll("[data-cut]").forEach((x) => x.removeAttribute("data-cut"));
  });
  // …and by touch: a press held still picks the item up.
  if (cfg.editable && window.__irenderTouchDrag) {
    window.__irenderTouchDrag(area, {
      pick: (node) => node.closest?.("[data-fm-item]"),
      start: (item) => {
        const id = item.getAttribute("data-fm-item");
        if (!selected.has(id)) pick(id);
        dragging = [...selected];
      },
      over: (x, y, under) => {
        unmark();
        const t = targetOf(under);
        if (allowed(t)) t.el.setAttribute("data-drop", "");
      },
      drop: (x, y, under) => {
        const t = targetOf(under);
        if (allowed(t)) move(dragging, t.id);
      },
      end: () => {
        dragging = null;
        unmark();
      },
      scroller: area,
    });
  }

  let seen = null;
  fetchFolder(folder());
  draw();
  return {
    update() {
      // Only when the files or the open folder changed since the last draw (a page's own
      // upload, say), not for what this wrote itself, nor while a name is being typed.
      if (renaming !== null || JSON.stringify([files(), folder()]) === seen) return;
      // A folder opened from state is fetched too.
      fetchFolder(folder());
      draw();
    },
  };
};
