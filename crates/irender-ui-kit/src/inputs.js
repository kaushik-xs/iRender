// irender-ui-kit: `ui-tags-input`, `ui-masked-input`, `ui-phone-input` and
// `ui-time-picker`. Each keeps a plain value in state and shows it its own way.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const L = (t, v) => window.__irender.label(t, v);
  const K = window.__irenderKit;
  const { make, icon } = K;
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

  // The list under a field: `items()` gives `{ label, small, value, selected, disabled }`s.
  function listbox(el, input, o) {
    const pop = el.querySelector("[data-in-pop]");
    const base = `in-${K.id()}`;
    pop.id = base;
    input.setAttribute("aria-controls", base);
    let items = [], on = -1;
    const isOpen = () => !pop.hidden;
    function mark(i, scroll = true) {
      on = i;
      [...pop.querySelectorAll("[role=option]")].forEach((n, k) => n.toggleAttribute("data-on", k === i));
      const n = pop.querySelectorAll("[role=option]")[i];
      if (n) {
        input.setAttribute("aria-activedescendant", n.id);
        if (scroll) n.scrollIntoView({ block: "nearest" });
      } else input.removeAttribute("aria-activedescendant");
    }
    function open(start) {
      items = o.items();
      const head = pop.querySelector("[data-in-search]");
      const kids = items.map((it, i) => make("div", { role: "option", id: `${base}-${i}`, "aria-selected": String(!!it.selected), "aria-disabled": it.disabled ? "true" : null }, it.mark ?? null, make("span", null, it.label), it.small ? make("small", null, it.small) : null));
      const none = !items.length && o.none ? make("div", { "data-in-none": "" }, o.none()) : null;
      // The list's own search stays where it is: taken out, even to be put back, it would
      // lose the focus.
      for (const n of [...pop.childNodes]) if (n !== head) n.remove();
      pop.append(...[...kids, none].filter(Boolean));
      if (!items.length && !o.none) return close();
      const was = pop.hidden;
      pop.hidden = false;
      o.trigger?.setAttribute("aria-expanded", "true");
      input.setAttribute("aria-expanded", "true");
      if (was) {
        pop.removeAttribute("data-up");
        const r = pop.getBoundingClientRect();
        pop.toggleAttribute("data-up", r.bottom > innerHeight && el.getBoundingClientRect().top > r.height);
      }
      mark(start === undefined ? items.findIndex((i) => i.selected) : start);
    }
    function close() {
      pop.hidden = true;
      on = -1;
      input.setAttribute("aria-expanded", "false");
      o.trigger?.setAttribute("aria-expanded", "false");
      input.removeAttribute("aria-activedescendant");
    }
    function pick(i) {
      const it = items[i];
      if (!it || it.disabled) return false;
      o.pick(it.value);
      return true;
    }
    pop.addEventListener("pointerdown", (e) => !e.target.closest("input") && e.preventDefault());
    pop.addEventListener("click", (e) => {
      const n = e.target.closest("[role=option]");
      if (n) pick([...pop.querySelectorAll("[role=option]")].indexOf(n));
    });
    // Arrows, Enter and Escape, from the field (or the list's own search).
    function key(e) {
      if (e.isComposing) return false;
      const step = (d) => {
        if (!isOpen()) return open();
        let i = on;
        for (let n = 0; n < items.length; n++) {
          i = (i + d + items.length) % items.length;
          if (i < 0) i = items.length - 1;
          if (!items[i].disabled) return mark(i);
        }
      };
      if (e.key === "ArrowDown") step(1);
      else if (e.key === "ArrowUp") on < 0 && isOpen() ? mark(items.length - 1) : step(-1);
      else if (e.key === "Enter" && isOpen() && on >= 0) pick(on);
      else if (e.key === "Escape" && isOpen()) close();
      else return false;
      e.preventDefault();
      e.stopPropagation();
      return true;
    }
    el.addEventListener("focusout", (e) => !el.contains(e.relatedTarget) && close());
    return { open, close, key, isOpen, pop, refresh: (start) => isOpen() && open(start) };
  }

  // ---------------------------------------------------------------- tags
  B["ui-tags-input"] = (el, api) => {
    const cfg = api.config, input = el.querySelector("input"), list = el.querySelector("[data-tags-list]");
    const tags = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind].map(String) : []);
    let re = null;
    try {
      re = cfg.pattern ? new RegExp(`^(?:${cfg.pattern})$`, "u") : null;
    } catch { /* a pattern the browser can't read checks nothing */ }
    const suggestions = () => (K.value(api, cfg.suggestions) ?? []).map((s) => String(K.isObj(s) ? s.value ?? s.label : s));
    const same = (a, b) => fold(a) === fold(b);
    const full = () => cfg.max != null && tags().length >= cfg.max;
    function add(texts) {
      const now = tags(), added = [];
      for (let t of texts) {
        t = t.trim().replace(/\s+/g, " ");
        if (cfg.lowercase) t = t.toLowerCase();
        const known = suggestions().find((s) => same(s, t));
        if (known !== undefined) t = known;
        if (!t || now.some((x) => same(x, t)) || (cfg.only && known === undefined) || (cfg.strict && re && !re.test(t)) || (cfg.max != null && now.length >= cfg.max)) continue;
        now.push(t);
        added.push(t);
      }
      if (!added.length) return false;
      api.set(cfg.bind, now);
      api.emit("change", { value: now, added });
      K.say(el, L("{name} added", { name: added.join(", ") }));
      return true;
    }
    function remove(i) {
      const now = tags(), gone = now[i];
      if (gone === undefined) return;
      const next = now.filter((_, k) => k !== i);
      api.set(cfg.bind, next);
      api.emit("change", { value: next, removed: gone });
      K.say(el, L("{name} removed", { name: gone }));
    }
    const box = listbox(el, input, {
      items: () => {
        const q = fold(input.value.trim()), has = tags();
        return full() ? [] : suggestions().filter((s) => !has.some((x) => same(x, s)) && (!q || fold(s).includes(q))).slice(0, 50).map((s) => ({ label: s, value: s }));
      },
      pick: (v) => {
        add([v]);
        input.value = "";
        box.refresh(-1);
      },
    });
    const commit = () => {
      const t = input.value;
      if (!t.trim()) return true;
      // One that's there already goes too; one that can't be added stays to be put right.
      if (add(t.split(/[,;\n]/)) || tags().some((x) => same(x, t.trim()))) return (input.value = ""), true;
      input.setAttribute("aria-invalid", "true");
      return false;
    };
    let was = null;
    function draw() {
      const now = tags(), sig = JSON.stringify([now, input.disabled]);
      input.placeholder = now.length ? "" : K.value(api, cfg.placeholder) ?? "";
      if (sig === was) return;
      was = sig;
      const at = [...list.children].indexOf(document.activeElement);
      list.replaceChildren(...now.map((t, i) => make("span", { "data-tag": i, role: "listitem", tabindex: -1, "data-bad": re && !re.test(t) }, make("span", null, t), input.disabled ? null : make("button", { type: "button", tabindex: -1, "aria-label": L("Remove {name}", { name: t }) }, icon("x", 12)))));
      if (at >= 0) (list.children[Math.min(at, now.length - 1)] ?? input).focus();
    }
    input.addEventListener("input", () => {
      input.removeAttribute("aria-invalid");
      if (/[,;]/.test(input.value)) commit();
      if (suggestions().length) box.open(input.value.trim() ? 0 : -1);
      if (!cfg.only && box.isOpen() && input.value.trim()) box.open(-1);
    });
    input.addEventListener("focus", () => suggestions().length && box.open(-1));
    input.addEventListener("blur", () => setTimeout(() => !el.contains(document.activeElement) && !cfg.only && commit()));
    input.addEventListener("paste", (e) => {
      const t = e.clipboardData.getData("text/plain");
      if (!/[,;\n\t]/.test(t)) return;
      e.preventDefault();
      add(t.split(/[,;\n\t]/));
    });
    input.addEventListener("keydown", (e) => {
      if (box.key(e)) return;
      if (e.key === "Enter" && input.value.trim()) e.preventDefault(), commit();
      else if (e.key === "Backspace" && !input.value && tags().length) e.preventDefault(), remove(tags().length - 1);
      else if (e.key === (K.rtl(el) ? "ArrowRight" : "ArrowLeft") && input.selectionStart === 0 && input.selectionEnd === 0) list.lastElementChild?.focus();
    });
    list.addEventListener("keydown", (e) => {
      const t = e.target.closest("[data-tag]");
      if (!t) return;
      const i = [...list.children].indexOf(t), next = K.rtl(el) ? "ArrowLeft" : "ArrowRight", prev = K.rtl(el) ? "ArrowRight" : "ArrowLeft";
      if (e.key === prev) list.children[i - 1]?.focus();
      else if (e.key === next) (list.children[i + 1] ?? input).focus();
      else if (e.key === "Delete" || e.key === "Backspace") remove(i);
      else return;
      e.preventDefault();
    });
    el.addEventListener("click", (e) => {
      const b = e.target.closest("[data-tag] button");
      if (b) remove([...list.children].indexOf(b.parentElement)), input.focus();
      else if (e.target === el || e.target === list) input.focus();
    });
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- masked
  const MASKS = { card: ["#### ###### #####", "#### #### #### ####"], iban: "AA## **** **** **** **** **** **** **** **", uuid: "********-****-****-****-************", ipv4: null };
  B["ui-masked-input"] = (el, api) => {
    const cfg = api.config, input = el.querySelector("input");
    const masks = () => {
      const m = K.value(api, cfg.mask);
      return [].concat(MASKS[m] ?? m).map(read);
    };
    // A mask as its slots: `{ test, upper }`, or `{ text }` for a character of its own.
    function read(m) {
      const out = [];
      for (let i = 0; i < m.length; i++) {
        const c = m[i];
        if (c === "\\" && i + 1 < m.length) out.push({ text: m[++i] });
        else if (c === "#") out.push({ test: /\d/ });
        else if (c === "A") out.push({ test: /\p{L}/u, upper: true });
        else if (c === "a") out.push({ test: /\p{L}/u });
        else if (c === "*") out.push({ test: /[\p{L}\d]/u, upper: cfg.mask === "iban" });
        else out.push({ text: c });
      }
      return out;
    }
    const slots = (m) => m.filter((s) => s.test).length;
    // Amex is the only 15-digit card the first mask is for.
    const maskFor = (text) => {
      const all = masks();
      if (cfg.mask === "card") return /^\D*3[47]/.test(text) ? all[0] : all[1];
      const n = [...text].filter((c) => /[\p{L}\d]/u.test(c)).length;
      return all.find((m) => slots(m) >= n) ?? all.at(-1);
    };
    // Text put in the mask: what's shown, what was typed, and whether it's full.
    function fit(text) {
      if (cfg.mask === "ipv4") {
        const p = text.replace(/[^\d.]/g, "").split(".").slice(0, 4).map((x) => x.slice(0, 3));
        const shown = p.join(".");
        return { shown, raw: shown, full: p.length === 4 && p.every((x) => x !== "" && Number(x) < 256) };
      }
      const m = maskFor(text), chars = [...text];
      let shown = "", raw = "", pending = "", i = 0;
      for (const s of m) {
        if (s.text !== undefined) {
          pending += s.text;
          if (chars[i] === s.text) i++;
          continue;
        }
        while (i < chars.length && !s.test.test(chars[i])) i++;
        if (i >= chars.length) break;
        const c = s.upper ? chars[i].toUpperCase() : chars[i];
        shown += pending + c;
        pending = "";
        raw += c;
        i++;
      }
      const full = raw.length === slots(m);
      return { shown: full ? shown + pending : shown, raw, full };
    }
    let last;
    const put = (f) => {
      last = cfg.raw ? f.raw : f.shown;
      const v = last === "" ? null : last;
      if ((api.state[cfg.bind] ?? null) !== v) api.set(cfg.bind, v);
      if (cfg.bindComplete && api.state[cfg.bindComplete] !== f.full) api.set(cfg.bindComplete, f.full);
    };
    input.addEventListener("input", (e) => {
      const at = input.selectionStart, before = [...input.value.slice(0, at)].filter((c) => /[\p{L}\d]/u.test(c)).length;
      let f = fit(input.value);
      // Deleting the mask's own character takes the one before it too.
      if (e.inputType?.startsWith("delete") && f.shown.length > input.value.length && f.shown.startsWith(input.value)) f = fit([...f.raw].slice(0, -1).join(""));
      input.value = f.shown;
      let caret = 0, seen = 0;
      while (caret < f.shown.length && seen < before) if (/[\p{L}\d]/u.test(f.shown[caret++])) seen++;
      if (at >= input.value.length || e.inputType === "insertFromPaste") caret = f.shown.length;
      input.setSelectionRange(caret, caret);
      put(f);
      api.emit("change", { value: api.state[cfg.bind], complete: f.full });
    });
    function draw() {
      const v = api.state[cfg.bind] ?? "";
      if (!cfg.placeholder && cfg.mask !== "ipv4") input.placeholder = masks().at(-1).map((s) => s.text ?? "_").join("");
      if (String(v) === last) return;
      const f = fit(String(v));
      input.value = f.shown;
      last = String(v);
      if (cfg.bindComplete && api.state[cfg.bindComplete] !== f.full) api.set(cfg.bindComplete, f.full);
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- phone
  // Dial codes, and how the common ones group and how long they are.
  const DIAL = Object.fromEntries("AC247 AD376 AE971 AF93 AG1268 AI1264 AL355 AM374 AO244 AR54 AS1684 AT43 AU61 AW297 AZ994 BA387 BB1246 BD880 BE32 BF226 BG359 BH973 BI257 BJ229 BM1441 BN673 BO591 BR55 BS1242 BT975 BW267 BY375 BZ501 CA1 CD243 CF236 CG242 CH41 CI225 CK682 CL56 CM237 CN86 CO57 CR506 CU53 CV238 CW599 CY357 CZ420 DE49 DJ253 DK45 DM1767 DO1809 DZ213 EC593 EE372 EG20 ER291 ES34 ET251 FI358 FJ679 FK500 FM691 FO298 FR33 GA241 GB44 GD1473 GE995 GF594 GH233 GI350 GL299 GM220 GN224 GP590 GQ240 GR30 GT502 GU1671 GW245 GY592 HK852 HN504 HR385 HT509 HU36 ID62 IE353 IL972 IN91 IQ964 IR98 IS354 IT39 JM1876 JO962 JP81 KE254 KG996 KH855 KI686 KM269 KN1869 KP850 KR82 KW965 KY1345 KZ7 LA856 LB961 LC1758 LI423 LK94 LR231 LS266 LT370 LU352 LV371 LY218 MA212 MC377 MD373 ME382 MG261 MH692 MK389 ML223 MM95 MN976 MO853 MP1670 MQ596 MR222 MS1664 MT356 MU230 MV960 MW265 MX52 MY60 MZ258 NA264 NC687 NE227 NG234 NI505 NL31 NO47 NP977 NR674 NU683 NZ64 OM968 PA507 PE51 PF689 PG675 PH63 PK92 PL48 PM508 PR1787 PS970 PT351 PW680 PY595 QA974 RE262 RO40 RS381 RU7 RW250 SA966 SB677 SC248 SD249 SE46 SG65 SH290 SI386 SK421 SL232 SM378 SN221 SO252 SR597 SS211 ST239 SV503 SX1721 SY963 SZ268 TC1649 TD235 TG228 TH66 TJ992 TK690 TL670 TM993 TN216 TO676 TR90 TT1868 TV688 TW886 TZ255 UA380 UG256 US1 UY598 UZ998 VC1784 VE58 VG1284 VI1340 VN84 VU678 WF681 WS685 XK383 YE967 YT262 ZA27 ZM260 ZW263".split(" ").map((s) => [s.slice(0, 2), s.slice(2)]));
  const SHAPE = { IN: [[5, 5], 10], US: [[3, 3, 4], 10], CA: [[3, 3, 4], 10], GB: [[4, 6], 9, 10], AE: [[2, 3, 4], 8, 9], SA: [[2, 3, 4], 9], AU: [[3, 3, 3], 9], FR: [[1, 2, 2, 2, 2], 9], DE: [[3, 4, 4], 6, 11], ES: [[3, 3, 3], 9], IT: [[3, 3, 4], 6, 11], NL: [[1, 4, 4], 9], BE: [[3, 2, 2, 2], 8, 9], CH: [[2, 3, 2, 2], 9], AT: [[3, 4, 4], 7, 13], SE: [[2, 3, 2, 2], 7, 10], NO: [[3, 2, 3], 8], DK: [[2, 2, 2, 2], 8], FI: [[2, 3, 4], 6, 11], PL: [[3, 3, 3], 9], PT: [[3, 3, 3], 9], IE: [[2, 3, 4], 7, 9], SG: [[4, 4], 8], HK: [[4, 4], 8], MY: [[2, 4, 4], 8, 10], CN: [[3, 4, 4], 11], JP: [[2, 4, 4], 9, 10], KR: [[2, 4, 4], 8, 10], BR: [[2, 5, 4], 10, 11], MX: [[2, 4, 4], 10], ZA: [[2, 3, 4], 9], NG: [[3, 3, 4], 8, 10], KE: [[3, 3, 3], 9], EG: [[2, 4, 4], 8, 10], PK: [[3, 3, 4], 10], BD: [[4, 6], 10], LK: [[2, 3, 4], 9], NP: [[3, 3, 4], 8, 10], PH: [[3, 3, 4], 10], ID: [[3, 4, 4], 8, 12], TH: [[2, 3, 4], 8, 9], VN: [[2, 3, 4], 9, 10], TR: [[3, 3, 2, 2], 10], RU: [[3, 3, 2, 2], 10], UA: [[2, 3, 2, 2], 9], IL: [[2, 3, 4], 8, 9], NZ: [[2, 3, 4], 8, 10] };
  const shapeOf = (c) => {
    const s = SHAPE[c] ?? (DIAL[c]?.length === 4 ? [[3, 4], 7] : [[3, 3, 4], 5, 13]);
    return { groups: s[0], min: s[1], max: s[2] ?? s[1] };
  };
  const flag = (c) => String.fromCodePoint(...[...c].map((x) => 0x1f1a5 + x.charCodeAt(0)));
  B["ui-phone-input"] = (el, api) => {
    const cfg = api.config, input = el.querySelector(":scope > input"), button = el.querySelector("[data-country]");
    const names = new Intl.DisplayNames([cfg.locale || navigator.language || "en"], { type: "region" });
    const name = (c) => {
      try {
        return names.of(c) ?? c;
      } catch {
        return c;
      }
    };
    const only = (Array.isArray(cfg.countries) ? cfg.countries.map((c) => String(c).toUpperCase()) : Object.keys(DIAL)).filter((c) => DIAL[c]);
    const all = [...only].sort((a, b) => name(a).localeCompare(name(b), cfg.locale || undefined));
    const region = (l) => {
      try {
        return new Intl.Locale(l).region;
      } catch {
        return undefined;
      }
    };
    const first = [cfg.country?.toUpperCase(), region(cfg.locale), ...(navigator.languages ?? []).map(region), "US"].find((c) => c && only.includes(c)) ?? all[0];
    let country = first, last;
    // A whole number → its country and the rest. A shared code keeps the country it was in.
    function split(e164) {
      const d = String(e164 ?? "").replace(/\D/g, "");
      let best = null;
      for (const c of only) if (d.startsWith(DIAL[c]) && (!best || DIAL[c].length > DIAL[best].length || (DIAL[c].length === DIAL[best].length && (c === country || (best !== country && (c === "US" || c === "RU" || c === "IT" || c === "GB")))))) best = c;
      return best ? { country: best, digits: d.slice(DIAL[best].length) } : { country, digits: d };
    }
    function group(digits) {
      const out = [];
      let i = 0;
      const g = shapeOf(country).groups;
      for (let k = 0; i < digits.length; k++) {
        const n = k < g.length - 1 ? g[k] : k === g.length - 1 ? Math.max(g[k], 99) : 99;
        out.push(digits.slice(i, i + n));
        i += n;
      }
      return out.join(" ");
    }
    const digitsOf = (s) => s.replace(/\D/g, "");
    function put(digits) {
      const v = digits ? `+${DIAL[country]}${digits}` : null;
      last = v;
      if ((api.state[cfg.bind] ?? null) !== v) api.set(cfg.bind, v);
      meta(digits);
    }
    function meta(digits) {
      const s = shapeOf(country), ok = digits.length >= s.min && digits.length <= s.max;
      if (cfg.bindCountry && api.state[cfg.bindCountry] !== country) api.set(cfg.bindCountry, country);
      if (cfg.bindValid && api.state[cfg.bindValid] !== ok) api.set(cfg.bindValid, ok);
      button.replaceChildren(make("span", { "aria-hidden": "true" }, flag(country)), make("small", null, `+${DIAL[country]}`), icon("chevron", 12));
      button.setAttribute("aria-label", L("Country: {name}, +{code}", { name: name(country), code: DIAL[country] }));
      if (!cfg.placeholder) input.placeholder = group("9876543210123".slice(0, s.max === s.min ? s.max : Math.min(10, s.max)));
    }
    const search = make("input", { "data-in-search": "", type: "search", placeholder: L("Search countries"), "aria-label": L("Search countries") });
    const box = listbox(el, search, {
      trigger: button,
      none: () => L("No results"),
      items: () => {
        const q = fold(search.value.trim()).replace(/^\+/, "");
        return all.filter((c) => !q || fold(name(c)).includes(q) || c.toLowerCase() === q || DIAL[c].startsWith(q)).map((c) => ({ label: name(c), small: `+${DIAL[c]}`, value: c, selected: c === country, mark: make("span", { "aria-hidden": "true" }, flag(c)) }));
      },
      pick: (c) => {
        country = c;
        const d = digitsOf(input.value);
        input.value = group(d);
        put(d);
        meta(d);
        box.close();
        input.focus();
        api.emit("country", { country: c });
      },
    });
    button.addEventListener("click", () => {
      if (box.isOpen()) return box.close();
      search.value = "";
      if (search.parentElement !== box.pop) box.pop.prepend(search);
      box.open();
      search.focus();
    });
    search.addEventListener("input", () => box.open(0));
    search.addEventListener("keydown", (e) => {
      const open = box.isOpen();
      box.key(e);
      if (open && e.key === "Escape") button.focus();
    });
    input.addEventListener("input", () => {
      const text = input.value, at = input.selectionStart, before = digitsOf(text.slice(0, at)).length;
      let d = digitsOf(text);
      // A whole number, with its code.
      if (/^\s*(\+|00)/.test(text)) {
        const s = split(text.replace(/^\s*00/, "+"));
        if (d.replace(/^00/, "").length > DIAL[s.country].length + 2 || d.length > 6) {
          country = s.country;
          d = s.digits;
          input.value = group(d);
          input.setSelectionRange(input.value.length, input.value.length);
          put(d);
        }
        return;
      }
      // A leading 0 is dialled within the country only (not in Italy).
      if (d.startsWith("0") && country !== "IT" && d.length > 1) d = d.replace(/^0+/, "");
      d = d.slice(0, 15 - DIAL[country].length);
      input.value = group(d);
      let caret = 0, seen = 0;
      while (caret < input.value.length && seen < before) if (/\d/.test(input.value[caret++])) seen++;
      if (at >= text.length) caret = input.value.length;
      input.setSelectionRange(caret, caret);
      put(d);
      api.emit("change", { value: api.state[cfg.bind] });
    });
    function draw() {
      const v = api.state[cfg.bind] ?? null;
      if (last !== undefined && v === last) return;
      last = v;
      const s = v ? split(v) : { country, digits: "" };
      country = s.country;
      input.value = group(s.digits);
      meta(s.digits);
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- time
  B["ui-time-picker"] = (el, api) => {
    const cfg = api.config, input = el.querySelector("input"), tool = el.querySelector("[data-in-tool]");
    const DAY = 86400;
    const pad = (n) => String(n).padStart(2, "0");
    const text = (s) => `${pad(Math.floor(s / 3600))}:${pad(Math.floor(s / 60) % 60)}${cfg.seconds ? `:${pad(s % 60)}` : ""}`;
    const secs = (t) => {
      const m = /^(\d{1,2}):(\d{2})(?::(\d{2}))?$/.exec(String(t ?? ""));
      return m && +m[1] < 24 && +m[2] < 60 ? +m[1] * 3600 + +m[2] * 60 + +(m[3] ?? 0) : null;
    };
    const fmt = new Intl.DateTimeFormat(cfg.locale || undefined, { hour: "numeric", minute: "2-digit", ...(cfg.seconds ? { second: "2-digit" } : {}), timeZone: "UTC", ...(cfg.hour12 === undefined ? {} : { hour12: !!cfg.hour12 }) });
    const show = (s) => (s === null ? "" : fmt.format(new Date(s * 1000)));
    const twelve = fmt.resolvedOptions().hour12;
    const value = () => secs(api.state[cfg.bind]);
    const limit = (name, d) => secs(K.value(api, cfg[name])) ?? d;
    function blocked(s) {
      if (s < limit("min", 0) || s > limit("max", DAY - 1)) return true;
      return (K.value(api, cfg.disabledTimes) ?? []).some((d) => (K.isObj(d) ? s >= (secs(d.from) ?? 0) && s < (secs(d.to) ?? DAY) : secs(d) === s));
    }
    // "9", "930", "9:30pm", "21.30", "noon" → seconds; null when it isn't a time.
    function parse(t) {
      const s = fold(t).trim().replace(/\./g, ":").replace(/:m:?/g, "m");
      if (s === "noon") return 12 * 3600;
      if (s === "midnight") return 0;
      if (s === "now") {
        const d = new Date();
        return d.getHours() * 3600 + d.getMinutes() * 60;
      }
      const half = /^(.*?)\s*(a|p)m?$/.exec(s), rest = (half ? half[1] : s).trim();
      let m = /^(\d{1,2})(?:[:h\s]+(\d{1,2}))?(?::(\d{1,2}))?$/.exec(rest) ?? /^(\d{1,2})(\d{2})(\d{2})?$/.exec(rest);
      if (!m) return null;
      let [h, min, sec] = [+m[1], +(m[2] ?? 0), +(m[3] ?? 0)];
      if (half) {
        if (h < 1 || h > 12) return null;
        h = (h % 12) + (half[2] === "p" ? 12 : 0);
      } else if (twelve && h >= 1 && h <= 6 && !/^0/.test(rest) && limit("min", 0) > h * 3600 + min * 60) h += 12; // "2" when it opens at 9
      return h < 24 && min < 60 && sec < 60 ? h * 3600 + min * 60 + (cfg.seconds ? sec : 0) : null;
    }
    function length(s) {
      const from = secs(K.value(api, cfg.durationFrom));
      if (from === null || s <= from) return null;
      const m = Math.round((s - from) / 60), h = Math.floor(m / 60);
      return h ? (m % 60 ? L("{h} h {m} min", { h, m: m % 60 }) : L("{h} h", { h })) : L("{m} min", { m });
    }
    const times = () => {
      const out = [], now = value(), q = fold(input.value.trim()), typed = document.activeElement === input && input.value !== show(now) && q;
      for (let s = 0; s < DAY; s += cfg.step * 60) {
        if (blocked(s)) {
          // Between the limits, shown but not picked.
          if (s < limit("min", 0) || s > limit("max", DAY - 1)) continue;
        }
        if (cfg.durationFrom !== undefined && secs(K.value(api, cfg.durationFrom)) !== null && s <= secs(K.value(api, cfg.durationFrom))) continue;
        out.push({ label: show(s), small: length(s), value: s, selected: s === now, disabled: blocked(s) });
      }
      return typed ? out.filter((o) => fold(o.label).startsWith(q) || fold(o.label).replace(/\D/g, "").startsWith(q.replace(/\D/g, "") || "x")) : out;
    };
    function put(s) {
      const v = s === null ? null : text(s);
      if ((api.state[cfg.bind] ?? null) !== v) {
        api.set(cfg.bind, v);
        api.emit("change", { value: v });
      }
      last = api.state[cfg.bind] ?? null;
      input.value = show(s);
      check(s);
    }
    const check = (s) => (s !== null && blocked(s) ? input.setAttribute("aria-invalid", "true") : input.removeAttribute("aria-invalid"));
    const box = listbox(el, input, {
      items: times,
      pick: (s) => {
        put(s);
        box.close();
        input.focus();
      },
    });
    let last;
    tool.append(icon("clock", 15));
    const openAt = () => {
      box.open();
      if (value() === null) return;
      // The nearest time, when the value isn't one of them.
      const all = times(), i = all.findIndex((o) => o.value >= value());
      box.open(i < 0 ? all.length - 1 : i);
    };
    tool.addEventListener("click", () => (box.isOpen() ? box.close() : (input.focus(), openAt())));
    input.addEventListener("click", () => !box.isOpen() && openAt());
    input.addEventListener("input", () => {
      input.removeAttribute("aria-invalid");
      box.open(input.value.trim() ? 0 : -1);
    });
    input.addEventListener("blur", () => {
      const t = input.value.trim();
      if (!t) return put(null);
      const s = parse(t);
      // Something that isn't a time stays, marked.
      if (s === null) return input.setAttribute("aria-invalid", "true");
      put(s);
    });
    input.addEventListener("keydown", (e) => {
      // Typed in full, Enter takes it as typed rather than the list's first.
      if (e.key === "Enter" && input.value.trim() && input.value !== show(value()) && parse(input.value) !== null && !e.isComposing) {
        e.preventDefault();
        box.close();
        return put(parse(input.value));
      }
      box.key(e);
    });
    function draw() {
      const v = api.state[cfg.bind] ?? null;
      if (last !== undefined && v === last) return check(value());
      last = v;
      if (document.activeElement !== input) input.value = show(value());
      check(value());
    }
    draw();
    return { update: draw };
  };
})();
