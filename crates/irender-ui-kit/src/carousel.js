// irender-ui-kit: the `ui-carousel` behaviour. The previous and next buttons scroll the
// track by one slide and are disabled at its ends; Left and Right do the same while the
// track has focus. Swiping and the scrollbar are the browser's own.
(window.__irenderBehaviors ||= {})["ui-carousel"] = (el) => {
  const track = el.querySelector("[data-track]");
  const prev = el.querySelector("[data-prev]");
  const next = el.querySelector("[data-next]");
  const still = matchMedia("(prefers-reduced-motion: reduce)");
  // The slides: the track's children, looking through `display: contents` wrappers
  // such as a list's. Marked for snapping.
  const slides = (parent = track) =>
    [...parent.children].flatMap((c) => (getComputedStyle(c).display === "contents" ? slides(c) : c.tagName === "TEMPLATE" ? [] : [c]));
  const mark = () => slides().forEach((s) => s.setAttribute("data-slide", ""));
  const step = () => {
    const first = slides()[0];
    const gap = parseFloat(getComputedStyle(track).columnGap) || 0;
    return first ? first.getBoundingClientRect().width + gap : track.clientWidth;
  };
  const go = (dir) => track.scrollBy({ left: dir * step(), behavior: still.matches ? "instant" : "smooth" });
  const ends = () => {
    mark();
    // Right to left pages scroll to negative offsets.
    const at = Math.abs(track.scrollLeft);
    prev.disabled = at < 2;
    next.disabled = at + track.clientWidth >= track.scrollWidth - 2;
  };
  prev.addEventListener("click", () => go(-1));
  next.addEventListener("click", () => go(1));
  track.addEventListener("keydown", (e) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const rtl = getComputedStyle(track).direction === "rtl";
    go((e.key === "ArrowRight") !== rtl ? 1 : -1);
  });
  track.addEventListener("scroll", ends, { passive: true });
  new ResizeObserver(ends).observe(track);
  // Slides a list adds or removes.
  new MutationObserver(ends).observe(track, { childList: true, subtree: true });
  ends();
  return { update: ends };
};
