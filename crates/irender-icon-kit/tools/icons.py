#!/usr/bin/env python3
"""Checks and previews the kit's icons. See ../GUIDELINES.md.

    tools/icons.py lint [PACK ...]                  # check the measurable rules
    tools/icons.py sheet [PACK ...] -o sheet.html   # a contact sheet to review by eye
    tools/icons.py sheet logistics -o s.html --png s.png   # ... and a screenshot (needs Chrome)
    tools/icons.py sheet logistics --filled -o s.html      # the filled style, beside the outline

Without PACK, every pack under icons/ is used.
"""

import argparse
import math
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "icons"
NS = "{http://www.w3.org/2000/svg}"
ROOT_ATTRS = {
    "viewBox": "0 0 24 24",
    "fill": "none",
    "stroke": "currentColor",
    "stroke-width": "1.4",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
}
FILLED_ROOT_ATTRS = dict(ROOT_ATTRS, fill="currentColor")
# Packs of logo marks: solid shapes kept as their owners drew them, exempt from
# the drawing rules (see "Logo marks" in the guidelines).
MARKS = {"brands"}
MARK_ROOT_ATTRS = {"viewBox": "-2 -2 28 28", "fill": "currentColor"}
# The only extra attributes a filled shape may set, and their only values.
FILLED_EXTRAS = {"fill": "none", "fill-rule": "evenodd"}
SHAPES = {
    "path": {"d"},
    "circle": {"cx", "cy", "r"},
    "ellipse": {"cx", "cy", "rx", "ry"},
    "rect": {"x", "y", "width", "height", "rx", "ry"},
    "line": {"x1", "y1", "x2", "y2"},
    "polyline": {"points"},
    "polygon": {"points"},
}
NAME = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
NUMBER = re.compile(r"-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?")
# Centrelines stay inside these (see "Canvas" in the guidelines).
MIN_Y, MAX_Y = 3, 21
MIN_X, MAX_X = 2, 22
EPS = 0.01


def packs(names):
    found = sorted(p for p in ROOT.iterdir() if p.is_dir())
    if not names:
        return found
    wanted = [ROOT / n for n in names]
    for p in wanted:
        if not p.is_dir():
            sys.exit(f"no pack `{p.name}` in {ROOT}")
    return wanted


def numbers(s):
    return [float(n) for n in NUMBER.findall(s)]


def on_grid(v):
    return abs(v * 2 - round(v * 2)) < EPS


# ---------------------------------------------------------------- path geometry


def tokens(d):
    for m in re.finditer(r"[MmLlHhVvCcSsQqTtAaZz]|" + NUMBER.pattern, d):
        yield m.group()


def arc_points(x1, y1, rx, ry, phi, large, sweep, x2, y2):
    """Samples an SVG arc (endpoint form) by converting it to centre form."""
    if rx == 0 or ry == 0:
        return [(x2, y2)]
    rx, ry = abs(rx), abs(ry)
    p = math.radians(phi)
    cp, sp = math.cos(p), math.sin(p)
    dx, dy = (x1 - x2) / 2, (y1 - y2) / 2
    x1p, y1p = cp * dx + sp * dy, -sp * dx + cp * dy
    lam = (x1p / rx) ** 2 + (y1p / ry) ** 2
    if lam > 1:
        rx, ry = rx * math.sqrt(lam), ry * math.sqrt(lam)
    num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p
    den = rx * rx * y1p * y1p + ry * ry * x1p * x1p
    co = math.sqrt(max(0, num / den)) if den else 0
    if large == sweep:
        co = -co
    cxp, cyp = co * rx * y1p / ry, -co * ry * x1p / rx
    cx = cp * cxp - sp * cyp + (x1 + x2) / 2
    cy = sp * cxp + cp * cyp + (y1 + y2) / 2

    def angle(ux, uy, vx, vy):
        a = math.atan2(ux * vy - uy * vx, ux * vx + uy * vy)
        return a

    t1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry)
    dt = angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry)
    if not sweep and dt > 0:
        dt -= 2 * math.pi
    elif sweep and dt < 0:
        dt += 2 * math.pi
    out = []
    for i in range(1, 33):
        t = t1 + dt * i / 32
        x, y = rx * math.cos(t), ry * math.sin(t)
        out.append((cp * x - sp * y + cx, sp * x + cp * y + cy))
    return out


