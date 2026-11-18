# The game stages: Solis's order, by what it lets you make

The order of work from 18 November 2026 ([decision 0122](../decisions/0122-the-game-stages.md)). The project lead wants to make every kind of 2D game in Solis, learning as they go, with 3D later because 2D features are easier to build first. So the 2D work is ordered by **the kind of game it lets you make**, one stage at a time.

**What doesn't change:**
- **Tools, never game systems** (CLAUDE.md, "The project lead's games are made with Solis, never built into it"). An inventory, quests, shops or crafting are games made with Solis's tools, such as text effects, drag and drop, tables and tags; they're never built into the engine. A line below may name a game use, but only as an example of what its tool is for. **The one exception is dialogue** ([decision 0124](../decisions/0124-a-dialogue-system-in-the-engine.md)): a full dialogue system is built into Solis, in stage 1.
- **Every feature is a general tool any game can use** (CLAUDE.md, "The project lead's games are made with Solis, never built into it"). A stage only decides *when* a feature is built. The features are the ones ROADMAP.md's phase sections already describe; each line below names the section it comes from.
- **How each is built** (technique, code, tests) is in [the plan after the rethink](after-the-rethink.md) and its research.
- One milestone per pull request, UI mocked up first, keys asked about first (rule 17), the scale target kept.

**What's new:**
- **Only the next milestone is planned in detail.** The stages below are the outline, revisited at every stage's end.
- **Each stage ends with a sample game of its kind,** made with Solis's own tools and offered in Learn, as the playground and the turret sample are. The project lead tries it as much as they like. They don't have to make a game to finish a stage.

## Stage 1: a top-down adventure

Walking a world, rooms and doors, talking to people, picking things up, an inventory, saving. Solis is closest to this already: top-down walking, tile maps, day and night, game UI.

