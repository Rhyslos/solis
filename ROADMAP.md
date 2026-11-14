# Solis roadmap

The plan from an empty repo to **Solis 0.1**, in phases. The engine is Rust; games are written in Lust, Rust or Lua ([decisions 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md) and [0003](docs/decisions/0003-games-in-rust-lust-or-lua.md)). DESIGN.md says what Solis is; this file says in what order it gets built.

**No timeframe:** phases are done when they're done (the project lead, 2026-10-27). A phase is finished when the project lead has tried its gate by hand. Light, sky and atmosphere got a phase of their own ([decision 0046](docs/decisions/0046-light-before-netcode.md)), and so did materials and shaders ([decision 0048](docs/decisions/0048-materials-and-shaders-before-effects.md)). Compatibility, first-class 2D, a playtest export and game UI then moved to the front, so the project lead's own 2D game can move to Solis ([decisions 0053](docs/decisions/0053-compatibility-before-2d.md) and [0054](docs/decisions/0054-first-class-2d.md)). Plans for phases not started are named by subject (`docs/plans/light.md`), since the order can still change.

Each phase ends with a **gate**: something the project lead tries by hand before the next phase starts. Each milestone inside a phase is one pull request.

## At a glance

| Phase | What you get |
| --- | --- |
| 0. Groundwork | The editor window opens, in Solis's theme |
| 1. Seeing things | Models, materials, lights and shadows in the viewport |
| 2. The world | Entities, components, areas and assemblies, saved as text |
| 3. The editor | Building an area by hand, with play mode |
| 4. Lust and Lua join | Components written in Lust or Lua, in play mode within 2 seconds |
| 5. Scale | 2,000 units and 20,000 projectiles at 60 fps; 10,000 and 100,000 still playable; the ceiling measured |
| 6. Simulation and play | Physics, input, a deterministic simulation |
| 7. Sound | Sound events, buses and ducking, sounds in the world, reverb zones, listeners |
| 8. Compatibility | Updates never lose work: backups before upgrades, old files keep opening, saves survive updates, the game-code interface guarded, real projects tested before an update ships |
| 9. 2D, part 1 | Pixel-perfect 2D, sprites from Aseprite, frame animation with the animation state machine and graph editor, tile maps that change as you play, autotiling, the tile painter, top-down movement, the 2D camera, lamps on sprites |
| 11. Playtest export | Games packaged for playtesting, without the editor |
| 12. Game UI | Sungraph UI: HUDs and menus drawn in Solis |
| *Three tracks (decision 0097)* | |
| Shared 1a. The look and the frame | The editor in the project lead's own design: one dark theme with five accents, then the tab bar, ribbon, editor groups, shelf and status bar; unfinished parts marked, and a setting that hides them |
| Shared 1b. The everyday screens | Home (building and placing), the Library, and the File pages: projects, Preferences, Project settings and Input |
| Shared 1c. 2D and its editors | Tile maps, tilesets and pictures' crops, sprite clips and animators, and game screens, each in the new frame |
| Shared 1d. The rest, finished | Audio and Debug; the screens not built yet as placeholders; the manual rewritten, the old code out, the review |
| Shared 2. Game code: finding and making things | Tile map questions, tags and world questions, spawning with values, game tests |
| 2D 1. The world and tiles | Levels and the world map, advanced tiling, pathfinding |
| 2D 2. Movement, light and polish | Platformer and grid-step controllers, triggers, soft shadows, 2D bounced light, the 2D sample: 2D complete |
| Shared 3. Time and feel | Timers, tween, steps in order, states, shake and fade, noise and weighted random |
| Shared 4. Data and the editor | Data tables, save slots, field hints, more kinds of settings, drawing in the editor, paths |
| Shared 5. Screens, words and sound | Text effects, button pictures, tooltips and drag and drop, translations and subtitles, music that follows the game |
| Shared 6. Worlds and testing | Rooms, pools, debug drawing and a cheat box, bug reports with recordings |
| Shared 7. Materials and shaders | Materials for sprites and models, values driven by the world, a node graph for shaders |
| Shared 8. Effects | Solstice: particles, trails, flipbooks; a pixel mode for pixel art |
| Shared 9. Weather and the finished picture | Weather and moods, bloom and grading, in 2D and 3D |
| Shared 10. Cutscenes and behaviour | A cutscene timeline, behaviour drawn as diagrams |
| *The rethink* | The project lead's big testing pass |
| Shared 11. Scripting, full export, Git and mods | The scripting window; finished games for Windows; teams through Git; mods |
| 3D 1. 3D animation and navigation | Animated characters from glTF; navmeshes |
| 3D 2 to 5. Light, sky and atmosphere | A physical sky, night and clouds; shadows, lamps, fog, reflections; bounced light |
| Shared 12. Netcode | Two PCs playing together over the internet |
| Shared 13. Solis 0.1 | The flagship space battle, docs, release |

Lust joins in Phase 4, right after the editor, so game logic can be written in Lust from then on ([decision 0016](docs/decisions/0016-how-lust-joins-solis.md)). Phases 0–3 don't need Lust; if it isn't ready by Phase 4, Phase 5 (Scale) goes first.

---

## Phase 0: Groundwork

