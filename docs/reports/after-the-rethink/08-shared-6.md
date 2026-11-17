# 08 · Shared 6: worlds and testing

ROADMAP.md (ideas 10, 11, 28 and 30): **rooms and moving between them** (into a house or a cellar, with a fade or a loading screen; what belonged to the room goes when you leave); **pools** (bullets, sparks, dropped items reused); **seeing inside the game while it runs** (debug drawing, a cheat box with your own commands, live values); **bug reports from playtesters** carrying a recording. **Gate:** "walk into a house and back without a hitch; type `give seeds 10` in the cheat box; replay a playtester's bug report."

Four milestones.

## What exists

- **Levels** (areas, renamed by R2) as files; play builds a `world::World` (hecs) from one level; `World:spawn` / `despawn` land at the tick's end and change the level document while playing (decision 0035, Stop puts it back); **game-wide objects** survive across the game (decision 0080) and are saved; save games hold everything (0038).
- **Projectiles** are already a pool of non-entities (a million, `world::Projectiles`); effects (Shared 8) will be non-entities too. So "pools" is about entities (dropped items, enemies, bullets made as prefabs).
- **The Console** with commands in the editor; **recordings** of input (`input::RECORDING_FORMAT`) that replay a game exactly; checksums; the playtest export with `log.txt` and `crash log.txt` in `Saved Games/<Game>` (decision 0093).
- **Properties shows the game's values while playing** (play mode, "Properties shows the game's values while playing").
- 2D 1 (recommended): rooms as level files plus a **world file** and a World view; Shared 4 (recommended): the **`Draw` API** for editor drawing.

## M1. Rooms and moving between them

**What it gives:** walk through a door into a house or down into a cellar, with a fade or a loading screen; things that belonged to the room go when you leave; what should persist (the player, the inventory, the day) carries on.

**Ways to build it:**
1. **Swap the level:** unload the current level's entities (except those marked persistent), load the next level's, place the player at an entry point. Simple, low memory; a short load (on a worker, behind a fade).
2. **Additive rooms:** several levels loaded into one world at once; entering a room loads it beside the others; leaving unloads it after a delay. Seamless (no fade needed), neighbours ready in advance, but more memory and rules about overlaps.
3. **Everything in one level**, rooms hidden and shown. Instant, but every room costs memory and tick time.

**Recommendation:** support **1 and 2 with the same machinery**: every entity carries the room (level) it came from; `Rooms:open(Level.Cellar)` loads a level's entities into the running world (built on a worker from the level file, landed at a tick's end like a big spawn), `Rooms:close(Level.Cellar)` removes everything the room made (its entities and anything spawned "in" it, by the Lust `node` rule the ideas list proposed: "what it made ending when it closes"). A transition (`Rooms:goTo(Level.House, Entry.FrontDoor, Transition.Fade)`) is open-new + move-player + close-old, behind a fade (Shared 3's fade) or a loading screen (a game UI screen). 2D 1's world map gives neighbours to pre-load for seamless 2D worlds.

- **What a room keeps when it closes:** options per room: **reset** (comes back as designed: a dungeon), **remember** (its changes kept in the save: a farmhouse whose furniture moved), or **keep running** (stays loaded). Remember means storing the room's changed entities in the save (a diff against its level file, as prefab overrides do).
- **Determinism:** opening and closing land at tick boundaries; loading on a worker must not make the tick depend on timing: the request is made on tick N, the room appears exactly on tick N + k for a fixed k (or the game pauses until it's ready, which is deterministic in ticks if the pause is counted as zero ticks). Recommended: the game pauses during the fade (game time doesn't run), so loading time never changes game results.
- **Persistent things:** entities marked "Keep between rooms" (the player, a companion) and game-wide objects.
- **Game code:** `Rooms:goTo(Level.Cellar, Entry.Stairs)`, `Rooms:open`, `Rooms:close`, `is Rooms:isOpen(Level.Cellar)?`, events `RoomOpened(Level level)`, `RoomClosed(Level level)`. `Entry` names come from marker entities in each level (generated states, rule 4).
- **Memory ownership:** DESIGN.md §3 and §8 want "states that own memory" for zones. hecs doesn't partition by room; deleting a room's entities is a query by a `Room` component. Fine at this scale; measure.

**Tests:** in and out 1,000 times leaves no entities behind (count); remember-mode changes survive save/load; transitions replay exactly; a room opening while game code spawns into it lands in a fixed order. **Scale case:** opening a 5,000-entity room within one frame budget on a worker plus a landing frame under target.

**Size:** large.

## M2. Pools

**What it gives:** bullets, sparks and dropped items reused so huge numbers stay smooth.

**First measure:** spawning today changes the level document and the hecs world at the tick's end (decision 0035), and each Lust component is made in Luau. The cost per spawn decides whether pools are needed. **Recommended:** a scale case "1,000 spawns and 1,000 despawns of a 3-component prefab per tick" first; make plain spawning fast (batch insertion, no document change for runtime spawns that the editor doesn't need to show, cached component templates) before adding pools.