def path_points(d):
    """Points along a path's centreline: every vertex, plus samples of curves."""
    toks = list(tokens(d))
    pts, i = [], 0
    x = y = sx = sy = 0.0
    cmd = None
    last_ctrl = None

    def num():
        nonlocal i
        v = float(toks[i])
        i += 1
        return v

    while i < len(toks):
        if re.match(r"[A-Za-z]", toks[i]):
            cmd = toks[i]
            i += 1
            if cmd in "Zz":
                x, y = sx, sy
                pts.append((x, y))
                continue
        rel = cmd.islower()
        c = cmd.upper()
        ox, oy = (x, y) if rel else (0, 0)
        if c == "M":
            x, y = ox + num(), oy + num()
            sx, sy = x, y
            pts.append((x, y))
            cmd = "l" if rel else "L"
        elif c == "L":
            x, y = ox + num(), oy + num()
            pts.append((x, y))
        elif c == "H":
            x = (x if rel else 0) + num()
            pts.append((x, y))
        elif c == "V":
            y = (y if rel else 0) + num()
            pts.append((x, y))
        elif c in "CSQT":
            if c == "C":
                c1 = (ox + num(), oy + num())
                c2 = (ox + num(), oy + num())
            elif c == "S":
                c1 = (2 * x - last_ctrl[0], 2 * y - last_ctrl[1]) if last_ctrl else (x, y)
                c2 = (ox + num(), oy + num())
            elif c == "Q":
                c1 = c2 = (ox + num(), oy + num())
            else:
                c1 = c2 = (2 * x - last_ctrl[0], 2 * y - last_ctrl[1]) if last_ctrl else (x, y)
            ex, ey = ox + num(), oy + num()
            for k in range(1, 17):
                t = k / 16
                if c in "QT":
                    px = (1 - t) ** 2 * x + 2 * (1 - t) * t * c1[0] + t * t * ex
                    py = (1 - t) ** 2 * y + 2 * (1 - t) * t * c1[1] + t * t * ey
                else:
                    px = (1 - t) ** 3 * x + 3 * (1 - t) ** 2 * t * c1[0] + 3 * (1 - t) * t * t * c2[0] + t**3 * ex
                    py = (1 - t) ** 3 * y + 3 * (1 - t) ** 2 * t * c1[1] + 3 * (1 - t) * t * t * c2[1] + t**3 * ey
                pts.append((px, py))
            last_ctrl = c2
            x, y = ex, ey
            continue
        elif c == "A":
            rx, ry, phi, large, sweep = num(), num(), num(), num(), num()
            ex, ey = ox + num(), oy + num()
            pts.extend(arc_points(x, y, rx, ry, phi, int(large), int(sweep), ex, ey))
            x, y = ex, ey
        last_ctrl = None
    return pts


def shape_points(tag, a):
    f = lambda k, d=0.0: float(a.get(k, d))
    if tag == "path":
        return path_points(a["d"])
    if tag in ("circle", "ellipse"):
        rx = f("r") if tag == "circle" else f("rx")
        ry = f("r") if tag == "circle" else f("ry")
        return [(f("cx") - rx, f("cy") - ry), (f("cx") + rx, f("cy") + ry)]
    if tag == "rect":
        return [(f("x"), f("y")), (f("x") + f("width"), f("y") + f("height"))]
    if tag == "line":
        return [(f("x1"), f("y1")), (f("x2"), f("y2"))]
    v = numbers(a["points"])
    return list(zip(v[0::2], v[1::2]))


# ---------------------------------------------------------------- lint


def filled_set(pack):
    """Names drawn in the filled style, and names listed as the same in both."""
    d = pack / "filled"
    filled = {i.stem for i in d.glob("*.svg")} if d.is_dir() else set()
    same_file = d / "_same.txt"
    same = set(same_file.read_text().split()) if same_file.exists() else set()
    return filled, same


