# render/tests/gpu

Renders real frames on a GPU and checks their pixels: one test program (`cargo test -p render --test gpu`), a file per feature. `main.rs` holds what every file uses: taking turns at the GPU (`gpu`), the editor's backdrop colours, the camera most tests look through, reading a pixel; and it brings in the few helpers that one file makes and others use (`render_lit` from `looks.rs`, `floor` from `lights.rs`, …).

| File | What it tests |
| --- | --- |
| `main.rs` | The shared helpers, and the list of files. |
| `backdrop.rs` | The backdrop's sky and ground, the ground axes and grid, sizes clamped, and the scale check's viewport frame. |
| `brushes.rs` | Brushes: a brush's flat triangles drawn one shade a face, and filling the view as a model does. |
| `looks.rs` | Models in each look: an imported model with its texture, outlines, glow, the normal texture, metal and plastic; `dump_renders_to_look_at` writes pictures to look at by eye. |
| `lights.rs` | The sun's shadows on models and the grid, and lamps: points, spots, ranges, many at once. |
| `environment.rs` | Tone mapping and exposure, the editor's backdrop and the daylight sky, fog, and two viewports keeping their own frames. |
| `instances.rs` | Copies of a model: mirrored, placed twice, one part alone. |
| `frames.rs` | Picking, the axes close up, what a frame says it drew and timed, and the same frame on one core or many. |
| `gathering.rs` | The CPU's side of drawing many copies (scale checks). |
| `culling.rs` | Leaving out what's out of view, a shadow from outside it, and a frame that sends nothing new. |
| `lods.rs` | Simpler levels far away, and a model's own levels. |
| `projectiles.rs` | Projectiles glowing, hidden behind models and in a layer's picture. |
| `pixel_stage.rs` | The pixel stage: exact at every common size, sliding by part of a pixel, filling the screen, layers over each other. |
| `sprites.rs` | Sprites: their pixels exactly as drawn, flips and tints, who's in front, tile layers, picking, lit sprites. |

Without a GPU the tests skip with a note; `REQUIRE_GPU=1` makes that a failure. With no graphics card, Mesa's software driver works: add `WGPU_BACKEND=gl`.

**Read next:** [`../README.md`](../README.md).
