// irender-ui-kit: what the components that use the device share (`window.__irenderDevice`):
// what it can do, what's allowed, and words for what went wrong.
(() => {
  if (window.__irenderDevice) return;
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const ua = navigator.userAgent;
  const ios = /iPad|iPhone|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1);
  const android = /Android/.test(ua), firefox = /Firefox|FxiOS/.test(ua), edge = /Edg/.test(ua), chrome = !edge && /Chrome|CriOS/.test(ua);
  const safari = !chrome && !edge && !firefox && /Safari/.test(ua);
  const standalone = () => matchMedia("(display-mode: standalone), (display-mode: fullscreen), (display-mode: minimal-ui)").matches || navigator.standalone === true;
  const NAMES = {
    camera: () => L("Camera"), microphone: () => L("Microphone"), geolocation: () => L("Location"), notifications: () => L("Notifications"), motion: () => L("Motion sensors"),
    nfc: () => L("NFC"), bluetooth: () => L("Bluetooth"), serial: () => L("Serial port"), usb: () => L("USB"), clipboard: () => L("Clipboard"), storage: () => L("Kept storage"),
    screen: () => L("Screen"), speech: () => L("Speech"), contacts: () => L("Contacts"), files: () => L("Files"), passkey: () => L("Passkey"), wake: () => L("Screen lock"),
  };
  const name = (what) => NAMES[what]?.() ?? what;
  // Why something didn't work, in words; null when someone changed their mind.
  function explain(e, what) {
    const n = name(what), code = e?.name;
    if (!window.isSecureContext) return L("{what} needs a secure page (https)", { what: n });
    if (code === "AbortError") return null;
    if (code === "NotAllowedError" || code === "SecurityError" || code === "PermissionDeniedError" || (what === "geolocation" && e?.code === 1)) return L("{what} isn't allowed for this site", { what: n });
    if (code === "NotFoundError" || code === "DevicesNotFoundError" || code === "OverconstrainedError") return L("{what}: none was found", { what: n });
    if (code === "NotReadableError" || code === "TrackStartError") return L("{what}: another app is using it", { what: n });
    if (code === "NotSupportedError") return L("{what} isn't something this browser or device can do", { what: n });
    if (what === "geolocation" && e?.code === 2) return L("The location can't be found here");
    if (what === "geolocation" && e?.code === 3) return L("Finding the location took too long");
    return String(e?.message || e || L("Something went wrong"));
  }
  const blocked = (e) => e?.name === "NotAllowedError" || e?.name === "SecurityError" || (e?.code === 1 && "PERMISSION_DENIED" in e);
  const unsupported = (what) => L("{what} isn't something this browser or device can do", { what: name(what) });
  // How to allow what's been blocked, for this browser.
  function steps(what) {
    const n = name(what);
    if (ios) return standalone() && what !== "notifications"
      ? [L("Open the Settings app"), L("Find this app in the list, or Safari under Apps"), L("Set {what} to Allow, then open the app again", { what: n })]
      : [L("Open the Settings app, then Apps, then Safari"), L("Set {what} to Allow or Ask", { what: n }), L("Come back and reload the page")];
    if (android) return [L("Tap the icon left of the address, or hold the app's icon and tap App info"), L("Open Permissions"), L("Set {what} to Allow, then reload", { what: n })];
    if (safari) return [L("In the menu bar, open Safari, then Settings for this website"), L("Set {what} to Allow", { what: n }), L("Reload the page")];
    if (firefox) return [L("Click the icon left of the address"), L("Clear the blocked permission for {what}", { what: n }), L("Reload the page, and allow it when asked")];
    return [L("Click the icon left of the address"), L("Set {what} to Allow", { what: n }), L("Reload the page")];
  }
  // `granted`, `denied`, `prompt`, or null when the browser doesn't say.
  async function permission(what) {
    if (what === "notifications" && "Notification" in window) return Notification.permission === "default" ? "prompt" : Notification.permission;
    const map = { camera: "camera", microphone: "microphone", geolocation: "geolocation", nfc: "nfc", clipboard: "clipboard-read", storage: "persistent-storage", notifications: "notifications" };
    if (!map[what]) return null;
    try {
      return (await navigator.permissions.query({ name: map[what] })).state;
    } catch {
      return null;
    }
  }
  async function watch(what, f) {
    const map = { geolocation: "geolocation", camera: "camera", microphone: "microphone", notifications: "notifications", nfc: "nfc", storage: "persistent-storage" };
    try {
      (await navigator.permissions.query({ name: map[what] })).addEventListener("change", () => f());
    } catch { /* it's looked at again when the page is */ }
  }
  const dataUrl = (blob) => new Promise((ok, no) => {
    const r = new FileReader();
    r.onload = () => ok(r.result);
    r.onerror = () => no(r.error);
    r.readAsDataURL(blob);
  });
  function toBlob(url) {
    const m = /^data:([^;,]*)([^,]*),(.*)$/s.exec(url);
    if (!m) return null;
    const bytes = m[2].includes("base64") ? Uint8Array.from(atob(m[3]), (c) => c.charCodeAt(0)) : new TextEncoder().encode(decodeURIComponent(m[3]));
    return new Blob([bytes], { type: m[1] || "application/octet-stream" });
  }
  const blobOf = (canvas, type, quality) => new Promise((ok) => canvas.toBlob(ok, type, quality));
  const size = (n) => (n < 1024 ? `${n} B` : n < 1048576 ? `${Math.round(n / 1024)} KB` : `${(n / 1048576).toFixed(1)} MB`);
  const time = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
  const stamp = (d = new Date()) => `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}-${String(d.getHours()).padStart(2, "0")}${String(d.getMinutes()).padStart(2, "0")}${String(d.getSeconds()).padStart(2, "0")}`;
  const btn = (kind, attrs, ...kids) => K.button({ "data-w-btn": kind ?? "", ...attrs }, ...kids);
  // A dialog with the steps to allow something.
  function help(el, what) {
    const close = btn("primary", { click: () => d.close() }, L("Close"));
    const d = K.make("dialog", { "aria-label": L("Allow {what}", { what: name(what) }) }, K.make("h2", null, L("Allow {what}", { what: name(what) })), K.make("ol", null, steps(what).map((s) => K.make("li", null, s))), K.make("div", { "data-w-row": "" }, close));
    d.addEventListener("close", () => d.remove());
    el.append(d);
    d.showModal();
  }
  // Runs `f` when the element leaves the page: streams and watches end with it.
  const gone = (el, f) => {
    const t = setInterval(() => el.isConnected || (clearInterval(t), f()), 1500);
    addEventListener("pagehide", f);
  };
  // A script or module from its address, once, when it's first needed.
  const loading = new Map();
  const load = (url, isModule) => {
    if (!loading.has(url)) loading.set(url, isModule ? import(url) : new Promise((ok, no) => document.head.append(K.make("script", { src: url, crossorigin: /^https?:/.test(url) ? "anonymous" : null, load: ok, error: () => (loading.delete(url), no(new Error(L("Couldn't load {url}", { url })))) }))));
    return loading.get(url);
  };
  const ok = (u) => typeof u === "string" && u !== "" && !/^\s*(javascript|vbscript|data):/i.test(u);
  // What's redrawn goes in here, apart from dialogs and what's read out.
  const body = (el) => {
    const b = K.make("div", { style: "display:contents" });
    el.append(b);
    return b;
  };
  window.__irenderDevice = { body, blocked, ios, android, firefox, safari, chrome, edge, standalone, name, explain, unsupported, steps, permission, watch, dataUrl, toBlob, blobOf, size, time, stamp, btn, help, gone, load, safeUrl: ok };
})();