- **Triggers:** walking into or out of a zone (a door, a counter, a pit) tells game code. (2D 2) *Built* ([below](#triggers)).
- **The same game on two PCs:** a recorded game replayed on two machines, checksums compared tick by tick, before timers put more maths into game state (decision 0120). (Shared 2) *Built.*
  - **As built** (branch `stage1/two-pcs`): `tests/two-pcs/` holds a small game (`game/`: 3D character, crates, a moving platform, drones spawned at random and despawned, a turret's projectiles and hits, a box zone and a round one that fires once; a 2D walker with a sprite and an animator in a tile map whose walls game code raises and lowers, a freeform zone; the game-wide `Director` changing the game's speed; Lust, Lua and Rust, the Rust being Solis's own Armour), ten seconds of recorded gamepad play (`game.solrec`, 48 KB) and the state at the end of every tick (`game.states`, 260 KB): both checksums, and the state part by part (`world::state_parts` with game code's values, `Workbench::game_state`), kept as what changed since the tick before, each part numbered once, a value over 32 characters kept as a checksum of it (`editor::StateList`). `crates/app/src/two_pcs.rs` plays it on one core and on four in `cargo test --workspace`, so on CI's Windows machine in every pull request; the first tick that differs fails with the parts named, both states written to `target/two-pcs/`, which the Windows job uploads, and a sentence on what to do. `SOLIS_UPDATE_TWO_PCS=1` writes the list again; `record_the_game` (ignored) records the game again. About 1.1 s on one core and 1.7 s on four here (the two run side by side; a debug build).
  - **Kept clear of decision 0106's gaps:** no tween has a done block, and no Lust `Random`.
  - **Found:** `--check-project` read prefabs as levels, so since they became `.prefab.lst` files a check knew none and a recording's spawns made nothing; fixed. Linux and four cores agree with Linux on one core. **Windows' first run** (pull request 284) differed at tick 9 in the `Director` object's `Maths:sin`, `exp` and `atan2`: Luau's `math` called the C library, which rounds differently on Windows. Game code's maths now comes from `libm`, pure Rust, the same bits everywhere (`script::exact_maths`, with each function's bits pinned in a test); Lust's runtime raises to powers with it too. A call costs 13 to 60 ns more (release build), so the benchmark's 20,000 sines and cosines a tick at the stress level add about 0.6 ms of game code's time, shared between cores. Lua's own `^` still calls the C library: the manual says to use `math.pow`.
- **Tags and questions about the world:** "all of a kind within 3 m", "the nearest". (Shared 2) *Built* ([below](#tags-and-questions-about-the-world)).
- **Timers and steps in order:** "open in 2 seconds", "walk there, wait, say this". (Shared 3) *Built* ([below](#the-next-milestone-timers-and-steps-in-order), with its As built note).
- **States that know when they start and stop.** (Shared 3)
- **Field hints and more kinds of settings:** sliders, limits, help, and fields that pick a sound, a sprite, a prefab or a list. (Shared 4, with 2D 1's typed fields)
- **Data tables:** any game's data (rows and typed columns) in one file, edited as a table and read by game code with its spelling checked. For example, items with prices and descriptions. (Shared 4)
- **Text layout, then text effects:** text that wraps, types itself out letter by letter, and has coloured, wavy or shaking words and icons inside it, for any text a game shows: menus, signs, item descriptions, and dialogue boxes. (Shared 5)
- **Translations and subtitles:** the game's text in several languages, with a check for missing lines, and captions for sounds and speech. Brought from the finishing stage for dialogue ([decision 0124](../decisions/0124-a-dialogue-system-in-the-engine.md)). (Shared 5)
- **Dialogue, a full system in the engine** (the project lead, 2026-11-18; [decision 0124](../decisions/0124-a-dialogue-system-in-the-engine.md), [the outline](dialogue/README.md)): conversations as graphs of lines, choices and branches, with conditions and effects that call game code; a dialogue box on text effects; a conversation editor on the shared graph editor; starting from the world with triggers, timers and states; voice lines and sounds at moments in a line, with subtitles in step; and one place for all of a game's dialogue. Several milestones, each mocked up first. If one needs something that comes later, it moves behind it.
- **Tooltips, and drag and drop in game screens:** any piece of a screen can show a tip and be dragged onto another, and game code hears what was dropped where. For example, inventories, shops, equipment slots or a hotbar. (Shared 5)
- **Fades:** to black and back, or another colour, as a general tool. (Shared 3, brought from stage 2's shake and flash.)
- **Rooms on a world map, and moving between them,** with a fade. (2D 1 and Shared 6)
- **A Door in the kit,** built on triggers, fades and rooms (the project lead, 2026-11-18): **Send to** a place in this level or an entry point in another room or level, **Two-way** (two doors linked, each sending to the other), and a **Transition** (none, or a fade, with its time). Written in Lust and open to read and change, like the kit's top-down walker. The trigger zone itself stays lean.
- **Sorting tall things:** walking behind and in front of trees and houses properly. (2D 1)
- **Pathfinding:** characters walk round walls to where they're going. (2D 1)
- **Save slots:** several saves with a picture and the time played. (Shared 4)
- **The sample:** a small top-down adventure in Learn, with conversations made in Solis's dialogue system, and an inventory written in Lust as a game maker would write it, not built into Solis.

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
- **Button pictures for each controller.** (Shared 5; translations and subtitles moved to stage 1, decision 0124)
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

## Built

### Triggers
Zones that tell game code when something walks in or out (stage 1's first milestone, branch `stage1/triggers`; [approved mock-up](../design/triggers/README.md)):
- `TriggerZone` (`level::builtin`): a box, a round shape (a circle in 2D, a sphere in 3D) or, in 2D, a freeform outline drawn point by point; Notices, Fires (every time or once), and Only tags, which became real with tags.
- Solis's own overlap pass (`world::triggers`) as each tick begins, on a tree of boxes made from where moving things are now; results sorted by zone and entity, so the same on any number of cores. Who is inside is game state (saves are version 10, with a `zones` section).
- Game code: `Entered(entity who)` and `Exited(entity who)` on the zone, `EnteredZone(entity zone)` and `LeftZone(entity zone)` on whoever walked, `inside()` on a zone; in Lust, Lua and Rust.
- The editor: Trigger zone on Home's Create, the teal outline with handles, the TriggerZone card and What game code hears with New code for this zone, freeform drawing and editing.
- Scale: 2,000 characters among 500 zones, about 1.2 ms a tick in a release build (target 2 ms).

### Tags and questions about the world
Labels such as Enemy or Collectible, and game code asking "every enemy within 3 m, nearest first" and "the nearest collectible" (stage 1's third milestone, branch `stage1/tags`; [mock-up](../design/tags/README.md)):
- The project's list in Project settings › **Tags**, kept in `tags.lst` (`level::tags`, version 1) by stable IDs (`reflect::TagId`), so a rename touches only the list; chips under an entity's name, prefabs carrying them (a copy's own change wins), the Level panel's search by tag.
- Game code: `Tag.Enemy` in `solis/project.lst`; `hasTag`, `addTag`, `removeTag` on entities and `World:within` and `World:nearest`, in Lust, Lua and Rust, answered from a uniform grid made once a tick (`script::TagView`), the same on any number of cores; trigger zones' Only tags real.
- Game state: kept over edits while playing, saved in the save's level, in the checksum and state parts once something carries a tag.
- Scale: 2,000 tagged units each asking `nearest` and `within` 3 m, about 0.8 ms a tick in a release build (target 2 ms); the same in Lust end to end 6 to 9 ms.

## The next milestone: timers and steps in order

**What it gives** (the project lead's answers, 2026-11-18; [report 05](../reports/after-the-rethink/05-shared-3.md), "The central problem" and M1): "open in 2 seconds", "every 5 seconds", "each day at six", and "walk there, wait, say this", as game state: saved, in the checksum, the same on one core or sixteen.

- **Named timers now;** blocks run later (`after 2.0 { … }`, `every 5.0 { … }`) wait for Lust, and will use only the component's own fields so saves keep them. The timer store is shaped for them: pending work by (due tick, entity, component, start order).
- **Names in the component's own code** (`state Timer = [DoorOpens, Respawn];`), kept by (entity, component, name), no project-wide list.
- **Game code** on the component: start a timer (a running one starts again), start one that repeats, start one at a time of day, stop it, ask whether it runs and how long is left; `TimerFinished(Timer which)` to the component that started it. Lust, Rust and Lua.
- **Exact game ticks:** seconds made whole ticks once; following `setGameSpeed`, still while paused; a repeating timer's next turn from its last.
- **Time of day:** a clock jumped past the hour fires it once; midnight the same.
- **Order:** the tick's first round, by (due tick, entity, component name, start order).
- **Ends:** with the entity or component, silently; new code keeps a timer while its component and name exist, else one Console note.
- **Game state:** saves (format 11, upgrader, fixture, Breaking line), the checksum and state parts once a timer exists; a save loaded mid-wait fires on the same tick. The two-PC game gets timers.
- **Steps in order** in the manual with one timer and a field for the step; no kit helper, no `wait`.
- **No editor UI and no new keys.**

**As built** (branch `stage1/timers`; [decision 0125](../decisions/0125-named-timers-now-blocks-later.md)):
- **The names, and why not a local state:** Lust (alpha 74) can't make the lead's shape work. An engine call's types are its own file's, and Lust has no "any state" type, so `startTimer` can't take each component's own `Timer`; and an event has one name in the whole program with fixed types, so `TimerFinished` can carry only one `Timer`. Checked against the compiler. The closest shape the guidelines allow: `Timer` is a project state in `solis/project.lst`, its options the names the game's Lust starts (`script::timer_names`: every `Timer.X` given to `startTimer`, `startTimerEvery` or `startTimerAtHour`, comments and text skipped), gathered as the engine is described, before the code is checked. No list to keep; a misspelt name anywhere but its start is a mistake before the game runs; the calls and handler are exactly what a local state will need, so moving the names into components later changes nothing else. Only a start call names a timer (a timer chosen through a variable must still be started by name somewhere). Lua and Rust name a timer by its text.
- **The calls** keep the research's names, which fit the guidelines' shapes: `startTimer(Timer.X, seconds)`, `startTimerEvery`, `startTimerAtHour(Timer.X, hour)` (variants' differences at the end), `stopTimer`, `timerRunning` (read after `is`), `timeLeft` (a noun; 0 when not running), entity methods called bare in a component (`solis/timers.lst`). On another entity, in a game-wide object or in a tween, a mistake with its line. Starting a running timer restarts it, as the brief asks, which the manual says to do once, not every tick (the guidelines' rule 8 would rather "already happening does nothing").
- **Ticks:** seconds rounded to the nearest tick; 0 finishes on the next tick, even at speed 0, never in the same round; a repeat waits at least a tick; the longest is 10,000,000 s. Game time is counted in units of 1/65,536 tick, each tick adding the speed in those, so 0.5, 2 and 0.25 count exactly. A repeating timer behind by several turns in one tick fires once and keeps its rhythm, as Lust's own `every` does. `timerRunning` and `timeLeft` answer as the tick started; starts and stops land at its end.
- **Time of day:** checked as each tick begins, from the hour as the last tick began to the hour now; setting the clock moves the day forward (as the world already does), so setting it back from seven to five passes no six.
- **Kept by the game** (`script::Game`, not the world): fired as each tick begins, before the game-wide objects, after the editor's engine events; `TimerFinished` reaches only its component (`Event::only`). Ended through `retain_on`/`retain_only`; after new code, dropped when their component is gone or (Lust) their name is no longer started, with one note listing them.
- **Saves** are version 11 (a `timers` section; 10 upgrades with none; the fixture holds a timer waiting; the save fuzz target has it as a seed). The checksum adds each timer's sum once one waits (so recordings from before keep their sums), and the state parts list them.
- **The two-PC game:** the hive pulses every 0.75 s through the `Director`'s speed changes and rings a bell as the day passes 8:03; each drone's one-second timer finishes unless it's despawned first. Timers belong to components, so the `Director` object can't start them (the brief's example), and the hive does. The game's code changed, so `record_the_game` recorded it again (a recording keeps each tick's checksums); the input is the same scripted gamepad.
- **Scale:** 100,000 timers waiting, 10,000 finishing and started again each tick: the store's own share about 1.3 ms a tick in a release build (target 2 ms), and making their 10,000 `TimerFinished` events about 0.9 ms more (`script` scale check; the events' text is three small allocations each). The first store, a sorted set of (due, entity, component, start, name), took 5 ms; slots found by numbers and buckets by due time made it four times quicker.
- **Not built:** showing running timers while playing (a follow-up: a Timers part in Properties while playing, needing a mock-up first).

**Asked of Lust** (ready to send):

> **Blocks run later, for engines.** Solis has named timers now (`startTimer(Timer.DoorOpens, 2.0)`, `on TimerFinished(Timer which)`) and would like blocks run later next:
> 1. `after N { … }` and `every N { … }` in a component's methods, for engines: the block runs N seconds of game time later (or every N), as the engine says.
> 2. A block may use only the component's fields and `this`. The checker refuses a local, a parameter or a loop variable inside one with a clear message ("count is a local, so it would be lost if the game is saved while this waits; keep it in a field"), so a pending block can always be saved.
> 3. Each block gets a stable ID: its file, its owner (the component) and its number among that owner's blocks, the same after a reload and after a code swap while the block is still there.
> 4. The host can list pending blocks, save them and restore them, and is told whose each is (entity, component), so it can order them by (due, entity, component, start order) on every core, drop them with their entity, and keep them across code swaps.
>
> Tweens' done blocks would use the same: Lust saying whose each tween is and giving its done block a stable ID closes decision 0106's three gaps (done blocks ordered across cores, kept across a code swap, and saved).
>
> **Also, for timer names in the component's own code:** a native method and an engine event that take the calling component's own state of a given name, so `state Timer = [DoorOpens, Respawn];` in each component works with one `startTimer` and one `TimerFinished` (today a native's types are its own file's, and an event has one set of types for the whole program).
