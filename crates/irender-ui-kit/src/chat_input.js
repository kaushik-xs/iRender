// irender-ui-kit: the `ui-chat-input` behaviour. Sending appends the user's message to
// the `messages` key, emits `send` (so the document's `on.send` actions run), and with
// an `endpoint` POSTs `{ messages }` and adds the reply: a JSON message, plain text, or
// server-sent events streamed into it chunk by chunk, any of which may carry the reply's
// UI, as `ui` JSON or server-rendered `html`. While a reply streams, the send button stops it,
// and the message has `streaming: true`, so its Markdown shows formatted as it arrives.
//
// With `threads` and `thread`, the messages live in the threads (`{ id, title, messages }`)
// and `messages` shows the current one's. Sending with no current thread starts one,
// titled after the message; a reply keeps streaming into its own thread if you switch.
(window.__irenderBehaviors ||= {})["ui-chat-input"] = (el, api) => {
  const { messages, endpoint, bind, waiting, threads, thread } = api.config;
  const input = el.querySelector("[data-input]");
  const button = el.querySelector("[data-send-button]");
  const suggestions = el.querySelector("[data-suggestions]");
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  let req = null; // the reply in flight: { tid, controller, started }

  const id = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 7);
  const array = (v) => (Array.isArray(v) ? v : []);
  const active = () => (threads ? api.state[thread] ?? null : null);
  const all = () => array(api.state[threads]);
  // A thread's messages, and replacing them; without threads, the `messages` key's.
  const read = (tid) => (threads ? array(all().find((t) => t.id === tid)?.messages) : array(api.state[messages]));
  const write = (tid, next) => {
    if (threads) api.set(threads, all().map((t) => (t.id === tid ? { ...t, messages: next } : t)));
    if (!threads || tid === active()) api.set(messages, next);
  };

  const sync = () => {
    const here = !!req && req.tid === active();
    button.toggleAttribute("data-streaming", here);
    button.setAttribute("aria-label", here ? L("Stop") : L("Send"));
    button.disabled = !here && (!!req || input.disabled || input.value.trim() === "");
    if (suggestions) suggestions.hidden = read(active()).length > 0;
    // Waiting for the current thread's reply to start. Set after this update, since a
    // set from inside one would nest another.
    const w = here && !req.started;
    if (waiting && api.state[waiting] !== w) queueMicrotask(() => api.state[waiting] !== w && api.set(waiting, w));
  };

  // The thread `messages` shows; switching threads shows the new one's.
  let shown = active();
  const update = () => {
    if (threads && active() !== shown) {
      shown = active();
      queueMicrotask(() => api.set(messages, read(shown)));
    }
    sync();
  };

  // The reply goes into the message with this id, added with its first change.
  // `change(message)` returns the fields to update.
  const replyInto = (tid, rid) => {
    let added = false;
    const reply = (change) => {
      req.started = true;
      if (added) return write(tid, read(tid).map((m) => (m.id === rid ? { ...m, ...change(m) } : m)));
      added = true;
      write(tid, [...read(tid), { id: rid, role: "assistant", content: "", ...change({ content: "" }) }]);
      sync();
    };
    // A streamed reply is `streaming` until it ends, so its Markdown shows finished.
    reply.done = () => {
      if (added && read(tid).some((m) => m.id === rid && m.streaming))
        write(tid, read(tid).map((m) => (m.id === rid ? (({ streaming, ...rest }) => rest)(m) : m)));
    };
    return reply;
  };
  // A chunk: text, or JSON with `content`, `delta`, `text` or OpenAI's
  // `choices[0].delta.content`, and maybe the reply's UI: `ui` (JSON, for the page to
  // render) or `html` (rendered on the server).
  const chunk = (data, append, reply) => {
    let v;
    try {
      v = JSON.parse(data);
    } catch {
      return append(data);
    }
    if (typeof v === "string") return append(v);
    append(v?.content ?? v?.delta ?? v?.text ?? v?.choices?.[0]?.delta?.content ?? "");
    if (typeof v?.html === "string") reply(() => ({ html: v.html }));
    if (v?.ui && typeof v.ui === "object") reply(() => ({ ui: v.ui }));
  };

  async function ask(tid) {
    req = { tid, controller: new AbortController(), started: false };
    sync();
    const reply = replyInto(tid, id());
    const append = (text) => text && reply((m) => ({ content: (m.content ?? "") + text, streaming: true }));
    try {
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "content-type": "application/json", accept: "text/event-stream, application/json, text/plain" },
        body: JSON.stringify({ messages: read(tid) }),
        signal: req.controller.signal,
      });
      if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
      const type = res.headers.get("content-type") || "";
      if (type.includes("json")) {
        const data = await res.json();
        const message = typeof data === "string" ? { content: data } : data?.message ?? data;
        reply(() => ({ ...message, role: message.role ?? "assistant" }));
      } else {
        const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
        const events = type.includes("event-stream");
        let buffer = "";
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          if (!events) {
            append(value);
            continue;
          }
          buffer += value;
          const lines = buffer.split(/\r?\n/);
          buffer = lines.pop();
          for (const line of lines) {
            if (!line.startsWith("data:")) continue;
            const data = line.slice(5).trimStart();
            if (data === "[DONE]") return;
            chunk(data, append, reply);
          }
        }
      }
    } catch (error) {
      if (error.name !== "AbortError") {
        write(tid, [...read(tid), { id: id(), role: "system", content: L("The reply failed: {error}", { error: error.message }), error: true }]);
        api.emit("irender:error", { error: String(error) });
      }
    } finally {
      reply.done();
      req = null;
      sync();
    }
  }

  const send = (text) => {
    text = text.trim();
    if (!text || req || input.disabled) return;
    const message = { id: id(), role: "user", content: text };
    let tid = active();
    if (threads) {
      // A new thread for the first message, titled after it; otherwise the current one
      // moves to the top.
      const current = all().find((t) => t.id === tid);
      if (!current) {
        tid = id();
        const title = text.length > 40 ? `${text.slice(0, 40).trimEnd()}…` : text;
        api.set(threads, [{ id: tid, title, messages: [] }, ...all()]);
        shown = tid;
        api.set(thread, tid);
      } else {
        api.set(threads, [current, ...all().filter((t) => t !== current)]);
      }
    }
    write(tid, [...read(tid), message]);
    // Before the text is cleared, so `on.send` actions can still read the bound key.
    api.emit("send", { text, message, thread: tid });
    input.value = "";
    if (bind) api.set(bind, "");
    sync();
    if (endpoint) ask(tid);
  };

  el.addEventListener("submit", (e) => {
    e.preventDefault();
    if (req && req.tid === active()) req.controller.abort();
    else send(input.value);
  });
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send(input.value);
    }
  });
  input.addEventListener("input", sync);
  suggestions?.addEventListener("click", (e) => {
    const b = e.target.closest("[data-suggestion]");
    if (b) send(b.textContent);
  });
  if (threads) queueMicrotask(() => api.set(messages, read(shown)));
  sync();
  return { update };
};
