# 0048. Materials and shaders get a phase, between light and effects

- **Status:** Accepted
- **Date:** 2026-10-26

## Problem

Light (Phase 8) and effects (now Phase 10) meet in materials: how a surface takes light, glows, gets wet or sways in the wind.

Today a material is whatever a glTF file brings, and the look (standard, stylized, unlit) is chosen for the whole view (decision 0013). DESIGN.md §7 put a node-based shader editor after 0.1, although "stylized games live on custom shaders".

The project lead asked for this to be **a general tool people can use in creative ways, not a feature for one need.** A plant that glows as night falls should be one use of a tool that drives any material value from the world, not a "bioluminescence" switch.

## Options

1. **Keep materials as they are;** the shader editor after 0.1. Effects would invent their own materials.
2. **A phase for materials and shaders after light, before effects.** Materials become assets, any value can be driven by the world, and a node graph makes shaders. Effects then draw with the same materials.
3. **Fold materials into the light phase.** It is already the largest phase.

## Decision

Option 2, chosen by the project lead. The order becomes:

| Phase | Content |
| --- | --- |
| 7 | Sound and animation |
| 8 | Light, sky and atmosphere |
| **9** | **Materials and shaders** |
| 10 | Effects (Solstice) |
| 11 | Game UI |
| 12 | Netcode |
| 13 | Scripting, export and Git |
| 14 | Solis 0.1 |

The phase's working plan is [docs/plans/phase-9-materials.md](../plans/phase-9-materials.md). Its choices are still to be answered.

## Consequences

- **0.1 comes about one and a half months later** again: about eighteen months in all, as a guide.
- **One graph editor:** materials, the animator (Phase 7) and effects share it, saved as readable text (decision 0039's rule).
- **The light plan provides the inputs:** the light reaching each surface, wetness and the looks. This phase lets any material read them.
- **The shader editor leaves DESIGN.md §7's "later" list.**
