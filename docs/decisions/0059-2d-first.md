# 0059. 2D first: 3D animation waits until 2D is complete

- **Status:** Accepted (changes the order of 0057 and 0054; keeps their designs)
- **Date:** 2026-10-28

## Problem

Phase 7 was "sound and animation": six sound milestones, then six for 3D skeletal animation (skins from glTF, skinned drawing, crowds, the state machine, the animator graph editor, cues), then the gate's level. The project lead's own game is 2D (a farming and life game, decision 0054), and Solis is being shaped so that game can be built in full in it. Skeletal animation is 3D-only work that the 2D game doesn't use, and it stood between today and 2D.

## Options

1. **Keep the order:** finish 3D animation in Phase 7, then compatibility, then 2D. 2D waits for a whole 3D feature.
2. **2D first (chosen):** Phase 7 becomes sound only. The parts of animation that 2D needs (the state machine, the animator graph editor, events on frames that play sounds) are built in Phase 9 with frame animation, for sprites first and made so skeletons use them later. 3D skeletal animation becomes its own phase after the 2D phases and the rest of what a 2D game needs (Phase 16, after Solstice).
3. **Drop 3D animation** from the plan. Solis is a 3D engine (DESIGN.md §2), so no.

## Decision

Option 2, chosen by the project lead: "only when the 2D part of the engine is complete, do we move onto the 3D parts."

- **Phase 7: Sound.** S1–S6, S8 and S7, now "the gate's level": footsteps from game code, music, a radio ducking the music and a cave with a reverb zone in the playground. Its gate is sound-only.
- **Phase 8:** compatibility, unchanged.
- **Phase 9, frame animation** gains the groundwork that was A4, A5 and A6's cues: the animation state machine, the animator graph editor (the shared graph editor of decision 0050, mocked up first) and frame events that can name a sound event. Each is built to drive any clip with frames, events and a length, so 3D reuses it unchanged.
- **Phase 16: 3D animation** (new): skins and clips from glTF, skinned drawing, crowds with baked clips, blending and blend spaces for skeletons, and cues and footsteps on skeletons. Decision 0057's design for these stands, and its research is kept in the animation plan.
- Netcode becomes Phase 17, scripting, the full export and Git Phase 18, and the 0.1 release Phase 19.

## Consequences

- 2D is reached about six milestones sooner, and Solis is "2D game ready" before light, materials and effects only add to it.
- The state machine is designed with only sprite frames in use at first. To keep 3D cheap later, its clips are read through one small interface (frames, events, length), as decision 0057 already asked, and its tests include a pretend clip that isn't a sprite.
- Footsteps in Phase 7's gate come from game code (a timer or the walk input), not from animation frames; footsteps on the right frame arrive with frame events in Phase 9.
- Earlier decisions keep their old phase numbers as written; this one says where things went.
