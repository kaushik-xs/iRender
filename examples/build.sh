#!/usr/bin/env sh
# Renders every example, plus an index page linking them, into one folder. Pages with
# the UI kit also get the chart kit, so their Theme button offers chart colours too.
#
#   examples/build.sh                 # into target/site
#   examples/build.sh path/to/out     # somewhere else
#   python3 examples/serve.py         # then open http://localhost:8000
#
# Serve the folder rather than opening the files: the search example fetches its
# API, which browsers block for file:// pages. `serve.py` is a static server plus the
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
mkdir -p "$out/search"
touch "$out/.irender-site"

cargo build -q -p irender-cli
# Where cargo put it, which `CARGO_TARGET_DIR` can change.
irender="${CARGO_TARGET_DIR:-target}/debug/irender"

$irender render examples/index.yaml --data examples/index.data.json --ui-kit --chart-kit --title "iRender examples" -o "$out/index.html"
$irender render examples/dashboard/dashboard.yaml --data examples/dashboard/dashboard.data.json \
  --fragments examples/dashboard/fragments --title "Orders dashboard" -o "$out/dashboard.html"
$irender render examples/search/search.json --title "Live search" -o "$out/search/index.html"
cp -R examples/search/api "$out/search/"
$irender render examples/wizard.yaml --title "Sign-up wizard" -o "$out/wizard.html"
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
for block in data-table-01 data-table-02; do
  $irender render "examples/blocks/$block.yaml" --data "examples/blocks/$block.data.json" \
    --ui-kit --chart-kit --icon-kit --title "$block" -o "$out/blocks/$block.html"
done
for block in data-table-03 data-table-04 data-table-05 data-table-06 tree-01 docs-01 expense-01 ticket-01 empty-states; do
  $irender render "examples/blocks/$block.yaml" --ui-kit --chart-kit --icon-kit --title "$block" -o "$out/blocks/$block.html"
done
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
mkdir -p "$out/flow"
$irender render examples/flow/gallery.yaml --data examples/flow/gallery.data.json --ui-kit --chart-kit --flow-kit \
  --title "Flow kit gallery" -o "$out/flow/index.html"
# The ER example's data is a package turned into nodes and edges by host code.
cargo run -q -p irender-flow-kit --example package_er -- examples/flow/er-package > "$out/flow/er.data.json"
$irender render examples/flow/er.yaml --data "$out/flow/er.data.json" --ui-kit --chart-kit --flow-kit --icon-kit \
  --title "ER diagram" -o "$out/flow/er.html"
$irender render examples/flow/process.yaml --data examples/flow/process-canvas.json --ui-kit --chart-kit --flow-kit --icon-kit \
  --title "Process canvas" -o "$out/flow/process.html"
# The icon gallery's data is the kit's own pack listing.
mkdir -p "$out/icons"
$irender icons --json > "$out/icons/packs.json"
$irender render examples/icons/gallery.yaml --data "$out/icons/packs.json" --ui-kit --chart-kit --icon-kit \
  --title "Icon gallery" -o "$out/icons/index.html"
$irender render examples/product.json --data examples/product.data.json --title "Product card" -o "$out/product.html"
$irender render examples/counter.yaml --title "Counter" -o "$out/counter.html"
cargo run -q -p irender-html --example storefront > "$out/storefront.html"
cargo run -q -p irender-html --example storefront -- --variant b > "$out/storefront-b.html"

# Every page gets the bar that links the examples (examples/site-nav.yaml).
python3 examples/site_nav.py "$irender" "$out"

echo "Rendered $(find "$out" -name '*.html' | wc -l | tr -d ' ') pages into $out"
echo "Serve:  python3 examples/serve.py 8000 $out   then open http://localhost:8000"
