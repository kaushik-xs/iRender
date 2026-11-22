// irender-ui-kit: `ui-nfc`, `ui-bluetooth`, `ui-serial`, `ui-usb` and `ui-file-handler`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const list = (v) => (Array.isArray(v) ? v : []);
  const put = (api, key, v) => key && JSON.stringify(api.state[key] ?? null) !== JSON.stringify(v) && api.set(key, v);
  const error = () => make("p", { "data-w-error": "", role: "alert" });
  const states = { disconnected: () => [L("Not connected"), null], connecting: () => [L("Connecting…"), "warn"], connected: () => [L("Connected"), "good"] };
  const tag = (s) => make("span", { "data-w-tag": "", "data-tone": states[s]()[1] }, states[s]()[0]);
  // Someone who closes the list of devices hasn't failed at anything.
  const closed = (e) => e?.name === "NotFoundError" && /cancel|chooser|no (port|device) selected/i.test(e.message);
  const hex = (b) => [...new Uint8Array(b.buffer ?? b, b.byteOffset ?? 0, b.byteLength)].map((x) => x.toString(16).padStart(2, "0")).join(" ");
  const unhex = (s) => Uint8Array.from(String(s).replace(/0x/gi, "").match(/[\da-f]{2}/gi) ?? [], (h) => parseInt(h, 16));
  const SIZES = { uint8: 1, int8: 1, uint16: 2, int16: 2, uint32: 4, int32: 4, float32: 4 };
  const METHOD = { uint8: "Uint8", int8: "Int8", uint16: "Uint16", int16: "Int16", uint32: "Uint32", int32: "Int32", float32: "Float32" };
  function decode(view, o) {
    if (o.as === "text") return new TextDecoder().decode(view);
    if (o.as === "hex") return hex(view);
    // The first byte says how wide the rate is.
    if (o.as === "heart-rate") return view.getUint8(0) & 1 ? view.getUint16(1, true) : view.getUint8(1);
    const as = METHOD[o.as] ? o.as : "uint8", at = Number(o.offset) || 0;
    if (view.byteLength < at + SIZES[as]) return null;
    const v = view[`get${METHOD[as]}`](at, !o.bigEndian);
    return o.scale ? +(v * Number(o.scale)).toPrecision(12) : v;
  }
  function encode(value, as) {
    if (as === "hex") return unhex(value);
    if (METHOD[as]) {
      const d = new DataView(new ArrayBuffer(SIZES[as]));
      d[`set${METHOD[as]}`](0, Number(value) || 0, true);
      return new Uint8Array(d.buffer);
    }
    if (as === "bytes" || Array.isArray(value)) return Uint8Array.from(list(value).map(Number));
    return new TextEncoder().encode(String(value ?? ""));
  }

  // ---------------------------------------------------------------- NFC
  B["ui-nfc"] = (el, api) => {
    const cfg = api.config, body = D.body(el), err = error(), has = "NDEFReader" in window;
    el.append(err);
    let reader = null, stop = null, mode = null, was = null, done = "";
    function records(m) {
      return [...m.records].map((r) => {
        const o = { type: r.recordType };
        try {
          if (r.recordType === "text") o.text = new TextDecoder(r.encoding || "utf-8").decode(r.data);
          else if (["url", "absolute-url"].includes(r.recordType)) o.text = new TextDecoder().decode(r.data);
          else if (r.recordType === "mime") (o.mime = r.mediaType), (o.text = /json|text/.test(r.mediaType) ? new TextDecoder().decode(r.data) : hex(r.data));
          else o.text = r.data ? hex(r.data) : "";
        } catch { /* it can't be read as that */ }
        return o;
      });
    }
    const end = () => (stop?.abort(), (stop = mode = null));
    async function go(next) {
      end();
      err.textContent = done = "";
      mode = next;
      stop = new AbortController();
      draw();
      try {
        reader = new NDEFReader();
        if (next === "read") {
          reader.onreading = (e) => {
            const rs = records(e.message), v = { serial: e.serialNumber || null, records: rs, text: rs.find((r) => r.text)?.text ?? "", at: new Date().toISOString() };
            api.set(cfg.bind, v);
            if (cfg.bindList) {
              const all = list(api.state[cfg.bindList]);
              if (!all.some((x) => x.serial === v.serial && x.text === v.text)) api.set(cfg.bindList, [...all, v]);
            }
            navigator.vibrate?.(60);
            done = L("Read {value}", { value: v.text || v.serial || "" });
            K.say(el, done);
            api.emit("tag", v);
            draw();
          };
          reader.onreadingerror = () => (err.textContent = L("That tag couldn't be read. Try again."));
          await reader.scan({ signal: stop.signal });
        } else {
          const v = String(K.value(api, cfg.write) ?? "");
          await reader.write({ records: [/^https?:\/\//i.test(v) ? { recordType: "url", data: v } : { recordType: "text", data: v }] }, { signal: stop.signal });
          if (cfg.lock) await reader.makeReadOnly({ signal: stop.signal });
          navigator.vibrate?.(60);
          done = L("Written");
          K.say(el, done);
          api.emit("written", { value: v });
          end();
        }
      } catch (e) {
        err.textContent = D.explain(e, "nfc") ?? "";
        end();
      }
      draw();
    }
    function draw() {
      const w = K.value(api, cfg.write), label = K.value(api, cfg.label) ?? L("Read a tag"), sig = JSON.stringify([mode, label, blank(w), done]);
      if (sig === was) return;
      was = sig;
      const focus = body.contains(document.activeElement);
      body.replaceChildren(make("div", { "data-w-row": "" }, ...(mode ? [btn("", { "data-stop": "", click: () => (end(), draw()) }, icon("stop", 14), L("Stop")), make("span", { "data-w-note": "" }, mode === "read" ? L("Hold a tag to the back of the phone") : L("Hold the tag to write to the back of the phone"))] : [btn("primary", { "data-read": "", disabled: !has, click: () => go("read") }, icon("nfc", 16), label), cfg.write !== undefined ? btn("", { "data-write": "", disabled: !has || blank(w), click: () => go("write") }, icon("edit", 15), L("Write a tag")) : null].filter(Boolean))), make("p", { "data-w-note": "" }, done || (has ? "" : D.unsupported("nfc"))));
      if (focus) body.querySelector("button")?.focus();
    }
    D.gone(el, end);
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- what a connected thing shares
  function shell(el, api, { has, what, label, connect, disconnect, more }) {
    const cfg = api.config, body = D.body(el), err = error();
    el.append(err);
    let state = "disconnected", name = "", was = null;
    const set = (s, n) => {
      if (n !== undefined) name = n;
      if (s !== state) api.emit(s, {});
      state = s;
      put(api, cfg.bindState, s);
      draw(true);
    };
    async function go() {
      err.textContent = "";
      try {
        await connect(set);
      } catch (e) {
        set("disconnected", "");
        if (!closed(e)) (err.textContent = D.explain(e, what) ?? ""), api.emit("failed", { name: e.name, error: err.textContent });
      }
    }
    const fail = (e) => (err.textContent = D.explain(e, what) ?? "");
    const run = (c) => c().catch(fail);
    function draw(force) {
      const lab = K.value(api, cfg.label) ?? label, sig = JSON.stringify([state, name, lab, more?.sig?.() ?? null]);
      if (!force && sig === was) return more?.update?.();
      was = sig;
      const focus = body.contains(document.activeElement);
      body.replaceChildren(...[
        make("div", { "data-w-row": "" }, state === "disconnected" ? btn("primary", { "data-connect": "", disabled: !has, click: go }, icon(what === "bluetooth" ? "bluetooth" : "usb", 16), lab) : btn("", { "data-disconnect": "", click: () => run(async () => { await disconnect(); set("disconnected", ""); }) }, L("Disconnect")), tag(state), name ? make("span", { "data-w-note": "" }, name) : null),
        has ? null : make("p", { "data-w-note": "" }, D.unsupported(what)),
        ...(more?.draw?.(state, run) ?? [])].filter(Boolean));
      if (focus) body.querySelector("button")?.focus();
      more?.update?.();
    }
    D.gone(el, () => state !== "disconnected" && disconnect());
    put(api, cfg.bindState, state);
    draw(true);
    return { set, fail, draw, get state() { return state; } };
  }
  const buttons = (cmds, run) => (cmds.length ? [make("div", { "data-w-row": "" }, cmds.map((c, i) => btn("", { "data-command": i, click: () => run(c.go) }, c.label)))] : []);

  // ---------------------------------------------------------------- Bluetooth
  B["ui-bluetooth"] = (el, api) => {
    const cfg = api.config, uuid = (v) => (typeof v === "string" && /^0x/i.test(v) ? Number(v) : v);
    const values = {};
    let device = null, chars = new Map();
    const at = async (server, o) => {
      const key = `${o.service}/${o.characteristic}`;
      if (!chars.has(key)) chars.set(key, await (await server.getPrimaryService(uuid(o.service))).getCharacteristic(uuid(o.characteristic)));
      return chars.get(key);
    };
    const got = (o, view) => {
      const v = decode(view, o);
      if (v === null) return;
      values[o.key] = v;
      if (cfg.bind) api.set(cfg.bind, { ...(isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : {}), [o.key]: v });
      api.emit("value", { key: o.key, value: v });
      s.draw();
    };
    const cmds = () => list(cfg.commands).map((c) => ({ label: K.value(api, c.label) ?? L("Send"), go: async () => {
      const ch = await at(device.gatt, c), data = encode(K.value(api, c.value), c.as);
      ch.writeValueWithResponse && ch.properties?.write !== false ? await ch.writeValueWithResponse(data) : await ch.writeValueWithoutResponse(data);
      api.emit("sent", { label: c.label });
    } }));
    const s = shell(el, api, {
      has: !!navigator.bluetooth, what: "bluetooth", label: L("Connect a device"),
      async connect(set) {
        const services = [...new Set([...cfg.read, ...list(cfg.commands)].map((o) => uuid(o.service)).filter((x) => !blank(x)))];
        device = await navigator.bluetooth.requestDevice(cfg.namePrefix ? { filters: [{ namePrefix: cfg.namePrefix }], optionalServices: services } : { filters: services.map((x) => ({ services: [x] })) });
        set("connecting", device.name || L("Unnamed device"));
        device.ongattserverdisconnected = () => (chars.clear(), set("disconnected", ""));
        const server = await device.gatt.connect();
        for (const o of cfg.read) {
          const ch = await at(server, o);
          if (o.notify) {
            ch.addEventListener("characteristicvaluechanged", (e) => got(o, e.target.value));
            await ch.startNotifications();
          }
          // Some only tell when something changes.
          if (!o.notify || ch.properties?.read) await ch.readValue().then((v) => got(o, v), (e) => { if (!o.notify) throw e; });
        }
        set("connected");
      },
      async disconnect() {
        chars.clear();
        device?.gatt?.disconnect();
      },
      more: {
        sig: () => list(cfg.commands).map((c) => K.value(api, c.label)),
        draw: (state, run) => (state === "connected" ? [make("dl", null, cfg.read.map((o) => make("div", null, make("dt", null, o.label ?? o.key), make("dd", { "data-key": o.key })))), ...buttons(cmds(), run)] : []),
        update: () => el.querySelectorAll("[data-key]").forEach((d) => {
          const o = cfg.read.find((x) => x.key === d.getAttribute("data-key")), v = values[o.key];
          d.textContent = v === undefined ? "–" : `${typeof v === "number" ? +v.toFixed(3) : v}${o.unit ? ` ${o.unit}` : ""}`;
        }),
      },
    });
    return { update: () => s.draw() };
  };

  // ---------------------------------------------------------------- serial
  B["ui-serial"] = (el, api) => {
    const cfg = api.config, max = cfg.max ?? 200, END = { lf: "\n", crlf: "\r\n", cr: "\r", none: "" }[cfg.end ?? "lf"];
    let port = null, reader = null, over = false, lines = [];
    const log = make("pre", { "data-hw-log": "", tabindex: 0, "aria-label": L("Lines") });
    const parse = (t) => {
      if (cfg.parse === "number") {
        const m = /-?\d+(?:\.\d+)?/.exec(t);
        return m ? Number(m[0]) : null;
      }
      if (cfg.parse === "csv") return t.split(",").map((x) => (x.trim() !== "" && !isNaN(x) ? Number(x) : x.trim()));
      if (cfg.parse === "json") try {
        return JSON.parse(t);
      } catch {
        return undefined;
      }
      return t;
    };
    function line(text, out) {
      lines = [...lines, { at: new Date().toISOString(), text, out: !!out }].slice(-max);
      if (cfg.bindLog) api.set(cfg.bindLog, lines);
      if (cfg.log) {
        const end = log.scrollHeight - log.scrollTop - log.clientHeight < 30;
        log.append(`${out ? "› " : ""}${text}\n`);
        while (log.childNodes.length > max) log.firstChild.remove();
        if (end) log.scrollTop = log.scrollHeight;
      }
      if (out) return;
      const v = parse(text);
      if (v !== undefined && v !== null && cfg.bind) api.set(cfg.bind, v);
      api.emit("line", { text, value: v ?? null });
    }
    async function listen(set) {
      const text = new TextDecoder();
      let left = "";
      while (port?.readable && !over) {
        reader = port.readable.getReader();
        try {
          for (;;) {
            const { value, done } = await reader.read();
            if (done) break;
            left += text.decode(value, { stream: true });
            const got = left.split(/\r\n|\n|\r(?!$)/);
            // The last piece isn't a whole line yet.
            left = got.pop();
            got.filter((t) => t !== "").forEach((t) => line(t));
          }
        } catch (e) {
          // Garbled bytes are passed over; an unplugged port is the end.
          if (!port?.readable) break;
        } finally {
          try { reader.releaseLock(); } catch { /* it's gone */ }
        }
      }
      if (!over) set("disconnected", "");
    }
    async function send(text) {
      const w = port.writable.getWriter();
      try {
        await w.write(new TextEncoder().encode(text + END));
      } finally {
        w.releaseLock();
      }
      line(text, true);
      api.emit("sent", { text });
    }
    const cmds = () => list(cfg.commands).map((c) => ({ label: K.value(api, c.label) ?? K.value(api, c.text), go: () => send(String(K.value(api, c.text) ?? "")) }));
    const s = shell(el, api, {
      has: !!navigator.serial, what: "serial", label: L("Connect a port"),
      async connect(set) {
        over = false;
        port = await navigator.serial.requestPort(list(cfg.filters).length ? { filters: cfg.filters } : {});
        set("connecting");
        await port.open({ baudRate: cfg.baud ?? 9600 });
        port.ondisconnect = () => ((over = true), (port = null), set("disconnected", ""));
        set("connected", L("{n} baud", { n: cfg.baud ?? 9600 }));
        listen(set);
      },
      async disconnect() {
        over = true;
        await reader?.cancel().catch(() => {});
        await port?.close().catch(() => {});
        port = null;
      },
      more: {
        sig: () => list(cfg.commands).map((c) => K.value(api, c.label)),
        draw: (state, run) => {
          const field = make("input", { "data-w-field": "", "aria-label": L("Line to send"), placeholder: L("Line to send"), autocomplete: "off" });
          return [...(cfg.log ? [log] : []), ...(state === "connected" ? [...buttons(cmds(), run), ...(cfg.send ? [make("form", { "data-hw-send": "", submit: (e) => { e.preventDefault(); const t = field.value; field.value = ""; run(() => send(t)); } }, field, btn("", { type: "submit" }, L("Send")))] : [])] : [])];
        },
      },
    });
    return { update: () => s.draw() };
  };

  // ---------------------------------------------------------------- USB
  B["ui-usb"] = (el, api) => {
    const cfg = api.config, as = cfg.as ?? "text";
    let device = null, ins = null, outs = null, num = null, over = false;
    const read = async (set) => {
      while (device?.opened && ins && !over) {
        try {
          const r = await device.transferIn(ins.endpointNumber, ins.packetSize || 64);
          if (r.status === "stall") await device.clearHalt("in", ins.endpointNumber);
          if (r.data?.byteLength) {
            const v = as === "hex" ? hex(r.data) : as === "bytes" ? [...new Uint8Array(r.data.buffer)] : new TextDecoder().decode(r.data);
            if (cfg.bind) api.set(cfg.bind, v);
            api.emit("data", { value: v });
          }
        } catch {
          break;
        }
      }
      if (!over && !device?.opened) set("disconnected", "");
    };
    const cmds = () => list(cfg.commands).map((c) => ({ label: K.value(api, c.label) ?? L("Send"), go: async () => {
      if (!outs) throw new Error(L("This device takes nothing"));
      const r = await device.transferOut(outs.endpointNumber, encode(K.value(api, c.value), c.as ?? as));
      if (r.status !== "ok") throw new Error(L("The device didn't take it"));
      api.emit("sent", { label: c.label });
    } }));
    const s = shell(el, api, {
      has: !!navigator.usb, what: "usb", label: L("Connect a device"),
      async connect(set) {
        over = false;
        device = await navigator.usb.requestDevice({ filters: list(cfg.filters).map((f) => Object.fromEntries(Object.entries(f).map(([k, v]) => [k, typeof v === "string" && /^0x/i.test(v) ? Number(v) : v]))) });
        set("connecting", device.productName || L("Unnamed device"));
        await device.open();
        if (!device.configuration) await device.selectConfiguration(1);
        const all = device.configuration.interfaces, want = cfg.interface !== undefined ? all.filter((i) => i.interfaceNumber === cfg.interface) : all;
        num = null;
        let why = null;
        for (const i of want) {
          try {
            await device.claimInterface(i.interfaceNumber);
            num = i.interfaceNumber;
            const es = i.alternate.endpoints;
            ins = es.find((e) => e.direction === "in" && e.type !== "isochronous") ?? null;
            outs = es.find((e) => e.direction === "out" && e.type !== "isochronous") ?? null;
            break;
          } catch (e) {
            why = e;
          }
        }
        if (num === null) {
          await device.close();
          throw why ?? new Error(L("This device has nothing that can be used"));
        }
        put(api, cfg.bindDevice, { name: device.productName ?? null, maker: device.manufacturerName ?? null, serial: device.serialNumber ?? null, vendorId: device.vendorId, productId: device.productId });
        set("connected");
        read(set);
      },
      async disconnect() {
        over = true;
        await device?.close().catch(() => {});
        device = null;
      },
      more: { sig: () => list(cfg.commands).map((c) => K.value(api, c.label)), draw: (state, run) => (state === "connected" ? buttons(cmds(), run) : []) },
    });
    navigator.usb?.addEventListener("disconnect", (e) => e.device === device && ((over = true), (device = null), s.set("disconnected", "")));
    return { update: () => s.draw() };
  };

  // ---------------------------------------------------------------- files
  B["ui-file-handler"] = (el, api) => {
    const cfg = api.config, as = cfg.as ?? "text", body = D.body(el), err = error();
    el.append(err);
    let handle = null, name = null, kept = null, was = null;
    const accept = isObj(cfg.accept) ? Object.fromEntries(Object.entries(cfg.accept).map(([t, e]) => [t, list(e).length ? list(e) : [e].flat()])) : null;
    const types = accept ? [{ accept }] : undefined, endings = accept ? Object.entries(accept).flat(2).join() : null;
    const text = (v) => (v === null || v === undefined ? "" : typeof v === "string" ? v : JSON.stringify(v, null, 2));
    async function take(f, h) {
      err.textContent = "";
      try {
        const v = { name: f.name, type: f.type || "", size: f.size };
        if (as === "data") v.url = await D.dataUrl(f);
        else if (as === "json") v.json = JSON.parse(await f.text());
        else v.text = await f.text();
        handle = h ?? null;
        name = f.name;
        kept = as === "text" ? v.text : as === "json" ? text(v.json) : null;
        api.set(cfg.bind, v);
        api.emit("open", v);
      } catch (e) {
        err.textContent = as === "json" && e instanceof SyntaxError ? L("{name} isn't JSON", { name: f.name }) : String(e.message);
      }
      draw();
    }
    async function pick() {
      if (!window.showOpenFilePicker) return input.click();
      try {
        const [h] = await showOpenFilePicker({ types, excludeAcceptAllOption: false });
        await take(await h.getFile(), h);
      } catch (e) {
        if (e.name !== "AbortError") input.click();
      }
    }
    const input = make("input", { type: "file", accept: endings, hidden: true, "aria-hidden": "true", tabindex: -1, change: () => { const f = input.files[0]; input.value = ""; f && take(f); } });
    el.append(input);
    async function save(again) {
      err.textContent = "";
      const v = K.value(api, cfg.save), data = typeof v === "string" && v.startsWith("data:") && as === "data" ? D.toBlob(v) : text(v), file = name ?? String(K.value(api, cfg.name) ?? "untitled.txt");
      try {
        if (!handle && window.showSaveFilePicker) handle = await showSaveFilePicker({ suggestedName: file, types });
        if (handle) {
          const w = await handle.createWritable();
          await w.write(data);
          await w.close();
          name = handle.name;
        } else window.__irenderSheets.save(file, new Blob([data]));
        kept = text(v);
        K.say(el, L("Saved"));
        api.emit("save", { name: name ?? file });
      } catch (e) {
        if (e.name === "AbortError") return;
        // A picker that isn't let here: a download is.
        if (!again && e.name === "SecurityError") return (handle = null), window.__irenderSheets.save(file, new Blob([data]));
        err.textContent = D.explain(e, "files") ?? "";
      }
      draw();
    }
    function draw() {
      const v = cfg.save === undefined ? null : K.value(api, cfg.save), dirty = cfg.save !== undefined && (kept ?? "") !== text(v), label = K.value(api, cfg.label) ?? L("Open a file"), sig = JSON.stringify([name, dirty, label]);
      put(api, cfg.bindDirty, dirty);
      if (sig === was) return;
      was = sig;
      const focus = body.contains(document.activeElement) ? document.activeElement.getAttribute("data-f") : null;
      body.replaceChildren(make("div", { "data-w-row": "" }, btn("", { "data-f": "open", click: pick }, icon("file", 15), label), cfg.save === undefined ? null : btn(dirty ? "primary" : "", { "data-f": "save", click: () => save() }, icon("download", 15), L("Save")), name ? make("span", { "data-w-note": "" }, name) : null, dirty && name ? make("span", { "data-w-tag": "", "data-tone": "warn" }, L("Not saved")) : null));
      if (focus) body.querySelector(`[data-f=${focus}]`)?.focus();
    }
    // Opened with the installed app.
    window.launchQueue?.setConsumer(async (p) => {
      for (const h of list(p.files).slice(0, 1)) if (h.kind === "file") await take(await h.getFile(), h);
    });
    draw();
    return { update: draw };
  };
})();
