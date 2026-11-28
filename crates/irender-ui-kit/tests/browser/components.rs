//! Forms, tables, pickers, media, developer tools, guidance and assistant components, each
//! page driven in the browser.

use super::drive;
use irender_core::Format;
use irender_html::Engine;

/// Renders `doc` (YAML, without its `version`), runs `driver` in it and gives the lines it
/// logged. `log` takes anything; `until` waits for what doesn't follow the page's clock.
fn run(name: &str, doc: &str, driver: &str) -> Option<String> {
    run_with(name, doc, driver, "")
}

/// [`run`], with `head` before the page's own scripts.
fn run_with(name: &str, doc: &str, driver: &str, head: &str) -> Option<String> {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&format!("version: 1\n{doc}"), Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</head>", &format!("{head}</head>"));
    let script = format!(
        r##"<script>
    window.addEventListener("load", async () => {{
      const out = [];
      const log = (...a) => out.push(a.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).join(" ").replace(/[\u202f\u00a0]/g, " "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Each turn yields once (a message, which the page's clock doesn't wait for), and every
      // 50th waits a little on that clock: without it Chrome's virtual time never goes idle,
      // and answers from other threads (IndexedDB, say) don't get back to the page. It gives
      // up after 10 seconds and 40,000 turns (or 400,000, should the clock stand still), and
      // says so, rather than going on as if `f` held.
      const until = async (f) => {{
        const start = Date.now();
        for (let i = 0; !f(); i++) {{
          if (i >= 400000 || (i >= 40000 && Date.now() - start > 10000)) throw new Error("until: still waiting after " + i + " turns");
          await (i % 50 ? new Promise((r) => {{ const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); }}) : wait(10));
        }}
      }};
      // A promise that doesn't follow the page's clock: storage, locks, workers, media.
      const real = async (p) => {{ let v, d = false, x; p.then((r) => {{ v = r; d = true; }}, (e) => {{ x = e; d = true; }}); await until(() => d); if (x) throw x; return v; }};
      const $ = (s, from = document) => from.querySelector(s), $$ = (s, from = document) => [...from.querySelectorAll(s)];
      const root = $("[data-i-root]"), S = () => root.__irender.state;
      const key = (k, m = {{}}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", {{ key: k, bubbles: true, cancelable: true, ...m }}));
      const type = (el, v) => {{ el.focus(); el.value = v; el.dispatchEvent(new Event("input", {{ bubbles: true }})); }};
      const pointer = (t, x, y, n) => n.dispatchEvent(new PointerEvent(t, {{ clientX: x, clientY: y, pointerId: 1, button: 0, bubbles: true, cancelable: true, pointerType: "mouse" }}));
      // A paste or drop, with what it carries: not every engine lets a script set it.
      const carry = (t, n, prop, data) => {{ const e = new Event(t, {{ bubbles: true, cancelable: true }}); Object.defineProperty(e, prop, {{ value: data }}); n.dispatchEvent(e); }};
      window.addEventListener("error", (e) => out.push("ERROR " + e.message));
      try {{
        await wait(50);
        {driver}
      }} catch (e) {{ out.push("EXCEPTION " + e.stack); }}
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    }});
    </script>"##
    );
    drive(name, &html.replace("</body>", &format!("{script}</body>")))
}

/// Runs a page and checks its lines.
fn check(name: &str, doc: &str, driver: &str, expected: &[&str]) {
    let Some(result) = run(name, doc, driver) else { return };
    let result = result.replace("&lt;", "<").replace("&gt;", ">");
    assert_eq!(result.lines().collect::<Vec<_>>(), expected, "\n{result}");
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_cells_are_edited_like_a_spreadsheet() {
    check(
        "table-edit",
        r##"
state:
  wrong: []
  changes: {}
  rows:
    - { id: 1, name: Ada, qty: 3, status: open, due: 2026-11-20, paid: true }
    - { id: 2, name: Bob, qty: nope, status: closed, due: 2026-12-01, paid: false }
    - { id: 3, name: "", qty: 7, status: open, due: 2026-11-25, paid: false }
root:
  type: ui-data-table
  props: { rows: "{{ rows }}", addRow: true, deletable: true, sort: name, bindErrors: wrong, bindChanges: changes, newRow: { status: open }, columnsMenu: false }
  children:
    - { type: ui-data-table-column, props: { field: name, edit: text, required: true } }
    - { type: ui-data-table-column, props: { field: qty, edit: number, min: 1 } }
    - { type: ui-data-table-column, props: { field: status, edit: select, options: [{ value: open, label: Open }, { value: closed, label: Closed }] } }
    - { type: ui-data-table-column, props: { field: due, edit: date } }
    - { type: ui-data-table-column, props: { field: paid, edit: checkbox } }
"##,
        r##"
        const cell = (r, c) => $$("[data-dt-body] > tr")[r].querySelector(`[data-dt-col=${c}]`);
        const text = () => $$("[data-dt-body] > tr:not([hidden])").map((t) => [...t.querySelectorAll("[data-dt-col]")].slice(0, 4).map((c) => c.textContent).join("|")).join(" ; ");
        log(text()); log(S().wrong.map((w) => w.message), $("[data-dt-problems]").textContent, cell(1, "qty").getAttribute("aria-invalid"));
        // Enter edits, and goes to the row that was below, which the edit sorts up.
        cell(0, "name").click(); log(document.activeElement === cell(0, "name"), !!$("[data-dt-editor]"));
        key("Enter"); $("[data-dt-editor]").value = "Zed"; key("Enter"); await wait(30);
        log(text(), document.activeElement.textContent, S().changes);
        key("z", { ctrlKey: true }); log("undo", text()); key("y", { ctrlKey: true }); log("redo", text()); key("z", { ctrlKey: true });
        // Escape leaves the value; Tab goes on to the next editable cell.
        cell(0, "name").click(); key("F2"); $("[data-dt-editor]").value = "No"; key("Escape"); log(S().rows[0].name, document.activeElement.getAttribute("data-dt-col"));
        key("Tab"); log(document.activeElement.getAttribute("data-dt-col")); key("9"); log($("[data-dt-editor]").value); key("Tab"); log(S().rows[0].qty, document.activeElement.getAttribute("data-dt-col"));
        // Pasted cells are read as their columns'; the ones past the end are new rows.
        cell(1, "qty").click();
        carry("paste", document.activeElement, "clipboardData", { getData: () => "1,234\t Closed\n5%\topen\n8\tclosed", files: [] });
        await wait(30); log("paste", text()); log(S().rows.length, S().rows[3]);
        // Filled down, cleared, and a box ticked.
        cell(0, "status").click(); key("ArrowDown", { shiftKey: true }); key("ArrowDown", { shiftKey: true }); key("d", { ctrlKey: true }); log(S().rows.slice(0, 3).map((r) => r.status));
        key("Delete"); log(S().rows.slice(0, 3).map((r) => r.status));
        cell(0, "paid").click(); key(" "); log(S().rows[0].paid);
        // Only the rows with problems.
        $("[data-dt-problems]").click(); log($$("[data-dt-body] > tr:not([hidden])").length); $("[data-dt-problems]").click();
        $("[data-dt-add-row]").click(); await wait(30); log(S().rows.length, S().rows.at(-1).status, document.activeElement.tagName); key("Escape");
        $$("[data-dt-select]")[0].click(); $("[data-dt-delete-rows]").click(); log(S().rows.length, S().changes[1], $("[data-dt-bulk]").hidden);
"##,
        &[
            "Ada|3|Open|2026-11-20 ; Bob|nope|Closed|2026-12-01 ; |7|Open|2026-11-25",
            r#"["Not a number","Required"] 2 problems true"#,
            "true false",
            r#"Bob|nope|Closed|2026-12-01 ; Zed|3|Open|2026-11-20 ; |7|Open|2026-11-25 Bob {"1":{"name":"Zed"}}"#,
            "undo Ada|3|Open|2026-11-20 ; Bob|nope|Closed|2026-12-01 ; |7|Open|2026-11-25",
            "redo Bob|nope|Closed|2026-12-01 ; Zed|3|Open|2026-11-20 ; |7|Open|2026-11-25",
            "Ada name",
            "qty",
            "9",
            "9 status",
            "paste Ada|9|Open|2026-11-20 ; Bob|1234|Closed|2026-12-01 ; |0.05|Open|2026-11-25 ; |8|Closed|",
            r#"4 {"status":"closed","qty":8,"id":"new-4"}"#,
            r#"["open","open","open"]"#,
            "[null,null,null]",
            "false",
            // The row the cursor is in stays, though it's right.
            "3",
            "5 open INPUT",
            "4 null true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn stepper_repeater_and_form() {
    check(
        "forms",
        r##"
state:
  step: who
  name: ""
  sent: false
  email: ""
  errors: {}
  dirty: false
  valid: false
  saved: 0
  wrong: []
  totals: {}
  lines: [ { item: Desk, qty: 2, price: 100.5 }, { item: Lamp, qty: 1, price: 40 } ]
root:
  type: stack
  children:
    - type: ui-stepper
      props: { bind: step, finishLabel: Send }
      on: { finish: { action: set_state, key: sent, value: true } }
      children:
        - { type: ui-step, props: { value: who, label: Who, valid: "{{ name != '' }}" }, children: [ { type: ui-input, props: { bind: name }, attrs: { id: name } } ] }
        - { type: ui-step, props: { value: what, label: What }, children: [ { type: ui-text, props: { text: two } } ] }
        - { type: ui-step, props: { value: last, label: Last, error: "{{ name == 'bad' }}" }, children: [ { type: ui-text, props: { text: three } } ] }
    - type: ui-repeater
      props:
        bind: lines
        bindErrors: wrong
        bindTotals: totals
        reorderable: true
        duplicable: true
        max: 4
        fields:
          - { field: item, label: Item, required: true }
          - { field: qty, label: Qty, type: number, min: 1, default: 1 }
          - { field: price, label: Price, type: currency }
          - { field: total, label: Total, compute: qty * price, summary: sum }
    - type: ui-form
      props: { bindErrors: errors, bindValid: valid, bindDirty: dirty, guard: true, rules: { email: { label: Email, required: true, type: email } } }
      on: { submit: { action: set_state, key: saved, value: "{{ saved + 1 }}" } }
      children:
        - { type: ui-error-summary }
        - { type: ui-field, props: { label: Email, error: "{{ errors.email }}" }, children: [ { type: ui-input, props: { bind: email }, attrs: { id: email } } ] }
        - { type: element, props: { tag: a }, attrs: { href: "/elsewhere" }, children: [ { type: text, props: { value: leave } } ] }
        - { type: ui-save-bar }
"##,
        r##"
        const steps = () => log($$("ol")[0].querySelectorAll("li").length, [...$$("ol > li[data-status]")].map((l) => l.dataset.status + (l.hasAttribute("data-error") ? "!" : "") + (l.querySelector("button").disabled ? "x" : "")).join());
        const next = () => $("[data-step-panel]:not([hidden]) [data-step-buttons]").lastElementChild;
        steps(); log(next().disabled); type($("#name"), "bad"); steps(); next().click(); next().click(); steps(); log(next().textContent); next().click(); log(S().sent);
        // The repeater: values, computed ones, totals, messages.
        const f = (n, i) => $$(`[data-field=${n}]`)[i];
        log(S().lines.map((l) => l.total), S().totals, f("total", 0).textContent);
        type(f("qty", 0), "3"); log(S().lines[0].total, S().totals); type(f("qty", 0), "abc"); log(S().wrong.map((w) => w.message)); f("qty", 0).blur(); await wait(30); log(f("qty", 0).getAttribute("aria-invalid")); type(f("qty", 0), "2");
        $$("[data-rep-copy]")[0].click(); log(S().lines.length); $$("[data-rep-remove]")[0].click(); log(S().lines.map((l) => l.item), document.activeElement.getAttribute("aria-label"));
        $$("[data-rep-grip]")[0].focus(); key("ArrowDown", { altKey: true }); log(S().lines.map((l) => l.item), document.activeElement === $$("[data-rep-grip]")[1]);
        f("item", 1).focus(); key("Enter"); log(S().lines.length, document.activeElement === f("item", 2), S().lines[2].qty);
        // The form: checked on leaving and on submit, its changes known.
        const bar = $("[data-form-bar]"), email = $("#email");
        log(bar.hidden, S().valid); type(email, "a"); log(bar.hidden, S().dirty, S().errors); email.blur(); $("a").focus(); await wait(160); log(S().errors);
        $("[data-form-save]").click(); await wait(10); log(S().saved, $("[role=alert]").textContent, document.activeElement.getAttribute("role"));
        $("[role=alert] button").click(); log(document.activeElement === email); $("a[href='/elsewhere']").click(); log($("dialog").open); $("[data-form-stay]").click();
        type(email, "a@b.co"); $("[data-form-save]").click(); log(S().saved, S().valid); root.__irender.set("dirty", false); log(bar.hidden);
        type(email, "x"); email.blur(); $("[data-form-discard]").click(); log(S().email, email.value, bar.hidden);
"##,
        &[
            "3 current,upcomingx,upcomingx",
            "true",
            "3 current,upcoming,upcoming!x",
            "3 done,done,current!",
            "Send",
            "true",
            r#"[201,40] {"total":241} 201"#,
            r#"301.5 {"total":341.5}"#,
            r#"["Not a number"]"#,
            "true",
            "3",
            r#"["Desk","Lamp"] Remove"#,
            r#"["Lamp","Desk"] true"#,
            "3 true 1",
            "true false",
            "false true {}",
            r#"{"email":"Enter an email address"}"#,
            "0 There's a problemEmail: Enter an email address alert",
            "true",
            "true",
            "1 true",
            "true",
            "a@b.co a@b.co true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn countdown_stands_still_while_paused() {
    check(
        "countdown-paused",
        r##"
state: { due: "+60s", hold: false, left: null, st: null, ended: 0 }
root:
  type: ui-countdown
  props: { to: "{{ due }}", paused: "{{ hold }}", bindLeft: left, bindStatus: st, warn: 20s }
  attrs: { id: c }
  on: { end: { action: set_state, key: ended, value: "{{ ended + 1 }}" } }
"##,
        r##"
        const c = $("#c"), text = () => $("[data-cd-text]", c).textContent, due = () => Date.parse($("[data-cd-text]", c).getAttribute("datetime"));
        // Read once paused: pausing works the time out afresh, and in real time (not Chrome's
        // virtual time) the last tick may have come late, so it can be lower than that tick's.
        await wait(2000); const left1 = S().left, was = c.hasAttribute("data-paused"); root.__irender.set("hold", true);
        const left0 = S().left, due0 = due(), t0 = Date.now(); log(left1 < 60 && left0 <= left1 && left0 > 30, was);
        // Paused: the time left stands still, and says so.
        await wait(5000);
        log(S().left === left0, text() === `${left0}s · Paused`, c.hasAttribute("data-paused"), c.getAttribute("aria-description").endsWith(", paused"));
        // Resumed: it goes on from there, the deadline moved out by the pause (five seconds,
        // or more on a busy machine).
        root.__irender.set("hold", false); const held = Date.now() - t0; await wait(2000);
        log(S().left < left0, Math.abs(due() - due0 - held) < 50, c.hasAttribute("data-paused"));
        // A new deadline from the page has the pause in it already: nothing is added.
        root.__irender.set("due", new Date(Date.now() + 30200).toISOString()); await wait(50);
        log(S().left, S().st);
        // Paused past zero, it doesn't end; resumed, it does.
        root.__irender.set("due", new Date(Date.now() + 1500).toISOString()); root.__irender.set("hold", true); await wait(3000);
        log(S().st, S().ended); root.__irender.set("hold", false); await wait(2000); log(S().st, S().ended);
"##,
        &["true false", "true true true true", "true true false", "30 ok", "warning 0", "over 1"],
    );
}

/// `ui-countdown` in working time, on a clock that starts two seconds before 5pm on a
/// Wednesday (local time).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn countdown_counts_working_hours() {
    let Some(result) = run_with(
        "countdown-hours",
        r##"
state: { left: null, wide: null }
root:
  type: stack
  children:
    - { type: ui-countdown, attrs: { id: c }, props: { to: "2026-12-03T09:30", from: "2026-12-02T15:00", workdays: true, hours: "09:00-17:00", bindLeft: left } }
    - { type: ui-countdown, attrs: { id: w }, props: { to: "2026-12-07T10:00", workdays: true, hours: "09:00-17:00", holidays: ["2026-12-04"], bindLeft: wide } }
"##,
        r##"
        const c = $("#c"), text = (n) => $("[data-cd-text]", n).textContent, used = () => Math.round(parseFloat(c.style.getPropertyValue("--used")));
        // Two seconds today and half an hour tomorrow; the bar has two hours of 2h 30m used.
        log(S().left, text(c), c.hasAttribute("data-closed"), used());
        // Thursday's eight hours and Monday's one; Friday's a holiday, the weekend doesn't work.
        log(S().wide, text($("#w")));
        // At five it stands still until nine tomorrow.
        await until(() => Date.now() >= Date.parse("2026-12-02T17:00:01")); await wait(1100);
        log(S().left, text(c), c.hasAttribute("data-closed"), c.getAttribute("aria-description"));
"##,
        r#"<script>{ const start = Date.now(), at = Date.parse("2026-12-02T16:59:58"), now = Date.now.bind(Date); Date.now = () => at + (now() - start); }</script>"#,
    ) else {
        return;
    };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        ["1802 30m 02s false 80", "32402 9h 0m", "1800 30m 00s · Out of hours true 30 minutes left, out of hours"],
        "\n{result}"
    );
}

/// `ui-countdown` with hours by weekday, and a night shift running past midnight, on a
/// clock at 23:00 on Friday 2026-12-04 (local time).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn countdown_counts_hours_by_weekday_and_night_shifts() {
    let Some(result) = run_with(
        "countdown-weekdays",
        r##"
state: { week: null, night: null }
root:
  type: stack
  children:
    - { type: ui-countdown, props: { to: "2026-12-07T11:00", hours: { sat: "10:00-14:00", mon: "09:00-17:00" }, bindLeft: week } }
    - { type: ui-countdown, props: { to: "2026-12-05T05:00", hours: "22:00-06:00", bindLeft: night } }
"##,
        r##"
        // Saturday's four hours and Monday's two; Friday night and Sunday don't work.
        // The night shift: from 23:00 to 05:00 tomorrow, all of it working.
        log(S().week / 3600, S().night / 3600);
"##,
        r#"<script>{ const start = Date.now(), at = Date.parse("2026-12-04T23:00:00"), now = Date.now.bind(Date); Date.now = () => at + (now() - start); }</script>"#,
    ) else {
        return;
    };
    assert_eq!(result.lines().collect::<Vec<_>>(), ["6 6"], "\n{result}");
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn stepper_keeps_the_furthest_step_reached() {
    check(
        "stepper-furthest",
        r##"
state: { step: one, far: null, name: "" }
root:
  type: ui-stepper
  props: { bind: step, bindFurthest: far }
  children:
    - { type: ui-step, props: { value: one, label: One, valid: "{{ name != '' }}" }, children: [ { type: ui-input, props: { bind: name }, attrs: { id: name } } ] }
    - { type: ui-step, props: { value: two, label: Two }, children: [ { type: ui-text, props: { text: two } } ] }
    - { type: ui-step, props: { value: three, label: Three }, children: [ { type: ui-text, props: { text: three } } ] }
    - { type: ui-step, props: { value: four, label: Four }, children: [ { type: ui-text, props: { text: four } } ] }
"##,
        r##"
        const steps = () => $$("ol > li").map((l) => l.dataset.status[0] + (l.querySelector("button").disabled ? "x" : "")).join(" ");
        const next = () => $("[data-step-panel]:not([hidden]) [data-step-buttons]").lastElementChild;
        const head = (i) => $$("ol > li button")[i];
        log(steps(), S().far); type($("#name"), "Ada"); next().click(); next().click(); log(steps(), S().step, S().far);
        // Back to the start: what was reached stays open, and done.
        head(0).click(); log(steps(), S().step, S().far);
        head(2).click(); log(steps(), S().step, S().far);
        $("[data-step-panel]:not([hidden]) [data-step-buttons]").firstElementChild.click(); log(S().step, S().far);
        // An earlier step that's no longer valid still shuts the ones after it.
        head(0).click(); type($("#name"), ""); log(steps());
        // The page moving the step on its own moves the furthest step with it, as it's read.
        type($("#name"), "Ada"); root.__irender.set("step", "four"); head(0).click(); log(steps(), S().far);
"##,
        &["c ux ux ux null", "d d c ux three three", "c d u ux one three", "d d c ux three three", "two three", "c ux ux ux", "c d d u four"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn mindmap_topics_have_icons_and_notes() {
    check(
        "mindmap-notes",
        r##"
state:
  sel: null
  map: { id: root, label: Launch, children: [ { id: a, label: Pricing, icon: "💰", note: Check with finance first }, { id: b, label: Docs } ] }
root: { type: ui-mindmap, attrs: { id: mm }, props: { bind: map, bindSelected: sel } }
"##,
        r##"
        const topic = (id) => $(`[data-mm-topic="${id}"]`), note = $("[data-mm-note]"), box = note.querySelector("textarea");
        log(topic("a").getAttribute("aria-label"), topic("a").getAttribute("aria-description"), !!topic("a").querySelector("[data-mm-has-note]"), !!topic("b").querySelector("[data-mm-has-note]"));
        log(topic("a").querySelector("text").textContent, topic("a").querySelector("title").textContent.replace(/\n+/g, " / "));
        // The selected topic's note is under the map to edit; the central one starts selected.
        log(note.hidden, box.value === "");
        const pick = async (id) => { for (const t of ["pointerdown", "pointerup"]) topic(id).dispatchEvent(new PointerEvent(t, { bubbles: true, button: 0, pointerId: 1 })); topic(id).dispatchEvent(new MouseEvent("click", { bubbles: true })); await wait(30); };
        await pick("a"); log(S().sel, box.value);
        // A note written for another topic is saved on it, and marks it.
        await pick("b");
        box.value = "Ask Mei"; box.dispatchEvent(new Event("change")); await wait(30);
        log(S().map.children[1].note, !!topic("b").querySelector("[data-mm-has-note]"));
"##,
        &["💰 Pricing Check with finance first true false", "💰 Pricing 💰 Pricing / Check with finance first", "false true", "a Check with finance first", "Ask Mei true"],
    );
}

/// `ui-mindmap`: topics reordered among their siblings (Alt and the arrows, or dragged onto
/// another's top or foot), a topic's link, lines between branches, and a tree from the top.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn mindmap_reorders_links_and_lays_out_down() {
    check(
        "mindmap-more",
        r##"
state:
  sel: null
  map: { id: root, label: Launch, links: [ { from: a2, to: b, label: needs } ], children: [ { id: a, label: Pricing, children: [ { id: a1, label: Tiers }, { id: a2, label: Trials, href: "https://example.com/trials" } ] }, { id: b, label: Docs } ] }
  tree: { id: r, label: Org, children: [ { id: x, label: Sales, children: [ { id: x1, label: North } ] }, { id: y, label: Support } ] }
root:
  type: stack
  children:
    - { type: ui-mindmap, attrs: { id: mm }, props: { bind: map, bindSelected: sel, layout: right } }
    - { type: ui-mindmap, attrs: { id: dn }, props: { bind: tree, layout: down } }
"##,
        r##"
        const mm = $("#mm"), dn = $("#dn");
        const topic = (el, id) => el.querySelector(`[data-mm-topic="${id}"]`), stage = (el) => el.querySelector("[tabindex]");
        const order = (t) => (t.children ?? []).map((c) => c.id).join(",");
        const pick = async (el, id) => { for (const t of ["pointerdown", "pointerup"]) topic(el, id).dispatchEvent(new PointerEvent(t, { bubbles: true, button: 0, pointerId: 1 })); await wait(30); };
        const key = (el, k, o = {}) => stage(el).dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...o }));
        // The link's mark, and the line between branches.
        log(!!topic(mm, "a2").querySelector("[data-mm-href]"), topic(mm, "a2").getAttribute("aria-description"), mm.querySelectorAll("[data-mm-link]").length, mm.querySelector("[data-mm-links] text")?.textContent);
        let opened = null; window.open = (u) => (opened = u);
        await pick(mm, "a2"); stage(mm).focus(); key(mm, "Enter", { ctrlKey: true }); await wait(10);
        log("open", opened);
        // Alt and ↑: Trials before Tiers.
        key(mm, "ArrowUp", { altKey: true }); await wait(30);
        log("alt", order(S().map.children[0]), S().sel);
        // Dragged onto the foot of Docs: after it, among the first branches.
        const g = topic(mm, "a1").querySelector("rect:not([data-mm-ring])").getBoundingClientRect(), d = topic(mm, "b").querySelector("rect:not([data-mm-ring])").getBoundingClientRect();
        const p = (type, x, y) => topic(mm, "a1").dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 2, clientX: x, clientY: y }));
        p("pointerdown", g.left + 5, g.top + 5); p("pointermove", g.left + 30, g.top + 30);
        p("pointermove", d.left + d.width / 2, d.bottom - 2); await wait(10);
        log("slot", !!mm.querySelector("[data-mm-slot]"));
        p("pointerup", d.left + d.width / 2, d.bottom - 2); await wait(30);
        log("dropped", order(S().map), order(S().map.children[0]));
        // Down: the children under their parent, side by side; ↓ goes in, → along.
        const box = (id) => topic(dn, id).querySelector("rect:not([data-mm-ring])").getBoundingClientRect();
        log("down", box("x").top > box("r").bottom, Math.abs(box("x").top - box("y").top) < 1, box("x1").top > box("x").bottom);
        await pick(dn, "r"); stage(dn).focus(); key(dn, "ArrowDown"); await wait(10); key(dn, "ArrowRight"); await wait(10);
        log("keys", dn.querySelector('[data-mm-topic][aria-selected="true"]').getAttribute("data-mm-topic"));
        key(dn, "ArrowLeft", { altKey: true }); await wait(30);
        log("alt across", order(S().tree));
"##,
        &[
            "true Has a link: ⌘ or Ctrl and Enter opens it 1 needs",
            "open https://example.com/trials",
            "alt a2,a1 a2",
            "slot true",
            "dropped a,b,a1 a2",
            "down true true true",
            "keys y",
            "alt across y,x",
        ],
    );
}

