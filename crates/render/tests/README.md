# render/tests

Tests that draw real frames on a GPU and check their pixels.

| File | What it does |
| --- | --- |
| [`gpu/`](gpu/README.md) | Renders real frames and checks colours, a file per feature: the backdrop and axes, models in each look, textures, metal and glow, the sun's shadows, lamps, sky, fog, exposure and tone mapping, copies and mirrored copies, levels of detail, leaving out what's out of view, picking, projectiles, the pixel stage, sprites, and the same frame on one core or many. Also holds the ignored `scale_check` tests that `tools/scale/scale-check.sh` runs. One test program, `gpu`. |
| `ui_gpu.rs` | Game UI drawn on a GPU (decision 0083): the sample HUD lands where the layout says on two screen sizes, pixel art has hard edges and smooth screens soft ones, half-seen shapes blend over the picture; game UI's text wrapped, aligned, cut with "…", falling back to another font and in a project's own font (stage 1, text layout), held to its approved picture. |
| `data/` | The approved picture of game UI's text, `text-layout.png` (480 × 320): a wrapped paragraph with a word broken between its letters, lines placed left, in the middle and right, a line cut with "…", letters Inter lacks taken from the project's next font, a project's own font (JetBrains Mono), and letter spacing with a taller line height. Drawn on Linux with Mesa's software driver and compared there only, at most one pixel in a hundred differing; `SOLIS_UPDATE_PICTURES=1` writes it again, after looking at it. |

Run with `cargo test -p render --test gpu`. Without a GPU the tests skip with a note; set `REQUIRE_GPU=1` to make a missing GPU a failure, as CI does. With no graphics card, Mesa's software driver works: add `WGPU_BACKEND=gl`.

**Read next:** [`../src`](../src/README.md) for the code these tests drive.