**Ways:**
1. **Faster spawning** only (often enough; ECS spawns are cheap; the costs are in Solis's own bookkeeping and Luau component creation).
2. **Engine-managed pools:** a prefab setting "Keep N ready"; `despawn` on a pooled entity disables it (hidden, out of physics and ticks) instead of destroying; `spawn` takes one from the pool and applies Shared 2's spawn-with-values overrides, resetting fields to the prefab's values first. Game code doesn't change.
3. **Game-managed pools** (an assembly that keeps what it makes: the ideas list's sketch): explicit `take` / `give back`.

**Recommendation:** 1, then 2 if the measurements say so. 2 keeps game code unchanged (the best kind of pool is one you don't see). Determinism: entity IDs must stay deterministic: a reused entity gets a **new ID** (from the tick's order, as spawns do) even though its storage is reused, so code holding the old ID sees it as gone. It needs an "inactive" state on entities, which Solis doesn't have yet (useful anyway: a disabled-in-editor checkbox).

**Tests:** a pooled entity's fields reset; a reused entity has a new ID; checksums identical with pooling on and off. **Size:** medium.

## M3. Seeing inside the game while it runs

**Debug drawing:** the `Draw` API from Shared 4 (line, circle, box, sphere, cone, arrow, text, path), callable from game code while playing, for one frame or for a set time (`Draw:circle(p, 3, Colour.Red, 2.0)`). Output only (never game state), so safe on every core (each shard's list joined in entity order); capped per frame; drawn in the editor's views, the Game panel, and in exported builds only when the developer turns it on (a project setting, off by default). Also built-in overlays: physics shapes (2D walls exist; 3D colliders), paths (2D 1), triggers (2D 2), navmeshes later.

**The cheat box:** type `give seeds 10` or `time 18:00`.
- **Your own commands:** a passed object whose methods are commands, e.g. `object Cheats { method give(Item item, int count) { … } method time(float hour) { … } }` named in project settings, or a `command` word in Lust. Arguments are parsed by their types (numbers, text, a state's options by name with completion, entities by name); built-in commands too (`time`, `speed`, `teleport`, `spawn`, `save`, `load`).
- **Commands change the game, so they're input:** a typed command becomes an event in the next tick's input frame (as menus' events do, decision 0088), so it's recorded and replays exactly. This is essential for the bug-report replay to work after a cheat.
- **Where:** the editor's Console (exists) while playing; in exported builds behind a developer setting and a key (ask: recommended `` ` `` / the key left of 1, which games commonly use; off in builds by default).

**Live values:** pin fields of chosen entities (or a game-wide object) to a small overlay while playing (a "watch" list), and the Properties panel already shows game values. Add sparkline graphs for numbers (last N ticks). **Keys:** none new in the editor.

**Tests:** commands replay; an unknown command or bad argument says why; draw calls never change checksums; the cap holds. **Size:** medium.

## M4. Bug reports from playtesters

**What it gives:** a playtester presses a key (or picks "Report a problem" in the pause menu), types a sentence, and a file is made with a recording of everything they pressed, so the developer replays exactly what went wrong.

**What goes in a report** (one file, a zip-like archive the editor opens):
- the **recording from a known starting point**: the start of the session, or the last load of a save plus that save (recordings from a save need the save too);
- the game's **build identity**: the exported program's build number and a hash of the project's code and data (a replay is exact only with the same code and data: the editor warns when they differ);
- the player's note, a screenshot (read back without waiting, as save pictures in Shared 4), `log.txt`, the crash log if any, the player's settings, Windows version and graphics card (no personal data: no user name, no paths outside the game's folder);
- the checksums every N ticks, so the replay can say "your replay diverged at tick 4,312" if something isn't deterministic (a bug of its own worth knowing).

**Replaying:** File › Open bug report: opens the project at that build's data (or warns), loads the starting save, replays the recording with the playback controls (pause, step, jump to tick by re-simulating from the start or from periodic snapshots), and shows the note and log. Snapshots every few seconds make jumping fast (save games are snapshots already).

**Sending:** a folder of reports in `Saved Games/<Game>/Reports` the playtester zips and sends (as the playtest export does now). Uploading somewhere is the game developer's business (decision 0098's spirit: no services built in).

**Size of recordings:** a frame of input is small; long sessions grow; recordings can be capped to the last N minutes from a rolling snapshot (keep a save every few minutes, and the input since the oldest kept one).

**Tests:** a report made in an exported game replays to the same checksums in the editor; a report from another build warns; a damaged report is refused with a message (fuzz the reader, rule 9). **Size:** medium.

## Dependencies and order

- M1 builds on 2D 1's world file and rooms, Shared 3's fade and Lust's `node` idea; M2 on Shared 2's spawn-with-values; M3's drawing on Shared 4's `Draw`; M4 on recordings (exist) and M3's commands-as-input.
- Rooms in 3D: the same mechanism serves "zones" (DESIGN.md §8) for 3D levels; Open worlds streams ground, not entities, so M1 is the entity side of streaming.

## Risks and questions for the project lead

1. **What a room keeps when you leave:** **recommended** a setting per room (reset, remember, keep running), remember by default for houses; alternative: always remember.
2. **Loading while the game waits:** **recommended** game time pauses during a transition so loading speed never changes results; alternative: fixed delay in ticks.
3. **Pools:** **recommended** measure and speed up spawning first, then engine-managed pools as a prefab setting; alternative: explicit pools in game code.
4. **Cheat box in exported games:** **recommended** off by default, a project setting to allow it in playtest builds, opened with the key left of 1; alternatives: editor only; always on.
5. **Bug report key:** **recommended** F8 in playtest builds and a pause-menu item; alternative: menu item only.
6. **What a report may hold:** **recommended** as listed, nothing personal.

## Crates

`zip` (MIT) if reports are single archive files (it may already be in the tree via other crates; check), or a plain folder per report (no crate). Recommended: a folder (no dependency; the playtester zips it, as with exports).

## Size

Phase: large (rooms large, the rest medium).

## Sources

- Lust's `node` (opens and closes; closing ends what it made): `LUST.md` §9 (local copy).
- Solis decisions 0035 (spawning during play), 0038 (saves), 0088 (UI events as input), 0093 (playtest export).
- Deterministic replay for debugging is the same principle as lockstep netcode's (DESIGN.md §6): [Gaffer On Games, Deterministic lockstep](https://gafferongames.com/post/deterministic_lockstep/).
