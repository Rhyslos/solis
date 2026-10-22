# Phase 1 review: performance, code and decisions

- **Date:** 2026-10-22, at the end of Phase 1 (after the finished-picture milestone, PR #21)
- **Covers:** every crate (`assets`, `render`, `editor`, `app`), the decisions in `docs/decisions/`, DESIGN.md, ROADMAP.md, CI and tools
- **How:** three independent read-throughs (render; editor and app; assets plus architecture), with the main claims checked against the code. Nothing was changed by the review itself.

## In short

The code is careful for its age:

- No `unwrap` or `panic` in the running editor, and every saved setting is cleaned on load and fuzz-tested.
- The frame loop never waits for the GPU.
- The colour, units, axis and light conversions check out against Blender's own importer.

Most decisions hold up. The problems are of three kinds:

1. **A handful of real bugs and holes to fix now** (cheap): ways a hostile model file can still exhaust memory or crash the editor, inward normals on mirrored meshes, a hard-coded program name, settings silently changed by Properties, depth flicker at distance.
2. **The shape of the data is Phase 1-shaped.** Models are baked into world space with no transforms, the scene is split between two structs, Properties edits state directly, and `workbench.rs` is 1,800 lines. That was right for "seeing things", but each of these has to change at the **start** of Phase 2, before entities, undo and prefabs are built on top.
3. **Decisions missing before Phase 2:** the ECS and its reflection, the scene format, the asset database and model structure, transforms, undo, the runtime/editor split, how hot reload of Lust works (decision 0016's 2-second goal is the hardest open problem), and CI now that the repository is public.

## 1. Fix now (one "Phase 1 fixes" pull request)

| # | Problem | Where | Why it matters |
|---|---|---|---|
| 1 | A buffer's real file size isn't checked against its declared size; image files share no memory budget; decoded textures have no total cap | `assets/src/reader.rs` (`read_limited`, image reads) | A small hostile `.gltf` can make the importer read the same 1 GB file a thousand times. The "never exhausts memory" promise in the assets README isn't true yet |
| 2 | A sparse accessor with no buffer view becomes `count` zeros, and positions are capped at 90M rather than 30M | `assets/src/reader.rs` accessor caps | A few hundred bytes can allocate several GB before the vertex limit is checked |
| 3 | One mesh can be placed up to a million times | `assets/src/geometry.rs` node walk | 1M meshes, then 3M GPU buffers made on the frame thread |
| 4 | One mesh over 256 MB of vertices crashes the editor (GPU buffer limit) | `render/src/gpu_model.rs`, egui-wgpu's default limits | The importer allows 30M vertices (960 MB) in one mesh; wgpu then panics |
| 5 | Normals made for mirrored meshes point inwards | `assets/src/geometry.rs` (`smooth_normals` runs before the winding flip) | Mirrored parts without normals look inside out in light |
| 6 | The program's name is written into a message | `app/src/window.rs` ("Solis imports .gltf…") | Breaks the renaming rule; use the branding module |
| 7 | Properties silently changes values that are allowed but outside the field's range (e.g. a sun at -45° becomes -10° just by being shown) | `editor/src/lighting.rs` DragValues | egui clamps existing values to the field's range by default |
| 8 | A crash inside an import thread leaves "Importing…" forever | `app/src/import.rs` | Nothing is sent back; catch it and report an error |
| 9 | The file dialog blocks the frame loop | `app/src/import.rs` (`FileDialog::pick_file`) | Breaks ground rule 10; use the async dialog |
| 10 | A model with enormous bounds turns the camera into NaN | `editor/src/camera.rs` (`frame`) | The radius can overflow to infinity |
| 11 | Depth precision: ~1.2 cm at 100 m, ~1.2 m at 1 km | `editor/src/camera.rs`, all pipelines | Flicker between ground and grid, and between close surfaces, at distance. Reversed-Z is a few lines now, many later |
| 12 | All viewports share one uniform buffer and shadow map | `render/src/renderer.rs` | Correct today only because each render is submitted separately; breaks with a second viewport recorded together or rendering on threads. Give each target its own |
| 13 | A GPU buffer is created every frame for lamp markers | `render/src/renderer.rs` | Small, but avoidable: keep one buffer |
| 14 | Documents out of date | `deny.toml` (still names Aventail's decisions and a GPL licence), "the repo is private" in DESIGN/CLAUDE/CI comments, old phase numbers in decisions 0002 and 0011 and `scale-check.sh`, DESIGN.md:5 "no code yet", ARCHITECTURE "assets planned" | Misleads future sessions |

## 2. Before Phase 2 starts (decisions and one refactor)

Phase 2 builds entities, scenes, prefabs, the asset database and undo. These decide its shape, so they should be settled first, each as a decision record:

1. **The ECS and reflection (high).** Write the requirements down before choosing `bevy_ecs` or Solis's own: components defined at run time (Lua), known by a stable name and field layout rather than Rust's `TypeId` (hot reload, saved scenes), a fixed iteration order (lockstep), and self-description for the inspector. These point to run-time registered components, or a thick reflection layer either way.
2. **Hot reload of Lust (high).** Decision 0016 has Lust built as a Rust library. Swapping a Rust library in a running program is fragile: Rust has no stable ABI, `TypeId`s change on every build, Windows locks loaded DLLs, linking with MSVC is slow, and `libloading` needs `unsafe` (which rule 7 only allows with a decision). The workable design is "save the world, swap the library, restore", through the reflection layer in item 1. So it has to be designed now, though it's built in Phase 4. Two seconds is optimistic, and worth measuring early.
3. **The model and asset structure (high).** Today an imported model is a flat list of meshes baked into world space: hierarchy, names, local transforms and shared meshes are lost. That blocks prefabs (a glTF should become an entity tree), stable sub-asset IDs (re-importing renumbers everything), instancing (a mesh used 100 times is stored 100 times), per-mesh culling, and picking (no CPU copy is kept). Proposed: unique meshes in local space, a node tree, mesh instances, and IDs per sub-asset by glTF index and name.
4. **Transforms on the GPU (high).** Meshes have no transform at all, so a moving unit can't be drawn without re-uploading it. Add a per-instance transform buffer from the start (not a per-draw uniform), so Phase 5 adds batching, culling and LODs without rewriting the vertex path. Today's one-draw-per-mesh structure would cost roughly 15–40 ms of CPU per frame at the 2,000-unit target.
5. **One scene document, and undo (high).** Models live in the Workbench, while lamps, sun and environment live in the Viewport's fields. Properties edits change them directly. Phase 2 needs one world or scene, with every edit going through commands (a whole drag merged into one undo step), and the self-describing inspector on top. Don't extend the hand-written Properties forms further; they get replaced.
6. **The scene format:** RON or TOML (both already used), the ID scheme, float formatting that round-trips exactly, schema versions and migration, how prefab overrides are written.
7. **The runtime/editor split (medium).** There's no player crate: `app` owns the frame loop through eframe and depends on the editor. Exported games need their own loop without egui. Settle where the frame loop and play mode live.
8. **Selection as entities (medium).** The Selection enum (Sun, Environment, Lamp, Model) can't do multi-select or gizmos, and mixes an asset (a model) with an instance of it. It becomes a set of entity IDs.
9. **CI now the repository is public (medium).** Decision 0001 was about saving private minutes. A new decision could run checks on every push to a pull request, test on Windows (the real target; DX12 is never tested today) on ready pull requests, add a weekly security check and Dependabot, and publish EXEs as Releases (Actions downloads need a GitHub login).
10. **The scale target's reference PC (low).** Name the hardware "60 fps" is measured on, and make DESIGN §2 and ROADMAP agree on the numbers. Give the scale check a model-heavy scene; it measures an empty one today.

**Refactor, as its own pull request:** split `editor/src/workbench.rs` (1,814 lines, against the ~500-line guideline) into the console, top bar, Scene panel and Properties, before Phase 2 adds entities to it. Also merge missing panels into saved layouts instead of resetting them (decision 0004 says so; today the first new panel would reset everyone's layout).

## 3. Later, at the phase that needs it

| When | What |
|---|---|
| Phase 2 (asset database) | Mips filtered in linear space, alpha premultiplied for cut-outs, a parallel import cache with BC7/BC5 compression; one worker pool for imports instead of a thread each; shared default textures and samplers; read vertex colours, tangents, emissive strength, texture transforms and `EXT_mesh_gpu_instancing`, and say which glTF extensions a file needs instead of "refers to parts that don't exist"; refuse `../` paths outside the model's folder |
| Phase 2 | Blended (see-through) materials and the tone-mapping mask: glass over the backdrop is part tone mapped; sort see-through meshes back to front; GPU tests for see-through and cut-out materials |
| Phase 3 | Redraw only when something changes (the editor redraws every frame, even idle: hard on laptop batteries); one GPU image per viewport; a virtualised Scene tree for thousands of entities; a borrowed `ViewportView` instead of copying the scene every frame |
| Phase 5 | Batching by mesh and material, frustum culling, LODs; cascaded shadow maps (one map fitted to every model gives 1.4 m texels on a 2 km battlefield) with bias in world units; split the shader file |
| Phase 9 | Draw the editor's overlays after tone mapping instead of the alpha-mask trick; tone-mapped MSAA resolve; keep the depth image for soft particles |

## 4. What holds up

- **Layering and boundaries:** the editor never touches the GPU, the window or files; the renderer knows nothing of the editor; layer-check enforces it.
- **Hostile-file stance:** size checks before reading, overflow-safe accessor maths, a node walk without recursion, image decode limits, NaN clean-up, fuzz tests. Section 1 closes the remaining holes.
- **Maths checked against Blender:** Y-up to Z-up, mirrored transforms, and the light units (lux/683 for suns, candela × 4π/683 for lamps) match Blender's glTF importer exactly.
- **Rendering structure:** the HDR image with a last pass (bloom slots in for Phase 9), direct and sky light kept apart (decision 0008), store-discard on MSAA images, careful OpenGL fallbacks, uniform layouts covered by tests.
- **Decisions** 0005 (netcode in layers), 0008 (swappable indirect light), 0015 (file extensions), 0017 (HDR with a mask) and Windows-only (which makes lockstep determinism easier).
- **Testing culture:** pixel tests for every look, the lights, sky, fog and tone mapping; kittest for the editor; fuzz tests for every file and setting read.
