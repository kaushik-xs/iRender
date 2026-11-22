// The service worker of an iRender app, written by `irender app` after `offline.js` and
// `APP` (the app's settings and the files it keeps). Pages come from the network when it
// answers in time, else from what's kept; the app's own files, from what's kept; other
// requests on the site, from what's kept while a newer one is fetched. Nothing under
// `skip`, and nothing that isn't a GET, is kept.
const KEPT = `irender-${APP.version}`, SEEN = "irender-seen", WAIT = 4000;
const scope = new URL(self.registration.scope);
const at = (u) => new URL(u, scope).href;
const kept = new Set(APP.files.map(at)), also = new Set(APP.also);

self.addEventListener("install", (e) => e.waitUntil((async () => {
  const cache = await caches.open(KEPT);
  const add = async (u, init) => {
    try {
      const res = await fetch(u, init);
      if (res.ok) await cache.put(u, res);
    } catch { /* kept when it's next asked for */ }
  };
  // A few at a time, past the browser's own cache.
  const list = [...[...kept].map((u) => [u, { cache: "reload" }]), ...[...also].map((u) => [u, { mode: "cors", credentials: "omit" }])];
  await Promise.all(Array.from({ length: 6 }, async () => {
    for (let next; (next = list.shift()); ) await add(...next);
  }));
})()));
self.addEventListener("activate", (e) => e.waitUntil((async () => {
  for (const name of await caches.keys()) if (name.startsWith("irender-") && name !== KEPT && name !== SEEN) await caches.delete(name);
  await self.clients.claim();
})()));
self.addEventListener("message", (e) => {
  if (e.data?.type === "skipWaiting") self.skipWaiting();
  else if (e.data?.type === "flush") e.waitUntil(__irenderOffline.queue.flush());
  else if (e.data?.type === "version") e.ports[0]?.postMessage({ version: APP.version, name: APP.name });
});

const find = async (req) => (await (await caches.open(KEPT)).match(req, { ignoreSearch: req.mode === "navigate" })) ?? (await (await caches.open(SEEN)).match(req));
const keep = async (req, res) => {
  if (res.ok && res.type !== "opaque") await (await caches.open(SEEN)).put(req, res.clone()).catch(() => {});
  return res;
};
async function page(req) {
  const net = fetch(req).then((res) => keep(req, res));
  net.catch(() => {});
  try {
    return await Promise.race([net, new Promise((_, no) => setTimeout(() => no(new Error("slow")), WAIT))]);
  } catch {
    const have = (await find(req)) ?? (req.url.endsWith("/") ? await caches.match(at(new URL(req.url).pathname + "index.html")) : null);
    if (have) return have;
    try {
      return await net;
    } catch {
      return (APP.fallback && (await caches.match(at(APP.fallback)))) || new Response("<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><title>Offline</title><body style='font:16px system-ui;display:grid;place-items:center;min-height:90vh;margin:0'><p>You're offline, and this page isn't kept on this device.</p>", { status: 503, headers: { "content-type": "text/html; charset=utf-8" } });
    }
  }
}
// What another app shared: kept for the page that takes it.
async function shared(req) {
  const form = await req.formData(), id = __irenderOffline.id(), files = [];
  for (const f of form.getAll("files")) if (f instanceof File && f.size) files.push({ name: f.name, type: f.type, size: f.size, blob: f });
  const text = (k) => { const v = form.get(k); return typeof v === "string" ? v : ""; };
  await __irenderOffline.db.put("shared", id, { id, title: text("title"), text: text("text"), url: text("url"), files, at: Date.now() });
  const to = new URL(APP.share, scope);
  to.searchParams.set("shared", id);
  return Response.redirect(to.href, 303);
}
self.addEventListener("fetch", (e) => {
  const req = e.request, url = new URL(req.url);
  if (req.method === "POST" && APP.share && url.href.split("?")[0] === at(APP.share)) return e.respondWith(shared(req));
  if (req.method !== "GET" || req.headers.has("range")) return;
  if (also.has(req.url)) return e.respondWith(find(req).then((r) => r ?? fetch(req)));
  if (url.origin !== location.origin || !url.href.startsWith(scope.href)) return;
  const path = url.pathname;
  if (APP.skip.some((s) => path.startsWith(s.startsWith("/") ? s : new URL(s, scope).pathname)) || /\/sw\.js$/.test(path) || (req.headers.get("accept") ?? "").includes("event-stream")) return;
  if (req.mode === "navigate") return e.respondWith(page(req));
  if (kept.has(url.href.split("#")[0])) return e.respondWith(find(req).then((r) => r ?? fetch(req)));
  e.respondWith((async () => {
    const have = await find(req), net = fetch(req).then((res) => keep(req, res));
    if (!have) return net.catch(() => Response.error());
    e.waitUntil(net.catch(() => {}));
    return have;
  })());
});

// The queue, when the connection's back.
self.addEventListener("sync", (e) => e.tag === "irender-queue" && e.waitUntil(__irenderOffline.queue.flush()));

// A push: { title, body, url, icon, image, tag, actions, count, silent, data }, as JSON or text.
self.addEventListener("push", (e) => {
  let d = {};
  try {
    d = e.data?.json() ?? {};
  } catch {
    d = { body: e.data?.text() };
  }
  const icon = d.icon ?? (APP.icon ? at(APP.icon) : undefined);
  e.waitUntil(Promise.all([
    self.registration.showNotification(d.title ?? APP.name, { body: d.body, icon, badge: d.badge ?? icon, image: d.image, tag: d.tag, renotify: !!(d.tag && d.renotify), requireInteraction: !!d.sticky, silent: !!d.silent, actions: d.actions, data: { url: d.url, ...(d.data ?? {}), actions: d.actions }, timestamp: d.at ? Date.parse(d.at) : undefined }),
    typeof d.count === "number" ? self.navigator.setAppBadge?.(d.count)?.catch(() => {}) : null,
    self.clients.matchAll({ type: "window" }).then((cs) => cs.forEach((c) => c.postMessage({ type: "push", data: d }))),
  ]));
});
self.addEventListener("notificationclick", (e) => {
  e.notification.close();
  const data = e.notification.data ?? {}, action = (data.actions ?? []).find((a) => a.action === e.action);
  const url = at(action?.url ?? data.url ?? APP.start);
  e.waitUntil((async () => {
    const open = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    for (const c of open) c.postMessage({ type: "notification", action: e.action || null, data });
    const same = open.find((c) => c.url === url) ?? open.find((c) => new URL(c.url).origin === new URL(url).origin);
    if (same) {
      await same.focus?.();
      if (same.url !== url) await same.navigate?.(url).catch(() => self.clients.openWindow(url));
    } else await self.clients.openWindow(url);
  })());
});
