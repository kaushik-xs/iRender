// irender-ui-kit: the `ui-file-upload` behaviour. The files live in state under `bind`:
//   { id, name, size, type, url, status, progress, error }
// status: "uploading", "done", "error", or "local" (kept in the browser: no `url` prop).
// The File objects themselves can't go in state, so they stay here, by id, for previews,
// retries and the zip. Previews and the zip need no libraries: images, PDFs and text
// are shown by the browser, and sheets.js reads CSV and .xlsx and writes the zip.
(window.__irenderBehaviors ||= {})["ui-file-upload"] = (el, api) => {
  const cfg = api.config, Z = window.__irenderSheets;
  const $ = (s, from = el) => from.querySelector(s);
  const input = $("[data-up-input]");
  const zone = $("[data-up-zone]");
  const list = $("[data-up-files]");
  const dialog = $("[data-up-preview]");
  const live = $("[data-up-live]");
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const resolve = (v) => {
    if (Array.isArray(v)) return v.map(resolve);
    if (!isObj(v)) return v;
    if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, api.state);
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
  };

  // ---- the files
  const files = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind] : []);
  const local = new Map(); // id → File
  const blobs = new Map(); // id → object URL
  const sending = new Map(); // id → XMLHttpRequest
  const write = (next) => api.set(cfg.bind, next);
  const patch = (id, change) => write(files().map((f) => (String(f.id) === id ? { ...f, ...change } : f)));
  const newId = () => globalThis.crypto?.randomUUID?.() ?? `f${Date.now().toString(36)}${Math.random().toString(36).slice(2, 7)}`;
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };

  // ---- kinds, sizes and names
  const ext = (name) => (/\.([a-z0-9]{1,6})$/i.exec(name ?? "")?.[1] ?? "").toLowerCase();
  function kind(f) {
    const e = ext(f.name), t = f.type ?? "";
    if (t.startsWith("image/") || ["png", "jpg", "jpeg", "gif", "webp", "svg", "avif", "bmp"].includes(e)) return "image";
    if (t === "application/pdf" || e === "pdf") return "pdf";
    if (["xlsx", "xls", "csv", "tsv", "ods"].includes(e) || /spreadsheet|excel|csv/.test(t)) return "sheet";
    if (["doc", "docx", "odt", "rtf", "ppt", "pptx", "key"].includes(e) || /word|presentation/.test(t)) return "doc";
    if (["zip", "rar", "7z", "gz", "tar"].includes(e) || /zip|compressed/.test(t)) return "archive";
    if (["txt", "md", "json", "log", "xml", "yaml", "yml", "html", "css", "js"].includes(e) || t.startsWith("text/")) return "text";
    return "other";
  }
  const size = (n) => {
    if (!(n >= 0)) return "";
    const units = ["B", "KB", "MB", "GB"];
    let i = 0;
    while (n >= 1024 && i < units.length - 1) (n /= 1024), i++;
    return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: i && n < 10 ? 1 : 0 }).format(n)} ${units[i]}`;
  };
  const href = (f) => blobs.get(String(f.id)) ?? f.url ?? null;

  // ---- checking files against `accept`, `maxSize` and `maxFiles`
  function accepted(file) {
    if (!cfg.accept) return true;
    const name = file.name.toLowerCase(), type = (file.type || "").toLowerCase();
    return cfg.accept.split(",").map((a) => a.trim().toLowerCase()).filter(Boolean).some((a) =>
      a.startsWith(".") ? name.endsWith(a) : a.endsWith("/*") ? type.startsWith(a.slice(0, -1)) : type === a);
  }
  function add(picked) {
    const problems = [];
    const room = cfg.maxFiles ? cfg.maxFiles - files().length : Infinity;
    let chosen = [...picked];
    if (!cfg.multiple) chosen = chosen.slice(0, 1);
    const fresh = [];
    for (const file of chosen) {
      if (!accepted(file)) problems.push(L("{name} isn't a type that can be added.", { name: file.name }));
      else if (cfg.maxSize && file.size > cfg.maxSize) problems.push(L("{name} is over the {size} limit.", { name: file.name, size: size(cfg.maxSize) }));
      else if (fresh.length >= room) problems.push(L("{name} wasn't added: at most {max} files.", { name: file.name, max: cfg.maxFiles }));
      else fresh.push(file);
    }
    const errors = $("[data-up-errors]");
    errors.replaceChildren(...problems.map((p) => Object.assign(document.createElement("li"), { textContent: p })));
    errors.hidden = !problems.length;
    if (!fresh.length) return;
    const url = resolve(cfg.url);
    const entries = fresh.map((file) => {
      const id = newId();
      local.set(id, file);
      blobs.set(id, URL.createObjectURL(file));
      return { id, name: file.name, size: file.size, type: file.type || "", status: url ? "uploading" : "local", progress: 0 };
    });
    // One file in place of the other when only one is allowed.
    const kept = cfg.multiple ? files() : [];
    if (!cfg.multiple) files().forEach((f) => forget(f.id));
    write([...kept, ...entries]);
    say(entries.length === 1 ? L("{name} added", { name: entries[0].name }) : L(["{n} file added", "{n} files added"], { n: entries.length }));
    api.emit("add", { files: entries });
    if (url) entries.forEach((e) => send(e.id));
  }

  // ---- uploading, three at a time
  const queue = [];
  function send(id) {
    queue.push(id);
    pump();
  }
  function pump() {
    while (sending.size < 3 && queue.length) start(queue.shift());
  }
  function start(id) {
    const file = local.get(id), url = resolve(cfg.url);
    if (!file || !url) return;
    const form = new FormData();
    for (const [k, v] of Object.entries(resolve(cfg.params) ?? {})) if (v !== null && v !== undefined) form.append(k, typeof v === "object" ? JSON.stringify(v) : String(v));
    form.append("file", file, file.name);
    const xhr = new XMLHttpRequest();
    sending.set(id, xhr);
    let shown = 0;
    patch(id, { status: "uploading", progress: 0, error: null });
    xhr.upload.onprogress = (e) => {
      if (!e.lengthComputable) return;
      const p = Math.round((e.loaded / e.total) * 100);
      // Every few percent is enough to draw.
      if (p - shown >= 5 || p === 100) (shown = p), patch(id, { progress: p });
    };
    const done = (change, event) => {
      sending.delete(id);
      if (files().some((f) => String(f.id) === id)) {
        // The server's id, when it sends one, is the one to keep (a host deletes by it).
        const now = change.id != null && String(change.id) !== id ? String(change.id) : id;
        if (now !== id) {
          for (const m of [local, blobs]) if (m.has(id)) m.set(now, m.get(id)), m.delete(id);
        }
        patch(id, change);
        const f = files().find((x) => String(x.id) === now);
        api.emit(event, { file: f });
        say(event === "uploaded" ? L("{name} uploaded", { name: f.name }) : L("{name} failed: {error}", { name: f.name, error: f.error }));
      }
      pump();
    };
    xhr.onload = () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        let answer = {};
        try {
          answer = JSON.parse(xhr.responseText);
        } catch {}
        done({ ...(isObj(answer) ? answer : {}), status: "done", progress: 100, error: null }, "uploaded");
      } else {
        let why = `${xhr.status} ${xhr.statusText}`.trim();
        try {
          const a = JSON.parse(xhr.responseText);
          why = a.error || a.message || why;
        } catch {}
        done({ status: "error", error: why || L("Upload failed") }, "error");
      }
    };
    xhr.onerror = () => done({ status: "error", error: L("Couldn't reach the server") }, "error");
    xhr.open("POST", url);
    xhr.setRequestHeader("accept", "application/json");
    xhr.send(form);
  }
  function forget(id) {
    sending.get(id)?.abort();
    sending.delete(id);
    const q = queue.indexOf(id);
    if (q >= 0) queue.splice(q, 1);
    local.delete(id);
    if (blobs.has(id)) URL.revokeObjectURL(blobs.get(id));
    blobs.delete(id);
  }
  function remove(id) {
    const f = files().find((x) => String(x.id) === id);
    if (!f) return;
    const shown = [...list.querySelectorAll("[data-up-file]")].map((x) => x.getAttribute("data-up-file"));
    const at = shown.indexOf(id);
    forget(id);
    write(files().filter((x) => String(x.id) !== id));
    api.emit("remove", { file: f });
    say(L("{name} removed", { name: f.name }));
    // Focus to the next file, the one before, or the drop zone.
    const next = shown[at + 1] ?? shown[at - 1];
    queueMicrotask(() => (next ? list.querySelector(`[data-up-file="${CSS.escape(next)}"] [data-up-open]`)?.focus() : input?.focus()));
  }

  // ---- drawing
  const svgs = {
    remove: '<svg width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8"/></svg>',
    retry: '<svg width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5v3h-3"/></svg>',
    file: '<svg width="22" height="22" style="width:calc(22px * var(--size-icon, 1));height:calc(22px * var(--size-icon, 1))" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" aria-hidden="true"><path d="M6 2.5h8l4.5 4.5v14.5H6z"/><path d="M14 2.5V7h4.5"/></svg>',
  };
  const make = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    if (text !== undefined) e.textContent = text;
    return e;
  };
  function card(f) {
    const k = kind(f), li = make("li", { "data-up-file": f.id, "data-kind": k, "data-status": f.status ?? "done" });
    const open = make("button", { type: "button", "data-up-open": "", "aria-label": L("Preview {name}", { name: f.name }) });
    const thumb = make("span", { "data-up-thumb": "" });
    const src = href(f);
    if (k === "image" && src) thumb.append(make("img", { src, alt: "", loading: "lazy" }));
    else {
      const badge = make("span", { "data-ext": "" });
      badge.innerHTML = svgs.file;
      badge.append(make("b", {}, (ext(f.name) || k).toUpperCase().slice(0, 4)));
      thumb.append(badge);
    }
    const meta = make("span", { "data-up-meta": "" });
    meta.append(make("span", { "data-up-name": "", title: f.name }, f.name));
    if (f.status === "uploading") {
      const p = make("progress", { max: 100, value: f.progress ?? 0, "aria-label": L("Uploading {name}", { name: f.name }) });
      meta.append(make("span", { "data-up-size": "" }, L("Uploading… {progress}%", { progress: String(f.progress ?? 0) })), p);
    } else if (f.status === "error") meta.append(make("span", { "data-up-error": "" }, f.error || L("Upload failed")));
    else meta.append(make("span", { "data-up-size": "" }, size(f.size)));
    open.append(thumb, meta);
    li.append(open);
    const actions = make("span", { "data-up-actions": "" });
    if (f.status === "error" && local.has(String(f.id))) {
      const b = make("button", { type: "button", "data-up-retry": "", "aria-label": L("Try {name} again", { name: f.name }), title: L("Try again") });
      b.innerHTML = svgs.retry;
      actions.append(b);
    }
    if (cfg.removable) {
      const b = make("button", { type: "button", "data-up-remove": "", "aria-label": L("Remove {name}", { name: f.name }), title: L("Remove") });
      b.innerHTML = svgs.remove;
      actions.append(b);
    }
    if (actions.children.length) li.append(actions);
    return li;
  }
  function draw() {
    const all = files();
    const focused = document.activeElement?.closest?.("[data-up-file]")?.getAttribute("data-up-file");
    const which = document.activeElement?.matches?.("[data-up-remove]") ? "[data-up-remove]" : "[data-up-open]";
    list.replaceChildren(...all.map(card));
    list.hidden = !all.length;
    const empty = $("[data-up-empty]");
    if (empty) empty.hidden = !!all.length;
    const bar = $("[data-up-bar]");
    bar.hidden = !all.length;
    const bytes = all.reduce((s, f) => s + (Number(f.size) || 0), 0);
    $("[data-up-total]").textContent = bytes ? L(["{n} file, {size}", "{n} files, {size}"], { n: all.length, size: size(bytes) }) : L(["{n} file", "{n} files"], { n: all.length });
    const zip = $("[data-up-zip]");
    if (zip) zip.disabled = !all.some(href) || all.some((f) => f.status === "uploading");
    if (input) {
      const full = cfg.maxFiles && all.length >= cfg.maxFiles;
      input.toggleAttribute("data-full", !!full);
      zone.toggleAttribute("aria-disabled", !!full);
    }
    if (focused) list.querySelector(`[data-up-file="${CSS.escape(focused)}"] ${which}`)?.focus();
    if (dialog.open) show(current);
  }

  // ---- the preview
  let current = 0;
  async function body(f) {
    const box = make("div", { "data-up-body": "" });
    const src = href(f), k = kind(f), e = ext(f.name);
    const none = (why) => {
      const n = make("div", { "data-up-none": "" });
      const icon = make("span", { "data-ext": "" });
      icon.innerHTML = svgs.file;
      n.append(icon, make("span", {}, why));
      return n;
    };
    if (!src) return box.append(none(L("Not uploaded yet, so there's nothing to show."))), box;
    try {
      if (k === "image") box.append(make("img", { src, alt: f.name }));
      else if (k === "pdf") box.append(make("iframe", { src, title: f.name }));
      else if (["csv", "tsv"].includes(e)) box.append(table(Z.csv(await (await fetch(src)).text(), e === "tsv" ? "\t" : ","), f.name));
      else if (e === "xlsx") box.append(table(await Z.xlsx(await (await fetch(src)).arrayBuffer()), f.name));
      else if (k === "text") box.append(make("pre", {}, (await (await fetch(src)).text()).slice(0, 200000)));
      else box.append(none(L("No preview for this kind of file. Download it to open it.")));
    } catch (err) {
      box.replaceChildren(none(L("Couldn't show this file ({error}).", { error: err.message })));
    }
    return box;
  }
  let shownId = null;
  async function show(i) {
    const all = files();
    if (!all.length) return dialog.close();
    current = (i + all.length) % all.length;
    const f = all[current];
    $("[data-up-title]", dialog).textContent = f.name;
    $("[data-up-count]", dialog).textContent = L("{n} of {count}", { n: current + 1, count: all.length });
    for (const b of dialog.querySelectorAll("[data-up-prev], [data-up-next]")) b.hidden = all.length < 2;
    const save = $("[data-up-save]", dialog);
    const src = href(f);
    save.hidden = !src;
    if (src) Object.assign(save, { href: src, download: f.name });
    if (shownId === f.id && $("[data-up-body]", dialog).childElementCount) return;
    shownId = f.id;
    // The last file's content goes at once, so it never shows under this one's name.
    $("[data-up-body]", dialog).replaceChildren();
    $("[data-up-body]", dialog).setAttribute("aria-busy", "true");
    const b = await body(f);
    if (shownId === f.id) $("[data-up-body]", dialog).replaceWith(b);
  }
  function open(id) {
    shownId = null;
    show(files().findIndex((f) => String(f.id) === id));
    if (!dialog.open) dialog.showModal();
    ($("[data-up-next]:not([hidden])", dialog) || $("[data-up-close]", dialog)).focus();
  }
  // A table of rows (the first as headings), cut to what's useful on screen.
  function table(rows, name) {
    const wrap = make("div", { "data-up-sheet": "" });
    const t = make("table", { "aria-label": name });
    const max = 500, cols = Math.min(50, Math.max(0, ...rows.slice(0, max).map((r) => r.length)));
    rows.slice(0, max).forEach((r, i) => {
      const tr = make("tr");
      for (let c = 0; c < cols; c++) tr.append(make(i ? "td" : "th", i ? {} : { scope: "col" }, r[c] ?? ""));
      t.append(tr);
    });
    wrap.append(t);
    if (rows.length > max) wrap.append(make("div", { "data-up-note": "" }, `The first ${max} of ${rows.length} rows.`));
    return wrap;
  }
  async function downloadAll() {
    const button = $("[data-up-zip]");
    button.disabled = true;
    say(L("Preparing the zip"));
    try {
      const used = new Map(), entries = [];
      for (const f of files()) {
        const src = href(f);
        if (!src) continue;
        const bytes = local.has(String(f.id)) ? new Uint8Array(await local.get(String(f.id)).arrayBuffer()) : new Uint8Array(await (await fetch(src)).arrayBuffer());
        // Two files of the same name get (2), (3), … so neither is lost.
        let name = f.name || "file", n = used.get(name) ?? 0;
        used.set(name, n + 1);
        if (n) name = name.replace(/(\.[^.]*)?$/, ` (${n + 1})$1`);
        entries.push([name, bytes]);
      }
      const url = URL.createObjectURL(Z.zip(entries));
      const a = make("a", { href: url, download: `${cfg.zipName}.zip` });
      document.body.append(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 60000);
      api.emit("download", { count: entries.length });
      say(L(["Downloaded {n} file", "Downloaded {n} files"], { n: entries.length }));
    } catch (err) {
      say(L("Couldn't make the zip: {error}", { error: err.message }));
    } finally {
      button.disabled = false;
    }
  }

  // ---- events
  input?.addEventListener("change", () => {
    add(input.files);
    input.value = "";
  });
  zone?.addEventListener("click", (e) => {
    if (input.hasAttribute("data-full")) {
      e.preventDefault();
      say(L("At most {max} files", { max: cfg.maxFiles }));
    }
  });
  if (zone) {
    let depth = 0;
    zone.addEventListener("dragenter", (e) => {
      e.preventDefault();
      if (!input.disabled) depth++, zone.setAttribute("data-over", "");
    });
    zone.addEventListener("dragover", (e) => e.preventDefault());
    zone.addEventListener("dragleave", () => --depth <= 0 && ((depth = 0), zone.removeAttribute("data-over")));
    zone.addEventListener("drop", (e) => {
      e.preventDefault();
      depth = 0;
      zone.removeAttribute("data-over");
      if (!input.disabled && e.dataTransfer?.files?.length) add(e.dataTransfer.files);
    });
  }
  list.addEventListener("click", (e) => {
    const id = e.target.closest("[data-up-file]")?.getAttribute("data-up-file");
    if (!id) return;
    if (e.target.closest("[data-up-remove]")) remove(id);
    else if (e.target.closest("[data-up-retry]")) send(id);
    else if (e.target.closest("[data-up-open]")) open(id);
  });
  $("[data-up-zip]")?.addEventListener("click", downloadAll);
  dialog.addEventListener("click", (e) => {
    if (e.target.closest("[data-up-prev]")) show(current - 1);
    else if (e.target.closest("[data-up-next]")) show(current + 1);
    else if (e.target.closest("[data-up-close]")) dialog.close();
  });
  dialog.addEventListener("keydown", (e) => {
    if (e.target.closest("input, textarea, select")) return;
    // Right to left, the next file is on the left.
    const fwd = getComputedStyle(dialog).direction === "rtl" ? -1 : 1;
    if (e.key === "ArrowLeft") e.preventDefault(), show(current - fwd);
    if (e.key === "ArrowRight") e.preventDefault(), show(current + fwd);
  });
  dialog.addEventListener("close", () => {
    const f = files()[current];
    if (f) list.querySelector(`[data-up-file="${CSS.escape(String(f.id))}"] [data-up-open]`)?.focus();
  });

  let drawn = null;
  draw();
  drawn = files();
  return {
    update() {
      const now = files();
      if (now === drawn) return;
      // Files dropped from state (from outside) are forgotten here too.
      for (const id of local.keys()) if (!now.some((f) => String(f.id) === id)) forget(id);
      drawn = now;
      draw();
    },
  };
};
