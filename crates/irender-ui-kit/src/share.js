// irender-ui-kit: `ui-push-opt-in`, `ui-app-badge`, `ui-share`, `ui-share-target`,
// `ui-contact-picker`, `ui-passkey` and `ui-app-lock`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, O = window.__irenderOffline, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const list = (v) => (Array.isArray(v) ? v : []);
  const say = (api, v, d) => K.value(api, v) ?? d;
  const keep = {
    get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
    set: (k, v) => { try { localStorage.setItem(k, v); } catch { /* not kept */ } },
  };
  const bytes = (s) => Uint8Array.from(atob(String(s).replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(String(s).length / 4) * 4, "=")), (c) => c.charCodeAt(0));
  const b64 = (b) => btoa(String.fromCharCode(...new Uint8Array(b))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  const post = async (url, body) => {
    const res = await fetch(url, { method: "POST", headers: { "content-type": "application/json", accept: "application/json" }, body: JSON.stringify(body) });
    const data = await res.json().catch(() => null);
    if (!res.ok) throw new Error(data?.error || data?.message || `${res.status} ${res.statusText}`.trim());
    return data;
  };

  // ---------------------------------------------------------------- push
  B["ui-push-opt-in"] = (el, api) => {
    const cfg = api.config, topics = list(cfg.topics).map((t) => (isObj(t) ? t : { value: t, label: t })), key = cfg.remember ? `irender-push:${cfg.remember}` : null, kept = `irender-push-topics:${location.pathname}`;
    const body = D.body(el);
    let chosen = new Set(topics.filter((t) => t.on !== false).map((t) => String(t.value))), state = null, sub = null, busy = false, error = null, was = null;
    try {
      const k = JSON.parse(keep.get(kept));
      if (Array.isArray(k)) chosen = new Set(k.map(String));
    } catch { /* as they start */ }
    const reg = async () => ("serviceWorker" in navigator ? (await navigator.serviceWorker.getRegistration().catch(() => null)) ?? null : null);
    async function look() {
      if (!("Notification" in window)) state = D.ios && !D.standalone() ? "install" : "unsupported";
      else if (Notification.permission === "denied") state = "denied";
      else {
        const r = await reg();
        sub = (await r?.pushManager?.getSubscription().catch(() => null)) ?? null;
        state = Notification.permission === "granted" && (!cfg.publicKey || sub) ? "on" : "off";
        if (keep.get(`${kept}:off`) === "1" && !sub) state = "off";
      }
      tell();
    }
    function tell() {
      const v = { state, topics: [...chosen], endpoint: sub?.endpoint ?? null };
      if (cfg.bind && JSON.stringify(api.state[cfg.bind] ?? null) !== JSON.stringify(v)) api.set(cfg.bind, v);
      draw();
    }
    async function save() {
      keep.set(kept, JSON.stringify([...chosen]));
      const url = K.value(api, cfg.saveUrl);
      if (url) await post(url, { subscription: sub ? sub.toJSON() : null, topics: [...chosen] });
    }
    async function run(f) {
      busy = true;
      error = null;
      draw();
      try {
        await f();
      } catch (e) {
        error = D.explain(e, "notifications");
        api.emit("failed", { error });
      }
      busy = false;
      await look();
    }
    const on = () => run(async () => {
      if ((await Notification.requestPermission()) !== "granted") return void api.emit("refused", {});
      keep.set(`${kept}:off`, "0");
      const pub = K.value(api, cfg.publicKey);
      if (pub) {
        const r = await reg();
        if (!r?.pushManager) throw new Error(L("Pushes need the app's service worker"));
        sub = await r.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytes(pub) });
      }
      await save();
      api.emit("on", { topics: [...chosen] });
    });
    const off = () => run(async () => {
      await sub?.unsubscribe();
      sub = null;
      keep.set(`${kept}:off`, "1");
      await save();
      api.emit("off", {});
    });
    async function sample() {
      const r = await reg(), o = { body: L("This is what one looks like."), tag: "irender-test" };
      r ? r.showNotification(document.title, o) : new Notification(document.title, o);
    }
    function draw() {
      const show = state !== null && !(cfg.gated && !K.value(api, cfg.when)) && !(state !== "on" && key && Number(keep.get(key)) > Date.now());
      const title = say(api, cfg.title, L("Stay up to date")), text = K.value(api, cfg.text), label = say(api, cfg.label, L("Turn on notifications"));
      const sig = JSON.stringify([show, state, busy, error, [...chosen], title, text, label]);
      if (sig === was) return;
      was = sig;
      el.hidden = !show;
      if (!show) return;
      const focus = body.contains(document.activeElement) ? document.activeElement.getAttribute("data-f") : null;
      const words = { denied: L("Notifications are blocked for this site."), install: L("Add the app to your home screen first: notifications only work from there."), unsupported: L("This browser can't show notifications.") };
      body.replaceChildren(make("div", { "data-w-card": "" },
        make("div", { "data-w-row": "" }, icon("bell", 18), make("p", { "data-w-title": "" }, title), state === "on" ? make("span", { "data-w-tag": "", "data-tone": "good" }, L("On")) : null),
        text || words[state] ? make("p", { "data-w-note": "" }, words[state] ?? text) : "",
        topics.length && (state === "on" || state === "off") ? make("fieldset", null, make("legend", null, L("About")), topics.map((t) => make("label", { "data-sh-check": "" }, make("input", { type: "checkbox", "data-f": `t${t.value}`, checked: chosen.has(String(t.value)), disabled: busy, change: (e) => { e.target.checked ? chosen.add(String(t.value)) : chosen.delete(String(t.value)); state === "on" ? run(save) : tell(); } }), make("span", null, t.label ?? t.value, t.description ? make("small", null, ` ${t.description}`) : null)))) : "",
        make("div", { "data-w-row": "" },
          state === "off" ? btn("primary", { "data-f": "on", disabled: busy, click: on }, label) : null,
          state === "off" && key ? btn("ghost", { "data-f": "no", click: () => (keep.set(key, String(Date.now() + (cfg.days ?? 30) * 864e5)), api.emit("dismiss", {}), draw()) }, L("Not now")) : null,
          state === "on" ? btn("", { "data-f": "off", disabled: busy, click: off }, L("Turn off")) : null,
          state === "on" && cfg.test ? btn("ghost", { "data-f": "test", click: sample }, L("Show one")) : null,
          state === "denied" ? btn("", { "data-f": "fix", click: () => D.help(el, "notifications") }, L("How to fix")) : null),
        make("p", { "data-w-error": "", role: "alert" }, error ?? "")));
      if (focus) body.querySelector(`[data-f="${CSS.escape(focus)}"]`)?.focus();
    }
    D.watch("notifications", () => el.isConnected && look());
    navigator.serviceWorker?.addEventListener("message", (e) => el.isConnected && e.data?.type && ["push", "notification"].includes(e.data.type) && api.emit(e.data.type, e.data));
    look();
    return { update: draw };
  };

  // ---------------------------------------------------------------- badge
  let titled = null;
  B["ui-app-badge"] = (el, api) => {
    const cfg = api.config;
    let was;
    const base = (titled ??= document.title.replace(/^\(\d+\+?\) /, ""));
    function update() {
      const v = K.value(api, cfg.count), n = v === true ? true : Math.max(0, Math.floor(Number(v) || 0));
      if (n === was) return;
      was = n;
      try {
        (n ? navigator.setAppBadge?.(n === true ? undefined : n) : navigator.clearAppBadge?.())?.catch(() => {});
      } catch { /* only an installed app has one */ }
      if (cfg.title) document.title = n && n !== true ? `(${n > 99 ? "99+" : n}) ${base}` : n ? `• ${base}` : base;
    }
    update();
    return { update };
  };

  // ---------------------------------------------------------------- share
  B["ui-share"] = (el, api) => {
    const cfg = api.config, name = make("span");
    const b = btn(cfg.variant === "primary" ? "primary" : cfg.variant === "ghost" ? "ghost" : "", { "data-share": "", click: go }, icon("share", 16), name);
    el.append(make("div", { "data-w-row": "" }, b));
    const now = () => {
      const v = Object.fromEntries(["url", "title", "text"].map((k) => [k, K.value(api, cfg[k])]).filter(([, x]) => !blank(x)).map(([k, x]) => [k, String(x)]));
      const files = list(K.value(api, cfg.files)).filter((f) => isObj(f) && f.url);
      if (!v.url && !v.text && !files.length) v.url = location.href;
      if (v.url) try {
        v.url = new URL(v.url, location.href).href;
      } catch {
        delete v.url;
      }
      return { v, files };
    };
    async function blobs(files) {
      const out = [];
      for (const f of files) {
        const blob = String(f.url).startsWith("data:") ? D.toBlob(f.url) : await (await fetch(f.url)).blob();
        out.push(new File([blob], f.name || "file", { type: f.type || blob.type }));
      }
      return out;
    }
    async function go() {
      const { v, files } = now();
      if (cfg.sheet !== "own" && navigator.share) {
        try {
          const data = files.length ? { ...v, files: await blobs(files) } : v;
          if (!navigator.canShare || navigator.canShare(data)) {
            await navigator.share(data);
            return void api.emit("share", { how: "sheet" });
          }
        } catch (e) {
          if (e.name === "AbortError") return;
          // Not from here: the list is.
        }
      }
      sheet(v, files);
    }
    function sheet(v, files) {
      const text = [v.title, v.text, v.url].filter(Boolean).join("\n");
      const row = (ic, label, go) => K.button({ click: () => { d.close(); go(); } }, icon(ic, 17), label);
      const link = (ic, label, href) => make("a", { href, target: /^mailto:|^sms:|^tel:/i.test(href) ? null : "_blank", rel: "noreferrer noopener", click: () => (d.close(), api.emit("share", { how: label })) }, icon(ic, 17), label);
      const fill = (h) => String(h).replace(/\{(url|title|text)\}/g, (_, k) => encodeURIComponent(v[k] ?? ""));
      const d = make("dialog", { "data-sh-list": "", "aria-label": L("Share") },
        v.url || v.text ? row("copy", v.url ? L("Copy the link") : L("Copy the text"), async () => {
          try {
            await navigator.clipboard.writeText(v.url ?? text);
          } catch {
            const t = make("textarea", { style: "position:fixed;opacity:0" });
            t.value = v.url ?? text;
            el.append(t);
            t.select();
            document.execCommand("copy");
            t.remove();
          }
          K.say(el, L("Copied"));
          name.textContent = L("Copied");
          setTimeout(label, 1600);
          api.emit("share", { how: "copy" });
        }) : null,
        v.url || v.text ? link("text", L("Email"), `mailto:?subject=${encodeURIComponent(v.title ?? "")}&body=${encodeURIComponent([v.text, v.url].filter(Boolean).join("\n\n"))}`) : null,
        list(cfg.targets).filter((t) => isObj(t) && D.safeUrl(fill(t.href))).map((t) => link("link", t.label ?? t.href, fill(t.href))),
        files.length ? row("download", files.length === 1 ? L("Save the file") : L("Save the files"), async () => {
          for (const f of await blobs(files)) window.__irenderSheets.save(f.name, f);
          api.emit("share", { how: "save" });
        }) : null,
        row("x", L("Cancel"), () => {}));
      d.addEventListener("close", () => (d.remove(), b.focus()));
      d.addEventListener("click", (e) => e.target === d && d.close());
      el.append(d);
      d.showModal();
    }
    const label = () => (name.textContent = say(api, cfg.label, L("Share")));
    label();
    return { update: label };
  };

  // ---------------------------------------------------------------- share target
  B["ui-share-target"] = (el, api) => {
    const cfg = api.config, body = D.body(el);
    let was = null;
    async function take() {
      const q = new URLSearchParams(location.search), id = q.get("shared");
      let got = null;
      if (id) {
        const r = await O.db.get("shared", id).catch(() => null);
        if (r) {
          got = { title: r.title, text: r.text, url: r.url, files: [], at: new Date(r.at).toISOString() };
          for (const f of r.files) got.files.push({ name: f.name, type: f.type, size: f.size, url: await D.dataUrl(f.blob) });
          await O.db.del("shared", id);
        }
      } else if (["title", "text", "url"].some((k) => q.get(`share-${k}`))) got = { title: q.get("share-title") ?? "", text: q.get("share-text") ?? "", url: q.get("share-url") ?? "", files: [], at: new Date().toISOString() };
      if (id) {
        // Reloading the page doesn't take it again.
        q.delete("shared");
        history.replaceState(history.state, "", location.pathname + (String(q) ? `?${q}` : "") + location.hash);
      }
      if (got) {
        api.set(cfg.bind, got);
        api.emit("shared", got);
      }
      // What was never taken is let go after a day.
      for (const r of await O.db.all("shared").catch(() => [])) if (Date.now() - r.at > 864e5) O.db.del("shared", r.id);
    }
    function draw() {
      if (cfg.quiet) return void (el.hidden = true);
      const v = isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : null, empty = K.value(api, cfg.emptyText), accept = say(api, cfg.acceptLabel, L("Add it")), sig = JSON.stringify([v, empty, accept]);
      if (sig === was) return;
      was = sig;
      el.hidden = !v && blank(empty);
      const link = v && /^https?:\/\//i.test(v.url ?? "") ? v.url : null;
      body.replaceChildren(v ? make("div", { "data-w-card": "" },
        v.title ? make("p", { "data-w-title": "" }, v.title) : "",
        v.text ? make("p", { style: "margin:0" }, K.linked(v.text)) : "",
        link ? make("a", { href: link, target: "_blank", rel: "noreferrer noopener", style: "overflow-wrap:anywhere" }, link) : "",
        list(v.files).length ? make("ul", { "data-w-list": "" }, v.files.map((f) => make("li", null, /^image\//.test(f.type) && /^data:image\//.test(f.url) ? make("img", { "data-sh-thumb": "", src: f.url, alt: "" }) : make("span", { "data-sh-thumb": "" }, icon("file", 18)), make("div", { "data-w-main": "" }, make("span", null, f.name), make("small", null, D.size(f.size ?? 0)))))) : "",
        make("div", { "data-w-row": "" }, btn("primary", { "data-accept": "", click: () => api.emit("accept", v) }, accept), btn("ghost", { click: () => (api.set(cfg.bind, null), api.emit("dismiss", {})) }, L("Leave it")))) : make("p", { "data-w-note": "" }, empty ?? ""));
    }
    draw();
    take().then(draw);
    return { update: draw };
  };

  // ---------------------------------------------------------------- contacts
  // The cards of a .vcf file: names, numbers and addresses, unfolded and unescaped.
  function cards(text) {
    const out = [];
    let c = null;
    const un = (s) => s.replace(/\\n/gi, "\n").replace(/\\([,;\\])/g, "$1").trim();
    for (const line of text.replace(/\r\n?/g, "\n").replace(/\n[ \t]/g, "").split("\n")) {
      const m = /^(?:[\w-]+\.)?([A-Za-z-]+)((?:;[^:]*)?):(.*)$/.exec(line);
      if (!m) continue;
      let [, k, p, v] = m;
      k = k.toUpperCase();
      if (/ENCODING=QUOTED-PRINTABLE/i.test(p)) {
        try {
          v = decodeURIComponent(v.replace(/=([0-9A-F]{2})/gi, "%$1"));
        } catch { /* as it is */ }
      }
      if (k === "BEGIN") c = { name: "", tel: [], email: [], address: [] };
      else if (k === "END" && c) out.push(c), (c = null);
      else if (!c) continue;
      else if (k === "FN") c.name = un(v);
      else if (k === "N" && !c.name) c.name = v.split(";").slice(0, 2).reverse().map(un).filter(Boolean).join(" ");
      else if (k === "TEL") c.tel.push(un(v).replace(/^tel:/i, ""));
      else if (k === "EMAIL") c.email.push(un(v));
      else if (k === "ADR") c.address.push(v.split(";").map(un).filter(Boolean).join(", "));
    }
    return out.filter((x) => x.name || x.tel.length || x.email.length);
  }
  B["ui-contact-picker"] = (el, api) => {
    const cfg = api.config, fields = list(cfg.fields).length ? cfg.fields.map(String) : ["name", "tel", "email"], can = "contacts" in navigator && "select" in (navigator.contacts ?? {});
    const body = D.body(el);
    let error = null, typing = false, was = null;
    const all = () => (cfg.multiple ? list(api.state[cfg.bind]).filter(isObj) : isObj(api.state[cfg.bind]) ? [api.state[cfg.bind]] : []);
    const only = (c) => Object.fromEntries(fields.map((f) => [f, c[f] ?? (f === "name" ? "" : [])]));
    function add(cs) {
      if (!cs.length) return;
      const next = cs.map(only);
      api.set(cfg.bind, cfg.multiple ? [...all(), ...next.filter((n) => !all().some((a) => JSON.stringify(a) === JSON.stringify(n)))] : next[0]);
      api.emit("pick", { contacts: next });
    }
    async function phone() {
      error = null;
      try {
        const got = await navigator.contacts.select(fields, { multiple: cfg.multiple });
        add(got.map((c) => ({ name: c.name?.[0] ?? "", tel: c.tel ?? [], email: c.email ?? [], address: (c.address ?? []).map((a) => [...(a.addressLine ?? []), a.city, a.postalCode, a.country].filter(Boolean).join(", ")) })));
      } catch (e) {
        error = D.explain(e, "contacts");
      }
      draw();
    }
    async function file(f) {
      error = null;
      try {
        const got = cards(await f.text());
        got.length ? add(cfg.multiple ? got : got.slice(0, 1)) : (error = L("There are no contacts in this file"));
      } catch (e) {
        error = String(e.message);
      }
      draw();
    }
    const names = { name: L("Name"), tel: L("Phone"), email: L("Email"), address: L("Address") };
    function draw() {
      const now = all(), label = say(api, cfg.label, cfg.multiple ? L("Choose contacts") : L("Choose a contact")), sig = JSON.stringify([now, error, typing, label]);
      if (sig === was) return;
      was = sig;
      const inputs = fields.map((f) => make("input", { "data-w-field": "", name: f, type: f === "tel" ? "tel" : f === "email" ? "email" : "text", placeholder: names[f], "aria-label": names[f], autocomplete: "off" }));
      const form = typing ? make("form", { "data-w-card": "", submit: (e) => {
        e.preventDefault();
        const c = Object.fromEntries(inputs.map((i) => [i.name, i.name === "name" ? i.value.trim() : i.value.trim() ? [i.value.trim()] : []]));
        if (!c.name && !Object.values(c).flat().some(Boolean)) return inputs[0].focus();
        typing = false;
        add([c]);
        draw();
      } }, make("div", { "data-sh-fields": "" }, inputs), make("div", { "data-w-row": "" }, btn("primary", { type: "submit" }, L("Add")), btn("ghost", { click: () => ((typing = false), draw()) }, L("Cancel")))) : null;
      const pick = make("input", { type: "file", accept: ".vcf,text/vcard,text/x-vcard", change: (e) => e.target.files[0] && file(e.target.files[0]) });
      body.replaceChildren(...[
        now.length ? make("ul", { "data-w-list": "" }, now.map((c) => make("li", null, make("span", { "data-sh-thumb": "" }, icon("user", 18)), make("div", { "data-w-main": "" }, make("span", null, c.name || list(c.tel)[0] || list(c.email)[0] || L("No name")), make("small", null, [...list(c.tel), ...list(c.email)].join(" · "))), btn("ghost", { "data-icon-only": "", "aria-label": L("Remove {name}", { name: c.name || "" }), click: () => (api.set(cfg.bind, cfg.multiple ? all().filter((x) => x !== c && JSON.stringify(x) !== JSON.stringify(c)) : null), api.emit("remove", c)) }, icon("x", 14))))) : null,
        form ?? make("div", { "data-w-row": "" },
          can ? btn("primary", { "data-pick": "", click: phone }, icon("user", 16), label) : null,
          make("label", { "data-w-btn": can ? "" : "primary" }, icon("file", 15), can ? L("From a file") : L("From a contact card"), pick),
          cfg.manual ? btn("", { "data-type": "", click: () => { typing = true; draw(); body.querySelector("input[name]")?.focus(); } }, icon("edit", 15), L("Type it")) : null),
        make("p", { "data-w-error": "", role: "alert" }, error ?? "")].filter(Boolean));
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- passkey
  const has = () => "PublicKeyCredential" in window && !!navigator.credentials;
  // The server's options and the answer travel as JSON, with their bytes in base64url.
  const creation = (o) => ({ ...o, challenge: bytes(o.challenge), user: { ...o.user, id: bytes(o.user.id) }, excludeCredentials: list(o.excludeCredentials).map((c) => ({ ...c, id: bytes(c.id) })) });
  const request = (o) => ({ ...o, challenge: bytes(o.challenge), allowCredentials: list(o.allowCredentials).map((c) => ({ ...c, id: bytes(c.id) })) });
  function answer(c) {
    if (c.toJSON) try {
      return c.toJSON();
    } catch { /* put together here */ }
    const r = c.response, out = { id: c.id, rawId: b64(c.rawId), type: c.type, clientExtensionResults: c.getClientExtensionResults?.() ?? {}, response: { clientDataJSON: b64(r.clientDataJSON) } };
    for (const k of ["attestationObject", "authenticatorData", "signature", "userHandle"]) if (r[k]) out.response[k] = b64(r[k]);
    if (r.getTransports) out.response.transports = r.getTransports();
    return out;
  }
  const rand = (n) => crypto.getRandomValues(new Uint8Array(n));
  const makeLocal = (label) => navigator.credentials.create({ publicKey: { challenge: rand(32), rp: { name: document.title || "App" }, user: { id: rand(16), name: label, displayName: label }, pubKeyCredParams: [{ type: "public-key", alg: -7 }, { type: "public-key", alg: -257 }], authenticatorSelection: { authenticatorAttachment: "platform", userVerification: "required", residentKey: "preferred" }, timeout: 60000 } });
  const useLocal = (ids) => navigator.credentials.get({ publicKey: { challenge: rand(32), allowCredentials: ids.map((id) => ({ type: "public-key", id: bytes(id), transports: ["internal"] })), userVerification: "required", timeout: 60000 } });
  B["ui-passkey"] = (el, api) => {
    const cfg = api.config, register = cfg.mode === "register", name = make("span"), error = make("p", { "data-w-error": "", role: "alert" });
    const b = btn(cfg.variant === "primary" ? "primary" : "", { "data-passkey": "", click: () => go() }, icon("key", 16), name);
    el.append(make("div", { "data-w-row": "" }, b), error);
    const store = "passkeys-local";
    let waiting = null;
    async function go(conditional) {
      error.textContent = "";
      if (!has()) return (error.textContent = D.unsupported("passkey"));
      waiting?.abort();
      waiting = new AbortController();
      const signal = waiting.signal;
      if (!conditional) b.disabled = true;
      try {
        let data;
        if (cfg.local) {
          const ids = (await O.db.get("kv", store)) ?? [];
          if (register) {
            const c = await makeLocal(String(K.value(api, cfg.name) ?? "user"));
            await O.db.put("kv", store, [...ids, b64(c.rawId)]);
            data = { ok: true, id: c.id };
          } else {
            if (!ids.length) throw new Error(L("There's no passkey on this device yet"));
            const c = await useLocal(ids);
            data = { ok: true, id: c.id };
          }
        } else {
          const raw = await post(K.value(api, cfg.optionsUrl), K.value(api, cfg.body) ?? {}), o = raw.publicKey ?? raw;
          const c = register ? await navigator.credentials.create({ publicKey: creation(o), signal }) : await navigator.credentials.get({ publicKey: request(o), signal, ...(conditional ? { mediation: "conditional" } : {}) });
          data = await post(K.value(api, cfg.verifyUrl), answer(c));
        }
        if (cfg.bind) api.set(cfg.bind, data);
        K.say(el, register ? L("The passkey was made") : L("Signed in"));
        api.emit("done", { result: data });
      } catch (e) {
        if (e.name === "AbortError" || (conditional && e.name === "NotAllowedError")) return;
        // Turned down, or left until it timed out: both look the same.
        error.textContent = e.name === "NotAllowedError" ? L("It was cancelled, or took too long. Try again.") : e.name === "InvalidStateError" ? L("This device already has a passkey here.") : D.explain(e, "passkey") ?? "";
        api.emit("failed", { name: e.name, error: error.textContent });
      } finally {
        b.disabled = false;
      }
    }
    const draw = () => (name.textContent = say(api, cfg.label, register ? L("Create a passkey") : L("Sign in with a passkey")));
    draw();
    if (!has()) b.disabled = true, (error.textContent = "", el.append(make("p", { "data-w-note": "" }, D.unsupported("passkey"))));
    else if (cfg.autofill && !register && !cfg.local) PublicKeyCredential.isConditionalMediationAvailable?.().then((ok) => ok && el.isConnected && go(true));
    D.gone(el, () => waiting?.abort());
    return { update: draw };
  };

  // ---------------------------------------------------------------- lock
  B["ui-app-lock"] = (el, api) => {
    const cfg = api.config, key = `lock:${cfg.name ?? "app"}`, N = cfg.digits, TRIES = cfg.tries ?? 5, AFTER = cfg.after ?? 5, HIDDEN = cfg.hidden ?? 30;
    const methods = list(cfg.methods).length ? cfg.methods : ["passkey", "pin"], canKey = methods.includes("passkey") && has(), canPin = methods.includes("pin") && !!crypto.subtle;
    const body = D.body(el);
    // `saved`: something was saved here, so what the store had at the start is out of date.
    let set = null, saved = false, locked = false, pin = "", step = null, first = "", note = "", last = Date.now(), away = 0, tick = 0, said = "";
    const ready = () => !!set && (set.pin || set.passkey);
    const hash = async (text, salt) => b64(await crypto.subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt: bytes(salt), iterations: 210000 }, await crypto.subtle.importKey("raw", new TextEncoder().encode(text), "PBKDF2", false, ["deriveBits"]), 256));
    const put = (v) => ((set = v), (saved = true), v ? O.db.put("kv", key, v) : O.db.del("kv", key)).catch(() => {});
    const tell = () => cfg.bindLocked && !!api.state[cfg.bindLocked] !== locked && api.set(cfg.bindLocked, locked);
    // ---- the cover
    const dots = make("div", { "data-lk-dots": "", role: "img" }), line = make("p", { "data-w-note": "", role: "alert" }), title = make("h2");
    const pad = make("div", { "data-lk-keys": "" }, [1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => K.button({ "data-k": n, click: () => press(String(n)) }, String(n))), K.button({ "data-plain": "", "data-k": "key", "aria-label": L("Use a passkey"), click: () => withKey() }, icon("key", 22)), K.button({ "data-k": 0, click: () => press("0") }, "0"), K.button({ "data-plain": "", "data-k": "back", "aria-label": L("Delete"), click: () => ((pin = pin.slice(0, -1)), drawPad()) }, icon("back", 22)));
    const big = btn("primary", { click: () => withKey() }, icon("key", 16), L("Unlock")), forgot = btn("ghost", { click: lost }, L("Forgot your PIN?")), cancel = btn("ghost", { click: () => quit() }, L("Cancel"));
    const pic = make("img", { alt: "", hidden: true }), inner = make("div", { "data-lk-in": "" }, pic, title, dots, line, pad, big, make("div", { "data-w-row": "" }, forgot, cancel));
    const cover = make("dialog", { "data-lk": "" }, inner);
    el.append(cover);
    cover.addEventListener("cancel", (e) => (e.preventDefault(), step && quit()));
    cover.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && !step) e.preventDefault();
      if (pad.hidden || e.ctrlKey || e.metaKey) return;
      if (/^\d$/.test(e.key)) e.preventDefault(), press(e.key);
      else if (e.key === "Backspace") e.preventDefault(), (pin = pin.slice(0, -1)), drawPad();
    });
    const waits = () => (set?.until && set.until > Date.now() ? Math.ceil((set.until - Date.now()) / 1000) : 0);
    function drawPad() {
      const s = waits(), need = step ? "pin" : set?.pin ? "pin" : "passkey";
      title.textContent = step === "new" ? L("Choose a PIN") : step === "again" ? L("Enter it again") : say(api, cfg.title, L("Locked"));
      dots.hidden = pad.hidden = need !== "pin";
      dots.replaceChildren(...Array.from({ length: N }, (_, i) => make("i", { "data-on": i < pin.length })));
      dots.setAttribute("aria-label", L("{n} of {of} digits", { n: pin.length, of: N }));
      pad.querySelectorAll("button").forEach((b) => (b.disabled = s > 0));
      pad.querySelector("[data-k=key]").style.visibility = !step && set?.passkey && canKey ? "" : "hidden";
      big.hidden = need === "pin";
      forgot.hidden = !!step || !set?.pin || blank(K.value(api, cfg.signOutUrl));
      cancel.hidden = !step;
      const t = s ? L("Too many tries. Wait {time}.", { time: D.time(s) }) : note;
      if (t !== said) line.textContent = said = t;
      const u = K.value(api, cfg.logo);
      pic.hidden = !!step || !D.safeUrl(u);
      if (!pic.hidden && pic.getAttribute("src") !== u) pic.src = u;
    }
    function open() {
      pin = "";
      drawPad();
      cover.setAttribute("aria-label", title.textContent);
      if (!cover.open) cover.showModal();
      (pad.hidden ? big : pad.querySelector("button")).focus({ preventScroll: true });
    }
    function lock() {
      if (!ready() || locked) return;
      locked = true;
      step = null;
      note = "";
      tell();
      open();
      api.emit("lock", {});
      if (set.passkey && canKey && document.visibilityState === "visible" && navigator.userActivation?.hasBeenActive) withKey(true);
    }
    function unlock() {
      locked = false;
      last = Date.now();
      put({ ...set, tries: 0, until: 0 });
      cover.close();
      tell();
      api.emit("unlock", {});
    }
    function quit() {
      step = null;
      first = pin = note = "";
      cover.close();
      draw();
    }
    function wrong(text) {
      pin = "";
      note = text;
      navigator.vibrate?.([40, 40, 40]);
      inner.setAttribute("data-wrong", "");
      setTimeout(() => inner.removeAttribute("data-wrong"), 400);
      drawPad();
    }
    async function press(d) {
      if (pin.length >= N || waits()) return;
      pin += d;
      note = "";
      drawPad();
      if (pin.length < N) return;
      const given = pin;
      if (step === "new") {
        if (/^(\d)\1+$/.test(given) || "01234567890".includes(given) || "09876543210".includes(given)) return wrong(L("That one's too easy to guess"));
        first = given;
        step = "again";
        pin = "";
        return drawPad();
      }
      if (step === "again") {
        if (given !== first) return (step = "new"), wrong(L("They weren't the same. Choose again."));
        const salt = b64(rand(16));
        await put({ ...(set ?? {}), pin: await hash(given, salt), salt, digits: N, tries: 0, until: 0 });
        quit();
        K.say(el, L("The PIN is set"));
        return api.emit("set", { method: "pin" });
      }
      if ((await hash(given, set.salt)) === set.pin) return unlock();
      const tries = (set.tries ?? 0) + 1;
      // Waits of half a minute, then one, two, four… after so many.
      await put({ ...set, tries, until: tries >= TRIES ? Date.now() + Math.min(3600, 30 * 2 ** (tries - TRIES)) * 1000 : 0 });
      api.emit("wrong", { tries });
      wrong(L("Wrong PIN"));
    }
    async function withKey(quiet) {
      try {
        await useLocal([set.passkey]);
        if (locked) unlock();
      } catch (e) {
        if (!quiet) (note = e.name === "NotAllowedError" ? L("It was cancelled, or took too long. Try again.") : D.explain(e, "passkey") ?? ""), drawPad();
      }
    }
    // Still locked: only signing in again (`reset`) takes the lock off.
    function lost() {
      api.emit("forgot", {});
      const u = K.value(api, cfg.signOutUrl);
      if (D.safeUrl(u)) location.assign(u);
    }
    async function reset() {
      if (!set) return;
      await put(null);
      locked = false;
      if (cover.open) cover.close();
      tell();
      draw();
    }
    async function addKey() {
      note = "";
      try {
        const c = await makeLocal(document.title || "App");
        await put({ ...(set ?? {}), passkey: b64(c.rawId) });
        api.emit("set", { method: "passkey" });
      } catch (e) {
        note = e.name === "NotAllowedError" ? L("It was cancelled, or took too long. Try again.") : D.explain(e, "passkey") ?? "";
      }
      draw();
    }
    // ---- the settings
    function draw() {
      if (locked || step) drawPad();
      if (!cfg.settings) return;
      const has = ready();
      body.replaceChildren(make("div", { "data-lk-panel": "" },
        make("div", { "data-w-row": "" }, icon("lock", 18), make("p", { "data-w-title": "" }, L("Lock this app")), make("span", { "data-w-tag": "", "data-tone": has ? "good" : null }, has ? L("On") : L("Off"))),
        make("p", { "data-w-note": "" }, has ? (AFTER ? L("It locks after {n} minutes, and once it's been out of sight.", { n: AFTER }) : L("It locks once it's been out of sight.")) : L("So that someone who picks up this device can't open it.")),
        make("div", { "data-w-row": "" },
          canPin ? btn(has ? "" : "primary", { "data-set": "pin", click: () => ((step = "new"), (note = ""), open()) }, set?.pin ? L("Change the PIN") : L("Set a PIN")) : null,
          canKey && !set?.passkey ? btn("", { "data-set": "passkey", click: addKey }, icon("key", 15), L("Use fingerprint or face")) : null,
          has ? btn("", { "data-lock": "", click: lock }, icon("lock", 15), L("Lock now")) : null,
          has ? btn("ghost", { "data-off": "", click: async () => { await put(null); api.emit("off", {}); draw(); } }, L("Turn off")) : null),
        make("p", { "data-w-error": "", role: "alert" }, step ? "" : note || (!canPin && !canKey ? D.unsupported("passkey") : ""))));
    }
    for (const ev of ["pointerdown", "keydown", "wheel", "touchstart"]) addEventListener(ev, () => (last = Date.now()), { passive: true, capture: true });
    document.addEventListener("visibilitychange", () => {
      if (!el.isConnected) return;
      if (document.visibilityState === "hidden") {
        away = Date.now();
        // At once, so that it's covered in the list of open apps.
        if (HIDDEN === 0) lock();
      } else if (away && Date.now() - away >= HIDDEN * 1000) lock();
    });
    tick = setInterval(() => {
      if (!el.isConnected) return clearInterval(tick);
      if (locked) drawPad();
      else if (AFTER && ready() && !step && Date.now() - last > AFTER * 60000) lock();
    }, 1000);
    draw();
    O.db.get("kv", key).then((v) => {
      // A slow store (a busy device) may answer after a PIN was set here: that one stands.
      if (saved) return;
      set = v ?? null;
      if (K.value(api, cfg.reset)) return reset();
      draw();
      // Opened afresh: locked.
      lock();
    }).catch(() => {});
    return { update: () => (K.value(api, cfg.reset) ? reset() : (locked || step) && drawPad()) };
  };
})();
