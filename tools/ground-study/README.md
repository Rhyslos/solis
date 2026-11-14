# ground-study

**Tool · not part of the program**

Tools 1b's first milestone, G0 ([the plan](../../docs/plans/tools-1.md)): the study behind the ground's storage and looks ([CONTRIBUTING.md](../../CONTRIBUTING.md) rule 22). Small working versions of each way a ground with caves and overhangs could be kept and drawn, written cleanly and tested, so the winners can become the ground's own crate later, and a program that measures them against each other at the scale target. The findings, numbers and pictures are in [docs/reports/ground-study.md](../../docs/reports/ground-study.md).

## Purpose

- Keep a ground's voxels three ways: fixed chunks in full, sparse chunks holding only the voxels near the surface, an octree per chunk (`src/store.rs`).
- Mesh each look two ways: Smooth by marching cubes or surface nets; Sharp by dual contouring or sharpened surface nets; Blocky by cube faces, plain or merged (`src/mesh/`).
- Sculpt it with the round brush across chunks, write and read its file as text and as bytes, and make each chunk solid with the physics crate's triangle collider.
- Time all of it (`src/study/`): memory and file size per square kilometre, a chunk made again, a brush stroke's frames, colliders, 2,000 characters walking, a chunk made on a worker against on the frame, and (with the `gpu` feature) uploads and the report's pictures through the engine's renderer.

It is a member of the workspace so it builds with the same libraries, lints and checks as everything else, but the engine never depends on it, and it adds no library of its own (`flate2` for the squeezed file size is already the program's).

## Running it

```
cargo test -p ground-study                       # the prototypes' tests
cargo run --release -p ground-study              # every measurement, as Markdown tables
WGPU_BACKEND=gl xvfb-run -a cargo run --release -p ground-study --features gpu -- uploads
WGPU_BACKEND=gl xvfb-run -a cargo run --release -p ground-study --features gpu -- pictures <folder>
```

`pictures` writes one PPM picture per way of meshing into the folder; the report's PNGs were made from them with ImageMagick (`convert x.ppm -colors 192 x.png`).

| Folder | What it holds |
| --- | --- |
| [`src/`](src/README.md) | The prototypes (the library) and the measuring program. |

## Must never

- Be depended on by the engine or the editor: what wins is rebuilt as the ground's own crate (Tools 1b, G1).
- Name the program: crate names are generic on purpose.

## Depends on

`glam`, `rayon`, `flate2`, and the program's `physics` and `reflect`; with the `gpu` feature, `render`, `assets`, `wgpu` and `pollster`.
