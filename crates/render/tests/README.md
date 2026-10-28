# render/tests

Tests that draw real frames on a GPU and check their pixels.

| File | What it does |
| --- | --- |
| `gpu.rs` | Renders real frames and checks colours: the backdrop and axes, models in each look, textures, metal and glow, the sun's shadows, lamps, sky, fog, exposure and tone mapping, copies and mirrored copies, levels of detail, leaving out what's out of view, picking, projectiles, and the same frame on one core or many. Also holds the ignored `scale_check` tests that `tools/scale/scale-check.sh` runs. |

Run with `cargo test -p render --test gpu`. Without a GPU the tests skip with a note; set `REQUIRE_GPU=1` to make a missing GPU a failure, as CI does. With no graphics card, Mesa's software driver works: add `WGPU_BACKEND=gl`.

**Read next:** [`../src`](../src/README.md) for the code these tests drive.