- A Cargo workspace with layered crates and a test that enforces the layers (as in Aventail).
- CI following [decision 0001](docs/decisions/0001-builds-on-demand.md): ready pull requests and manual runs only.
- A window (winit) with wgpu drawing into it.
- The egui editor shell, as in [decision 0004](docs/decisions/0004-editor-layout.md): the one-row top bar with document tabs, and dockable panels (`egui_dock`) for Area, Properties, the viewport, the library and the console, empty for now. Layouts saved per computer, with the three presets.
- **The theme:** dark greys and one accent colour, kept in one file so it's easy to adjust.

**Decided:** MIT, and Aventail's foundation code is copied in and adapted rather than shared ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).

**Gate:** the editor opens on your PC, panels move and remember their places, and the theme feels right.

## Phase 1: Seeing things

**Decided:** lighting is a choice per project, with real-time lights and sky light first ([decision 0008](docs/decisions/0008-lighting-is-a-project-choice.md)); the camera flies with Unreal's controls ([decision 0009](docs/decisions/0009-fly-camera-controls.md)).

Milestones, one pull request each:

1. **Fly camera:** moving around the viewport, with the speed shown and remembered.
2. **Models:** importing glTF models and textures, drawn with a simple material.
3. **Materials:** a standard lit material and a stylized/toon option.
4. **Lights and shadows:** directional, point and spot lights; shadows from the sun; ambient light from the sky.
5. **The finished picture:** HDR, tonemapping, sky and fog.

**Gate:** a model from Aventail or Blender looks good in the viewport, in both the standard and the stylized look.

## Phase 2: The world

The working plan, with the order milestones are built in, is [docs/plans/phase-2.md](docs/plans/phase-2.md).

- Entities and components, in an area document ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)).
- Transforms and parenting.
- Components that describe themselves, so the inspector can show any component, built-in or custom.
- Areas as text files that merge cleanly in Git: one entity per block, stable IDs, fields in a fixed order ([decision 0006](docs/decisions/0006-git-built-in.md)).
- New projects come with a `.gitignore` and `.gitattributes` (Git LFS for models, textures and sound); generated files stay out of the repository.
- Assemblies: nested, with per-instance overrides.
- The asset database: stable IDs kept in a small text file beside each asset, so renaming a file doesn't break anything; assets reload when changed on disk.
- Undo and redo for every edit.

**Decided:** an area document now and the game's fast storage in Phase 3 (0019); Lust-style area files (0020); the asset database and models as node trees (0021); undo as reversible edits (0022).

**Gate:** save an area, change it in a text editor, and see it reload. Rename a texture and nothing breaks. Two people change different entities in the same area on two branches, and Git merges them without a conflict.

## Phase 3: The editor

- **First:** the game's fast storage, chosen with a benchmark at the scale target (`hecs` or Solis's own), built from the area document when Play is pressed ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)). Chosen: `hecs` ([decision 0024](docs/decisions/0024-game-storage-is-hecs.md)). The working plan is [docs/plans/phase-3.md](docs/plans/phase-3.md).
- Picking and selection in the viewport, and move, rotate and scale gizmos.
- The entity list and the inspector.
- The library: assets and code, with a collapsible panel.
- The console: logs, errors and commands.
- Play mode, which resets the area when you stop. Several viewports.

**Gate:** build a small area from scratch in the editor, press Play, stop, and everything is back as it was.

## Phase 4: Lust and Lua join

Components in Rust work from Phase 2, since they're the engine's own kind. This phase adds the other two. It comes straight after the editor, because Lust components need the inspector and play mode, and nothing in Phases 5 to 7 has to wait for it ([decision 0016](docs/decisions/0016-how-lust-joins-solis.md)).

**Ready in Lust already:** `native` methods, `lust build --library`, vectors and rotations, components, events, and the compiler as a library with swaps (LUST.md §21, Lust 2.0.0-alpha.32), tested in Luau by Lust's `engine-test`.

- **Lust builds into Solis:** Solis writes the files that describe the engine to Lust (`solis/entity.lst`, `solis/world.lst` and so on) from its own Rust code, and checks a game's Lust with the compiler as a library; while editing it runs as Luau, and in built games as Rust, its `native` methods calling the engine.
- `object … like Component` becomes the same kind of component the engine's own are, with `has` fields in the inspector. Lust's mistakes show in the Problems panel with file and line, never a Rust error.
- **Engine types and events:** vectors and rotations as real value types in Lust, and typed events between components (`onHit(Entity by, int damage)`).
- **Fast changes** (DESIGN.md §3.1): the engine prebuilt, only the game's Lust built, and loaded into play mode without restarting.
- **Lua components** through Luau: the same inspector fields, events and entities, with the game API's Lua face.
- **A first game in Lust:** a small sample (a turret with ammo, cooldown and damage) that the later phases keep growing, and a first page of the Lust guide for Solis.

**Decided before starting:** while editing, Lust runs as Luau and is swapped into the running game, keeping values; built games run it as Rust ([decision 0028](docs/decisions/0028-lust-runs-as-luau-while-editing.md)). The working plan is [docs/plans/phase-4.md](docs/plans/phase-4.md).

**Gate:** change a Lust component while the game is running and see the change within **2 seconds**; a Lua component changes instantly. A mistake in Lust shows a friendly message, never a Rust error. One entity can carry a Rust, a Lust and a Lua component that talk to each other through events.

## Phase 5: Scale

The phase that proves DESIGN.md §2 early, while changing course is still cheap.

