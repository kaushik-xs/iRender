// irender-ui-kit: the `ui-input-otp` behaviour. The real field lies over the boxes;
// this keeps out what the pattern doesn't allow (from typing and pasting alike), writes
// the code to state, draws it in the boxes with the active one marked, and fires a
// bubbling `complete` event (`detail.value`) when the last box fills.
(window.__irenderBehaviors ||= {})["ui-input-otp"] = (el, api) => {
  const cfg = api.config;
  const input = el.querySelector("[data-otp-input]");
  const slots = [...el.querySelectorAll("[data-otp-slot]")];
  const keep = { digits: /[0-9]/, letters: /[A-Za-z]/, alphanumeric: /[A-Za-z0-9]/ }[cfg.pattern] || /[0-9]/;
  const clean = (s) => [...String(s ?? "")].filter((c) => keep.test(c)).join("").toUpperCase().slice(0, cfg.length);
  let last = clean(input.value), done = last.length === cfg.length;

  function draw() {
    const v = input.value;
    const focused = document.activeElement === input;
    // The caret's box: where the next character goes, or the last when full.
    const at = Math.min(input.selectionStart ?? v.length, cfg.length - 1);
    const end = input.selectionEnd ?? at;
    slots.forEach((s, i) => {
      const c = v[i];
      if (cfg.mask) {
        s.replaceChildren(...(c ? [Object.assign(document.createElement("span"), {})] : []));
        s.firstChild?.setAttribute("data-otp-dot", "");
      } else s.textContent = c ?? "";
      s.toggleAttribute("data-empty", !c);
      s.toggleAttribute("data-active", focused && (i === at || (end > at && i >= at && i < end)));
    });
  }

  function commit() {
    const v = clean(input.value);
    if (v !== input.value) {
      const pos = Math.min(v.length, input.selectionStart ?? v.length);
      input.value = v;
      input.setSelectionRange(pos, pos);
    }
    if (v !== last) {
      last = v;
      if (cfg.bind) api.set(cfg.bind, v);
      const full = v.length === cfg.length;
      if (full && !done) api.emit("complete", { value: v });
      done = full;
    }
    draw();
  }

  input.addEventListener("input", commit);
  // Typing over a full code replaces from the caret, as the boxes suggest.
  input.addEventListener("beforeinput", (e) => {
    if (e.inputType !== "insertText" || !e.data || input.value.length < cfg.length) return;
    const at = input.selectionStart ?? 0;
    if (input.selectionEnd === at && at < cfg.length) input.setSelectionRange(at, at + 1);
  });
  for (const ev of ["focus", "blur", "keyup", "click", "select"]) input.addEventListener(ev, () => requestAnimationFrame(draw));
  document.addEventListener("selectionchange", () => document.activeElement === input && draw());

  draw();
  return {
    // The code set from state (cleared after a wrong attempt, say).
    update(state) {
      if (!cfg.bind) return;
      const v = clean(state[cfg.bind]);
      if (v === last) return;
      last = v;
      done = v.length === cfg.length;
      input.value = v;
      draw();
    },
  };
};
