# render/src

Draws viewports on the GPU with wgpu, into offscreen images: the sky, every copy of every model in the chosen look, the sun with its shadows, lamps, fog, projectiles, the ground grid and axes, all in HDR and then tone mapped for the screen. Also answers what is under the mouse, and times the GPU's passes. It knows nothing about the editor.

**Start reading at** `renderer.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers; checks the shaders compile. | |
| `renderer.rs` | Draws one viewport frame into its own image: shadow map, sky, models, grid, axes, projectiles, then the last tone-mapping pass. | `ViewportRenderer`, `ViewportTarget`, `LineVertex` |
| `frame.rs` | What the renderer needs each frame (camera, colours, look, grid plane) and how it is laid out for the GPU. | `FrameParams`, `Look`, `Shading`, `GridPlane` |
| `gpu_model.rs` | Models on the GPU: an `assets::Model` uploaded once into buffers and textures, then drawn every frame; also the outline and shadow passes. | `GpuModel` |
| `instances.rs` | Where each copy of a mesh is drawn: all placements gathered into one buffer each frame, grouped so each mesh is drawn once for all its copies; copies out of view are left out, far ones get simpler levels. | `Placement` |
| `lighting.rs` | The sun and lamps, the sun's shadow map fitted to what's in view, and the markers drawn where lamps are. | `Sun`, `Light`, `LightKind`, `sun_view_fitted` |
| `environment.rs` | The sky (the editor's backdrop or a daylight sky), fog, exposure and tone mapping. | `Environment`, `Sky`, `Fog`, `Tonemap` |
| `projectiles.rs` | Projectiles drawn in one go, each a glowing streak. | `Bolt` |
| `pixel.rs` | The pixel stage (Phase 9, D1): a 2D game's small picture scaled up to the screen in whole steps or to fill, slid by the camera's bit of a pixel. | `pixel_layout`, `PixelLayout`, `PixelImage`, `PixelScaling` |
| `pick.rs` | Picking: draws placement numbers and depth into a small image to say what is under the mouse. | `PickResult` |
| `timing.rs` | How long the GPU takes over each pass, for the Profiler, read back a frame or two later without waiting; and counts of what was drawn. | `GpuTimes`, `FrameStats` |
| `readback.rs` | Copies a finished image back from the GPU, for tests and screenshots. The only place that waits on the GPU. | `read_pixels`, `read_texture` |
| `viewport.wgsl` | The shaders (small programs that run on the GPU): sky, grid, lines, models in each look, lighting and shadows, and the final exposure, tone mapping and sRGB pass (`post_fs`). | `mesh_vs`, `sky_fs`, `grid_fs`, `post_fs` |
| `pick.wgsl` | The picking shader, kept apart so the viewport shader stays as it is. | |
| `pixel.wgsl` | The pixel stage's shader: one screen-filling triangle that picks (or, for fill, sharply blends) the small picture's pixels. | `pixel_vs`, `pixel_fs` |

## How the pieces fit

The caller uploads each model once (`ViewportRenderer::upload_model`, giving a `GpuModel`), then each frame hands the renderer a `FrameParams`, an `Environment`, the `Sun`, the `Light`s and the `Placement`s, with a `ViewportTarget` to draw into. The Rust files fill buffers whose layout must match the `.wgsl` files. The finished colour image is plain 8-bit RGBA that the editor shows as it is ([ARCHITECTURE.md](../../../ARCHITECTURE.md), "Colour in the viewport").

**Read next:** the crate's [README](../README.md); the pixel tests in [`../tests`](../tests/README.md) show each feature at work.
