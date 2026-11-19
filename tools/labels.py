#!/usr/bin/env python3
"""Lists the kits' interface text a document's `labels` can change, read from the source.

Keys are the English passed to `cx.label`, `cx.label_with` and `cx.label_count` (Rust)
and to `L` (a kit script's alias for `window.__irender.label`), plus those named in a
`// Labels: "…", "…"` comment where the text reaches the lookup through a variable.
Writes Markdown to stdout; tools/llm-docs.sh puts it in the skill's references.
"""
import glob
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
STR = r'"((?:[^"\\]|\\.)*)"'
PATTERNS = [
    (re.compile(r"\blabel_count\(\s*" + STR + r"\s*,\s*" + STR), "count"),
    (re.compile(r"\bL\(\s*\[\s*" + STR + r"\s*,\s*" + STR + r"\s*\]"), "count"),
    (re.compile(r"\blabel(?:_with)?\(\s*" + STR), "text"),
    (re.compile(r"\bL\(\s*" + STR), "text"),
]
MARKED = re.compile(r"(?://|/\*)\s*Labels(?: \([^)]*\))?:\s*(.*)")

KITS = [
    ("irender-ui-kit", "UI kit"),
    ("irender-markdown-kit", "Markdown kit"),
    ("irender-chart-kit", "Chart kit"),
    ("irender-flow-kit", "Flow kit"),
    ("irender-html", "Core"),
]


def unescape(s):
    return s.replace('\\"', '"').replace("\\\\", "\\")


def keys(path):
    """(key, plural one-form or None) in the order they appear."""
    out = []
    # Doc comments show the calls as examples; they aren't keys.
    text = "\n".join(l for l in open(path, encoding="utf-8").read().splitlines() if not l.lstrip().startswith(("///", "//!")))
    for line in text.splitlines():
        m = MARKED.search(line)
        if m:
            out += [(unescape(k), None) for k in re.findall(STR, m.group(1))]
    for pattern, kind in PATTERNS:
        for m in pattern.finditer(text):
            if kind == "count":
                out.append((m.start(), unescape(m.group(2)), unescape(m.group(1))))
            else:
                out.append((m.start(), unescape(m.group(1)), None))
    marked = [x for x in out if len(x) == 2]
    found = sorted(x for x in out if len(x) == 3)
    seen, result = set(), []
    for key, one in marked + [(k, o) for _, k, o in found]:
        if key not in seen:
            seen.add(key)
            result.append((key, one))
    # A count's key also seen as plain text keeps its plural form.
    plural = {k: o for k, o in result if o}
    return [(k, plural.get(k)) for k, _ in result]


def yaml_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    print("# Interface text (labels)\n")
    print("The kits' fixed words (buttons, counts, messages, screen-reader text) can be given in the document's language. Set `lang` and, in `labels`, the text to use for any of these English keys; the rest stay English. `{name}` marks a value filled in (keep it in your text). A count's key takes its plural forms, `{ one: …, other: … }` (and `zero`, `two`, `few`, `many` where the language has them, as `Intl.PluralRules` names them); a single text is used for every count.\n")
    print("```yaml\nversion: 1\nlang: es\nlabels:\n  \"Rows per page\": \"Filas por página\"\n  \"Page {page} of {pages}\": \"Página {page} de {pages}\"\n  \"{n} results\": { one: \"{n} resultado\", other: \"{n} resultados\" }\nroot: …\n```\n")
    total = 0
    for crate, name in KITS:
        files = sorted(glob.glob(os.path.join(ROOT, "crates", crate, "src", "*.rs")) + glob.glob(os.path.join(ROOT, "crates", crate, "src", "*.js")))
        sections = []
        for f in files:
            ks = keys(f)
            if ks:
                sections.append((os.path.basename(f), ks))
        if not sections:
            continue
        print(f"## {name}\n")
        for file, ks in sections:
            print(f"`{file}`\n")
            print("```yaml")
            for k, one in ks:
                total += 1
                if one:
                    print(f"{yaml_str(k)}: {{ one: {yaml_str(one)}, other: {yaml_str(k)} }}")
                else:
                    print(f"{yaml_str(k)}: {yaml_str(k)}")
            print("```\n")
    print(f"{total} keys.", file=sys.stderr)


main()
