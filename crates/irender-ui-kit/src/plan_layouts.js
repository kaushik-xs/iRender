// irender-ui-kit: the layouts `ui-plan` knows (`window.__irenderPlan(layout)`): a few words
// about a cabin, a cinema, an auditorium or a hall become its seats, each an item with a
// place, and marks (walls, screens, row numbers) that can't be picked. Units are the
// plan's own; a seat is 26 wide.
(() => {
  if (window.__irenderPlan) return;
  const S = 26, list = (v) => (Array.isArray(v) ? v : []), isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const int = (v, d, lo, hi) => Math.min(hi, Math.max(lo, Number.isFinite(Number(v)) && v !== null && v !== "" ? Math.round(Number(v)) : d));
  const L = (t, v) => window.__irender.label(t, v);
  // A, B, … Z, AA, AB: rows by letter, leaving out those in `skip`.
  function letters(n, skip = "") {
    const base = [..."ABCDEFGHIJKLMNOPQRSTUVWXYZ"].filter((c) => !String(skip).toUpperCase().includes(c)), out = [];
    for (let i = 0; i < n; i++) out.push(i < base.length ? base[i] : base[Math.floor(i / base.length) - 1] + base[i % base.length]);
    return out;
  }
  const names = (rows, skip, numbers) => (Array.isArray(rows) ? rows.map(String) : numbers ? Array.from({ length: int(rows, 10, 1, 120) }, (_, i) => String(i + 1)) : letters(int(rows, 10, 1, 120), skip));
  const rr = (x, y, w, h, r) => `M${x + r} ${y}h${w - 2 * r}a${r} ${r} 0 0 1 ${r} ${r}v${h - 2 * r}a${r} ${r} 0 0 1 ${-r} ${r}h${2 * r - w}a${r} ${r} 0 0 1 ${-r} ${-r}v${2 * r - h}a${r} ${r} 0 0 1 ${r} ${-r}z`;
  const seat = (id, label, x, y, o = {}) => ({ id, label, shape: "seat", x, y, width: S, height: S, ...o });
  const text = (t, x, y, o = {}) => ({ text: String(t), x, y, ...o });
  // What a section says about its rows goes on their seats.
  const from = (s) => (s ? Object.fromEntries(["section", "price", "tone", "color", "status"].map((k) => [k, k === "section" ? s.name ?? s.section : s[k]]).filter(([, v]) => v !== undefined)) : {});
  const sectionOf = (o, row) => list(o.sections).find((s) => (Array.isArray(s.rows) ? s.rows.map(String).includes(String(row)) : Number(row) >= Number(s.from ?? -1e9) && Number(row) <= Number(s.to ?? 1e9) && (s.from !== undefined || s.to !== undefined)));
  // Everything within a margin of the top left corner.
  function settle(items, marks, pad = 24) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    const see = (x, y) => ((x0 = Math.min(x0, x)), (y0 = Math.min(y0, y)), (x1 = Math.max(x1, x)), (y1 = Math.max(y1, y)));
    for (const i of items) i.points ? i.points.forEach((p) => see(p[0] + (i.x ?? 0), p[1] + (i.y ?? 0))) : (see(i.x, i.y), see(i.x + i.width, i.y + (i.height ?? i.width)));
    for (const m of marks) (m.box ? (see(m.box[0], m.box[1]), see(m.box[2], m.box[3])) : m.x !== undefined ? (see(m.x - 8, m.y - 8), see(m.x + 8, m.y + 8)) : null);
    if (x0 === Infinity) x0 = y0 = 0, x1 = y1 = 100;
    const dx = pad - x0, dy = pad - y0;
    for (const i of items) (i.x = +((i.x ?? 0) + dx).toFixed(2)), (i.y = +((i.y ?? 0) + dy).toFixed(2));
    for (const m of marks) m.path ? (m.dx = dx, m.dy = dy) : ((m.x += dx), (m.y += dy));
    return { items, marks, width: Math.ceil(x1 - x0 + pad * 2), height: Math.ceil(y1 - y0 + pad * 2) };
  }

  // ---------------------------------------------------------------- cabins
  // Seats in rows with aisles where the letters have spaces: "ABC DEF". Laid out front up,
  // then turned so the front is to the left unless `direction` is `down`.
  function cabin(o, kind) {
    const items = [], marks = [], skip = list(o.skip).map(String), missing = new Set(list(o.missing).map(String)), exits = new Set(list(o.exits).map(String));
    const last = int(o.rows, kind === "aircraft" ? 30 : 12, 1, 150), AISLE = kind === "train" ? 30 : 26;
    const rows = Array.from({ length: last }, (_, i) => i + int(o.first, 1, 0, 9999)).filter((r) => !skip.includes(String(r)));
    const plan = (r, i) => {
      const s = sectionOf(o, r), given = (i === rows.length - 1 && o.last) || s?.seats || o.seats || "ABC DEF", n = Number(given);
      // A number of them has one aisle, in the middle.
      const pattern = n > 0 ? letters(int(n, 6, 1, 20)).map((c, k) => (k === Math.ceil(n / 2) ? ` ${c}` : c)).join("") : String(given).trim();
      const k = Math.min(2, Math.max(0.8, Number(s?.size) || 1));
      return { s, pattern, k };
    };
    const wide = (p) => [...p.pattern].reduce((w, c) => w + (c === " " ? AISLE : S * p.k + 4), 0) - 4;
    const W = Math.max(...rows.map((r, i) => wide(plan(r, i)))), facing = !!o.facing;
    let y = 0, was = null;
    const rowY = {};
    rows.forEach((r, i) => {
      const p = plan(r, i), w = wide(p), key = String(r);
      for (const b of list(o.blocks).filter((b) => String(b.before) === key)) y += block(b, y);
      if (was && p.s !== was) {
        y += 16;
        if (p.s?.name) marks.push(text(p.s.name, W / 2, y, { size: 12, soft: true })), (y += 14);
      } else if (i === 0 && p.s?.name) marks.push(text(p.s.name, W / 2, y - 6, { size: 12, soft: true })), (y += 8);
      was = p.s;
      if (exits.has(key)) {
        y += 12;
        for (const side of [0, 1]) marks.push(text(L("Exit"), side ? W + 24 : -24, y + (S * p.k) / 2, { size: 10, tone: "success", rot: 0 }));
      }
      rowY[r] = y;
      let x = (W - w) / 2, aisle = null;
      for (const c of p.pattern) {
        if (c === " ") {
          aisle ??= x - 2 + AISLE / 2;
          x += AISLE;
          continue;
        }
        const id = `${r}${c}`, turn = facing && i % 2 ? 180 : 0;
        if (!missing.has(id)) items.push(seat(id, id, x, y, { width: S * p.k, height: S * p.k, row: key, seat: c, turn, ...from(p.s), ...(exits.has(key) ? { exit: true } : {}) }));
        x += S * p.k + 4;
      }
      if (aisle !== null && o.numbers !== false) marks.push(text(r, aisle, y + (S * p.k) / 2, { size: 10, soft: true }));
      // Two rows facing each other share a table.
      const gap = Number(p.s?.pitch) || Number(o.pitch) || 12;
      if (facing && i % 2 === 0 && i < rows.length - 1) {
        let tx = (W - w) / 2;
        for (const part of p.pattern.split(" ")) {
          const tw = part.length * (S * p.k + 4) - 4;
          marks.push({ path: rr(tx + 3, y + S * p.k + 4, tw - 6, 14, 3), fill: true, box: [tx, y, tx + tw, y + 20] });
          tx += tw + 4 + AISLE - 4 + 4;
        }
        y += S * p.k + 22;
      } else y += S * p.k + gap;
      for (const b of list(o.blocks).filter((b) => String(b.after) === key)) y += block(b, y);
    });
    function block(b, y0) {
      const h = Number(b.height) || 30, half = (W - AISLE) / 2, side = b.side ?? "both";
      for (const [on, x] of [["left", 0], ["right", W - half]]) if (side === on || side === "both") {
        marks.push({ path: rr(x, y0 + 2, half, h - 8, 4), fill: true, box: [x, y0, x + half, y0 + h] });
        marks.push(text(b.label ?? "", x + half / 2, y0 + h / 2 - 2, { size: 10, soft: true }));
      }
      return h;
    }
    const H = y, m = 14;
    if (kind === "aircraft") {
      const n = W * 0.75, t = W * 0.9, wing = list(o.wings);
      if (wing.length === 2 && rowY[wing[0]] !== undefined && rowY[wing[1]] !== undefined) {
        const a = rowY[wing[0]], b = rowY[wing[1]] + S, out = Math.min(150, W * 0.8);
        marks.unshift({ path: `M${-m} ${a}L${-m - out} ${a + (b - a) * 0.62}V${b + 14}L${-m} ${b}Z`, ground: true, box: [-m - out, a, 0, b + 14] }, { path: `M${W + m} ${a}L${W + m + out} ${a + (b - a) * 0.62}V${b + 14}L${W + m} ${b}Z`, ground: true, box: [W, a, W + m + out, b + 14] });
      }
      marks.unshift({ path: `M${-m} ${-6}C${-m} ${-n * 0.6} ${W * 0.28} ${-n} ${W / 2} ${-n}S${W + m} ${-n * 0.6} ${W + m} -6V${H}C${W + m} ${H + t * 0.5} ${W * 0.62} ${H + t} ${W / 2} ${H + t}S${-m} ${H + t * 0.5} ${-m} ${H}Z`, ground: true, box: [-m, -n, W + m, H + t] });
    } else if (kind === "bus") {
      marks.push({ path: rr(W - S, -50, S, S, 6), fill: true, box: [W - S, -50, W, -24] }, text(L("Driver"), W - S / 2, -16, { size: 9, soft: true }), text(L("Door"), 10, -38, { size: 10, tone: "success" }));
      marks.unshift({ path: rr(-m, -66, W + m * 2, H + 66 + 6, 18), ground: true, box: [-m, -66, W + m, H + 6] });
    } else marks.unshift({ path: rr(-m, -26, W + m * 2, H + 44, 10), ground: true, box: [-m, -26, W + m, H + 18] });
    if (o.direction !== "down") turnLeft(items, marks);
    return settle(items, marks);
  }
  // The front to the left: the port side is then at the foot.
  function turnLeft(items, marks) {
    for (const i of items) [i.x, i.y, i.width, i.height, i.turn] = [i.y, -i.x - i.width, i.height, i.width, (i.turn ?? 0) + 270];
    for (const m of marks) {
      if (m.box) m.box = [m.box[1], -m.box[2], m.box[3], -m.box[0]];
      if (m.path) m.rot = 270;
      else [m.x, m.y] = [m.y, -m.x];
    }
  }

  // ---------------------------------------------------------------- rows before a screen
  function cinema(o) {
    const items = [], marks = [], rows = names(o.rows ?? 10, o.skipLetters ?? "I"), missing = new Set(list(o.missing).map(String)), chairs = new Set(list(o.wheelchair).map(String)), pairs = new Set(list(o.pairs).map(String)), cross = new Set(list(o.cross).map(String));
    const per = (r, i) => int(isObj(o.seats) ? o.seats[r] ?? o.seats.default : Array.isArray(o.seats) ? o.seats[i] : o.seats, 16, 1, 120);
    const most = Math.max(...rows.map(per)), aisles = list(o.aisles).map(Number), curve = Math.min(1, Math.max(0, Number(o.curve ?? 0.25)));
    const G = S + 5, A = 28, at = (n) => (n - 1) * G + aisles.filter((a) => a < n).length * A;
    const full = at(most) + S, mid = full / 2, bend = curve * full * 0.16;
    let y = 0;
    rows.forEach((r, i) => {
      const n = per(r, i), s = sectionOf(o, r), off = (most - n) / 2, off0 = Math.floor(off);
      // Nearer the screen towards the ends, as the rows curve round it.
      for (let k = 1; k <= n; k++) {
        const num = o.from === "right" ? n + 1 - k : k, id = `${r}${num}`;
        if (missing.has(id)) continue;
        let x = at(k + off0) + (off - off0) * G;
        if (pairs.has(r)) x += k % 2 ? 2 : -2;
        const d = (x + S / 2 - mid) / mid, yy = y - bend * d * d, turn = -Math.atan2(2 * bend * d, mid) * (180 / Math.PI);
        items.push(seat(id, id, x, yy, { row: r, seat: String(num), turn: +turn.toFixed(1), ...from(s), ...(chairs.has(id) ? { shape: "space", label: "♿" } : {}) }));
      }
      if (o.numbers !== false) for (const x of [-18, full + 18]) marks.push(text(r, x, y + S / 2 - bend, { size: 11, soft: true }));
      y += S + 10 + (cross.has(r) ? 30 : 0);
    });
    const sy = -64 - bend, sw = full * 0.84, sx = (full - sw) / 2;
    if (o.screen !== false) {
      marks.push({ path: `M${sx} ${sy + 14}Q${mid} ${sy - 12} ${sx + sw} ${sy + 14}L${sx + sw} ${sy + 22}Q${mid} ${sy - 4} ${sx} ${sy + 22}Z`, fill: true, box: [sx, sy - 12, sx + sw, sy + 24] });
      marks.push(text(o.screen === true || !o.screen ? L("Screen") : o.screen, mid, sy + 24, { size: 12, soft: true }));
    }
    for (const d of list(o.doors)) marks.push(text(`${L("Exit")}`, d === "right" ? full + 50 : -50, d === "front" ? 0 : y - S, { size: 11, tone: "success" }));
    return settle(items, marks);
  }

  // ---------------------------------------------------------------- rows round a stage
  function auditorium(o) {
    const items = [], marks = [], tiers = list(o.tiers).length ? o.tiers : [{ name: o.name, rows: o.rows ?? 12 }];
    const blocks = int(o.blocks, 3, 1, 9), angle = Math.min(300, Math.max(30, Number(o.angle) || 120)), G = S + 4, missing = new Set(list(o.missing).map(String)), chairs = new Set(list(o.wheelchair).map(String));
    let R = Math.max(120, Number(o.radius) || 190);
    const R0 = R, rad = Math.PI / 180;
    tiers.forEach((t, ti) => {
      const rows = names(t.rows ?? 6, o.skipLetters ?? "I"), tag = t.prefix ?? (ti ? String(t.name ?? ti + 1)[0].toUpperCase() : "");
      if (ti) R += Number(t.gap) || 46;
      if (t.name) marks.push(text(t.name, 0, R - 14, { size: 12, soft: true }));
      rows.forEach((r) => {
        const aisle = 34 / R / rad, span = (angle - aisle * (blocks - 1)) / blocks;
        let num = 1;
        for (let b = 0; b < blocks; b++) {
          const a0 = -angle / 2 + b * (span + aisle), n = int(Array.isArray(t.seats) ? t.seats[b] : t.seats, Math.floor((R * span * rad) / G), 1, 200), s = sectionOf(o, r) ?? list(o.sections).find((x) => Number(x.block) === b + 1);
          // Left to right as one faces the stage, which is above.
          for (let k = 0; k < n; k++) {
            const a = (a0 + span * ((k + 0.5) / n)) * rad, id = `${tag}${r}${num}`, x = Math.sin(a) * R, y = Math.cos(a) * R;
            if (!missing.has(id)) items.push(seat(id, id, x - S / 2, y - S / 2, { row: `${tag}${r}`, seat: String(num), block: b + 1, turn: +(-a / rad).toFixed(1), ...(t.name ? { section: t.name } : {}), ...(t.price !== undefined ? { price: t.price } : {}), ...(t.tone ? { tone: t.tone } : {}), ...from(s), ...(chairs.has(id) ? { shape: "space", label: "♿" } : {}) }));
            num++;
          }
          if (o.numbers !== false && b < blocks - 1) {
            const a = (a0 + span + aisle / 2) * rad;
            marks.push(text(`${tag}${r}`, Math.sin(a) * R, Math.cos(a) * R, { size: 10, soft: true }));
          }
        }
        R += S + 9;
      });
    });
    const w = R0 * 1.15, d = R0 * 0.62;
    marks.push({ path: `M${-w / 2} 0H${w / 2}V${d * 0.62}Q0 ${d * 1.12} ${-w / 2} ${d * 0.62}Z`, fill: true, box: [-w / 2, 0, w / 2, d] }, text(o.stage ?? L("Stage"), 0, d * 0.42, { size: 15, soft: true }));
    return settle(items, marks);
  }

  // ---------------------------------------------------------------- an arena's blocks
  function stadium(o) {
    const items = [], marks = [], tiers = list(o.tiers).length ? o.tiers : [{}, {}], n = int(o.blocks, 24, 6, 72), fw = Number(o.width) || 420, fh = Number(o.height) || 260;
    let rx = fw * 0.5 + 70, ry = fh * 0.5 + 70, k = 1;
    const pt = (a, x, y) => {
      // Between an oval and a rectangle, as a ground is.
      const c = Math.cos(a), s = Math.sin(a), e = 2 / 3;
      return [+(Math.sign(c) * Math.abs(c) ** e * x).toFixed(1), +(Math.sign(s) * Math.abs(s) ** e * y).toFixed(1)];
    };
    tiers.forEach((t, ti) => {
      const deep = Number(t.depth) || 58;
      for (let b = 0; b < n; b++) {
        const a0 = ((b + 0.04) / n) * 2 * Math.PI - Math.PI / 2, a1 = ((b + 0.96) / n) * 2 * Math.PI - Math.PI / 2, inner = [], outer = [];
        for (let i = 0; i <= 4; i++) {
          const a = a0 + ((a1 - a0) * i) / 4;
          inner.push(pt(a, rx, ry));
          outer.unshift(pt(a, rx + deep, ry + deep));
        }
        const id = t.prefix ? `${t.prefix}${b + 1}` : String((ti + 1) * 100 + b + 1);
        items.push({ id, label: id, shape: "polygon", x: 0, y: 0, points: [...inner, ...outer], ...(t.name ? { section: t.name } : {}), ...(t.price !== undefined ? { price: t.price } : {}), ...from(list(o.sections).find((s) => list(s.blocks).map(String).includes(id))) });
      }
      rx += deep + 12;
      ry += deep + 12;
      k++;
    });
    const x = fw / 2, y = fh / 2;
    marks.push({ path: rr(-x, -y, fw, fh, 8), fill: true, box: [-x, -y, x, y] });
    if (o.field !== "stage") marks.push({ path: `M0 ${-y}V${y}M${fh * 0.16} 0a${fh * 0.16} ${fh * 0.16} 0 1 0 ${-fh * 0.32} 0a${fh * 0.16} ${fh * 0.16} 0 1 0 ${fh * 0.32} 0`, line: true, box: [-x, -y, x, y] });
    marks.push(text(o.label ?? L("Field"), 0, o.field === "stage" ? 0 : y * 0.7, { size: 15, soft: true }));
    return settle(items, marks);
  }

  // ---------------------------------------------------------------- rooms set out for people
  function hall(o) {
    const items = [], marks = [], style = o.style ?? "banquet", front = (w, y = -70) => {
      if (o.stage === false) return;
      marks.push({ path: rr(w / 2 - Math.max(90, w * 0.22), y, Math.max(180, w * 0.44), 34, 5), fill: true, box: [w / 2 - 90, y, w / 2 + 90, y + 34] }, text(o.stage && o.stage !== true ? o.stage : L("Stage"), w / 2, y + 17, { size: 13, soft: true }));
    };
    const chair = (id, x, y, turn, extra) => items.push(seat(id, id.split("-").at(-1), x - S / 2, y - S / 2, { turn, width: S, height: S, ...extra }));
    if (style === "theatre") return cinema({ curve: 0, screen: o.stage === false ? false : o.stage && o.stage !== true ? o.stage : L("Stage"), ...o, rows: o.rows ?? 10, seats: o.seats ?? 14 });
    if (style === "banquet" || style === "cabaret") {
      const n = int(o.tables, 12, 1, 200), per = int(o.seats, style === "cabaret" ? 6 : 8, 1, 16), cols = int(o.columns, Math.ceil(Math.sqrt(n * 1.5)), 1, 40), r = Math.max(30, (per * (S + 4)) / (2 * Math.PI) - S / 2 - 2), D = r * 2 + S * 2 + 40;
      for (let i = 0; i < n; i++) {
        const row = Math.floor(i / cols), inRow = Math.min(cols, n - row * cols), x = (i % cols) * D + ((cols - inRow) * D) / 2 + D / 2 + (o.stagger && row % 2 ? D / 2 : 0), y = row * D * 0.94 + D / 2, t = String(i + 1);
        items.push({ id: `T${t}`, label: t, shape: "circle", x: x - r, y: y - r, width: r * 2, table: t, kind: "table", ...(o.tablesPick === false ? { locked: true } : {}) });
        for (let k = 0; k < per; k++) {
          // Cabaret leaves the side towards the stage open.
          const a = style === "cabaret" ? Math.PI * (-0.15 + (1.3 * k) / Math.max(1, per - 1)) : (k / per) * 2 * Math.PI - Math.PI / 2;
          const cx = x + Math.cos(a) * (r + S / 2 + 4), cy = y + Math.sin(a) * (r + S / 2 + 4);
          chair(`T${t}-${k + 1}`, cx, cy, (a * 180) / Math.PI - 90, { table: t, seat: String(k + 1) });
        }
      }
      front(cols * D);
    } else if (style === "classroom") {
      const rows = int(o.rows, 5, 1, 60), cols = int(o.columns, 3, 1, 20), per = int(o.seats, 3, 1, 10), tw = per * (S + 6) + 6, D = tw + 36;
      for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
        const x = c * D, y = r * (S + 54), row = letters(rows)[r];
        marks.push({ path: rr(x, y, tw, 18, 3), fill: true, box: [x, y, x + tw, y + 18] });
        for (let k = 0; k < per; k++) chair(`${row}${c * per + k + 1}`, x + 6 + k * (S + 6) + S / 2, y + 22 + S / 2, 0, { row });
      }
      front(cols * D - 36);
    } else if (style === "exhibition") {
      const rows = int(o.rows, 4, 1, 30), cols = int(o.columns, 6, 1, 40), w = Number(o.standWidth) || 90, h = Number(o.standHeight) || 70;
      // Back to back, with an aisle either side.
      for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
        const id = `${letters(rows)[r]}${c + 1}`, y = r * h + Math.floor((r + 1) / 2) * 56;
        items.push({ id, label: id, x: c * (w + 4), y, width: w, height: h, row: letters(rows)[r], ...from(sectionOf(o, letters(rows)[r])) });
      }
    } else {
      // One table, seen from above: its sides have chairs outside (and inside, for a U).
      const long = int(o.seats, 8, 1, 40), end = int(o.ends, style === "boardroom" ? 1 : 4, 0, 20), G = S + 8, T = 34;
      const w = (style === "boardroom" ? Math.max(1, end) : Math.max(2, end)) * G + (style === "boardroom" ? 30 : T), h = long * G + 16;
      let n = 1;
      const side = (count, f) => Array.from({ length: count }, (_, i) => f((i + 0.5) / count)).forEach(([x, y, turn]) => chair(String(n++), x, y, turn, {}));
      if (style === "boardroom") marks.push({ path: rr(0, 0, w, h, 14), fill: true, box: [0, 0, w, h] });
      else if (style === "ushape") marks.push({ path: `M0 0H${T}V${h - T}H${w - T}V0H${w}V${h}H0Z`, fill: true, box: [0, 0, w, h] });
      else marks.push({ path: `${rr(0, 0, w, h, 4)}${rr(T, T, w - 2 * T, h - 2 * T, 2)}`, fill: true, box: [0, 0, w, h] });
      side(long, (t) => [-S / 2 - 4, h * t, 90]);
      side(end, (t) => [w * t, h + S / 2 + 4, 0]);
      side(long, (t) => [w + S / 2 + 4, h * (1 - t), 270]);
      if (style !== "ushape") side(end, (t) => [w * (1 - t), -S / 2 - 4, 180]);
      else front(w, -90);
    }
    return settle(items, marks);
  }

  // ---------------------------------------------------------------- bays and desks
  function parking(o) {
    const items = [], marks = [], rows = int(o.rows, 4, 1, 40), bays = int(o.bays, 12, 1, 80), w = 46, h = 86, lane = 84, tag = letters(rows), ev = new Set(list(o.charging).map(String)), disabled = new Set(list(o.accessible).map(String));
    let y = 0;
    for (let r = 0; r < rows; r++) {
      for (let b = 0; b < bays; b++) {
        const id = `${tag[r]}${b + 1}`;
        items.push({ id, label: id, ...(ev.has(id) ? { sublabel: "⚡", charging: true } : disabled.has(id) ? { sublabel: "♿", accessible: true } : {}), x: b * (w + 2), y, width: w, height: h, radius: 2, row: tag[r] });
      }
      y += h + (r % 2 ? 4 : lane);
      if (r % 2 === 0) marks.push(text("→", bays * (w + 2) * 0.25, y - lane / 2, { size: 26, soft: true }), text("→", bays * (w + 2) * 0.7, y - lane / 2, { size: 26, soft: true }));
    }
    return settle(items, marks);
  }
  function office(o) {
    const items = [], marks = [], pods = int(o.pods, 6, 1, 100), per = int(o.desks, 4, 2, 12), cols = int(o.columns, Math.ceil(Math.sqrt(pods)), 1, 20), half = Math.ceil(per / 2), dw = 64, dh = 44, PW = half * dw + 50, PH = dh * 2 + 90;
    for (let p = 0; p < pods; p++) {
      const x = (p % cols) * PW, y = Math.floor(p / cols) * PH, name = letters(pods)[p];
      for (let d = 0; d < per; d++) {
        const top = d < half, i = d % half, id = `${name}${d + 1}`, dx = x + i * dw, dy = y + (top ? 0 : dh);
        items.push({ id, label: id, x: dx + 1, y: dy + 1, width: dw - 2, height: dh - 2, pod: name });
        marks.push({ path: rr(dx + dw / 2 - 9, top ? dy - 20 : dy + dh + 4, 18, 16, 6), fill: true, box: [dx, dy - 20, dx + dw, dy + dh + 20] });
      }
    }
    return settle(items, marks);
  }

  const KINDS = { aircraft: (o) => cabin(o, "aircraft"), bus: (o) => cabin({ seats: "AB CD", ...o }, "bus"), train: (o) => cabin({ seats: "AB CD", facing: true, ...o }, "train"), cinema, auditorium, theatre: (o) => auditorium({ angle: 100, blocks: 3, tiers: [{ name: L("Stalls"), rows: 10 }, { name: L("Circle"), rows: 5 }], ...o }), stadium, hall, parking, office };
  const made = new Map();
  window.__irenderPlan = (layout) => {
    if (!isObj(layout) || !KINDS[layout.kind]) return null;
    const key = JSON.stringify(layout);
    if (!made.has(key)) {
      if (made.size > 40) made.clear();
      const out = KINDS[layout.kind](layout);
      // Two floors can have the same seats: theirs are told apart.
      if (layout.prefix) for (const i of out.items) i.id = `${layout.prefix}${i.id}`;
      if (layout.status !== undefined) for (const i of out.items) i.status ??= layout.status;
      made.set(key, out);
    }
    return made.get(key);
  };
})();
