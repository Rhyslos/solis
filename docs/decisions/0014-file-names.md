# 0014. File names: project.solis, .solscene, .solprefab, .solmat

- **Status:** Accepted
- **Date:** 2026-10-22

## Problem

Phase 2 brings projects, scenes, prefabs and materials saved as files. Their names are needed before then by other programs: Aventail 3D exports models straight into a game's project folder and needs a way to recognise a Solis project, as it recognises Godot's by `project.godot`.

## Decision

The project lead's choice:

| What | File |
|---|---|
| Project | `project.solis`, one per game, in the project's top folder (like Godot's `project.godot`) |
| Scene | `name.solscene` |
| Prefab | `name.solprefab` |
| Material | `name.solmat` |

- The names live in `crates/app/src/branding.rs` (`PROJECT_FILE`, `SCENE_EXTENSION`, `PREFAB_EXTENSION`, `MATERIAL_EXTENSION`), as they carry the program's name, and are listed in `RENAMING.md`.
- What goes inside these files (text made to merge, decision 0006) is decided at the start of Phase 2. Only the names are settled here.

## Consequences

- A folder with `project.solis`, or a folder inside one, is a Solis project. Aventail 3D uses this to export into it with its Solis settings (Aventail's decision 0122).
- Every kind of file says what it is from its name alone, so the editor's Library, Git's merge driver (decision 0006) and Windows can each tell them apart.
- Renaming the program would mean renaming these, and old projects would need converting; see `RENAMING.md`.
- Models stay in standard formats (glTF), and textures in image formats; they get no Solis extension.
