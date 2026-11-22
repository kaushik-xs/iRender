// irender-ui-kit: spreadsheets and zips without libraries (`window.__irenderSheets`): CSV
// is split here, .xlsx is unzipped with the browser's DecompressionStream and read as
// XML, and files are zipped uncompressed.
(() => {
  if (window.__irenderSheets) return;
  // CSV with quoted fields, doubled quotes and line breaks inside quotes. Without `sep`,
  // whichever of comma, semicolon and tab the first line has most of.
  function csv(text, sep) {
    text = text.replace(/^\uFEFF/, "");
    if (!sep) {
      const first = text.slice(0, text.search(/\r?\n|$/)).replace(/"[^"]*"/g, "");
      sep = [",", ";", "\t", "|"].map((s) => [s, first.split(s).length]).sort((a, b) => b[1] - a[1])[0][0];
    }
    const rows = [];
    let row = [], cell = "", quoted = false;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (quoted) {
        if (c === '"' && text[i + 1] === '"') (cell += '"'), i++;
        else if (c === '"') quoted = false;
        else cell += c;
      } else if (c === '"' && !cell) quoted = true;
      else if (c === sep) row.push(cell), (cell = "");
      else if (c === "\n" || c === "\r") {
        if (c === "\r" && text[i + 1] === "\n") i++;
        row.push(cell), rows.push(row), (row = []), (cell = "");
      } else cell += c;
    }
    if (cell || row.length) row.push(cell), rows.push(row);
    return rows;
  }
  // Rows as CSV. A cell that a spreadsheet would run as a formula is quoted as text.
  const toCsv = (rows) => rows.map((r) => r.map((v) => {
    let s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    if (/^[=+\-@\t\r]/.test(s) && (s.trim() === "" || isNaN(s))) s = "'" + s;
    return /[",\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  }).join(",")).join("\r\n");

  async function unzip(buffer) {
    const v = new DataView(buffer), out = new Map();
    let end = buffer.byteLength - 22;
    while (end >= 0 && v.getUint32(end, true) !== 0x06054b50) end--;
    if (end < 0) throw new Error("not a zip file");
    let at = v.getUint32(end + 16, true);
    const count = v.getUint16(end + 10, true);
    const dec = new TextDecoder();
    for (let n = 0; n < count; n++) {
      const method = v.getUint16(at + 10, true), packed = v.getUint32(at + 20, true);
      const nameLen = v.getUint16(at + 28, true), extra = v.getUint16(at + 30, true), note = v.getUint16(at + 32, true);
      const offset = v.getUint32(at + 42, true);
      const name = dec.decode(new Uint8Array(buffer, at + 46, nameLen));
      const start = offset + 30 + v.getUint16(offset + 26, true) + v.getUint16(offset + 28, true);
      out.set(name, { method, data: new Uint8Array(buffer, start, packed) });
      at += 46 + nameLen + extra + note;
    }
    return async (name) => {
      const f = out.get(name);
      if (!f) return null;
      if (f.method === 0) return dec.decode(f.data);
      const stream = new Blob([f.data]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
      return new Response(stream).text();
    };
  }
  const xml = (s) => new DOMParser().parseFromString(s, "application/xml");
  // The sheets of an .xlsx, `[{ name, path }]` in their order.
  async function listSheets(read) {
    const book = await read("xl/workbook.xml"), rels = await read("xl/_rels/workbook.xml.rels");
    if (!book || !rels) return [{ name: "Sheet1", path: "xl/worksheets/sheet1.xml" }];
    const target = new Map([...xml(rels).getElementsByTagName("Relationship")].map((r) => [r.getAttribute("Id"), r.getAttribute("Target")]));
    return [...xml(book).getElementsByTagName("sheet")].map((s) => {
      const t = target.get(s.getAttribute("r:id")) ?? "worksheets/sheet1.xml";
      return { name: s.getAttribute("name"), path: t.startsWith("/") ? t.slice(1) : "xl/" + t };
    });
  }
  // A sheet of an .xlsx (the first, or `index`) as rows of text; dates as `YYYY-MM-DD`
  // (and the time when there is one).
  async function xlsx(buffer, index = 0) {
    const read = await unzip(buffer);
    const text = (n) => [...n.getElementsByTagName("t")].map((t) => t.textContent).join("");
    const shared = await read("xl/sharedStrings.xml");
    const strings = shared ? [...xml(shared).getElementsByTagName("si")].map(text) : [];
    // Which cell styles are dates: the built-in formats, and custom ones with y, m, d or h.
    const styles = await read("xl/styles.xml"), dates = [];
    let epoch = Date.UTC(1899, 11, 30);
    if (styles) {
      const x = xml(styles);
      const custom = new Map([...x.getElementsByTagName("numFmt")].map((f) => [Number(f.getAttribute("numFmtId")), f.getAttribute("formatCode") ?? ""]));
      const xfs = x.getElementsByTagName("cellXfs")[0];
      [...(xfs?.getElementsByTagName("xf") ?? [])].forEach((f, i) => {
        const id = Number(f.getAttribute("numFmtId"));
        const code = custom.get(id)?.replace(/"[^"]*"|\[[^\]]*\]|\\./g, "");
        dates[i] = (id >= 14 && id <= 22) || (id >= 45 && id <= 47) || (code !== undefined && /[ymdh]/i.test(code));
      });
      const book = await read("xl/workbook.xml");
      if (book && /date1904="(1|true)"/.test(book)) epoch = Date.UTC(1904, 0, 1);
    }
    const when = (n) => {
      const t = new Date(Math.round((epoch + n * 86400000) / 1000) * 1000).toISOString();
      return n % 1 === 0 ? t.slice(0, 10) : t.slice(0, 16).replace("T", " ");
    };
    const all = await listSheets(read);
    const sheet = (await read(all[index]?.path ?? "")) ?? (await read("xl/worksheets/sheet1.xml"));
    if (!sheet) throw new Error("no sheet found");
    const rows = [];
    let y = -1;
    for (const r of xml(sheet).getElementsByTagName("row")) {
      y = r.hasAttribute("r") ? Number(r.getAttribute("r")) - 1 : y + 1;
      const row = (rows[y] = []);
      let x = -1;
      for (const c of r.getElementsByTagName("c")) {
        const ref = (c.getAttribute("r") ?? "").replace(/\d+$/, "");
        x = ref ? [...ref].reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0) - 1 : x + 1;
        const t = c.getAttribute("t"), v = c.getElementsByTagName("v")[0]?.textContent ?? "";
        row[x] = t === "s" ? strings[Number(v)] ?? ""
          : t === "inlineStr" ? text(c)
          : t === "b" ? (v === "1" ? "TRUE" : "FALSE")
          : !t && v !== "" && dates[Number(c.getAttribute("s"))] ? when(Number(v))
          : v;
      }
    }
    return Object.assign(Array.from(rows, (r) => Array.from(r ?? [], (c) => c ?? "")), { sheets: all.map((s) => s.name) });
  }

  // An uncompressed zip of [name, bytes] pairs.
  const crcTable = Array.from({ length: 256 }, (_, n) => {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    return c >>> 0;
  });
  const crc32 = (bytes) => {
    let c = 0xffffffff;
    for (const b of bytes) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  function zip(entries, type = "application/zip") {
    const enc = new TextEncoder(), parts = [], central = [];
    let offset = 0;
    const header = (size) => new DataView(new ArrayBuffer(size));
    for (const [name, bytes] of entries) {
      const n = enc.encode(name), crc = crc32(bytes);
      const h = header(30);
      h.setUint32(0, 0x04034b50, true), h.setUint16(4, 20, true), h.setUint16(6, 0x0800, true);
      h.setUint32(14, crc, true), h.setUint32(18, bytes.length, true), h.setUint32(22, bytes.length, true), h.setUint16(26, n.length, true);
      parts.push(h, n, bytes);
      const c = header(46);
      c.setUint32(0, 0x02014b50, true), c.setUint16(4, 20, true), c.setUint16(6, 20, true), c.setUint16(8, 0x0800, true);
      c.setUint32(16, crc, true), c.setUint32(20, bytes.length, true), c.setUint32(24, bytes.length, true), c.setUint16(28, n.length, true);
      c.setUint32(42, offset, true);
      central.push(c, n);
      offset += 30 + n.length + bytes.length;
    }
    const size = central.reduce((s, p) => s + p.byteLength, 0);
    const end = header(22);
    end.setUint32(0, 0x06054b50, true), end.setUint16(8, entries.length, true), end.setUint16(10, entries.length, true);
    end.setUint32(12, size, true), end.setUint32(16, offset, true);
    return new Blob([...parts, ...central, end], { type });
  }
  // Rows as an .xlsx: numbers as numbers, `YYYY-MM-DD` as dates, the first row in bold and
  // kept in view. `formats`, per column: `percent`, `0.00`, … or none.
  function workbook(rows, name = "Sheet1", formats = []) {
    const enc = new TextEncoder();
    const esc = (s) => String(s).replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/g, "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    const col = (x) => {
      let s = "";
      for (x++; x > 0; x = Math.floor((x - 1) / 26)) s = String.fromCharCode(65 + ((x - 1) % 26)) + s;
      return s;
    };
    const codes = [...new Set(formats.filter((f) => f && f !== "percent"))];
    const style = (f) => (f === "percent" ? 3 : f ? 4 + codes.indexOf(f) : 0);
    const cell = (v, x, y) => {
      const ref = col(x) + (y + 1);
      if (v === null || v === undefined || v === "") return "";
      if (y > 0 && typeof v === "number" && Number.isFinite(v)) return `<c r="${ref}" s="${style(formats[x])}"><v>${v}</v></c>`;
      if (y > 0 && typeof v === "boolean") return `<c r="${ref}" t="b"><v>${v ? 1 : 0}</v></c>`;
      if (y > 0 && typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v) && !isNaN(Date.parse(v))) return `<c r="${ref}" s="2"><v>${(Date.parse(v) - Date.UTC(1899, 11, 30)) / 86400000}</v></c>`;
      return `<c r="${ref}" t="inlineStr"${y === 0 ? ' s="1"' : ""}><is><t xml:space="preserve">${esc(typeof v === "object" ? JSON.stringify(v) : v)}</t></is></c>`;
    };
    const widths = (rows[0] ?? []).map((_, x) => Math.min(60, Math.max(8, ...rows.slice(0, 200).map((r) => String(r[x] ?? "").length + 2))));
    const doc = (s) => enc.encode('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n' + s);
    const M = "http://schemas.openxmlformats.org/spreadsheetml/2006/main", R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships", P = "http://schemas.openxmlformats.org/package/2006";
    const T = "application/vnd.openxmlformats-officedocument.spreadsheetml";
    return zip([
      ["[Content_Types].xml", doc(`<Types xmlns="${P}/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="${T}.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="${T}.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="${T}.styles+xml"/></Types>`)],
      ["_rels/.rels", doc(`<Relationships xmlns="${P}/relationships"><Relationship Id="rId1" Type="${R}/officeDocument" Target="xl/workbook.xml"/></Relationships>`)],
      ["xl/workbook.xml", doc(`<workbook xmlns="${M}" xmlns:r="${R}"><sheets><sheet name="${esc(name).replace(/[\\/?*[\]:]/g, " ").slice(0, 31) || "Sheet1"}" sheetId="1" r:id="rId1"/></sheets></workbook>`)],
      ["xl/_rels/workbook.xml.rels", doc(`<Relationships xmlns="${P}/relationships"><Relationship Id="rId1" Type="${R}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="${R}/styles" Target="styles.xml"/></Relationships>`)],
      ["xl/styles.xml", doc(`<styleSheet xmlns="${M}">${codes.length ? `<numFmts count="${codes.length}">${codes.map((c, i) => `<numFmt numFmtId="${170 + i}" formatCode="${esc(c)}"/>`).join("")}</numFmts>` : ""}<fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="${4 + codes.length}"><xf/><xf fontId="1" applyFont="1"/><xf numFmtId="14" applyNumberFormat="1"/><xf numFmtId="10" applyNumberFormat="1"/>${codes.map((_, i) => `<xf numFmtId="${170 + i}" applyNumberFormat="1"/>`).join("")}</cellXfs></styleSheet>`)],
      ["xl/worksheets/sheet1.xml", doc(`<worksheet xmlns="${M}"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><cols>${widths.map((w, i) => `<col min="${i + 1}" max="${i + 1}" width="${w}" customWidth="1"/>`).join("")}</cols><sheetData>${rows.map((r, y) => `<row r="${y + 1}">${r.map((v, x) => cell(v, x, y)).join("")}</row>`).join("")}</sheetData></worksheet>`)],
    ], T + ".sheet");
  }
  // Saves a blob as a file.
  function save(blob, name) {
    const url = URL.createObjectURL(blob), a = Object.assign(document.createElement("a"), { href: url, download: name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
  }
  window.__irenderSheets = { csv, toCsv, xlsx, zip, workbook, save };
})();
