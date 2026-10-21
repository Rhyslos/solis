# 0007. MIT licence, and Solis stays separate from Aventail

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

Phase 0 needed two answers: which licence Solis uses, and whether Solis shares code with Aventail 3D (which already has a wgpu viewport, an egui interface with docking, and a theme) or keeps its own.

## Decision

**Solis is MIT licensed.** It's the norm for indie engines (Godot and Bevy use it) and lets anyone make commercial games with Solis without restrictions.

**Solis stays separate from Aventail.** No shared crates or common repository: each project changes at its own pace.

**Aventail's code may be copied into Solis and adapted.** Aventail is GPL, but its copyright belongs to its owner, Rhyslos: its only code authors are Rhyslos and Claude working for Rhyslos (one automated version bump by Dependabot aside). An owner can release their own code under another licence, so code copied from Aventail into Solis is MIT here. Copied code comes from the owner's own work only; anything Aventail took from elsewhere keeps its own licence.

**Third-party material keeps its licence.** Fonts copied from Aventail (Inter, and the fallback fonts egui bundles) are under the SIL Open Font License or similar, which allow use in an MIT project. Their licence texts go in `licenses/`.

## Consequences

- Dependencies must have licences compatible with MIT (MIT, Apache 2.0, BSD, ISC, Zlib and similar), checked with `cargo deny`, as in Aventail.
- If someone else ever contributes to Aventail, their code can't be copied into Solis without their permission. Copying is safe while Rhyslos is Aventail's only author.