/// `ui-mindmap`: a picture on a topic, the radial layout, and lines between topics drawn,
/// picked, labelled and removed on the map.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn mindmap_pictures_radial_and_lines_by_hand() {
    check(
        "mindmap-radial",
        r##"
state:
  map: { id: r, label: Plan, children: [ { id: a, label: Build, image: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10'/%3E", children: [ { id: a1, label: Code }, { id: a2, label: Test } ] }, { id: b, label: Ship, image: { src: "javascript:alert(1)" } }, { id: c, label: Tell, image: { src: "/p.png", height: 400 } } ] }
root: { type: ui-mindmap, attrs: { id: mm }, props: { bind: map, layout: radial, height: 600 } }
"##,
        r##"
        const mm = $("#mm"), stage = mm.querySelector("[tabindex]");
        const topic = (id) => mm.querySelector(`[data-mm-topic="${id}"]`), box = (id) => topic(id).querySelector("rect:not([data-mm-ring])").getBoundingClientRect();
        const mid = (id) => { const b = box(id); return [b.left + b.width / 2, b.top + b.height / 2]; };
        const press = async (n, x, y) => { for (const t of ["pointerdown", "pointerup"]) n.dispatchEvent(new PointerEvent(t, { bubbles: true, button: 0, pointerId: 1, clientX: x, clientY: y })); await wait(30); };
        // Pictures: only an address that's allowed, kept to 200 px; the topic grows to hold it.
        log(!!topic("a").querySelector("[data-mm-image]"), !!topic("b").querySelector("[data-mm-image]"), topic("c").querySelector("[data-mm-image]")?.getAttribute("height"), Math.round(box("a").height) > Math.round(box("b").height));
        // Round the middle: the first branches as far from it, all round; a child further out than its parent.
        const [rx, ry] = mid("r"), far = (id) => Math.round(Math.hypot(mid(id)[0] - rx, mid(id)[1] - ry));
        log(far("a") === far("b") && far("b") === far("c"), far("a1") > far("a"), mid("b")[1] > ry, new Set(["a", "b", "c"].map((id) => Math.sign(Math.round(mid(id)[0] - rx)))).size > 1);
        // A line by hand: Alt+L on Code, → … pick Ship, Enter.
        await press(topic("a1"), ...mid("a1")); stage.focus();
        stage.dispatchEvent(new KeyboardEvent("keydown", { key: "¬", code: "KeyL", altKey: true, bubbles: true, cancelable: true })); await wait(10);
        log("pressed", mm.querySelector("[data-mm-bar] [aria-pressed]").getAttribute("aria-pressed"));
        await press(topic("b"), ...mid("b"));
        log("drawn", JSON.stringify(S().map.links), mm.querySelectorAll("[data-mm-link]").length, mm.querySelector("[data-mm-bar] [aria-pressed]").getAttribute("aria-pressed"));
        // Joined already: not twice, either way round.
        await press(topic("b"), ...mid("b")); stage.focus(); stage.dispatchEvent(new KeyboardEvent("keydown", { key: "¬", code: "KeyL", altKey: true, bubbles: true, cancelable: true }));
        await press(topic("a1"), ...mid("a1"));
        log("again", S().map.links.length);
        // Its label under the map; then a click on it picks it, and Delete removes it.
        await press(topic("a1"), ...mid("a1"));
        const lab = mm.querySelector("[data-mm-link-label]"); lab.value = "ships"; lab.dispatchEvent(new Event("change", { bubbles: true })); await wait(30);
        log("label", JSON.stringify(S().map.links), mm.querySelector("[data-mm-links] text")?.textContent, mm.querySelector("[data-mm-linklist] span").textContent);
        const hit = mm.querySelector("[data-mm-link-hit]"), len = hit.getTotalLength(), pt = hit.getPointAtLength(len / 2), m = hit.getScreenCTM();
        await press(hit, pt.x * m.a + pt.y * m.c + m.e, pt.x * m.b + pt.y * m.d + m.f);
        log("picked", mm.querySelector('[data-mm-link][stroke="var(--color-primary)"]')?.getAttribute("data-mm-link"), mm.querySelector("[data-mm-linklist] li[data-picked]")?.dataset.mmLine);
        stage.focus(); key("Delete"); await wait(30);
        log("removed", JSON.stringify(S().map.links), !!topic("a1"));
        // Escape stops a line half made.
        stage.dispatchEvent(new KeyboardEvent("keydown", { key: "¬", code: "KeyL", altKey: true, bubbles: true, cancelable: true })); key("Escape"); await wait(10);
        await press(topic("c"), ...mid("c"));
        log("stopped", S().map.links.length);
        // The keys round the middle: → out, ↓ along, ← back in.
        await press(topic("r"), ...mid("r")); stage.focus(); key("ArrowRight"); key("ArrowDown"); await wait(10);
        const now = () => mm.querySelector('[data-mm-topic][aria-selected="true"]').getAttribute("data-mm-topic");
        const at1 = now(); key("ArrowLeft"); await wait(10);
        log("keys", at1, now());
"##,
        &[
            "true false 200 true",
            "true true true true",
            "pressed true",
            "drawn [{\"from\":\"a1\",\"to\":\"b\"}] 1 false",
            "again 1",
            "label [{\"from\":\"a1\",\"to\":\"b\",\"label\":\"ships\"}] ships → Ship",
            "picked a1>b a1>b",
            "removed [] true",
            "stopped 0",
            "keys b r",
        ],
    );
}

/// `ui-whiteboard`: a frame carries what's inside it, groups move and select together,
/// locked items stay, items go to the front and back, copy and paste between boards, a
/// drawing resizes, pictures, and PNG export.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn whiteboard_frames_groups_locks_and_pictures() {
    check(
        "whiteboard-more",
        r##"
state:
  other: []
  board:
    - { id: f, type: frame, x: 0, y: 0, w: 300, h: 200, text: Ideas }
    - { id: a, type: rect, x: 20, y: 40, w: 100, h: 60 }
    - { id: b, type: rect, x: 400, y: 40, w: 100, h: 60 }
    - { id: c, type: ellipse, x: 400, y: 200, w: 60, h: 60 }
    - { id: p, type: pen, x: 600, y: 0, points: [ [0, 0], [100, 50] ] }
    - { id: i, type: image, x: 600, y: 200, w: 40, h: 40, src: "data:image/gif;base64,R0lGODlhAQABAIAAAP///wAAACwAAAAAAQABAAACAkQBADs=" }
root:
  type: stack
  children:
    - { type: ui-whiteboard, attrs: { id: wb }, props: { bind: board, height: 500 } }
    - { type: ui-whiteboard, attrs: { id: wb2 }, props: { bind: other, height: 200 } }
"##,
        r##"
        const wb = $("#wb"), stage = wb.querySelector("[data-wb-stage]"), wb2 = $("#wb2"), stage2 = wb2.querySelector("[data-wb-stage]");
        const at = (id) => S().board.find((i) => i.id === id);
        const where = (id) => `${at(id).x},${at(id).y}`;
        const shape = (id) => wb.querySelector(`[data-wb-item="${id}"]`);
        const pt = (id, el = wb) => { const r = el.querySelector(`[data-wb-item="${id}"]`).querySelector("rect, ellipse, path, image").getBoundingClientRect(); return [r.left + 3, r.top + 3]; };
        const ptr = (type, x, y, o = {}) => (o.target ?? stage).dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerId: 5, clientX: x, clientY: y, ...o }));
        const drag = async (from, dx, dy, o = {}) => { ptr("pointerdown", from[0], from[1], o); ptr("pointermove", from[0] + dx / 2, from[1] + dy / 2, o); ptr("pointermove", from[0] + dx, from[1] + dy, o); ptr("pointerup", from[0] + dx, from[1] + dy, o); await wait(30); };
        const key = (k, o = {}, el = stage) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...o }));
        // The frame is drawn first, behind; dragging it brings what's inside along.
        log("order", [...wb.querySelectorAll("[data-wb-item]")].map((g) => g.dataset.wbItem).join(""), wb.querySelector('[data-wb-item="f"] text').textContent, !!wb.querySelector('[data-wb-item="i"] image'));
        await drag(pt("f"), 50, 0, { target: shape("f").querySelector("rect") });
        // (Moves are in the board's units, the pointer's over the view's zoom.)
        log("frame", at("f").x > 0, at("a").x - 20 === at("f").x && at("a").y === 40, where("b"));
        // b and c grouped: clicking one selects both, and they move together.
        stage.focus();
        ptr("pointerdown", ...pt("b"), { target: shape("b").querySelector("rect") }); ptr("pointerup", ...pt("b"), { target: shape("b").querySelector("rect") }); await wait(10);
        ptr("pointerdown", ...pt("c"), { target: shape("c").querySelector("ellipse"), shiftKey: true }); ptr("pointerup", ...pt("c"), { target: shape("c").querySelector("ellipse") }); await wait(10);
        key("g", { metaKey: true }); await wait(20);
        log("group", at("b").group === at("c").group && !!at("b").group);
        key("Escape"); await wait(10);
        await drag(pt("c"), 0, 30, { target: shape("c").querySelector("ellipse") });
        log("moved together", at("b").y > 40, at("b").y - 40 === at("c").y - 200, at("b").x, at("c").x);
        // Locked: neither the keys nor Delete move it.
        key("l", { metaKey: true, shiftKey: true }); await wait(20);
        const before = where("b");
        key("ArrowRight"); await wait(10); key("Delete"); await wait(20);
        log("locked", at("b").locked, where("b") === before, S().board.length, !!shape("b").querySelector("path[stroke]") || shape("b").hasAttribute("data-locked"));
        key("l", { metaKey: true, shiftKey: true }); await wait(20);
        // To the back and the front.
        key("]", { metaKey: true }); await wait(20);
        log("front", S().board.map((i) => i.id).join(""));
        key("[", { metaKey: true }); await wait(20);
        log("back", S().board.map((i) => i.id).join(""));
        // Copied here, pasted on the other board: new ids, still grouped, a little down and right.
        // (Firefox's made-up clipboard events carry no data: it's put on by hand.)
        const dt = new DataTransfer(), clip = (type) => Object.defineProperty(new Event(type, { bubbles: true, cancelable: true }), "clipboardData", { value: dt });
        stage.dispatchEvent(clip("copy"));
        stage2.focus(); stage2.dispatchEvent(clip("paste")); await wait(20);
        const o = S().other;
        log("pasted", o.length, o.every((i) => !["b", "c"].includes(i.id)), o[0].group === o[1].group, o.find((i) => i.type === "rect").x - at("b").x);
        // The drawing, resized from its corner: its points stretch.
        stage.focus(); key("Escape");
        ptr("pointerdown", ...pt("p"), { target: shape("p").querySelector("path") }); ptr("pointerup", ...pt("p"), { target: shape("p").querySelector("path") }); await wait(10);
        const hd = wb.querySelector("[data-wb-handle]").getBoundingClientRect();
        await drag([hd.left + 3, hd.top + 3], 100, 50, { target: wb.querySelector("[data-wb-handle]") });
        const [px, py] = at("p").points[1];
        log("stretched", px > 100, py > 50, px / py === 2);
        // PNG.
        let made = null; wb.addEventListener("export", (e) => (made = e.detail.format));
        const realURL = URL.createObjectURL; URL.createObjectURL = () => "blob:x";
        wb.querySelector("[data-wb-png]").click(); await until(() => made);
        URL.createObjectURL = realURL;
        log("png", made);
"##,
        &[
            "order fabcpi Ideas true",
            "frame true true 400,40",
            "group true",
            "moved together true true 400 400",
            "locked true true 6 true",
            "front fapibc",
            "back bcfapi",
            "pasted 2 true true 24",
            "stretched true true true",
            "png png",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn whiteboard_arrows_stay_joined_to_shapes() {
    check(
        "whiteboard-joins",
        r##"
state:
  board:
    - { id: a, type: rect, x: 0, y: 0, w: 100, h: 60 }
    - { id: b, type: ellipse, x: 300, y: 0, w: 100, h: 60 }
root: { type: ui-whiteboard, attrs: { id: wb }, props: { bind: board, height: 400 } }
"##,
        r##"
        const wb = $("#wb"), stage = wb.querySelector("[data-wb-stage]");
        const r = () => stage.getBoundingClientRect(), pt = (x, y) => ({ clientX: r().left + x + 24, clientY: r().top + y + 24 });
        const ev = (type, x, y, target) => (target ?? document.elementFromPoint(pt(x, y).clientX, pt(x, y).clientY) ?? stage).dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, button: 0, buttons: 1, isPrimary: true, ...pt(x, y) }));
        const drag = async (x0, y0, x1, y1) => { ev("pointerdown", x0, y0); await wait(5); for (let k = 1; k <= 4; k++) { ev("pointermove", x0 + (x1 - x0) * k / 4, y0 + (y1 - y0) * k / 4, stage); await wait(2); } ev("pointerup", x1, y1, stage); await wait(10); };
        const key = (k) => stage.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
        const line = () => wb.querySelector('[data-type=arrow] path').getAttribute("d").match(/^M(-?[\d.]+) (-?[\d.]+)L(-?[\d.]+) (-?[\d.]+)/).slice(1).map((n) => Math.round(Number(n))).join(",");
        const arrow = () => S().board.find((i) => i.type === "arrow");
        // Drawn from the rectangle to the ellipse: joined to both, its ends on their edges.
        stage.focus(); key("a"); await drag(50, 30, 350, 30);
        log(arrow().from, arrow().to, line(), !!wb.querySelector("[data-type=arrow][data-joined]"));
        // The ellipse moves: the arrow follows it, writing nothing of its own.
        key("v"); const before = JSON.stringify(arrow()); await drag(350, 30, 350, 230);
        log(line(), JSON.stringify(arrow()) === before);
        // The arrow moved alone comes loose, where it was.
        ev("pointerdown", 200, 130); ev("pointerup", 200, 130, stage); await wait(10); await drag(200, 130, 200, 160);
        log(arrow().from ?? "-", arrow().to ?? "-", line());
"##,
        &["a b 100,30,300,30 true", "95,60,317,208 true", "- - 95,90,317,238"],
    );
}

/// `ui-whiteboard`: an arrow's ends dragged onto another shape join it, pinned to the spot
/// when let go at its edge, and dragged off let go; drawn to an edge, pinned from the start.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn whiteboard_arrow_ends_join_again_by_dragging() {
    check(
        "whiteboard-ends",
        r##"
state:
  board:
    - { id: a, type: rect, x: 0, y: 0, w: 100, h: 60, text: Start }
    - { id: b, type: rect, x: 300, y: 0, w: 100, h: 60, text: End }
    - { id: c, type: ellipse, x: 300, y: 200, w: 100, h: 60, text: Other }
    - { id: x, type: arrow, x: 0, y: 0, points: [ [0, 0], [10, 0] ], from: a, to: b }
root: { type: ui-whiteboard, attrs: { id: wb }, props: { bind: board, height: 400 } }
"##,
        r##"
        const wb = $("#wb"), stage = wb.querySelector("[data-wb-stage]");
        const r = () => stage.getBoundingClientRect(), pt = (x, y) => ({ clientX: r().left + x + 24, clientY: r().top + y + 24 });
        const ev = (type, x, y, target) => (target ?? document.elementFromPoint(pt(x, y).clientX, pt(x, y).clientY) ?? stage).dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, button: 0, buttons: 1, isPrimary: true, ...pt(x, y) }));
        const drag = async (x0, y0, x1, y1, first) => { ev("pointerdown", x0, y0, first); await wait(5); for (let k = 1; k <= 4; k++) { ev("pointermove", x0 + (x1 - x0) * k / 4, y0 + (y1 - y0) * k / 4, stage); await wait(2); } ev("pointerup", x1, y1, stage); await wait(10); };
        const key = (k) => stage.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
        const line = () => wb.querySelector('[data-wb-item=x] path, [data-type=arrow]:last-of-type path').getAttribute("d").match(/^M(-?[\d.]+) (-?[\d.]+)L(-?[\d.]+) (-?[\d.]+)/).slice(1).map((n) => Math.round(Number(n))).join(",");
        const arrow = () => S().board.find((i) => i.id === "x"), joins = (i = arrow()) => [i.from ?? "-", i.fromAt ?? "-", i.to ?? "-", i.toAt ?? "-"].join(" ");
        // Picked, the arrow shows its two ends.
        stage.focus(); ev("pointerdown", 200, 30); ev("pointerup", 200, 30, stage); await wait(10);
        log($$("#wb [data-wb-end]").map((e) => e.dataset.wbEnd + ":" + Math.round(e.getAttribute("cx"))).join(","));
        // Its end dragged into the ellipse: joined to it, not pinned (let go in the middle); its outline marked on the way.
        const to = () => wb.querySelector("[data-wb-end=to]");
        ev("pointerdown", 300, 30, to()); ev("pointermove", 330, 120, stage); ev("pointermove", 350, 230, stage); await wait(5);
        log("over", !!wb.querySelector("[data-wb-target]"));
        ev("pointerup", 350, 230, stage); await wait(10);
        log(joins(), line(), $("[data-wb-live], [role=status]")?.textContent ?? "");
        // Dragged again, let go on the ellipse's top: pinned there, and it stays there as the other end moves.
        await drag(...to().getAttribute("cx") && [Number(to().getAttribute("cx")), Number(to().getAttribute("cy"))], 350, 202, to());
        log(joins(), line());
        // The start dragged onto the right edge of End: pinned to it, the other end keeping its pin.
        const from = () => wb.querySelector("[data-wb-end=from]");
        await drag(Number(from().getAttribute("cx")), Number(from().getAttribute("cy")), 398, 40, from());
        log(joins(), line());
        // Dragged off onto the empty board: let go there.
        await drag(Number(from().getAttribute("cx")), Number(from().getAttribute("cy")), 150, 300, from());
        log(joins(), line());
        // Undone, it's joined again.
        key("z"); stage.dispatchEvent(new KeyboardEvent("keydown", { key: "z", ctrlKey: true, bubbles: true })); await wait(10);
        log("undone", arrow().from);
        // A new arrow drawn to the left edge of End: pinned from the start.
        key("a"); await drag(50, 100, 302, 30);
        const fresh = S().board.at(-1);
        log(fresh.from ?? "-", fresh.to, JSON.stringify(fresh.toAt), fresh.fromAt ?? "-");
