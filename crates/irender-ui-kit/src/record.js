// irender-ui-kit: `ui-audio-recorder`, `ui-video-recorder`, `ui-voice-input`,
// `ui-screen-capture`, `ui-read-aloud`, `ui-wake-lock` and `ui-swipe-actions`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const list = (v) => (Array.isArray(v) ? v : []);
  const kinds = (video) => (video ? ["video/mp4;codecs=avc1.42E01E,mp4a.40.2", "video/webm;codecs=vp9,opus", "video/webm;codecs=vp8,opus", "video/webm", "video/mp4"] : ["audio/webm;codecs=opus", "audio/mp4", "audio/ogg;codecs=opus", "audio/webm"]).find((t) => window.MediaRecorder?.isTypeSupported(t));
  const media = (u) => (typeof u === "string" && /^(data:(audio|video|image)\/|blob:|https?:|\/|\.)/i.test(u) ? u : null);
  // Records a stream until it's stopped or has gone on for `max` seconds.
  function recorder(stream, { video, bitrate, max, onTick, onEnd }) {
    const type = kinds(video), rec = new MediaRecorder(stream, { ...(type ? { mimeType: type } : {}), ...(video ? { videoBitsPerSecond: bitrate } : { audioBitsPerSecond: bitrate }) });
    const parts = [];
    let before = 0, since = Date.now(), timer = 0, dropped = false;
    const seconds = () => before + (since ? (Date.now() - since) / 1000 : 0);
    rec.ondataavailable = (e) => e.data.size && parts.push(e.data);
    rec.onstop = async () => {
      clearInterval(timer);
      if (dropped) return onEnd(null);
      const blob = new Blob(parts, { type: (rec.mimeType || type || "").split(";")[0] }), at = new Date(), ext = blob.type.split("/")[1] || "bin";
      onEnd({ id: K.id(), url: await D.dataUrl(blob), name: `${video ? "video" : "audio"}-${D.stamp(at)}.${ext === "mp4" && !video ? "m4a" : ext}`, type: blob.type, size: blob.size, seconds: Math.round(seconds() * 10) / 10, at: at.toISOString() });
    };
    const bank = () => {
      if (since) before += (Date.now() - since) / 1000;
      since = 0;
    };
    rec.start(1000);
    timer = setInterval(() => {
      onTick(seconds(), rec.state);
      if (seconds() >= max && rec.state !== "inactive") stop();
    }, 100);
    const stop = () => {
      if (rec.state === "inactive") return;
      bank();
      rec.stop();
    };
    return { stop, drop: () => ((dropped = true), stop()), toggle: () => (rec.state === "recording" ? (bank(), rec.pause()) : ((since = Date.now()), rec.resume())), get state() { return rec.state; }, seconds };
  }
  const wave = (canvas, peaks, done, color) => {
    const r = Math.max(1, devicePixelRatio || 1), w = canvas.clientWidth, h = canvas.clientHeight;
    if (!w) return;
    canvas.width = w * r;
    canvas.height = h * r;
    const g = canvas.getContext("2d"), n = Math.max(1, Math.floor(w / 4));
    g.scale(r, r);
    g.fillStyle = color;
    for (let i = 0; i < n; i++) {
      // The last of them as it's recorded, all of them when it's played.
      const p = done === null ? peaks[peaks.length - n + i] ?? 0 : peaks.length ? peaks[Math.floor((i / n) * peaks.length)] : 0, bar = Math.max(2, p * (h - 4));
      g.globalAlpha = done === null || i / n <= done ? 1 : 0.35;
      g.fillRect(i * 4, (h - bar) / 2, 2.4, bar);
    }
  };

  // ---------------------------------------------------------------- audio
  B["ui-audio-recorder"] = (el, api) => {
    const cfg = api.config, max = cfg.max ?? 300;
    const body = D.body(el);
    let stream = null, rec = null, peaks = [], ctx = null, error = null, raf = 0, was = null;
    const players = new Map();
    const all = () => (cfg.multiple ? list(api.state[cfg.bind]).filter(isObj) : isObj(api.state[cfg.bind]) ? [api.state[cfg.bind]] : []);
    const off = () => !!K.value(api, cfg.disabled);
    const canvas = make("canvas", { "data-rc-wave": "", role: "img", "aria-label": L("What's heard") }), time = make("span", { "data-rc-time": "", role: "timer" });
    function end() {
      cancelAnimationFrame(raf);
      stream?.getTracks().forEach((t) => t.stop());
      ctx?.close().catch(() => {});
      stream = ctx = rec = null;
    }
    async function start() {
      error = null;
      if (!navigator.mediaDevices?.getUserMedia || !window.MediaRecorder) return (error = D.unsupported("microphone")), draw();
      try {
        stream = await navigator.mediaDevices.getUserMedia({ audio: { echoCancellation: true, noiseSuppression: true } });
        if (!el.isConnected) return end();
        peaks = [];
        rec = recorder(stream, {
          video: false, bitrate: cfg.bitrate ?? 48000, max,
          onTick: (s) => (time.textContent = `${D.time(s)} / ${D.time(max)}`),
          onEnd: (v) => {
            const heard = peaks;
            end();
            if (v) {
              // About a hundred bars, for drawing it later.
              const n = Math.min(100, heard.length), p = Array.from({ length: n }, (_, i) => +Math.max(...heard.slice(Math.floor((i / n) * heard.length), Math.max(Math.floor(((i + 1) / n) * heard.length), Math.floor((i / n) * heard.length) + 1))).toFixed(2));
              const value = { ...v, peaks: p };
              api.set(cfg.bind, cfg.multiple ? [...all(), value] : value);
              api.emit("record", value);
              K.say(el, L("Recorded"));
            }
            draw();
          },
        });
        ctx = new (window.AudioContext || window.webkitAudioContext)();
        const an = ctx.createAnalyser(), buf = new Uint8Array((an.fftSize = 1024));
        ctx.createMediaStreamSource(stream).connect(an);
        let last = 0;
        const look = (t) => {
          if (!rec) return;
          if (t - last > 60 && rec.state === "recording") {
            last = t;
            an.getByteTimeDomainData(buf);
            peaks.push(Math.min(1, Math.max(...buf.map((b) => Math.abs(b - 128))) / 100));
            wave(canvas, peaks, null, getComputedStyle(canvas).color);
          }
          raf = requestAnimationFrame(look);
        };
        raf = requestAnimationFrame(look);
        api.emit("start", {});
      } catch (e) {
        end();
        error = D.explain(e, "microphone");
        api.emit("failed", { name: e.name, error });
      }
      draw();
      body.querySelector("[data-stop]")?.focus();
    }
    function player(v) {
      let p = players.get(v.id);
      if (p) return p;
      const a = new Audio(media(v.url) ?? ""), play = btn("", { "data-play": "", "data-icon-only": "", click: () => (a.paused ? a.play().catch(() => {}) : a.pause()) }), c = make("canvas", { "data-rc-wave": "", role: "slider", tabindex: 0, "aria-label": L("Position"), "aria-valuemin": 0, "aria-valuemax": Math.round(v.seconds ?? 0) }), t = make("span", { "data-rc-time": "" });
      // A recording that's streamed doesn't say how long it is.
      const of = () => (Number.isFinite(a.duration) ? a.duration : v.seconds) || 1;
      const show = () => {
        play.replaceChildren(icon(a.paused ? "play" : "pause", 16));
        play.setAttribute("aria-label", a.paused ? L("Play") : L("Pause"));
        t.textContent = D.time(a.paused && !a.currentTime ? v.seconds ?? 0 : a.currentTime);
        c.setAttribute("aria-valuenow", Math.round(a.currentTime));
        wave(c, list(v.peaks), K.clamp(a.currentTime / of(), 0, 1), getComputedStyle(c).color);
      };
      const go = (x) => {
        try {
          a.currentTime = K.clamp(x, 0, of() - 0.05);
        } catch { /* it isn't ready */ }
        show();
      };
      c.addEventListener("click", (e) => go(((e.clientX - c.getBoundingClientRect().left) / c.clientWidth) * of()));
      c.addEventListener("keydown", (e) => {
        const by = { ArrowRight: 5, ArrowLeft: -5 }[e.key];
        if (by) e.preventDefault(), go(a.currentTime + by);
      });
      for (const n of ["play", "pause", "timeupdate", "ended"]) a.addEventListener(n, () => (n === "play" && players.forEach((o) => o.a !== a && o.a.pause()), show()));
      const drop = btn("ghost", { "data-drop": "", "data-icon-only": "", "aria-label": L("Delete the recording"), click: () => { a.pause(); players.delete(v.id); api.set(cfg.bind, cfg.multiple ? all().filter((x) => x.id !== v.id) : null); api.emit("remove", v); draw(); } }, icon("trash", 15));
      new ResizeObserver(show).observe(c);
      p = { a, show, row: make("div", { "data-w-row": "", "data-audio": v.id, style: "flex-wrap:nowrap" }, play, c, t, drop), drop };
      players.set(v.id, p);
      return p;
    }
    function draw() {
      const now = all(), dis = off(), label = K.value(api, cfg.label) ?? (now.length && !cfg.multiple ? L("Record again") : L("Record")), sig = JSON.stringify([now.map((v) => v.id), dis, label, error, !!rec, rec?.state]);
      if (sig === was) return;
      was = sig;
      for (const [id, p] of players) if (!now.some((v) => v.id === id)) p.a.pause(), players.delete(id);
      body.replaceChildren(...now.map((v) => {
        const p = player(v);
        p.drop.hidden = dis;
        return p.row;
      }), rec ? make("div", { "data-w-row": "", style: "flex-wrap:nowrap" }, make("span", { "data-rc-dot": "", "data-on": rec.state === "recording" }), canvas, time, btn("", { "data-icon-only": "", "aria-label": rec.state === "paused" ? L("Go on") : L("Pause"), click: () => (rec.toggle(), draw()) }, icon(rec.state === "paused" ? "mic" : "pause", 16)), btn("primary", { "data-stop": "", click: () => rec.stop() }, icon("stop", 14), L("Done")), btn("ghost", { "data-icon-only": "", "aria-label": L("Throw it away"), click: () => rec.drop() }, icon("x", 15))) : make("div", { "data-w-row": "" }, dis ? null : btn(now.length && !cfg.multiple ? "" : "primary", { "data-record": "", click: start }, icon("mic", 16), label)), make("p", { "data-w-error": "", role: "alert" }, error ?? ""));
      now.forEach((v) => player(v).show());
    }
    D.gone(el, () => (rec?.drop(), end(), players.forEach((p) => p.a.pause())));
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- video and screen
  function film(el, api, { screen }) {
    const cfg = api.config, max = cfg.max ?? (screen ? 60 : 30), body = make("div", { "data-w-row": "" }), error = make("p", { "data-w-error": "", role: "alert" });
    const modes = screen ? list(cfg.modes).length ? cfg.modes : ["picture", "video"] : ["video"];
    const video = make("video", { playsinline: true, "aria-label": screen ? L("What's shared") : L("Camera") }), shown = make("video", { playsinline: true, controls: true, hidden: true }), pic = make("img", { alt: L("What was taken"), hidden: true });
    const badge = make("div", { "data-rc-badge": "", role: "timer", hidden: true }, make("span", { "data-rc-dot": "", "data-on": "" }), make("span")), view = make("div", { "data-rc-view": "", hidden: true }, video, shown, pic, badge);
    video.muted = true;
    el.append(view, body, error);
    let stream = null, rec = null, facing = cfg.facing === "back" ? "environment" : "user", busy = false;
    const now = () => (isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : null);
    const off = () => !!K.value(api, cfg.disabled);
    const can = screen ? !!navigator.mediaDevices?.getDisplayMedia : !!navigator.mediaDevices?.getUserMedia && !!window.MediaRecorder;
    function end() {
      stream?.getTracks().forEach((t) => t.stop());
      stream = rec = null;
      video.srcObject = null;
    }
    async function get() {
      if (screen) return navigator.mediaDevices.getDisplayMedia({ video: true, audio: cfg.audio });
      const want = { video: { facingMode: { ideal: facing }, height: { ideal: cfg.height ?? 720 } }, audio: cfg.audio };
      return navigator.mediaDevices.getUserMedia(want);
    }
    async function open() {
      if (busy) return false;
      busy = true;
      error.textContent = "";
      try {
        end();
        const s = await get();
        if (!el.isConnected) return s.getTracks().forEach((t) => t.stop()), false;
        stream = s;
        s.getVideoTracks()[0].addEventListener("ended", () => stream === s && (rec ? rec.stop() : (end(), draw())));
        video.srcObject = s;
        if (!screen) video.toggleAttribute("data-front", (s.getVideoTracks()[0].getSettings?.().facingMode ?? "user") === "user");
        draw();
        await video.play().catch(() => {});
        return true;
      } catch (e) {
        end();
        // Not picking a screen says the same as not being allowed one.
        error.textContent = screen && e.name === "NotAllowedError" ? L("Nothing was shared") : D.explain(e, screen ? "screen" : "camera") ?? "";
        api.emit("failed", { name: e.name, error: error.textContent });
        return false;
      } finally {
        busy = false;
        draw();
      }
    }
    function keep(v, what) {
      api.set(cfg.bind, v);
      api.emit(what, v);
      K.say(el, what === "picture" ? L("Picture taken") : L("Recorded"));
    }
    function record() {
      rec = recorder(stream, { video: true, bitrate: cfg.bitrate ?? (screen ? 2500000 : 1200000), max, onTick: (s) => (badge.lastChild.textContent = `${D.time(s)} / ${D.time(max)}`), onEnd: (v) => { end(); if (v) keep(v, "record"); draw(); } });
      badge.lastChild.textContent = `0:00 / ${D.time(max)}`;
      api.emit("start", {});
      draw();
      body.querySelector("[data-stop]")?.focus();
    }
    async function shot() {
      if (!(await open())) return;
      // The picker's own window is gone a moment later.
      await new Promise((r) => setTimeout(r, 350));
      const c = make("canvas", { width: video.videoWidth, height: video.videoHeight });
      c.getContext("2d").drawImage(video, 0, 0);
      const url = c.toDataURL("image/png"), at = new Date();
      end();
      keep({ id: K.id(), url, name: `screen-${D.stamp(at)}.png`, type: "image/png", size: Math.round(((url.length - 22) * 3) / 4), width: c.width, height: c.height, at: at.toISOString() }, "picture");
      draw();
    }
    const clear = () => (api.set(cfg.bind, null), api.emit("remove", {}), draw());
    let was = null;
    function draw() {
      const v = now(), dis = off(), label = K.value(api, cfg.label), sig = JSON.stringify([v?.id, v?.url?.length, dis, label, !!stream, rec?.state ?? null]);
      if (sig === was) return;
      was = sig;
      const isPic = /^image\//.test(v?.type ?? ""), src = v && !stream ? media(v.url) : null;
      view.hidden = !stream && !src;
      video.hidden = !stream;
      shown.hidden = !src || isPic;
      pic.hidden = !src || !isPic;
      if (src && isPic) pic.src = src;
      else if (src && shown.getAttribute("src") !== src) shown.src = src;
      if (!src) shown.pause();
      badge.hidden = !rec;
      badge.firstChild.toggleAttribute("data-on", rec?.state === "recording");
      const focus = body.contains(document.activeElement);
      body.replaceChildren(...(rec
        ? [btn("danger", { "data-stop": "", click: () => rec.stop() }, icon("stop", 14), L("Stop")), btn("", { click: () => (rec.toggle(), draw()) }, icon(rec.state === "paused" ? "play" : "pause", 15), rec.state === "paused" ? L("Go on") : L("Pause")), btn("ghost", { click: () => rec.drop() }, L("Throw it away"))]
        : stream && !screen ? [btn("danger", { "data-record": "", click: record }, icon("circle", 14), L("Record")), btn("", { click: () => ((facing = facing === "user" ? "environment" : "user"), open()) }, icon("flip", 15), L("Switch camera")), btn("ghost", { click: () => (end(), draw()) }, L("Close"))]
        : dis ? []
        : screen ? [...(modes.includes("picture") ? [btn("primary", { "data-picture": "", disabled: !can, click: shot }, icon("screen", 16), L("Take a picture of a screen"))] : []), ...(modes.includes("video") ? [btn(modes.includes("picture") ? "" : "primary", { "data-record": "", disabled: !can || !window.MediaRecorder, click: async () => (await open()) && record() }, icon("video", 16), L("Record a screen"))] : []), v ? btn("ghost", { click: clear }, L("Delete")) : null]
        : [btn(v ? "" : "primary", { "data-open": "", disabled: !can, click: open }, icon("video", 16), label ?? (v ? L("Take it again") : L("Open the camera"))), v ? btn("ghost", { "data-drop": "", click: clear }, icon("trash", 15), L("Delete")) : null]).filter(Boolean));
      if (focus) body.querySelector("button")?.focus();
    }
    if (!can) error.textContent = D.unsupported(screen ? "screen" : "camera");
    if (!screen) document.addEventListener("visibilitychange", () => document.visibilityState === "hidden" && stream && !rec && (end(), draw()));
    D.gone(el, () => (rec?.drop(), end()));
    draw();
    return { update: draw };
  }
  B["ui-video-recorder"] = (el, api) => film(el, api, { screen: false });
  B["ui-screen-capture"] = (el, api) => film(el, api, { screen: true });

  // ---------------------------------------------------------------- speech to text
  B["ui-voice-input"] = (el, api) => {
    const cfg = api.config, SR = () => window.SpeechRecognition || window.webkitSpeechRecognition, send = () => K.value(api, cfg.transcribeUrl);
    let on = false, rec = null, base = "", said = "", stream = null, busy = false;
    const name = make("span"), b = btn("", { "data-speak": "", "aria-pressed": "false", "data-icon-only": cfg.iconOnly, click: () => (on ? stop() : start()) }), text = make("div", { "data-vi-text": "", "aria-live": "polite" }), error = make("p", { "data-w-error": "", role: "alert" });
    el.append(make("div", { "data-w-row": "" }, b), text, error);
    const join = (a, c) => (a && c ? `${a.replace(/\s+$/, "")} ${c}` : a || c);
    const lang = () => K.value(api, cfg.lang) || document.documentElement.lang || navigator.language;
    function put(v) {
      const out = v.trim();
      if (out !== (api.state[cfg.bind] ?? "")) api.set(cfg.bind, out);
    }
    function start() {
      error.textContent = "";
      base = cfg.append ? String(api.state[cfg.bind] ?? "") : "";
      said = "";
      if (SR()) return listen();
      if (send() && navigator.mediaDevices?.getUserMedia && window.MediaRecorder) return record();
      error.textContent = L("This browser can't turn speech into text. The keyboard's own microphone may.");
    }
    function listen() {
      rec = new (SR())();
      rec.lang = lang();
      rec.interimResults = true;
      rec.continuous = cfg.continuous;
      rec.onresult = (e) => {
        let sure = "", maybe = "";
        for (const r of e.results) r.isFinal ? (sure = join(sure, r[0].transcript.trim())) : (maybe = join(maybe, r[0].transcript.trim()));
        said = sure;
        text.textContent = maybe;
        if (sure) put(join(base, sure));
      };
      rec.onerror = (e) => {
        if (e.error === "no-speech") error.textContent = L("Nothing was heard");
        else if (e.error !== "aborted") error.textContent = e.error === "not-allowed" || e.error === "service-not-allowed" ? D.explain({ name: "NotAllowedError" }, "microphone") : e.error === "network" ? L("Turning speech into text needs a connection here") : L("The microphone couldn't be used");
        api.emit("failed", { error: e.error });
      };
      rec.onend = () => {
        // Some stop by themselves after a while.
        if (on && cfg.continuous && el.isConnected && !error.textContent) {
          base = cfg.append || said ? join(base, said) : base;
          said = "";
          try {
            return rec.start();
          } catch { /* it's over */ }
        }
        on = false;
        text.textContent = "";
        draw();
        api.emit("end", { text: api.state[cfg.bind] ?? "" });
      };
      try {
        rec.start();
        on = true;
        api.emit("start", {});
      } catch (e) {
        error.textContent = String(e.message);
      }
      draw();
    }
    async function record() {
      try {
        stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      } catch (e) {
        return (error.textContent = D.explain(e, "microphone") ?? ""), draw();
      }
      on = true;
      rec = recorder(stream, { video: false, bitrate: 48000, max: 120, onTick: (s) => (text.textContent = D.time(s)), onEnd: async (v) => {
        stream.getTracks().forEach((t) => t.stop());
        on = false;
        text.textContent = "";
        busy = true;
        draw();
        try {
          const f = new FormData();
          f.append("audio", D.toBlob(v.url), v.name);
          f.append("lang", lang());
          const res = await fetch(send(), { method: "POST", body: f, headers: { accept: "application/json" } });
          if (!res.ok) throw new Error(`${res.status} ${res.statusText}`.trim());
          put(join(base, String((await res.json()).text ?? "")));
        } catch (e) {
          error.textContent = String(e.message);
        }
        busy = false;
        draw();
        api.emit("end", { text: api.state[cfg.bind] ?? "" });
      } });
      draw();
    }
    function stop() {
      on = false;
      rec?.stop();
      draw();
    }
    function draw() {
      const label = on ? L("Stop") : busy ? L("Working it out…") : K.value(api, cfg.label) ?? L("Speak");
      b.setAttribute("aria-pressed", String(on));
      b.toggleAttribute("data-listening", on);
      b.disabled = busy;
      if (name.textContent !== label) {
        name.textContent = label;
        b.replaceChildren(icon(on ? "stop" : "mic", 16), ...(cfg.iconOnly ? [] : [name]));
        if (cfg.iconOnly) b.setAttribute("aria-label", label), (b.title = label);
      }
    }
    D.gone(el, () => ((on = false), rec?.abort?.(), stream?.getTracks().forEach((t) => t.stop())));
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- read aloud
  B["ui-read-aloud"] = (el, api) => {
    const cfg = api.config;
    let state = "stopped", at = 0, parts = [], rate = cfg.rate ?? 1, turn = 0;
    const row = make("div", { "data-w-row": "" }), note = make("p", { "data-w-note": "" });
    el.append(row, note);
    const tell = (s) => {
      state = s;
      if (cfg.bindState && api.state[cfg.bindState] !== s) api.set(cfg.bindState, s);
      api.emit(s, {});
      draw();
    };
    // Sentences, with where each is in the page when it's read from there.
    function split() {
      const cut = (t) => t.match(/[^.!?。！？\n]+[.!?。！？]*["”’)]*\s*|\n+/g) ?? [];
      if (!cfg.target) return cut(String(K.value(api, cfg.text) ?? "")).map((t) => ({ text: t.trim() })).filter((p) => p.text);
      let t = null;
      try {
        t = document.querySelector(cfg.target);
      } catch { /* not a selector */ }
      if (!t) return [];
      const out = [], walk = document.createTreeWalker(t, NodeFilter.SHOW_TEXT, { acceptNode: (n) => (n.parentElement.closest("script, style, template, [aria-hidden=true], [hidden]") || !n.parentElement.getClientRects().length ? 2 : 1) });
      for (let n = walk.nextNode(); n; n = walk.nextNode()) {
        let i = 0;
        for (const s of cut(n.data)) {
          if (s.trim()) {
            const r = document.createRange();
            r.setStart(n, i);
            r.setEnd(n, Math.min(n.data.length, i + s.length));
            out.push({ text: s.trim(), range: r });
          }
          i += s.length;
        }
      }
      return out;
    }
    const mark = (r) => {
      if (!window.CSS?.highlights || !window.Highlight) return;
      r ? CSS.highlights.set("ui-reading", new Highlight(r)) : CSS.highlights.delete("ui-reading");
    };
    function say() {
      const S = window.speechSynthesis, p = parts[at], mine = ++turn;
      if (!p) return (at = 0), mark(null), tell("stopped");
      const u = new SpeechSynthesisUtterance(p.text), want = String(K.value(api, cfg.voice) ?? "").toLowerCase(), lang = K.value(api, cfg.lang) || document.documentElement.lang;
      const voice = want ? S.getVoices().find((v) => v.name.toLowerCase().startsWith(want)) : null;
      if (voice) u.voice = voice;
      if (lang) u.lang = lang;
      u.rate = rate;
      mark(p.range);
      u.onend = () => mine === turn && state === "playing" && (at++, say());
      u.onerror = (e) => mine === turn && !["interrupted", "canceled"].includes(e.error) && ((note.textContent = L("It couldn't be read out")), tell("stopped"));
      S.speak(u);
    }
    function play() {
      const S = window.speechSynthesis;
      if (!S) return (note.textContent = D.unsupported("speech"));
      note.textContent = "";
      if (state === "paused") {
        state = "playing";
        tell("playing");
        // From the start of the sentence: pausing isn't kept on every phone.
        return say();
      }
      S.cancel();
      parts = split();
      at = 0;
      if (!parts.length) return (note.textContent = L("There's nothing to read"));
      tell("playing");
      say();
    }
    function halt(to) {
      turn++;
      window.speechSynthesis?.cancel();
      if (to === "stopped") at = 0, mark(null);
      tell(to);
    }
    function draw() {
      const focus = row.contains(document.activeElement);
      row.replaceChildren(...[
        btn(state === "stopped" ? "" : "primary", { "data-read": "", click: () => (state === "playing" ? halt("paused") : play()) }, icon(state === "playing" ? "pause" : state === "paused" ? "play" : "speaker", 16), state === "playing" ? L("Pause") : state === "paused" ? L("Go on") : K.value(api, cfg.label) ?? L("Read aloud")),
        state === "stopped" ? null : btn("ghost", { "data-halt": "", click: () => halt("stopped") }, icon("stop", 14), L("Stop")),
        cfg.controls ? make("select", { "data-w-field": "", "aria-label": L("Speed"), change: (e) => { rate = Number(e.target.value); if (state === "playing") turn++, window.speechSynthesis.cancel(), say(); } }, [0.75, 1, 1.25, 1.5, 2].map((r) => make("option", { value: r, selected: r === rate }, `${r}×`))) : null].filter(Boolean));
      if (focus) row.querySelector("button").focus();
    }
    D.gone(el, () => (window.speechSynthesis?.cancel(), mark(null)));
    draw();
    return {};
  };

  // ---------------------------------------------------------------- wake lock
  B["ui-wake-lock"] = (el, api) => {
    const cfg = api.config;
    let lock = null, want = false, asking = false;
    const input = make("input", { type: "checkbox", role: "switch", change: () => api.set(cfg.bind, input.checked) }), name = make("span");
    if (cfg.bind) el.append(make("label", null, input, name));
    else el.hidden = true;
    const tell = () => cfg.bindHeld && !!api.state[cfg.bindHeld] !== !!lock && api.set(cfg.bindHeld, !!lock);
    async function sync() {
      if (!navigator.wakeLock || asking) return;
      if (want && !lock && document.visibilityState === "visible") {
        asking = true;
        try {
          lock = await navigator.wakeLock.request("screen");
          lock.addEventListener("release", () => ((lock = null), tell()));
        } catch { /* low battery, say */ }
        asking = false;
        if (!want) lock?.release();
      } else if (!want && lock) await lock.release();
      tell();
    }
    function update() {
      const now = cfg.bind ? !!api.state[cfg.bind] : !!K.value(api, cfg.active);
      input.checked = now;
      const can = "wakeLock" in navigator;
      input.disabled = !can;
      name.textContent = K.value(api, cfg.label) ?? L("Keep the screen on");
      el.title = can ? "" : D.unsupported("wake");
      if (now !== want) (want = now), sync();
    }
    document.addEventListener("visibilitychange", () => el.isConnected && sync());
    D.gone(el, () => lock?.release());
    update();
    return { update };
  };

  // ---------------------------------------------------------------- swipe
  B["ui-swipe-actions"] = (el, api) => {
    const cfg = api.config, inner = el.querySelector(":scope > [data-sw-in]");
    const sides = { start: make("div", { "data-sw-acts": "start" }), end: make("div", { "data-sw-acts": "end" }) };
    let x = 0, drag = null, was = null, more = null;
    const acts = (s) => list(K.value(api, cfg[s])).filter(isObj);
    const off = () => !!K.value(api, cfg.disabled);
    el.prepend(sides.start, sides.end);
    // Swiped towards the end shows what's under its start, whichever way that is.
    const sign = () => (K.rtl(el) ? -1 : 1), widths = {};
    const wide = (s) => widths[s] || (sides[s].style.minWidth = "", (widths[s] = sides[s].offsetWidth));
    function to(v) {
      x = v;
      // Its buttons fill what's been uncovered.
      for (const s of ["start", "end"]) {
        const w = wide(s);
        sides[s].style.minWidth = (s === "start" ? v : -v) > w ? `${Math.abs(v)}px` : "";
      }
      inner.style.transform = v ? `translateX(${v * sign()}px)` : "";
      sides.start.toggleAttribute("data-on", v > 0);
      sides.end.toggleAttribute("data-on", v < 0);
      for (const s of ["start", "end"]) sides[s].querySelectorAll("button").forEach((b) => (b.tabIndex = (s === "start" ? v > 0 : v < 0) ? 0 : -1));
    }
    const shut = () => x && to(0);
    function run(a) {
      to(0);
      api.emit(String(a.value), { value: a.value });
      api.emit("swipe", { value: a.value });
    }
    function draw() {
      const now = JSON.stringify([acts("start"), acts("end"), K.value(api, cfg.label) ?? null]);
      if (now === was) return;
      was = now;
      for (const s of ["start", "end"]) sides[s].replaceChildren(...acts(s).map((a) => K.button({ "data-act": a.value, "data-tone": a.tone, tabindex: -1, click: () => run(a) }, blank(a.icon) ? null : make("span", { "aria-hidden": "true" }, a.icon), a.label ?? a.value)));
      widths.start = widths.end = 0;
      more?.remove();
      if (cfg.menu) {
        const label = K.value(api, cfg.label);
        more = K.button({ "data-sw-more": "", "aria-label": label ? L("Actions for {name}", { name: label }) : L("Actions"), click: () => {
          if (x) return shut();
          const s = acts("end").length ? "end" : "start";
          to(s === "end" ? -wide("end") : wide("start"));
          sides[s].querySelector("button")?.focus();
        } }, icon("more", 16));
        el.append(more);
      }
      to(0);
    }
    inner.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 || off() || e.target.closest("input, textarea, select, [contenteditable]")) return;
      drag = { id: e.pointerId, px: e.clientX, py: e.clientY, x0: x, on: false, t: e.timeStamp };
    });
    inner.addEventListener("pointermove", (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      const dx = (e.clientX - drag.px) * sign(), dy = e.clientY - drag.py;
      if (!drag.on) {
        if (Math.abs(dy) > 10 && Math.abs(dy) > Math.abs(dx)) return (drag = null);
        if (Math.abs(dx) < 8) return;
        drag.on = true;
        inner.setAttribute("data-drag", "");
        K.capture(inner, e);
      }
      let v = drag.x0 + dx;
      const s = v > 0 ? "start" : "end", n = acts(s).length, w = wide(s), far = Math.abs(v);
      // Past its buttons it's harder, or nothing's there.
      v = !n ? v * 0.15 : far > w && !cfg.full ? Math.sign(v) * (w + (far - w) * 0.2) : v;
      to(v);
    });
    const up = (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      const d = drag;
      drag = null;
      inner.removeAttribute("data-drag");
      if (!d.on) return;
      el.__swiped = true;
      setTimeout(() => (el.__swiped = false));
      const s = x > 0 ? "start" : "end", a = acts(s), w = wide(s), far = Math.abs(x);
      if (!a.length || e.type === "pointercancel") return to(0);
      if (cfg.full && far > Math.max(w + 30, el.clientWidth * 0.6)) return run(s === "start" ? a[0] : a.at(-1));
      to(far > w * 0.45 && (x > 0) === (x - d.x0 > 0 || Math.abs(d.x0) > 0) ? Math.sign(x) * w : 0);
    };
    inner.addEventListener("pointerup", up);
    inner.addEventListener("pointercancel", up);
    inner.addEventListener("dragstart", (e) => e.preventDefault());
    // The click that ends a swipe, or closes an open row, isn't the row's.
    inner.addEventListener("click", (e) => {
      if (el.__swiped || x) e.stopPropagation(), e.preventDefault(), el.__swiped || shut();
    }, true);
    el.addEventListener("keydown", (e) => e.key === "Escape" && x && (shut(), more?.focus()));
    document.addEventListener("pointerdown", (e) => el.isConnected && x && !el.contains(e.target) && shut());
    draw();
    return { update: draw };
  };
})();
