#!/usr/bin/env sh
# Renders every example, plus an index page linking them, into one folder. Pages with
# the UI kit also get the chart kit, so their Theme button offers chart colours too.
#
#   examples/build.sh                 # into target/site
#   examples/build.sh path/to/out     # somewhere else
#   python3 examples/serve.py         # then open http://localhost:8000
#
# Serve the folder rather than opening the files: the galleries fetch their APIs, which
# browsers block for file:// pages. `serve.py` is a static server plus the
# live endpoints the chart gallery polls and streams; `python3 -m http.server 8000 -d
# target/site` serves everything else.
set -eu
cd "$(dirname "$0")/.."
out=${1:-target/site}
# Start from an empty folder, so pages from earlier builds don't linger, but only one
# this script built (it leaves `.irender-site` in it): never a folder it didn't make.
if [ -f "$out/.irender-site" ]; then
  rm -rf "${out:?}"
elif [ -d "$out" ] && [ -n "$(ls -A "$out")" ]; then
  echo "note: $out wasn't built by this script, so pages already in it are kept" >&2
fi
mkdir -p "$out"
touch "$out/.irender-site"

cargo build -q -p irender-cli
# Where cargo put it, which `CARGO_TARGET_DIR` can change.
irender="${CARGO_TARGET_DIR:-target}/debug/irender"

$irender render examples/index.yaml --data examples/index.data.json --ui-kit --chart-kit --title "iRender examples" -o "$out/index.html"
# The gallery's documentation: sections from examples/ui-kit/docs.json, with each
# component's props read from the components themselves.
python3 examples/ui-kit/docs.py "$irender" > "$out/ui-kit.data.json"
$irender render examples/ui-kit/gallery.yaml --data "$out/ui-kit.data.json" --ui-kit --chart-kit --markdown-kit --title "UI kit" -o "$out/ui-kit.html"
cp -R examples/ui-kit/api "$out/"
# Blocks: whole pages, previewed together on blocks/index.html.
mkdir -p "$out/blocks"
for block in login-03 login-04 signup sidebar-03 sidebar-07 settings pricing error-404 error-500 error-403 error-503 error-offline error-session otp-verify; do
  $irender render "examples/blocks/$block.yaml" --ui-kit --icon-kit --title "$block" -o "$out/blocks/$block.html"
done
$irender render examples/blocks/dashboard-01.yaml --data examples/blocks/dashboard-01.data.json \
  --ui-kit --chart-kit --icon-kit --title "dashboard-01" -o "$out/blocks/dashboard-01.html"
for block in data-table-01 data-table-02 inventory-01; do
  $irender render "examples/blocks/$block.yaml" --data "examples/blocks/$block.data.json" \
    --ui-kit --chart-kit --icon-kit --title "$block" -o "$out/blocks/$block.html"
done
for block in report-01 crm-01 data-table-03 data-table-04 data-table-05 data-table-06 data-table-07 i18n-01 i18n-02 tree-01 docs-01 expense-01 ticket-01 gantt-01 dashboard-02 kanban-01 kanban-02 calendar-01 workspace-01 mail-01 empty-states data-table-08 import-01 invoice-01 intake-01 profile-01 supplier-01 record-01 pivot-01 schedule-01 plan-01 rules-01 review-01 scan-01 ops-01 agent-01 seats-flight-01 seats-cinema-01 seats-hall-01 floors-01 visit-01 capture-01 devices-01 form-designer-01 job-report-01 drive-01; do
  $irender render "examples/blocks/$block.yaml" --ui-kit --chart-kit --icon-kit --title "$block" -o "$out/blocks/$block.html"