"##,
        &[
            "from:100,to:300",
            "over true",
            "a - c - 95,60,317,208 Arrow joined to Other",
            "a - c 0.5,0 100,58,350,200",
            "b 1,0.667 c 0.5,0 400,40,350,200",
            "- - c 0.5,0 150,300,350,200",
            "undone b",
            "- b [0,0.5] -",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gauge_rounds_its_figure_and_has_ticks() {
    check(
        "gauge-ticks",
        r##"
state: { oee: 72.4567 }
root:
  type: stack
  children:
    - { type: ui-gauge, attrs: { id: g }, props: { value: "{{ oee }}", decimals: 1, unit: "%", ticks: 4 } }
    - { type: ui-gauge, attrs: { id: f }, props: { value: 3.14159, decimals: 2, ticks: 10, shape: half } }
"##,
        r##"
        const fig = (id) => $(`#${id} [data-gauge-value] > span`).textContent;
        log(fig("g"), $$("#g [data-tick]").length, fig("f"), $$("#f [data-tick]").length);
        root.__irender.set("oee", 88.06); await wait(30); log(fig("g"), $("#g").getAttribute("aria-valuenow"));
        // The first and last ticks sit at the dial's ends.
        const t = $$("#f [data-tick]"), a = t[0].getBoundingClientRect(), b = t.at(-1).getBoundingClientRect();
        log(Math.abs(a.top - b.top) < 1, a.left < b.left);
"##,
        &["72.5 5 3.14 11", "88.1 88.06", "true true"],
    );
}

/// `ui-compare-slider` with three pictures (two dividers, kept in order, dragged by the
/// nearest) and `zoom` (every picture alike, the dividers staying put, the view moved by
/// dragging away from them).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn compare_slider_three_pictures_and_zoom() {
    check(
        "compare-many",
        r##"
state: { at: null, z: 1 }
root:
  type: stack
  style: { width: 400px }
  children:
    - type: ui-compare-slider
      attrs: { id: m }
      props:
        bind: at
        ratio: "2/1"
        pictures:
          - { src: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E", label: May }
          - { src: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E", label: June }
          - { src: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E", label: July }
    - type: ui-compare-slider
      attrs: { id: z }
      props: { zoom: 3, bindZoom: z, ratio: "2/1", position: 50, before: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E", after: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E" }
"##,
        r##"
        const m = $("#m"), f = $("#m [data-cmp-frame]"), r = f.getBoundingClientRect();
        const clips = (id) => $$(`#${id} [data-cmp-after]`).map((l) => getComputedStyle(l).clipPath).join(" ");
        const tags = $$("#m [data-cmp-tag]").map((t) => Math.round(t.getBoundingClientRect().left - r.left)).join(",");
        log(clips("m"), tags, $$("#m input").map((i) => i.getAttribute("aria-label") + ":" + i.value).join(","), m.getAttribute("aria-label"));
        // A press near the second divider moves it, and never past the first.
        pointer("pointerdown", r.left + 260, r.top + 50, f); pointer("pointermove", r.left + 300, r.top + 50, f); pointer("pointerup", r.left + 300, r.top + 50, f); await wait(20);
        log(JSON.stringify(S().at), clips("m"));
        pointer("pointerdown", r.left + 290, r.top + 50, f); pointer("pointermove", r.left + 40, r.top + 50, f); pointer("pointerup", r.left + 40, r.top + 50, f); await wait(20);
        log(JSON.stringify(S().at));
        root.__irender.set("at", [10, 90]); await wait(20); log(clips("m"), $$("#m input").map((i) => i.value).join(","));

        const z = $("#z"), zf = $("#z [data-cmp-frame]"), zr = zf.getBoundingClientRect(), img = () => $$("#z img").map((i) => getComputedStyle(i).transform).join(" ");
        const bar = () => [$("#z output").textContent, ...$$("#z [data-cmp-zoom]").map((b) => b.dataset.cmpZoom + (b.disabled ? "-" : "+"))].join(" ");
        log(bar(), zf.hasAttribute("data-zoomed"));
        $("#z [data-cmp-zoom=in]").click(); await wait(20);
        log(bar(), img(), clips("z"), S().z);
        // Ctrl and the wheel at the top left corner: that point stays put.
        zf.dispatchEvent(new WheelEvent("wheel", { bubbles: true, cancelable: true, ctrlKey: true, deltaY: -1000, clientX: zr.left, clientY: zr.top })); await wait(20);
        log(bar(), $("#z img").style.transform || getComputedStyle($("#z img")).transform);
        // Dragging away from the divider moves the view, kept covering the frame.
        pointer("pointerdown", zr.left + 300, zr.top + 100, zf); pointer("pointermove", zr.left + 200, zr.top + 60, zf); pointer("pointerup", zr.left + 200, zr.top + 60, zf); await wait(20);
        log(getComputedStyle(zf).getPropertyValue("--zx"), getComputedStyle(zf).getPropertyValue("--zy"), $("#z input").value);
        pointer("pointerdown", zr.left + 2000, zr.top + 100, zf); pointer("pointermove", zr.left + 4000, zr.top + 100, zf); pointer("pointerup", zr.left + 4000, zr.top + 100, zf); await wait(20);
        log(getComputedStyle(zf).getPropertyValue("--zx"), $("#z input").value);
        // Near the divider (it doesn't move with the zoom), a drag takes it.
        pointer("pointerdown", zr.left + 202, zr.top + 100, zf); pointer("pointermove", zr.left + 100, zr.top + 100, zf); pointer("pointerup", zr.left + 100, zr.top + 100, zf); await wait(20);
        log($("#z input").value, clips("z"));
        $("#z input").focus(); key("-"); await wait(20); log(bar());
        key("0"); await wait(20); log(bar(), zf.hasAttribute("data-zoomed"), S().z);
        root.__irender.set("z", 2); await wait(20); log(bar(), getComputedStyle(zf).getPropertyValue("--zx"));
"##,
        &[
            "inset(0px 0px 0px 33%) inset(0px 0px 0px 67%) 10,142,278 Divider 1:33,Divider 2:67 Comparison",
            "[33,75] inset(0px 0px 0px 33%) inset(0px 0px 0px 75%)",
            "[33,33]",
            "inset(0px 0px 0px 10%) inset(0px 0px 0px 90%) 10,90",
            "100% out- in+ reset- false",
            "150% out+ in+ reset+ matrix(1.5, 0, 0, 1.5, -100, -50) matrix(1.5, 0, 0, 1.5, -100, -50) inset(0px 0px 0px 50%) 1.5",
            "300% out+ in- reset+ matrix(3, 0, 0, 3, -200, -100)",
            "-300 -140 50",
            "0 50",
            "25 inset(0px 0px 0px 25%)",
            "200% out+ in+ reset+",
            "100% out- in+ reset- false 1",
            "200% out+ in+ reset+ -200",
        ],
    );
}

/// `format: oklch`: written as OKLCH, and typed OKLCH (any of its forms) read, a colour
/// outside sRGB brought into it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn color_picker_writes_and_reads_oklch() {
    check(
        "color-oklch",
        r##"
state: { c: null }
root: { type: ui-color-picker, props: { bind: c, inline: true, format: oklch, alpha: true } }
"##,
        r##"
        const field = $("[data-color-text]");
        const put = async (v) => { field.value = v; field.dispatchEvent(new Event("change", { bubbles: true })); await wait(30); return S().c; };
        log(await put("#ff0000"), await put("#1e3a8a"), await put("#777777"), await put("white"));
        log(await put("oklch(62.8% 0.2577 29.23)"), await put("oklch(0.5 0.1 200deg / 50%)"), await put("oklch(70% 50% 140)"));
        // Far outside sRGB: each channel cut to its range.
        log(await put("oklch(90% 0.4 150)"), field.getAttribute("aria-invalid"));
        log(await put("oklch(nope)"), field.getAttribute("aria-invalid"));
"##,
        &[
            "oklch(62.8% 0.258 29.2) oklch(37.9% 0.138 265.5) oklch(56.9% 0 none) oklch(100% 0 none)",
            // The second is just outside sRGB (its red below 0), so it moves; the third is inside and comes back as it was.
            "oklch(62.8% 0.258 29.2) oklch(50.7% 0.086 201.2 / 0.5) oklch(70% 0.2 140)",
            "oklch(86.6% 0.295 142.5) null",
            "oklch(86.6% 0.295 142.5) true",
        ],
    );
}

/// `min`, `max` and `bands` from state: the arc, the ends, the ticks' numbers, the bands
/// and the value's tone and name follow them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gauge_range_and_bands_follow_state() {
    check(
        "gauge-live",
        r##"
state:
  v: 50
  lo: 0
  hi: 100
  bands: [ { to: 60, tone: success, label: OK }, { to: 80, tone: warning, label: High }, { tone: danger, label: Too high } ]
root:
  type: stack
  children:
    - { type: ui-gauge, attrs: { id: g }, props: { value: "{{ v }}", min: "{{ lo }}", max: "{{ hi }}", bands: "{{ bands }}", ticks: 4, tickLabels: true, unit: "°C" } }
    - { type: ui-gauge, attrs: { id: h }, props: { value: 50, max: "{{ hi }}", bands: [ { to: 50, tone: success }, { tone: danger } ] } }
"##,
        r##"
        const g = $("#g"), p = () => getComputedStyle(g).getPropertyValue("--p").trim();
        const state = () => [p(), g.dataset.tone, g.getAttribute("aria-valuemin"), g.getAttribute("aria-valuemax"), g.getAttribute("aria-valuetext"), $$("#g [data-gauge-end]").map((e) => e.textContent).join("/"), $$("#g [data-gauge-tick]").map((e) => e.textContent).join("/"), $$("#g [data-band]").map((b) => b.getAttribute("stroke-dasharray")).join("|")];
        log(...state());
        root.__irender.set("hi", 200); await wait(30); log(...state());
        root.__irender.set("bands", [{ to: 40, tone: "danger", label: "Low" }, { tone: "success" }]); await wait(30); log(...state());
        root.__irender.set("v", 20); await wait(30); log(...state());
        // A band past `max` is left out.
        root.__irender.set("bands", [{ to: 500, tone: "info" }]); await wait(30); log($$("#g [data-band]").length, g.dataset.tone);
        // Fixed bands on a range from state: placed on it as it changes.
        const h = $("#h"), hb = () => $$("#h [data-band]").map((b) => b.getAttribute("stroke-dasharray")).join("|");
        log(hb(), h.dataset.tone); root.__irender.set("hi", 400); await wait(30); log(hb(), h.dataset.tone);
"##,
        &[
            "50 success 0 100 50 °C, OK 0/100 25/50/75 0 0 60 200|0 60 20 200|0 80 20 200",
            "25 success 0 200 50 °C, OK 0/200 50/100/150 0 0 30 200|0 30 10 200|0 40 60 200",
            "25 success 0 200 50 °C 0/200 50/100/150 0 0 20 200|0 20 80 200",
            "10 danger 0 200 20 °C, Low 0/200 50/100/150 0 0 20 200|0 20 80 200",
            "0 primary",
            // The first band ends at 50: a quarter of 200, then an eighth of 400; the value 50 stays in it.
            "0 0 25 200|0 25 75 200 success",
            "0 0 12.5 200|0 12.5 87.5 200 success",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn event_calendar_marks_repeat_and_badges_click() {
    check(
        "calendar-marks",
        r##"
state:
  picked: null
  hit: null
  days:
    - { date: "2026-11-01", label: Closed, tone: neutral, repeat: { every: week, on: [0] } }
    - { date: "2026-11-09", end: "2026-11-10", repeat: { every: month, count: 2 }, items: [ { text: Audit, id: audit, tone: warning }, Stock ] }
root:
  type: ui-event-calendar
  props: { events: [], days: "{{ days }}", date: "2026-11-15", bindDay: picked, bindBadge: hit }
"##,
        r##"
        const cell = (d) => $(`[data-ev-day="${d}"]`);
        // Every Sunday is closed; the audit runs two days, this month and next.
        log(["2026-11-01", "2026-11-08", "2026-11-22", "2026-11-29"].map((d) => cell(d)?.querySelector("[data-ev-note]")?.textContent ?? "-").join(","));
        log(["2026-11-09", "2026-11-10", "2026-11-11"].map((d) => cell(d)?.querySelectorAll("[data-ev-badge]").length ?? "-").join(","));
        // The badge with an id is a button: its click is its own, not the day's.
        const b = cell("2026-11-10").querySelector("button[data-ev-badge]"); b.click(); await wait(30);
        log(S().hit.item.id, S().hit.date, S().picked, cell("2026-11-10").querySelector("span[data-ev-badge]").textContent);
        // By the keys: the focused day's badge buttons join the tab order, the others' don't.
        const tabs = () => ["2026-11-09", "2026-11-10"].map((d) => cell(d).querySelector("button[data-ev-badge]").tabIndex).join(",");
        cell("2026-11-10").focus(); cell("2026-11-10").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })); await wait(30);
        log("tabs", tabs(), document.activeElement.dataset.evDay);
        $("[data-ev-next]").click(); await wait(30); log(cell("2026-12-09")?.querySelectorAll("[data-ev-badge]").length ?? "-", cell("2026-12-06")?.querySelector("[data-ev-note]")?.textContent ?? "-");
        $("[data-ev-next]").click(); await wait(30); log(cell("2027-01-09")?.querySelectorAll("[data-ev-badge]").length ?? 0);
"##,
        &["Closed,Closed,Closed,Closed", "2,2,0", "audit 2026-11-10 null Stock", "tabs 0,-1 2026-11-09", "2 Closed", "0"],
    );
}

/// `ui-file-browser` with `url` and `uploadUrl`, against a fetch (and an XMLHttpRequest)
/// answered in the page: folders loaded once, again with Refresh or `refresh`, a page at
/// a time; files dropped from the desktop going up with how far they've got, and one that
/// failed sent again.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn file_browser_loads_folders_and_takes_dropped_files() {
    let Some(result) = run_with(
        "file-browser-server",
        r##"
state: { files: [], at: null, again: 0 }
root: { type: ui-file-browser, props: { bind: files, bindFolder: at, url: /api/files, uploadUrl: /api/upload, view: list, refresh: "{{ again }}" } }
"##,
        r##"
        const names = () => $$("[data-fm-item]").map((i) => i.querySelector("[data-fm-name]").textContent + (i.dataset.fmUp ? "(" + i.dataset.fmUp + ")" : "")).join(",");
        await until(() => $$("[data-fm-item]").length > 0); log(window.asked.join(" "), names());
        // A folder is fetched the first time it's opened, not again.
        root.__irender.set("at", "docs"); await until(() => names().includes("plan.pdf")); log(window.asked.join(" "), names());
        root.__irender.set("at", null); await wait(30); root.__irender.set("at", "docs"); await wait(30); log(window.asked.length);
        // One that fails says so, with Try again.
        window.fail = true; root.__irender.set("at", "old"); await until(() => $("[data-fm-retry]")); log($("[role=alert]").textContent);
        window.fail = false; $("[data-fm-retry]").click(); await until(() => names().includes("2019.zip")); log(names());
        // Files from the desktop, dropped on a folder: up they go, and the answer takes their place.
        root.__irender.set("at", null); await wait(30);
        const dt = new DataTransfer(); dt.items.add(new File(["hello"], "notes.txt", { type: "text/plain" })); dt.items.add(new File(["x"], "bad.txt"));
        const target = $('[data-fm-item="docs"]');
        carry("dragover", target, "dataTransfer", dt); log(target.hasAttribute("data-drop"));
        carry("drop", target, "dataTransfer", dt); await wait(0);
        root.__irender.set("at", "docs"); await wait(0); log(names());
        window.progress(0.5); await wait(10);
        log("half", $$("[data-fm-progress]").map((p) => p.value).join(","), $('[data-fm-item][data-fm-up=going] [data-fm-size]').textContent);
        window.release(); await until(() => !names().includes("going")); log(names(), JSON.stringify(window.sent), !!$("[data-fm-retry-upload]"));
        // Sent again, it goes up this time.
        window.bad = false; $("[data-fm-retry-upload]").click(); await wait(0); log(names()); window.release(); await until(() => !names().includes("going")); log(names(), window.sent.length);
        // Refreshed: the folder's answer as it is now (plan.pdf gone, new.doc there).
        window.docs = [{ id: "n", name: "new.doc" }];
        $("[data-fm-refresh]").click(); await until(() => names().includes("new.doc")); log("refreshed", names());
        window.docs = [{ id: "n", name: "new.doc" }, { id: "m", name: "more.doc" }];
        root.__irender.set("again", 1); await until(() => names().includes("more.doc")); log("refresh", names());
        // A big folder a page at a time.
        root.__irender.set("at", "big"); await until(() => $("[data-fm-load-more]")); log("page", $$("[data-fm-item]").length);
        $("[data-fm-load-more]").click(); await until(() => $$("[data-fm-item]").length === 4); log("more", $$("[data-fm-item]").length, !!$("[data-fm-load-more]"), window.asked.slice(-2).join(" "));
"##,
        r##"<script>
        window.asked = []; window.sent = []; window.bad = true; let held = [], going = [];
        window.docs = [{ id: "p", name: "plan.pdf", size: 20 }];
        window.release = () => { held.forEach((f) => f()); held = []; going = []; };
        window.progress = (p) => going.forEach((x) => x.upload.onprogress?.({ lengthComputable: true, loaded: p * 100, total: 100 }));
        window.XMLHttpRequest = class {
          constructor() { this.upload = {}; this.status = 0; this.responseText = ""; }
          open(m, u) { this.url = String(u); }
          setRequestHeader() {}
          send(body) {
            const name = body.get("file").name, folder = body.get("folder");
            window.sent.push(name + "→" + folder);
            going.push(this);
            held.push(() => {
              if (name === "bad.txt" && window.bad) { this.status = 500; return this.onload(); }
              this.status = 200; this.responseText = JSON.stringify({ id: "srv-" + name, name, size: 5 }); this.onload();
            });
          }
        };
        window.fetch = (u, o = {}) => {
          const url = new URL(u, location.href), ok = (v) => Promise.resolve({ ok: true, json: () => Promise.resolve(v) });
          const f = url.searchParams.get("folder"), after = url.searchParams.get("after"); window.asked.push((f ?? "top") + (after ? "@" + after : ""));
          if (window.fail) return Promise.resolve({ ok: false, status: 503 });
          if (f === "big") return ok(after ? { items: [{ id: "b3", name: "3.txt" }, { id: "b4", name: "4.txt" }] } : { items: [{ id: "b1", name: "1.txt" }, { id: "b2", name: "2.txt" }], next: "c2" });
          return ok(f === null ? [{ id: "docs", name: "Docs", folder: true }, { id: "old", name: "Old", folder: true }, { id: "big", name: "Big", folder: true }, { id: "a", name: "a.png", size: 10 }] : f === "docs" ? window.docs : [{ id: "z", name: "2019.zip" }]);
        };
        </script>"##,
    ) else {
        return;
    };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "top Big,Docs,Old,a.png",
            "top docs plan.pdf",
            "2",
            "Couldn't load this folder Try again",
            "2019.zip",
            "true",
            "bad.txt(going),notes.txt(going),plan.pdf",
            "half 50,50 Uploading… 50%",
            r#"bad.txt(failed),notes.txt,plan.pdf ["notes.txt→docs","bad.txt→docs"] true"#,
            "bad.txt(going),notes.txt,plan.pdf",
            "bad.txt,notes.txt,plan.pdf 3",
            // The folder as the server has it now: what this test's server didn't keep goes.
            "refreshed new.doc",
            "refresh more.doc,new.doc",
            "page 2",
            "more 4 false big big@c2",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn timeline_filters_and_loads_more() {
    check(
        "timeline-filters",
        r##"
state:
  f: null
  seen: null
  busy: false
  older: true
  feed:
    - { id: 1, who: Ada, kind: comment, action: commented, when: "2026-11-27T10:00" }
    - { id: 2, who: Bob, kind: status, action: changed the status, when: "2026-11-27T09:00" }
    - { id: 3, who: Ada, kind: status, action: changed the status, when: "2026-11-26T09:00" }
    - { id: 4, who: Ada, kind: comment, action: commented, when: "2026-11-25T09:00" }
root:
  type: ui-timeline
  props: { items: "{{ feed }}", filters: true, bindFilter: f, limit: 2, hasMore: "{{ older }}", loading: "{{ busy }}", bindSeen: seen }
  on:
    more:
      - { action: set_state, key: busy, value: true }
"##,
        r##"
        const ids = () => $$("[data-tl-item]").filter((li) => li.checkVisibility()).map((li) => li.getAttribute("data-who") + ":" + li.getAttribute("data-kind")).join(" ");
        const btn = () => $("[data-tl-show] button"), shownBtn = () => ($("[data-tl-show]").hidden ? "-" : btn().textContent);
        log($$("[data-tl-filters] select").map((s) => [...s.options].map((o) => o.textContent).join("/")).join(" | "));
        log(ids(), shownBtn());
        // Ada's entries only: the limit counts those.
        const who = $("[data-tl-who]"); who.value = "Ada"; who.dispatchEvent(new Event("change")); await wait(30);
        log(JSON.stringify(S().f), ids(), shownBtn());
        // State picks the kind too.
        root.__irender.set("f", { who: "Ada", kind: "status" }); await wait(30); log(ids(), $("[data-tl-kind]").value);
        root.__irender.set("f", { who: "Bob", kind: "comment" }); await wait(30); log($("[data-tl-none]")?.textContent ?? "-");
        // Several at once, from state: any of them; the menu says how many.
        let told = []; root.addEventListener("filter", (e) => told.push(JSON.stringify(e.detail)));
        root.__irender.set("f", { who: ["Ada", "Bob"], kind: "status" }); await wait(30); log(ids(), $("[data-tl-who]").selectedOptions[0].textContent);
        who.value = "Bob"; $("[data-tl-who]").value = "Bob"; $("[data-tl-who]").dispatchEvent(new Event("change")); await wait(30); log(JSON.stringify(S().f), told.join(" "));
        root.__irender.set("f", null); await wait(30);
        // Show the rest, then Load more fires `more` and waits while loading.
        btn().click(); await wait(30); log(ids(), shownBtn());
        let asked = null; root.addEventListener("more", (e) => (asked = e.detail.filter));
        btn().click(); await wait(30); log(S().busy, shownBtn(), btn().disabled, JSON.stringify(asked));
        root.__irender.set("feed", [...S().feed, { id: 5, who: "Cy", kind: "file", action: "added a file", when: "2026-11-24T09:00" }]); root.__irender.set("busy", false); root.__irender.set("older", false); await wait(30);
        log(ids(), shownBtn(), document.activeElement.getAttribute("data-who"));
        // Leaving the page: when it was last seen.
        window.dispatchEvent(new Event("pagehide")); await wait(10); log(typeof S().seen, Date.parse(S().seen) > Date.parse("2026-11-27T10:00"));
"##,
        &[
            "Everyone/Ada/Bob | Every kind/comment/status",
            "Ada:comment Bob:status Show 2 more",
            r#"{"who":"Ada","kind":null} Ada:comment Ada:status Show 1 more"#,
            "Ada:status status",
            "Nothing matches these filters",
            "Bob:status Ada:status 2 picked",
            r#"{"who":"Bob","kind":"status"} {"who":"Bob","kind":"status"}"#,
            "Ada:comment Bob:status Ada:status Ada:comment Load more",
            r#"true Loading… true {"who":null,"kind":null}"#,
            "Ada:comment Bob:status Ada:status Ada:comment Cy:file - Cy",
            "string true",
        ],
    );
}

/// `ui-approval`: a stage's due date from a clock fixed at 3 December 2026, handing a turn
/// to someone else, and the decisions kept as an earlier round when it's sent again.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn approval_hands_off_and_keeps_rounds() {
    let Some(result) = run_with(
        "approval-rounds",
        r##"
state:
  me: ravi
  st: null
  stages:
    - { id: mgr, title: Manager, due: "2026-12-01", approvers: [ { id: ravi, name: Ravi Kumar } ] }
    - { id: fin, title: Finance, due: "2026-12-10", approvers: [ { id: asha, name: Asha Rao } ] }
root:
  type: ui-approval
  props: { bind: stages, me: "{{ me }}", owner: ravi, bindStatus: st, people: [ { id: ravi, name: Ravi Kumar }, { id: mei, name: Mei Lin } ] }
"##,
        r##"
        const due = () => $$("[data-ap-due]").map((d) => d.textContent + (d.hasAttribute("data-overdue") ? "!" : "")).join(" | ");
        log(due(), $$("[data-ap-handoff] option").map((o) => o.textContent).join(","));
        // Ravi hands his turn to Mei, who then decides for him.
        $("[data-ap-handoff] select").value = "mei"; $("[data-ap-act=handoff]").click(); await wait(30);
        log(JSON.stringify(S().stages[0].approvers), $("[data-ap-for]").textContent);
        root.__irender.set("me", "mei"); await wait(30);
        $("[data-ap-decide] textarea").value = "Too much"; $("[data-ap-decide] textarea").dispatchEvent(new Event("input")); $("[data-ap-act=rejected]").click(); await wait(30);
        log(S().st);
        // Ravi sends it again: Mei's no is kept as an earlier round.
        root.__irender.set("me", "ravi"); await wait(30); $("[data-ap-act=resend]").click(); await wait(30);
        log(S().st, JSON.stringify(S().stages[0].rounds.map((r) => r.approvers.map((p) => p.decision + ":" + p.comment))), $("[data-ap-rounds] summary").textContent, $("[data-ap-rounds] li").textContent.split(",")[0]);
"##,
        r#"<script>{ const at = new Date(2026, 11, 3, 10).getTime(), start = Date.now(), real = Date; window.Date = class extends real { constructor(...a) { super(...(a.length ? a : [at + (real.now() - start)])); } static now() { return at + (real.now() - start); } }; }</script>"#,
    ) else {
        return;
    };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "Overdue by 2 days! | Due Dec 10, 2026 Pick someone,Mei Lin",
            r#"[{"id":"mei","name":"Mei Lin","for":{"id":"ravi","name":"Ravi Kumar"}}] for Ravi Kumar"#,
            "rejected",
            r#"pending [["rejected:Too much"]] 1 earlier round Mei Lin: Rejected (“Too much”)"#,
        ],
        "\n{result}"
    );
}