- GPU instancing, frustum culling and LODs.
- A job system, so the engine uses every CPU core.
- Drawing large numbers of small, fast things (projectiles) cheaply.
- A benchmark (Help > Make the benchmark…) at three levels ([decision 0030](docs/decisions/0030-scale-is-tested-at-three-levels.md)): **target**, 2,000 moving units and 20,000 projectiles; **stress**, 10,000 and 100,000; and a **ceiling** run that keeps adding until 60 fps ends.
- A frame profiler.

**Decided:** the job system is `rayon` ([decision 0031](docs/decisions/0031-the-job-system-is-rayon.md)); LODs are made on import or come from the model ([decision 0032](docs/decisions/0032-lods-made-on-import-or-the-model-s-own.md)). The working plan: [docs/plans/phase-5.md](docs/plans/phase-5.md).

**Gate:** on your PC, the target level runs at 60 fps and the stress level holds 30 fps without stutter; the ceiling run reports how far Solis goes; and the frame profiler shows where the time goes.

## Phase 6: Simulation and play

- A fixed-timestep simulation, separate from the frame rate.
- Determinism rules: seeded randomness, checksums of the game state, and Rapier physics in its determinism mode; Lust's checker and Lua warnings flag code that could break them.
- Input as named actions, with gamepads and rebinding.
- Save games.
- The first ready-made gameplay components, written in Lust as examples for game makers: a character controller and a camera controller.

**Gate:** walk a character around a level with physics, save, quit and load. The same recorded inputs always give exactly the same result.

## Phase 6 hardening

Stronger tests before building on the game loop ([decision 0042](docs/decisions/0042-a-hardening-milestone-before-phase-7.md)). Phase 6's reviews kept finding problems, some caused by earlier fixes, so this milestone makes the tests catch them first. The working plan: [docs/plans/phase-6-hardening.md](docs/plans/phase-6-hardening.md).

- Merge the open pull requests once tested (last: the project lead asked for the tests first).
- Whole-game invariant tests: random games with saving, loading, replays and edits, checked against rules that must always hold.
- Fuzzing every reader of a file a project can ship.
- Measuring the tests: coverage, and mutation testing on the core crates.
- Checks that stay: in CI, weekly, and in each phase's review.

**Gate:** the project lead reads the hardening report (coverage, mutations caught, hours fuzzed and what was found) and tries the playground once more.

## Phase 7: Sound

The working plan is [docs/plans/phase-7-sound.md](docs/plans/phase-7-sound.md) ([decisions 0056](docs/decisions/0056-sound-with-kira.md) and [0058](docs/decisions/0058-listeners-and-who-hears-what.md)). Animation left this phase ([decision 0059](docs/decisions/0059-2d-first.md)): frame animation and the state machine come with 2D (Phase 9), and 3D skeletal animation has its own phase once 2D is complete (3D 1).

- **Sound** (Kira): sound events with random takes, pitch and volume; buses and ducking; sounds placed in the world; reverb zones; listeners, and who hears what.

**Gate:** footsteps each sound a little different, the music dips while someone speaks, and walking into a reverb zone makes the steps echo.

## Phase 8: Compatibility

The project lead's game moves to Solis next, so before it does, updates must never lose work or break a game ([decision 0053](docs/decisions/0053-compatibility-before-2d.md)). The working plan is [docs/plans/compatibility.md](docs/plans/compatibility.md). Safety here is automatic: nothing to remember to use.

