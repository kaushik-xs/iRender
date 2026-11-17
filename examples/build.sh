#!/usr/bin/env sh
# Renders every example, plus an index page linking them, into one folder.
#
#   examples/build.sh                 # into target/site
#   examples/build.sh path/to/out     # somewhere else
#   python3 -m http.server 8000 -d target/site   # then open http://localhost:8000
#
# Serve the folder rather than opening the files: the search example fetches its
# API, which browsers block for file:// pages.
set -eu
cd "$(dirname "$0")/.."
out=${1:-target/site}
mkdir -p "$out/search"

cargo build -q -p irender-cli
# Where cargo put it, which `CARGO_TARGET_DIR` can change.
irender="${CARGO_TARGET_DIR:-target}/debug/irender"

$irender render examples/index.yaml --data examples/index.data.json --title "iRender examples" -o "$out/index.html"
$irender render examples/dashboard/dashboard.yaml --data examples/dashboard/dashboard.data.json \
  --fragments examples/dashboard/fragments --title "Orders dashboard" -o "$out/dashboard.html"
$irender render examples/search/search.json --title "Live search" -o "$out/search/index.html"
cp -R examples/search/api "$out/search/"
$irender render examples/wizard.yaml --title "Sign-up wizard" -o "$out/wizard.html"
$irender render examples/product.json --data examples/product.data.json --title "Product card" -o "$out/product.html"
$irender render examples/counter.yaml --title "Counter" -o "$out/counter.html"
cargo run -q -p irender-html --example storefront > "$out/storefront.html"
cargo run -q -p irender-html --example storefront -- --variant b > "$out/storefront-b.html"

echo "Rendered $(find "$out" -name '*.html' | wc -l | tr -d ' ') pages into $out"
echo "Serve:  python3 -m http.server 8000 -d $out   then open http://localhost:8000"
