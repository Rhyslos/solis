# assets

**Layer 1 · Engine**

Reads models and textures from files into plain data the renderer can upload: glTF 2.0 (`.gltf` with its `.bin` and images, or a single `.glb`), with PNG and JPEG textures.

## Purpose

Turn a model file into a `Model`: each mesh once, in its own space, and a tree of nodes placing them (a mesh used many times is stored once; glTF's node placement `T` becomes `C·T·C⁻¹` in Solis's axes; `Model::placed_meshes()` bakes the old flat view), in Solis's units (metres, Z up; glTF's Y-up is turned so its up becomes Z), with materials (glTF's metal and roughness: colour, metal, roughness, normal, occlusion and glow, each with its texture; see-through handling; double-sided) and textures with every mip level worked out. Lights the file carries (`KHR_lights_punctual`, as Blender exports them: suns, point lights and spotlights) come in placed, in Blender's units (`lights.rs`). Missing normals are filled in; triangle strips and fans become plain triangles; points and lines are left out with a warning.

`sample` builds models in code, for tests and for trying the importer: a textured cube on a red base, a bigger scene for checking the looks (`showcase_glb`, also Help > Add the sample scene), and the benchmark's ship (`ship_glb`). `cargo run -p assets --example sample -- [--scene] out.glb` writes them to a file.

## Must never

- Crash on a damaged or hostile file. Every file is treated as hostile (CONTRIBUTING.md, rule 9): sizes are checked against `Limits` before anything big is allocated (a buffer only ever reads the bytes it declares, buffers and image files share one byte budget, decoded textures another, and corner counts are checked from the file's own numbers before reading), every buffer, view and accessor is checked to fit before it is read, images are decoded with size and memory limits, files are only read from the model's folder or below, and errors explain the problem in plain words. Fuzz tests feed it thousands of damaged files.
- Touch the GPU, the window or the editor
- Fetch anything from the internet: a model that refers to data elsewhere is refused

## Depends on

`gltf` (the file's structure only), `image` (PNG and JPEG), `base64`, `percent-encoding`, `meshopt` (meshoptimizer: levels of detail), glam and bytemuck. No Solis crates.

## Status

Phase 1 complete: glTF models with their full materials and lights, read within memory limits (and meshes split to fit the graphics card's buffers). Phase 2: meshes in their own space with a node tree. Phase 5: levels of detail (`lods.rs`, decision 0032): each mesh of an imported model gets up to three simpler versions (`Mesh::lods`, about a half, a quarter and an eighth of the triangles, sharing its corners, with how far each strays), made with meshoptimizer within limits (64 to 4 million triangles; a level must shrink by a fifth or simplifying stops); nodes named `Name_LOD1` … give their meshes to the node `Name` instead (moved into its space), and aren't drawn themselves. Animation arrives in Phase 7.
