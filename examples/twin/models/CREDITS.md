# Models

These models are from [NVIDIA SimReady Assets](https://huggingface.co/datasets/nvidia/simready-assets),
© NVIDIA Corporation, licensed [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). They have
been modified: converted from USD to glTF (Y up, metres, origin at the bottom centre), simplified,
compressed with meshopt and their textures re-encoded as WebP. `tools/simready/` does this.

| File | SimReady asset |
|---|---|
| `amr.glb` | `robotics_small_bot_a01` |
| `carton.glb` | `box_cardboard_a01` |
| `conveyor.glb` | `warehouse_conveyorBelt_a05` |
| `crate_blue.glb` | `crate_blue_a01` |
| `crate_green.glb` | `crate_green_a01` |
| `drum_blue.glb` | `drum_plastic_blue_a01` |
| `drum_steel.glb` | `drum_steel_a01` |
| `extinguisher.glb` | `fireExtinguisher_stand_a01` |
| `floor_sign.glb` | `sign_floor_movable_a01` |
| `forklift.glb` | `vehicle_forklift_orange_a01` |
| `guard.glb` | `wire_machineGuard_a01` |
| `hopper.glb` | `hopper_dumpling_steel_blue_a04` |
| `hvls_fan.glb` | `ventilation_ceilingFan_hvls_a01` |
| `ibc_tank.glb` | `ibcTank_steel_stainless_a01` |
| `pallet.glb` | `warehouse_pallet_a01` |
| `pallet_jack.glb` | `palletJack_lowProfile_a01` |
| `rack.glb` | `rack_large_empty_a01` |
| `table_ss.glb` | `table_metal_stainless_a03` |
| `milk.glb` | `food_processed_milk_a01` |
| `bottle_water.glb` | `food_beverage_bottledWater_b01` |
| `bottle_drink.glb` | `food_beverage_bottledDrink_a01` |
| `juice_box.glb` | `food_beverage_boxedDrink_a01` |
| `can_red.glb` | `food_beverage_can_b01` |
| `crate_red.glb` | `crate_red_a01` |
| `crate_yellow.glb` | `crate_yellow_a01` |

`beacon.obj` and `bracket.stl` were made for these examples (`examples/twin/models`), to show other formats: an OBJ in metres, and an STL in millimetres with z up.
