# render

**Layer 1 · Engine**

Draws viewports with wgpu into offscreen images: for now the backdrop (a calm sky with a horizon), the ground grid (1 m, 10 m and 100 m lines) and the X and Y axes. Shapes, materials, lights and shadows arrive in Phase 1.

## Purpose

Turn a camera and a few colours ([`FrameParams`]) into a finished image in a `ViewportTarget`, on whatever GPU device it is given. The editor shows that image in its viewport panel; games will draw through the same crate.

## Must never

- Depend on the editor or any interface crate: games ship without the editor
- Wait for the GPU in the frame loop (only the test helper `read_pixels` waits)
- Hold on to the window or the GPU device: it is handed a device for each call

## Colour

The image is plain 8-bit RGBA; the shaders convert linear colour to sRGB as they write, so it works on GPUs (such as OpenGL ones) that can't reinterpret an image's format. See ARCHITECTURE.md, "Colour in the viewport".

## Tests

`tests/gpu.rs` renders real frames and checks pixels. It skips without a GPU, unless `REQUIRE_GPU=1` is set. Without a graphics card, Mesa's software driver works: `WGPU_BACKEND=gl cargo test -p render --test gpu`.

## Depends on

wgpu, glam and bytemuck. No Solis crates.

## Status

Phase 0: backdrop, grid and axes.