done
# The twins draw 3D models from twin/models.
mkdir -p "$out/twin/models"
cp examples/twin/models/*.glb examples/twin/models/*.obj examples/twin/models/*.stl "$out/twin/models/"
mkdir -p "$out/twin/previews"
cp examples/twin/previews/*.jpg "$out/twin/previews/"
for block in twin-01 twin-02 twin-03 twin-04; do
  $irender render "examples/blocks/$block.yaml" --ui-kit --icon-kit --twin-kit --title "$block" -o "$out/blocks/$block.html"
done
# One page per example, so a browser draws one 3D view at a time; the index only links them.
$irender render examples/twin/index.yaml --ui-kit --twin-kit --title "Digital twins" -o "$out/twin/index.html"
for page in world machine-data colour-by-value paths popups formats live-data editing; do
  $irender render "examples/twin/$page.yaml" --ui-kit --twin-kit --title "$page" -o "$out/twin/$page.html"
done
# The sites are on a map. --ui-kit --chart-kit --icon-kit --map-kit --title "map-01" -o "$out/blocks/map-01.html"
# Installable apps: each in a folder of its own, with its manifest and service worker.
for app in field-01 keep-01; do
  mkdir -p "$out/blocks/$app"
  $irender render "examples/blocks/$app.yaml" --ui-kit --icon-kit --title "$app" -o "$out/blocks/$app/index.html"
done
# The incident's service map is a flow.
$irender render examples/blocks/incident-01.yaml --ui-kit --chart-kit --flow-kit --icon-kit --title "incident-01" -o "$out/blocks/incident-01.html"
# The chat's replies include charts, images and a video.
$irender render examples/blocks/chat.yaml --ui-kit --chart-kit --icon-kit --markdown-kit --title "chat" -o "$out/blocks/chat.html"
cp -R examples/blocks/media "$out/blocks/"
# Sample receipts for the expense claim and the gallery's file upload.
cp -R examples/blocks/files "$out/blocks/"
# The browser renderer for chat replies sent as `ui` JSON, when the WebAssembly target is
# installed (`rustup target add wasm32-unknown-unknown`). Without it the chat block asks
# serve.py's render endpoint instead.
if [ -d "$(rustc --print sysroot)/lib/rustlib/wasm32-unknown-unknown" ]; then
  cargo build -q -p irender-wasm --target wasm32-unknown-unknown --profile wasm
  mkdir -p "$out/irender"
  cp target/wasm32-unknown-unknown/wasm/irender_wasm.wasm "$out/irender/"
else
  echo "build.sh: no wasm32-unknown-unknown target; the chat block will render replies' UI on the server" >&2
fi
$irender render examples/blocks/markdown.yaml --ui-kit --icon-kit --markdown-kit --title "markdown" -o "$out/blocks/markdown.html"
$irender render examples/blocks/theme.yaml --ui-kit --chart-kit --icon-kit --title "theme" -o "$out/blocks/theme.html"
$irender render examples/blocks/index.yaml --ui-kit --chart-kit --title "Blocks" -o "$out/blocks/index.html"
mkdir -p "$out/charts"
$irender render examples/charts/gallery.yaml --data examples/charts/gallery.data.json --ui-kit --chart-kit \
  --title "Chart kit gallery" -o "$out/charts/index.html"
cp -R examples/charts/api "$out/charts/"
# Flows: a document each, previewed together on flow/index.html.
mkdir -p "$out/flow"
for flow in workflow-01 subflow-01 flowchart-01; do
  $irender render "examples/flow/$flow.yaml" --ui-kit --chart-kit --flow-kit --title "$flow" -o "$out/flow/$flow.html"
done
for flow in basic-01 er-04; do
  $irender render "examples/flow/$flow.yaml" --data "examples/flow/$flow.data.json" --ui-kit --chart-kit --flow-kit \
    --title "$flow" -o "$out/flow/$flow.html"
done
# The ER diagrams' data is a package turned into nodes and edges by host code.
cargo run -q -p irender-flow-kit --example package_er -- examples/flow/er-package > "$out/flow/er.data.json"
for flow in er-01 er-02 er-03; do
  $irender render "examples/flow/$flow.yaml" --data "$out/flow/er.data.json" --ui-kit --chart-kit --flow-kit --icon-kit \
    --title "$flow" -o "$out/flow/$flow.html"
done
for flow in process-01 process-02 process-03; do
  $irender render "examples/flow/$flow.yaml" --data examples/flow/process-canvas.json --ui-kit --chart-kit --flow-kit --icon-kit \
    --title "$flow" -o "$out/flow/$flow.html"
done
$irender render examples/flow/index.yaml --ui-kit --chart-kit --title "Flow" -o "$out/flow/index.html"
# The Google map on the maps page is only drawn with a key.
mkdir -p "$out/maps"
python3 -c 'import json,os,sys; d = json.load(open("examples/maps/gallery.data.json")); d["googleKey"] = os.environ.get("GOOGLE_MAPS_API_KEY", ""); json.dump(d, sys.stdout)' > "$out/maps/gallery.data.json"
cp examples/maps/floor-plan.svg "$out/maps/"
$irender render examples/maps/gallery.yaml --data "$out/maps/gallery.data.json" --ui-kit --chart-kit --map-kit --title "Maps" -o "$out/maps/index.html"
# The icon gallery's data is the kit's own pack listing.
mkdir -p "$out/icons"
$irender icons --json > "$out/icons/packs.json"
$irender render examples/icons/gallery.yaml --data "$out/icons/packs.json" --ui-kit --chart-kit --icon-kit \
  --title "Icon gallery" -o "$out/icons/index.html"

# Every page gets the bar that links the examples (examples/site-nav.yaml).
python3 examples/site_nav.py "$irender" "$out"

# Written once the pages are as they'll be served.
for app in field-01 keep-01; do
  $irender app "examples/blocks/$app.yaml" --dir "$out/blocks/$app"
done

echo "Rendered $(find "$out" -name '*.html' | wc -l | tr -d ' ') pages into $out"
echo "Serve:  python3 examples/serve.py 8000 $out   then open http://localhost:8000"
