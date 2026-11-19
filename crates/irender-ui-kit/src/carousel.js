// irender-ui-kit: the `ui-carousel` behaviour. The previous and next buttons scroll the
// track to the slide before or after the current one (so slides of different widths step
// evenly) and are disabled at its ends, unless `loop` goes round; Left and Right do the
// same while the track has focus. With `dots`, a dot per slide shows the current one and
// goes to its slide. With `autoplay` (ms), it moves on by itself, round and round, and
// pauses while the pointer or focus is on it, the tab is hidden, the reader prefers less
// motion, or its pause button says so. Swiping and the scrollbar are the browser's own.
(window.__irenderBehaviors ||= {})["ui-carousel"] = (el, api) => {
  const cfg = api?.config ?? {};
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const track = el.querySelector("[data-track]");
  const prev = el.querySelector("[data-prev]");
  const next = el.querySelector("[data-next]");
  const dots = el.querySelector("[data-dots]");
  const play = el.querySelector("[data-play]");
  const still = matchMedia("(prefers-reduced-motion: reduce)");
  const rtl = () => getComputedStyle(track).direction === "rtl";
  // The slides: the track's children, looking through `display: contents` wrappers
  // such as a list's. Marked for snapping, and named "2 of 5".
  const slides = (parent = track) =>
    [...parent.children].flatMap((c) => (getComputedStyle(c).display === "contents" ? slides(c) : c.tagName === "TEMPLATE" ? [] : [c]));
  // How far a slide's start is from the track's.
  const offset = (s) => {
    const a = s.getBoundingClientRect(), b = track.getBoundingClientRect();
    return rtl() ? b.right - a.right : a.left - b.left;
  };
  // The slide at the start of the view: the one nearest it.
  const current = () => {
    const all = slides();
    let best = 0;
    all.forEach((s, i) => Math.abs(offset(s)) < Math.abs(offset(all[best])) && (best = i));
    return best;
  };
  function show(i) {
    const all = slides();
    if (!all.length) return;
    const s = all[(i + all.length) % all.length];
    track.scrollBy({ left: (rtl() ? -1 : 1) * offset(s), behavior: still.matches ? "instant" : "smooth" });
    // The dots follow at once, not only as the scroll's events come.
    setTimeout(ends);
  }
  const go = (dir) => {
    const all = slides(), i = current();
    // Past an end: round to the other, with `loop`; else nowhere.
    if (!cfg.loop && (i + dir < 0 || (dir > 0 && atEnd()))) return;
    show(dir > 0 && atEnd() ? 0 : i + dir < 0 ? all.length - 1 : i + dir);
  };
  // Right to left pages scroll to negative offsets.
  const atEnd = () => Math.abs(track.scrollLeft) + track.clientWidth >= track.scrollWidth - 2;
  function ends() {
    const all = slides();
    all.forEach((s, i) => {
      s.setAttribute("data-slide", "");
      s.setAttribute("role", "group");
      s.setAttribute("aria-roledescription", L("slide"));
      if (!s.hasAttribute("aria-label")) s.setAttribute("aria-label", L("{n} of {count}", { n: i + 1, count: all.length }));
    });
    prev.disabled = !cfg.loop && Math.abs(track.scrollLeft) < 2;
    next.disabled = !cfg.loop && atEnd();
    if (dots) {
      // A dot per slide; the current one marked.
      if (dots.children.length !== all.length) {
        dots.replaceChildren(
          ...all.map((_, i) => {
            const d = document.createElement("button");
            d.type = "button";
            d.setAttribute("data-dot", String(i));
            d.setAttribute("aria-label", L("Go to slide {n}", { n: i + 1 }));
            return d;
          }),
        );
      }
      const at = atEnd() ? all.length - 1 : current();
      [...dots.children].forEach((d, i) => (i === at ? d.setAttribute("aria-current", "true") : d.removeAttribute("aria-current")));
    }
  }
  prev.addEventListener("click", () => go(-1));
  next.addEventListener("click", () => go(1));
  dots?.addEventListener("click", (e) => {
    const d = e.target.closest("[data-dot]");
    if (d) show(Number(d.getAttribute("data-dot")));
  });
  track.addEventListener("keydown", (e) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    go((e.key === "ArrowRight") !== rtl() ? 1 : -1);
  });
  track.addEventListener("scroll", ends, { passive: true });
  new ResizeObserver(ends).observe(track);
  // Slides a list adds or removes.
  new MutationObserver(ends).observe(track, { childList: true, subtree: true });

  // ---- autoplay
  const every = Number(cfg.autoplay) || 0;
  let paused = false, held = 0, timer = 0;
  const running = () => every > 0 && !paused && !held && !document.hidden && !still.matches;
  function tick() {
    clearTimeout(timer);
    if (running()) timer = setTimeout(() => (running() && show(atEnd() ? 0 : current() + 1), tick()), every);
  }
  // The pointer or focus on it holds it still.
  const hold = (d) => () => ((held = Math.max(0, held + d)), tick());
  if (every > 0) {
    el.addEventListener("pointerenter", hold(1));
    el.addEventListener("pointerleave", hold(-1));
    el.addEventListener("focusin", (e) => e.target !== play && hold(1)());
    el.addEventListener("focusout", (e) => e.target !== play && hold(-1)());
    document.addEventListener("visibilitychange", tick);
    play?.addEventListener("click", () => {
      paused = !paused;
      play.setAttribute("aria-pressed", String(paused));
      play.setAttribute("aria-label", paused ? L("Play slides") : L("Pause slides"));
      play.textContent = paused ? "▶" : "❚❚";
      tick();
    });
    tick();
  }
  ends();
  return { update: ends };
};
