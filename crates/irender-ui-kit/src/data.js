// irender-ui-kit: records fetched by id and joined into rows (`window.__irenderData`),
// shared by every component on the page. `ui-data-source` says how one kind of record is
// fetched: many ids in one call (`batch`, GET or POST), or one at a time (`one`).
// `ui-data-query` fills a state key from an API or from ids, and `ui-data-table`'s
// `expand` joins records into its rows. Ids asked for while a page draws are fetched
// together, each id once; a record is kept in memory until its `ttl` passes, and shown
// while it's fetched again. With `persist: device` records are kept on the device too,
// per `scope` (the person signed in), and shown from there first: the fields named
// `private` encrypted with the vault's key (offline.js, unlocked by `ui-app-lock`), and
// not kept at all without one.
(() => {
  if (window.__irenderData) return;
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const blank = (v) => v === null || v === undefined || v === "";
  const at = (v, path) => (blank(path) ? v : String(path).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), v));
  // A path or an http(s) URL, as the table's `url`.
  const okUrl = (u) => {
    const l = String(u ?? "").trim().toLowerCase();
    if (!l || l.includes("\\") || l.startsWith("//") || /[\u0000-\u001f]/.test(l)) return false;
    if (/^https?:\/\//.test(l)) return true;
    const end = l.search(/[/?#]/);
    return !(end < 0 ? l : l.slice(0, end)).includes(":");
  };
  const sameOrigin = (u) => {
    try {
      return new URL(u, location.href).origin === location.origin;
    } catch {
      return false;
    }
  };
  const join = (u, p) => {
    const q = String(p);
    return q ? u + (u.includes("?") ? "&" : "?") + q : u;
  };
  // Query parameters: blank ones left out, objects as JSON.
  const params = (o) => {
    const p = new URLSearchParams();
    if (isObj(o)) for (const [k, v] of Object.entries(o)) if (!blank(v)) p.set(k, typeof v === "object" ? JSON.stringify(v) : String(v));
    return p;
  };
  // Where the records are in an answer: `pick`, else the answer when it's a list, else
  // its `rows`, `data` or `items`.
  const records = (answer, pick) => {
    if (!blank(pick)) return at(answer, pick);
    if (Array.isArray(answer) || !isObj(answer)) return answer;
    for (const k of ["rows", "data", "items"]) if (Array.isArray(answer[k])) return answer[k];
    return answer;
  };
  // The page's requests for data say so, and the service worker leaves them alone; nor
  // does the browser's own cache keep them: what they answer (people's details, say)
  // stays in the page's memory.
  async function getJson(url, init = {}, headers = {}) {
    if (!okUrl(url)) throw new Error("url");
    const own = sameOrigin(url) ? { "x-irender-data": "1" } : {};
    const res = await fetch(url, { cache: "no-store", ...init, headers: { accept: "application/json", ...own, ...headers, ...init.headers } });
    if (res.status === 304) return { same: true, etag: res.headers.get("etag") };
    if (res.status === 404) return { missing: true };
    if (!res.ok) throw new Error(String(res.status));
    return { answer: await res.json(), etag: res.headers.get("etag") };
  }

  // ---- on the device: `r␁scope␁source␁id` → { v, enc, at, etag }, `v` without the
  // private fields, which are in `enc`, sealed. Queries' rows: `q␁scope␁query`.
  const O = window.__irenderOffline, me = Math.random().toString(36).slice(2);
  const SEP = "\u0001";
  const fields = (cfg) => (Array.isArray(cfg?.private) ? cfg.private.map(String) : []);
  const retain = (cfg) => Number(cfg?.retain ?? 30) * 864e5;
  // A record without its private fields, and those fields alone (null when it has none).
  function split(cfg, v) {
    const priv = fields(cfg);
    if (!isObj(v) || !priv.length) return [v, null];
    const open = { ...v }, closed = {};
    for (const f of priv) if (f in open) (closed[f] = open[f]), delete open[f];
    return [open, Object.keys(closed).length ? closed : null];
  }
  // What's kept, as a value to show: the private fields put back while the vault's open
  // and they're younger than `retain`; `partial` when they couldn't be.
  async function unkeep(cfg, k) {
    if (!k) return null;
    const young = Date.now() - k.at < retain(cfg);
    const priv = k.enc && young ? await O.vault.open(k.enc) : null;
    const back = (v, p) => (isObj(v) && isObj(p) ? { ...v, ...p } : v);
    const v = Array.isArray(k.v) ? k.v.map((r, i) => back(r, priv?.[i])) : back(k.v, priv);
    return { v, at: k.at, etag: k.etag ?? null, partial: !!k.enc && !priv, total: k.total };
  }
  // A value to keep: its private fields sealed, or left out with no key.
  async function keep(cfg, v, extra) {
    const many = Array.isArray(v), parts = (many ? v : [v]).map((r) => split(cfg, r));
    const priv = parts.some(([, p]) => p) ? (many ? parts.map(([, p]) => p) : parts[0][1]) : null;
    const enc = priv ? await O.vault.seal(priv) : null;
    return { v: many ? parts.map(([o]) => o) : parts[0][0], enc, at: Date.now(), ...extra };
  }

  // ---- sources: { name, cfg, recs: id → record, wanted, timer }
  // A record: { s: loading | ready | missing | error, v, at, n, again }; `n` counts its
  // changes, `again` marks a kept one being fetched again.
  const sources = new Map();
  const source = (name) => {
    let s = sources.get(name);
    if (!s) sources.set(name, (s = { name, cfg: null, scope: null, recs: new Map(), wanted: new Set(), timer: 0 }));
    return s;
  };
  // Kept on the device: asked for, with somewhere to keep it and someone it's kept for.
  const device = (s) => !!O && s.cfg?.persist === "device" && !blank(s.scope);
  const slot = (s, id) => ["r", s.scope, s.name, id].join(SEP);
  const idOf = (id) => String(id);
  // Fetch failures are tried again after a while, not each time a row draws.
  const RETRY = 15e3;
  const fresh = (s, r) => {
    if (!r) return false;
    if (r.s === "loading" || r.again) return true;
    // At least five seconds: what's drawn asks again each time it's drawn.
    const ttl = Math.max(5, Number(s.cfg?.ttl ?? 300)) * 1e3;
    return Date.now() - r.at < (r.s === "error" ? Math.min(RETRY, ttl) : ttl);
  };

  // Components hear which sources changed, once per turn.
  const listeners = new Set();
  let changed = new Set(), telling = 0;
  const touch = (name) => {
    changed.add(name);
    telling ||= setTimeout(() => {
      telling = 0;
      const names = changed;
      changed = new Set();
      listeners.forEach((f) => f(names));
    });
  };
  const put = (s, id, rec) => {
    const old = s.recs.get(id);
    s.recs.set(id, { ...rec, n: (old?.n ?? 0) + 1 });
    touch(s.name);
  };

  // `scope` is whose records these are: another one (someone else signed in) lets the
  // records in memory go.
  function define(name, cfg, scope = null) {
    const s = source(name);
    const was = s.scope;
    s.cfg = cfg;
    s.scope = blank(scope) ? null : String(scope);
    if (was !== null && was !== s.scope && s.recs.size) {
      s.recs.clear();
      touch(name);
    }
    if (s.wanted.size) schedule(s);
  }
  function want(name, ids) {
    const s = source(name);
    let more = false;
    for (const id of ids) {
      if (blank(id) || typeof id === "object") continue;
      const k = idOf(id);
      if (!fresh(s, s.recs.get(k)) && !s.wanted.has(k)) {
        s.wanted.add(k);
        more = true;
      }
    }
    if (more) schedule(s);
  }
  // Until the turn ends, so that every row drawn now asks in the same call. A source
  // declared later on the page takes the ids asked for before it.
  function schedule(s) {
    if (!s.cfg || s.timer) return;
    s.timer = setTimeout(() => flush(s), Math.max(0, Number(s.cfg.wait) || 0));
  }

  async function flush(s) {
    s.timer = 0;
    let ids = [...s.wanted];
    s.wanted.clear();
    if (!ids.length) return;
    for (const id of ids) {
      const r = s.recs.get(id);
      if (r && r.v !== undefined) r.again = true;
      else put(s, id, { s: "loading", at: 0 });
    }
    // What's kept shows first; only what's missing, or older than `ttl`, is fetched.
    if (device(s)) {
      const scope = s.scope, cold = ids.filter((id) => s.recs.get(id)?.v === undefined);
      const found = await readKept(s, cold).catch(() => new Map());
      if (s.scope !== scope) return;
      for (const [id, k] of found) put(s, id, { s: k.v === null ? "missing" : "ready", ...k, again: true });
      const ttl = Number(s.cfg.ttl ?? 300) * 1e3;
      ids = ids.filter((id) => {
        const r = s.recs.get(id);
        if (r && r.s !== "loading" && !r.partial && Date.now() - r.at < ttl) return (r.again = false), false;
        return true;
      });
      if (!ids.length) return;
    }
    const cfg = s.cfg, jobs = [];
    if (isObj(cfg.batch)) for (const chunk of chunks(cfg.batch, ids)) jobs.push(() => fetchMany(s, chunk));
    else if (!blank(cfg.one)) for (const id of ids) jobs.push(() => fetchOne(s, id));
    else for (const id of ids) put(s, id, { s: "error", at: Date.now(), error: "no batch or one" });
    run(s, jobs);
  }
  // At most `concurrency` calls at a time per source (default 4).
  function run(s, jobs) {
    const limit = Math.max(1, Number(s.cfg.concurrency) || 4);
    let next = 0;
    const lane = async () => {
      while (next < jobs.length) await jobs[next++]();
    };
    for (let i = 0; i < Math.min(limit, jobs.length); i++) lane();
  }
  // Ids in calls of at most `max` (default 50); a GET's address also stays under 2,000
  // characters.
  function chunks(b, ids) {
    const max = Math.max(1, Number(b.max) || 50), out = [];
    let cur = [];
    for (const id of ids) {
      if (cur.length && (cur.length >= max || ((b.method ?? "GET") === "GET" && batchUrl(b, [...cur, id]).length > 2000))) out.push(cur), (cur = []);
      cur.push(id);
    }
    if (cur.length) out.push(cur);
    return out;
  }
  const param = (b) => b.param ?? "ids";
  function batchUrl(b, ids) {
    const p = params(b.params);
    if (b.style === "repeat") for (const id of ids) p.append(param(b), id);
    else p.set(param(b), ids.join(","));
    return join(String(b.url), p);
  }
  // `{ id: record }`, from a list of records (by their `key`) or a map by id.
  function byId(s, got) {
    const key = s.cfg.key ?? "id", out = new Map();
    if (Array.isArray(got)) for (const r of got) isObj(r) && !blank(at(r, key)) && out.set(idOf(at(r, key)), r);
    else if (isObj(got)) for (const [k, r] of Object.entries(got)) r != null && out.set(idOf(k), r);
    return out;
  }
  async function fetchMany(s, ids) {
    const b = s.cfg.batch;
    try {
      const post = String(b.method ?? "GET").toUpperCase() === "POST";
      const init = post ? { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ ...(isObj(b.body) ? b.body : {}), [param(b)]: ids }) } : {};
      const { answer, missing } = await getJson(post ? join(String(b.url), params(b.params)) : batchUrl(b, ids), init, s.cfg.headers);
      const got = missing ? new Map() : byId(s, records(answer, b.pick));
      const now = Date.now();
      for (const id of ids) put(s, id, got.has(id) ? { s: "ready", v: got.get(id), at: now } : { s: "missing", v: null, at: now });
      writeKept(s, ids.map((id) => [id, got.get(id) ?? null]));
    } catch (e) {
      failed(s, ids, e);
    }
  }
  async function fetchOne(s, id) {
    try {
      const url = String(s.cfg.one).replaceAll("{id}", encodeURIComponent(id));
      // What's here is asked about by its ETag: 304, it's still right.
      const had = s.recs.get(id), etag = had?.v != null && !had.partial ? had.etag : null;
      const { answer, missing, same, etag: tag } = await getJson(url, etag ? { headers: { "if-none-match": etag } } : {}, s.cfg.headers);
      if (same) {
        put(s, id, { ...had, at: Date.now(), again: false });
        return writeKept(s, [[id, had.v, tag ?? etag]]);
      }
      const v = missing ? null : records(answer, s.cfg.pick);
      put(s, id, isObj(v) ? { s: "ready", v, at: Date.now(), etag: tag } : { s: "missing", v: null, at: Date.now() });
      writeKept(s, [[id, isObj(v) ? v : null, tag]]);
    } catch (e) {
      failed(s, [id], e);
    }
  }
  // A record that was kept stays shown; it's fetched again with the next ask after a while.
  function failed(s, ids, e) {
    const now = Date.now();
    for (const id of ids) {
      const r = s.recs.get(id);
      put(s, id, r && r.v !== undefined && r.s === "ready" ? { ...r, again: false, at: now - Number(s.cfg.ttl ?? 300) * 1e3 + RETRY } : { s: "error", at: now, error: String(e?.message ?? e) });
    }
  }

  // ---- kept on the device
  async function readKept(s, ids) {
    const out = new Map();
    if (!ids.length) return out;
    const got = await O.db.getMany("records", ids.map((id) => slot(s, id)));
    for (let i = 0; i < ids.length; i++) {
      const k = await unkeep(s.cfg, got[i]);
      if (k) out.set(ids[i], k);
    }
    return out;
  }
  // `[id, record | null, etag]`s: an id with no record is kept as none, so it isn't asked
  // for again offline.
  async function writeKept(s, list) {
    if (!device(s) || !list.length) return;
    try {
      const pairs = await Promise.all(list.map(async ([id, v, etag]) => [slot(s, id), v === null ? { v: null, enc: null, at: Date.now() } : await keep(s.cfg, v, etag ? { etag } : {})]));
      await O.db.putMany("records", pairs);
      O.tell({ type: "records", scope: s.scope, name: s.name, ids: list.map(([id]) => id), from: me });
    } catch { /* not kept: fetched again next time */ }
  }
  // Another tab kept newer records: shown here too.
  async function heard(m) {
    if (m.type === "vault-gone") return forgetSealed();
    if (m.type === "wiped" && m.from !== me) {
      for (const s of sources.values()) if ((!m.name || s.name === m.name) && (m.scope == null || s.scope === String(m.scope))) s.recs.clear(), touch(s.name);
      return;
    }
    if (m.type !== "records" || m.from === me) return;
    const s = sources.get(m.name);
    if (!s || !device(s) || s.scope !== m.scope) return;
    const ids = m.ids.filter((id) => s.recs.has(id));
    for (const [id, k] of await readKept(s, ids)) put(s, id, { s: k.v === null ? "missing" : "ready", ...k });
  }
  O?.on((m) => heard(m).catch(() => {}));
  // Locked, the private fields leave memory; unlocked, they come back from the device.
  O?.vault.on(async (state) => {
    for (const s of sources.values()) {
      if (!device(s) || !fields(s.cfg).length) continue;
      if (state === "closed") {
        for (const [id, r] of s.recs) if (isObj(r.v)) s.recs.set(id, { ...r, v: split(s.cfg, r.v)[0], partial: true, n: r.n + 1 });
        touch(s.name);
      } else {
        const ids = [...s.recs].filter(([, r]) => r.partial).map(([id]) => id);
        for (const [id, k] of await readKept(s, ids).catch(() => new Map())) if (!k.partial) put(s, id, { s: "ready", ...k });
      }
    }
  });
  // The key's gone: what it sealed is let go everywhere on the device.
  async function forgetSealed() {
    const all = await O.db.entries("records").catch(() => []);
    await O.db.putMany("records", all.filter(([, k]) => k?.enc).map(([key, k]) => [key, { ...k, enc: null }])).catch(() => {});
  }
  // Everything kept for `scope` (and of `name`, if given), on the device and in memory.
  async function wipe(scope, name) {
    for (const s of sources.values()) {
      if ((name && s.name !== name) || (scope != null && s.scope !== String(scope))) continue;
      s.recs.clear();
      touch(s.name);
    }
    if (!O) return;
    const mine = (key) => {
      const [kind, sc, n] = key.split(SEP);
      return (scope == null || sc === String(scope)) && (!name || (kind === "r" && n === name));
    };
    const all = await O.db.entries("records").catch(() => []);
    await O.db.putMany("records", all.filter(([key]) => mine(key)).map(([key]) => [key, null])).catch(() => {});
    O.tell({ type: "wiped", scope, name, from: me });
  }

  // ---- joining: `{ name: { source, from } }` puts the records whose ids are in each row's
  // `from` (one id, or a list) in its `name`. While some are still on their way the row's
  // `$loading` has `name: true`; one that couldn't be fetched, its `$failed`. A row that
  // didn't change keeps its identity, so lists keep its nodes. With `ask` false, only
  // records already here are joined, and nothing is fetched.
  const memos = new WeakMap();
  function joinRow(row, expand, ask = true) {
    if (!isObj(row) || !isObj(expand)) return row;
    let memo = memos.get(expand);
    if (!memo) memos.set(expand, (memo = new WeakMap()));
    const parts = [];
    let sig = "";
    for (const [name, e] of Object.entries(expand)) {
      const raw = at(row, e.from ?? `${name}Id`), many = Array.isArray(raw);
      const ids = (many ? raw : [raw]).filter((x) => !blank(x) && typeof x !== "object");
      if (ask) want(e.source, ids);
      const s = source(e.source), recs = ids.map((id) => s.recs.get(idOf(id)));
      sig += recs.map((r) => r?.n ?? "-").join(",") + ";";
      parts.push({ name, many, recs });
    }
    const m = memo.get(row);
    if (m && m.sig === sig) return m.out;
    const out = { ...row }, loading = {}, failed = {};
    const value = (r) => (r && r.v !== undefined ? r.v : null);
    for (const { name, many, recs } of parts) {
      out[name] = many ? recs.map(value).filter((v) => v !== null) : value(recs[0]) ?? null;
      if (recs.some((r) => !r || (r.s === "loading" && r.v === undefined))) loading[name] = true;
      if (recs.some((r) => r?.s === "error")) failed[name] = true;
    }
    if (Object.keys(loading).length) out.$loading = loading;
    if (Object.keys(failed).length) out.$failed = failed;
    memo.set(row, { sig, out });
    return out;
  }
  // The sources an `expand` reads.
  const reads = (expand) => (isObj(expand) ? Object.values(expand).map((e) => e.source) : []);

  // Records by id, as `joinRow` puts them: `{ value, loading, failed }`.
  function lookup(name, ids) {
    want(name, ids);
    const s = source(name);
    return ids.map((id) => {
      const r = blank(id) ? null : s.recs.get(idOf(id));
      return { value: r && r.v !== undefined ? r.v : null, loading: !blank(id) && (!r || (r.s === "loading" && r.v === undefined)), failed: r?.s === "error" };
    });
  }
  // Fetched again the next time they're asked for: every record of a source, or some.
  function invalidate(name, ids) {
    const s = source(name);
    for (const [id, r] of s.recs) if (!ids || ids.map(idOf).includes(id)) s.recs.set(id, { ...r, at: 0, again: false, n: r.n + 1 });
    touch(name);
  }

  window.__irenderData = {
    define,
    want,
    join: joinRow,
    reads,
    lookup,
    invalidate,
    wipe,
    getJson,
    records,
    params,
    on: (f) => (listeners.add(f), () => listeners.delete(f)),
  };

  // ---------------------------------------------------------------- the components
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit;
  // `scope` and `clear` may follow state: someone else signed in, or signing out.
  B["ui-data-source"] = (el, api) => {
    const cfg = api.config;
    let cleared = false;
    function update() {
      define(cfg.name, cfg, K.value(api, cfg.scope));
      const clear = !!K.value(api, cfg.clear);
      if (clear && !cleared) wipe(K.value(api, cfg.scope) ?? null, cfg.name).then(() => api.emit("cleared", {}));
      cleared = clear;
    }
    update();
    return { update };
  };

  // Fills `into` with rows from `url` (or the records for `ids` of `source`), joined by
  // `expand`; `status` gets { state, pending, total, error, at, cached }. With `persist:
  // device` the last answer for each address is kept, shown first and when it can't be
  // loaded (`stale`).
  B["ui-data-query"] = (el, api) => {
    const cfg = api.config, D = window.__irenderData;
    let key = null, seq = 0, rows = null, one = false, total = null, error = null, loading = false, at = null, timer = 0, last, lastStatus;
    let cached = false, partial = false;
    const scope = () => K.value(api, cfg.scope);
    const kept = () => !!O && cfg.persist === "device" && !blank(scope());
    const qslot = (q) => ["q", scope(), JSON.stringify([q.url ?? "", q.params ?? null])].join(SEP);
    async function fromDevice(q, id) {
      const k = await unkeep(cfg, await O.db.get("records", qslot(q)).catch(() => null));
      if (!k || id !== seq || !Array.isArray(k.v)) return;
      // Fresher rows already came from the server.
      if (!cached && rows !== null && !loading) return;
      rows = k.v;
      one = !!k.one;
      total = k.total ?? rows.length;
      at = new Date(k.at).toISOString();
      cached = true;
      partial = k.partial;
      publish();
    }
    O?.vault.on((state) => {
      if (!kept() || !fields(cfg).length || !rows) return;
      if (state === "closed") (rows = rows.map((r) => split(cfg, r)[0])), (partial = true), publish();
      else if (partial && key) fromDevice(JSON.parse(key), seq);
    });
    const used = new Set([...reads(cfg.expand), ...(cfg.source ? [cfg.source] : [])]);
    function ask() {
      if (cfg.source) return { ids: K.value(api, cfg.ids) ?? null, refresh: K.value(api, cfg.refresh) };
      return { url: K.value(api, cfg.url), params: K.value(api, cfg.params), refresh: K.value(api, cfg.refresh) };
    }
    async function load(q) {
      const id = ++seq;
      loading = true;
      error = null;
      cached = false;
      if (kept()) await fromDevice(q, id);
      if (id !== seq) return;
      publish();
      try {
        const { answer, missing } = await getJson(join(String(q.url ?? ""), params(q.params)), {}, cfg.headers);
        if (id !== seq) return;
        const got = missing ? [] : records(answer, cfg.pick);
        one = isObj(got);
        rows = one ? [got] : Array.isArray(got) ? got : [];
        total = Number.isFinite(answer?.total) ? answer.total : rows.length;
        at = new Date().toISOString();
        loading = cached = partial = false;
        if (kept()) keep(cfg, rows, { total, one }).then((v) => O.db.put("records", qslot(q), v)).catch(() => {});
        api.emit("loaded", { rows, total });
      } catch (e) {
        if (id !== seq) return;
        loading = false;
        error = String(e?.message ?? e);
        api.emit("failed", { error });
      }
      publish();
    }
    function publish() {
      let value, pending = 0;
      if (cfg.source) {
        const q = JSON.parse(key);
        const many = Array.isArray(q.ids), ids = many ? q.ids : [q.ids];
        const found = lookup(cfg.source, ids);
        pending = found.filter((f) => f.loading).length;
        const failed = found.some((f) => f.failed);
        error = failed ? "failed" : null;
        const recs = found.map((f) => (f.value && cfg.expand ? joinRow(f.value, cfg.expand) : f.value));
        pending += recs.filter((r) => r?.$loading).length;
        value = many ? recs.filter((r) => r !== null) : recs[0] ?? null;
        total = many ? value.length : value ? 1 : 0;
        loading = pending > 0 && !recs.some(Boolean);
      } else {
        const out = (rows ?? []).map((r) => (cfg.expand ? joinRow(r, cfg.expand) : r));
        pending = out.filter((r) => r?.$loading).length;
        value = rows === null ? (one ? null : []) : one ? out[0] ?? null : out;
      }
      // The same rows as before: nothing to tell.
      const same = Array.isArray(value) && Array.isArray(last) ? value.length === last.length && value.every((r, i) => r === last[i]) : value === last;
      if (!same) {
        last = value;
        api.set(cfg.into, value);
      }
      if (cfg.status) {
        const st = { state: error ? (rows?.length ? "stale" : "error") : loading && !cached ? "loading" : pending ? "joining" : rows === null && !cfg.source ? "idle" : "ready", pending, total, error, at, cached };
        const s = JSON.stringify(st);
        if (s !== lastStatus) {
          lastStatus = s;
          api.set(cfg.status, st);
        }
      }
    }
    D.on((names) => [...used].some((n) => names.has(n)) && el.isConnected && publish());
    let cleared = false;
    function update() {
      // Signing out: the answers kept for this scope go, and so do the rows.
      const clear = !!K.value(api, cfg.clear);
      if (clear && !cleared) {
        if (O && !blank(scope())) O.db.entries("records", ["q", scope(), ""].join(SEP)).then((all) => O.db.putMany("records", all.map(([k]) => [k, null]))).catch(() => {});
        rows = null;
        cached = false;
        key = null;
        publish();
      }
      cleared = clear;
      if (clear || K.value(api, cfg.paused)) return;
      const q = ask(), k = JSON.stringify(q);
      if (k === key) return;
      const first = key === null;
      key = k;
      if (cfg.source) return publish();
      if (blank(q.url)) return;
      clearTimeout(timer);
      // Typing in a filter waits for a pause; the first load goes at once.
      if (first) load(q);
      else timer = setTimeout(() => load(q), 200);
    }
    update();
    return { update };
  };
})();
