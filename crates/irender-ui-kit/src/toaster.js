// irender-ui-kit: the `ui-toaster` behaviour. Setting the bound key adds a toast to
// the stack and clears the key, so the next one can follow. Each toast hides after
// its own `duration`; timers pause while the pointer is over the toasts or focus is
// in them, so one being read doesn't vanish, and a bar along the bottom counts down
// what is left.
(window.__irenderBehaviors ||= {})["ui-toaster"] = (el, api) => {
  const { key, duration, max } = api.config;
  const tpl = el.querySelector("template");
  const shown = []; // oldest first: { el, bar, total, left (ms; 0 = until dismissed), since, timer, anim }
  const still = matchMedia("(prefers-reduced-motion: reduce)");
  let last = null;
  let paused = false;

  const remove = (t) => {
    const i = shown.indexOf(t);
    if (i < 0) return;
    shown.splice(i, 1);
    clearTimeout(t.timer);
    t.anim?.cancel();
    // A short fade on the way out; `data-leaving` keeps it out of the way meanwhile.
    t.el.dataset.leaving = "";
    t.el.inert = true;
    if (still.matches || !t.el.animate) return t.el.remove();
    t.el.style.pointerEvents = "none";
    t.el
      .animate([{ opacity: 1 }, { opacity: 0, transform: "scale(0.96)" }], { duration: 150, easing: "ease-in" })
      .finished.then(() => t.el.remove(), () => t.el.remove());
  };
  const run = (t) => {
    if (paused || !(t.left > 0)) return;
    t.since = Date.now();
    t.timer = setTimeout(() => remove(t), t.left);
    t.anim?.cancel();
    t.anim = t.bar?.animate(
      [{ transform: `scaleX(${Math.min(1, t.left / t.total)})` }, { transform: "scaleX(0)" }],
      { duration: t.left, fill: "forwards" },
    );
  };
  const stop = (t) => {
    if (t.since === null) return;
    clearTimeout(t.timer);
    t.anim?.pause();
    t.left -= Date.now() - t.since;
    t.since = null;
  };
  const pause = () => {
    if (paused) return;
    paused = true;
    shown.forEach(stop);
  };
  const resume = () => {
    if (!paused || el.matches(":hover") || el.contains(document.activeElement)) return;
    paused = false;
    // Whatever ran out while paused gets a moment more.
    for (const t of shown) {
      if (t.left !== 0) t.left = Math.max(t.left, 1000);
      run(t);
    }
  };
  el.addEventListener("pointerenter", pause);
  el.addEventListener("pointerleave", resume);
  el.addEventListener("focusin", pause);
  el.addEventListener("focusout", (e) => el.contains(e.relatedTarget) || queueMicrotask(resume));

  const show = (v) => {
    const node = tpl.content.firstElementChild.cloneNode(true);
    const text = (sel, value) => {
      const part = node.querySelector(sel);
      if (value === null || value === undefined || value === "") part.remove();
      else part.textContent = typeof value === "string" ? value : JSON.stringify(value);
    };
    const toast = typeof v === "object" ? v : { title: v };
    text("[data-title]", toast.title);
    text("[data-description]", toast.description);
    if (toast.variant) node.dataset.variant = toast.variant;
    const ms = Number(toast.duration ?? duration);
    const left = Number.isFinite(ms) && ms > 0 ? ms : 0;
    const bar = node.querySelector("[data-progress]");
    if (!left) bar.remove(); // stays until dismissed: nothing to count down
    const t = { el: node, bar: left ? bar : null, total: left, left, since: null };
    node.querySelector("[data-dismiss]").addEventListener("click", () => remove(t));
    el.append(node);
    shown.push(t);
    while (shown.length > max) remove(shown[0]);
    run(t);
  };

  const update = (state) => {
    const v = state[key];
    if (v === last) return;
    last = v;
    if (v === null || v === undefined || v === false) return;
    show(v);
    // After this update finishes: a set from inside it would nest another one.
    queueMicrotask(() => api.set(key, null));
  };
  update(api.state);
  return { update };
};
