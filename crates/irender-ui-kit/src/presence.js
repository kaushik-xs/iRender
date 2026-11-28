// irender-ui-kit: the `ui-presence` behaviour, with `announce`. The avatars are the
// runtime's; this only compares who's listed with who was, and says who joined and who
// left, politely, for screen readers.
(window.__irenderBehaviors ||= {})["ui-presence"] = (el, api) => {
  const L = (t, v) => window.__irender.label(t, v);
  const live = el.querySelector("[data-pr-live]");
  const who = () => new Map([...el.querySelectorAll("[data-pr-list] [data-pr-person]")].map((li) => [li.getAttribute("data-pr-person"), li.querySelector("[data-pr-name] > span")?.textContent ?? ""]));
  let was = who();
  return {
    update() {
      // After the runtime has drawn the list again.
      queueMicrotask(() => {
        const now = who();
        const came = [...now].filter(([k]) => !was.has(k)).map(([, n]) => n);
        const went = [...was].filter(([k]) => !now.has(k)).map(([, n]) => n);
        was = now;
        const said = [...came.map((n) => L("{name} joined", { name: n })), ...went.map((n) => L("{name} left", { name: n }))];
        if (said.length && live) live.textContent = said.join(". ");
      });
    },
  };
};
