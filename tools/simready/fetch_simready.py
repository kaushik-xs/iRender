# Downloads only the render payloads (USD layers and textures) of NVIDIA SimReady assets.
#   python3 fetch_simready.py OUT_DIR asset_name…
import json, os, sys, urllib.request, concurrent.futures as cf
BASE = "https://huggingface.co/datasets/nvidia/simready-assets"
out = sys.argv[1]
def get(name):
    tree = json.load(urllib.request.urlopen(f"https://huggingface.co/api/datasets/nvidia/simready-assets/tree/main/{name}?recursive=true"))
    want = []
    for f in tree:
        p = f["path"]
        if f["type"] != "file" or "/.thumbs/" in p or "/source_assets/" in p or "/.metadata/" in p: continue
        if p.endswith((".usd", ".usda", ".png", ".jpg")) : want.append(p)
    for p in want:
        dst = os.path.join(out, p)
        if os.path.exists(dst): continue
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        urllib.request.urlretrieve(f"{BASE}/resolve/main/{p}", dst)
    return name, len(want)
with cf.ThreadPoolExecutor(6) as ex:
    for r in ex.map(get, sys.argv[2:]): print(*r, flush=True)
