// irender-ui-kit: the `ui-iframe` behaviour. The frame is `loading` until its page has
// loaded, then `ready`; past `timeout` seconds without loading it's `failed` (a page that
// refuses to be framed often loads blank, which nothing outside the frame can tell). The
// status goes to `bindStatus`, with `load` and `failed` events. With `height: auto` it's
// as tall as the page says: a message `{ height }` from it (or, from this site, its size
// measured). Messages from its page go to `bindMessage` and fire `message`; `send` is
// posted to it, to its own origin only (any, sandboxed without `same-origin`, which
// gives it none), each time it changes.
(window.__irenderBehaviors ||= {})["ui-iframe"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit;
  let status = null, timer = 0, sent, observer = null;
  const origin = () => {
    try {
      return new URL(el.src, location.href).origin;
    } catch {
      return null;
    }
  };
  function set(s) {
    if (s === status) return;
    status = s;
    el.toggleAttribute("data-loading", s === "loading");
    el.toggleAttribute("data-failed", s === "failed");
    el.setAttribute("aria-busy", String(s === "loading"));
    if (cfg.bindStatus) api.set(cfg.bindStatus, s);
    if (s !== "loading") api.emit(s === "ready" ? "load" : "failed", { src: el.src });
  }
  function start() {
    clearTimeout(timer);
    observer?.disconnect();
    set("loading");
    const secs = Number(cfg.timeout ?? 20);
    if (secs > 0) timer = setTimeout(() => status === "loading" && set("failed"), secs * 1e3);
  }
  const fit = (h) => {
    const n = Math.ceil(Number(h));
    if (cfg.auto && n > 0 && n < 1e5) el.style.height = `${Math.max(Number(cfg.minHeight ?? 0), n)}px`;
  };
  el.addEventListener("load", () => {
    // An empty frame (no `src` yet, or about:blank) hasn't loaded anything.
    if (!el.getAttribute("src") || el.src === "about:blank") return;
    clearTimeout(timer);
    set("ready");
    // A page from this site is measured; another one says how tall it is.
    let doc = null;
    try {
      doc = el.contentDocument;
    } catch { /* another site's */ }
    if (cfg.auto && doc?.documentElement && typeof ResizeObserver === "function") {
      observer = new ResizeObserver(() => fit(doc.documentElement.scrollHeight));
      observer.observe(doc.documentElement);
    }
    sent = undefined;
    update();
  });
  addEventListener("message", (e) => {
    if (!el.isConnected || e.source !== el.contentWindow) return;
    const d = e.data;
    if (d && typeof d === "object" && Number.isFinite(Number(d.height)) && (d.type == null || d.type === "resize" || d.type === "irender:height")) fit(d.height);
    if (cfg.bindMessage) api.set(cfg.bindMessage, d);
    api.emit("message", { data: d, origin: e.origin });
  });
  // `src` changing (it may follow state) loads again.
  new MutationObserver(() => start()).observe(el, { attributes: true, attributeFilter: ["src"] });
  function update() {
    if (!("send" in cfg) || status !== "ready") return;
    const v = K.value(api, cfg.send), s = JSON.stringify(v ?? null);
    if (s === sent || v === undefined || v === null) return;
    sent = s;
    // A page sandboxed without `same-origin` has an origin of its own (none to name).
    const opaque = el.hasAttribute("sandbox") && !el.sandbox.contains("allow-same-origin");
    const to = opaque ? "*" : origin();
    if (to && to !== "null") el.contentWindow?.postMessage(v, to);
  }
  if (el.getAttribute("src")) start();
  return { update };
};