- **A backup before every upgrade:** before Solis changes any file in a project, it saves a dated copy and checks the upgraded file reads back correctly; if any step fails, nothing changes.
- **Old files keep opening, forever:** every format (areas, assemblies, the project, settings, input, recordings, and later tile maps) has an upgrader from every older version, and an example file of each old version is kept as a test.
- **Save games survive updates,** so a game's update never wipes its players' progress.
- **The game-code interface is guarded:** a test fails if what game code can call changes by accident; a deliberate rename keeps the old name working, warns in Problems and offers to fix the code.
- **Real projects are tested:** a project (the samples, and the project lead's game if wanted, from a private repository or on their own PC) is opened and played from a recording before an update ships; if it breaks, the update is stopped.
- **Breaking changes are labelled** in the changelog, and must bring their upgrader.

**Gate:** open a project saved by an older build, see the backup and the upgrade; a save game from before an update loads after it.

## Usable first (decision 0070)

The engine audit of 3 November ([decision 0070](docs/decisions/0070-usable-first.md)) put a new order ahead of the rest: everything needed to make a Stardew Valley-like farming game in Lust, with the editor's everyday basics first. It takes over Phase 9's last three milestones; Phase 10 and everything after follow it. One milestone per pull request, as always.

1. **Editor basics (E1):** blue, nothing shown that isn't built, New in the Library (right-click anywhere, or **+ New**), a start page. *Built.*
2. **One project file, the rest as Lust:** what the editor makes is written as Lust data files. *Begun: the base, tile maps and tilesets are Lust ([decision 0081](docs/decisions/0081-what-solis-makes-is-lust.md)); the other files follow, one at a time ([plan](docs/plans/lust-data.md)).*
3. **The API milestone:** game code changes tiles and their values, drives animators and plays clips (four Lust changes, approved 3 November). *Built: clips by name ([decision 0077](docs/decisions/0077-clips-and-cues-as-names.md)), animators and tile maps reached through their own files ([decisions 0078](docs/decisions/0078-animators-reached-through-their-own-files.md) and [0079](docs/decisions/0079-tile-maps-reached-through-their-own-files.md)). It was built without the triggers step 5 left for it.*
4. **Game-wide objects:** Lust's `object` (a clock, money, the inventory), ticked by Solis and kept in saves. *Built: they tick first and change only in their own update ([decision 0080](docs/decisions/0080-objects-change-only-in-their-own-update.md)); telling one something from a component comes later.*
5. **2D physics and top-down movement:** tile collision, 8 directions for the player, 4 for NPCs. *Built ahead of 2 to 4, which were waiting on Lust's changes at the time ([decision 0071](docs/decisions/0071-2d-physics-and-top-down-movement.md)). **Triggers are not built:** walking into a zone (a door, a shop counter) doesn't tell game code yet. Decision 0071 left them for the API milestone, which was built without them; they are now part of [2D 2](#2d-2-movement-light-and-polish)'s "More movement" milestone.*
6. **The 2D camera** that follows the player. *Built: the kit's FollowCamera2D, and cells at places ([decision 0082](docs/decisions/0082-the-2d-follow-camera-and-cells-at-places.md)).*
7. **Game UI and the UI editor:** menus, HUD, inventory, dialogue, a pause menu with key rebinding (Phase 12, brought forward). *Built: shapes laid out on any screen ([decision 0083](docs/decisions/0083-game-ui-on-any-screen.md)), text ([decision 0084](docs/decisions/0084-game-ui-text.md)), menus that send events ([decision 0088](docs/decisions/0088-menus-send-events.md)), Atlas Crop and the UI editor ([decision 0089](docs/decisions/0089-atlas-crop-and-the-ui-editor.md)), and the player's settings ([decision 0091](docs/decisions/0091-player-settings-and-the-window.md)). **Before the UI editor, the editor overhaul** ([decision 0085](docs/decisions/0085-the-editor-overhaul.md), [plan](docs/plans/editor-overhaul.md)): built: the frame, Properties, the Library drawer, one search, playing, workspaces, help in place; the UI editor lives in its UI workspace. The project lead's order (2026-11-05): the overhaul's parts, then all of game UI, before anything else. Screens are Lust's own views read through the engine ([decision 0087](docs/decisions/0087-screens-through-the-engine.md)). **Before game UI, the launcher** ([decision 0086](docs/decisions/0086-the-launcher.md), [plan](docs/plans/launcher.md)): built: projects, Learn and Updates on a page of their own.*
8. **Day and night, and lamps** on sprites. *Built ([decision 0092](docs/decisions/0092-day-and-night-and-lamps-on-sprites.md)).*
9. **The playtest export** (Phase 11, brought forward). *Built ([decision 0093](docs/decisions/0093-the-playtest-export.md)).*

**Gate:** the project lead starts their farming game in Solis, in Lust: walk the farm, till, plant and water, crops grow over days, a menu and an inventory, saving, and a build a friend can play.

## Phase 9: 2D, part 1: a 2D game can move in

First-class 2D ([decision 0054](docs/decisions/0054-first-class-2d.md)), built as flat things in the same world, so lamps, effects, materials and the rest work on 2D too, with real 2D tools on top. The working plan, with the research behind it and the bar every milestone must pass, is [docs/plans/2d.md](docs/plans/2d.md).

- **The pixel-perfect view and the 2D editor:** drawn at the game's own resolution and scaled up crisply, smooth scrolling without shimmer, sharp text and menus.
- **Sprites from Aseprite:** tags, slices, events, layers and palettes, reloaded on save; atlases packed automatically; 100,000 sprites drawn cheaply.
- **Frame animation,** with the animation state machine and the animator graph editor built here, for sprite frames first, and made so 3D skeletons use the same ones later (3D 1). Frame events can play sound events (footsteps with no code).
- **Tile maps that change as you play:** cells store meaning and their own data (a crop's growth), changes batched per chunk, saved as text that merges in Git.
- **Autotiling:** each tileset chooses its kind (16-tile, 47-tile, edge sets or custom rules), set up with a wizard.
- **The tile painter.**
- **2D physics** (Rapier2D) with tile collision merged into smooth shapes, triggers, and a ready-made top-down controller in Lust. *Moved to "Usable first", and built there without the triggers, which are now part of 2D 2.*
- **The 2D camera and feel tools.** *Moved to "Usable first".*
- **Lamps and day and night on sprites,** crisp on the pixel grid. *Moved to "Usable first".*

**Gate:** the project lead rebuilds a slice of their own game in Lust: walk the farm, till, plant and water, crops grow over days, sleep, lamps at night; smooth, crisp, no jitter.

## Phase 11: Playtest export

- **File > Export for playtesting:** a folder to zip and send, with a game program without the editor and the project's files packed beside it.
- Lust and Lua run as in Play, so there's no long compile; windowed, fullscreen and resolution settings; saves in the player's saved-games folder; a crash log playtesters can send back.
- What waits for the full export (Shared 11): building a game's own Rust code, and an installer. Store features such as Steam's are left to each game's developer ([decision 0098](docs/decisions/0098-ideas-become-milestones.md)).

**Gate:** a friend plays an exported build on their PC without Solis installed.

## Phase 12: Game UI

- **Sungraph UI** ([decision 0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md)), whole: a visual UI editor where game UI is drawn canvas style (lines, shapes, curves, text) instead of built from images; values tied live to game code; every element reachable from code, because a UI file is Lust (`view { }`); styles such as glow, pixelation and stylized outlines over the whole UI. HUDs and menus are made with it. Plan: [docs/plans/game-ui.md](docs/plans/game-ui.md).

**Gate:** a health bar and a pause menu on screen, drawn and tied to the game in the editor.

## How the rest is built: three tracks

The project lead's order of 10 November 2026 ([decision 0097](docs/decisions/0097-three-tracks.md)): 2D is finished first, so whole 2D games can be made; 3D comes after; and what isn't purely 2D or 3D is built **once, for both**, so no system is made twice. Big phases are split into smaller ones. "Was" says where each came from. The ideas list and the editor's UI remade joined as milestones the same day ([decision 0098](docs/decisions/0098-ideas-become-milestones.md)).

**The order:**

1. **Shared 1a to 1d:** the editor's UI, remade to the project lead's design: the look and the frame, the everyday screens, 2D and its editors, then the rest and the finish. *Built.* Then **Shared 1e** (layouts and resizing, *built*), the 27 problems of the full review's second pass, and **Tools 1** (shape brushes and world sculpting), moved ahead on 13 November 2026 ([decision 0108](docs/decisions/0108-shape-brushes-and-sculpting-come-next.md)).
2. Finish "Usable first" (one project file, the rest as Lust) and its gate.
3. **Shared 2:** game code: finding and making things (the rest of the playtest fixes).
4. **2D 1 and 2D 2:** 2D complete.
5. **Shared 3 to 6:** the tools from the ideas list: time and feel, data and the editor, screens and sound, worlds and testing.
6. **Shared 7 to 10:** materials and shaders, effects, weather and the finished picture, cutscenes and behaviour, for sprites and models alike.
7. **A pause to rethink,** and the project lead's big testing pass: 2D and effects are complete.
8. **Shared 11:** scripting, the full export, Git and mods, so finished games can ship.
9. **3D 1 to 5:** 3D animation and navigation, then light, sky and atmosphere in four steps.
10. **Shared 12 and 13:** netcode, and Solis 0.1.

Steps 8 to 10 are written down so nothing is lost, but they aren't planned in detail or reordered until the rethink ([decision 0059](docs/decisions/0059-2d-first.md)): 2D and effects will turn up things nobody can foresee yet.

**Where 2D and 3D stand:** [docs/plans/roadmap-2d-3d.md](docs/plans/roadmap-2d-3d.md). **Ideas:** every idea in [docs/plans/ideas.md](docs/plans/ideas.md) is now a milestone below, except two left out on purpose ([decision 0098](docs/decisions/0098-ideas-become-milestones.md)).

---

## Shared 1: The editor's UI, remade

Added 10 November 2026: the editor's interface rebuilt from scratch, with the project lead's own design. The design arrived on 11 November: the "G layout", a tab bar, a ribbon, editor groups, a shelf and a status bar, in one dark theme ([`docs/design/g-layout/`](docs/design/g-layout/README.md), [decision 0099](docs/decisions/0099-the-g-layout.md)). It is sixteen milestones, so it is built as four small phases, each with its own gate; the last carries the gate the phase was given at first. After every milestone the editor works and everything that existed before can still be reached. Where the engine lacks something the design shows, a placeholder stands in its place, and one setting hides them all ([decision 0101](docs/decisions/0101-unfinished-parts-as-placeholders.md)). Parts the design doesn't draw (the 2D editors, above all) are mocked up in its language and approved before they are built. Plan: [docs/plans/shared-1-ui-remake.md](docs/plans/shared-1-ui-remake.md).

## Shared 1a: The look and the frame

- **The look:** one theme, five accents, the design's sizes and type ([decision 0100](docs/decisions/0100-one-theme-five-accents.md)). *Built.*
- **The kit:** icons, the design's controls, and unfinished parts marked, with a setting that hides them ([decision 0101](docs/decisions/0101-unfinished-parts-as-placeholders.md)). *Built.*
- **The top:** tab bar, ribbon and command search in place of the menus ([decision 0103](docs/decisions/0103-todays-keys-stay.md)). *Built.*
- **The middle:** editor groups, the shelf and the status bar in place of the dock ([decision 0104](docs/decisions/0104-docking-is-solis-s-own.md)). *Built.*

**Gate:** the editor opens in the new frame, everything that worked before can be found, and "Hide unfinished parts" leaves no gaps.

## Shared 1b: The everyday screens

- **Home: the level editor.** *Built.*
- **Home: placing.** *Built.*
- **The Library.** *Built.*
- **File: projects, Learn and updates** ([decision 0105](docs/decisions/0105-solis-starts-on-the-file-page.md)). *Built.*
- **File: Preferences, Project settings and Input.** *Built.*

**Gate:** build a small area, place things from the shelf, change a setting and a key, and open another project, all on screens that match their pictures.

## Shared 1c: 2D and its editors

- **Tile maps, tilesets and pictures' crops.** *Built.*
- **Animate: sprite clips and animators.** *Built.*
- **Game screens.** *Built.*

**Gate:** paint a map, edit a tileset, change a clip and an animator, and lay out a screen, each as its approved mock-up.

## Shared 1d: The rest, finished

- **Audio.** *Built.*
- **Debug.** *Built.*
- **The unbuilt screens, the Build tab and dragging tabs.** *Built.*
- **Finish:** the manual, the old code out, the review. *Built, all but the review and the scale check, which the project lead paused (answer 34): both are owed before the gate.*

**Gate:** the project lead works in the new editor for an evening and wouldn't go back.

## Shared 1e: Layouts and resizing

Added 13 November 2026 by the project lead, after trying the remade interface: the Area and Assets lists out of the compact bottom shelf, into a more usual layout, and every edge between panels draggable ([decision 0109](docs/decisions/0109-layouts-as-a-setting.md)).

- **Layouts as a setting:** three layouts (Area left and Assets along the bottom; Area and Assets stacked on the left; Area left and Assets right) and today's shelf, chosen in Preferences, so the project lead can try them before picking one. *Built.*
- **Every edge resizable:** every border between panels and columns can be dragged, each tab remembers its sizes, and a double-click puts a border back. *Built.*

**Gate:** each layout can be chosen and works on every tab, every border drags and is remembered after a restart, and the editor's frame stays inside the scale check's targets.

## The second pass's 27 problems

The problems found by the full review's second pass ([the report](docs/reviews/2026-11-full-review-second-pass.md)), fixed in pull requests grouped by kind: stability and correctness, speed (with scale cases), and the small mismatches.

## Tools 1a: Shape brushes

Moved ahead of the rest of 2D on 13 November 2026 ([decision 0108](docs/decisions/0108-shape-brushes-and-sculpting-come-next.md)); what the tools are: [decision 0111](docs/decisions/0111-tools-1-brushes-and-ground.md); plan: [docs/plans/tools-1.md](docs/plans/tools-1.md). Mocked up and approved before it's built; every tool has a scale case.

- **B1 The shape core** (built): an editable polygon mesh (Aventail's, decision 0007, the `mesh` crate), the list's shapes, a Brush component saved in the area (area files' version 3), drawn and solid.
- **B2 The list and drawing out** (built): **B** opens the list over the view (or Home › Create › Shape), each **B** moves down, **Space** confirms, **Esc** closes; drag the footprint on the grid, pull up the height, click; one undo step a brush.
- **B3 Editing the insides:** **Tab**, then **1** points, **2** edges, **3** faces, moved with the gizmos.
- **B4 Extrude and shaping:** **E** then move and click, or Shift + drag on the gizmo; inset, split, delete.
- **B5 Materials, speed, the manual.**

**Gate:** a small level blocked out with brushes, edited and extruded, played, saved and reopened.

## Tools 1b: World sculpting

- **G0 The study** (rule 22): competing ways to store and draw a ground with caves, measured.
- **G1 The ground, Smooth:** one per area, in chunks made on workers, solid, saved.
- **G2 Sculpting:** left-drag adds, Ctrl digs, Shift smooths, [ and ] size, Shift + [ and ] strength; Flatten.
- **G3 Blocky and Sharp:** the ground's look as a setting per ground.
- **G4 Painting:** materials painted with the same brush.
- **G5 Speed, game code, the manual.**

**Gate:** a level of brushes on a sculpted, painted ground with a cave, in each look, played, saved and reopened, at the scale check's targets.

---

## Shared 2: Game code: finding and making things

The fixes the playtest games asked for (steps 2 to 5), and idea 34.

- **Asking a tile map questions:** what's at a cell, the cells of a terrain, world positions to cells and back.
- **Tags and questions about the world:** label entities ("crop", "enemy"), spelling checked before the game runs, and ask for all within 3 metres, or the nearest. (Ideas 8 and 9.)
- **Spawning with values:** a spawned entity's fields set as it's made.
- **Game tests:** small automatic checks ("a crop grows in three days"), run the way Solis tests itself. (Idea 29.)
- **A note for code files nothing fetches,** in Problems, with a quick fix. (Idea 34.)

**Gate:** a game finds the nearest enemy by tag, spawns a bullet already moving, and has a test that passes.

## 2D 1: The world and tiles

Was Phase 10, part one.

- **Levels and the world:** entities with typed fields, rooms on a world map, sort points for tall things.
- **Advanced tiling:** rules for decoration and variety, scatter brushes, patterns and stamps.
- **Pathfinding** on tile maps: A* and flow fields, updated as the map changes.

**Gate:** a level of rooms on a world map, decorated with the new tiling tools, with a character that finds its way around walls as the map changes.

## 2D 2: Movement, light and polish

Was Phase 10, part two, and 2D's light from Phase 13.

- **More movement:** grid-step and platformer controllers (coyote time, jump buffering, wall-jump), one-way platforms, slopes and moving platforms. And **triggers:** walking into or out of a zone (a door, a shop counter) tells game code; left over from Phase 9's 2D physics ([decision 0071](docs/decisions/0071-2d-physics-and-top-down-movement.md)).
- **Soft shadows** from walls and objects, and optional normal and glow layers.
- **Light for 2D worlds:** 2D bounced light and the sky over 2D.
- **Polish:** parallax (built early, [decision 0094](docs/decisions/0094-parallax-layers.md)), palette swaps and cycling, clean pixel-art rotation, hit-stop and squash, the 2D sample and the manual's 2D section.

**Gate (2D complete):** the 2D sample and the project lead's extended game slice; every speed budget met; the old tools' known problems, each a passing test.

## Shared 3: Time and feel

Ideas 1 to 7 and 16: tools almost every game reaches for.

- **Timers:** "do this in 2 seconds", "every day"; they pause with the game and are kept in saves.
- **Tween:** a value changes smoothly over time, with a choice of easing.
- **Steps in order:** "walk there, wait a second, say this", written top to bottom, for little cutscenes.
- **States that know when they start and stop,** and stop what was started in them.
- **Screen shake, flash and fade,** and controller rumble.
- **Smooth randomness and weighted random:** noise that drifts gently, rare drops, and shuffled decks so nothing repeats too soon.

**Gate:** a hit shakes the screen and flashes the sprite, a menu slides in, a door opens two seconds after a switch, and a chest drops a rare item now and then.

## Shared 4: Data and the editor

Ideas 15, 18 to 21 and 12.

- **Data tables:** lists such as "every crop: days to grow, price, season", kept as a file and edited as a table.
- **Save slots:** several saves with a picture, the date and time played, autosave; the player's own settings kept apart.
- **Field hints:** a component's fields get sliders, limits and a sentence saying what they do.
- **More kinds of settings:** a field can be a colour, a sound, a sprite, an assembly or a list.
- **Components that draw in the editor,** such as a guard's view cone while you place it.
- **Paths drawn in the editor,** for a guard to walk, a platform to follow or a camera to glide along.

**Gate:** a shop reads its prices from a table; a guard's patrol path and view cone are drawn and tuned in the editor; the game saves to three slots.

## Shared 5: Screens, words and sound

Ideas 23 to 27 and 17.

- **Text effects:** text that types itself out, wavy or shaking words, small icons inside text.
- **Button pictures that match the player's controller,** switching between keyboard and gamepad.
- **Tooltips, and drag and drop** for inventories and shops.
- **Translations and subtitles:** the game's text in several languages, with a check for missing lines, and captions for sounds and speech.
- **Music that follows the game:** layers fade in when danger is near, and changes land on the beat.

**Gate:** a shop where items are dragged into a bag, with tooltips, in two languages; dialogue types itself out; the music swells when an enemy is near.

## Shared 6: Worlds and testing

Ideas 10, 11, 28 and 30.

- **Rooms and moving between them:** into a house or a cellar, with a fade or a loading screen; what belonged to the room goes when you leave.
- **Pools:** bullets, sparks and dropped items reused, so huge numbers stay smooth.
- **Seeing inside the game while it runs:** debug drawing, a cheat box with your own commands, and live values.
- **Bug reports from playtesters** carrying a recording of everything they pressed, to replay exactly what went wrong.

**Gate:** walk into a house and back without a hitch; type `give seeds 10` in the cheat box; replay a playtester's bug report.

## Shared 7: Materials and shaders

Was Phase 14. For sprites and models alike: how a surface takes light, glows, dissolves, gets wet or sways in the wind ([decision 0048](docs/decisions/0048-materials-and-shaders-before-effects.md)). The working plan is [docs/plans/materials.md](docs/plans/materials.md).

- **Materials as assets** (`.solmat`), each with its own look (standard, stylized, light ramp, unlit), edited in Properties with a live preview.
- **Inputs that drive any material value:** the light around the surface, time, distance, height, the weather's wetness, noise, and values set by game code.
- **A node graph for materials and shaders,** saved as readable text, sharing its graph editor with the animator and the effects.
- **Code for experts:** a node holding a few lines of shader code, checked before it runs.
- **Building blocks for stylized looks:** light ramps, rim light, hatching, dithering, outlines, wind sway, triplanar mapping; for sprites, dissolve, flash, outline and glow.
- **Per-copy values at scale:** thousands of copies of one material, each with its own colour or glow, still drawn together.

**Gate:** without writing code, make a sprite that flashes white when hit and dissolves when it dies, a plant that glows as night falls, a toon character with its own outline, and a tree that sways; the benchmark's target still runs at 60 fps.

## Shared 8: Effects

Was Phase 15.

- **Solstice, the effects suite:** the file, the particles (on the CPU and the GPU) and game code ([plan](docs/plans/effects.md)); particles are drawn with Shared 7's materials.
- **The effect preview tab and its graph.**
- **Flipbooks, eroded and mesh particles:** the stylized look.
- **Trails and ribbons.**
- **The effects kit and the battle.**
- **A pixel mode for pixel-art games:** pixel-shaped particles, movement snapped to the pixel grid, colours from the game's palette, and a ready-made set (wind streaks, fireflies, bubbles, rain, snow, dust, falling leaves, sparkles, smoke puffs); fireflies and sparks can give off real light.

**Gate:** laser bolts, hits and explosions look good in the stylized look, in the benchmark battle, at 60 fps; in a pixel-art game, fireflies light a field at night and wind streaks cross it, all crisp on the pixel grid.

## Shared 9: Weather and the finished picture

From Phase 13: both are for 2D and 3D alike.

- **The finished picture, part two:** bloom, white balance, colour grading.
- **Weather and moods:** weather files keyed by time of day, blended, with one overcast control, bringing their effects (rain, snow) from Shared 8.

**Gate:** a 2D scene and a 3D scene each go from a clear morning to a rainy evening, blended smoothly, with bloom on the lamps.

## Shared 10: Cutscenes and behaviour

Ideas 22 and 14, after effects, so cutscenes and characters can use everything before them.

- **A cutscene timeline:** camera moves, sounds, animations and effects arranged along a timeline.
- **Behaviour for characters:** a villager's day or an animal's habits, drawn as a diagram like the Animator.

**Gate:** a short cutscene plays from the timeline, and villagers follow their day on their own.

## The rethink

2D and effects are complete. The project lead's big testing pass happens here, and everything below is planned in detail, or reordered, only now.

## Shared 11: Scripting, full export, Git and mods

Was Phase 18. Before 3D, so finished 2D games can ship.

- **The scripting window:** LustScript and Lua through Luau, calling the same commands as the built-in tools, with undo, a sandbox and time limits.
- **The full export:** finished games for Windows ([decision 0012](docs/decisions/0012-windows-only.md)), with the game's own Rust built in and Lust compiled for speed, an installer, and Build-and-Play from the top bar. It grows from the playtest export.
- **The Git panel** ([decision 0006](docs/decisions/0006-git-built-in.md)): changes with plain-language area summaries, commit, pull and push, branches, history, Git LFS locking, and new projects from or published to a repository.
- **The area merge tool:** merges areas and assemblies entity by entity; when two people changed the same field, asks which to keep.
- **Mods:** players change and add to a game with LustScript (`.lsc`), which is what LustScript was made for, with the same sandbox as the scripting window. (Was idea 31.)

**Gate:** write a small editor tool that places objects in a ring; export a game and run it on a PC without Solis installed; two people work on the same project through GitHub for an evening and resolve one real area conflict in the editor.

## 3D 1: 3D animation and navigation

Was Phase 16 ([decision 0059](docs/decisions/0059-2d-first.md)). The working plan is [docs/plans/phase-7-animation.md](docs/plans/phase-7-animation.md) (decision 0057).

- **Skins and clips from glTF,** read safely.
- **Skinned drawing,** playing one clip.
- **Crowds:** baked clips and animation levels of detail.
- **Blending and blend spaces** for skeletons, on the state machine 2D brought.
- **Cues on 3D clips:** footsteps on the right frames, in the playground.
- **Navigation in 3D:** navmeshes built from the world, and characters walking around each other. (Was idea 13's 3D half.)

**Gate:** a character walks and runs with smooth blending, its footsteps play on the right frames, and it finds its way across a 3D level.

## 3D 2: Real light and the sky

Was Phase 13, part one. Solis's selling point: skies, sunsets and light that feel right, never flat ([plan](docs/plans/light.md), [decision 0046](docs/decisions/0046-light-before-netcode.md)).

- **Measure first:** a light showcase project and quality levels (Low, Default, High).
- **Real light and automatic exposure,** as eyes adjust.
- **A physical sky** (Hillaire's method): blue by day, orange at sunset, blue at twilight, haze that tints distance; the painted sky kept as a style.
- **Light from the sky:** ambient light and reflections from the real sky, instead of one flat colour.

**Decide before starting:** the light plan's open questions (light units, time of day as game state, quality levels, the night sky, the sky's styles).

**Gate:** in the light showcase, noon to sunset with no jumps, and the golden hour feels right.

## 3D 3: Night, clouds and shadows

Was Phase 13, part two.

- **Time of day and night:** sun, moons and stars where they really are, the aurora; colour curves and a tint over the sky for stylized games.
- **Clouds:** a flat layer, and volumetric clouds at High.
- **Ambient occlusion.**
- **Shadows near and far:** cascades, contact and soft shadows.

**Gate:** the showcase runs from noon to night under clouds, with shadows reaching to the horizon.

## 3D 4: Lamps, fog and reflections

Was Phase 13, part three.

- **Many lamps, glowing surfaces and area lights** (and glow that wakes in the dark, for plants and creatures).
- **The light ramp look,** for blocky games: simple, readable light where dark means dark.
- **Volumetric fog and light shafts,** with moving, coloured fog volumes and caustics.
- **Reflection probes.**
- **Reflections on the screen and wet surfaces** (for neon streets in the rain).

**Gate:** a rainy street at night with hundreds of lamps, fog and wet reflections; the stylized look is as good.

## 3D 5: Bounced light

Was Phase 13, part four.

- **Baked light probes:** bounced light (decision 0008's second source of indirect light).
- **Real-time bounced light,** traced against a simplified world (no ray-tracing hardware needed).
- **Bounce from the screen,** joined with the probes.
- **An experiment:** a Solis method for bounced light, kept only if it wins.

**Gate:** indoors never looks flat; the benchmark's target still runs at 60 fps.

## Shared 12: Netcode

Was Phase 17. What it needs (fixed ticks, determinism, checksums, save games) is built, and a CI check keeps two PCs playing the same game until then. Netcode in layers ([decision 0005](docs/decisions/0005-netcode-in-layers.md)):

- **Connections and sessions:** reliable and unreliable messages, encryption, lobbies, and relays for players behind home routers.
- **Network tools:** a lag and packet-loss simulator, network stats, desync detection, and replays.
- **Lockstep,** as an optional module, for strategy games.
- **Other netcode models plug in,** with a small example each.
- **Dedicated servers.**

**Decide before starting:** how relays are run.

**Gate:** two PCs on different networks play the benchmark area together without desyncing, with simulated lag.

## Shared 13: Solis 0.1

Was Phase 19.

- **The flagship sample, written in Lust:** a small space battle, over the network.
- **Docs:** getting started, a first-game tutorial, and a reference for the engine as Lust sees it.
- **New project templates.**
- **The 0.1 release.**

**Gate:** someone new to Solis follows the tutorial and makes something that runs.

---

## Not in 0.1

Open-world streaming, terrain's texture painting and scattering, the full Solstice toolkit, rollback netcode, advanced animation (IK, root motion), Linux and macOS (decision 0012).

**Not planned at all** ([decision 0098](docs/decisions/0098-ideas-become-milestones.md)): photo mode and screenshots, and store features built in (Steam's achievements, cloud saves, Workshop): those are for each game's developer, not the engine.

## Who does what

- **Claude:** the engine in Rust, and the Lust side of Phase 4 in the Lust repo.
- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 4: the gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if you want to try some.
