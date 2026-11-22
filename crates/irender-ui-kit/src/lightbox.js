// irender-ui-kit: the `ui-lightbox` behaviour: the pictures over the page, one at a time.
(window.__irenderBehaviors ||= {})["ui-lightbox"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon } = K;
  const safe = (u) => typeof u === "string" && !/^\s*(javascript|vbscript):/i.test(u);
  const images = () => (K.value(api, cfg.images) ?? []).map((i) => (K.isObj(i) ? i : { src: i })).filter((i) => safe(i.src));
  let at = -1, view = null, dialog = null, parts = null, from = null;

  function build() {
    const img = make("img", { alt: "", draggable: "false" });
    const prev = button({ "data-lb-nav": "-1", "aria-label": L("Previous image"), click: () => step(-1) }, icon("left", 20));
    const next = button({ "data-lb-nav": "1", "aria-label": L("Next image"), click: () => step(1) }, icon("right", 20));
    const stage = make("div", { "data-lb-stage": "" }, img);
    const count = make("span", { "data-lb-count": "", "aria-live": "polite" });
    const save = cfg.download ? make("a", { "data-lb-save": "", "aria-label": L("Download"), title: L("Download") }, icon("download", 18)) : null;
    const caption = make("div", { "data-lb-caption": "" }), strip = make("div", { "data-lb-strip": "" });
    const bar = make("div", { "data-lb-bar": "" }, count,
      button({ "aria-label": L("Zoom out"), title: L("Zoom out"), "data-z": "out", click: () => view.zoom(view.scale / 1.5) }, icon("zoomout", 18)),
      button({ "aria-label": L("Zoom in"), title: L("Zoom in"), "data-z": "in", click: () => view.zoom(view.scale * 1.5) }, icon("zoomin", 18)),
      save, button({ "aria-label": L("Close"), title: L("Close"), click: () => close() }, icon("x", 18)));
    dialog = make("dialog", { "aria-label": L("Image") }, bar, make("div", { style: "position:relative;display:grid;min-height:0" }, prev, stage, next), make("div", { "data-lb-foot": "" }, caption, strip));
    el.append(dialog);
    parts = { img, prev, next, stage, count, save, caption, strip, out: bar.querySelector("[data-z=out]"), in: bar.querySelector("[data-z=in]") };
    view = window.__irenderPanZoom(stage, img, {
      wheel: true, max: 8,
      onChange: ({ scale }) => {
        parts.out.disabled = scale <= 1;
        parts.in.disabled = scale >= 8;
      },
    });
    // A sideways swipe steps while the picture isn't zoomed.
    let sx = null;
    stage.addEventListener("pointerdown", (e) => (sx = e.pointerType === "mouse" || view.scale > 1 ? null : [e.clientX, e.clientY]));
    stage.addEventListener("pointerup", (e) => {
      if (!sx) return;
      const dx = e.clientX - sx[0], dy = e.clientY - sx[1];
      sx = null;
      if (Math.abs(dx) > 60 && Math.abs(dx) > Math.abs(dy) * 1.5) step(dx * (K.rtl(el) ? -1 : 1) < 0 ? 1 : -1);
      else if (dy > 120) close();
    });
    stage.addEventListener("click", (e) => e.target === stage && view.scale <= 1 && close());
    dialog.addEventListener("keydown", (e) => {
      const k = K.rtl(el) ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[e.key] ?? e.key : e.key;
      const z = view.scale > 1;
      if (k === "ArrowRight") z ? view.pan(-60, 0) : step(1);
      else if (k === "ArrowLeft") z ? view.pan(60, 0) : step(-1);
      else if (k === "ArrowUp" && z) view.pan(0, 60);
      else if (k === "ArrowDown" && z) view.pan(0, -60);
      else if (k === "Home") go(0);
      else if (k === "End") go(images().length - 1);
      else if (k === "+" || k === "=") view.zoom(view.scale * 1.5);
      else if (k === "-") view.zoom(view.scale / 1.5);
      else if (k === "0") view.reset();
      else return;
      e.preventDefault();
    });
    dialog.addEventListener("close", () => {
      at = -1;
      if (cfg.bind && api.state[cfg.bind] != null) api.set(cfg.bind, null);
      api.emit("close", {});
      from?.focus();
    });
  }
  function show() {
    const all = images(), im = all[at];
    if (!im) return close();
    const { img, prev, next, count, save, caption, strip } = parts;
    img.setAttribute("data-loading", "");
    img.onload = img.onerror = () => {
      img.removeAttribute("data-loading");
      view.reset();
    };
    img.src = im.src;
    img.alt = im.alt ?? im.caption ?? "";
    if (img.complete) img.onload();
    view.reset();
    prev.hidden = next.hidden = all.length < 2;
    count.textContent = L("{n} of {count}", { n: at + 1, count: all.length });
    caption.textContent = im.caption ?? "";
    if (save) {
      save.href = im.src;
      save.download = typeof im.download === "string" ? im.download : im.src.split(/[?#]/)[0].split("/").pop() || "image";
    }
    strip.hidden = all.length < 2;
    if (strip.children.length !== all.length) strip.replaceChildren(...all.map((x, i) => button({ "aria-label": L("Image {n}", { n: i + 1 }), click: () => go(i) }, make("img", { src: safe(x.thumb) ? x.thumb : x.src, alt: "", loading: "lazy" }))));
    [...strip.children].forEach((b, i) => b.setAttribute("aria-current", String(i === at)));
    const cur = strip.children[at];
    if (cur) strip.scrollTo({ left: cur.offsetLeft - strip.clientWidth / 2 + cur.offsetWidth / 2 });
    // The next one's ready before it's asked for.
    if (all.length > 1) new Image().src = all[(at + 1) % all.length].src;
  }
  function go(i) {
    const n = images().length;
    if (!n) return;
    at = (i + n) % n;
    if (cfg.bind && api.state[cfg.bind] !== at) api.set(cfg.bind, at);
    api.emit("show", { index: at, image: images()[at] });
    show();
  }
  const step = (d) => go(at + d);
  function open(i, by) {
    if (!dialog) build();
    from = by ?? document.activeElement;
    if (!dialog.open) dialog.showModal();
    go(i);
  }
  const close = () => dialog?.open && dialog.close();
  el.addEventListener("click", (e) => {
    const a = e.target.closest("[data-lb-open]");
    if (!a || e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
    e.preventDefault();
    open([...el.querySelectorAll("[data-lb-open]")].indexOf(a), a);
  });
  return {
    update() {
      if (!cfg.bind) return;
      const v = api.state[cfg.bind];
      if (v == null) close();
      else if (Number(v) !== at) open(Number(v));
    },
  };
};
