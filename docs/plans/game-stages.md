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
- **Timers and steps in order:** "open in 2 seconds", "walk there, wait, say this". (Shared 3) *Built* ([below](#timers-and-steps-in-order)).
- **States that know when they start and stop.** (Shared 3) *Built* ([below](#the-next-milestone-states-that-know-when-they-start-and-stop), with its As built note).
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

### Timers and steps in order
Named waits game code starts on a component (stage 1's fourth milestone, branch `stage1/timers`; [decision 0125](../decisions/0125-named-timers-now-blocks-later.md)):
- `startTimer(Timer.DoorOpens, 2.0)`, `startTimerEvery`, `startTimerAtHour`, `stopTimer`, `timerRunning`, `timeLeft`, and `TimerFinished(Timer which)` to the component that started it alone (`Event::only`), in the tick's first round, in (due, entity, component, start) order; Lust, Lua and Rust.
- Names gathered from where the game's Lust starts them into `Timer` in `solis/project.lst` (`script::timer_names`): Lust can't yet take a component's own state in a native or an event, so a local `state Timer` waits for Lust (asked: blocks run later and local timer names, below).
- Exact game ticks (units of 1/65,536 tick, following `setGameSpeed`); a clock jumped past an hour fires once; kept by the game (`script::Game`), ended with their component, kept by new code while their name is started.
- Saves version 11 (a `timers` section), the checksum and state parts once one waits; the two-PC game's hive pulses and rings, and its drones' timers end with them.
- Scale: 100,000 waiting, 10,000 finishing a tick: the store's share 1.2 to 1.6 ms a tick (release), about 0.9 ms more making their events.
- Steps in order in the manual with one timer and a field for the step.

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

## The next milestone: states that know when they start and stop

**What it gives** (the project lead's answers, 2026-11-18; [report 05](../reports/after-the-rethink/05-shared-3.md), M3): a stunned character flashes, plays a dizzy loop and starts a timer to recover, and everything started while stunned stops by itself when the stun ends, however it ends.

- **No new Lust words:** a component's own Lust `state` is the tool ("can't we use state for this?"). Entering is just the code that sets `state = Mood.Stunned;`.
- **Things started in a state end with it:** each timer, tween and looping sound a component starts while in an option belongs to it and ends when the component leaves it (a timer without `TimerFinished`, a tween where it is without its done block, the loop stopped). Spawned entities stay. Options inside options count separately, as in Lust. A component without a state of its own, or something started outside its methods, belongs to no option. A small way to opt out.
- **A state-changed event:** noticed as the tick ends, heard in the next tick's first round after `TimerFinished`, by entity then component; carrying `from` and `to`, typed; once a tick (first `from`, last `to`), nothing for a round trip. Ending happens at the moment of the change, before any handler.
- **Where the state is read:** the component's state at the moment of the call; no reading every component's state every tick.
- **Game state:** what each pending thing belongs to, and what's waiting to be heard, saved and in the checksum (only once something belongs, so older recordings keep their sums).
- **The two-PC check** gets a component whose timers and loop end with its state. **No editor UI and no new keys.**

**As built** (branch `stage1/states`; [decision 0126](../decisions/0126-things-started-in-a-state-end-with-it.md)):
- **The event is named after its state,** declared by the game beside it: `event MoodChanged(Mood from, Mood to);`. An event is one name in the whole program with fixed types, so one `StateChanged` can't carry each component's own state; and its declaration must see the game's state, so the game writes it. Lust's compiled code gives a component's state's options but not its name, so Solis reads the name from where the component sets its starting option (`state = Mood.Calm;` in its `has`; `script::state_names`), and checks the event as the code compiles: declared otherwise, it's a mistake saying how to write it. Only components whose event is declared, and which handle it (`on MoodChanged(…)`, read from the source too), are followed at all.
- **Lust's runtime is told about states** (`script::states::own_states`, as `exact_maths` adjusts it, on existing lines so no line moves; a Lust that reads otherwise is a plain mistake, never a half change): its `setstate`, which every change of an object's own state goes through, stops the component's tweens of the options it left (where they are, without their done blocks) and tells Solis through a hidden native, kept in the tick's order as `Change::State`; its `tween` notes the option of the component running as the tween starts.
- **What's running:** a Lust component with a state of its own runs wrapped (Solis's own Lua, `states::RUNNING`), noting itself as the one running, so a timer or sound native reads its `state` at the moment of the call. Components without one aren't wrapped and cost nothing more; nothing reads every component's state.
- **Ending:** the game ends a component's timers of the options it left in order with their starts (`TimerOp`; the timer store keeps each component's timers that belong to an option, so a change looks at those alone). A `TimerFinished` made as the tick began, from an option the component has left by the time it's delivered, isn't delivered (`Event::during`, checked against the component's state at delivery). Sounds: each play says what it belongs to (`Started`); the app numbers each (component, option) (`app::loop_owners`) and fades that number's loops (an event set to Loop; a one-shot finishes) with the sound device's `stop_game_loops_of`. Spawned entities are never touched.
- **The opt-out is `keepStarted()`** (`solis/states.lst`), called bare: what the component has started in its state so far no longer belongs to it, as Lust's `keep` holds onto an object; what it starts after belongs as usual. Chosen over a `…Kept` variant of every start (many calls, and a tween has no call to vary) and Lust's own `keep` (which takes an object).
- **Heard once a tick:** each change lands with its phase; as the tick ends the game sorts them by (entity, component), keeping each component's in order, and makes `<State>Changed(first from, last to)` for each that moved, sent to that component alone (`Event::only`) in the next tick's first round after the timers' events. What waits is game state.
- **Lua and Rust:** no state of their own (a Lua component's fields are plain values Solis doesn't see change; a Rust component's are its own), so what they start belongs to no option. `keepStarted` is there for them too and keeps nothing.
- **Saves** are version 12 (a timer's line ends `in OPTION`; a `states` section holds what waits; 11 upgrades with none; the fixture has both; the save fuzz target's seed is it). There's no "last seen state" to keep: loading sets each component's state without going through `setstate`, so nothing is heard twice, and the timers loaded still end with their state. Tweens still aren't saved (decision 0106). The checksum adds a timer's option, and what waits, only when there is one, so recordings from before keep their sums.
- **The two-PC game:** the hive's swarm wakes and settles every fourth drone; buzzing, it stings on a repeating timer, glows with a tween (no done block) and hums on a loop (`hum.solsound`, set to Loop, with no takes), all ended as it settles, and it counts its `SwarmChanged`s; its pulse and bell, started first, are kept with `keepStarted()`. The game's code changed, so `record_the_game` recorded it again (a recording keeps each tick's checksums; it took seconds here, not the hour feared), with the same scripted gamepad. At its last tick the hive has changed state three times, stung 16 times, and its `Sting` waits `in Buzzing` while `Pulse` and `Bell` belong to none.
- **Scale** (`script` scale check, release build, one core; the coordinator's shape, 2026-11-19): 10,000 components with a state of their own, each with a timer and a tween waiting in it, and **1,000 of them changing state each tick** (their timer and tween ended and started again, and the change heard by an empty handler). **Solis's own part is 4.8 to 6.6 ms a tick, over the 2 ms target**; the test asserts the 8 ms reached. It's measured as the same game with a plain field for the mood (nothing ends, nothing is heard) taken from it; the whole tick is about 20 ms. All 10,000 changing every tick is reported, not asserted: 46 to 54 ms of Solis's own (the whole tick 83 to 91 ms). Each change crosses from Luau into Rust once, and its timer's ending, the change landing and its event cost about 3 µs together at this size (wrapping the 10,000 components' updates to note which runs costs about 0.4 ms a tick in all).
  - **What it took:** the first build took about 100 ms for all 10,000 ending alone. A profile found each component's tweens kept in a set whose keys came and went every tick (now a list), every timer start making a string of every native's key for a mistake that wasn't there (now only for a mistake; timers everywhere are quicker), and strings made for every change (now Lust's own, options shared). **Hearing was slow for Solis's own reasons**, `TimerFinished` too: the 1 ms for timers' events counted only making them, while delivering 10,000 to empty handlers took about 28 ms (`TimerFinished`) and 42 ms (`MoodChanged`). Each event built its values' list with new Luau strings (two options, and `n`), its handler's name and a copy of the event. Now strings and lists of options are made once a phase and shared (Lust only reads them), the `on_` name only for a Lua component, and the event is lent, not copied: delivering 10,000 went to about 15 ms (`TimerFinished`) and 23 ms (`MoodChanged`), the rest being the call into Lust and Lust's handler checking its values. Only components that handle their state's event are followed, so a game that doesn't listen pays only for the ending.
  - **What Lust would need** to reach 2 ms: the change told to the engine without a call per change (a host hook the runtime calls once per batch, or the engine reading a list Lust keeps), and a way to stop a component's tweens; with both, Solis would neither adjust Lust's runtime nor wrap updates (asked below).
- **Not built:** showing a component's state and what belongs to it while playing (a follow-up: a States part in Properties while playing, needing a mock-up first).

**Asked of Lust** (ready to send):

> **An engine event that carries the component's own state.** Solis's components now hear their own state change, but each game must declare an event per state (`event MoodChanged(Mood from, Mood to);`), since an event has one set of types for the whole program, and Solis reads which state a component has from its source, since the compiled schema gives only the options. Could Lust:
> 1. let an engine file declare an event whose values are "the receiving component's own state" (for example `event StateChanged(own from, own to);`), checked in each component's `on StateChanged(Mood from, Mood to)` against that component's own state, so one engine event serves every state; and
> 2. name the state in a component's schema entry for its own `state` (`{ "state", "state", "state", { … }, "Mood" }`), so the engine needn't read the source?
>
> **And state changes told cheaply, for speed:** at 1,000 changes a tick Solis spends 5 to 6 ms, mostly crossing from Luau into Rust once per change. Could the runtime:
> 1. keep the changes of components' own states in a list the engine reads once a phase (component, from, to, and the place in its component's calls, so the engine can order them with its own calls), rather than calling the engine at each change; and
> 2. let the engine stop the tweens a given component started in a given option, without their done blocks, and say which option each tween started in (the component running as it started),
>
> so Solis needn't adjust Lust's runtime, nor wrap each update to know which component runs.

