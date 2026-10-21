# 0006. Git built into the editor, for teams

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

Games are made in groups. People need to share their changes, see what others changed, and combine work without breaking scenes. In most engines this is painful: scene files merge badly, big binary files bloat the history, and two people editing the same model by accident lose work.

## Decision

Git, built into the editor, with project files designed for it.

**Project files made for Git** (from Phase 2, so this is never retrofitted):
- **Scenes and prefabs are text,** one entity per block with stable IDs and fields in a fixed order, so changes show as small, readable differences and most merges just work.
- **Each asset's ID lives in a small text file beside it,** not in one shared database file every change would touch.
- **Generated files are never committed:** imported and compressed assets and builds live in a cache Solis can rebuild.
- **New projects come with a `.gitignore` and `.gitattributes`,** with Git LFS set up for models, textures and sound.

**A Git panel in the editor** (dockable like every panel, decision 0004):
- Changed files, with a plain-language summary of scene changes ("Carrier_01 moved", "Health.maxHp 4000 → 5000"), not just text lines.
- Commit, pull and push, branches, and history with one-click going back to an earlier version of a file.
- **Locking** binary assets (Git LFS locks), so two people can't edit the same model at once. The library shows who has what locked.
- Markers in the library and the scene tree for new, changed and locked items.
- New project from a repository, and publish a project to GitHub, GitLab or any Git server.

**Merging scenes:** Solis registers its own merge driver for scene and prefab files. It merges entity by entity and field by field. When two people changed the same field, the editor asks which to keep, side by side, instead of leaving conflict markers in the file.

**How Solis talks to Git:** through the `git` command and Git LFS, installed alongside Solis if missing. Unlike libraries such as libgit2 or gitoxide, the command line handles LFS and sign-in (Git Credential Manager) exactly like every other Git tool, and anything the panel can't do still works from a terminal.

## Consequences

- The scene format (DESIGN.md §8, ROADMAP.md Phase 2) must merge well; this is now a hard requirement, tested with real three-way merges.
- Git LFS hosting has storage limits on free plans (GitHub's is small); the docs should say so.
- Perforce, which big studios use, is out of scope.
