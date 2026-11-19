// irender-ui-kit: anchor positioning for browsers without it (Safari before 26, Firefox
// ESR). The kit places menus, popovers, filters and pickers with `anchor-name`,
// `position-anchor`, `position-area` and `position-try-fallbacks`; a browser that
// doesn't know them drops them, and a popover opens in the middle of the screen. This
// reads those declarations from the page's own style sheets (the browser still has
// their text), finds which elements they apply to, and places each open one beside its
// anchor: on the side the area says, flipped when it doesn't fit, kept on screen, and
// moved as the page scrolls. It does nothing where the browser positions anchors itself,
// unless <html data-anchor-fallback> asks it to (for tests).
(() => {
  if (window.__irenderAnchors) return;
  const forced = document.documentElement.hasAttribute("data-anchor-fallback");
  if (!forced && window.CSS?.supports?.("position-area", "bottom")) return;
  window.__irenderAnchors = true;
  const PROPS = ["anchor-name", "position-anchor", "position-area", "position-try-fallbacks", "position-try"];
  const LAYERS = { "i-base": 1, "i-instance": 2 };

  // ---- the rules: every style rule that sets one of PROPS, with its layer, specificity
  // and place in the page, from the style elements' text
  let rules = [], sig = "";
  function read() {
    // A test's sheet that turns the browser's own anchoring off isn't the page's.
    const sheets = [...document.querySelectorAll("style:not([data-anchor-off])")];
    const now = sheets.map((s) => s.textContent.length).join(",");
    if (now === sig) return;
    sig = now;
    rules = [];
    let order = 0;
    for (const s of sheets) {
      const css = s.textContent;
      if (!PROPS.some((p) => css.includes(p))) continue;
      let i = 0;
      const block = (layer, on) => {
        while (i < css.length) {
          while (i < css.length && /\s/.test(css[i])) i++;
          if (css[i] === "}") return void i++;
          let j = i;
          while (j < css.length && css[j] !== "{" && css[j] !== ";" && css[j] !== "}") j++;
          const head = css.slice(i, j).trim();
          if (css[j] !== "{") {
            i = j + 1;
            continue;
          }
          i = j + 1;
          if (head.startsWith("@layer")) block(LAYERS[head.slice(6).trim()] ?? 3, on);
          else if (head.startsWith("@media")) block(layer, on && matchMedia(head.slice(6).trim()).matches);
          else if (head.startsWith("@supports")) block(layer, on && CSS.supports(head.slice(9).trim()));
          else if (head.startsWith("@")) block(layer, false);
          else {
            const end = css.indexOf("}", i);
            const body = css.slice(i, end < 0 ? css.length : end);
            i = end < 0 ? css.length : end + 1;
            if (!on) continue;
            const props = {};
            for (const d of body.split(";")) {
              const k = d.indexOf(":"), name = d.slice(0, k).trim();
              if (k > 0 && PROPS.includes(name)) props[name] = d.slice(k + 1).replace(/!important/, "").trim();
            }
            if (Object.keys(props).length) for (const sel of head.split(",")) rules.push({ sel: sel.trim(), layer, spec: specificity(sel), order: order++, props });
          }
        }
      };
      block(3, true);
    }
  }
  // Ids, then classes, attributes and pseudo-classes, then elements.
  function specificity(sel) {
    const s = sel.replace(/:(?:is|where|not|has)\(([^()]*)\)/g, " $1 ");
    const ids = (s.match(/#[\w-]+/g) || []).length;
    const classes = (s.match(/\.[\w-]+|\[[^\]]*\]|:(?!:)[\w-]+/g) || []).length;
    const tags = (s.replace(/\[[^\]]*\]/g, "").match(/(^|[\s>+~])[a-z][\w-]*/gi) || []).length;
    return ids * 1e6 + classes * 1e3 + tags;
  }
  const matches = (el, sel) => {
    try {
      return el.matches(sel);
    } catch {
      return false;
    }
  };
  // An element's value for `prop`: the winning rule's, with `var()` read from the element.
  function value(el, prop) {
    let best = null;
    for (const r of rules) {
      if (!(prop in r.props) || !matches(el, r.sel)) continue;
      if (!best || r.layer > best.layer || (r.layer === best.layer && (r.spec > best.spec || (r.spec === best.spec && r.order > best.order)))) best = r;
    }
    if (!best) return null;
    return best.props[prop].replace(/var\(\s*(--[\w-]+)\s*(?:,([^)]*))?\)/g, (_, name, fallback) => getComputedStyle(el).getPropertyValue(name).trim() || (fallback ?? "").trim());
  }

  // ---- placing
  // The element named `name`: the last one shown, as the browser picks.
  function anchorFor(el, name) {
    if (!name || name === "none" || name === "auto") {
      // No name: the popover's invoker, its implicit anchor.
      return el.id ? document.querySelector(`[popovertarget="${CSS.escape(el.id)}"], [commandfor="${CSS.escape(el.id)}"]`) : null;
    }
    let found = null;
    for (const r of rules) {
      // A literal name that isn't this one can't be it; a `var()` has to be read.
      const v = r.props["anchor-name"];
      if (v === undefined || (!v.includes("var(") && !v.split(",").some((x) => x.trim() === name))) continue;
      for (const a of document.querySelectorAll(r.sel)) if (value(a, "anchor-name")?.split(",").map((x) => x.trim()).includes(name) && a.getClientRects().length) found = a;
    }
    return found;
  }
  const FLIP_BLOCK = { top: "bottom", bottom: "top", "span-top": "span-bottom", "span-bottom": "span-top" };
  const FLIP_INLINE = { left: "right", right: "left", "span-left": "span-right", "span-right": "span-left" };
  // Where the element goes for `area` (`bottom span-right`, `right span-bottom`, `top`, …).
  function spot(area, a, w, h, m) {
    const words = area.split(/\s+/);
    const v = words.find((x) => /^(top|bottom|span-top|span-bottom)$/.test(x)), hz = words.find((x) => /^(left|right|span-left|span-right)$/.test(x));
    const top = v === "bottom" ? a.bottom + m.top
      : v === "top" ? a.top - h - m.bottom
      : v === "span-bottom" ? a.top + m.top
      : v === "span-top" ? a.bottom - h - m.bottom
      : a.top + a.height / 2 - h / 2;
    const left = hz === "right" ? a.right + m.left
      : hz === "left" ? a.left - w - m.right
      : hz === "span-right" ? a.left + m.left
      : hz === "span-left" ? a.right - w - m.right
      : a.left + a.width / 2 - w / 2;
    return { top, left };
  }
  const fits = (p, w, h) => p.top >= 0 && p.left >= 0 && p.top + h <= innerHeight && p.left + w <= innerWidth;
  function place(el) {
    const area = value(el, "position-area");
    if (!area) return;
    const anchor = anchorFor(el, value(el, "position-anchor"));
    if (!anchor) return;
    // Auto insets make `margin: auto` nothing, so the margins read are the gaps the CSS
    // sets. They stay, and the place given is the margin box's.
    Object.assign(el.style, { position: "fixed", inset: "auto" });
    const cs = getComputedStyle(el);
    const m = { top: parseFloat(cs.marginTop) || 0, right: parseFloat(cs.marginRight) || 0, bottom: parseFloat(cs.marginBottom) || 0, left: parseFloat(cs.marginLeft) || 0 };
    const a = anchor.getBoundingClientRect(), w = el.offsetWidth, h = el.offsetHeight;
    // The area, then each fallback in turn, the first that fits.
    const tries = [area];
    for (const f of (value(el, "position-try-fallbacks") ?? value(el, "position-try") ?? "").split(",").map((x) => x.trim()).filter(Boolean)) {
      const flip = (map) => (x) => x.split(/\s+/).map((t) => map[t] ?? t).join(" ");
      let t = area;
      if (f.includes("flip-block")) t = flip(FLIP_BLOCK)(t);
      if (f.includes("flip-inline")) t = flip(FLIP_INLINE)(t);
      tries.push(t);
    }
    let p = spot(area, a, w, h, m);
    for (const t of tries) {
      const q = spot(t, a, w, h, m);
      if (fits(q, w, h)) {
        p = q;
        break;
      }
    }
    // Kept on screen.
    const left = Math.max(4, Math.min(p.left, innerWidth - w - 4)), top = Math.max(4, Math.min(p.top, innerHeight - h - 4));
    el.style.left = `${Math.round(left - m.left)}px`;
    el.style.top = `${Math.round(top - m.top)}px`;
    el.setAttribute("data-anchored", "");
  }

  // ---- when: open popovers and shown anchored elements, every frame while any is shown
  let frame = 0, timer = 0;
  function update() {
    cancelAnimationFrame(frame);
    clearTimeout(timer);
    frame = timer = 0;
    read();
    const placed = new Set();
    for (const r of rules) {
      if (!("position-anchor" in r.props) && !("position-area" in r.props)) continue;
      let list;
      try {
        list = document.querySelectorAll(r.sel);
      } catch {
        continue;
      }
      for (const el of list) {
        if (placed.has(el)) continue;
        placed.add(el);
        const open = el.hasAttribute("popover") ? el.matches(":popover-open") : el.getClientRects().length > 0;
        if (open) place(el);
      }
    }
    if ([...placed].some((el) => el.hasAttribute("data-anchored") && (el.hasAttribute("popover") ? el.matches(":popover-open") : el.getClientRects().length))) schedule();
  }
  // The next frame, or a moment from now where frames don't come (a hidden tab).
  const schedule = () => {
    frame ||= requestAnimationFrame(update);
    timer ||= setTimeout(update, 50);
  };
  for (const type of ["toggle", "click", "keydown", "focusin", "input"]) document.addEventListener(type, schedule, true);
  addEventListener("scroll", schedule, { capture: true, passive: true });
  addEventListener("resize", schedule);
  new MutationObserver(schedule).observe(document.documentElement, { subtree: true, attributes: true, attributeFilter: ["hidden", "open", "class", "aria-expanded"] });
  schedule();
})();
