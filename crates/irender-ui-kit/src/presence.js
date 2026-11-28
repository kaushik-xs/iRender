// irender-ui-kit: the `ui-presence` behaviour. With `announce` it compares who's listed
// with who was, and says who joined and who left, politely, for screen readers; with
// `bindFollow`, a person's Go to follows them (their `id` to state, and `follow`), and
// pressed again stops. The avatars and buttons are the runtime's.
(window.__irenderBehaviors ||= {})["ui-presence"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const live = el.querySelector("[data-pr-live]");
  const who = () => new Map([...el.querySelectorAll("[data-pr-list] [data-pr-person]")].map((li) => [li.getAttribute("data-pr-person"), li.querySelector("[data-pr-name] > span")?.textContent ?? ""]));
  let was = who();
  el.addEventListener("click", (e) => {
    const b = e.target.closest?.("[data-pr-follow]");
    if (!b || !cfg.bindFollow) return;
    const key = b.getAttribute("data-pr-follow");
    const people = K.value(api, cfg.people);
    const person = (Array.isArray(people) ? people : []).find((p) => String(p?.id ?? p?.name) === key) ?? null;
    const on = b.getAttribute("aria-pressed") === "true";
    api.set(cfg.bindFollow, on ? null : person?.id ?? key);
    if (!on) api.emit("follow", { person });
    // Labels: "Following {name}", "Stopped following {name}"
    if (live) live.textContent = on ? L("Stopped following {name}", { name: person?.name ?? key }) : L("Following {name}", { name: person?.name ?? key });
  });
  return {
    update() {
      if (!live || !el.hasAttribute("data-pr-announce")) return;
      // After the runtime has drawn the list again.
      queueMicrotask(() => {
        const now = who();
        const came = [...now].filter(([k]) => !was.has(k)).map(([, n]) => n);
        const went = [...was].filter(([k]) => !now.has(k)).map(([, n]) => n);
        was = now;
        const said = [...came.map((n) => L("{name} joined", { name: n })), ...went.map((n) => L("{name} left", { name: n }))];
        if (said.length) live.textContent = said.join(". ");
      });
    },
  };
};
