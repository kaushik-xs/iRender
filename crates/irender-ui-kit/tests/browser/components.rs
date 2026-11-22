//! Forms, tables, pickers, media, developer tools, guidance and assistant components, each
//! page driven in the browser.

use super::drive;
use irender_core::Format;
use irender_html::Engine;

/// Renders `doc` (YAML, without its `version`), runs `driver` in it and gives the lines it
/// logged. `log` takes anything; `until` waits for what doesn't follow the page's clock.
fn run(name: &str, doc: &str, driver: &str) -> Option<String> {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&format!("version: 1\n{doc}"), Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let script = format!(
        r##"<script>
    window.addEventListener("load", async () => {{
      const out = [];
      const log = (...a) => out.push(a.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).join(" ").replace(/[\u202f\u00a0]/g, " "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => {{
        for (let i = 0; i < 20000 && !f(); i++) await new Promise((r) => {{ const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); }});
      }};
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
