#!/usr/bin/env python3
"""Puts the examples' navigation bar (site-nav.yaml) at the top of every built page, so
each example links to the others. examples/build.sh runs it last:

    python3 examples/site_nav.py path/to/irender target/site

The bar is rendered with `--embed` for each page, with the way back to the site's root
and the page's own href, and goes right after `<body>`. Blocks show in frames on the
blocks page, where the bar takes itself out.
"""

import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
# The pages in the bar itself; every other example goes under More.
MAIN = {
    "ui-kit.html": "UI kit",
    "blocks/index.html": "Blocks",
    "charts/index.html": "Charts",
    "flow/index.html": "Flow",
    "icons/index.html": "Icons",
    "dashboard.html": "Dashboard",
}
# Pages where the bar stays at the top while they scroll. Their layout reads its height
# from `--site-nav` (sticky sidebars sit below it).
STICKY = {"ui-kit.html"}
STICKY_HEIGHT = "<style>:root { --site-nav: 48px; }</style>"
# Removes the bar inside a frame, where the page around it already has one.
UNFRAMED = "<script>if (self !== top) document.currentScript.previousElementSibling.remove();</script>"


def main():
    irender, site = sys.argv[1], sys.argv[2]
    with open(os.path.join(HERE, "index.data.json")) as f:
        examples = json.load(f)["examples"]
    main_pages = [{"href": h, "label": l} for h, l in MAIN.items()]
    more = [{"href": e["href"], "label": e["title"]} for e in examples if e["href"] not in MAIN]
    pages = sorted(
        os.path.relpath(os.path.join(d, f), site).replace(os.sep, "/")
        for d, _, files in os.walk(site)
        for f in files
        if f.endswith(".html")
    )
    for page in pages:
        depth = page.count("/")
        data = {"root": "../" * depth, "current": page, "main": main_pages, "more": more, "sticky": page in STICKY}
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as tmp:
            json.dump(data, tmp)
        try:
            nav = subprocess.run(
                [irender, "render", os.path.join(HERE, "site-nav.yaml"), "--data", tmp.name, "--embed"],
                capture_output=True, text=True, check=True,
            ).stdout
        finally:
            os.unlink(tmp.name)
        path = os.path.join(site, page)
        with open(path) as f:
            html = f.read()
        if "data-site-nav" in html:
            continue  # already has it
        framed = page.startswith("blocks/") and page != "blocks/index.html"
        insert = nav + (UNFRAMED if framed else "") + (STICKY_HEIGHT if page in STICKY else "")
        html, n = re.subn(r"(<body[^>]*>)", lambda m: m.group(1) + insert, html, count=1)
        if n:
            with open(path, "w") as f:
                f.write(html)
    print(f"site_nav.py: linked {len(pages)} pages")


if __name__ == "__main__":
    main()
