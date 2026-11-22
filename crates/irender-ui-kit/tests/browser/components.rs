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
      const until = async (f) => {{
        for (let i = 0; i < 40000 && !f(); i++) await new Promise((r) => {{ const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); }});
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
        $("[data-app-screen=home]").scrollTop = 120; await wait(300);
        root.__irender.set("n", 3); $$("[data-app-tab]")[1].click(); await wait(30);
        log(title(), shown(), tabs());
        root.__irender.set("at", "one"); await wait(30);
        log(title(), shown(), tabs(), $("[data-app-back]").hidden);
        $("[data-app-back]").click(); await wait(30); log(S().at);
        $$("[data-app-tab]")[0].click(); await wait(300); log(S().at, $("[data-app-screen=home]").scrollTop);
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