/// `ui-approval` with stages side by side: Legal and Security together after the manager,
/// each decided on its own, the group done when both are; a turn handed on, taken back.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn approval_runs_stages_side_by_side() {
    check(
        "approval-parallel",
        r##"
state:
  me: asha
  st: null
  stages:
    - { id: mgr, title: Manager, approvers: [ { id: ravi, name: Ravi Kumar, decision: approved } ] }
    - { id: legal, title: Legal, approvers: [ { id: asha, name: Asha Rao } ] }
    - { id: sec, title: Security, parallel: true, approvers: [ { id: mei, name: Mei Lin } ] }
    - { id: fin, title: Finance, approvers: [ { id: cy, name: Cy Park } ] }
root:
  type: ui-approval
  props: { bind: stages, me: "{{ me }}", bindStatus: st, people: [ { id: bo, name: Bo Chen } ] }
"##,
        r##"
        const states = () => $$("[data-ap-stage]").map((li) => li.dataset.apStage + ":" + li.dataset.state + (li.hasAttribute("data-parallel") ? "∥" : "")).join(" ");
        const head = () => $("[data-ap-summary] b").textContent;
        log(states(), "|", head(), "|", $("[data-ap-summary] small").textContent, "|", $("[data-ap-decide] label").textContent);
        // Asha approves Legal: Security is still open, so Finance waits.
        $("[data-ap-act=approved]").click(); await wait(30);
        log(states(), "|", head());
        // Mei hands Security to Bo, then takes it back.
        root.__irender.set("me", "mei"); await wait(30);
        $("[data-ap-handoff] select").value = "bo"; $("[data-ap-act=handoff]").click(); await wait(30);
        log(S().stages[2].approvers[0].id, $("[data-ap-act=takeback]").textContent);
        $("[data-ap-act=takeback]").click(); await wait(30);
        log(JSON.stringify(S().stages[2].approvers), !!$("[data-ap-decide]"));
        $("[data-ap-act=approved]").click(); await wait(30);
        log(states(), "|", head(), S().st);
"##,
        &[
            "mgr:approved legal:current sec:current∥ fin:waiting | Waiting for Legal and Security | 0 of 2 side by side signed off · stages 2–3 of 4 | Your decision on Legal, Asha Rao",
            "mgr:approved legal:approved sec:current∥ fin:waiting | Waiting for Security",
            "bo Take back your turn from Bo Chen",
            r#"[{"id":"mei","name":"Mei Lin"}] true"#,
            "mgr:approved legal:approved sec:approved∥ fin:current | Waiting for Finance pending",
        ],
    );
}

/// `ui-sortable-list` with `childrenField`: items inside items, moved in and out and along
/// by the keys and by dragging (as far in as the pointer is to the right), and removed
/// with what's inside them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sortable_list_nests_items() {
    check(
        "sortable-nested",
        r##"
state:
  plan:
    - { id: a, label: Alpha, children: [ { id: a1, label: One }, { id: a2, label: Two } ] }
    - { id: b, label: Bravo }
    - { id: c, label: Charlie }
root: { type: ui-sortable-list, attrs: { id: t }, props: { bind: plan, childrenField: children, maxDepth: 2, numbered: true, removable: true }, style: { width: 320px } }
"##,
        r##"
        const shape = (l = S().plan) => l.map((v) => v.id + (v.children?.length ? "(" + shape(v.children) + ")" : "")).join(",");
        const grip = (k) => $(`[data-sl-item="${k}"] > [data-sl-row] > [data-sl-grip]`);
        const press = (k, key, o = {}) => grip(k).dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...o }));
        log(shape(), $$("[data-sl-place]").map((p) => p.textContent).join(","), $(`[data-sl-item="a2"]`).dataset.slDepth);
        // Bravo, picked up: → into Alpha (last), ↑ before Two, ← out after Alpha.
        grip("b").focus(); press("b", " "); press("b", "ArrowRight"); await wait(20);
        log("in", shape(), $("[data-sl-live]").textContent, document.activeElement === grip("b"), !!$('[data-sl-item="b"][data-sl-picked]'));
        press("b", "ArrowUp"); await wait(20); log("up", shape());
        // First in Alpha now, with nothing before it to go into.
        press("b", "ArrowUp"); await wait(20); press("b", "ArrowRight"); await wait(20); log("too deep", shape());
        press("b", "ArrowLeft"); await wait(20); log("out", shape(), $("[data-sl-live]").textContent);
        press("b", "Escape"); await wait(20); log("put back", shape());
        // Alt and the arrows, straight away: Charlie into Bravo.
        grip("c").focus(); press("c", "ArrowRight", { altKey: true }); await wait(20); log("alt", shape());
        // Dragged: One down below Charlie and to the left, out to the top level, last.
        const g = grip("a1"), from = g.getBoundingClientRect(), to = $('[data-sl-item="c"] > [data-sl-row]').getBoundingClientRect();
        const p = (type, x, y) => g.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 4, pointerType: "mouse", button: 0, clientX: x, clientY: y }));
        p("pointerdown", from.left + 4, from.top + 4); p("pointermove", from.left + 10, from.top + 20);
        p("pointermove", from.left - 60, to.bottom - 2); await wait(10);
        log("line", $$("[data-sl-drop]").map((r) => r.parentElement.dataset.slItem + ":" + r.dataset.slDrop + ":" + r.style.getPropertyValue("--sl-indent")).join(","));
        p("pointerup", from.left - 60, to.bottom - 2); await wait(20);
        log("dragged out", shape());
        // Dragged right over Alpha's last child: in, as far as the row above allows.
        const g2 = grip("a1"), f2 = g2.getBoundingClientRect(), two = $('[data-sl-item="a2"] > [data-sl-row]').getBoundingClientRect();
        const q = (type, x, y) => g2.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 5, pointerType: "mouse", button: 0, clientX: x, clientY: y }));
        q("pointerdown", f2.left + 4, f2.top + 4); q("pointermove", f2.left + 10, f2.top - 10);
        q("pointermove", f2.left + 30, two.bottom - 2); await wait(10); q("pointerup", f2.left + 30, two.bottom - 2); await wait(20);
        log("dragged in", shape());
        // Removing Alpha takes what's in it.
        $('[data-sl-item="a"] > [data-sl-row] > [data-sl-remove]').click(); await wait(20);
        log("removed", shape(), $("[data-sl-live]").textContent, document.activeElement.closest("[data-sl-item]")?.dataset.slItem);
"##,
        &[
            "a(a1,a2),b,c 1,1.1,1.2,2,3 1",
            "in a(a1,a2,b),c Bravo moved into Alpha, 3 of 3 true true",
            "up a(a1,b,a2),c",
            "too deep a(b,a1,a2),c",
            "out a(a1,a2),b,c Bravo moved to the top level, 2 of 3",
            "put back a(a1,a2),b,c",
            "alt a(a1,a2),b(c)",
            "line c:after:-24px",
            "dragged out a(a2),b(c),a1",
            "dragged in a(a2,a1),b(c)",
            "removed b(c) Alpha and 2 inside it removed b",
        ],
    );
}

