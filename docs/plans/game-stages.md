# The game stages: Solis's order, by what it lets you make

The order of work from 18 November 2026 ([decision 0122](../decisions/0122-the-game-stages.md)). The project lead wants to make every kind of 2D game in Solis, learning as they go, with 3D later because 2D features are easier to build first. So the 2D work is ordered by **the kind of game it lets you make**, one stage at a time.

**What doesn't change:**
- **Every feature is a general tool any game can use** (CLAUDE.md, "The project lead's games are made with Solis, never built into it"). A stage only decides *when* a feature is built. The features are the ones ROADMAP.md's phase sections already describe; each line below names the section it comes from.
- **How each is built** (technique, code, tests) is in [the plan after the rethink](after-the-rethink.md) and its research.
- One milestone per pull request, UI mocked up first, keys asked about first (rule 17), the scale target kept.

**What's new:**
- **Only the next milestone is planned in detail.** The stages below are the outline, revisited at every stage's end.
- **Each stage ends with a sample game of its kind,** made with Solis's own tools and offered in Learn, as the playground and the turret sample are. The project lead tries it as much as they like. They don't have to make a game to finish a stage.

## Stage 1: a top-down adventure

Walking a world, rooms and doors, talking to people, picking things up, an inventory, saving. Solis is closest to this already: top-down walking, tile maps, day and night, game UI.

