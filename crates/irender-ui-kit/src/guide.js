// irender-ui-kit: `ui-tour`, `ui-banner` and `ui-session-timeout`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;
  const store = {
    get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
    set: (k, v) => { try { localStorage.setItem(k, v); } catch { /* not kept */ } },
  };

  // ---------------------------------------------------------------- tour
  B["ui-tour"] = (el, api) => {
    const cfg = api.config, key = cfg.id ? `irender-tour:${cfg.id}` : null;
    const steps = () => (K.value(api, cfg.steps) ?? []).filter(isObj);
    let at = null, ui = null, from = null, frame = 0;
    const set = (i) => api.set(cfg.bind, i);
    function finish(done) {
      if (key) store.set(key, done ? "done" : "skipped");
      api.emit(done ? "finish" : "skip", { step: at });
      set(null);
    }
    function build() {
      const card = make("div", { "data-tour-card": "", role: "dialog", "aria-modal": cfg.interactive ? null : "true", tabindex: -1 });
      const cover = make("div", { "data-tour-cover": "" }), hole = make("div", { "data-tour-hole": "", "data-plain": !cfg.backdrop });
      el.append(cover, hole, card);
      ui = { card, cover, hole };
      cover.hidden = cfg.interactive;
      card.addEventListener("keydown", (e) => {
        const k = K.rtl(el) ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[e.key] ?? e.key : e.key;
        if (e.key === "Escape") finish(false);
        else if (k === "ArrowRight") next();
        else if (k === "ArrowLeft" && at > 0) set(at - 1);
        else if (e.key === "Tab" && !cfg.interactive) {
          // Kept within the card.
          const bs = [...card.querySelectorAll("button")], i = bs.indexOf(document.activeElement);
          if (e.shiftKey ? i > 0 : i >= 0 && i < bs.length - 1) return;
          bs[e.shiftKey ? bs.length - 1 : 0].focus();
        } else return;
        e.preventDefault();
      });
    }
    const next = () => (at >= steps().length - 1 ? finish(true) : set(at + 1));
    const target = (s) => {
      try {
        const t = s.target ? document.querySelector(s.target) : null;
        return t && t.getClientRects().length ? t : null;
      } catch {
        return null;
      }
    };
    function show(i) {
      const list = steps(), s = list[i];
      if (!s) return hide();
      if (!ui) build();
      if (at === null) from = document.activeElement;
      at = i;
      if (isObj(s.set)) for (const [k, v] of Object.entries(s.set)) if (JSON.stringify(api.state[k]) !== JSON.stringify(v)) api.set(k, v);
      const id = `tour-${K.id()}`;
      const { card } = ui;
      card.setAttribute("aria-labelledby", id);
      card.replaceChildren(...[
        s.image && /^(https?:|\/|\.{0,2}\/|[\w-]+[/.])/i.test(s.image) ? make("img", { src: s.image, alt: "" }) : null,
        make("h2", { id }, s.title ?? ""), s.text ? make("p", null, s.text) : null,
        make("div", { "data-tour-foot": "" }, make("small", null, L("{n} of {count}", { n: i + 1, count: list.length })), i > 0 ? button({ click: () => set(i - 1) }, L("Back")) : null, button({ "data-next": "", click: next }, i === list.length - 1 ? K.value(api, cfg.doneLabel) ?? L("Done") : L("Next"))),
        button({ "data-skip": "", "aria-label": L("Skip the tour"), title: L("Skip the tour"), click: () => finish(false) }, icon("x", 14)),
      ].filter(Boolean));
      api.emit("step", { step: i });
      const t = target(s);
      t?.scrollIntoView({ block: "center", inline: "nearest" });
      place(true);
      card.querySelector("[data-next]").focus({ preventScroll: true });
      cancelAnimationFrame(frame);
      // Keeps to its part as the page scrolls or moves.
      const follow = () => {
        if (at === null || !el.isConnected) return;
        place();
        frame = requestAnimationFrame(follow);
      };
      frame = requestAnimationFrame(follow);
    }
    let last = "";
    function place(force) {
      const s = steps()[at], t = s && target(s), { card, cover, hole } = ui, vw = innerWidth, vh = innerHeight;
      const r = t?.getBoundingClientRect(), sig = JSON.stringify([r && [r.left, r.top, r.width, r.height], vw, vh, at, card.offsetHeight]);
      if (!force && sig === last) return;
      last = sig;
      hole.hidden = !r;
      cover.toggleAttribute("data-dim", !r && cfg.backdrop);
      const w = card.offsetWidth, h = card.offsetHeight, g = 12;
      if (!r) return void Object.assign(card.style, { left: (vw - w) / 2 + "px", top: (vh - h) / 2 + "px" });
      Object.assign(hole.style, { left: r.left - 6 + "px", top: r.top - 6 + "px", width: r.width + 12 + "px", height: r.height + 12 + "px" });
      const room = { bottom: vh - r.bottom - 6 - g - h, top: r.top - 6 - g - h, right: vw - r.right - 6 - g - w, left: r.left - 6 - g - w };
      const want = K.rtl(el) ? { left: "right", right: "left" }[s.side] ?? s.side : s.side;
      const side = [want, "bottom", "top", "right", "left"].find((x) => x && room[x] >= 8) ?? (room.bottom > room.top ? "bottom" : "top");
      const x = side === "right" ? r.right + 6 + g : side === "left" ? r.left - 6 - g - w : r.left + r.width / 2 - w / 2;
      const y = side === "bottom" ? r.bottom + 6 + g : side === "top" ? r.top - 6 - g - h : r.top + r.height / 2 - h / 2;
      Object.assign(card.style, { left: K.clamp(x, 12, vw - w - 12) + "px", top: K.clamp(y, 12, Math.max(12, vh - h - 12)) + "px" });
    }
    function hide() {
      at = null;
      cancelAnimationFrame(frame);
      ui?.card.remove();
      ui?.cover.remove();
      ui?.hole.remove();
      ui = null;
      last = "";
      from?.focus?.({ preventScroll: true });
      from = null;
    }
    function draw() {
      const v = api.state[cfg.bind], i = blank(v) || v === false ? null : Number(v);
      if (i === null) return void (at !== null && hide());
      if (i !== at) show(i);
    }
    // A skipped tour stays skipped; one the state starts isn't asked about.
    if (cfg.autoStart && key && !store.get(key) && blank(api.state[cfg.bind])) setTimeout(() => el.isConnected && blank(api.state[cfg.bind]) && set(0), 400);
    return { update: draw };
  };

  // ---------------------------------------------------------------- banner
  B["ui-banner"] = (el, api) => {
    const cfg = api.config, key = cfg.remember ? `irender-banner:${cfg.remember}` : null, left = el.querySelector("[data-banner-left]");
    const ms = (v) => {
      const t = blank(v) ? NaN : Date.parse(v);
      return isNaN(t) ? null : t;
    };
    let asked = false;
    function draw() {
      const now = Date.now(), from = ms(K.value(api, cfg.from)), until = ms(K.value(api, cfg.until));
      if (key && !asked) {
        asked = true;
        if (store.get(key) && !api.state[cfg.bind]) return void setTimeout(() => api.set(cfg.bind, true));
      }
      const shut = !!(cfg.bind && api.state[cfg.bind]);
      if (key && shut) store.set(key, "1");
      el.hidden = shut || (from !== null && now < from) || (until !== null && now >= until);
      if (left) {
        const s = Math.max(0, Math.floor(((ms(K.value(api, cfg.countdown)) ?? now) - now) / 1000)), d = Math.floor(s / 86400), p = (n) => String(n).padStart(2, "0");
        left.textContent = (d ? `${L("{n}d", { n: d })} ` : "") + `${p(Math.floor(s / 3600) % 24)}:${p(Math.floor(s / 60) % 60)}:${p(s % 60)}`;
      }
    }
    draw();
    if (cfg.from !== undefined || cfg.until !== undefined || left) {
      const t = setInterval(() => (el.isConnected ? draw() : clearInterval(t)), 1000);
    }
    return { update: draw };
  };

  // ---------------------------------------------------------------- session
  B["ui-session-timeout"] = (el, api) => {
    const cfg = api.config, dialog = el.querySelector("dialog"), text = el.querySelector("[data-left]"), bar = el.querySelector("progress");
    const AFTER = cfg.after * 60000, WARN = cfg.warn * 60000, name = `irender-${cfg.channel ?? "session"}`;
    let last = Date.now(), over = false, sent = 0;
    const channel = "BroadcastChannel" in window ? new BroadcastChannel(name) : null;
    // Being used, here or in another tab of the same name.
    const active = (tell = true) => {
      if (over || dialog.open) return;
      last = Date.now();
      if (tell && last - sent > 5000) (sent = last), channel?.postMessage({ at: last });
    };
    for (const e of ["pointerdown", "keydown", "wheel", "touchstart"]) addEventListener(e, () => active(), { passive: true, capture: true });
    channel?.addEventListener("message", (e) => {
      if (e.data?.at) {
        last = Math.max(last, e.data.at);
        if (e.data.stay && dialog.open) dialog.close();
      } else if (e.data?.out && !over) end(false);
    });
    const ends = () => {
      const at = Date.parse(K.value(api, cfg.expires) ?? "");
      return isNaN(at) ? last + AFTER : at;
    };
    const go = () => {
      const u = K.value(api, cfg.signOutUrl);
      if (typeof u === "string" && u && !/^\s*(javascript|vbscript|data):/i.test(u)) location.assign(u);
    };
    function end(tell = true) {
      over = true;
      if (dialog.open) dialog.close();
      put(null);
      if (tell) channel?.postMessage({ out: true });
      api.emit("expire", {});
      go();
    }
    const put = (v) => cfg.bindLeft && (api.state[cfg.bindLeft] ?? null) !== v && api.set(cfg.bindLeft, v);
    async function stay() {
      last = Date.now();
      dialog.close();
      put(null);
      channel?.postMessage({ at: last, stay: true });
      api.emit("extend", {});
      const u = K.value(api, cfg.extendUrl);
      if (u) try {
        const r = await fetch(u, { method: "POST", headers: { accept: "application/json" } });
        if (r.status === 401 || r.status === 403) end();
      } catch { /* tried again the next time it asks */ }
    }
    el.querySelector("[data-stay]").addEventListener("click", stay);
    el.querySelector("[data-out]").addEventListener("click", () => end());
    dialog.addEventListener("cancel", (e) => (e.preventDefault(), stay()));
    let said = null;
    function tick() {
      if (over) return;
      const left = ends() - Date.now();
      if (left <= 0) return end();
      if (left > WARN) return void (dialog.open && (dialog.close(), put(null)));
      if (!dialog.open) {
        dialog.showModal();
        el.querySelector("[data-stay]").focus();
        api.emit("warn", { seconds: Math.ceil(left / 1000) });
        said = null;
      }
      const s = Math.ceil(left / 1000);
      text.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
      text.toggleAttribute("data-soon", s <= 15);
      bar.value = left / WARN;
      put(s);
      // Read out now and then, not every second.
      if (said === null || (s <= 30 ? s % 10 === 0 : s % 30 === 0)) if (said !== s) (said = s), K.say(dialog, L("{time} left", { time: text.textContent }));
    }
    const t = setInterval(() => (el.isConnected ? tick() : clearInterval(t)), 1000);
    tick();
    return {};
  };
})();