/// `ui-form-builder`: a group of conditions inside the conditions (`(a) && ((b) || (c))`),
/// in the preview, in the inspector, written into the document and read back from it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_builder_nests_conditions() {
    check(
        "builder-nested",
        r##"
state:
  doc: null
  fields:
    - { id: kind, type: radio, label: Kind, options: [Home, Work] }
    - { id: urgent, type: checkbox, label: Urgent }
    - { id: size, type: select, label: Size, options: [S, M, L] }
    - { id: note, type: text, label: Note, required: true, when: { match: all, conditions: [ { field: kind, value: Work }, { match: any, conditions: [ { field: urgent, value: true }, { field: size, value: L } ] } ] } }
    - { id: after, type: text, label: After }
root: { type: ui-form-builder, attrs: { id: fb }, props: { bind: fields, bindDocument: doc } }
"##,
        r##"
        $("[data-fb-tab=preview]").click(); await wait(30);
        const shown = () => $$("[data-fb-preview] [data-fb-for]").map((f) => f.dataset.fbFor).join();
        const radio = (v) => $(`[data-fb-preview] input[value="${v}"]`);
        log(shown());
        radio("Work").click(); await wait(30); log("work", shown());
        $("[data-fb-for=urgent] input").click(); await wait(30); log("urgent", shown());
        $("[data-fb-for=urgent] input").click(); await wait(30);
        const size = $("[data-fb-for=size] select"); size.value = "L"; size.dispatchEvent(new Event("change", { bubbles: true })); await wait(30); log("large", shown());
        radio("Home").click(); await wait(30); log("home", shown());
        // The document: one `if` with the group in brackets of its own.
        const cond = (function find(n) { if (n?.type === "if" && n.children?.[0]?.type === "ui-field") return n.props.when; for (const c of n?.children ?? []) { const w = find(c); if (w) return w; } })(S().doc.root);
        log(cond, S().doc.root.children[1].children[0].props.rules.note.required);
        // The inspector: the group in a box of its own, its rows and how they join.
        $("[data-fb-tab=build]").click(); await wait(30);
        $$("[data-fb-pick]")[3].click(); await wait(30);
        log("inspector", $$("[data-fb-cond]").length, $$("[data-fb-group]").length, $$("[data-fb-group] [data-fb-cond]").length, $("[id$=-in-match]").value, $("[id$=-in-match1]").value);
        // Joined by all inside: Work, and both Urgent and L.
        const m1 = $("[id$=-in-match1]"); m1.value = "all"; m1.dispatchEvent(new Event("change", { bubbles: true })); await wait(30);
        log(JSON.stringify(S().fields[3].when));
        // A group added at the top, and one removed.
        // (The last "Add a group" is the top one's: the inner group has its own.)
        $$("[data-fb-add-group]").at(-1).click(); await wait(30);
        log("groups", $$("[data-fb-group]").length, S().fields[3].when.conditions.length);
        $$("[data-fb-ungroup]")[0].click(); await wait(30);
        log("ungrouped", JSON.stringify(S().fields[3].when.conditions.map((c) => c.conditions ? c.match + c.conditions.length : c.field)));
        // Read back from its document: the same tree.
        const before = JSON.stringify(S().fields[3].when);
        $("[data-fb-tab=document]").click(); await wait(30);
        const paste = $("[id$=-load]"); paste.value = JSON.stringify(S().doc); $("[data-fb-load]").click(); await wait(30);
        log("loaded", JSON.stringify(S().fields[3].when) === before ? "same" : JSON.stringify(S().fields[3].when));
        // `!( … )` round a group: none of them.
        const neg = JSON.parse(JSON.stringify(S().doc));
        (function find(n) { if (n?.type === "if" && n.children?.[0]?.type === "ui-field") return (n.props.when = '{{ (kind == "Work") && !((urgent) || (size == "L")) }}'); (n?.children ?? []).forEach(find); })(neg.root);
        $("[id$=-load]").value = JSON.stringify(neg); $("[data-fb-load]").click(); await wait(30);
        log("none", JSON.stringify(S().fields[3].when));
"##,
        &[
            "kind,urgent,size,after",
            "work kind,urgent,size,after",
            "urgent kind,urgent,size,note,after",
            "large kind,urgent,size,note,after",
            "home kind,urgent,size,after",
            r#"{{ (kind == "Work") && ((urgent) || (size == "L")) }} {{ (kind == "Work") && ((urgent) || (size == "L")) }}"#,
            "inspector 3 1 2 all any",
            r#"{"match":"all","conditions":[{"field":"kind","value":"Work"},{"match":"all","conditions":[{"field":"urgent","value":true},{"field":"size","value":"L"}]}]}"#,
            "groups 2 3",
            r#"ungrouped ["kind","any2"]"#,
            "loaded same",
            r#"none {"match":"all","conditions":[{"field":"kind","value":"Work"},{"match":"any","conditions":[{"field":"urgent","value":true},{"field":"size","value":"L"}],"not":true}]}"#,
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn combobox_options_show_pictures_and_badges() {
    check(
        "combobox-rich",
        r##"
state: { who: [], people: [ { id: ada, name: Ada Lovelace, photo: "data:image/gif;base64,R0lGODlhAQABAAAAACw=", role: Owner, n: 3 }, { id: bob, name: Bob Ray, role: null, n: 12 } ] }
root:
  type: ui-combobox
  props: { bind: who, multiple: true, options: "{{ people }}", valueField: id, labelField: name, imageField: photo, badgeField: n, descriptionField: role, label: People }
"##,
        r##"
        const input = $("input[role=combobox]"); input.focus(); input.click(); type(input, ""); await wait(30);
        const rows = $$("[role=option]").map((r) => [r.querySelector("img") ? "img" : "-", r.querySelector("[data-ui-option-label] > span:not([data-ui-badge])").textContent, r.querySelector("[data-ui-badge]")?.textContent ?? "-", r.querySelector("[data-ui-description]")?.textContent ?? "-"].join(" "));
        log(rows.join(" | "));
        // The picture is decoration: the option is still named by its label.
        log($("[role=option] img").alt === "", $$("[role=option]")[0].textContent.includes("Ada Lovelace"));
        // A chosen one's chip shows its picture too.
        $$("[role=option]")[0].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true })); await wait(30);
        log($$("[data-ui-chip]").map((c) => (c.querySelector("img") ? "img " : "") + c.textContent.replace(/[×✕]/g, "").trim()).join("|"));
"##,
        &["img Ada Lovelace 3 Owner | - Bob Ray 12 -", "true true", "img Ada Lovelace"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn date_picker_picks_a_week() {
    check(
        "week-picker",
        r##"
state: { wk: "2026-W48", span: null, first: "2026-W01" }
root:
  type: stack
  children:
    - { type: ui-calendar, props: { bind: wk, picker: week, bindRange: span, weekStart: 1 } }
    - { type: ui-date-picker, props: { bind: first, picker: week, locale: en-GB }, attrs: { id: dp } }
"##,
        r##"
        const day = (k) => $(`[data-i-x=ui-calendar] [data-key="${k}"]`), marks = () => $$("[data-i-x=ui-calendar] [aria-pressed=true]").map((b) => b.dataset.key.slice(8)).join(",");
        // Week 48 of 2026 runs from Monday 23 to Sunday 29 November.
        log(marks(), day("2026-11-23").hasAttribute("data-range-start"), day("2026-11-29").hasAttribute("data-range-end"));
        // A day picks its week, written as the ISO week, its days in `bindRange`.
        day("2026-11-04").click(); await wait(30); log(S().wk, JSON.stringify(S().span), marks());
        // The first week of a year can start in December: 31 Dec 2025 is in 2026-W01.
        log($("#dp [data-cal-trigger]").textContent.trim());
"##,
        &["23,24,25,26,27,28,29 true true", r#"2026-W45 {"from":"2026-11-02","to":"2026-11-08"} 02,03,04,05,06,07,08"#, "Week 1, 2026 (29 Dec – 4 Jan)"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_builder_shows_fields_when_and_checks_patterns() {
    let doc = r##"
state:
  doc: null
  fields:
    - { id: contact, type: radio, label: Contact by, options: [Email, Phone], required: true }
    - { id: email, type: email, label: Email, required: true, when: { field: contact, value: Email } }
    - { id: code, type: text, label: Code, pattern: "[A-Z]{2}[0-9]{2}", patternMessage: Two letters then two digits, maxLength: 4 }
root: { type: ui-form-builder, attrs: { id: fb }, props: { bind: fields, bindDocument: doc } }
"##;
    let Some(result) = run(
        "builder-when",
        doc,
        r##"
        $("[data-fb-tab=preview]").click(); await wait(30);
        const shown = () => $$("[data-fb-preview] [data-fb-for]").map((f) => f.dataset.fbFor).join(), errs = () => $$("[data-fb-error]").map((e) => e.textContent).join("|");
        const radio = (v) => $(`[data-fb-preview] input[value="${v}"]`), send = () => $("[data-fb-preview] [type=submit]").click();
        log(shown());
        radio("Email").click(); await wait(30); log(shown());
        type($("[data-fb-preview] input[type=text]"), "ab12"); send(); await wait(30); log(errs());
        radio("Phone").click(); await wait(30); type($("[data-fb-preview] input[type=text]"), "AB12"); send(); await wait(30);
        log($("[data-fb-preview] pre").textContent.replace(/\s+/g, " "));
        // The inspector: Email's condition, and renaming Contact's key carries it along.
        $("[data-fb-tab=build]").click(); await wait(30);
        $$("[data-fb-pick]")[1].click(); await wait(30); log($("[id$=-in-when0]").value, $("[id$=-in-is0]").value);
        $$("[data-fb-pick]")[0].click(); await wait(30); const key = $("[id$=-in-key]"); key.value = "via"; key.dispatchEvent(new Event("change", { bubbles: true })); await wait(30);
        log(JSON.stringify(S().fields[1].when));
        log("DOC " + JSON.stringify(S().doc));
"##,
    ) else {
        return;
    };
    let mut lines: Vec<&str> = result.lines().collect();
    let exported = lines.pop().and_then(|l| l.strip_prefix("DOC ")).unwrap_or_else(|| panic!("no document:\n{result}")).to_string();
    assert_eq!(
        lines,
        [
            "contact,code",
            "contact,email,code",
            "Email is required.|Two letters then two digits",
            r#"{ "contact": "Phone", "code": "AB12" }"#,
            "contact Email",
            r#"{"field":"via","value":"Email"}"#,
        ],
        "\n{result}"
    );
    // The document it gives out renders, the email field in an `if`, required while shown.
    let json: serde_json::Value = serde_json::from_str(&exported).unwrap();
    let text = json.to_string();
    assert!(text.contains(r#""when":"{{ via == \"Email\" }}""#) && text.contains(r#""required":"{{ via == \"Email\" }}""#), "{text}");
    assert!(text.contains(r#""pattern":"[A-Z]{2}[0-9]{2}""#) && text.contains(r#""maxLength":4"#), "{text}");
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let parsed = irender_core::parse(&text, Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&parsed).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &serde_json::json!({})).unwrap();
}

/// `ui-form-builder`'s phone, time, rating and file fields, widths, conditions joined by
/// any or all (and "is not"), undo and redo, and its document loaded back into fields.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_builder_combines_conditions_and_reads_its_document_back() {
    let doc = r##"
state:
  doc: null
  fields:
    - { id: contact, type: radio, label: Contact by, options: [Email, Phone, Post] }
    - { id: news, type: checkbox, label: Send news }
    - { id: email, type: email, label: Email, required: true, when: { match: any, conditions: [ { field: contact, value: Email }, { field: news, value: true } ] } }
    - { id: phone, type: phone, label: Phone, width: half, when: { conditions: [ { field: contact, value: Email, not: true }, { field: contact, value: Post, not: true } ] } }
    - { id: best, type: time, label: Best time, width: half, min: "09:00", max: "17:00" }
    - { id: stars, type: rating, label: How was it, max: 4 }
    - { id: proof, type: file, label: Proof, accept: ".pdf", multiple: false }
root: { type: ui-form-builder, attrs: { id: fb }, props: { bind: fields, bindDocument: doc, uploadUrl: /api/files } }
"##;
    let Some(result) = run(
        "builder-more",
        doc,
        r##"
        $("[data-fb-tab=preview]").click(); await wait(30);
        const shown = () => $$("[data-fb-preview] [data-fb-for]").map((f) => f.dataset.fbFor + (f.dataset.fbWidth ? "/" + f.dataset.fbWidth : "")).join();
        const radio = (v) => $(`[data-fb-preview] input[value="${v}"]`);
        log(shown(), $$("[data-fb-stars] input").length, $("[data-fb-for=proof] input").type, $("[data-fb-for=proof] input").multiple, $("[data-fb-for=best] input").type);
        radio("Phone").click(); await wait(30); log("phone", shown());
        radio("Email").click(); await wait(30); log("email", shown());
        radio("Post").click(); await wait(30); log("post", shown());
        $("[data-fb-for=news] input").click(); await wait(30); log("news", shown());
        // Undo and redo, from the build tab.
        $("[data-fb-tab=build]").click(); await wait(30);
        $$("[data-fb-pick]")[5].click(); await wait(30);
        const label = $("[id$=-in-label]"); label.value = "Stars"; label.dispatchEvent(new Event("input", { bubbles: true })); await wait(30);
        log("renamed", S().fields[5].label, S().fields[5].id);
        $("[data-fb-undo]").click(); await wait(30); log("undone", S().fields[5].label, S().fields[5].id);
        $("[data-fb-redo]").click(); await wait(30); log("redone", S().fields[5].label);
        $("[data-fb-undo]").click(); await wait(30);
        // The conditions in the inspector: two rows, joined by any.
        $$("[data-fb-pick]")[2].click(); await wait(30);
        log("conds", $$("[data-fb-cond]").length, $("[id$=-in-match]").value);
        // The document, back into fields.
        // Compared with their keys in order, and `match: all` (the default) said.
        const norm = (list) => JSON.stringify(list.map((f) => Object.fromEntries(Object.entries({ ...f, ...(f.when?.conditions ? { when: { match: f.when.match ?? "all", conditions: f.when.conditions } } : {}) }).sort(([a], [b]) => (a < b ? -1 : 1)))));
        const before = norm(S().fields);
        $("[data-fb-tab=document]").click(); await wait(30);
        const paste = $("[id$=-load]"); paste.value = JSON.stringify(S().doc); $("[data-fb-load]").click(); await wait(30);
        log("loaded", norm(S().fields) === before ? "same" : norm(S().fields));
        // Drawn again after loading: the box is a new one.
        $("[id$=-load]").value = '{"version":1,"root":{"type":"ui-field","children":[{"type":"ui-map"}]}}'; $("[data-fb-load]").click(); await wait(30);
        log("bad", $("[data-fb-load-box] [data-fb-error]").textContent);
        log("DOC " + JSON.stringify(S().doc));
"##,
    ) else {
        return;
    };
    let mut lines: Vec<&str> = result.lines().collect();
    let exported = lines.pop().and_then(|l| l.strip_prefix("DOC ")).unwrap_or_else(|| panic!("no document:\n{result}")).to_string();
    assert_eq!(
        lines,
        [
            // Nothing picked: phone shows (it isn't Email, it isn't Post).
            "contact,news,phone/half,best/half,stars,proof 4 file false time",
            "phone contact,news,phone/half,best/half,stars,proof",
            "email contact,news,email,best/half,stars,proof",
            "post contact,news,best/half,stars,proof",
            "news contact,news,email,best/half,stars,proof",
            "renamed Stars stars",
            "undone How was it stars",
            "redone Stars",
            "conds 2 any",
            "loaded same",
            "bad That isn't a form this builder made: a field it doesn't know: ui-map",
        ],
        "\n{result}"
    );
    let json: serde_json::Value = serde_json::from_str(&exported).unwrap();
    let text = json.to_string();
    assert!(text.contains(r#""when":"{{ (contact == \"Email\") || (news) }}""#), "{text}");
    assert!(text.contains(r#""when":"{{ (!(contact == \"Email\")) && (!(contact == \"Post\")) }}""#), "{text}");
    assert!(text.contains(r#""type":"ui-rating","props":{"bind":"stars","max":4}"#) && text.contains(r#""url":"/api/files""#) && text.contains(r#""type":"ui-phone-input""#), "{text}");
    assert!(text.contains(r#""grid-template-columns":"repeat(auto-fit, minmax(240px, 1fr))""#), "{text}");
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let parsed = irender_core::parse(&text, Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&parsed).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &serde_json::json!({})).unwrap();
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_cells_fill_by_their_corner() {
    check(
        "table-fill",
        r##"
state:
  rows:
    - { id: 1, size: S, qty: 1 }
    - { id: 2, size: M, qty: 2 }
    - { id: 3, size: L, qty: 3 }
    - { id: 4, size: XL, qty: 4 }
    - { id: 5, size: XXL, qty: 5 }
    - { id: 6, size: "", qty: 6 }
root:
  type: ui-data-table
  props: { rows: "{{ rows }}", columnsMenu: false }
  children:
    - { type: ui-data-table-column, props: { field: id } }
    - { type: ui-data-table-column, props: { field: size, edit: text } }
    - { type: ui-data-table-column, props: { field: qty, edit: number } }
"##,
        r##"
        const cell = (r, c) => $$("[data-dt-body] > tr")[r].querySelector(`[data-dt-col=${c}]`), h = () => $("[data-dt-fill]");
        const sizes = () => S().rows.map((r) => r.size || "-").join(",");
        const drag = async (toRow) => {
          const b = h().getBoundingClientRect(), x = b.left + 4, y = cell(toRow, "size").getBoundingClientRect(), to = y.top + y.height / 2;
          pointer("pointerdown", x, b.top + 4, h()); pointer("pointermove", x, to, h()); await wait(20);
          const marked = $$("td[data-dt-fill-to]").length; pointer("pointerup", x, to, h()); await wait(30); return marked;
        };
        // No handle until a cell has the focus; then it's on the cell's corner.
        // It may settle a frame later, once the columns have.
        const onCorner = () => Math.abs(h().getBoundingClientRect().left + 5 - cell(0, "size").getBoundingClientRect().right) < 1;
        log(h().hidden); cell(0, "size").click(); await until(onCorner).catch(() => {});
        const c0 = cell(0, "size").getBoundingClientRect(), hb = h().getBoundingClientRect();
        log(h().hidden, Math.round(hb.left + 5 - c0.right), Math.round(hb.top + 5 - c0.bottom));
        // The columns widen after it's placed (as when fonts load late): it follows the corner.
        cell(0, "size").style.paddingInlineEnd = "120px"; await until(onCorner).catch(() => {});
        const c1 = cell(0, "size").getBoundingClientRect(), hb1 = h().getBoundingClientRect();
        log(c1.right > c0.right, Math.round(hb1.left + 5 - c1.right), Math.round(hb1.top + 5 - c1.bottom)); cell(0, "size").style.paddingInlineEnd = ""; await wait(30);
        // Two picked, dragged down three rows: they repeat; then the whole lot is picked.
        key("ArrowDown", { shiftKey: true }); await wait(20);
        log(await drag(4), sizes(), $$("td[data-dt-in]").length);
        key("z", { ctrlKey: true }); log("undo", sizes());
        // Up from the last of the picked rows.
        cell(4, "size").click(); key("ArrowDown", { shiftKey: true }); await wait(20);
        log(await drag(2), sizes());
"##,
        &["true", "false 0 0", "true 0 0", "3 S,M,S,M,S,- 5", "undo S,M,L,XL,XXL,-", "2 S,M,XXL,-,XXL,-"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn repeater_rows_pick_from_a_combobox() {
    check(
        "repeater-combobox",
        r##"
state: { trips: [{ city: BER }, { city: null }], wrong: [] }
root:
  type: ui-repeater
  props:
    bind: trips
    bindErrors: wrong
    fields: [ { field: city, label: City, type: combobox, options: [ { value: BER, label: Berlin }, { value: LIS, label: Lisbon }, { value: OSL, label: Oslo } ] } ]
"##,
        r##"
        const c = (i) => $$("[data-field=city]")[i], list = $(`#${c(0).getAttribute("list")}`);
        log(c(0).value, c(1).value, c(0).getAttribute("list") === c(1).getAttribute("list"), [...list.options].map((o) => o.value).join());
        // A label typed is its value; so is the value itself.
        type(c(1), "lisbon"); log(JSON.stringify(S().trips.map((t) => t.city)));
        type(c(1), "OSL"); c(1).blur(); await wait(30); log(S().trips[1].city, c(1).value);
        // Anything else stays as typed, and is said not to be one of them once it's left.
        type(c(0), "Paris"); c(0).blur(); await wait(30); log(S().trips[0].city, S().wrong.map((w) => w.message).join(), $$("[data-rep-error]")[0].textContent);
"##,
        &["Berlin  true Berlin,Lisbon,Oslo", r#"["BER","LIS"]"#, "OSL Oslo", "Paris Not one of the choices Not one of the choices"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_section_open_state_is_in_state() {
    check(
        "form-section-bind",
        r##"
state: { more: false, code: "", errors: {} }
root:
  type: ui-form
  props: { bindErrors: errors, rules: { code: { label: Code, required: true } } }
  children:
    - { type: ui-error-summary }
    - type: ui-form-section
      props: { title: More, collapsible: true, bindOpen: more }
      children:
        - { type: ui-field, props: { label: Code }, children: [ { type: ui-input, props: { bind: code } } ] }
    - { type: ui-button, props: { label: Go, type: submit }, attrs: { id: go } }
"##,
        r##"
        const d = $("details[data-section-in]"), sum = $("summary");
        log(d.open, S().more);
        sum.click(); await wait(30); log(d.open, S().more);
        root.__irender.set("more", false); await wait(30); log(d.open);
        // A stopped submit opens it for its wrong field: the key follows.
        $("#go").click(); await wait(30); log(d.open, S().more);
"##,
        &["false false", "true true", "false", "true true"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_checks_the_rows_of_its_repeaters() {
    check(
        "form-rows",
        r##"
state: { who: "Ada", lines: [{ item: Desk, qty: 2 }], errors: {}, valid: false, dirty: false, sent: 0 }
root:
  type: ui-form
  props: { bindErrors: errors, bindValid: valid, bindDirty: dirty, rules: { who: { label: Name, required: true } } }
  on: { submit: { action: set_state, key: sent, value: "{{ sent + 1 }}" } }
  children:
    - { type: ui-error-summary }
    - { type: ui-input, props: { bind: who } }
    - type: ui-repeater
      props: { bind: lines, itemLabel: Line, fields: [ { field: item, label: Item, required: true }, { field: qty, label: Qty, type: number, min: 1 } ] }
    - { type: ui-button, props: { label: Send, type: submit }, attrs: { id: go } }
    - { type: ui-save-bar }
"##,
        r##"
        const f = (n, i) => $$(`[data-field=${n}]`)[i], msgs = () => $$("[data-rep-error]").map((m) => m.textContent).filter(Boolean).join("|");
        log(S().valid, S().dirty);
        // A new row is wrong: the form isn't valid, and its changes are unsaved changes.
        $("[data-rep-add]").click(); await wait(30); log(S().valid, S().dirty, msgs() || "-");
        // Sending stops: the summary lists the row's field, and the row shows its messages.
        $("#go").click(); await wait(30); log(S().sent, $("[role=alert] li").textContent, msgs(), JSON.stringify(Object.keys(S().errors)));
        $("[role=alert] button").click(); await wait(30); log(document.activeElement === f("item", 1));
        type(f("item", 1), "Lamp"); type(f("qty", 1), "0"); await wait(30); log($("[role=alert] li").textContent, S().valid);
        type(f("qty", 1), "3"); await wait(30); log(S().valid, $("[role=alert]").hidden);
        // Discard puts the rows back, and their messages go.
        $("[data-form-discard]").click(); await wait(30); log(S().lines.length, S().dirty, msgs() || "-");
        $("#go").click(); await wait(30); log(S().sent);
"##,
        &[
            "true false",
            "false true -",
            r#"0 Line 2, Item: Required Required ["lines.1.item"]"#,
            "true",
            "Line 2, Qty: At least 1 false",
            "true true",
            "1 false -",
            "1",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_sections_fold_and_open_for_a_wrong_field() {
    check(
        "form-sections-fold",
        r##"
state: { email: "", phone: "", errors: {} }
root:
  type: ui-form
  props: { columns: 2, bindErrors: errors, rules: { email: { label: Email, required: true } } }
  children:
    - { type: ui-error-summary }
    - type: ui-form-section
      props: { title: Contact, description: How to reach you, collapsible: true, open: false, layout: aside, columns: 2, span: full }
      children:
        - { type: ui-field, props: { label: Email, error: "{{ errors.email }}" }, children: [ { type: ui-input, props: { bind: email }, attrs: { id: email } } ] }
        - { type: ui-field, props: { label: Phone }, children: [ { type: ui-input, props: { bind: phone }, attrs: { id: phone } } ] }
    - { type: ui-button, props: { label: Go, type: submit }, attrs: { id: go } }
"##,
        r##"
        const d = $("details[data-section-in]"), sum = $("summary[data-section-head]"), email = $("#email");
        log(d.open, email.checkVisibility(), sum.querySelector("h3").textContent, getComputedStyle(d).display);
        sum.click(); await wait(30); log(d.open, email.checkVisibility(), $("#phone").getBoundingClientRect().left > email.getBoundingClientRect().left);
        // The heading sits beside the fields while there's room.
        log(sum.getBoundingClientRect().right <= email.getBoundingClientRect().left);
        sum.click(); await wait(30); log(d.open);
        // A submit with a wrong field in it opens the section and goes there.
        $("#go").click(); await wait(30); log(d.open, S().errors.email);
        $("[role=alert] button").click(); await wait(30); log(d.open, document.activeElement === email);
        // Folded, it says how many of its fields need fixing; open, or once they're right, it doesn't.
        const count = $("[data-section-count]"), shown = () => count.checkVisibility() ? count.textContent : "-";
        log(shown()); sum.click(); await wait(30); log(d.open, shown());
        $("[role=alert] button").click(); await wait(30); log(d.open, document.activeElement === email);
        type(email, "a@b.co"); sum.click(); await wait(30); log(d.open, shown(), d.hasAttribute("data-wrong"));
"##,
        &["false false Contact grid", "true true true", "true", "false", "true Required", "true true", "-", "false 1 to fix", "true true", "false - false"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_asks_the_server_with_remote_rules() {
    check(
        "form-remote",
        r##"
state: { user: "", errors: {}, valid: false, checking: [], sent: 0 }
root:
  type: ui-form
  props: { bindErrors: errors, bindValid: valid, bindChecking: checking, rules: { user: { label: User, minLength: 3, remote: { url: /avail, wait: 300 } } } }
  on: { submit: { action: set_state, key: sent, value: "{{ sent + 1 }}" } }
  children:
    - { type: ui-input, props: { bind: user }, attrs: { id: user } }
    - { type: ui-button, props: { label: Go, type: submit }, attrs: { id: go } }
"##,
        r##"
        const asked = []; let release;
        window.fetch = (u) => {
          asked.push(String(u)); const v = new URL(u, "http://x").searchParams.get("value");
          if (v === "down") return Promise.reject(new Error("down"));
          if (v === "slow") return new Promise((r) => { release = (ok) => r({ ok: true, json: () => Promise.resolve(ok) }); });
          return Promise.resolve({ ok: true, json: () => Promise.resolve(v === "taken" ? { valid: false, message: "Taken" } : { valid: true }) });
        };
        const fails = []; document.addEventListener("failed", (e) => fails.push(e.detail.key));
        const user = $("#user");
        // Too short by the other rules: not asked.
        type(user, "ab"); await wait(400); log(asked.length, S().checking);
        // Asked a while after typing stops; shown once the field is left.
        type(user, "taken"); log(S().checking, S().valid, $("form").getAttribute("aria-busy")); await wait(350); log(asked, S().checking, S().errors); user.blur(); $("#go").focus(); await wait(160); log(S().errors);
        // Only the last of quick changes is asked; an answer had before isn't asked again.
        type(user, "fine"); await wait(100); type(user, "fine2"); await wait(350); log(asked.length, asked.at(-1), S().valid, S().errors);
        type(user, "taken"); log(asked.length, S().checking, S().errors);
        // A submit waits for the answer, asked at once.
        type(user, "slow"); $("#go").click(); await wait(10); log(asked.at(-1), S().sent); release(true); await wait(10); log(S().sent, S().valid);
        // Not valid while it's being asked.
        type(user, "slower"); log(S().valid); await wait(350);
        // Can't be reached: it passes, and the form says so.
        type(user, "down"); await wait(350); log(fails, S().valid, S().errors); $("#go").click(); await wait(10); log(S().sent);
"##,
        &[
            "0 []",
            r#"["user"] false true"#,
            r#"["/avail?value=taken"] [] {}"#,
            r#"{"user":"Taken"}"#,
            "2 /avail?value=fine2 true {}",
            r#"2 [] {"user":"Taken"}"#,
            "/avail?value=slow 0",
            "1 true",
            "false",
            r#"["user"] true {}"#,
            "2",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn import_reads_matches_checks_and_hands_over() {
    check(
        "import",
        r##"
state: { draft: [], rows: [] }
root:
  type: ui-import
  props:
    bind: draft
    bindImported: rows
    fields:
      - { field: name, label: Full name, required: true, aliases: [contact] }
      - { field: email, label: Email, required: true, unique: true }
      - { field: plan, label: Plan, type: select, options: [{ value: f, label: Free }, { value: p, label: Pro }] }
      - { field: seats, label: Seats, type: number, min: 1 }
      - { field: since, label: Joined on, type: date }
"##,
        r##"
        const csv = 'Contact,E-mail,Joined,Plan,Seats\nAda,ada@x.io,2026-01-02,pro,"1,200"\nBob,bob@x.io,03 Feb 2026,Free,0\nCy,cy@x.io,2026-03-01,Gold,3\n';
        carry("paste", $("[data-imp-zone] input"), "clipboardData", { files: [new File([csv], "c.csv", { type: "text/csv" })], getData: () => "" });
        await until(() => !$("[data-imp-step=match]").hidden);
        log($$("[data-imp-map] select").map((s) => s.selectedOptions[0].text).join("|"), $("[data-imp-file]").textContent);
        $("[data-imp-review]").click(); await wait(30);
        log(S().draft[0]); log(S().draft[1].since, S().draft[1].seats); log($("[data-imp-counts]").textContent, $("[data-dt-problems]").textContent, $("[data-imp-go]").textContent);
        // Put right in the table.
        const cell = $$("[data-dt-body] > tr")[2].querySelector("[data-dt-col=plan]"); cell.click(); key("Enter"); const s = $("[data-dt-editor]"); s.value = String([...s.options].findIndex((o) => o.text === "Pro")); s.dispatchEvent(new Event("change", { bubbles: true })); await wait(30);
        log($("[data-imp-go]").textContent); $("[data-imp-go]").click(); await wait(30);
        log($("[data-imp-result]").textContent, $("[data-imp-done] [data-imp-muted]").textContent, S().rows.map((r) => r.name), "_row" in S().rows[0]);
        // Excel written and read back: numbers, dates, text that needs escaping.
        const Z = window.__irenderSheets;
        let back = null; Z.workbook([["A", "D"], [1.5, "2026-02-03"], ["x & <y>", true]]).arrayBuffer().then((b) => Z.xlsx(b)).then((r) => (back = r)); await until(() => back);
        log(JSON.stringify(back), Z.csv('a;"b;c"\n1;2'), Z.toCsv([["=1+1", "a,b", -5]]));
"##,
        &[
            "Contact|E-mail|Plan|Seats|Joined c.csv · 3 rows",
            r#"{"_row":2,"name":"Ada","email":"ada@x.io","plan":"p","seats":1200,"since":"2026-01-02"}"#,
            "2026-02-03 0",
            "1 row is ready · 2 need attention (left out unless put right) 2 problems Import 1 row",
            "Import 2 rows",
            r#"2 rows imported 1 row was left out ["Ada","Cy"] false"#,
            r#"[["A","D"],["1.5","2026-02-03"],["x & <y>","TRUE"]] [["a","b;c"],["1","2"]] '=1+1,"a,b",-5"#,
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn stats_details_comments_and_typed_inputs() {
    check(
        "inputs",
        r##"
lang: en-US
state:
  rev: 48210
  owner: Ada
  stage: won
  tags: [alpha]
  card: null
  policy: ""
  full: false
  phone: "+919876543210"
  country: ""
  ok: false
  t1: "09:30"
  t2: null
  text: ""
  named: []
  thread: [ { id: a, who: Ada, when: "2026-11-20T08:00:00Z", text: "Hi @Bob Stone", replies: [ { id: b, who: Bob Stone, when: "2026-11-20T09:00:00Z", text: Thanks } ] } ]
root:
  type: stack
  children:
    - { type: ui-stat, props: { label: Revenue, value: "{{ rev }}", format: currency, currency: EUR, decimals: 0, previous: 40000 } }
    - { type: ui-stat, props: { label: Enrolled, value: 640, target: 800 } }
    - type: ui-description-list
      children:
        - { type: ui-description-item, props: { label: Owner, bind: owner } }
        - { type: ui-description-item, props: { label: Stage, bind: stage, edit: select, options: [{ value: won, label: Won }, { value: lost, label: Lost }] } }
    - { type: ui-tags-input, props: { bind: tags, suggestions: [Beta, Gamma], label: Tags } }
    - { type: ui-masked-input, props: { bind: card, mask: card, label: Card } }
    - { type: ui-masked-input, props: { bind: policy, mask: "AA-####/**", raw: true, bindComplete: full, label: Policy } }
    - { type: ui-phone-input, props: { bind: phone, bindCountry: country, bindValid: ok, label: Phone } }
    - { type: ui-time-picker, props: { bind: t1, min: "09:00", max: "18:00", label: Start } }
    - { type: ui-time-picker, props: { bind: t2, durationFrom: "{{ t1 }}", hour12: false, label: End } }
    - { type: ui-mention-input, props: { bind: text, people: [Ada, Bob Stone], bindMentions: named, label: Message } }
    - { type: ui-comments, props: { bind: thread, user: Bob Stone, people: [Ada, Bob Stone], resolvable: true } }
"##,
        r##"
        const I = (l) => $(`input[aria-label="${l}"], textarea[aria-label="${l}"]`);
        log($("[data-stat-figure]").textContent, $("[data-stat-change]").textContent, $("[data-tone]").dataset.tone, $("[data-stat-target]").textContent);
        root.__irender.set("rev", 30000); log($("[data-stat-figure]").textContent, $("[data-tone]").dataset.tone);
        const edits = $$("[data-dl-edit]"); edits[0].click(); $("[data-dl-input]").value = "Zed"; key("Enter"); log(S().owner, document.activeElement === edits[0]);
        edits[1].click(); const sel = $("[data-dl-input]"); sel.value = "1"; sel.dispatchEvent(new Event("change", { bubbles: true })); log(S().stage, edits[1].textContent);
        const t = I("Tags"); type(t, "beta,"); log(S().tags); type(t, "zed"); key("Enter"); key("Backspace"); log(S().tags);
        type(I("Card"), "4111111111111111"); log(S().card); type(I("Card"), "378282246310005"); log(S().card);
        type(I("Policy"), "ab1234c9"); log(S().policy, I("Policy").value, S().full);
        const p = I("Phone"); log(p.value, S().country, S().ok); type(p, "+44 20 7946 0958"); log(S().phone, S().country); type(p, "020 7946 09"); log(S().phone, S().ok);
        for (const v of ["2", "930", "9:45am", "14.15", "25:00"]) { type(I("Start"), v); I("Start").blur(); await wait(5); log(v, S().t1, I("Start").getAttribute("aria-invalid")); }
        const e = I("End"); e.focus(); e.click(); log($$("[role=option]", e.parentElement).slice(0, 2).map((o) => o.textContent)); key("ArrowDown"); key("Enter"); log(S().t2);
        const m = I("Message"); m.focus(); m.value = "hi @bo"; m.setSelectionRange(6, 6); m.dispatchEvent(new Event("input", { bubbles: true })); log($$("[data-mention-list] [role=option]").length); key("Enter"); log(S().text, S().named);
        const box = $$("section textarea").at(-1); type(box, "New one"); box.closest("[data-c-box]").querySelector("[data-primary]").click(); await wait(30); log(S().thread.length, S().thread[1].who, $("[data-mention]").dataset.mention);
        $("[data-do=reply-a]").click(); await wait(20); type(document.activeElement, "Re"); key("Enter", { ctrlKey: true }); log(S().thread[0].replies.length);
        $("[data-do='delete-b']").click(); $("[data-do=confirm-delete]").click(); log(S().thread[0].replies.map((r) => r.text));
        $("[data-do=resolve-a]").click(); log(S().thread[0].resolved, !!$("[data-do='open-a']"));
"##,
        &[
            "€48,210 21% good 80% of 800",
            "€30,000 bad",
            "Zed true",
            "lost Lost",
            r#"["alpha","Beta"]"#,
            r#"["alpha","Beta"]"#,
            "4111 1111 1111 1111",
            "3782 822463 10005",
            "AB1234c9 AB-1234/c9 true",
            "98765 43210 IN true",
            "+442079460958 GB",
            "+4420794609 false",
            "2 14:00 null",
            "930 09:30 null",
            "9:45am 09:45 null",
            "14.15 14:15 null",
            "25:00 14:15 true",
            r#"["14:3015 min","15:0045 min"]"#,
            "14:30",
            "1",
            r#"hi @Bob Stone  ["Bob Stone"]"#,
            "2 Bob Stone me",
            "2",
            r#"["Re"]"#,
            "true true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn pickers_lightbox_pivot_and_scheduler() {
    check(
        "pickers",
        r##"
lang: en-US
state:
  at: null
  q: { op: and, rules: [ { field: age, operator: gte, value: 18 }, { op: or, rules: [ { field: city, operator: in, value: [Oslo] }, { field: name, operator: contains, value: "" } ] } ] }
  words: ""
  ok: false
  hits: []
  people: [ { name: Ada, age: 36, city: Oslo }, { name: Bob, age: 12, city: Oslo }, { name: Cy, age: 50, city: Rome } ]
  picked: [read]
  dept: b1
  nodes: [ { id: a, label: Clinical, children: [ { id: a1, label: Surgery } ] }, { id: b, label: Office, children: [ { id: b1, label: Finance } ] } ]
  cell: null
  layout: null
  sales: [ { region: North, rep: Ann, date: 2026-01-04, amt: 120 }, { region: North, rep: Bo, date: 2026-02-14, amt: 80 }, { region: South, rep: Cy, date: 2026-01-20, amt: 300 }, { region: South, rep: Cy, date: 2026-04-01, amt: 50 } ]
  rooms: [ { id: r1, title: One }, { id: r2, title: Two }, { id: r3, title: Three } ]
  book: [ { id: a, resource: r1, title: A, start: "2026-11-23T09:00", end: "2026-11-23T11:30" }, { id: b, resource: r1, title: B, start: "2026-11-23T11:00", end: "2026-11-23T12:00" }, { id: c, resource: r2, title: C, start: "2026-11-23T10:00", end: "2026-11-23T11:00" } ]
  day: 2026-11-23
  clashes: []
  open: null
root:
  type: stack
  children:
    - { type: ui-query-builder, props: { bind: q, bindText: words, bindValid: ok, rows: "{{ people }}", bindMatches: hits, fields: [ { field: age, label: Age, type: number }, { field: name, label: Name }, { field: city, label: City, type: select, options: [Oslo, Rome] } ] } }
    - { type: ui-transfer-list, props: { bind: picked, options: [read, write, delete], reorderable: true } }
    - { type: ui-tree-select, props: { bind: dept, nodes: "{{ nodes }}", path: true, label: Dept } }
    - { type: ui-lightbox, props: { bind: at, images: [ "data:image/gif;base64,R0lGODlhAQABAAAAACw=", "/b.png", "/c.png" ], max: 2 } }
    - { type: ui-pivot, props: { rows: "{{ sales }}", fields: [ { field: region, label: Region }, { field: rep, label: Rep }, { field: date, label: Quarter, part: quarter } ], down: [region, rep], across: [date], values: [ { field: amt, label: Amount } ], bindCell: cell, bind: layout } }
    - { type: ui-scheduler, props: { resources: "{{ rooms }}", events: "{{ book }}", bindEvents: book, bindEvent: open, bindDate: day, bindClashes: clashes, from: "08:00", to: "18:00", creatable: true, now: false } }
"##,
        r##"
        log(S().words, S().ok, S().hits.map((h) => h.name));
        const v = $('[data-path="1.1:v"]'); type(v, "c"); log(S().hits.map((h) => h.name), S().ok, document.activeElement === $('[data-path="1.1:v"]'));
        $('[data-path=":add"]').click(); log(S().q.rules.length, document.activeElement.getAttribute("data-path")); $('[data-path="2:x"]').click(); $('[data-path=":or"]').click(); log(S().words);
        const lists = $$("[data-t-list] [role=listbox]"); lists[0].querySelector("[role=option]").click(); $("[data-m=add]").click(); lists[1].focus(); key("ArrowDown"); key("ArrowUp", { altKey: true }); log(S().picked);
        const ts = $("[data-ts-button]"); log(ts.textContent); ts.click(); const pop = $("[data-i-x=ui-tree-select] [popover]"); log(pop.matches(":popover-open"));
        const item = $$("[role=treeitem]", pop).find((n) => n.textContent.includes("Clinical")); (item.querySelector("[data-tree-row]") || item).click(); await wait(20); log(S().dept, pop.matches(":popover-open"), ts.textContent);
        $$("[data-lb-open]")[1].click(); log(S().at, $("dialog").open, $("[data-lb-count]").textContent); key("ArrowRight"); log(S().at); root.__irender.set("at", null); log($("dialog").open, $$("[data-lb-figure]:not([hidden])").length, $("[data-lb-more]").textContent);
        const table = () => $$("[data-i-x=ui-pivot] tr").map((r) => [...r.cells].map((c) => c.textContent).join("|")).join(" ; ");
        log(table()); $$("[data-pv-fold]")[0].click(); log($$("[data-i-x=ui-pivot] tbody tr").length, S().layout.closed.length); $$("td[data-click]")[0].click(); log(S().cell.rows.length, S().cell.value);
        $$("[data-pv-sort]").at(-1).click(); log($$("[data-i-x=ui-pivot] tbody th").map((t) => t.textContent));
        log(S().clashes); const ev = (id) => $(`[data-sc-event="${id}"]`);
        ev("c").focus(); key("ArrowRight", { altKey: true }); log(S().book[2].start, S().book[2].end); key("ArrowDown", { altKey: true }); log(S().book[2].resource, document.activeElement.getAttribute("data-sc-event")); key("ArrowRight", { altKey: true, shiftKey: true }); log(S().book[2].end); key("Enter"); log(S().open.id);
        const row = $("[data-sc-row=r2]"); row.scrollIntoView({ block: "center" }); await wait(20); const b = row.getBoundingClientRect();
        pointer("pointerdown", b.left + b.width * 0.5 + 2, b.top + 10, row); pointer("pointermove", b.left + b.width * 0.6, b.top + 10, row); pointer("pointerup", b.left + b.width * 0.6, b.top + 10, row); log(S().book.length, S().book[3].start, S().book[3].end, S().book[3].resource);
        $("[data-key=v-week]").click(); log($$("[data-sc-head] span").length); $("[data-key=v-day]").click(); $("[data-key=next]").click(); log(S().day);
"##,
        &[
            r#"Age is at least 18 and (City is any of Oslo or Name contains …) false ["Ada"]"#,
            r#"["Ada","Cy"] true true"#,
            "3 2:f",
            "Age is at least 18 or (City is any of Oslo or Name contains “c”)",
            r#"["write","read"]"#,
            "Office / Finance",
            "true",
            "a false Clinical",
            "1 true 2 of 3",
            "2",
            "false 2 +1",
            "Region / Rep|Q1 2026|Q2 2026|Total ; North|200||200 ; Ann|120||120 ; Bo|80||80 ; South|300|50|350 ; Cy|300|50|350 ; Total|500|50|550",
            "3 1",
            "2 200",
            r#"["South","Cy","North"]"#,
            r#"["a","b"]"#,
            "2026-11-23T10:15 2026-11-23T11:15",
            "r3 c",
            "2026-11-23T11:30",
            "c",
            "4 2026-11-23T13:00 2026-11-23T14:00 r2",
            "7",
            "2026-11-24",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn codes_diff_log_and_code_editor() {
    check(
        "devtools",
        r##"
state:
  code: ""
  list: []
  a: "one\ntwo\nthree\nfour\nfive\nsix\nseven"
  b: "one\n2\nthree\nfour\nfive\nsix\nseven\neight"
  log: [ { time: "2026-11-22T10:00:00Z", level: info, text: Started }, { level: error, text: "Not \u001b[31mgood\u001b[0m" }, "WARN plain" ]
  src: "const a = 1;\n"
  cur: null
root:
  type: stack
  children:
    - { type: ui-qr-code, props: { value: "https://example.com" } }
    - { type: ui-barcode, props: { value: "400638133393", type: ean13 } }
    - { type: ui-barcode, props: { value: "é" } }
    - { type: ui-code-scanner, props: { bind: code, bindList: list } }
    - { type: ui-diff, props: { before: "{{ a }}", after: "{{ b }}", context: 1 } }
    - { type: ui-diff, props: { mode: fields, before: { name: Ada, seats: 3 }, after: { name: Ada, seats: 5 }, fieldLabels: { seats: Seats } } }
    - { type: ui-log, props: { lines: "{{ log }}" } }
    - { type: ui-code-editor, props: { bind: src, language: js, bindCursor: cur, diagnostics: [ { line: 1, message: Unused, severity: warning } ], completions: [ { label: customer.name } ] } }
"##,
        r##"
        // A QR code's fixed parts and size; how it reads is checked by eye and by scanners.
        const m = window.__irenderQr("https://example.com", "M"); log(m.length, m[0].slice(0, 8).map(Number).join(""), m[6].slice(8, 16).map(Number).join(""), window.__irenderQr("x".repeat(3000), "H"));
        log($$("svg[data-code]").length, $$("svg[data-code]")[1].getAttribute("aria-label"), $("[data-code-error]").textContent);
        log(window.__irenderBarcode("code128", "AB12").w.join("").length, window.__irenderBarcode("ean13", "4006381333930"), window.__irenderBarcode("itf", "123").text);
        const i = $("[data-scan-input]"); for (const c of ["X1", "X2", "X1"]) { i.value = c; i.focus(); key("Enter"); } log(S().code, S().list);
        const d = $$("[data-i-x=ui-diff]"); log($$("[data-added], [data-removed]", d[0]).map((n) => n.textContent), $$("[data-fold]", d[0]).length, $$("tr", d[0]).length); $("[data-fold] button", d[0]).click(); log($$("[data-fold]", d[0]).length);
        $$("[data-d-bar] button", d[0])[1].click(); log(d[0].dataset.view, $$("[data-k=add]", d[0]).length); log($$("tbody tr", d[1]).map((r) => r.textContent));
        log(window.__irenderDiff([..."ABCABBA"], [..."CBABAC"]).filter((e) => e[0] !== " ").length);
        log($$("[data-log-line]").map((l) => l.dataset.lv), $("[data-log-text] span[style]").textContent); root.__irender.set("log", [...S().log, "new"]); log($$("[data-log-line]").length);
        type($("[data-i-x=ui-log] input"), "new"); log($$("[data-log-line]").length, $("mark").textContent); type($("[data-i-x=ui-log] input"), ""); $("[data-lv=error][aria-pressed]").click(); log($$("[data-log-line]").length);
        const t = $("[data-i-x=ui-code-editor] textarea"); t.focus(); t.setSelectionRange(t.value.length, t.value.length); key("{"); key("Enter"); log(S().src, S().cur);
        t.setRangeText("cust", t.selectionStart, t.selectionEnd, "end"); t.dispatchEvent(new Event("input", { bubbles: true })); log($$("[data-ce-list] [role=option]").length); key("Enter"); key("/", { ctrlKey: true }); log(S().src, $("[data-ce-problems]").textContent);
"##,
        &[
            "25 11111110 10101010 null",
            "2 Barcode: 4 006381 333931 “é” can't be made into this kind of barcode",
            "43 null 0123",
            r#"X1 ["X1","X2"]"#,
            r#"["+2","−1"] 1 6"#,
            "0",
            "unified 2",
            r#"["Seats35"]"#,
            "5",
            r#"["info","error","warn"] good"#,
            "4",
            "1 new",
            "3",
            r#"const a = 1;"#,
            "{",
            "  ",
            r#"} {"line":3,"column":3}"#,
            "1",
            "const a = 1;",
            "{",
            "  // customer.name",
            "} Line 1: Unused",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn signature_plan_guidance_and_assistant() {
    check(
        "media",
        r##"
state:
  sig: null
  meta: null
  sel: null
  item: null
  rooms: [ { id: a, x: 40, y: 40, width: 200, height: 140, label: Ward A, status: Free }, { id: b, x: 280, y: 40, width: 160, height: 140, label: Lab, status: Busy }, { id: d, shape: polygon, points: [[0, 0], [180, 0], [120, 140]], x: 700, y: 300, label: Yard, locked: true } ]
  tour: null
  shut: false
  tab: a
  files: [ { id: 1, name: a.pdf, size: 2400 }, { id: 2, name: b.png, status: uploading, progress: 40 } ]
  allow: null
  answers: {}
  done: false
  messages: [ { role: user, content: Hi, attachments: [ { name: r.pdf } ] }, { role: assistant, content: Yes, sources: [ { title: Guide } ] } ]
root:
  type: stack
  children:
    - { type: ui-banner, props: { text: Soon, bind: shut, countdown: "2099-01-01T00:00" } }
    - { type: ui-button, props: { label: One }, attrs: { id: one } }
    - { type: ui-button, props: { label: Two }, attrs: { id: two } }
    - { type: ui-tour, props: { bind: tour, steps: [ { target: "#one", title: First }, { target: "#two", title: Second, set: { tab: b } }, { title: Done } ] } }
    - { type: ui-session-timeout, props: { after: 20, warn: 1 } }
    - { type: ui-signature, props: { bind: sig, bindMeta: meta } }
    - { type: ui-plan, props: { items: "{{ rooms }}", bindItems: rooms, bind: sel, bindItem: item, tones: { Free: success, Busy: danger }, grid: 20, legend: true, search: true } }
    - { type: ui-conversation, props: { messages: "{{ messages }}", markdown: false } }
    - { type: ui-attachment, props: { files: "{{ files }}", bindFiles: files }, attrs: { id: files } }
    - { type: ui-sources, props: { text: "Cost fell [1] and [9].", items: [ { title: Review, url: "https://example.com/a" } ] }, attrs: { id: sources } }
    - { type: ui-tool-call, props: { title: Cancel it, status: approval, input: { id: 42 }, bindApproval: allow } }
    - { type: ui-trace, props: { spans: [ { id: a, name: Answer, start: 0, duration: 4200 }, { id: b, parent: a, name: Search, start: 100, duration: 900 }, { id: c, parent: a, name: Write, start: 1100, duration: 3000, status: error, error: Cut off } ] } }
    - { type: ui-questionnaire, props: { bind: answers, bindDone: done, questions: [ { id: kind, question: What for?, options: [Work, Home], required: true }, { id: n, question: How many?, type: number, min: 1 }, { id: s, question: How sure?, type: scale } ] } }
"##,
        r##"
        root.__irender.set("tour", 0); await wait(30); log($("[data-tour-card] h2").textContent, document.activeElement.textContent); key("ArrowRight"); await wait(20); log(S().tour, S().tab); key("ArrowRight"); $("[data-tour-card] [data-next]").click(); log(S().tour, !!$("[data-tour-card]"));
        log(/^[\d,]+d \d\d:\d\d:\d\d$/.test($("[data-banner-left]").textContent)); $("[data-banner-close]").click(); log(S().shut, $("[data-banner-close]").parentElement.hidden, !!$("[data-i-x=ui-session-timeout] dialog[open]"));
        const c = $("[data-sg-pad] canvas"); c.scrollIntoView({ block: "center" }); await wait(20); const b = c.getBoundingClientRect();
        pointer("pointerdown", b.left + 50, b.top + 60, c); for (let i = 0; i < 30; i++) pointer("pointermove", b.left + 50 + i * 4, b.top + 60 + (i % 7), c); pointer("pointerup", b.left + 170, b.top + 60, c);
        log(S().sig.slice(0, 22), S().meta.method); $("[data-mode=type]").click(); log(S().sig); type($("[data-sg-type]"), "Ada"); log(S().meta.method, S().meta.name);
        $("[data-pl-item=a]").dispatchEvent(new MouseEvent("click", { bubbles: true })); log(S().sel, S().item.label); $("[data-pl-item=a]").focus(); key("ArrowRight", { altKey: true }); log(S().rooms[0].x, S().item.x); key("ArrowRight"); log(document.activeElement.getAttribute("data-pl-item"), $("[data-pl-legend]").textContent);
        type($("[data-pl-search] input"), "yard"); key("Enter"); log(S().sel);
        log($$("[data-role] [data-i-x=ui-attachment]").length, $$("[data-role] [data-i-x=ui-sources]").length);
        $("#files [data-tool]").click(); log(S().files.length, $("#files small").textContent); log($$("#sources [data-cite]").length); $("#sources [data-cite]").click(); log($("#sources [data-hit] a").textContent);
        $("[data-yes]").click(); log(S().allow); const ti = () => $$("[data-i-x=ui-trace] [role=treeitem]"); ti()[2].click(); log($$("[data-detail] dd").map((x) => x.textContent)); ti()[0].focus(); key("ArrowLeft"); log(ti().length);
        const Q = $("[data-i-x=ui-questionnaire]"), go = () => Q.querySelector("[data-primary]").click(); go(); log(Q.querySelector("[data-error]").textContent); Q.querySelector("input").click(); await wait(250); log(S().answers, Q.querySelector("legend").textContent);
        type(Q.querySelector("input"), "0"); go(); log(Q.querySelector("[data-error]").textContent); type(Q.querySelector("input"), "3"); key("Enter"); Q.querySelectorAll("[data-scale] input")[3].click(); await wait(20); go(); log(S().answers, S().done, Q.querySelectorAll("dd").length);
"##,
        &[
            "First Next",
            "1 b",
            "null false",
            "true",
            "true true false",
            "data:image/png;base64, drawn",
            "null",
            "typed Ada",
            "a Ward A",
            "60 60",
            "b Free (1)Busy (1)",
            "d",
            "1 1",
            "1 Uploading… 40%",
            "1",
            "Review",
            "true",
            r#"["Write","3.0 s","Cut off"]"#,
            "1",
            "Please answer this one",
            r#"{"kind":"Work"} How many?"#,
            "At least 1",
            r#"{"kind":"Work","n":3,"s":4} true 3"#,
        ],
    );
}

/// The search in a list keeps the focus as the list is redrawn under it (the country list
/// used to close at the first letter).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn lists_stay_open_as_their_search_is_typed_in() {
    check(
        "list-search",
        r##"
lang: en-US
state: { phone: null, q: "https://example.com" }
root:
  type: stack
  children:
    - { type: ui-phone-input, props: { bind: phone, country: IN, label: Phone } }
    - { type: ui-qr-code, props: { value: "{{ q }}", logoText: AC, logoSize: 0.3 } }
    - { type: ui-qr-code, props: { value: "{{ q }}", logo: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E", logoShape: circle } }
"##,
        r##"
        const open = () => !$("[data-i-x=ui-phone-input] [data-in-pop]").hidden, found = () => $$("[data-i-x=ui-phone-input] [role=option] span:not([aria-hidden])").map((o) => o.textContent);
        $("[data-country]").click(); const s = $("[data-in-search]"); log(document.activeElement === s, open());
        for (const t of ["u", "un", "united k"]) { s.value = t; s.dispatchEvent(new Event("input", { bubbles: true })); await wait(5); log(t, document.activeElement === s, open(), found().includes("United Kingdom"), found().length < 2); }
        key("Enter"); log($("[data-country]").getAttribute("aria-label"), document.activeElement.type);
        // A logo takes the level that still reads without what's under it, and a space of its own.
        const at = (n) => $$("svg[data-code]")[n]; log(at(0).viewBox.baseVal.width, at(0).querySelector("text").textContent, at(1).querySelector("image").getAttribute("preserveAspectRatio"));
        const before = at(0).querySelector("path").getAttribute("d").length; root.__irender.set("q", "https://example.com/b"); log(at(0).querySelector("path").getAttribute("d").length !== before);
"##,
        &[
            "true true",
            "u true true true false",
            "un true true true false",
            "united k true true true true",
            "Country: United Kingdom, +44 tel",
            "37 AC xMidYMid slice",
            "true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn repeating_events_message_actions_tabs_ratings_and_lookups() {
    check(
        "more",
        r##"
state:
  ev: null
  evs: [ { id: s, title: Standup, start: "2026-11-02T09:00", end: "2026-11-02T09:15", repeat: { every: week, on: [1, 3], until: "2026-11-20" } }, { id: m, title: Rent, start: "2026-01-31", repeat: month } ]
  model: a
  tab: b
  stars: 2.5
  msgs: [ { id: u1, role: user, content: Hi }, { id: a1, role: assistant, content: Hello } ]
  draft: ""
  files: [ { name: x.pdf } ]
  pick: f7
  f: {}
root:
  type: stack
  children:
    - { type: ui-event-calendar, props: { events: "{{ evs }}", bindEvents: evs, bindEvent: ev, date: "2026-11-10", today: "2026-11-10", weekStart: 1 } }
    - { type: ui-tabs, props: { bind: tab, orientation: vertical, activation: manual }, children: [ { type: ui-tab-panel, props: { value: a } }, { type: ui-tab-panel, props: { value: b } } ] }
    - { type: ui-rating, props: { bind: stars, half: true, label: Rate } }
    - { type: ui-conversation, props: { messages: "{{ msgs }}", markdown: false, actions: [copy, retry, edit, up, down] } }
    - { type: ui-chat-input, props: { messages: msgs, bind: draft, endpoint: /none, body: { model: "{{ model }}" }, headers: { x-key: abc }, bindFiles: files } }
    - { type: ui-combobox, props: { bind: pick, searchUrl: /s, labels: [ { value: f7, label: Seven } ] } }
    - { type: ui-filters, props: { bind: f }, children: [ { type: ui-filter, props: { field: customer, label: Customer, optionsUrl: /customers } } ] }
"##,
        r##"
        const sent = []; window.fetch = (u, o) => {
          sent.push([String(u), o && o.body ? JSON.parse(o.body) : null, o && o.headers && o.headers["x-key"]]);
          if (!String(u).startsWith("/customers")) return Promise.reject(new Error("down"));
          const q = new URL(u, "http://x").searchParams.get("q");
          return Promise.resolve({ ok: true, json: () => Promise.resolve(["Acme", "Beta", "Ada"].filter((x) => x.toLowerCase().includes(q)).map((x) => ({ value: x, label: x, count: 2 }))) });
        };
        const shown = () => $$("[data-ev-event]").map((e) => e.getAttribute("data-ev-event"));
        // Each of its days; a month without a 31st has none.
        log(shown()); const c = $('[data-ev-event="s@2026-11-04"]'); c.click(); log(S().ev.start, S().ev.occurrence);
        // One day moved is an event of its own.
        $('[data-ev-event="s@2026-11-04"]').focus(); key("ArrowRight", { altKey: true }); log(S().evs.length, S().evs[0].repeat.except, S().evs[1].start, shown().length);
        const tabs = $$("[role=tab]"); tabs[1].focus(); key("ArrowUp"); log(document.activeElement.textContent, S().tab); document.activeElement.click(); log(S().tab, $("[role=tablist]").getAttribute("aria-orientation"));
        log($$("[data-half] label[data-on]").length); $$("[data-half] input")[6].click(); log(S().stars, $$("[data-half] label[data-on]").length);
        log(getComputedStyle($$("[data-act=up]")[0]).display, getComputedStyle($$("[data-act=edit]")[1]).display); $$("[data-act=up]")[1].click(); log(S().msgs[1].feedback, $$("[data-act=up]")[1].getAttribute("aria-pressed"));
        $$("[data-act=edit]")[0].click(); log(S().msgs.length, S().draft);
        $("form [data-send-button]").click(); await wait(50); const [url, body, head] = sent.at(-1); log(url, body.model, body.messages[0].attachments, head, S().files, S().msgs.map((m) => m.role)); const n = sent.length; $("[data-try-again]").click(); await wait(30); log(sent.length - n, sent.at(-1)[1].messages.length, S().msgs.length);
        log($("[data-i-x=ui-combobox]").textContent.includes("Seven"));
        $$("button").find((b) => b.textContent.includes("Filters")).click(); await wait(50); log($$("[data-fl-pick]").length); type($("[data-fl-search]"), "ac"); await wait(400); log(sent.at(-1)[0], $$("[data-fl-options] label").map((l) => l.textContent)); $("[data-fl-pick]").click(); $("[data-fl-apply]")?.click(); await wait(20); log(S().f);
"##,
        &[
            r#"["m@2026-10-31","s@2026-11-02","s@2026-11-04","s@2026-11-09","s@2026-11-11","s@2026-11-16","s@2026-11-18"]"#,
            "2026-11-04T09:00 2026-11-04",
            r#"3 ["2026-11-04"] 2026-11-05T09:00 7"#,
            "a b",
            "a vertical",
            "5",
            "3.5 7",
            "none none",
            "up true",
            "0 Hi",
            r#"/none a [{"name":"x.pdf"}] abc [] ["user","system"]"#,
            "1 1 2",
            "true",
            "3",
            r#"/customers?q=ac ["Acme2"]"#,
            r#"{"customer":["Acme"]}"#,
        ],
    );
}

// ---------------------------------------------------------------- apps

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn app_shell_screens_tabs_and_back() {
    check(
        "app-shell",
        r##"
state: { at: "", n: 0, busy: false }
root:
  type: ui-app-shell
  props: { bind: at, pullToRefresh: true, bindRefreshing: busy, height: 400 }
  children:
    - { type: ui-app-actions, children: [ { type: text, props: { value: actions } } ] }
    - { type: ui-app-screen, props: { value: home, label: Home, icon: "⌂" }, children: [ { type: element, style: { height: 900 } } ] }
    - { type: ui-app-screen, props: { value: list, label: List, icon: "≡", badge: "{{ n }}" } }
    - { type: ui-app-screen, props: { value: one, label: One, title: "Number {{ n }}", parent: list } }
"##,
        r##"
        const title = () => $("[data-app-title]").textContent, shown = () => $$("[data-app-screen]").filter((s) => !s.hidden).map((s) => s.getAttribute("data-app-screen")).join();
        const tabs = () => $$("[data-app-tab]").map((t) => t.textContent + (t.getAttribute("aria-current") ? "*" : "")).join("|");
        log(title(), shown(), tabs(), $("[data-app-back]").hidden);
        // The shell keeps a screen's place from its scroll event, which a busy machine may
        // send late: wait for it rather than a fixed time.
        const home = $("[data-app-screen=home]"), scrolled = new Promise((r) => home.addEventListener("scroll", r, { once: true }));
        home.scrollTop = 120; await Promise.race([scrolled, wait(2000)]);
        root.__irender.set("n", 3); $$("[data-app-tab]")[1].click(); await wait(30);
        log(title(), shown(), tabs());
        root.__irender.set("at", "one"); await wait(30);
        log(title(), shown(), tabs(), $("[data-app-back]").hidden);
        $("[data-app-back]").click(); await wait(30); log(S().at);
        $$("[data-app-tab]")[0].click(); await until(() => home.scrollTop === 120).catch(() => {}); log(S().at, home.scrollTop);
"##,
        &["Home home ⌂Home*|≡0List true", "List list ⌂Home|≡3List*", "Number 3 one ⌂Home|≡3List* false", "list", "home 120"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn offline_requests_are_kept_and_sent_later() {
    check(
        "offline-queue",
        r##"
state: { net: "", waiting: null, n: null, answer: null, title: Pump, kept: null, ready: false, draft: "" }
root:
  type: stack
  children:
    - { type: ui-offline-queue, props: { paths: [/api/], labels: { /api/records: Report }, labelField: title, bind: waiting, bindCount: n } }
    - { type: ui-local-store, props: { keys: [draft], name: "test", where: session, bindReady: ready } }
    - { type: button, props: { label: send }, on: { click: { action: submit, endpoint: "https://example.test/api/records", into: answer, body: { title: "{{ title }}" } } } }
"##,
        r##"
        // Pages from a file have no site: the paths are matched as they'd be on one.
        const O = window.__irenderOffline;
        await until(() => S().ready); log(S().ready, $("[data-w-note]").textContent);
        const q = await real(O.queue.add({ url: "https://example.test/api/a", body: "{}", label: "First" }));
        await real(O.queue.add({ url: "https://example.test/api/b", body: "{}", label: "Second" }));
        await until(() => S().n === 2); log(S().waiting.map((w) => [w.label, w.status, w.method]));
        // Gone again: no connection stops it, with nothing lost.
        const sent = [];
        self.__irenderFetch = async () => { throw new TypeError("no connection"); };
        log(await real(O.queue.flush()), (await real(O.queue.list())).length);
        // Back: the first goes, and the second's refused.
        self.__irenderFetch = async (url, init) => { sent.push(url.slice(-1) + init.headers["x-queued-id"].length); return url.endsWith("a") ? new Response('{"id":7}', { headers: { "content-type": "application/json" } }) : new Response('{"error":"No such site"}', { status: 422, headers: { "content-type": "application/json" } }); };
        log(await real(O.queue.flush()), sent.length); await until(() => S().waiting.length === 1 && S().waiting[0].status === "failed");
        log(S().waiting.map((w) => [w.label, w.status, w.error]), $("[data-w-tag]").textContent);
        // It's asked twice before it's deleted.
        $("[data-drop]").click(); log((await real(O.queue.list())).length); $("[data-drop]").click(); await until(() => S().n === 0); log(S().n);
        root.__irender.set("draft", "hello"); await wait(500); await until(() => sessionStorage.getItem("irender-store:test")); log(JSON.parse(sessionStorage.getItem("irender-store:test")).values); sessionStorage.clear();
"##,
        &["true Everything has been sent", r#"[["First","waiting","POST"],["Second","waiting","POST"]]"#, "0 2", "1 2", r#"[["Second","failed","No such site"]] Refused"#, "1", "0", r#"{"draft":"hello"}"#],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn camera_shrinks_and_stamps_chosen_photos() {
    check(
        "camera",
        r##"
state: { photos: [], one: null }
root:
  type: stack
  children:
    - { type: ui-camera, props: { bind: photos, multiple: true, max: 2, maxSize: 200, stamp: Site 4 }, attrs: { id: many } }
    - { type: ui-camera, props: { bind: one, ratio: 1, maxSize: 100, type: png }, attrs: { id: one } }
"##,
        r##"
        const c = document.createElement("canvas"); c.width = 800; c.height = 400; c.getContext("2d").fillRect(0, 0, 800, 400);
        // The decoding is the browser's.
        window.createImageBitmap = async () => c;
        const pick = (id, ...names) => { const d = new DataTransfer(); for (const n of names) d.items.add(new File(["x"], n, { type: "image/png" })); const i = $(`#${id} input[type=file]`); i.files = d.files; i.dispatchEvent(new Event("change")); };
        pick("many", "a.png", "b.png", "c.png"); await wait(50);
        log(S().photos.map((p) => [p.width, p.height, p.type, p.url.startsWith("data:image/jpeg"), p.size > 0]), $$("#many [data-cam-shots] img").length);
        log($("#many input[type=file]").disabled);
        // It's stamped along its foot: not all black there.
        const i = new Image(); i.src = S().photos[0].url; await until(() => i.complete && i.naturalWidth);
        const g = document.createElement("canvas").getContext("2d"); g.drawImage(i, 0, 0); log([...g.getImageData(0, 80, 200, 20).data].some((v, k) => k % 4 < 3 && v > 200), [...g.getImageData(0, 0, 200, 70).data].some((v, k) => k % 4 < 3 && v > 200));
        $("#many [data-cam-shots] button").click(); await wait(30); log(S().photos.length, $("#many input[type=file]").disabled);
        pick("one", "a.png"); await wait(50); log(S().one.width, S().one.height, S().one.type, S().one.name.endsWith(".png"));
"##,
        &[r#"[[200,100,"image/jpeg",true,true],[200,100,"image/jpeg",true,true]] 2"#, "true", "true false", "1 false", "100 100 image/png true"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn photo_marks_are_drawn_measured_and_undone() {
    check(
        "annotate",
        r##"
state: { src: null, out: null, marks: [] }
root: { type: ui-photo-annotate, props: { src: "{{ src }}", bind: out, bindMarks: marks, unit: mm, type: png } }
"##,
        r##"
        const c = document.createElement("canvas"); c.width = 400; c.height = 200; const g = c.getContext("2d"); g.fillStyle = "#fff"; g.fillRect(0, 0, 400, 200); g.fillStyle = "#000"; for (let i = 0; i < 400; i += 4) g.fillRect(i, 0, 2, 200);
        root.__irender.set("src", { url: c.toDataURL() }); const canvas = $("canvas"); await until(() => !canvas.hidden && canvas.clientWidth); await wait(30);
        const b = () => canvas.getBoundingClientRect();
        const drag = (x1, y1, x2, y2) => { const r = b(); const p = (t, x, y) => pointer(t, r.left + x * r.width, r.top + y * r.height, canvas); p("pointerdown", x1, y1); p("pointermove", x2, y2); p("pointerup", x2, y2); };
        const pick = (t) => $(`[data-tool=${t}]`).click();
        const types = () => S().marks.map((m) => m.type).join();
        pick("arrow"); drag(0.1, 0.8, 0.4, 0.4); pick("blur"); drag(0.5, 0.1, 0.9, 0.5); await wait(20); log(types(), S().out.startsWith("data:image/png"));
        // The stripes under the blur are one grey, apart from the edge.
        const i = new Image(); i.src = S().out; await until(() => i.complete && i.naturalWidth);
        const h = document.createElement("canvas").getContext("2d", { willReadFrequently: true }); h.canvas.width = 400; h.canvas.height = 200; h.drawImage(i, 0, 0);
        const row = (x) => [...h.getImageData(x, 60, 16, 1).data].filter((v, k) => k % 4 === 0); log(new Set(row(60)).size > 1, Math.max(...row(260)) - Math.min(...row(260)) < 200);
        // A length given once measures the rest.
        pick("measure"); drag(0.1, 0.9, 0.6, 0.9); await wait(10); type($("[data-an-text]"), "40"); key("Enter"); await wait(10); log(S().marks[2].length);
        drag(0.1, 0.95, 0.35, 0.95); await wait(10); log($("[data-an-text]") === null, types());
        pick("text"); pointer("pointerdown", b().left + 20, b().top + 20, canvas); type($("[data-an-text]"), "Crack"); key("Enter"); await wait(10); log(S().marks[4].text);
        canvas.focus(); key("z", { ctrlKey: true }); await wait(10); log(S().marks.length); key("z", { ctrlKey: true, shiftKey: true }); await wait(10); log(S().marks.length);
        // Selected, moved with the keys and deleted.
        pick("select"); drag(0.25, 0.6, 0.25, 0.6); key("ArrowRight"); await wait(10); log(S().marks[0].points[0][0]); key("Delete"); await wait(10); log(types());
"##,
        &["arrow,blur true", "true true", "40", "true arrow,blur,measure,measure", "Crack", "4", "5", "0.11", "blur,measure,measure,text"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn permissions_are_asked_for_and_explained() {
    check(
        "permissions",
        r##"
state: { p: null, ok: false }
root: { type: ui-permission-manager, props: { bind: p, bindReady: ok, permissions: [ { name: camera, required: true, reason: Photos }, { name: geolocation, label: Where you are } ] } }
"##,
        r##"
        // Stand-ins for a device's own answers.
        const answers = { camera: "prompt", geolocation: "denied" };
        window.__irenderDevice.permission; Object.defineProperty(navigator, "permissions", { configurable: true, value: { query: async ({ name }) => ({ state: answers[name], addEventListener() {} }) } });
        Object.defineProperty(navigator, "mediaDevices", { configurable: true, value: { getUserMedia: async () => { answers.camera = "granted"; return { getTracks: () => [] }; } } });
        Object.defineProperty(window, "isSecureContext", { configurable: true, value: true });
        document.dispatchEvent(new Event("visibilitychange")); await until(() => S().p?.geolocation === "denied" && S().p?.camera === "prompt");
        log(S().p, S().ok, $$("[data-w-tag]").map((t) => t.textContent));
        $("[data-perm=camera] button").click(); await until(() => S().ok); log(S().p.camera, $("[data-perm=camera] button"));
        $("[data-perm=geolocation] button").click(); await wait(20); log($("dialog").open, $$("dialog li").length > 1, $("dialog h2").textContent);
"##,
        &[r#"{"camera":"prompt","geolocation":"denied"} false ["Not asked yet","Blocked"]"#, "granted null", "true true Allow Location"],
    );
}

/// A device that's where the test says: `at(lat, lng, accuracy)`.
const GEO: &str = r##"
        Object.defineProperty(window, "isSecureContext", { configurable: true, value: true });
        let now = null; const watchers = new Map();
        const pos = () => ({ coords: { latitude: now[0], longitude: now[1], accuracy: now[2] ?? 5, altitude: null, heading: null, speed: null }, timestamp: Date.now() });
        const at = (...p) => { now = p; watchers.forEach((f) => f(pos())); };
        Object.defineProperty(navigator, "geolocation", { configurable: true, value: { getCurrentPosition: (ok, no) => setTimeout(() => (now ? ok(pos()) : no({ code: 1, PERMISSION_DENIED: 1 })), 1), watchPosition: (ok, no) => { const id = watchers.size + 1; watchers.set(id, ok); if (now) setTimeout(() => watchers.has(id) && ok(pos()), 1); return id; }, clearWatch: (id) => watchers.delete(id) } });
"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn location_checkin_and_route_follow_the_device() {
    check(
        "geo",
        r##"
state: { loc: null, visit: null, log: [], photo: null, route: null, heading: 350 }
root:
  type: stack
  children:
    - { type: ui-location, props: { bind: loc, manual: true }, attrs: { id: loc } }
    - { type: ui-geo-checkin, props: { bind: visit, bindLog: log, ready: "{{ photo != null }}", photo: "{{ photo }}", places: [ { id: a, name: Depot, lat: 10, lng: 20, radius: 100 }, { id: b, name: Yard, points: [ [10.01, 20.01], [10.02, 20.01], [10.02, 20.02], [10.01, 20.02] ] } ] }, attrs: { id: ci } }
    - { type: ui-route-tracker, props: { bind: route, awake: false }, attrs: { id: rt } }
    - { type: ui-compass, props: { heading: "{{ heading }}", from: "{{ loc }}", target: { lat: 11, lng: 20 } } }
"##,
        &format!(
            "{GEO}{}",
            r##"
        $("#loc [data-locate]").click(); await wait(20); log($("#loc [data-w-error]").textContent, $$("#loc button").length);
        at(10.005, 20, 8); $("#loc [data-locate]").click(); await wait(20); log(S().loc.lat, S().loc.accuracy, $("#loc [data-w-tag]").textContent, $("#loc a").href.includes("10.005"));
        const f = $("#loc input"); f.focus(); f.value = "10"; f.dispatchEvent(new Event("change")); await wait(10); log(S().loc.lat, S().loc.typed);
        const ci = () => $("#ci [data-check]");
        log("gated", ci().disabled); root.__irender.set("photo", "p1"); await wait(10);
        ci().click(); await wait(30); log(S().visit, $("#ci [data-w-error]").textContent);
        at(10.015, 20.015, 8); ci().click(); await wait(30); log(S().visit.place.name, S().visit.in.inside, S().visit.in.photo, ci().getAttribute("data-check"));
        at(10.05, 20.05, 300); ci().click(); await wait(12100); log(!!S().visit, $("#ci [data-w-error]").textContent.includes("300 m"));
        at(10.05, 20.05, 8); ci().click(); await wait(30); log(S().visit, S().log.length, S().log[0].out.inside, S().log[0].out.distance > 1000);
        at(10, 20, 5); $("#rt [data-start]").click(); await wait(10);
        at(10, 20, 5); at(10.00001, 20, 5); at(10.001, 20, 5); at(10.5, 20, 900); await wait(10); log(S().route.state, S().route.points.length, S().route.distance);
        $("#rt [data-pause]").click(); at(10.1, 20, 5); await wait(10); $("#rt [data-start]").click(); at(10.1, 20, 5); at(10.101, 20, 5); await wait(10); log(S().route.points.length, S().route.distance);
        $("#rt [data-finish]").click(); await wait(10); log(S().route.state, watchers.size);
        log($("[data-dial-value]").textContent, $$("[data-w-note]").at(-1).textContent); root.__irender.set("heading", 100); await wait(10); log($("[data-dial-value]").textContent);
"##
        ),
        &[
            "Location isn't allowed for this site 2",
            "10.005 8 ± 8 m true",
            "10 true",
            "gated true",
            "null You're 456 m from Depot. Come closer to check in.",
            "Yard true p1 out",
            "true true",
            "null 1 false true",
            "recording 2 111",
            "4 222",
            "done 0",
            "350° N 111.2 km away, at 0°",
            "100° E",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn field_map_markers_drawing_and_moving() {
    check(
        "map",
        r##"
state: { sel: null, view: null, drawn: [], pt: null, rows: [ { id: a, lat: 10, lng: 20, label: A }, { id: b, lat: 10.0002, lng: 20.0002, label: B }, { id: c, lat: 12, lng: 23, label: C } ] }
root: { type: ui-field-map, style: { width: 600 }, props: { tiles: none, cluster: true, markers: "{{ rows }}", bindSelected: sel, bindView: view, bindPoint: pt, bindDrawn: drawn, draw: [line, circle, marker], height: 400 } }
"##,
        r##"
        await until(() => $$("[data-map-pin]").length); await wait(200);
        const pane = $("[data-map-pane]"), r = pane.getBoundingClientRect();
        const click = (x, y) => { const t = document.elementFromPoint(r.left + x, r.top + y); pointer("pointerdown", r.left + x, r.top + y, t); pointer("pointerup", r.left + x, r.top + y, t); t.dispatchEvent(new MouseEvent("click", { clientX: r.left + x, clientY: r.top + y, bubbles: true, cancelable: true })); };
        log($$("[data-map-pin]").map((p) => p.getAttribute("data-map-pin")), $("[data-map-cluster]").textContent, Math.round(S().view.lat), Math.round(S().view.lng * 10) / 10);
        $("[data-map-pin=c]").click(); await wait(20); log(S().sel, $("[data-map-pop] strong").textContent, $("[data-map-pin=c]").getAttribute("aria-pressed"));
        $("[data-map-cluster]").click(); await wait(200); log($$("[data-map-pin]").map((p) => p.getAttribute("data-map-pin")).sort().join(), !!$("[data-map-cluster]"), S().view.zoom > 14);
        click(300, 350); await wait(20); log(S().sel, !!S().pt && Math.abs(S().pt.lat - 10) < 0.01);
        // A drag moves it, and isn't a click.
        const before = S().view.lng; pointer("pointerdown", r.left + 300, r.top + 350, pane); pointer("pointermove", r.left + 200, r.top + 350, pane); pointer("pointerup", r.left + 200, r.top + 350, pane); await wait(200); log(S().view.lng > before);
        pane.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
        $("[data-draw=line]").click(); click(100, 300); click(200, 320); click(300, 300); await wait(20); pane.focus(); key("Enter"); await wait(20); log(S().drawn.map((d) => [d.type, d.points.length]), S().sel === S().drawn[0].id);
        $("[data-draw=circle]").click(); pointer("pointerdown", r.left + 400, r.top + 300, pane); pointer("pointermove", r.left + 440, r.top + 300, pane); pointer("pointerup", r.left + 440, r.top + 300, pane); await wait(20); log(S().drawn[1].type, S().drawn[1].radius > 1, $$("[data-map-handle]").length);
        pane.focus(); key("Delete"); await wait(20); log(S().drawn.length, S().sel);
        root.__irender.set("rows", [{ id: "z", lat: 10, lng: 20 }]); await wait(20); log($$("[data-map-pin]").length);
"##,
        &[r#"["c"] 2 11 21.5"#, "c C true", "a,b,c false true", "null true", "true", r#"[["line",3]] true"#, "circle true 1", "1 null", "1"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn plan_layouts_seats_and_floors() {
    check(
        "plan-layouts",
        r##"
state: { ids: [], picked: [], total: 0, floor: main, kind: cinema, one: null }
root:
  type: stack
  children:
    - type: ui-plan
      attrs: { id: plane }
      props:
        bindFloor: floor
        floors:
          - { id: up, label: Up, layout: { kind: aircraft, first: 70, rows: 3, seats: "A C", status: Free } }
          - { id: main, label: Main, items: [ { id: stairs, label: Stairs, x: 0, y: 0, to: up } ], layout: { kind: aircraft, rows: 10, seats: "ABC DEF", skip: [4], missing: [1A], status: Free, sections: [ { name: Front, from: 1, to: 2, price: 50 }, { name: Back, from: 3, to: 10, price: 10 } ] } }
        data: { 1B: { status: Taken } }
        bind: ids
        bindPicked: picked
        bindTotal: total
        multiple: true
        max: 2
        available: [Free]
        search: true
        legend: true
    - { type: ui-plan, attrs: { id: room }, props: { layout: { kind: "{{ kind }}", rows: 3, seats: 6, style: banquet, tables: 2, blocks: 8, tiers: [ {} ], bays: 4, pods: 2 }, bind: one } }
"##,
        r##"
        const seats = (id) => $$(`#${id} [data-pl-item]`).map((s) => s.getAttribute("data-pl-item"));
        const at = (id) => $(`#plane [data-pl-item="${id}"]`), press = (id) => at(id).dispatchEvent(new MouseEvent("click", { bubbles: true }));
        log(seats("plane").length, seats("plane").slice(0, 3).join(), seats("plane").includes("4A"), at("1B").getAttribute("aria-disabled"), $$("#plane [data-pl-floors] button").map((b) => b.textContent + b.getAttribute("aria-pressed")));
        press("1B"); press("1C"); await wait(10); log(S().ids, S().total); press("5F"); press("6F"); await wait(10); log(S().ids, S().total, S().picked.map((p) => [p.section, p.row, p.floor]));
        press("1C"); await wait(10); log(S().ids, S().total);
        // Stairs go to another floor, whose seats add to those picked.
        press("stairs"); await wait(20); log(S().floor, seats("plane").join()); press("70A"); await wait(10); log(S().ids, S().picked.map((p) => p.floor));
        const find = $("#plane [data-pl-search] input"); type(find, "9d"); $("#plane [data-pl-search] li button").click(); await wait(20); log(S().floor, document.activeElement.getAttribute("data-pl-item"), $$("#plane [tabindex='0'][data-pl-item]").length);
        key("PageUp"); await wait(10); log(S().floor);
        const kinds = {}; for (const k of ["cinema", "auditorium", "theatre", "stadium", "hall", "parking", "office", "bus", "train", "nothing"]) { root.__irender.set("kind", k); await wait(5); kinds[k] = seats("room").length; } log(kinds);
        root.__irender.set("kind", "cinema"); await wait(10); $('#room [data-pl-item="B2"]').dispatchEvent(new MouseEvent("click", { bubbles: true })); await wait(10); log(S().one, $('#room [data-pl-item="B2"]').getAttribute("aria-label"));
"##,
        &[
            r#"54 1B,1C,1D false true ["Upfalse","Maintrue"]"#,
            r#"["1C"] 50"#,
            r#"["1C","5F"] 60 [["Front","1","main"],["Back","5","main"]]"#,
            r#"["5F"] 10"#,
            "up 70A,70C,71A,71C,72A,72C",
            r#"["5F","70A"] ["main","up"]"#,
            "main 9D 1",
            "up",
            r#"{"cinema":18,"auditorium":56,"theatre":48,"stadium":8,"hall":14,"parking":12,"office":8,"bus":18,"train":18,"nothing":0}"#,
            "B2 B2",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sharing_contacts_badge_and_push() {
    check(
        "share",
        r##"
state: { n: 3, people: [], push: null, done: false, got: null }
root:
  type: stack
  children:
    - { type: ui-app-badge, props: { count: "{{ n }}" } }
    - { type: ui-share, attrs: { id: share }, props: { title: T, text: Look, sheet: own, targets: [ { label: Bad, href: "javascript:alert(1)" }, { label: Chat, href: "https://chat.test/?t={text}" } ] } }
    - { type: ui-contact-picker, attrs: { id: cp }, props: { bind: people, multiple: true } }
    - { type: ui-push-opt-in, attrs: { id: push }, props: { bind: push, when: "{{ done }}", topics: [ a, { value: b, label: B, on: false } ] } }
    - { type: ui-share-target, props: { bind: got, acceptLabel: Keep }, on: { accept: { action: set_state, key: n, value: 9 } } }
"##,
        r##"
        log(document.title); root.__irender.set("n", 0); await wait(10); log(document.title); root.__irender.set("n", 120); await wait(10); log(document.title);
        $("#share [data-share]").click(); await wait(10); log($$("#share dialog a").map((a) => [a.textContent, a.getAttribute("href")]), $$("#share dialog button").map((b) => b.textContent)); $("#share dialog").close();
        const d = new DataTransfer(); d.items.add(new File(["BEGIN:VCARD\nFN:Ada Love\n lace\nTEL;TYPE=CELL:+44 1\nEND:VCARD\nBEGIN:VCARD\nN:Hopper;Grace;;;\nEMAIL:g@x.test\nEND:VCARD\n"], "a.vcf"));
        const i = $("#cp input[type=file]"); i.files = d.files; i.dispatchEvent(new Event("change", { bubbles: true })); await until(() => S().people.length === 2); log(S().people);
        $("#cp [data-type]").click(); type($("#cp input[name=name]"), "Alan"); $("#cp form").requestSubmit(); await wait(10); log(S().people[2]); $("#cp [data-w-list] button").click(); await wait(10); log(S().people.map((p) => p.name));
        log($("#push").hidden); root.__irender.set("done", true); await wait(10); await until(() => S().push); log(S().push?.topics);
        root.__irender.set("got", { title: "A trip", text: "see https://x.test", url: "javascript:alert(1)", files: [] }); await wait(10); log($("[data-w-card] [data-w-title]") && $$("[data-w-card]").at(-1).textContent, $$("[data-w-card] a").map((a) => a.getAttribute("href"))); $("[data-accept]").click(); await wait(10); log(S().n);
"##,
        &[
            "(3) test",
            "test",
            "(99+) test",
            r#"[["Email","mailto:?subject=T&body=Look"],["Chat","https://chat.test/?t=Look"]] ["Copy the text","Cancel"]"#,
            r#"[{"name":"Ada Lovelace","tel":["+44 1"],"email":[]},{"name":"Grace Hopper","tel":[],"email":["g@x.test"]}]"#,
            r#"{"name":"Alan","tel":[],"email":[]}"#,
            r#"["Grace Hopper","Alan"]"#,
            "true",
            r#"["a"]"#,
            r#"A tripsee https://x.testKeepLeave it ["https://x.test"]"#,
            "9",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn app_lock_asks_for_the_pin() {
    check(
        "lock",
        r##"
state: { locked: false, reset: false }
root: { type: ui-app-lock, props: { methods: [pin], bindLocked: locked, name: test, tries: 2, after: 0, reset: "{{ reset }}" } }
"##,
        r##"
        // Pages from a file may keep nothing, and needn't here.
        const box = new Map(), db = window.__irenderOffline.db; db.put = async (s, k, v) => void box.set(k, v); db.del = async (s, k) => void box.delete(k);
        const tap = async (d) => { for (const c of d) $(`[data-k="${c}"]`).click(); await wait(20); };
        if (!crypto.subtle) { log("skip"); throw new Error("no crypto"); }
        $("[data-set=pin]").click(); await wait(10); log($("dialog h2").textContent); await tap("1234"); log($("dialog [data-w-note]").textContent);
        await tap("2580"); log($("dialog h2").textContent); await tap("2581"); log($("dialog [data-w-note]").textContent); await tap("2580"); await tap("2580"); await until(() => box.size); await wait(20); log($("dialog").open, box.get("lock:test").pin.includes("2580"));
        $("[data-lock]").click(); await wait(20); log(S().locked, $("dialog").open, $$("[data-lk-dots] i").length);
        await tap("1111"); await until(() => $("dialog [data-w-note]").textContent); log($("dialog [data-w-note]").textContent); await tap("1111"); await until(() => $("dialog [data-w-note]").textContent.startsWith("Too many")); log($("[data-k='1']").disabled, S().locked);
        $("dialog").dispatchEvent(new Event("cancel", { cancelable: true })); log($("dialog").open);
        root.__irender.set("reset", true); await wait(30); log(S().locked, $("dialog").open, box.size);
"##,
        &["Choose a PIN", "That one's too easy to guess", "Enter it again", "They weren't the same. Choose again.", "false false", "true true 4", "Wrong PIN", "true true", "true", "false false 0"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn swiping_speech_and_wake_lock() {
    check(
        "swipe",
        r##"
state: { last: "", said: before, read: "", on: false, held: false }
root:
  type: stack
  children:
    - type: ui-swipe-actions
      attrs: { id: sw }
      style: { width: 300 }
      props: { label: Ada, start: [ { value: read, label: Read } ], end: [ { value: later, label: Later }, { value: delete, label: Delete, tone: danger } ] }
      on: { delete: { action: set_state, key: last, value: deleted }, read: { action: set_state, key: last, value: read }, later: { action: set_state, key: last, value: later } }
      children: [ { type: element, style: { height: 50 } } ]
    - { type: ui-voice-input, attrs: { id: vo }, props: { bind: said } }
    - { type: ui-read-aloud, attrs: { id: ra }, props: { text: "One here. Two here!", bindState: read } }
    - { type: ui-wake-lock, attrs: { id: wl }, props: { bind: on, bindHeld: held } }
"##,
        r##"
        const row = $("[data-sw-in]"), r = row.getBoundingClientRect();
        const swipe = async (dx) => { pointer("pointerdown", r.left + 150, r.top + 20, row); pointer("pointermove", r.left + 150 + dx / 2, r.top + 20, row); pointer("pointermove", r.left + 150 + dx, r.top + 20, row); pointer("pointerup", r.left + 150 + dx, r.top + 20, row); await wait(20); };
        await swipe(-20); log(row.style.transform, S().last);
        await swipe(-100); log(row.style.transform !== "", $("[data-sw-acts=end]").hasAttribute("data-on")); $("[data-act=later]").click(); await wait(10); log(S().last, row.style.transform);
        await swipe(-260); log(S().last); await swipe(60); $("[data-act=read]").click(); await wait(10); log(S().last);
        $("[data-sw-more]").click(); log(document.activeElement.getAttribute("data-act"), $("[data-sw-more]").getAttribute("aria-label")); key("Escape"); log(row.style.transform);
        // Stand-ins for the device's speech and screen lock.
        let sr; window.SpeechRecognition = window.webkitSpeechRecognition = class { start() { sr = this; } stop() { this.onend(); } };
        const heard = (t, isFinal) => sr.onresult({ results: [Object.assign([{ transcript: t }], { isFinal })] });
        $("[data-speak]").click(); heard("hello", false); log($("[data-vi-text]").textContent, S().said); heard("hello there", true); await wait(10); log(S().said, $("[data-speak]").getAttribute("aria-pressed")); $("[data-speak]").click(); await wait(10); log($("[data-speak]").getAttribute("aria-pressed"));
        const said = []; let now; window.SpeechSynthesisUtterance = class { constructor(t) { this.text = t; } };
        Object.defineProperty(window, "speechSynthesis", { configurable: true, value: { speak: (u) => { now = u; said.push(u.text); }, cancel() {}, getVoices: () => [] } });
        $("[data-read]").click(); await wait(10); log(S().read, said); now.onend(); await wait(10); $("[data-read]").click(); await wait(10); log(S().read); $("[data-read]").click(); await wait(10); now.onend(); await wait(10); log(S().read, said);
        let gone; Object.defineProperty(navigator, "wakeLock", { configurable: true, value: { request: async () => ({ addEventListener: (n, f) => (gone = f), release: async () => gone() }) } });
        root.__irender.set("on", true); await wait(20); log(S().held); root.__irender.set("on", false); await wait(20); log(S().held);
"##,
        &[" ", "true true", "later ", "deleted", "read", "later Actions for Ada", "", "hello before", "before hello there true", "false", r#"playing ["One here."]"#, "paused", r#"stopped ["One here.","Two here!","Two here!"]"#, "true", "false"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn documents_are_found_straightened_and_made_a_pdf() {
    check(
        "scan-doc",
        r##"
state: { pages: [], pdf: null }
root: { type: ui-document-scanner, props: { bind: pages, bindPdf: pdf, filter: bw, paper: a4 } }
"##,
        r##"
        // A white page, turned a little, on a dark desk, with a black square near its top left.
        const c = document.createElement("canvas"); c.width = 400; c.height = 300; const g = c.getContext("2d");
        g.fillStyle = "#222"; g.fillRect(0, 0, 400, 300); g.fillStyle = "#f4f4f0"; g.beginPath(); [[120, 40], [300, 60], [290, 260], [90, 250]].forEach((p) => g.lineTo(...p)); g.fill(); g.fillStyle = "#000"; g.fillRect(140, 70, 30, 30);
        const Z = window.__irenderScan, q = Z.edges(c, 400, 300).map(([x, y]) => [Math.round(x * 400), Math.round(y * 300)]);
        log(q.every((p, i) => Math.hypot(p[0] - [[120, 40], [300, 60], [290, 260], [90, 250]][i][0], p[1] - [[120, 40], [300, 60], [290, 260], [90, 250]][i][1]) < 8));
        window.createImageBitmap = async () => c; const d = new DataTransfer(); d.items.add(new File(["x"], "a.png", { type: "image/png" })); const i = $("input[type=file]"); i.files = d.files; i.dispatchEvent(new Event("change", { bubbles: true })); await wait(60);
        log($$("[data-ds-corner]").filter((x) => !x.hidden).length, $("[data-ds-view]").hidden);
        const before = $("[data-ds-corner='0']").style.left; $("[data-ds-corner='0']").focus(); key("ArrowRight"); log($("[data-ds-corner='0']").style.left !== before);
        $("[data-f=find]").click(); $("[data-accept]").click(); await wait(20); await until(() => S().pdf); const p = S().pages[0];
        log(S().pages.length, Math.abs(p.width - 200) < 14, Math.abs(p.height - 212) < 16, p.url.startsWith("data:image/jpeg"), S().pdf.pages, S().pdf.name);
        // The square is black on white, where it was on the page.
        const im = new Image(); im.src = p.url; await until(() => im.complete && im.naturalWidth); const h = document.createElement("canvas").getContext("2d"); h.canvas.width = p.width; h.canvas.height = p.height; h.drawImage(im, 0, 0); const at = (x, y) => h.getImageData(Math.round(x * p.width), Math.round(y * p.height), 1, 1).data[0];
        log(at(0.7, 0.7) > 220, at(0.18, 0.21) < 120);
        const pdf = atob(S().pdf.url.split(",")[1]); log(pdf.startsWith("%PDF-1.4"), /\/MediaBox \[0 0 595.28 841.89\]/.test(pdf), pdf.trim().endsWith("%%EOF"));
        // The table of where each part starts is right.
        const x = Number(/startxref\n(\d+)/.exec(pdf)[1]); log(pdf.slice(x, x + 4), [...pdf.slice(x).matchAll(/(\d{10}) 00000 n/g)].every((m, k) => pdf.slice(Number(m[1])).startsWith(`${k + 1} 0 obj`)));
        $$("[data-ds-pages] button").at(-1).click(); await wait(20); log(S().pages.length, S().pdf);
"##,
        &["true", "4 false", "true", "1 true true true 1 scan.pdf", "true true", "true true true", "xref true", "0 null"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn serial_bluetooth_usb_nfc_and_files() {
    // Stand-ins for the devices, there before the page's own scripts.
    const DEVICES: &str = r##"<script>
      Object.defineProperty(window, "isSecureContext", { configurable: true, value: true });
      const own = (n, value) => Object.defineProperty(navigator, n, { configurable: true, value });
      window.sent = [];
      const enc = new TextEncoder(), dec = new TextDecoder();
      own("serial", { requestPort: async () => { let push; const readable = new ReadableStream({ start: (c) => (push = c) }); window.serialPush = (t) => push.enqueue(enc.encode(t)); return { readable, writable: { getWriter: () => ({ write: async (b) => sent.push(dec.decode(b)), releaseLock() {} }) }, open: async () => {}, close: async () => {} }; } });
      const ch = { properties: { read: true }, listeners: [], addEventListener(n, f) { this.listeners.push(f); }, startNotifications: async () => {}, readValue: async () => new DataView(Uint8Array.of(0, 72).buffer), writeValueWithResponse: async (b) => sent.push([...b]) };
      window.beat = (n) => ch.listeners.forEach((f) => f({ target: { value: new DataView(Uint8Array.of(0, n).buffer) } }));
      own("bluetooth", { requestDevice: async (o) => ((window.asked = JSON.stringify(o)), (window.belt = { name: "Belt", gatt: { connect: async () => window.belt.gatt, disconnect: () => window.belt.ongattserverdisconnected(), getPrimaryService: async () => ({ getCharacteristic: async () => ch }) } })) });
      let answer; own("usb", { addEventListener() {}, requestDevice: async () => ({ opened: true, productName: "Printer", vendorId: 1, productId: 2, open: async () => {}, close: async () => {}, configuration: { interfaces: [{ interfaceNumber: 0, alternate: { endpoints: [{ direction: "in", type: "bulk", endpointNumber: 1, packetSize: 64 }, { direction: "out", type: "bulk", endpointNumber: 2 }] } }] }, claimInterface: async () => {}, transferIn: () => new Promise((r) => (answer = r)), transferOut: async (n, b) => (sent.push(dec.decode(b)), { status: "ok" }) }) });
      window.usbSay = (t) => answer({ status: "ok", data: new DataView(enc.encode(t).buffer) });
      window.NDEFReader = class { async scan() { window.tag = (r) => this.onreading({ serialNumber: "04:a1", message: { records: r } }); } async write(m) { sent.push(m.records[0]); } };
      window.showOpenFilePicker = window.showSaveFilePicker = undefined;
    </script>"##;
    let doc = r##"
state: { w: null, log: null, port: null, hr: null, usb: null, tag: null, note: "a", file: null, dirty: false }
root:
  type: stack
  children:
    - { type: ui-serial, attrs: { id: se }, props: { bind: w, bindLog: log, bindState: port, parse: number, send: true, commands: [ { label: Zero, text: Z } ] } }
    - { type: ui-bluetooth, attrs: { id: bl }, props: { bind: hr, read: [ { key: rate, service: heart_rate, characteristic: "0x2a37", as: heart-rate, notify: true, unit: bpm } ], commands: [ { label: Reset, service: heart_rate, characteristic: heart_rate_control_point, value: 1, as: uint8 } ] } }
    - { type: ui-usb, attrs: { id: us }, props: { bind: usb, filters: [], commands: [ { label: Print, value: "Hi {{ w }}" } ] } }
    - { type: ui-nfc, attrs: { id: nf }, props: { bind: tag, write: "https://x.test" } }
    - { type: ui-file-handler, attrs: { id: fi }, props: { bind: file, save: "{{ note }}", bindDirty: dirty } }
"##;
    let driver = r##"
        const go = async (s) => { $(s).click(); await wait(30); };
        await go("#se [data-connect]"); log(S().port); serialPush("12."); serialPush("5 g\r\n-3 g\npart"); await wait(30); log(S().w, S().log.map((l) => l.text));
        await go("#se [data-command]"); type($("#se form input"), "W"); $("#se form").requestSubmit(); await wait(20); log(sent.splice(0)); await go("#se [data-disconnect]"); log(S().port);
        await go("#bl [data-connect]"); log(asked, S().hr, $("#bl dd").textContent); beat(80); await wait(10); log(S().hr.rate, $("#bl dd").textContent); await go("#bl [data-command]"); log(sent.splice(0)); await go("#bl [data-disconnect]"); log($("#bl [data-w-tag]").textContent);
        await go("#us [data-connect]"); usbSay("ok"); await wait(20); log(S().usb, $("#us").textContent.includes("Printer")); await go("#us [data-command]"); log(sent.splice(0));
        await go("#nf [data-read]"); tag([{ recordType: "text", data: new TextEncoder().encode("shelf 4") }]); await wait(10); log(S().tag.text, S().tag.serial); await go("#nf [data-stop]"); await go("#nf [data-write]"); log(sent.splice(0));
        log(S().dirty); const d = new DataTransfer(); d.items.add(new File(["a"], "n.txt")); const i = $("#fi input"); i.files = d.files; i.dispatchEvent(new Event("change")); await until(() => S().file); await wait(10); log(S().file.name, S().file.text, S().dirty); root.__irender.set("note", "b"); await wait(10); log(S().dirty);
"##;
    let Some(result) = run_with("hardware", doc, driver, DEVICES) else { return };
    let result = result.replace("&lt;", "<").replace("&gt;", ">");
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "connected",
            r#"-3 ["12.5 g","-3 g"]"#,
            r#"["Z\n","W\n"]"#,
            "disconnected",
            r#"{"filters":[{"services":["heart_rate"]}]} {"rate":72} 72 bpm"#,
            "80 80 bpm",
            "[[1]]",
            "Not connected",
            "ok true",
            r#"["Hi -3"]"#,
            "shelf 4 04:a1",
            r#"[{"recordType":"url","data":"https://x.test"}]"#,
            "true",
            "n.txt a false",
            "true"
        ],
        "\n{result}"
    );
}
