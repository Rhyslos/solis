# docs/reports/after-the-rethink

The research behind [the plan after the rethink](../../plans/after-the-rethink.md), asked for by the project lead on 17 November 2026. It covers everything left after the editor rethink: the rest of 2D, the shared tools, and the rest of 3D. For each milestone it says what it gives, what already exists in Solis, two or three ways to build it compared (rule 22), how to build it in Solis's own code, its tests and scale cases, the questions for the project lead, and its rough size. It was read against `origin/main` at e763513. The plan says what was checked and corrected. Sources are listed at the end of each file; other products are named only there, or as technical facts (rule 20).

| File | Phase |
| --- | --- |
| `00-overview.md` | What runs through every phase, the order and its problems, the dependency graph, every question (Q1 to Q81) and every source |
| `01-usable-first.md` | Finishing "Usable first": the rest of the files as Lust (L2 to L6) and the gate |
| `02-shared-2.md` | Shared 2: tile map questions, tags and world questions, spawning with values, game tests |
| `03-2d-1.md` | 2D 1: rooms on a world map, sorting tall things, advanced tiling, pathfinding |
| `04-2d-2.md` | 2D 2: triggers, platformer and grid-step controllers, soft shadows, 2D bounced light, polish |
| `05-shared-3.md` | Shared 3: timers, tweens, states that start and stop, shake and fade, noise and weighted random |
| `06-shared-4.md` | Shared 4: field hints, more kinds of settings, data tables, paths, drawing in the editor, save slots |
| `07-shared-5.md` | Shared 5: text layout and effects, button pictures, tooltips and drag and drop, translations, music |
| `08-shared-6.md` | Shared 6: rooms and moving between them, pools, debug drawing and the cheat box, bug reports |
| `09-shared-7.md` | Shared 7: materials and shaders |
| `10-shared-8.md` | Shared 8: effects |
| `11-shared-9.md` | Shared 9: bloom, white balance, grading, weather and moods |
| `12-shared-10.md` | Shared 10: the cutscene timeline and behaviour for characters |
| `13-tools-1b.md` | Tools 1b: the sculpted ground (G1 to G5) |
| `14-open-worlds.md` | Open worlds (W1 to W5), revised from decision 0117 |
| `15-3d-1.md` | 3D 1: skeletal animation from glTF, navmeshes and avoidance |
| `16-3d-2-3.md` | 3D 2 and 3D 3: the frame's shape, exposure, the physical sky, night, clouds, ambient occlusion, shadows |
| `17-3d-4-5.md` | 3D 4 and 3D 5: many lamps, fog, reflections, wet surfaces, bounced light |
| `18-shared-11-13.md` | Shared 11 to 13, briefly: what scripting, export, Git, mods, netcode and 0.1 need from earlier phases |
