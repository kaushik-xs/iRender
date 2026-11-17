// irender-ui-kit: the `ui-conversation` behaviour. The log keeps to its latest message
// as messages arrive or stream in, unless the reader has scrolled up; then the "latest"
// button shows, and brings them back down. A message the user sends always does.
(window.__irenderBehaviors ||= {})["ui-conversation"] = (el) => {
  const log = el.querySelector("[data-log]");
  const latest = el.querySelector("[data-latest]");
  let stick = true;
  const near = () => log.scrollHeight - log.scrollTop - log.clientHeight < 40;
  const bottom = () => {
    log.scrollTop = log.scrollHeight;
    stick = true;
    latest.hidden = true;
  };
  log.addEventListener("scroll", () => {
    stick = near();
    latest.hidden = stick;
  });
  latest.addEventListener("click", () => {
    bottom();
    log.focus({ preventScroll: true });
  });
  const sent = (records) => records.some((r) => [...r.addedNodes].some((n) => n.nodeType === 1 && n.matches("[data-role=user]")));
  new MutationObserver((records) => {
    if (stick || sent(records)) bottom();
    else if (!near()) latest.hidden = false;
  }).observe(log, { childList: true, subtree: true, characterData: true });
  new ResizeObserver(() => stick && bottom()).observe(log);
  // Images and videos grow once they load.
  for (const type of ["load", "loadedmetadata"]) log.addEventListener(type, () => stick && bottom(), true);
  bottom();
};
