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
- **Tags and questions about the world:** "all of a kind within 3 m", "the nearest". (Shared 2) *Built* ([below](#the-next-milestone-tags-and-questions-about-the-world), with its As built note).
- **Timers and steps in order:** "open in 2 seconds", "walk there, wait, say this". (Shared 3)
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

## The next milestone: tags and questions about the world

**What it gives** (approved by the project lead, 2026-11-18; [mock-up](../design/tags/README.md)): labels such as Enemy or Collectible that entities carry, with the spelling checked before the game runs, and game code asking "every enemy within 3 m, nearest first" and "the nearest collectible". From [report 02](../reports/after-the-rethink/02-shared-2.md), M2.

**The project's list** (Project settings › **Tags**, a tab of its own): each tag's name and "Used by N entities and M prefabs"; **+ New tag**, which starts naming one; a double-click on a name renames it **everywhere** (levels, prefabs and game code's generated name); ⋯ › Delete, which asks first when anything uses it, saying what. Names are checked: a name Lust can make an option of, never twice, with a plain reason when refused. Kept in `tags.lst` in the project's folder, a Lust data file (decision 0081) with a format, a version, a fixture and a fuzz target; read when the project opens and watched for changes.

**An entity's tags:** under its name in Properties, chips with ×, and **+ Tag ▾** (a list with search, the project's tags, and New tag…). Each change one undo step; several picked entities show and edit the tags they share. The Level panel shows an entity's tags faintly after its name, and its search finds entities by tag. Saved in the level. **Prefabs carry tags** (the lead's answer): a prefab's are on every copy; a copy may add or take one away, with the changed dot and ↶ (R5's rule: a copy's own change wins).

**Game code** (rule 25): tags are a state, `Tag.Enemy`, in the project's Lust file; on entities `hasTag(Tag.X)`, `addTag(Tag.X)` and `removeTag(Tag.X)` (changes landing at the tick's end in order); on the world `World:within(Tag.X, place, radius)` (nearest first, ties by entity ID) and `World:nearest(Tag.X, place)` (the nearest, or nothing); in Lust, Rust and Lua. **Distance** (the lead's answer): across the ground in a 2D project, whatever the layers; in 3D in a 3D project or Both.

**Fast and the same on every core:** answered from a uniform grid made once a tick from where tagged entities are as it starts (decision 0033), its cell sized sensibly and big radii looking through the tag's own list; the scale target is 2,000 tagged units each asking in one tick. Tags are game state: saved, in the checksum and the two-PC check's state parts. **Trigger zones' Only tags** becomes real.

**Keys:** only the approved double-click to rename (rule 17).

**As built** (branch `stage1/tags`):
- **Kept by ID:** each tag has a stable ID (`reflect::TagId`); `tags.lst` (`level::tags`, version 1) holds IDs and names, and an entity's `Tags` component (`level::builtin`) one line of IDs. A rename changes only `tags.lst`: every level, prefab and save follows without being written again. A tag gone from the list (deleted, or lost in a merge) is carried by nothing: not shown, not seen by the game, and left out at the entity's next change of tags. A trigger zone's Only tags are IDs too (`only_tags`). A new component and a new field needed no new level version; `tags.lst` is a new format at version 1.
- **The names game code can use:** a capital first, then letters, digits and `_`, not all capitals, not one of Lust's own words or `None`, at most 40 characters, no two the same but for capitals; each refusal says why and suggests a name (`big door` → BigDoor). At most 1,024 tags a project and 32 an entity.
- **The calls' names** are the research's: `within` and `nearest` read as what they give back (no `get`, no `Of`), so they fit rule 5's shapes as readers; `hasTag` reads after `is`; `addTag` and `removeTag` are verbs. `hasTag` sees the tags as the tick started (a tag added is seen from the next tick), as `within` and `nearest` do; Lua and Rust name a tag by its text (`"Enemy"`), as they name the project's other names.
- **The grid** (`script::tag_view`, `TagView`): made once a tick by the editor as the tick begins (`move_world_on`), from `world::World::tagged`, and shared by every phase and core. Each tag's members are kept in ID order and, from 32 of them, counted into a uniform grid over the ground: cells sized from the area the members cover, about two to a cell, never more than four cells a member. `within` looks in the cells its square of distance reaches (a hair more, for rounding), or through the tag's whole list when that square has more cells than the tag has members; `nearest` looks ring by ring outwards until nothing nearer can be left. Both measure and sort the same way, so a property test holds the grid to a look through every member.
- **In the Library** the project's `tags.lst` (at the top of its files) shows as **Project's tags** with a gear, and opens Project settings › Tags; the code gatherer leaves it out of the game's program. **Several zones picked** show their shared Only tags by name, and leave the row out when they differ.
- **Used by** counts the open level as it is now, the project's other levels as saved (each read and its prefabs' copies made, on a worker, when the Tags tab is shown) and the prefabs carrying the tag.
- **Watching `tags.lst`:** read again when the project's files change and its time or size on disk differ (only its metadata is looked at otherwise). A project from before decision 0118 keeps its files in `assets/`, so its `tags.lst` beside `project.solis` is read when it opens and when the editor saves it.
- **Game state:** tags game code changes are kept over edits while playing (`world::game_set`), saved (the save's level holds them; no new save version) and in the checksum and the state parts only once something carries a tag, so the two-PC check's recorded game is unchanged.
- **Scale:** 2,000 tagged units in a 200 m square, half of them enemies, each asking `nearest` and `within` 3 m, with the view made once a tick: about 0.8 ms a tick in a release build (target 2 ms; `script` scale check). The same in Lust, end to end (`editor`'s `scale_check_2000_lust_units_asking_by_tag_each_tick`), is 6 to 9 ms a tick in a release build (target one frame, 16 ms), beside 3 ms for 2,000 components each finding one entity by name: each unit makes two calls and gets a list back.
