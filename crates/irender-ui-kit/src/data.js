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
    const res = await native(url, { cache: "no-store", ...init, headers: { accept: "application/json", ...own, ...headers, ...init.headers } });
    if (res.status === 304) return { same: true, etag: res.headers.get("etag") };
    // Gone, or no longer theirs: what was kept of it goes too.
    if (res.status === 404 || res.status === 403 || res.status === 410) return { missing: true };
    if (!res.ok) throw Object.assign(new Error(String(res.status)), { status: res.status });
    return { answer: await res.json(), etag: res.headers.get("etag") };
  }

  // ---- on the device: `r␁scope␁source␁id` → { v, enc, at, etag }, `v` without the
  // private fields, which are in `enc`, sealed. Queries' rows: `q␁scope␁query`.
  const O = window.__irenderOffline, me = Math.random().toString(36).slice(2);
  // The page's own fetch, before writes to a source are taken over (below); `submit`
  // goes through the one that takes them.
  const native = self.__irenderFetch ?? self.fetch.bind(self);
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
  // and they're younger than `retain`; `partial` when they aren't there (sealed and
  // locked, too old, or kept without a key), so it's fetched again rather than trusted.
  async function unkeep(cfg, k) {
    if (!k) return null;
    const young = Date.now() - k.at < retain(cfg);
    const priv = k.enc && young ? await O.vault.open(k.enc) : null;
    const back = (v, p) => (isObj(v) && isObj(p) ? { ...v, ...p } : v);
    const v = Array.isArray(k.v) ? k.v.map((r, i) => back(r, priv?.[i])) : back(k.v, priv);
    const some = Array.isArray(k.v) ? k.v.length > 0 : k.v != null;
    return { v, at: k.at, etag: k.etag ?? null, partial: fields(cfg).length > 0 && some && !priv, total: k.total, one: k.one, cursor: k.cursor, fullAt: k.fullAt };
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
  // Changes made here, waiting to be sent, in order: see "writes" below.
  let writes = [];
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
    return Date.now() - r.at < (r.s === "error" ? Math.min(RETRY, ttl) : r.s === "missing" ? missingTtl(s) : ttl);
  };
  // How long an id with no record is taken at its word: `missingTtl`, else `ttl`.
  const missingTtl = (s) => Math.max(5, Number(s.cfg?.missingTtl ?? s.cfg?.ttl ?? 300)) * 1e3;

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
  // A record changed here and not yet saved is `pending`; one the server changed too,
  // waiting for someone to choose, `conflict`.
  const put = (s, id, rec) => {
    const old = s.recs.get(id), mine = writes.filter((w) => w.source === s.name && w.rid === id && w.status !== "refused");
    s.recs.set(id, { ...rec, pending: mine.length > 0, conflict: mine.some((w) => w.status === "conflict"), n: (old?.n ?? 0) + 1 });
    touch(s.name);
  };

  // A source named in `expand` (or a query's `source`) and declared nowhere: once the page
  // has loaded, its ids fail (`$failed`, a query's `error`) rather than wait for it, and the
  // console says which. Declared later after all (a part of the page shown later), they're
  // fetched then.
  const UNDECLARED = "no ui-data-source";
  let loaded = false;
  function undeclared(s) {
    if (s.cfg || !s.wanted.size) return;
    if (!s.warned) (s.warned = true), console.warn(`irender: nothing on the page declares the ui-data-source "${s.name}"; add one with \`name: ${s.name}\``);
    const now = Date.now();
    for (const id of s.wanted) put(s, id, { s: "error", at: now, error: UNDECLARED });
    s.wanted.clear();
  }
  const settle = () => setTimeout(() => ((loaded = true), sources.forEach(undeclared)));
  if (document.readyState === "complete") settle();
  else addEventListener("load", settle, { once: true });

  // `scope` is whose records these are: another one (someone else signed in) lets the
  // records in memory go.
  function define(name, cfg, scope = null) {
    const s = source(name);
    const was = s.scope;
    if (!s.cfg) {
      for (const [id, r] of s.recs) if (r.error === UNDECLARED) s.recs.delete(id), s.wanted.add(id);
      if (s.wanted.size) touch(name);
    }
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
    if (!s.cfg) return void (loaded && setTimeout(() => undeclared(s)));
    if (s.timer) return;
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
        if (r && r.s !== "loading" && !r.partial && Date.now() - r.at < (r.s === "missing" ? missingTtl(s) : ttl)) return (r.again = false), false;
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
    const b = s.cfg.batch, scope = s.scope;
    try {
      const post = String(b.method ?? "GET").toUpperCase() === "POST";
      const init = post ? { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ ...(isObj(b.body) ? b.body : {}), [param(b)]: ids }) } : {};
      const { answer, missing } = await getJson(post ? join(String(b.url), params(b.params)) : batchUrl(b, ids), init, s.cfg.headers);
      // Someone else by now: the answer was for whoever asked, and isn't shown or kept.
      if (s.scope !== scope) return;
      const got = missing ? new Map() : byId(s, records(answer, b.pick));
      const now = Date.now();
      for (const id of ids) put(s, id, got.has(id) ? { s: "ready", v: got.get(id), at: now } : { s: "missing", v: null, at: now });
      writeKept(s, ids.map((id) => [id, got.get(id) ?? null]));
    } catch (e) {
      if (s.scope === scope) failed(s, ids, e);
    }
  }
  async function fetchOne(s, id) {
    const scope = s.scope;
    try {
      const url = String(s.cfg.one).replaceAll("{id}", encodeURIComponent(id));
      // What's here is asked about by its ETag: 304, it's still right.
      const had = s.recs.get(id), etag = had?.v != null && !had.partial ? had.etag : null;
      const { answer, missing, same, etag: tag } = await getJson(url, etag ? { headers: { "if-none-match": etag } } : {}, s.cfg.headers);
      if (s.scope !== scope) return;
      if (same) {
        put(s, id, { ...had, at: Date.now(), again: false });
        return writeKept(s, [[id, had.v, tag ?? etag]]);
      }
      const v = missing ? null : records(answer, s.cfg.pick);
      put(s, id, isObj(v) ? { s: "ready", v, at: Date.now(), etag: tag } : { s: "missing", v: null, at: Date.now() });
      writeKept(s, [[id, isObj(v) ? v : null, tag]]);
    } catch (e) {
      if (s.scope === scope) failed(s, [id], e);
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
    const out = { ...row }, loading = {}, failed = {}, pending = {}, conflict = {};
    const value = (r) => (r && r.v !== undefined ? r.v : null);
    for (const { name, many, recs } of parts) {
      out[name] = many ? recs.map(value).filter((v) => v !== null) : value(recs[0]) ?? null;
      if (recs.some((r) => !r || (r.s === "loading" && r.v === undefined))) loading[name] = true;
      if (recs.some((r) => r?.s === "error")) failed[name] = true;
      if (recs.some((r) => r?.pending)) pending[name] = true;
      if (recs.some((r) => r?.conflict)) conflict[name] = true;
    }
    if (Object.keys(loading).length) out.$loading = loading;
    if (Object.keys(failed).length) out.$failed = failed;
    if (Object.keys(pending).length) out.$pending = pending;
    if (Object.keys(conflict).length) out.$conflict = conflict;
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
  // Those that couldn't be fetched (all of them, or some), asked for again now: a cell's
  // retry button. They failed together, as a rule, so one button asks for them all.
  function retry(name, ids) {
    const s = source(name), only = ids && new Set(ids.filter((id) => !blank(id)).map(idOf)), again = [];
    for (const [id, r] of s.recs) if (r.s === "error" && (!only || only.has(id))) s.recs.delete(id), again.push(id);
    touch(name);
    want(name, again);
  }
  // Fetched again the next time they're asked for (at once, where they're drawn): every
  // record of a source, or some. One changed here and not yet saved keeps what it has: it's
  // fetched again when it's saved.
  function invalidate(name, ids) {
    const s = source(name), only = ids && new Set(ids.filter((id) => !blank(id)).map(idOf));
    for (const [id, r] of s.recs) if ((!only || only.has(id)) && !r.pending && r.s !== "loading") s.recs.set(id, { ...r, at: 0, again: false, n: r.n + 1 });
    touch(name);
  }

  // ---- writes: a `submit` to a source's `write.url` (`/api/people/{id}`: PATCH or PUT
  // changes a record, DELETE deletes it) or `write.create` (POST) is taken over here. The
  // record changes at once (`$pending` on joined rows), and the change waits in a queue,
  // kept on the device, to be sent in order with the version it was made on (`If-Match`,
  // or the `write.version` field). A server that changed it meanwhile answers 409 or 412,
  // and `write.conflict` settles it: `merge` (default) keeps both sides' changes field
  // by field and asks only about a field both changed, `server-wins`, `client-wins`, or
  // `ask`, which waits for someone to choose (`ui-data-conflicts`).
  // An item: { id, source, scope, rid, kind (update, delete, create), changes, base,
  // version, at, tries, next, status (waiting, sending, conflict, refused), error, theirs, fields }
  const wslot = (w) => ["w", w.scope ?? "", String(w.at).padStart(15, "0"), w.id].join(SEP);
  const same = (a, b) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);
  const writer = (s) => (isObj(s?.cfg?.write) ? s.cfg.write : null);
  const versionOf = (s, rec) => {
    const how = writer(s)?.version ?? "etag";
    return how === "etag" ? rec?.etag ?? null : rec?.v ? at(rec.v, how) ?? null : null;
  };
  const wheard = new Set();
  const told = () => wheard.forEach((f) => f());
  // Kept on the device with the source's private fields sealed; without a key they stay
  // in memory, and go with the page.
  // One write's saves and its removal happen in order, so a late save can't bring back
  // one that's gone.
  const io = new Map();
  const inTurn = (w, f) => {
    const p = (io.get(w.id) ?? Promise.resolve()).then(f).catch(() => {});
    io.set(w.id, p);
    return p;
  };
  const saveWrite = (w) => inTurn(w, async () => {
    const s = sources.get(w.source);
    if (w.gone || !s || !device(s)) return;
    const { changes, base, theirs, gone, clashes, ...rest } = w;
    const secret = fields(s.cfg).length > 0;
    const enc = secret ? await O.vault.seal({ changes, base, theirs }) : null;
    if (secret && !enc) return;
    await O.db.put("records", wslot(w), secret ? { ...rest, enc } : { ...rest, changes, base, theirs });
  });
  const dropWrite = (w) => {
    writes = writes.filter((x) => x !== w);
    w.gone = true;
    if (O) inTurn(w, () => O.db.del("records", wslot(w))).then(() => io.delete(w.id));
  };
  // The writes kept for a scope, as the page opens (and as the vault opens, for sealed ones).
  async function loadWrites(scope) {
    if (!O || blank(scope)) return;
    const kept = await O.db.entries("records", ["w", scope, ""].join(SEP)).catch(() => []);
    let more = false;
    for (const [, w] of kept) {
      if (!w || writes.some((x) => x.id === w.id)) continue;
      let item = w;
      if (w.enc) {
        const open = await O.vault.open(w.enc);
        if (!open) continue;
        const { enc, ...rest } = w;
        item = { ...rest, ...open };
      }
      writes.push({ ...item, status: item.status === "sending" ? "waiting" : item.status });
      more = true;
    }
    if (!more) return;
    writes.sort((a, b) => a.at - b.at);
    for (const w of writes) {
      const s = sources.get(w.source), r = s?.recs.get(w.rid);
      if (r) put(s, w.rid, r);
    }
    told();
    flushWrites();
  }

  // Which source a request writes to, and how.
  function writeOf(url, method) {
    let path;
    try {
      const u = new URL(url, location.href);
      path = u.origin + u.pathname;
    } catch {
      return null;
    }
    for (const s of sources.values()) {
      const w = writer(s);
      if (!w) continue;
      const full = (u) => (u ? new URL(String(u), location.href) : null);
      const create = full(w.create);
      if (method === "POST" && create && create.origin + create.pathname === path) return { s, kind: "create" };
      // `{id}` stands for any one segment; a marker the URL parser keeps holds its place.
      const one = full(String(w.url ?? "").replace("{id}", "IRENDERID"));
      if (!one) continue;
      const pattern = decodeURIComponent(one.origin + one.pathname).split("IRENDERID").map((x) => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("([^/]+)");
      const m = new RegExp(`^${pattern}$`).exec(decodeURIComponent(path));
      if (!m) continue;
      if (method === "DELETE") return { s, kind: "delete", id: m[1] };
      if (method === "PATCH" || method === "PUT" || (method === "POST" && String(w.method ?? "").toUpperCase() === "POST")) return { s, kind: "update", id: m[1] };
    }
    return null;
  }

  // A change made here: shown at once, and queued. Answers what `submit` gets back.
  async function change(s, kind, id, changes) {
    const key = s.cfg.key ?? "id";
    if (kind === "create") id = `tmp-${O?.id?.() ?? Math.random().toString(36).slice(2)}`;
    const rid = idOf(id), rec = s.recs.get(rid);
    const before = rec?.v ?? null;
    const base = kind === "delete" ? before : Object.fromEntries(Object.keys(changes).map((f) => [f, isObj(before) ? before[f] ?? null : null]));
    // Edits to a record still waiting are sent as one.
    const open = kind === "update" ? writes.find((w) => w.source === s.name && w.rid === rid && w.kind !== "delete" && w.status === "waiting") : null;
    if (open) {
      for (const f of Object.keys(changes)) if (!(f in open.base) && open.kind === "update") open.base[f] = base[f];
      open.changes = { ...open.changes, ...changes };
      saveWrite(open);
    } else {
      const w = { id: O?.id?.() ?? Math.random().toString(36).slice(2), source: s.name, scope: s.scope, rid, kind, changes: kind === "delete" ? {} : changes, base, version: versionOf(s, rec), at: Date.now(), tries: 0, next: 0, status: "waiting", error: null };
      writes.push(w);
      saveWrite(w);
    }
    const v = kind === "delete" ? null : { ...(isObj(before) ? before : {}), ...changes, ...(kind === "create" ? { [key]: rid } : {}) };
    put(s, rid, { ...(rec ?? {}), s: v === null ? "missing" : "ready", v, at: rec?.at ?? Date.now(), again: false });
    writeKept(s, [[rid, v, rec?.etag]]);
    told();
    flushWrites();
    return { queued: true, id: rid, record: v };
  }

  // Sent in order, one at a time across tabs; stops without a connection.
  let flushing = null;
  function flushWrites() {
    if (flushing) return flushing.then(() => writes.some((w) => w.status === "waiting" && w.next <= Date.now()) && flushWrites());
    // The next one ready each time: a merged change goes again at once.
    const ready = (w) => w.status === "waiting" && w.next <= Date.now() && writer(sources.get(w.source)) && sources.get(w.source).scope === w.scope;
    const go = async () => {
      for (let w = writes.find(ready); w; w = writes.find(ready)) {
        const s = sources.get(w.source);
        if (!(await sendWrite(s, writer(s), w))) break;
      }
    };
    flushing = (self.navigator?.locks ? navigator.locks.request("irender-data-writes", { ifAvailable: true }, (l) => (l ? go() : 0)) : go()).finally(() => (flushing = null));
    return flushing;
  }
  // False when there's no connection, to stop there.
  async function sendWrite(s, cfg, w) {
    const key = s.cfg.key ?? "id", how = cfg.version ?? "etag";
    const url = w.kind === "create" ? String(cfg.create) : String(cfg.url).replaceAll("{id}", encodeURIComponent(w.rid));
    const method = w.kind === "create" ? "POST" : w.kind === "delete" ? "DELETE" : String(cfg.method ?? "PATCH").toUpperCase();
    const body = w.kind === "delete" ? undefined : JSON.stringify({ ...w.changes, ...(how !== "etag" && w.version != null && w.kind === "update" ? { [how]: w.version } : {}) });
    const headers = { accept: "application/json", "x-irender-data": "1", "x-queued-id": w.id, ...(body ? { "content-type": "application/json" } : {}), ...(w.version != null && !w.force ? { "if-match": how === "etag" ? String(w.version) : `"${w.version}"` } : {}), ...(s.cfg.headers ?? {}) };
    w.status = "sending";
    told();
    let res;
    try {
      res = await native(url, { method, headers, body, cache: "no-store" });
    } catch {
      w.status = "waiting";
      told();
      return false;
    }
    let answer = null;
    try {
      answer = (res.headers.get("content-type") ?? "").includes("json") ? await res.json() : null;
    } catch { /* nothing back */ }
    const etag = res.headers.get("etag");
    if (res.ok) {
      dropWrite(w);
      let rid = w.rid;
      // Made here: from now on it's known by the server's id, in writes still waiting too.
      if (w.kind === "create" && isObj(answer) && !blank(at(answer, key))) {
        rid = idOf(at(answer, key));
        s.recs.delete(w.rid);
        for (const x of writes) if (x.source === s.name && x.rid === w.rid) (x.rid = rid), saveWrite(x);
        if (O && device(s)) O.db.del("records", slot(s, w.rid)).catch(() => {});
        emitAll("created", { source: s.name, temp: w.rid, id: rid });
      }
      if (w.kind === "delete") put(s, rid, { s: "missing", v: null, at: Date.now() });
      else {
        const v = isObj(answer) && !blank(at(answer, key)) ? answer : s.recs.get(rid)?.v;
        put(s, rid, { s: "ready", v, at: Date.now(), etag });
        writeKept(s, [[rid, v ?? null, etag]]);
      }
      emitAll("saved", { source: s.name, id: rid, kind: w.kind });
      told();
      return true;
    }
    if (res.status === 409 || res.status === 412) {
      await conflict(s, cfg, w, isObj(answer) && !blank(at(answer, key)) ? answer : null, etag);
      return true;
    }
    if (res.status >= 500 || res.status === 429 || res.status === 408) {
      w.tries++;
      w.next = Date.now() + Math.min(3600, 5 * 2 ** w.tries) * 1e3;
      w.status = "waiting";
      w.error = String(res.status);
      saveWrite(w);
      setTimeout(flushWrites, w.next - Date.now() + 50);
      told();
      return true;
    }
    // Refused: what was changed here goes back to what it was.
    w.status = "refused";
    w.error = String(answer?.error ?? answer?.message ?? res.status);
    undo(s, w);
    saveWrite(w);
    emitAll("refused", { source: s.name, id: w.rid, error: w.error });
    told();
    return true;
  }
  function undo(s, w) {
    const rec = s.recs.get(w.rid);
    if (w.kind === "create") put(s, w.rid, { s: "missing", v: null, at: Date.now() });
    else if (w.kind === "delete") put(s, w.rid, { ...(rec ?? {}), s: "ready", v: w.base, at: Date.now() });
    else if (rec) put(s, w.rid, { ...rec, v: { ...(rec.v ?? {}), ...w.base } });
    writeKept(s, [[w.rid, s.recs.get(w.rid)?.v ?? null]]);
  }
  // The server changed it too: `theirs` is what it has now (from the answer, else fetched).
  async function conflict(s, cfg, w, theirs, etag) {
    if (!theirs && !blank(s.cfg.one)) {
      try {
        const got = await getJson(String(s.cfg.one).replaceAll("{id}", encodeURIComponent(w.rid)), {}, s.cfg.headers);
        theirs = got.missing ? null : records(got.answer, s.cfg.pick);
        etag = got.etag ?? etag;
      } catch { /* settled without it */ }
    }
    const how = cfg.version ?? "etag", version = how === "etag" ? etag : isObj(theirs) ? at(theirs, how) ?? null : null;
    // Clashing again and again (the server keeps changing it): someone chooses.
    w.clashes = (w.clashes ?? 0) + 1;
    const policy = w.clashes > 3 ? "ask" : cfg.conflict ?? "merge";
    if (!isObj(theirs) || w.kind === "delete") {
      // Nothing to compare with (or a delete): the server's word stands, unless ours must.
      if (policy === "client-wins") return Object.assign(w, { status: "waiting", force: true }), saveWrite(w);
      dropWrite(w);
      put(s, w.rid, isObj(theirs) ? { s: "ready", v: theirs, at: Date.now(), etag } : { s: "missing", v: null, at: Date.now() });
      writeKept(s, [[w.rid, isObj(theirs) ? theirs : null, etag]]);
      emitAll("conflict", { source: s.name, id: w.rid, settled: "server" });
      return told();
    }
    const changed = Object.keys(w.changes);
    const both = changed.filter((f) => !same(theirs[f], w.base[f]) && !same(theirs[f], w.changes[f]));
    if (policy === "server-wins") {
      dropWrite(w);
      put(s, w.rid, { s: "ready", v: theirs, at: Date.now(), etag });
      writeKept(s, [[w.rid, theirs, etag]]);
      emitAll("conflict", { source: s.name, id: w.rid, settled: "server", fields: both });
      return told();
    }
    if (policy === "client-wins" || (policy === "merge" && !both.length)) {
      // Ours go on top of theirs, sent again on their version.
      Object.assign(w, { status: "waiting", version, base: Object.fromEntries(changed.map((f) => [f, theirs[f] ?? null])), error: null });
      put(s, w.rid, { ...s.recs.get(w.rid), v: { ...theirs, ...w.changes }, etag });
      saveWrite(w);
      emitAll("conflict", { source: s.name, id: w.rid, settled: "merged", fields: both });
      return told();
    }
    Object.assign(w, { status: "conflict", theirs, version, fields: (policy === "ask" ? changed.filter((f) => !same(theirs[f], w.changes[f])) : both).map((f) => ({ field: f, mine: w.changes[f] ?? null, theirs: theirs[f] ?? null, base: w.base[f] ?? null })) });
    put(s, w.rid, { ...s.recs.get(w.rid), etag });
    saveWrite(w);
    emitAll("conflict", { source: s.name, id: w.rid, settled: null, fields: w.fields.map((f) => f.field) });
    told();
  }
  // Someone chose: `choice` is mine, theirs, or `{ field: mine | theirs }`.
  function resolve(wid, choice) {
    const w = writes.find((x) => x.id === wid), s = w && sources.get(w.source);
    if (!w || !s) return;
    if (w.status === "refused") return dropWrite(w), put(s, w.rid, s.recs.get(w.rid) ?? { s: "missing", v: null, at: 0 }), told();
    if (w.status !== "conflict") return;
    const pick = (f) => (isObj(choice) ? choice[f] ?? "mine" : choice) === "theirs";
    const keep = Object.fromEntries(Object.entries(w.changes).filter(([f]) => !(w.fields.some((x) => x.field === f) && pick(f))));
    const theirs = w.theirs;
    delete w.theirs;
    delete w.fields;
    if (!Object.keys(keep).length) {
      dropWrite(w);
      put(s, w.rid, { ...s.recs.get(w.rid), s: "ready", v: theirs });
      writeKept(s, [[w.rid, theirs]]);
    } else {
      Object.assign(w, { changes: keep, base: Object.fromEntries(Object.keys(keep).map((f) => [f, theirs[f] ?? null])), status: "waiting", error: null });
      put(s, w.rid, { ...s.recs.get(w.rid), v: { ...theirs, ...keep } });
      writeKept(s, [[w.rid, { ...theirs, ...keep }]]);
      saveWrite(w);
    }
    told();
    flushWrites();
  }
  // Events on the sources that are on the page, for `on:` actions.
  const elements = new Map();
  const emitAll = (name, detail) => elements.get(detail.source)?.forEach((api) => api.emit(name, detail));

  // `submit` (and any fetch) to a source's write address is taken over.
  if (!self.fetch.__irenderData) {
    const before = self.fetch;
    const taking = async function (input, init = {}) {
      const isReq = input instanceof Request;
      const url = isReq ? input.url : String(input), method = String(init.method ?? (isReq ? input.method : "GET")).toUpperCase();
      const headers = new Headers(init.headers ?? (isReq ? input.headers : {}));
      const to = method !== "GET" && !headers.has("x-irender-data") ? writeOf(url, method) : null;
      if (!to) return before.call(self, input, init);
      let changes = {};
      const raw = init.body ?? (isReq ? await input.clone().text() : undefined);
      if (typeof raw === "string" && raw) {
        try {
          changes = JSON.parse(raw);
        } catch {
          return before.call(self, input, init);
        }
      }
      if (!isObj(changes)) return before.call(self, input, init);
      const out = await change(to.s, to.kind, to.id, changes);
      return new Response(JSON.stringify(out), { status: 202, headers: { "content-type": "application/json" } });
    };
    taking.__irenderData = true;
    taking.__irender = before.__irender;
    self.fetch = taking;
  }
  addEventListener("online", () => flushWrites());
  O?.vault.on((state) => state === "open" && [...new Set([...sources.values()].map((x) => x.scope))].forEach((sc) => loadWrites(sc)));

  window.__irenderData = {
    define,
    want,
    join: joinRow,
    reads,
    lookup,
    invalidate,
    retry,
    wipe,
    resolve,
    flush: flushWrites,
    writes: () => writes,
    onWrites: (f) => (wheard.add(f), () => wheard.delete(f)),
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
    let cleared = false, was, said;
    if (!elements.has(cfg.name)) elements.set(cfg.name, new Set());
    elements.get(cfg.name).add(api);
    // `bindWrites`: this source's changes waiting, being sent, in conflict or refused.
    const tell = () => {
      if (!cfg.bindWrites || !el.isConnected) return;
      const list = writes.filter((w) => w.source === cfg.name).map((w) => ({ id: w.id, record: w.rid, kind: w.kind, status: w.status, error: w.error, changes: w.changes, fields: w.fields ?? null, at: new Date(w.at).toISOString() }));
      const now = JSON.stringify(list);
      if (now !== said) (said = now), api.set(cfg.bindWrites, list);
    };
    wheard.add(tell);
    function update() {
      const scope = K.value(api, cfg.scope);
      define(cfg.name, cfg, scope);
      if (scope !== was) (was = scope), loadWrites(scope);
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
    let key = null, seq = 0, rows = null, one = false, total = null, error = null, loading = false, stamp = null, timer = 0, last, lastStatus;
    let cached = false, partial = false;
    // `sync`: what the rows shown were synced to, `{ base, cursor, fullAt }`, so that the
    // next load asks only for what changed since (the server's cursor, never this
    // device's clock), and every `full` hours for everything.
    const S = isObj(cfg.sync) ? cfg.sync : cfg.sync ? {} : null;
    let mark = null;
    const baseOf = (q) => JSON.stringify([q.url ?? "", q.params ?? null]);
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
      stamp = new Date(k.at).toISOString();
      mark = S ? { base: baseOf(q), cursor: k.cursor ?? null, fullAt: k.fullAt ?? 0 } : null;
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
      const base = baseOf(q);
      // What's in memory for this address is as new as what's kept, or newer.
      if (kept() && !(mark?.base === base && Array.isArray(rows) && !partial)) await fromDevice(q, id);
      if (id !== seq) return;
      publish();
      // Only onto rows that are whole: a partial answer is loaded again in full.
      const delta = !!S && !partial && mark?.base === base && !blank(mark.cursor) && Array.isArray(rows) && Date.now() - mark.fullAt < Number(S.full ?? 24) * 36e5;
      const p = params(q.params);
      if (delta) p.set(S.param ?? "since", String(mark.cursor));
      try {
        const { answer, missing } = await getJson(join(String(q.url ?? ""), p), {}, cfg.headers);
        if (id !== seq) return;
        const got = missing ? [] : records(answer, cfg.pick);
        if (delta && !missing) {
          rows = merge(rows, Array.isArray(got) ? got : [], at(answer, S.deleted ?? "deleted"));
          total = rows.length;
        } else {
          one = isObj(got);
          rows = one ? [got] : Array.isArray(got) ? got : [];
          total = Number.isFinite(answer?.total) ? answer.total : rows.length;
        }
        const cursor = S && !missing ? at(answer, S.cursor ?? "cursor") ?? null : null;
        mark = S ? { base, cursor, fullAt: delta && !missing ? mark.fullAt : Date.now() } : null;
        stamp = new Date().toISOString();
        loading = cached = partial = false;
        if (kept()) keep(cfg, rows, { total, one, cursor: mark?.cursor ?? null, fullAt: mark?.fullAt ?? 0 }).then((v) => O.db.put("records", qslot(q), v)).catch(() => {});
        api.emit("loaded", { rows, total, delta });
      } catch (e) {
        if (id !== seq) return;
        loading = false;
        error = String(e?.message ?? e);
        // Signed out on the server: the page decides what to do (`clear`, say).
        if (e?.status === 401) api.emit("unauthorized", {});
        api.emit("failed", { error });
      }
      publish();
    }
    // What changed since, into the rows: a changed row takes its old one's place, a new one
    // goes at the end; one whose `tombstone` field is true, or whose id is in `deleted`, goes.
    function merge(old, changed, deleted) {
      const key = S.key ?? "id", tomb = S.tombstone ?? "deleted";
      const out = new Map(old.map((r, i) => [blank(at(r, key)) ? `\u0000${i}` : idOf(at(r, key)), r]));
      for (const r of changed) {
        if (!isObj(r) || blank(at(r, key))) continue;
        const k = idOf(at(r, key));
        if (at(r, tomb) === true) out.delete(k);
        else out.set(k, r);
      }
      if (Array.isArray(deleted)) for (const d of deleted) out.delete(idOf(isObj(d) ? at(d, key) : d));
      return [...out.values()];
    }
    // Loaded again: every `every` seconds while the page is in sight, and when the
    // connection comes back.
    const again = () => {
      if (!el.isConnected || !key || cfg.source || loading || K.value(api, cfg.paused) || K.value(api, cfg.clear)) return;
      const q = JSON.parse(key);
      if (!blank(q.url)) load(q);
    };
    if (Number(cfg.every) > 0) {
      const tick = setInterval(() => (el.isConnected ? document.visibilityState === "visible" && again() : clearInterval(tick)), Math.max(5, Number(cfg.every)) * 1e3);
    }
    addEventListener("online", again);
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
        const st = { state: error ? (rows?.length ? "stale" : "error") : loading && !cached ? "loading" : pending ? "joining" : rows === null && !cfg.source ? "idle" : "ready", pending, total, error, at: stamp, cached };
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

  // Changes the server changed too, field by field (yours or theirs), and refused ones.
  B["ui-data-conflicts"] = (el, api) => {
    const cfg = api.config, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
    const make = K.make, body = D.body(el), picks = new Map();
    let was = null;
    const show = (v) => (v === null || v === undefined || v === "" ? "—" : typeof v === "object" ? JSON.stringify(v) : String(v));
    const label = (f) => (isObj(cfg.labels) && cfg.labels[f] != null ? String(cfg.labels[f]) : f);
    const nameOf = (w) => {
      const v = sources.get(w.source)?.recs.get(w.rid)?.v;
      return (cfg.nameField && isObj(v) && !blank(v[cfg.nameField]) ? String(v[cfg.nameField]) : null) ?? w.rid;
    };
    function draw() {
      const list = writes.filter((w) => (w.status === "conflict" || w.status === "refused") && (!cfg.source || w.source === cfg.source));
      const empty = K.value(api, cfg.emptyText);
      const now = JSON.stringify([list.map((w) => [w.id, w.status, w.fields, w.error]), empty]);
      if (now === was) return;
      was = now;
      el.hidden = !list.length && blank(empty);
      if (!list.length) return body.replaceChildren(blank(empty) ? "" : make("p", { "data-w-note": "" }, String(empty)));
      body.replaceChildren(...list.map((w) => {
        if (w.status === "refused")
          return make("div", { "data-w-card": "", "data-id": w.id },
            make("p", { "data-w-title": "" }, L("{name} wasn't saved", { name: nameOf(w) })),
            make("p", { "data-w-error": "" }, w.error ?? ""),
            make("div", { "data-w-row": "" }, D.btn("", { "data-dismiss": "", click: () => resolve(w.id) }, L("Dismiss"))));
        const chosen = picks.get(w.id) ?? {};
        const rows = (w.fields ?? []).map(({ field, mine, theirs }) => {
          const name = `dc-${w.id}-${field}`;
          const opt = (which, value) => make("label", null, make("input", { type: "radio", name, value: which, checked: (chosen[field] ?? "mine") === which, change: () => picks.set(w.id, { ...(picks.get(w.id) ?? {}), [field]: which }) }), make("span", null, show(value)), make("small", null, which === "mine" ? L("Yours") : L("Theirs")));
          return make("div", { "data-dc-field": field, role: "radiogroup", "aria-label": label(field) }, make("span", null, label(field)), opt("mine", mine), opt("theirs", theirs));
        });
        return make("div", { "data-w-card": "", "data-id": w.id },
          make("p", { "data-w-title": "" }, `${K.value(api, cfg.title) ?? L("Changed by someone else")}: ${nameOf(w)}`),
          make("p", { "data-w-note": "" }, L("They changed it while you did. Pick what to keep.")),
          ...rows,
          make("div", { "data-w-row": "" },
            D.btn("primary", { "data-keep": "", click: () => (resolve(w.id, picks.get(w.id) ?? "mine"), picks.delete(w.id)) }, L("Save")),
            D.btn("ghost", { "data-theirs": "", click: () => (resolve(w.id, "theirs"), picks.delete(w.id)) }, L("Keep theirs"))));
      }));
    }
    wheard.add(() => el.isConnected && draw());
    draw();
    return { update: draw };
  };
})();