- **Triggers:** walking into or out of a zone (a door, a counter, a pit) tells game code. *Built* ([below](#the-next-milestone-triggers), with its As built note). (2D 2)
- **The same game on two PCs:** a recorded game replayed on two machines, checksums compared tick by tick, before timers put more maths into game state (decision 0120). (Shared 2) *Built.*
  - **As built** (branch `stage1/two-pcs`): `tests/two-pcs/` holds a small game (`game/`: 3D character, crates, a moving platform, drones spawned at random and despawned, a turret's projectiles and hits, a box zone and a round one that fires once; a 2D walker with a sprite and an animator in a tile map whose walls game code raises and lowers, a freeform zone; the game-wide `Director` changing the game's speed; Lust, Lua and Rust, the Rust being Solis's own Armour), ten seconds of recorded gamepad play (`game.solrec`, 48 KB) and the state at the end of every tick (`game.states`, 260 KB): both checksums, and the state part by part (`world::state_parts` with game code's values, `Workbench::game_state`), kept as what changed since the tick before, each part numbered once, a value over 32 characters kept as a checksum of it (`editor::StateList`). `crates/app/src/two_pcs.rs` plays it on one core and on four in `cargo test --workspace`, so on CI's Windows machine in every pull request; the first tick that differs fails with the parts named, both states written to `target/two-pcs/`, which the Windows job uploads, and a sentence on what to do. `SOLIS_UPDATE_TWO_PCS=1` writes the list again; `record_the_game` (ignored) records the game again. About 1.1 s on one core and 1.7 s on four here (the two run side by side; a debug build).
  - **Kept clear of decision 0106's gaps:** no tween has a done block, and no Lust `Random`.
  - **Found:** `--check-project` read prefabs as levels, so since they became `.prefab.lst` files a check knew none and a recording's spawns made nothing; fixed. Linux and four cores agree with Linux on one core. **Windows' first run** (pull request 284) differed at tick 9 in the `Director` object's `Maths:sin`, `exp` and `atan2`: Luau's `math` called the C library, which rounds differently on Windows. Game code's maths now comes from `libm`, pure Rust, the same bits everywhere (`script::exact_maths`, with each function's bits pinned in a test); Lust's runtime raises to powers with it too. A call costs 13 to 60 ns more (release build), so the benchmark's 20,000 sines and cosines a tick at the stress level add about 0.6 ms of game code's time, shared between cores. Lua's own `^` still calls the C library: the manual says to use `math.pow`.
- **Tags and questions about the world:** "all of a kind within 3 m", "the nearest". (Shared 2)
- **Timers and steps in order:** "open in 2 seconds", "walk there, wait, say this". (Shared 3)
- **States that know when they start and stop.** (Shared 3)
- **Field hints and more kinds of settings:** sliders, limits, help, and fields that pick a sound, a sprite, a prefab or a list. (Shared 4, with 2D 1's typed fields)
- **Data tables:** items, prices, descriptions in one file, edited as a table. (Shared 4)
- **Text layout, then text effects:** dialogue that types itself out, coloured and wavy words, icons in text. (Shared 5)
- **Tooltips, and drag and drop,** for an inventory and a shop. (Shared 5)
- **Fades:** to black and back, or another colour, as a general tool. (Shared 3, brought from stage 2's shake and flash.)
- **Rooms on a world map, and moving between them,** with a fade. (2D 1 and Shared 6)
- **A Door in the kit,** built on triggers, fades and rooms (the project lead, 2026-11-18): **Send to** a place in this level or an entry point in another room or level, **Two-way** (two doors linked, each sending to the other), and a **Transition** (none, or a fade, with its time). Written in Lust and open to read and change, like the kit's top-down walker. The trigger zone itself stays lean.
- **Sorting tall things:** walking behind and in front of trees and houses properly. (2D 1)
- **Pathfinding:** characters walk round walls to where they're going. (2D 1)
- **Save slots:** several saves with a picture and the time played. (Shared 4)
- **The sample:** a small top-down adventure in Learn.

## Stage 2: a platformer

Running and jumping, platforms that move, enemies that hurt, a camera that feels good.

- **Platformer movement:** jumps set by height and time, coyote time, jump buffering, wall jumps; one-way platforms, slopes and moving platforms. (2D 2)
- **Tweens made solid:** saved, and the same on any number of cores. (Shared 3)
- **Paths drawn in the editor,** for platforms and enemies to follow. (Shared 4)
- **Components that draw in the editor,** such as an enemy's reach while you place it. (Shared 4)
- **Screen shake, flash, rumble, hit-stop, squash.** (Shared 3; fades came in stage 1)
- **The sample:** a small platformer in Learn.

## Stage 3: a puzzle or turn-based game

Moving one tile at a time, levels to solve, turns, chance.

- **Grid-step movement:** one tile at a time, with buffered turns. (2D 2)
- **Asking a tile map questions:** "which cells hold this", "how many". (Shared 2)
- **Game tests:** small automatic checks ("this level can be solved"), run from the editor. (Shared 2)
- **Smooth randomness and weighted random.** (Shared 3)
- **Advanced tiling:** decoration rules, scatter brushes, stamps. (2D 1)
- **The sample:** a small puzzle game in Learn.

## Stage 4: an action game or shooter

Many enemies and projectiles, hits that feel good, effects and light.

- **Spawning with values, and pools:** a bullet already moving, and thousands reused smoothly. (Shared 2 and Shared 6)
- **The renderer's restructure, then materials:** a sprite that flashes when hit and dissolves when it dies. (Shared 7)
- **Projectile hits as game state, then effects:** particles, trails, flipbooks, the pixel mode. (Shared 8)
- **Soft shadows, normal and glow layers, 2D bounced light, the sky over 2D.** (2D 2)
- **Music that follows the game.** (Shared 5)
- **The sample:** a small shooter in Learn.

## After the four stages: finishing and sharing games

- **Polish for 2D:** palette swaps and cycling, clean pixel-art rotation. (2D 2)
- **The finished picture and weather:** bloom, grading, weather and moods. (Shared 9)
- **Button pictures for each controller, translations and subtitles.** (Shared 5)
- **Seeing inside the game:** debug drawing and the cheat box; bug reports from playtesters. (Shared 6)
- **Cutscenes and behaviour.** (Shared 10)
- **The rethink:** the project lead's big testing pass.
- **Scripting, the full export, Git and mods.** (Shared 11)

## Later

- **Every file as Lust** ("Usable first", L2 to L6): animators, sounds, levels, sidecars, saves and recordings. It doesn't help make games now; the cost of waiting is only a few more file upgraders.
- **A note for code files nothing fetches.** (Shared 2)

## 3D, after 2D

Planned in detail when 2D is done. The outline stays as ROADMAP.md has it:
- 3D 1, animation and navigation, with feet on slopes;
- Tools 1b, the sculpted ground (G1 waits on its branch);
- Open worlds;
- foliage;
- 3D 2 to 5, light, sky and atmosphere;
- netcode;
- Solis 0.1.

## The next milestone: triggers

**What it gives:** a zone you place in a level (a door, a shop counter, a pit) tells game code when something walks in or out. Every kind of game above uses it. Left over from decision 0071.

**How** (from [report 04](../reports/after-the-rethink/04-2d-2.md), M1):
- **Solis's own overlap pass after physics:** each tick, each trigger asks which characters and bodies overlap its shape. The results are sorted by trigger and entity, and compared with last tick's, so the events come in the same order on any number of cores.
- **The same code for 2D and 3D** (decision 0097: built once): `Trigger2D` on the flat world, and `Trigger` on the 3D world, as box or circle (sphere in 3D).
- **Which things it notices:** characters and bodies by default, optionally only some kinds, once tags exist (stage 1's next steps).
- **Once or every time.**
- **What's inside is game state:** saved, and in the checksum, so loading a save doesn't fire "entered" again.

**Game code** (rule 25; answered by the project lead, 2026-11-18):
- The zone hears `Entered(entity who)` and `Exited(entity who)`.
- The one who walked in hears `EnteredZone(entity zone)` and `LeftZone(entity zone)`.
- All of this works in Lust, Rust and Lua, and a zone carries its own code like any entity (+ Component).
- Something that goes in and out within one tick, such as a teleport straight through, fires nothing.

**Editor** ([approved mock-up](../design/triggers/README.md)):
- **Trigger zone** on Home's ribbon (an unfinished part until now) makes one in the view, where you're looking.
- The zone is a teal outline: solid with handles when picked, dashed when not, with its name and size beside it. In 2D it's a box or a circle; in 3D a box or a sphere.
- Its card in Properties has:
  - Shape and Size;
  - Notices (characters and bodies);
  - Fires (every time, or once);
  - Only tags, greyed until tags exist;
  - Inside now, filled while playing.
- A card says what game code hears, with a **New code for this zone…** button that makes a code file with empty `Entered` and `Exited` handlers, attached to the zone.
- A zone can be made into a prefab like any entity.
- **Freeform, in 2D only** (the project lead, 2026-11-18): Shape in 2D is Box, Circle or Freeform; 3D keeps Box and Sphere.
  - **Drawing (the lead's choice):** choosing Freeform starts drawing in the view. Each click places a point, and clicking the first point or pressing `Enter` closes the shape. `Esc` cancels and keeps the previous shape.
  - **Editing:** `Tab` edits the points, as shape brushes do. Drag a point to move it, `Ctrl` + click on an edge adds a point, `Delete` removes the picked point (never below 3), and `Tab` or `Esc` finishes.
  - Shapes may bend inward; they're split into simple pieces for the overlap test. Outlines that cross themselves are refused with a reason. There's a limit of 256 points.

**Tests and scale:**
- in, staying, out;
- two in the same tick, in order;
- the teleport rule;
- saved inside and loaded with no second "entered";
- 1 and 4 cores the same;
- 2,000 characters and 500 triggers within the scale target.

**Not in this milestone:** doors, teleporting and fades. They're the Door in the kit, later in stage 1.

**As built** (branch `stage1/triggers`):
- **The component:** `TriggerZone` (`level::builtin`), a box, a round shape (shown as a circle in 2D and a sphere in 3D) or a freeform outline in 2D (saved as `Box`, `Round`, `Freeform`), Notices (characters and bodies, the only choice until tags) and Fires (every time, once). A new component needed no new level version; `tests/fixtures/area/v5/zones.sollevel` is its example, and the level fuzz target reads zones.
- **The overlap pass** (`world::triggers`) runs as each tick begins, on where physics' last step left everything, not straight after the step: its events and the inside set a save keeps can then never be apart. It asks a tree of boxes made from where the moving things are now (parry's BVH), not Rapier's own query pipeline, which a physics step alone brings up to date: right after an edit while playing, a save or a load it would know nothing, and every zone would see everyone leave and come back.
- **What's noticed:** "characters and bodies" means things that move: characters and dynamic or kinematic bodies in 3D, characters in 2D (2D has no other moving bodies). Fixed bodies, walls and brushes never count. A zone never counts itself.
- **Once** sends the first tick's `Entered` (and `EnteredZone`), then nothing more, not even that visitor's `Exited`; the zone still knows who's inside. **Despawned while inside:** the zone hears `Exited`. **A zone despawned:** nothing is said.
- **Game code:** the four events in `solis/zones.lst`, and `inside()` on a zone (`method entity array entity:inside() native;`), as the tick started.
- **Saves** are version 10, with a `zones` section; version 9's upgrader adds an empty one.
- **The editor:** Trigger zone on Home's Create (in More in 3D, where the unfinished part was); the outline, label and handles over every view (a click on the outline picks a zone; sizes snap as moves do); the TriggerZone card and What game code hears; New code for this zone (a file named after the zone at the top of the project, a `main.lst` made if there's none, and otherwise the line to add said in the Console); freeform drawing and editing with the keys asked about. The card's title is the component's name, `TriggerZone`, as every card's is; the header and the Level panel say "Trigger zone".
- **Scale:** the overlap pass for 2,000 characters among 500 zones (a third of them freeform) takes about 1.2 ms a tick in a release build (target 2 ms).
