# assets

**Layer 1 · Engine**

Reads models and textures from files into plain data the renderer can upload: glTF 2.0 (`.gltf` with its `.bin` and images, or a single `.glb`), with PNG and JPEG textures.

## Purpose

Turn a model file into a `Model`: meshes already placed where the file's nodes put them, in Solis's units (metres, Z up; glTF's Y-up is turned so its up becomes Z), with materials (a colour, a colour texture, see-through handling, double-sided) and textures with every mip level worked out. Missing normals are filled in; triangle strips and fans become plain triangles; points and lines are left out with a warning.

`sample` builds a small model in code (a textured cube on a red base), for tests and for trying the importer: `cargo run -p assets --example sample -- sample.glb`.

## Must never

- Crash on a damaged or hostile file. Every file is treated as hostile (CONTRIBUTING.md, rule 9): sizes are checked against `Limits` before anything big is allocated, every buffer, view and accessor is checked to fit before it is read, images are decoded with size and memory limits, and errors explain the problem in plain words. Fuzz tests feed it thousands of damaged files.
- Touch the GPU, the window or the editor
- Fetch anything from the internet: a model that refers to data elsewhere is refused

## Depends on

`gltf` (the file's structure only), `image` (PNG and JPEG), `base64`, `percent-encoding`, glam and bytemuck. No Solis crates.

## Status

Phase 1, milestone 2: glTF models with colour textures. The rest of the material (metal, roughness, normal maps) arrives with materials in milestone 3; animation in Phase 6.
