// irender-ui-kit: the `ui-password-checklist` behaviour. Checks each rule against the
// password as it changes (from `value`, which reads state), ticks the ones met, sets
// the strength bar, and writes whether all are met to `bindValid`.
(window.__irenderBehaviors ||= {})["ui-password-checklist"] = (el, api) => {
  const cfg = api.config;
  const items = [...el.querySelectorAll("li")];
  const tests = cfg.rules.map((r) => {
    if (r.min != null) return (s) => [...s].length >= r.min;
    try {
      const re = new RegExp(r.pattern, "u");
      return (s) => re.test(s);
    } catch {
      return () => false;
    }
  });
  const read = () => {
    const v = cfg.value;
    const s = v !== null && typeof v === "object" && "$irender" in v ? window.__irender.evaluate(v.$irender, api.state) : v;
    return s == null ? "" : String(s);
  };
  const L = (text, vars) => window.__irender.label(text, vars);
  const words = [L("Enter a password"), L("Weak"), L("Fair"), L("Good"), L("Strong")];
  let last = null;
  function check() {
    const s = read();
    if (s === last) return;
    last = s;
    const met = tests.map((t) => !!s && t(s));
    items.forEach((li, i) => {
      li.toggleAttribute("data-met", met[i]);
      li.querySelector("[data-state]").textContent = met[i] ? ", met" : ", not met";
    });
    const all = met.every(Boolean);
    // Strength: Weak under half the rules, Fair from half, Good with all of them, and
    // Strong with all and 12 characters or more.
    const share = met.filter(Boolean).length / met.length;
    const strength = !s ? 0 : all ? ([...s].length >= 12 ? 4 : 3) : share < 0.5 ? 1 : 2;
    el.setAttribute("data-strength", strength);
    const text = el.querySelector("[data-strength-text]");
    if (text) text.textContent = s ? `Strength: ${words[strength]}` : words[0];
    if (cfg.bindValid && api.state[cfg.bindValid] !== all) api.set(cfg.bindValid, all);
  }
  check();
  return { update: check };
};
