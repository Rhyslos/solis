# assets

**Layer 1 · Engine**

Reads models and textures from files into plain data the renderer can upload: glTF 2.0 (`.gltf` with its `.bin` and images, or a single `.glb`), with PNG and JPEG textures.

## Purpose

Turn a model file into a `Model`: meshes already placed where the file's nodes put them, in Solis's units (metres, Z up; glTF's Y-up is turned so its up becomes Z), with materials (glTF's metal and roughness: colour, metal, roughness, normal, occlusion and glow, each with its texture; see-through handling; double-sided) and textures with every mip level worked out. Missing normals are filled in; triangle strips and fans become plain triangles; points and lines are left out with a warning.

`sample` builds models in code, for tests and for trying the importer: a textured cube on a red base, and a bigger scene for checking the looks (`showcase_glb`, also Help > Add the sample scene). `cargo run -p assets --example sample -- [--scene] out.glb` writes them to a file.

## Must never

- Crash on a damaged or hostile file. Every file is treated as hostile (CONTRIBUTING.md, rule 9): sizes are checked against `Limits` before anything big is allocated, every buffer, view and accessor is checked to fit before it is read, images are decoded with size and memory limits, and errors explain the problem in plain words. Fuzz tests feed it thousands of damaged files.
- Touch the GPU, the window or the editor
- Fetch anything from the internet: a model that refers to data elsewhere is refused

## Depends on

`gltf` (the file's structure only), `image` (PNG and JPEG), `base64`, `percent-encoding`, glam and bytemuck. No Solis crates.

## Status

Phase 1, milestone 3: glTF models with their full materials. Animation arrives in Phase 6.
