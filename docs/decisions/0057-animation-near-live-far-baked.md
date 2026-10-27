# 0057. Animation: live poses near, baked clips far, and one state machine for 2D and 3D

- **Status:** Accepted
- **Date:** 2026-10-27 (answers given 2026-10-25)

## Problem

Phase 7 brings animation (ROADMAP.md; DESIGN.md §10):
- skeletal animation from glTF;
- state machines;
- blending;
- animation events.

It has to work at the scale target (DESIGN.md §2), where 10,000 units at the stress level can't each have every bone worked out and sent every frame. The Phase 7 animation plan ([docs/plans/phase-7-animation.md](../plans/phase-7-animation.md)) asked eleven questions.

Since then, first-class 2D was agreed (decision 0054): sprite frames must use the same state machine.

## Decision

Chosen by the project lead: the recommendations, except the two marked.

1. **Live poses near, baked clips far.**
   - Near units get every bone worked out and blended each frame.
   - Far units play clips baked once, when the model loads (a pose per 30th of a second, kept on the graphics card), and say only "clip, frame and the share to the next".
   - Solis chooses per unit, by size on screen, within a budget.
2. **Footstep moments (cues) are written per clip** in the model's `.solmeta` sidecar, on a clip timeline in Solis.
3. **Blend spaces by one number and by two** (speed; forwards and sideways for strafing), both in Phase 7. *(Changed from the recommendation, which had two numbers later.)*
4. **Clips from other files are matched by bone names.** Missing bones stay at rest, and Problems names them.
5. **No root motion in Phase 7:** game code moves characters; animation is how they look, plus their cues.
6. **The animator** is saved as `.solanim` (readable text, decision 0039's rule for graphs) and placed on an entity by a built-in `Animator` component.
7. **The graph editor's controls:**
   - left-drag moves a state;
   - left-drag on empty space draws a selection box;
   - drag from a state's edge, or right-click > Make transition, adds a transition;
   - right-click on empty space adds a state or a blend space;
   - middle-drag pans;
   - the wheel zooms around the pointer;
   - F frames everything;
   - Delete removes;
   - double-click edits a state's clip.

   These become the shared graph editor's controls for every graph in Solis (decision 0050).
8. **Previews:** the viewport animates only with **Preview animation** on (decision 0026). The animator tab's own preview always plays, with a slider per parameter.
9. **Game code's names** (`solis/animation`):
   - `setAnimNumber`, `setAnimSwitch`, `triggerAnim`, `setAnimSpeed`;
   - `jumpToAnimState`, `animStateOf`, `animTimeOf`;
   - and the event `AnimationCue(string name)`.

   Where a bone is comes later.
10. **The gate's character is the project lead's own.** *(Changed from the recommendation.)* No Solis-made robot goes into the playground or samples. Automatic tests still need a small skinned model, which Solis makes in code for the tests only.
11. **glTF only;** FBX later (Blender converts it).

**One state machine for 2D and 3D** (decision 0054). The animator drives any *clip*: something with a length, frames or keys, cues, and a way to be sampled. A 3D clip gives bone poses; a 2D clip (from Aseprite, Phase 9) gives a sprite frame. States, transitions, parameters, blend spaces, cues, saving and the graph editor are shared. A4 builds it this way, and its tests include a stand-in "frame clip" so the 2D path is proven before 2D exists.

**The state machine is game state; poses are not.** Animators move on once per tick from the parameters game code set. Their state is in the checksum and in saves. Cues reach game code and the sound system in a fixed order. Poses are only for drawing.

## Consequences

- **A new `animation` crate** (layer 1): sampling, blending, the state machine, `.solanim`. It has no GPU and no editor.
- **Instances grow from 52 to 64 bytes,** to say where their pose is. Bone matrices go into a floating-point texture, so skinning works on every card, including the software driver CI uses.
- **Saves become `solis save 2`,** still reading version 1, and checking against the compatibility work that follows (decision 0053).
- **The animator tab is a new editor window,** so it's mocked up for the project lead before it's built (CLAUDE.md). Its graph editor is built as the shared one, to be reused by materials, effects and anything later.
- **The roadmap's two animation milestones become six** (A1–A6).
