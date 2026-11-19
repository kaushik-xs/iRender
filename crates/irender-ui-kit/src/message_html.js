// irender-ui-kit: the `ui-message-html` behaviour. Shows a message's UI and starts it
// as a root of its own. Its `html` was rendered on a server; a `ui` without it is
// rendered here, by the first of these that works:
//   1. `browser`: irender-wasm's module, loaded the first time it's needed.
//   2. `url`: the host's render endpoint, which gets `{ ui }` and answers with markup
//      (or JSON `{ html }`).
// When neither can, the message says so. The markup's scripts are the kits'
// behaviours, re-added so that they run.
(() => {
  const modules = (window.__irenderWasm ||= {});
  // Loaded once per page. A module that fails to load stays failed, so later messages
  // go straight to the next way.
  const load = (src) =>
    (modules[src] ||= (async () => {
      const res = await fetch(src);
      if (!res.ok) throw new Error(`${src}: ${res.status}`);
      // Streaming needs the server to send `application/wasm`; otherwise compile the bytes.
      const { instance } = await WebAssembly.instantiateStreaming(res.clone(), {}).catch(async () =>
        WebAssembly.instantiate(await res.arrayBuffer(), {}),
      );
      return instance.exports;
    })());

  const inBrowser = async (src, ui) => {
    const x = await load(src);
    const input = new TextEncoder().encode(ui);
    const ptr = x.irender_alloc(input.length);
    new Uint8Array(x.memory.buffer, ptr, input.length).set(input);
    let status;
    try {
      status = x.irender_render(ptr, input.length);
    } catch (e) {
      // A trap leaves the module unusable: load it afresh next time.
      delete modules[src];
      throw e;
    }
    const out = new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.irender_out_ptr(), x.irender_out_len()));
    if (status !== 0) throw new Error(out);
    return out;
  };

  const onServer = async (url, ui) => {
    const res = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json", accept: "text/html, application/json" },
      body: `{"ui":${ui}}`,
    });
    if (!res.ok) throw new Error(`${url}: ${res.status} ${await res.text()}`);
    return (res.headers.get("content-type") || "").includes("json") ? (await res.json()).html : res.text();
  };

  const render = async (ui, { browser, url }) => {
    if (browser) {
      try {
        return await inBrowser(browser, ui);
      } catch (e) {
        if (!url) throw e;
        console.warn("irender: rendering a message's UI on the server instead:", e.message);
      }
    }
    if (url) return onServer(url, ui);
    throw new Error("no `render` to render it with");
  };

  (window.__irenderBehaviors ||= {})["ui-message-html"] = (el, api) => {
    // Interface text in the document's words (its `labels`), by the English.
    const L = (text, vars) => window.__irender.label(text, vars);
    let shown = null;
    let turn = 0;
    const insert = (html) => {
      el.innerHTML = html;
      for (const old of el.querySelectorAll("script:not([type])")) {
        const script = document.createElement("script");
        script.textContent = old.textContent;
        old.replaceWith(script);
      }
      window.__irender.mount(el);
    };
    const show = () => {
      const html = el.getAttribute("data-html") || "";
      const ui = el.getAttribute("data-ui-json") || "";
      const key = html || ui;
      if (key === shown) return;
      shown = key;
      const mine = ++turn;
      if (html || !ui) return insert(html);
      el.innerHTML = "";
      render(ui, api.config).then(
        (h) => mine === turn && insert(h),
        (e) => {
          if (mine !== turn) return;
          console.warn("irender: a message's UI can't be shown:", e.message);
          el.innerHTML = "";
          const note = document.createElement("p");
          note.setAttribute("data-ui-note", "");
          note.textContent = L("This reply has UI that can't be shown here.");
          el.append(note);
        },
      );
    };
    show();
    return { update: show };
  };
})();
