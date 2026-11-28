// irender-ui-kit: the `ui-approval` behaviour. From the stages in state it works out where
// the sign-off stands: a stage is approved when its rule is met (all its approvers, any
// one, or a number), rejected when it no longer can be, waiting for changes when one asks
// for them; the first stage not approved is the current one, and the ones after it wait.
// It draws that, gives whoever's looking (`me`) the buttons on their turn, and writes each
// decision back with its comment and time. The one who sent it (`owner`) can send it again
// after changes or a rejection, which keeps the decisions as an earlier round and clears
// them. A stage can be `due` by a date; an approver can hand their turn to one of `people`,
// and take it back while it's undecided. A stage with `parallel: true` runs alongside the
// one before it: the group is current together, and done when all of it is.
// Everything it writes is text.
(window.__irenderBehaviors ||= {})["ui-approval"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const summary = el.querySelector("[data-ap-summary]"), view = el.querySelector("[data-ap-view]"), live = el.querySelector("[data-ap-live]");
  const say = (msg) => live && (live.textContent = msg);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const h = (tag, attrs = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else e.setAttribute(k, v === true ? "" : v);
    }
    e.append(...kids.filter((k) => k !== null && k !== undefined && k !== false));
    return e;
  };
  const stages = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind].filter(isObj) : []);
  const same = (a, b) => a !== null && a !== undefined && b !== null && b !== undefined && String(a) === String(b);
  const loc = cfg.locale || document.documentElement.lang || undefined;
  const when = (v) => {
    const t = v ? Date.parse(v) : NaN;
    return Number.isFinite(t) ? new Intl.DateTimeFormat(loc, { dateStyle: "medium", timeStyle: "short" }).format(t) : "";
  };

  // ---- where a stage stands, by its rule
  function need(s) {
    const n = (s.approvers ?? []).length;
    return s.rule === "any" ? 1 : typeof s.rule === "number" ? Math.min(n, s.rule) : n;
  }
  function stateOf(s) {
    const people = s.approvers ?? [];
    const yes = people.filter((p) => p.decision === "approved").length;
    const no = people.filter((p) => p.decision === "rejected").length;
    if (people.some((p) => p.decision === "changes")) return "changes";
    if (yes >= need(s)) return "approved";
    // Rejected once too many said no for the rule still to be met.
    if (people.length - no < need(s)) return "rejected";
    return "pending";
  }
  // Every stage's state, and the sign-off's: approved, rejected, changes or pending. Stages
  // side by side (`parallel`) are a group: the first group not all approved is current,
  // every one of its stages that isn't approved yet (`open`, the ones being decided), and
  // a rejection or a request for changes in it is the sign-off's.
  function standing() {
    const list = stages();
    const groups = [];
    list.forEach((s, i) => (s.parallel && groups.length ? groups.at(-1).push(i) : groups.push([i])));
    let current = -1, overall = "approved";
    const states = new Array(list.length).fill("waiting"), open = [];
    for (const g of groups) {
      const st = g.map((i) => stateOf(list[i]));
      if (st.every((x) => x === "approved")) {
        g.forEach((i) => (states[i] = "approved"));
        continue;
      }
      g.forEach((i, k) => (states[i] = st[k] === "pending" ? "current" : st[k]));
      open.push(...g.filter((i, k) => st[k] === "pending"));
      current = g.find((i, k) => st[k] !== "approved");
      overall = st.includes("rejected") ? "rejected" : st.includes("changes") ? "changes" : "pending";
      break;
    }
    if (!list.length) overall = "pending";
    return { list, states, current, overall, open, group: groups.find((g) => g.includes(current)) ?? [] };
  }
  // Labels: "All of {n} must approve", "Any one of {n} can approve", "{k} of {n} must approve"
  function ruleText(s) {
    const n = (s.approvers ?? []).length, k = need(s);
    if (n === 1) return "";
    return k === n ? L("All of {n} must approve", { n }) : k === 1 ? L("Any one of {n} can approve", { n }) : L("{k} of {n} must approve", { k, n });
  }
  const DECIDED = { approved: "Approved", changes: "Changes requested", rejected: "Rejected" };
  const initials = (name) => String(name).split(/\s+/).slice(0, 2).map((w) => w[0] ?? "").join("").toUpperCase();
  const mine = (s) => (s.approvers ?? []).find((p) => same(p.id, K.value(api, cfg.me)));
  // A date alone (`2026-12-01`) is that local day, not midnight in London.
  const parseDay = (v) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(v ?? ""));
    return m ? new Date(+m[1], +m[2] - 1, +m[3]).getTime() : v ? Date.parse(v) : NaN;
  };
  const day = (v) => {
    const t = parseDay(v);
    return Number.isFinite(t) ? new Intl.DateTimeFormat(loc, { dateStyle: "medium" }).format(t) : "";
  };
  // When a stage is due, said from today: overdue once the day has passed.
  function dueText(s, live) {
    const t = parseDay(s.due);
    if (!Number.isFinite(t)) return null;
    const days = Math.floor((t - new Date().setHours(0, 0, 0, 0)) / 864e5);
    // Labels: "Due {date}", "Due today", "Overdue by {n} day"
    if (!live) return { text: L("Due {date}", { date: day(s.due) }), late: false };
    if (days < 0) return { text: L(["Overdue by {n} day", "Overdue by {n} days"], { n: -days }), late: true };
    return { text: days === 0 ? L("Due today") : L("Due {date}", { date: day(s.due) }), late: false };
  }

  // ---- drawing
  let last = null, draft = "", wrong = null;
  function draw() {
    const { list, states, current, overall, open, group } = standing();
    const who = (d) => list.flatMap((s) => s.approvers ?? []).find((p) => p.decision === d);
    // What it comes to, in a sentence.
    let head, sub = "";
    if (overall === "approved") (head = L("Approved")), (sub = L(["{n} stage signed off", "All {n} stages signed off"], { n: list.length }));
    else if (overall === "rejected") {
      const p = who("rejected");
      head = L("Rejected by {name}", { name: p?.name ?? "" });
      sub = p?.comment ?? "";
    } else if (overall === "changes") {
      const p = who("changes");
      head = L("{name} asked for changes", { name: p?.name ?? "" });
      sub = p?.comment ?? "";
    } else if (current >= 0 && group.length > 1) {
      // Side by side: the ones still waiting, by name.
      const names = open.map((i) => list[i].title ?? L("Stage {n}", { n: i + 1 }));
      head = L("Waiting for {stage}", { stage: new Intl.ListFormat(loc, { type: "conjunction" }).format(names) });
      // Labels: "{done} of {n} side by side signed off · stages {from}–{to} of {total}"
      sub = L("{done} of {n} side by side signed off · stages {from}–{to} of {total}", { done: group.length - open.length, n: group.length, from: group[0] + 1, to: group.at(-1) + 1, total: list.length });
    } else if (current >= 0) {
      const s = list[current], yes = (s.approvers ?? []).filter((p) => p.decision === "approved").length;
      head = L("Waiting for {stage}", { stage: s.title ?? L("Stage {n}", { n: current + 1 }) });
      sub = L("{yes} of {need} approvals · stage {n} of {total}", { yes, need: need(s), n: current + 1, total: list.length });
    } else head = L("Nothing to approve yet");
    summary.setAttribute("data-state", overall);
    summary.replaceChildren(h("div", {}, h("b", {}, head), sub ? h("small", {}, sub) : null));

    const ol = h("ol", { "data-ap-stages": true });
    list.forEach((s, i) => {
      const st = states[i];
      const mark = { approved: "✓", rejected: "✕", changes: "!" }[st] ?? String(i + 1);
      const people = h("ul", { "data-ap-people": true }, ...(s.approvers ?? []).map((p) => {
        const d = p.decision && DECIDED[p.decision] ? p.decision : "none";
        const pic = p.avatar ? h("img", { src: p.avatar, alt: "" }) : initials(p.name);
        return h("li", { "data-ap-person": p.id, "data-decision": d },
          h("span", { "data-ap-avatar": true, "aria-hidden": "true" }, pic),
          h("span", { "data-ap-name": true }, p.name, p.decision && p.at ? h("small", {}, when(p.at)) : null,
            // Labels: "for {name}"
            p.for ? h("span", { "data-ap-for": true }, L("for {name}", { name: p.for.name ?? p.for.id ?? "" })) : null),
          h("span", { "data-ap-decision": true }, d === "none" ? (st === "waiting" ? L("Not yet") : L("Waiting")) : L(DECIDED[d])),
          p.comment ? h("blockquote", {}, p.comment) : null);
      }));
      // Labels: "Approved", "Rejected", "Changes requested", "Waiting", "Up next"
      const said = { approved: L("Approved"), rejected: L("Rejected"), changes: L("Changes requested"), current: L("Waiting"), waiting: L("Up next") }[st];
      const due = st === "approved" ? null : dueText(s, st === "current" || st === "changes");
      // Earlier rounds: who decided what, before it was sent again.
      const rounds = Array.isArray(s.rounds) && s.rounds.length
        ? h("details", { "data-ap-rounds": true },
          h("summary", {}, L(["{n} earlier round", "{n} earlier rounds"], { n: s.rounds.length })),
          h("ol", {}, ...s.rounds.map((r) => h("li", {}, (r.approvers ?? []).filter((p) => p.decision).map((p) => `${p.name}: ${L(DECIDED[p.decision] ?? p.decision)}${p.comment ? ` (“${p.comment}”)` : ""}${p.at ? `, ${when(p.at)}` : ""}`).join("; ") || L("No decisions")))))
        : null;
      // Labels: "Alongside the stage before"
      ol.append(h("li", { "data-ap-stage": s.id ?? i, "data-state": st, "data-parallel": (s.parallel && i > 0) || null, "aria-current": st === "current" ? "step" : null, "aria-description": s.parallel && i > 0 ? L("Alongside the stage before") : null },
        h("span", { "data-ap-mark": true, "aria-hidden": "true" }, mark),
        h("div", { "data-ap-body": true },
          h("div", { "data-ap-head": true }, h("h3", {}, s.title ?? L("Stage {n}", { n: i + 1 })), h("span", { "data-ap-rule": true }, [said, ruleText(s)].filter(Boolean).join(" · ")),
            due ? h("span", { "data-ap-due": true, "data-overdue": due.late || null }, due.text) : null),
          people, rounds)));
    });
    const parts = [ol];

    // On their turn: whoever's looking decides, in the first open stage they're on.
    const at = cfg.readOnly ? -1 : open.find((i) => !mine(list[i])?.decision && mine(list[i])) ?? -1;
    const me = at >= 0 ? mine(list[at]) : null;
    const turn = at;
    if (me && !me.decision) {
      const id = `${cfg.id}-comment`;
      const note = h("textarea", { id, rows: "3", placeholder: L("Optional when approving"), "aria-invalid": wrong ? "true" : null, "aria-describedby": wrong ? `${id}-error` : null });
      note.value = draft;
      note.addEventListener("input", () => (draft = note.value));
      const act = (d, text) => h("button", { type: "button", "data-ap-act": d, onclick: () => decide(turn, me.id, d) }, text);
      parts.push(h("div", { "data-ap-decide": true, role: "group", "aria-label": L("Your decision") },
        h("label", { for: id }, group.length > 1 ? L("Your decision on {stage}, {name}", { stage: list[turn].title ?? L("Stage {n}", { n: turn + 1 }), name: me.name }) : L("Your decision, {name}", { name: me.name })), note,
        wrong ? h("span", { "data-ap-error": true, id: `${id}-error` }, wrong) : null,
        h("div", { "data-ap-buttons": true }, act("approved", L("Approve")), act("changes", L("Request changes")), act("rejected", L("Reject")))));
      // Handing the turn to someone else, from `people` not already on this stage.
      const others = (Array.isArray(K.value(api, cfg.people)) ? K.value(api, cfg.people) : []).filter((p) => isObj(p) && !(list[turn].approvers ?? []).some((a) => same(a.id, p.id)));
      if (others.length) {
        const pid = `${cfg.id}-handoff`;
        const pick = h("select", { id: pid }, h("option", { value: "" }, L("Pick someone")), ...others.map((p) => h("option", { value: String(p.id) }, p.name ?? String(p.id))));
        // Labels: "Hand it to", "Hand over"
        parts.push(h("div", { "data-ap-handoff": true }, h("label", { for: pid }, L("Hand it to")), pick,
          h("div", { "data-ap-buttons": true }, h("button", { type: "button", "data-ap-act": "handoff", onclick: () => handoff(turn, me, others.find((p) => String(p.id) === pick.value)) }, L("Hand over")))));
      }
    }
    // A turn handed on and not yet decided: whoever handed it can take it back.
    if (!cfg.readOnly) {
      for (const i of open) {
        const p = (list[i].approvers ?? []).find((a) => !a.decision && a.for && same(a.for.id, K.value(api, cfg.me)));
        // Labels: "Take back your turn from {name}"
        if (p) parts.push(h("div", { "data-ap-buttons": true }, h("button", { type: "button", "data-ap-act": "takeback", onclick: () => takeBack(i, p) }, L("Take back your turn from {name}", { name: p.name }))));
      }
    }
    // After changes or a rejection, whoever sent it can send it again.
    if ((overall === "changes" || overall === "rejected") && !cfg.readOnly && same(K.value(api, cfg.owner), K.value(api, cfg.me))) {
      parts.push(h("div", { "data-ap-buttons": true }, h("button", { type: "button", "data-ap-act": "resend", onclick: resend }, L("Send it again"))));
    }
    const focused = view.contains(document.activeElement) ? document.activeElement.getAttribute("data-ap-act") ?? (document.activeElement.tagName === "TEXTAREA" ? "note" : null) : null;
    view.replaceChildren(...parts);
    if (focused === "note") view.querySelector("textarea")?.focus();
    else if (focused) (view.querySelector(`[data-ap-act="${focused}"]`) ?? summary).focus?.();

    if (cfg.bindStatus && api.state[cfg.bindStatus] !== overall) api.set(cfg.bindStatus, overall);
    if (last !== null && last !== overall && (overall === "approved" || overall === "rejected")) api.emit("done", { status: overall });
    last = overall;
  }

  // The summary as a sentence or two, for screen readers.
  const told = () => [...summary.querySelectorAll("b, small")].map((x) => x.textContent).filter(Boolean).join(". ");

  // ---- deciding, and sending again
  function decide(i, who, decision) {
    const comment = draft.trim();
    // Saying no, or asking for changes, needs a reason.
    if (decision !== "approved" && !comment) {
      wrong = decision === "rejected" ? L("Say why you're rejecting it") : L("Say what needs to change");
      draw();
      view.querySelector("textarea")?.focus();
      say(wrong);
      return;
    }
    wrong = null;
    draft = "";
    const at = new Date().toISOString();
    const next = stages().map((s, k) => (k !== i ? s : { ...s, approvers: (s.approvers ?? []).map((p) => (same(p.id, who) ? { ...p, decision, comment: comment || undefined, at } : p)) }));
    api.set(cfg.bind, next);
    api.emit("decide", { stage: next[i].id ?? i, approver: who, decision, comment });
    draw();
    say(L("{decision}: {stage}", { decision: L(DECIDED[decision]), stage: next[i].title ?? L("Stage {n}", { n: i + 1 }) }) + ". " + told());
  }
  // The turn goes to `to`, who takes the approver's place and is shown deciding for them.
  function handoff(i, from, to) {
    if (!to) return say(L("Pick someone to hand it to"));
    const taken = { id: to.id, name: to.name ?? String(to.id), ...(to.avatar ? { avatar: to.avatar } : {}), for: { id: from.id, name: from.name } };
    const next = stages().map((s, k) => (k !== i ? s : { ...s, approvers: (s.approvers ?? []).map((p) => (same(p.id, from.id) ? taken : p)) }));
    api.set(cfg.bind, next);
    api.emit("handoff", { stage: next[i].id ?? i, from: from.id, to: to.id });
    draw();
    say(L("Handed to {name}", { name: taken.name }) + ". " + told());
  }
  // The one who handed it on is the approver again.
  function takeBack(i, p) {
    const back = { id: p.for.id, name: p.for.name ?? String(p.for.id) };
    const next = stages().map((s, k) => (k !== i ? s : { ...s, approvers: (s.approvers ?? []).map((a) => (a === p || (same(a.id, p.id) && a.for) ? back : a)) }));
    api.set(cfg.bind, next);
    api.emit("takeback", { stage: next[i].id ?? i, from: p.id, to: back.id });
    draw();
    // Labels: "Your turn again"
    say(L("Your turn again") + ". " + told());
  }
  function resend() {
    // The decisions so far become an earlier round of each stage that had any.
    const next = stages().map((s) => {
      const decided = (s.approvers ?? []).some((p) => p.decision);
      return {
        ...s,
        ...(decided ? { rounds: [...(Array.isArray(s.rounds) ? s.rounds : []), { approvers: s.approvers.map(({ id, name, decision, comment, at }) => ({ id, name, decision, comment, at })) }] } : {}),
        approvers: (s.approvers ?? []).map(({ decision, comment, at, ...p }) => p),
      };
    });
    api.set(cfg.bind, next);
    api.emit("resend", {});
    draw();
    say(L("Sent again") + ". " + told());
  }

  draw();
  let seen = JSON.stringify([stages(), K.value(api, cfg.me)]);
  return {
    update() {
      // Decisions made elsewhere (another approver's, from the server) show as they come.
      const now = JSON.stringify([stages(), K.value(api, cfg.me)]);
      if (now === seen) return;
      seen = now;
      draw();
    },
  };
};