def lint_icon(path, filled=False):
    problems, warnings = [], []
    root_attrs = FILLED_ROOT_ATTRS if filled else ROOT_ATTRS
    if not NAME.match(path.stem):
        problems.append("name must be kebab-case")
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as e:
        return [f"not valid XML: {e}"], []
    if root.tag != NS + "svg":
        return ["root must be <svg xmlns=\"http://www.w3.org/2000/svg\">"], []
    for k, v in root_attrs.items():
        if root.get(k) != v:
            problems.append(f"root needs {k}=\"{v}\"")
    extra = set(root.attrib) - set(root_attrs)
    if extra:
        problems.append(f"root has extra attributes: {', '.join(sorted(extra))}")
    shapes = list(root)
    if not shapes:
        problems.append("no shapes")
    if len(shapes) > 10:
        warnings.append(f"{len(shapes)} shapes; aim for 8 or fewer")
    xs, ys = [], []
    for el in shapes:
        tag = el.tag.replace(NS, "")
        if tag not in SHAPES:
            problems.append(f"<{tag}> is not allowed")
            continue
        if len(el):
            problems.append(f"<{tag}> must not have children")
        extras = {k for k in el.attrib if filled and k in FILLED_EXTRAS}
        for k in extras:
            if el.get(k) != FILLED_EXTRAS[k]:
                problems.append(f"<{tag}> {k} may only be \"{FILLED_EXTRAS[k]}\"")
        bad = set(el.attrib) - SHAPES[tag] - extras
        if bad:
            problems.append(f"<{tag}> has disallowed attributes: {', '.join(sorted(bad))}")
        missing = SHAPES[tag] - set(el.attrib) - {"rx", "ry"} if tag == "rect" else SHAPES[tag] - set(el.attrib)
        if missing:
            problems.append(f"<{tag}> is missing {', '.join(sorted(missing))}")
            continue
        for k, v in el.attrib.items():
            if k in extras:
                continue
            for n in numbers(v):
                if k == "d" or k == "points" or k in SHAPES[tag]:
                    if not on_grid(n):
                        problems.append(f"<{tag}> {k}: {n:g} is off the half-pixel grid")
                        break
            if re.search(r"\d\.\d*0\b|\.0\b", v):
                warnings.append(f"<{tag}> {k}: trailing zeros")
        try:
            pts = shape_points(tag, el.attrib)
        except (IndexError, ValueError) as e:
            problems.append(f"<{tag}> could not be measured: {e}")
            continue
        xs += [p[0] for p in pts]
        ys += [p[1] for p in pts]
    if xs:
        lo_x, hi_x, lo_y, hi_y = min(xs), max(xs), min(ys), max(ys)
        if lo_y < MIN_Y - EPS or hi_y > MAX_Y + EPS:
            problems.append(f"vertical extent {lo_y:g}…{hi_y:g} leaves the live area ({MIN_Y}…{MAX_Y})")
        if lo_x < MIN_X - EPS or hi_x > MAX_X + EPS:
            problems.append(f"horizontal extent {lo_x:g}…{hi_x:g} leaves the live area ({MIN_X}…{MAX_X})")
    return problems, warnings


def lint_mark(path):
    """A logo mark: kept as its owner drew it, so only the markup is checked."""
    problems = []
    if not NAME.match(path.stem):
        problems.append("name must be kebab-case")
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as e:
        return [f"not valid XML: {e}"], []
    if root.tag != NS + "svg" or dict(root.attrib) != MARK_ROOT_ATTRS:
        problems.append('root must be exactly <svg xmlns="http://www.w3.org/2000/svg" viewBox="-2 -2 28 28" fill="currentColor">')
    shapes = list(root)
    if not shapes:
        problems.append("no shapes")
    for el in shapes:
        if el.tag != NS + "path" or set(el.attrib) != {"d"} or len(el):
            problems.append("only <path d=\"…\"/> elements are allowed")
            break
    return problems, []


def lint(args):
    failed = count = 0
    for pack in packs(args.packs):
        meta = pack / "_pack.txt"
        if not meta.exists() or len(meta.read_text().strip().splitlines()) < 2:
            print(f"{pack.name}/_pack.txt: needs a title line and a description line")
            failed += 1
        for icon in sorted(pack.glob("*.svg")):
            count += 1
            problems, warnings = lint_mark(icon) if pack.name in MARKS else lint_icon(icon)
            for p in problems:
                print(f"{pack.name}/{icon.name}: error: {p}")
            for w in warnings:
                print(f"{pack.name}/{icon.name}: warning: {w}")
            failed += bool(problems)
        filled, same = filled_set(pack)
        outline = {i.stem for i in pack.glob("*.svg")}
        if pack.name in MARKS and filled:
            print(f"{pack.name}/filled: error: logo marks have one style; list them in filled/_same.txt")
            failed += 1
        for name in sorted(filled):
            count += 1
            problems, warnings = lint_icon(pack / "filled" / f"{name}.svg", filled=True)
            for p in problems:
                print(f"{pack.name}/filled/{name}.svg: error: {p}")
            for w in warnings:
                print(f"{pack.name}/filled/{name}.svg: warning: {w}")
            failed += bool(problems)
        for name in sorted(filled - outline):
            print(f"{pack.name}/filled/{name}.svg: error: no outline icon of that name")
            failed += 1
        for name in sorted(same - outline):
            print(f"{pack.name}/filled/_same.txt: error: `{name}` is not an icon in the pack")
            failed += 1
        for name in sorted(same & filled):
            print(f"{pack.name}/filled/_same.txt: error: `{name}` also has a filled drawing")
            failed += 1
        missing = sorted(outline - filled - same)
        if missing and not args.partial:
            print(f"{pack.name}: error: no filled style (draw it or list it in filled/_same.txt): {' '.join(missing)}")
            failed += 1
    print(f"{count} icons, {failed} with errors")
    return 1 if failed else 0


