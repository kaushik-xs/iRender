// irender-ui-kit: the buttons under a chat message (`ui-message`'s `actions`). Copy is done
// here; the rest go up as `messageaction`, which the page's `ui-chat-input` answers.
(window.__irenderBehaviors ||= {})["ui-message-actions"] = (el, api) => {
  const L = (t, v) => window.__irender.label(t, v);
  el.addEventListener("click", async (e) => {
    const b = e.target.closest("[data-act]");
    if (!b || !el.contains(b)) return;
    const action = b.getAttribute("data-act"), id = el.getAttribute("data-message");
    api.emit("messageaction", { action, id });
    if (action !== "copy") return;
    const text = el.parentElement.querySelector(":scope > [data-bubble]")?.innerText ?? "";
    let ok = true;
    try {
      await navigator.clipboard.writeText(text.trim());
    } catch {
      ok = false;
    }
    b.title = ok ? L("Copied") : L("Couldn't copy");
    b.setAttribute("aria-label", b.title);
    b.toggleAttribute("data-copied", ok);
    setTimeout(() => {
      b.title = L("Copy");
      b.setAttribute("aria-label", b.title);
      b.removeAttribute("data-copied");
    }, 1600);
  });
  return {};
};
