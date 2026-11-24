// irender-ui-kit: what's kept on the device (`self.__irenderOffline`), in a page and in the
// service worker alike: a key and value store in IndexedDB, and the queue of requests
// waiting to be sent. Without IndexedDB (a private window, say) it keeps to memory.
(() => {
  if (self.__irenderOffline) return;
  const STORES = ["kv", "queue", "shared"];
  let opening = null;
  const open = () => (opening ??= new Promise((ok) => {
    let req;
    try {
      req = indexedDB.open("irender", 1);
    } catch {
      return ok(null);
    }
    req.onupgradeneeded = () => STORES.forEach((s) => req.result.objectStoreNames.contains(s) || req.result.createObjectStore(s));
    req.onsuccess = () => {
      // Another tab has a newer one: let it go on.
      req.result.onversionchange = () => (req.result.close(), (opening = null));
      ok(req.result);
    };
    req.onerror = req.onblocked = () => ok(null);
  }));
  const memory = Object.fromEntries(STORES.map((s) => [s, new Map()]));
  async function run(store, mode, f) {
    const db = await open();
    if (!db) return f(null);
    return new Promise((ok, no) => {
      const tx = db.transaction(store, mode), req = f(tx.objectStore(store));
      tx.oncomplete = () => ok(req?.result);
      tx.onerror = tx.onabort = () => no(tx.error ?? new Error("storage failed"));
    });
  }
  const db = {
    get: (store, key) => run(store, "readonly", (s) => (s ? s.get(key) : memory[store].get(key))),
    put: (store, key, value) => run(store, "readwrite", (s) => (s ? s.put(value, key) : memory[store].set(key, value) && undefined)),
    del: (store, key) => run(store, "readwrite", (s) => (s ? s.delete(key) : memory[store].delete(key) && undefined)),
    clear: (store) => run(store, "readwrite", (s) => (s ? s.clear() : memory[store].clear())),
    all: (store) => run(store, "readonly", (s) => (s ? s.getAll() : [...memory[store].values()])),
  };
  const id = () => self.crypto?.randomUUID?.() ?? `q${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
  // Other tabs, and the worker, hear of changes.
  const listeners = new Set();
  const channel = "BroadcastChannel" in self ? new BroadcastChannel("irender-offline") : null;
  channel?.addEventListener("message", (e) => listeners.forEach((f) => f(e.data)));
  const tell = (message) => {
    channel?.postMessage(message);
    listeners.forEach((f) => f(message));
  };

  // ---- the queue: { id, url, method, headers, body | form, label, at, tries, next, status, error }
  // status: waiting, sending, failed (the server refused it: it isn't sent again by itself)
  const queue = {
    list: async () => (await db.all("queue")).sort((a, b) => a.at - b.at),
    async add(item) {
      const q = { method: "POST", headers: {}, label: "", ...item, id: item.id ?? id(), at: Date.now(), tries: 0, next: 0, status: "waiting", error: null };
      await db.put("queue", q.id, q);
      tell({ type: "queue" });
      // Sent by the browser when the connection's back, even with the page closed, where it
      // can. Not waited for: `ready` never settles on a page without a service worker.
      self.navigator?.serviceWorker?.ready?.then((r) => r.sync?.register("irender-queue")).catch(() => { /* sent when the app is next open */ });
      return q;
    },
    async remove(qid) {
      await db.del("queue", qid);
      tell({ type: "queue" });
    },
    async retry(qid) {
      for (const q of await queue.list()) if (qid ? q.id === qid : q.status === "failed") await db.put("queue", q.id, { ...q, status: "waiting", next: 0, error: null });
      tell({ type: "queue" });
      return queue.flush();
    },
    // Sends what's waiting, in order, one at a time across tabs. Stops when there's no
    // connection; a refusal fails that one and goes on.
    flush: () => {
      const go = async () => {
        let sent = 0;
        for (const q of await queue.list()) {
          if (q.status === "failed" || q.next > Date.now()) continue;
          await db.put("queue", q.id, { ...q, status: "sending" });
          tell({ type: "queue" });
          let body = q.body;
          if (q.form) {
            body = new FormData();
            for (const [k, v, name] of q.form) v instanceof Blob ? body.append(k, v, name ?? "file") : body.append(k, v);
          }
          let res;
          try {
            res = await (self.__irenderFetch ?? fetch)(q.url, { method: q.method, headers: { ...q.headers, "x-queued-id": q.id }, body, credentials: "same-origin" });
          } catch {
            await db.put("queue", q.id, { ...q, status: "waiting" });
            break;
          }
          if (res.ok) {
            let data = null;
            try {
              data = (res.headers.get("content-type") ?? "").includes("json") ? await res.json() : null;
            } catch { /* nothing to hand back */ }
            await db.del("queue", q.id);
            sent++;
            tell({ type: "sent", id: q.id, label: q.label, url: q.url, data });
          } else if (res.status >= 500 || res.status === 429 || res.status === 408) {
            // Tried again later, longer each time, up to an hour.
            const tries = q.tries + 1, wait = Math.min(3600, 5 * 2 ** tries) * 1000;
            await db.put("queue", q.id, { ...q, status: "waiting", tries, next: Date.now() + wait, error: `${res.status} ${res.statusText}`.trim() });
          } else {
            let error = `${res.status} ${res.statusText}`.trim();
            try {
              const d = await res.clone().json();
              error = String(d.error ?? d.message ?? error);
            } catch { /* the status says it */ }
            await db.put("queue", q.id, { ...q, status: "failed", tries: q.tries + 1, error });
            tell({ type: "refused", id: q.id, label: q.label, error });
          }
        }
        tell({ type: "queue" });
        return sent;
      };
      return self.navigator?.locks ? navigator.locks.request("irender-queue", { ifAvailable: true }, (l) => (l ? go() : 0)) : go();
    },
  };
  self.__irenderOffline = { db, queue, id, tell, on: (f) => (listeners.add(f), () => listeners.delete(f)) };
})();
