# 0032. LODs are made on import, or come from the model

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

Thousands of units can't all be drawn in full detail: far away, a ship a few pixels high doesn't need its 20,000 triangles. Simpler versions (levels of detail, LODs) are needed. Where do they come from?

## Options

1. **Automatic only:** Solis simplifies each mesh itself.
2. **The model's own only:** only models made with LODs get them.
3. **Both:** Solis makes them on import; a model that brings its own uses those instead.

## Decision

Option 3, chosen by the project lead (2026-10-24).

- **Made on import:** two or three simpler versions of each mesh (about half, a quarter and an eighth of the triangles), with `meshopt` (meshoptimizer, MIT or Apache), kept in the asset cache so it's done once. Within a time and size limit per mesh; a mesh that can't be simplified well keeps only its full version.
- **The model's own:** meshes named `Name_LOD1`, `Name_LOD2` … in the file (as Aventail and Blender can export) are used instead, and aren't drawn as parts of their own.
- **Choosing:** by the copy's size on screen, with a little overlap so a copy at the boundary doesn't flicker.

## Consequences

- Any model gets cheaper far away with no work, and artists who want control make their own.
- The asset cache grows a little per model.
- The manual explains LODs and how to make your own.
