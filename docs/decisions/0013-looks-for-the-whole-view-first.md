# 0013. The standard and stylized looks, chosen for the whole view first

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

Phase 1's third milestone brings "a standard lit material and a stylized/toon option". Solis is aimed at stylized games (DESIGN.md §1), so both must look good. But where is the choice made? In most engines a toon look is a property of each material. Solis has no material assets yet: they arrive with the asset database in Phase 2, and imported models only carry glTF's own material settings, which have no toon option.

## Options

1. **Per material, now:** invent a place to store a choice per material before materials are assets. Thrown away in Phase 2.
2. **For the whole view, now; per material from Phase 2.** The viewport's look menu picks Standard, Stylized or Unlit for every model. When materials become assets, each gets its own setting, and the viewport menu becomes "as the materials say" plus overrides for checking.

## Decision

Option 2.

- **Standard:** glTF's metal-and-roughness material, as Blender, Substance Painter and most engines make it: colour, metal, roughness, normal, occlusion and glow textures, with GGX highlights.
- **Stylized:** cel shading: two to four flat bands of light with smooth edges, a crisp highlight on glossy surfaces, a rim of sky light, and dark outlines (back faces pushed outwards, with smoothed normals so boxes outline without gaps).
- **Unlit:** colours and textures as painted.

Light comes from a fixed sun and the sky light (decision 0008's first source of indirect light) until real lights arrive in milestone 4. The look is remembered between sessions.

## Consequences

- One look for everything in the view until Phase 2.
- Outlines cost a second draw of each solid model when they're on.
- The colour range is still clipped at white until tonemapping arrives in milestone 5, so very bright highlights flatten.
