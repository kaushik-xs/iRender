// irender-ui-kit: `ui-install-prompt`, `ui-update-prompt`, `ui-offline-indicator`,
// `ui-offline-queue`, `ui-local-store` and `ui-app-shell`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, O = window.__irenderOffline, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const keep = {
    get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
    set: (k, v) => { try { localStorage.setItem(k, v); } catch { /* not kept */ } },
  };
  const say = (api, v, d) => K.value(api, v) ?? d;
  const bar = (floating, img, title, text, ...controls) => make("div", { "data-pwa-bar": "", "data-floating": floating, role: floating ? "status" : "group", "aria-label": floating ? null : title }, img, make("div", { "data-pwa-text": "" }, title ? make("strong", null, title) : null, text ? make("span", null, text) : null), controls);

  // ---------------------------------------------------------------- install
  // The browser's prompt, held until it's asked for.
  let offer = null, done = false;
  const told = new Set();
  addEventListener("beforeinstallprompt", (e) => {
    e.preventDefault();
    offer = e;
    told.forEach((f) => f());
  });
  addEventListener("appinstalled", () => {
    offer = null;
    done = true;
    told.forEach((f) => f("installed"));
  });
  const manual = () => D.ios || (D.safari && "serviceWorker" in navigator) || (D.android && D.firefox);
  const how = () =>
    D.ios ? [L("Tap Share in the browser's bar"), L("Choose Add to Home Screen"), L("Tap Add")]
    : D.safari ? [L("In the menu bar, open File"), L("Choose Add to Dock"), L("Click Add")]
    : D.android ? [L("Open the browser's menu"), L("Choose Install, or Add to Home screen"), L("Confirm it")]
    : [L("Look for the install icon at the end of the address bar, or open the browser's menu"), L("Choose Install, or Add to Dock or desktop")];
  B["ui-install-prompt"] = (el, api) => {
    const cfg = api.config, key = cfg.remember ? `irender-install:${cfg.remember}` : null;
    const put = (v) => cfg.bindState && api.state[cfg.bindState] !== v && api.set(cfg.bindState, v);
    const away = () => key && Number(keep.get(key)) > Date.now();
    async function ask() {
      if (!offer) return steps();
      const e = offer;
      offer = null;
      e.prompt();
      const { outcome } = await e.userChoice;
      api.emit(outcome === "accepted" ? "install" : "dismiss", {});
      draw();
    }
    function steps() {
      const d = make("dialog", { "aria-label": L("Install the app") }, make("h2", null, L("Install the app")), make("ol", null, how().map((s) => make("li", null, s))), make("div", { "data-w-row": "" }, btn("primary", { click: () => d.close() }, L("Close"))));
      d.addEventListener("close", () => d.remove());
      el.append(d);
      d.showModal();
      api.emit("steps", {});
    }
    let was = null;
    function draw() {
      const state = done || D.standalone() ? "installed" : offer ? "available" : manual() ? "steps" : "unavailable";
      put(state);
      const show = cfg.always || (!away() && (state === "available" || state === "steps"));
      const go = say(api, cfg.label, L("Install the app")), title = say(api, cfg.title, document.querySelector("meta[name=apple-mobile-web-app-title]")?.content ?? document.title), text = say(api, cfg.text, L("Opens from your home screen, and works offline.")), pic = K.value(api, cfg.icon);
      const now = JSON.stringify([show, go, title, text, pic]);
      if (now === was) return;
      was = now;
      const open = el.querySelector("dialog");
      el.replaceChildren(...(open ? [open] : []));
      if (!show) return;
      const b = btn("primary", { "data-install": "", click: ask }, icon("download", 15), go);
      if (cfg.variant !== "card" && cfg.variant !== "floating") return el.prepend(b);
      const later = key ? btn("ghost", { click: () => (keep.set(key, String(Date.now() + (cfg.days ?? 30) * 864e5)), api.emit("dismiss", {}), draw()) }, L("Not now")) : null;
      el.prepend(bar(cfg.variant === "floating", D.safeUrl(pic) ? make("img", { src: pic, alt: "" }) : null, title, text, later, b));
    }
    told.add((v) => el.isConnected && (v === "installed" && api.emit("installed", {}), draw()));
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- update
  B["ui-update-prompt"] = (el, api) => {
    const cfg = api.config;
    let waiting = null, later = false, reloading = false, was = null;
    const put = (v) => cfg.bind && !!api.state[cfg.bind] !== v && api.set(cfg.bind, v);
    function reload() {
      api.emit("reload", {});
      reloading = true;
      const go = () => location.reload();
      if (!waiting) return go();
      navigator.serviceWorker.addEventListener("controllerchange", go, { once: true });
      waiting.postMessage({ type: "skipWaiting" });
      // It may have been taken over already.
      setTimeout(go, 4000);
    }
    function draw() {
      const show = !!K.value(api, cfg.show) || (!!waiting && !later);
      const title = say(api, cfg.title, L("A new version is ready")), text = K.value(api, cfg.text), label = say(api, cfg.label, L("Reload"));
      const now = JSON.stringify([show, title, text, label]);
      if (now === was) return;
      was = now;
      el.replaceChildren();
      if (!show) return;
      const no = cfg.laterLabel === null ? null : btn("ghost", { click: () => ((later = true), api.emit("later", {}), draw()) }, cfg.laterLabel ?? L("Later"));
      el.append(bar(!cfg.inline, null, title, text, no, btn("primary", { "data-reload": "", click: reload }, icon("refresh", 15), label)));
    }
    const found = (w) => {
      if (reloading || waiting === w) return;
      waiting = w;
      later = false;
      put(true);
      api.emit("ready", {});
      draw();
    };
    if ("serviceWorker" in navigator && location.protocol !== "file:") {
      navigator.serviceWorker.getRegistration().then((reg) => {
        // Registered as the page loads, which this may be ahead of.
        if (!reg) return navigator.serviceWorker.ready;
        return reg;
      }).then((reg) => {
        if (reg.waiting && navigator.serviceWorker.controller) found(reg.waiting);
        reg.addEventListener("updatefound", () => {
          const w = reg.installing;
          // The first one isn't a new version.
          w?.addEventListener("statechange", () => w.state === "installed" && navigator.serviceWorker.controller && found(w));
        });
        const look = () => el.isConnected && navigator.onLine && reg.update().catch(() => {});
        document.addEventListener("visibilitychange", () => document.visibilityState === "visible" && look());
        setInterval(look, Math.max(1, cfg.every ?? 60) * 60000);
      }).catch(() => {});
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- offline indicator
  B["ui-offline-indicator"] = (el, api) => {
    const cfg = api.config;
    let state = navigator.onLine ? "online" : "offline", timer = 0, was = null;
    const tones = { online: "good", offline: "bad", reconnecting: "warn" };
    function to(s) {
      if (s !== state) {
        state = s;
        api.emit(s, {});
      }
      if (cfg.bind && api.state[cfg.bind] !== s) api.set(cfg.bind, s);
      draw();
    }
    async function check() {
      clearTimeout(timer);
      const url = K.value(api, cfg.checkUrl);
      if (!url) return to(navigator.onLine ? "online" : "offline");
      if (state !== "online" && navigator.onLine) to("reconnecting");
      try {
        await (self.__irenderFetch ?? fetch)(url, { method: "HEAD", cache: "no-store", signal: AbortSignal.timeout?.(8000) });
        to("online");
      } catch {
        to(navigator.onLine ? "reconnecting" : "offline");
        timer = setTimeout(check, Math.max(2, cfg.every ?? 15) * 1000);
      }
    }
    function draw() {
      const text = state === "offline" ? say(api, cfg.offlineText, L("Offline")) : state === "online" ? say(api, cfg.onlineText, L("Online")) : say(api, cfg.reconnectingText, L("Reconnecting…"));
      const show = cfg.variant !== "none" && (state !== "online" || cfg.showOnline), more = cfg.variant === "banner" && state !== "online" ? K.value(api, cfg.detail) : null;
      const now = JSON.stringify([show, state, text, more]);
      if (now === was) return;
      was = now;
      el.hidden = !show;
      el.replaceChildren(make("span", { "data-w-tag": "", "data-pwa-state": state, "data-tone": tones[state], "aria-label": cfg.variant === "dot" ? text : null }, make("i"), cfg.variant === "dot" ? null : make("strong", { style: "font-weight:inherit" }, text), more ? make("span", null, more) : null));
    }
    addEventListener("online", () => el.isConnected && check());
    addEventListener("offline", () => el.isConnected && check());
    document.addEventListener("visibilitychange", () => el.isConnected && document.visibilityState === "visible" && check());
    to(state);
    check();
    return { update: () => (draw(), cfg.bind && api.state[cfg.bind] !== state && api.set(cfg.bind, state)) };
  };

  // ---------------------------------------------------------------- offline queue
  // The requests a page makes go through here while a queue is on it: one that can't
  // reach the server is kept, and answered as accepted.
  const queues = new Set();
  const under = (url, p) => {
    try {
      return url.startsWith(new URL(p, location.href).href);
    } catch {
      return false;
    }
  };
  const taker = (url, method) => [...queues].find((q) => q.el.isConnected && q.takes(url, method));
  self.__irenderFetch ??= self.fetch.bind(self);
  async function add(q, url, method, headers, body) {
    const item = { url, method, headers, label: q.name(url, body) };
    if (body instanceof FormData) item.form = [...body].map(([k, v]) => [k, v, v instanceof File ? v.name : undefined]);
    else item.body = body;
    const kept = await O.queue.add(item);
    q.api.emit("queued", { id: kept.id, label: kept.label, url });
    return kept;
  }
  // For uploads, which aren't fetched: true once it's been kept.
  window.__irenderQueue = async (url, method, body) => {
    const full = new URL(url, location.href).href, q = taker(full, method);
    return q ? (await add(q, full, method, {}, body)).id : null;
  };
  if (!self.fetch.__irender) {
    const through = async (input, init = {}) => {
      const isReq = input instanceof Request, url = new URL(isReq ? input.url : String(input), location.href).href, method = (init.method ?? (isReq ? input.method : "GET")).toUpperCase();
      const q = taker(url, method);
      if (!q) return self.__irenderFetch(input, init);
      const headers = Object.fromEntries(new Headers(init.headers ?? (isReq ? input.headers : {})));
      let body = init.body ?? (isReq ? await input.clone().blob() : undefined);
      try {
        if (!navigator.onLine) throw new TypeError("offline");
        return await self.__irenderFetch(input, init);
      } catch (e) {
        // Stopped on purpose, or something that can't be sent again.
        if (e?.name === "AbortError" || body instanceof ReadableStream) throw e;
        if (body instanceof URLSearchParams) (body = String(body)), (headers["content-type"] ??= "application/x-www-form-urlencoded;charset=UTF-8");
        const kept = await add(q, url, method, headers, body);
        return new Response(JSON.stringify({ queued: true, id: kept.id }), { status: 202, headers: { "content-type": "application/json" } });
      }
    };
    through.__irender = true;
    self.fetch = through;
  }
  B["ui-offline-queue"] = (el, api) => {
    const cfg = api.config, methods = (cfg.methods ?? ["POST", "PUT", "PATCH", "DELETE"]).map((m) => String(m).toUpperCase());
    const paths = [cfg.paths ?? []].flat().map(String), labels = isObj(cfg.labels) ? cfg.labels : {};
    const body = D.body(el);
    let list = [], was = null;
    const q = {
      el, api,
      takes: (url, method) => methods.includes(method) && (paths.length ? paths.some((p) => under(url, p)) : new URL(url).origin === location.origin),
      name(url, body) {
        const path = Object.keys(labels).find((p) => under(url, p));
        let which = null;
        if (cfg.labelField) try {
          which = body instanceof FormData ? body.get(cfg.labelField) : JSON.parse(body)?.[cfg.labelField];
        } catch { /* no name of its own */ }
        return [path ? labels[path] : path === undefined && !which ? new URL(url).pathname : null, blank(which) || typeof which === "object" ? null : String(which)].filter(Boolean).join(": ");
      },
    };
    queues.add(q);
    const flush = () => el.isConnected && navigator.onLine && O.queue.flush();
    async function read() {
      list = await O.queue.list();
      const view = list.map(({ id, label, url, method, at, status, error, tries }) => ({ id, label, url, method, at: new Date(at).toISOString(), status, error, tries }));
      if (cfg.bind && JSON.stringify(api.state[cfg.bind] ?? null) !== JSON.stringify(view)) api.set(cfg.bind, view);
      if (cfg.bindCount && api.state[cfg.bindCount] !== view.length) api.set(cfg.bindCount, view.length);
      draw();
    }
    const status = { waiting: () => [L("Waiting"), "info"], sending: () => [L("Sending…"), "warn"], failed: () => [L("Refused"), "bad"] };
    function draw() {
      const n = list.length, show = cfg.variant !== "none" && (n > 0 || !cfg.hideEmpty);
      const title = say(api, cfg.title, L("Waiting to be sent")), none = say(api, cfg.emptyText, L("Everything has been sent"));
      const now = JSON.stringify([show, title, none, list.map((x) => [x.id, x.status, x.error, x.tries])]);
      if (now === was) return;
      was = now;
      el.hidden = !show;
      if (!show) return body.replaceChildren();
      if (cfg.variant === "badge") return body.replaceChildren(make("span", { "data-w-tag": "", "data-tone": list.some((x) => x.status === "failed") ? "bad" : n ? "info" : "good", title: n ? title : none }, icon(n ? "cloud" : "check", 14), n ? L("{n} waiting", { n }) : L("All sent")));
      const rows = list.map((x) => {
        const [word, tone] = (status[x.status] ?? status.waiting)();
        return make("li", { "data-id": x.id },
          make("div", { "data-w-main": "" }, make("span", null, x.label || new URL(x.url).pathname), make("small", null, [K.ago(x.at), x.error].filter(Boolean).join(" · "))),
          make("span", { "data-w-tag": "", "data-tone": tone }, word),
          x.status === "failed" ? btn("ghost", { "data-retry": "", "aria-label": L("Send {name} again", { name: x.label }), title: L("Send again"), "data-icon-only": "", click: () => O.queue.retry(x.id) }, icon("refresh", 15)) : null,
          btn("ghost", { "data-drop": "", "aria-label": L("Don't send {name}", { name: x.label }), title: L("Don't send"), "data-icon-only": "", disabled: x.status === "sending", click: () => drop(x) }, icon("trash", 15)));
      });
      body.replaceChildren(
        make("div", { "data-w-row": "" }, make("p", { "data-w-title": "" }, n ? `${title} (${n})` : title), make("span", { "data-w-push": "" }), n ? btn("", { "data-send": "", click: () => O.queue.retry().then((s) => s || K.say(el, L("Still waiting for a connection"))) }, L("Send now")) : null),
        n ? make("ul", { "data-w-list": "" }, rows) : make("p", { "data-w-note": "" }, none));
    }
    function drop(x) {
      // Asked twice, as it's gone for good.
      const b = el.querySelector(`[data-id="${CSS.escape(x.id)}"] [data-drop]`);
      if (!b.hasAttribute("data-sure")) {
        b.setAttribute("data-sure", "");
        b.setAttribute("data-w-btn", "danger");
        b.setAttribute("aria-label", L("Press again to delete it"));
        K.say(el, L("Press again to delete it"));
        setTimeout(() => ((was = null), el.isConnected && draw()), 4000);
        return;
      }
      O.queue.remove(x.id);
    }
    O.on(async (m) => {
      if (!el.isConnected) return;
      if (m.type === "queue") read();
      else if (m.type === "sent") {
        api.emit("sent", m);
        K.say(el, L("{name} was sent", { name: m.label }));
        if (!(await O.queue.list()).length) api.emit("drained", {});
      } else if (m.type === "refused") api.emit("refused", m);
    });
    addEventListener("online", flush);
    document.addEventListener("visibilitychange", () => document.visibilityState === "visible" && flush());
    const t = setInterval(() => (el.isConnected ? list.length && O.queue.flush() : clearInterval(t)), Math.max(5, cfg.retryEvery ?? 60) * 1000);
    read().then(() => list.length && flush());
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- local store
  B["ui-local-store"] = (el, api) => {
    const cfg = api.config, name = `store:${cfg.name ?? location.pathname}`, session = cfg.where === "session";
    const me = O.id();
    let ready = false, timer = 0, cleared = false;
    const last = new Map();
    const text = (k) => JSON.stringify(api.state[k] ?? null);
    const now = () => Object.fromEntries(cfg.keys.map((k) => [k, api.state[k] ?? null]));
    async function get() {
      if (!session) return O.db.get("kv", name);
      try {
        return JSON.parse(sessionStorage.getItem(`irender-${name}`));
      } catch {
        return null;
      }
    }
    const put = async (v) => {
      if (!session) return v ? O.db.put("kv", name, v) : O.db.del("kv", name);
      try {
        v ? sessionStorage.setItem(`irender-${name}`, JSON.stringify(v)) : sessionStorage.removeItem(`irender-${name}`);
      } catch { /* full, or not allowed */ }
    };
    async function load() {
      let kept = null;
      try {
        kept = await get();
      } catch { /* starts afresh */ }
      const old = kept && ((cfg.maxAge && Date.now() - kept.at > cfg.maxAge * 864e5) || (cfg.version !== undefined && JSON.stringify(kept.version ?? null) !== JSON.stringify(cfg.version)));
      if (old) await put(null);
      else if (kept) for (const k of cfg.keys) if (k in kept.values && JSON.stringify(kept.values[k]) !== text(k)) api.set(k, kept.values[k]);
      for (const k of cfg.keys) last.set(k, text(k));
      if (cfg.bindSaved && kept && !old) api.set(cfg.bindSaved, new Date(kept.at).toISOString());
      if (!ready) {
        ready = true;
        if (cfg.bindReady) api.set(cfg.bindReady, true);
        api.emit("ready", { restored: !!kept && !old });
      }
    }
    async function save() {
      clearTimeout(timer);
      timer = 0;
      const at = Date.now();
      for (const k of cfg.keys) last.set(k, text(k));
      try {
        await put({ at, version: cfg.version, values: now() });
      } catch (e) {
        return api.emit("failed", { error: String(e?.message ?? e) });
      }
      if (cfg.bindSaved) api.set(cfg.bindSaved, new Date(at).toISOString());
      if (cfg.sync && !session) O.tell({ type: "store", name, from: me });
    }
    O.on((m) => m.type === "store" && m.name === name && m.from !== me && cfg.sync && el.isConnected && load());
    const flush = () => timer && save();
    document.addEventListener("visibilitychange", () => document.visibilityState === "hidden" && flush());
    addEventListener("pagehide", flush);
    load();
    return {
      update() {
        if (!ready) return;
        const clear = !!K.value(api, cfg.clear);
        if (clear && !cleared) {
          clearTimeout(timer);
          timer = 0;
          put(null);
          for (const k of cfg.keys) last.set(k, text(k));
          api.emit("cleared", {});
        }
        cleared = clear;
        if (clear || !cfg.keys.some((k) => last.get(k) !== text(k))) return;
        clearTimeout(timer);
        timer = setTimeout(save, 300);
      },
    };
  };

  // ---------------------------------------------------------------- app shell
  B["ui-app-shell"] = (el, api) => {
    const cfg = api.config, main = el.querySelector("[data-app-main]"), bind = cfg.bind;
    const at = () => (main.querySelector(`:scope > [data-app-screen="${CSS.escape(String(api.state[bind] ?? ""))}"]`) ? api.state[bind] : cfg.first);
    const screen = (v) => main.querySelector(`:scope > [data-app-screen="${CSS.escape(String(v))}"]`);
    let was = at(), popped = false;
    // Where each screen had been scrolled to: a hidden one forgets.
    const tops = new Map();
    main.addEventListener("scroll", (e) => e.target.matches("[data-app-screen]") && !e.target.hidden && tops.set(e.target.getAttribute("data-app-screen"), e.target.scrollTop), true);
    if (cfg.history) {
      history.replaceState({ ...history.state, irenderScreen: was }, "");
      addEventListener("popstate", (e) => {
        const v = e.state?.irenderScreen;
        if (!el.isConnected || v === undefined || v === at()) return;
        popped = true;
        api.set(bind, v);
      });
    }
    let moved = [];
    function update() {
      const v = at();
      if (v === was) return;
      const from = was;
      was = v;
      const s = screen(v);
      // Gone into afresh, a screen under another starts at its top.
      if (s) requestAnimationFrame(() => (s.scrollTop = cfg.parents[v] === from ? 0 : tops.get(v) ?? 0));
      if (cfg.history && !popped) {
        if (cfg.parents[from] === v && moved.at(-1) === v) moved.pop(), history.back();
        else moved.push(from), history.pushState({ irenderScreen: v }, "");
      } else if (popped) moved.pop();
      popped = false;
      api.emit("screen", { value: v, from });
    }
    // A tab pressed again goes to its screen's top.
    el.addEventListener("click", (e) => {
      const t = e.target.closest("[data-app-tab]");
      if (t && t.getAttribute("data-app-tab") === was) screen(was)?.scrollTo({ top: 0, behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
    }, true);
    if (cfg.pullToRefresh) {
      const FAR = 64, spin = make("div", { "data-app-pull": "", "aria-hidden": "true" }, icon("refresh", 18));
      main.append(spin);
      let pull = null, busy = false;
      const place = (d) => {
        spin.style.opacity = d > 8 ? "1" : "0";
        spin.style.transform = `translateY(${d}px) rotate(${d * 3}deg)`;
      };
      const end = () => {
        busy = false;
        spin.removeAttribute("data-on");
        spin.style.transition = "transform .2s, opacity .2s";
        place(0);
      };
      main.addEventListener("touchstart", (e) => {
        const s = e.target.closest("[data-app-screen]");
        pull = !busy && e.touches.length === 1 && s && s.scrollTop <= 0 ? { x: e.touches[0].clientX, y: e.touches[0].clientY, d: 0 } : null;
      }, { passive: true });
      main.addEventListener("touchmove", (e) => {
        if (!pull) return;
        const dy = e.touches[0].clientY - pull.y, dx = Math.abs(e.touches[0].clientX - pull.x);
        if (pull.d === 0 && (dy <= 4 || dx > dy)) return void (dy < -4 || dx > 8) && (pull = null);
        e.preventDefault();
        spin.style.transition = "none";
        // Harder the further it goes.
        place((pull.d = Math.max(1, Math.min(FAR * 1.4, dy * 0.5))));
      }, { passive: false });
      const up = () => {
        if (!pull) return;
        const far = pull.d >= FAR;
        pull = null;
        if (!far) return end();
        busy = true;
        spin.style.transition = "transform .2s";
        place(FAR * 0.6);
        spin.setAttribute("data-on", "");
        if (cfg.bindRefreshing) api.set(cfg.bindRefreshing, true);
        else setTimeout(end, 900);
        api.emit("refresh", { screen: was });
      };
      main.addEventListener("touchend", up);
      main.addEventListener("touchcancel", up);
      const follow = update;
      update = () => (follow(), cfg.bindRefreshing && busy && !api.state[cfg.bindRefreshing] && end());
    }
    return { update: () => update() };
  };
})();
