# render

**Layer 1 · Engine**

Draws viewports with wgpu into offscreen images: the backdrop (a calm sky with a horizon), imported models with glTF's full material in the standard, stylized (with outlines) or unlit look, lit by the sun (with a shadow map, also darkening the ground), up to 16 point lights and spotlights, and the sky; drawn in HDR and tone mapped (AgX, ACES or none) with exposure, under the editor's backdrop or a daylight sky, with optional fog (`environment.rs`, [decision 0017](../../docs/decisions/0017-hdr-tone-mapping-sky-and-fog.md)); the ground grid (1 m, 10 m and 100 m lines), the X and Y axes, and small markers where lamps are. `lighting.rs` holds the sun and lamps, in Blender's units ([decision 0014](../../docs/decisions/0014-lights-placed-two-ways.md)).

## Purpose

Turn a camera, a few colours and the sun ([`FrameParams`]), plus models placed in the world (`Placement`: a model and where it is) and lamps, into a finished image in a `ViewportTarget`, on whatever GPU device it is given. The editor shows that image in its viewport panel; games will draw through the same crate.

## Must never

- Depend on the editor or any interface crate: games ship without the editor
- Wait for the GPU in the frame loop (only the test helper `read_pixels` waits)
- Hold on to the window or the GPU device: it is handed a device for each call

## Colour

The image is plain 8-bit RGBA; the shaders convert linear colour to sRGB as they write, so it works on GPUs (such as OpenGL ones) that can't reinterpret an image's format. See ARCHITECTURE.md, "Colour in the viewport".

## Tests

`tests/gpu.rs` renders real frames and checks pixels. It skips without a GPU, unless `REQUIRE_GPU=1` is set. Without a graphics card, Mesa's software driver works: `WGPU_BACKEND=gl cargo test -p render --test gpu`.

## Depends on

`assets` (for the model data it uploads), wgpu, glam and bytemuck.

## Status

Phase 1 complete: backdrop and daylight sky, grid, axes, models in three looks, the sun with shadows, lamps, fog, HDR and tone mapping. Phase 2: every mesh copy is an instance in one buffer per frame, each mesh drawn once for all its copies; mirrored copies use pipelines that treat the other side as the front. A placement can draw one node of a model on its own (an unpacked part).
