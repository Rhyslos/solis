# docs/plans

Working plans for the phases of [ROADMAP.md](../../ROADMAP.md). Phase numbers stop at 12; later phases are named by track (`2D 1`, `Shared 7`, `3D 2`; [decision 0097](../decisions/0097-three-tracks.md)), and a plan written before then may still say "Phase 13" to "Phase 19" inside: ROADMAP.md's "was" notes say which track each became. A plan records the choices made before any code, the controls the project lead chose, and a plan for each milestone, so a later session can carry on where the last one stopped. Milestones are ticked off here as they are built. A plan marked "draft" here still has questions for the project lead.

| File | Phase or track |
| --- | --- |
| `phase-2.md` | 2: The world (entities, undo, the asset database, areas as text, assemblies) |
| `phase-3.md` | 3: The editor |
| `phase-4.md` | 4: Lust and Lua join |
| `phase-5.md` | 5: Scale |
| `phase-6.md` | 6: Simulation and play |
| `phase-6-hardening.md` | The hardening milestone between Phases 6 and 7 (decision 0042) |
| `phase-7-sound.md` | 7: Sound |
| `phase-7-animation.md` | Animation: the state machine and graph editor, built in 9 (2D); skeletons in 3D 1, not started (decision 0059) |
| `compatibility.md` | 8: Compatibility (built) |
| `2d.md` | 9 (built), and 2D 1 and 2D 2 (was Phase 10; to come, with parallax built early): first-class 2D |
| `lust-data.md` | Usable first, 2: what Solis makes is Lust (decision 0081); begun: tile maps and tilesets built, the other files to come |
| `launcher.md` | Usable first, 7: the launcher before game UI (decision 0086), built; `launcher-*.png` are its mock-up |
| `editor-overhaul.md` | Usable first, 7: the editor overhaul before the game UI editor (decision 0085), built; `editor-overhaul-level.png` is its mock-up |
| `game-ui.md` | 12: Game UI (built in "Usable first", step 7) |
| `light.md` | 3D 2 to 5: light, sky and atmosphere, with its 2D light in 2D 2 and its weather, bloom and grading in Shared 9 (draft; not started) |
| `materials.md` | Shared 7: Materials and shaders (draft; not started) |
| `effects.md` | Shared 8: Effects (draft; not started) |
| `roadmap-2d-3d.md` | No phase: where 2D and 3D each stand (built, planned, later, not talked about yet), a snapshot from 8 November 2026 |
| `shared-1-ui-remake.md` | Shared 1a to 1d: the editor's UI remade to the project lead's design ([`docs/design/g-layout/`](../design/g-layout/README.md), decisions 0099 to 0101): what the project lead has decided, the sixteen milestones M1 to M16, the placeholder mechanism, what happens to saved settings, the mock-up rounds, and the decisions still open. Begun: M1 (the look) built |
| `shared-1-ui-remake-inventory.md` | The plan's companion: every piece of today's interface and where it goes, every element of the design and what stands behind it, the keys, the design's tokens, and what egui doesn't give for free |
| `shared-1-ui-remake-handover.md` | Where the UI remake stands (paused 2026-11-11 with M1 to M3 merged) and how to pick it up: what to ask the project lead first, what M3 left, the builder brief, and the assignment for M4. |
| `ideas.md` | Shared 2 to 6, 10 and 11, 2D 1 and 3D 1: tools that help make games (timers, tween, tags and more), each idea marked with the milestone that builds it (decision 0098) |

Name a new plan after its phase (`phase-N.md`) or its subject, and add it here.