# ---------------------------------------------------------------- sheet

SHEET_CSS = """
body { font: 12px system-ui, sans-serif; margin: 24px; background: #fff; color: #111; }
h2 { font-size: 16px; margin: 24px 0 8px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, 120px); gap: 12px; }
.cell { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.big { width: 96px; height: 96px; position: relative;
  background-image: linear-gradient(#eef 1px, transparent 1px), linear-gradient(90deg, #eef 1px, transparent 1px);
  background-size: 4px 4px; outline: 1px solid #ccd; }
.big::after { content: ""; position: absolute; inset: 8px; outline: 1px dashed #f99; }
.big svg { width: 96px; height: 96px; position: relative; z-index: 1; }
.sizes { display: flex; gap: 8px; align-items: center; }
.sizes svg { width: 24px; height: 24px; }
.sizes > svg:nth-child(2) { width: 16px; height: 16px; }
.dark { background: #111; color: #eee; padding: 2px 4px; border-radius: 4px; display: flex; gap: 6px; }
.dark svg { width: 24px; height: 24px; }
.same { opacity: .35; }
.name { font-family: ui-monospace, monospace; font-size: 11px; text-align: center; word-break: break-all; }
"""


def sheet(args):
    parts = [f"<!doctype html><meta charset=utf-8><style>{SHEET_CSS}</style>"]
    total = 0
    for pack in packs(args.packs):
        icons = sorted(pack.glob("*.svg"))
        total += len(icons)
        parts.append(f"<h2>{pack.name} ({len(icons)})</h2><div class=grid>")
        filled, same = filled_set(pack)
        for icon in icons:
            svg = icon.read_text().strip()
            if not args.filled:
                parts.append(
                    f"<div class=cell><div class=big>{svg}</div>"
                    f"<div class=sizes>{svg}{svg}<span class=dark>{svg}</span></div>"
                    f"<div class=name>{icon.stem}</div></div>"
                )
                continue
            # Filled: large filled view, then outline 24, filled 16, filled 24 on dark.
            if icon.stem in filled:
                fill, mark = (pack / "filled" / f"{icon.stem}.svg").read_text().strip(), ""
            elif icon.stem in same:
                fill, mark = svg, " class=same"
            else:
                fill, mark = "<svg viewBox='0 0 24 24'><text x=4 y=16 font-size=10 fill=red>?</text></svg>", ""
            parts.append(
                f"<div class=cell><div class=big{mark}>{fill}</div>"
                f"<div class=sizes><span>{svg}</span>{fill}<span class=dark>{fill}</span></div>"
                f"<div class=name>{icon.stem}</div></div>"
            )
        parts.append("</div>")
    out = Path(args.out)
    out.write_text("".join(parts))
    print(f"{total} icons -> {out}")
    if args.png:
        per_row = (args.width - 48) // 132
        rows = sum(-(-len(list(p.glob('*.svg'))) // per_row) for p in packs(args.packs))
        height = 60 + rows * 152 + 50 * len(packs(args.packs))
        chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
        subprocess.run(
            [chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars", f"--screenshot={Path(args.png).resolve()}",
             f"--window-size={args.width},{height}", out.resolve().as_uri()],
            check=True, capture_output=True,
        )
        print(f"screenshot -> {args.png}")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("lint")
    p.add_argument("packs", nargs="*")
    p.add_argument("--partial", action="store_true", help="don't require a filled style for every icon yet")
    p = sub.add_parser("sheet")
    p.add_argument("packs", nargs="*")
    p.add_argument("-o", "--out", default="sheet.html")
    p.add_argument("--png")
    p.add_argument("--filled", action="store_true", help="show the filled style (faded: same as outline; ? : missing)")
    p.add_argument("--width", type=int, default=1100)
    args = ap.parse_args()
    sys.exit(lint(args) if args.cmd == "lint" else sheet(args))


if __name__ == "__main__":
    main()
