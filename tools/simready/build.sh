#!/usr/bin/env bash
# Downloads NVIDIA SimReady assets and converts them to GLB for irender-twin-kit:
# Y up, metres, standing at their base, glTF materials, WebP textures, meshopt.
#
#   tools/simready/build.sh OUT_DIR [name asset ratio texture-size]…
#   tools/simready/build.sh examples/twin/models forklift vehicle_forklift_orange_a01 0.2 1024
#
# With no assets named, it makes the ones in examples/twin/models (see CREDITS.md there).
# Needs python3 with `usd-core numpy pillow` (`pip install usd-core numpy pillow`), and
# Node.js for gltf-transform (fetched by npx). Assets: https://huggingface.co/datasets/nvidia/simready-assets
# (CC BY 4.0: credit NVIDIA and say they were modified).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${1:?usage: build.sh OUT_DIR [name asset ratio texture-size]…}"
shift
work="${WORK:-${TMPDIR:-/tmp}/irender-simready}"
py="${PY:-python3}"
mkdir -p "$work/dl" "$work/glb" "$out"

# name  SimReady asset  simplify ratio  texture size
assets="$*"
if [ -z "$assets" ]; then
  assets="
amr robotics_small_bot_a01 0.05 512
carton box_cardboard_a01 1 512
conveyor warehouse_conveyorBelt_a05 0.3 512
crate_blue crate_blue_a01 0.1 512
crate_green crate_green_a01 0.1 512
drum_blue drum_plastic_blue_a01 0.3 512
drum_steel drum_steel_a01 0.2 512
extinguisher fireExtinguisher_stand_a01 0.15 512
floor_sign sign_floor_movable_a01 0.15 512
forklift vehicle_forklift_orange_a01 0.2 1024
guard wire_machineGuard_a01 0.3 512
hopper hopper_dumpling_steel_blue_a04 0.3 512
hvls_fan ventilation_ceilingFan_hvls_a01 0.1 512
ibc_tank ibcTank_steel_stainless_a01 0.15 512
pallet warehouse_pallet_a01 1 512
pallet_jack palletJack_lowProfile_a01 0.2 512
rack rack_large_empty_a01 0.5 512
table_ss table_metal_stainless_a03 0.6 512
"
fi

echo "$assets" | xargs -n4 | awk '{print $2}' | xargs "$py" -I "$here/fetch_simready.py" "$work/dl"
echo "$assets" | xargs -n4 | while read -r name src ratio tex; do
  usd=$(ls "$work/dl/$src/simready_usd/"sm_*.usd* | head -1)
  "$py" -I "$here/usd2glb.py" "$usd" "$work/glb/$name.glb" --tex 2048
  npx --yes @gltf-transform/cli@4.1.0 optimize "$work/glb/$name.glb" "$out/$name.glb" \
    --compress meshopt --texture-compress webp --texture-size "$tex" \
    --simplify-ratio "$ratio" --simplify-error 0.0008 --instance false --flatten false
done
