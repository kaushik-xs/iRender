// irender-ui-kit: the `ui-scroll-spy` behaviour. The current section is the last one whose
// top has passed a line a little below the top of the window (below `offset`, and below
// a sticky bar that sets `--site-nav`); at the very bottom of the page, the last one. Its
// link gets `aria-current="location"`, and a list that scrolls on its own scrolls to keep
// the link in view, without moving the page.
(window.__irenderBehaviors ||= {})["ui-scroll-spy"] = (el, api) => {
  const links = () => [...el.querySelectorAll('a[href^="#"]')].filter((a) => a.getAttribute("href").length > 1);
  const target = (a) => document.getElementById(decodeURIComponent(a.getAttribute("href").slice(1)));
  const covered = () => {
    const bar = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--site-nav")) || 0;
    return (Number(api.config.offset) || 0) + bar;
  };
  let current = null;

  function update() {
    const list = links().map((a) => [a, target(a)]).filter(([, t]) => t);
    if (!list.length) return;
    const line = covered() + Math.min(120, innerHeight / 4);
    let active = list[0][0];
    for (const [a, t] of list) if (t.getBoundingClientRect().top <= line) active = a;
    // At the bottom, sections too short to reach the line still get their turn.
    if (innerHeight + scrollY >= document.documentElement.scrollHeight - 2) active = list.at(-1)[0];
    if (active === current) return;
    current?.removeAttribute("aria-current");
    active.setAttribute("aria-current", "location");
    current = active;
    reveal(active);
  }

  // Into view in the nearest ancestor that scrolls (a sticky sidebar, say), never the page.
  function reveal(a) {
    for (let box = a.parentElement; box && box !== document.body && box !== document.documentElement; box = box.parentElement) {
      if (box.scrollHeight <= box.clientHeight + 1 || !/(auto|scroll)/.test(getComputedStyle(box).overflowY)) continue;
      const r = a.getBoundingClientRect(), b = box.getBoundingClientRect();
      if (r.top < b.top) box.scrollTop -= b.top - r.top + 8;
      else if (r.bottom > b.bottom) box.scrollTop += r.bottom - b.bottom + 8;
      return;
    }
  }

  // At most one update per 50 ms while scrolling, and one after the last event.
  let last = 0, trailing = null;
  const onScroll = () => {
    clearTimeout(trailing);
    if (Date.now() - last >= 50) {
      last = Date.now();
      update();
    } else trailing = setTimeout(() => ((last = Date.now()), update()), 50);
  };
  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("resize", onScroll, { passive: true });
  addEventListener("hashchange", () => setTimeout(update));
  update();
  return {};
};
