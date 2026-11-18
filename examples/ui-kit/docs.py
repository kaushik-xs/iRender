#!/usr/bin/env python3
"""The UI kit gallery's data: its sections (docs.json, written by hand) with the props of
the components each one shows, read from the components themselves, so the reference
never drifts from the code.

    python3 examples/ui-kit/docs.py path/to/irender > ui-kit.data.json

Each section's data is keyed by its id with `-` as `_` (expressions read `doc.date_picker`):
`doc.<id>` is a one-item list holding the section (title, group, summary, usage), and
`api.<id>` its components, each with its props as `{ name, type, doc, required }`.
"""

import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def main():
    irender = sys.argv[1]
    out = subprocess.run([irender, "components", "--ui-kit", "--json"], capture_output=True, text=True, check=True).stdout
    components = {c["name"]: c for c in json.loads(out)["components"]}
    with open(os.path.join(HERE, "docs.json")) as f:
        sections = json.load(f)["sections"]
    covered = set()
    doc, api, groups = {}, {}, []
    for s in sections:
        key = s["id"].replace("-", "_")
        doc[key] = [s]
        cs = []
        for name in s["components"]:
            c = components[name]
            covered.add(name)
            props = [
                {
                    "name": p["name"],
                    "type": " | ".join(p["values"]) if p["values"] else {"statekey": "state key", "any": "any"}.get(p["type"], p["type"]),
                    "doc": p["doc"],
                    "required": p["required"],
                }
                for p in c["props"]
            ]
            cs.append({"name": name, "doc": c["doc"], "props": props, "parts": ", ".join(c["parts"]), "children": c["children"]})
        api[key] = cs
        if not groups or groups[-1]["title"] != s["group"]:
            groups.append({"title": s["group"], "sections": []})
        # `nav`: a shorter name for the sidebar, where long titles wrap.
        groups[-1]["sections"].append({"id": s["id"], "title": s["title"], "nav": s.get("nav", s["title"])})
    missing = sorted(n for n in components if n.startswith("ui-") and n not in covered)
    if missing:
        print(f"docs.py: not in any section of docs.json: {', '.join(missing)}", file=sys.stderr)
    count = sum(1 for n in components if n.startswith("ui-"))
    json.dump({"groups": groups, "doc": doc, "api": api, "componentCount": count}, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
