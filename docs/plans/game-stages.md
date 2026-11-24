# The game stages: Solis's order, by what it lets you make

> **The scope reset (2026-11-21, decision 0135) comes first:** see [scope-reset.md](scope-reset.md). The order below continues only where the "done" list needs it.

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
- **States that know when they start and stop.** (Shared 3) *Built* ([below](#states-that-know-when-they-start-and-stop)).
- **Field hints and more kinds of settings:** sliders, limits, help, and fields that pick a sound, a sprite, a prefab or a list. (Shared 4, with 2D 1's typed fields) *Built*, all but lists, which wait for Lust ([below](#field-hints-and-more-kinds-of-settings)).
- **Data tables:** any game's data (rows and typed columns) in one file, edited as a table and read by game code with its spelling checked. For example, items with prices and descriptions. (Shared 4) *Built* ([below](#data-tables)).
- **Text layout, then text effects:** text that wraps, types itself out letter by letter, and has coloured, wavy or shaking words and icons inside it, for any text a game shows: menus, signs, item descriptions, and dialogue boxes. (Shared 5) Text layout *built* ([below](#text-layout)); text effects *built* ([below](#text-effects)).
- **Translations and subtitles:** the game's text in several languages, with a check for missing lines, and captions for sounds and speech. Brought from the finishing stage for dialogue ([decision 0124](../decisions/0124-a-dialogue-system-in-the-engine.md)). (Shared 5) *Built* ([below](#the-next-milestone-translations-and-subtitles), with its As built note).
- **Fixes from making Orc Invasion** (the project lead, 2026-11-20; [decision 0133](../decisions/0133-fixes-from-making-orc-invasion.md), [the report](../reports/orc-invasion-hurdles.md)): a game made with Solis found 27 hurdles. Four milestones fix Solis's 23, before dialogue, and Lust's four go on the requests for Lust ([below](#fixes-from-making-orc-invasion)):
  1. Bugs, and a project check that starts the game. *Built* ([below](#fixes-1-bugs-and-a-project-check-that-starts-the-game-done)).
  2. The missing game-code calls.
  3. Clips by action and direction, and a character's other look.
  4. Solis's files written down, and playing a game from the command line.
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

### States that know when they start and stop
What a component starts in its own Lust state ends when it leaves it (stage 1's fifth milestone, branch `stage1/states`; [decision 0126](../decisions/0126-things-started-in-a-state-end-with-it.md)):
- No new Lust words: a component's own `state` is the tool. Each timer, tween and looping sound it starts in an option belongs to it and ends when the component leaves it (a timer without `TimerFinished`, a tween where it is without its done block, the loop faded); spawned entities stay. `keepStarted()` keeps what was started so far.
- The component hears its change through an event the game declares beside its state (`event MoodChanged(Mood from, Mood to);`), once a tick (first `from`, last `to`), in the next tick's first round after the timers' events; Lust can't yet give an engine event the component's own state (asked, below).
- Lust's runtime is adjusted on existing lines (`script::states::own_states`) so a state change stops its tweens and tells Solis; components with a state of their own run wrapped so a native reads their state at the moment of the call; Lua and Rust components have none, so what they start belongs to no option.
- Saves version 12 (a timer's option; a `states` section of what waits), the checksum only once something belongs; the two-PC game's swarm wakes and settles.
- Scale: 10,000 components with a timer and a tween in their state and 1,000 changing a tick, about 5 to 6 ms (release), mostly crossing from Luau into Rust per change; what Lust would need for 2 ms is asked below.

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

### Field hints and more kinds of settings
A component's own settings shown as its code asks (stage 1's sixth milestone, branch `stage1/field-hints`; [decision 0127](../decisions/0127-field-hints-and-pickers.md); [mock-up](../design/field-hints/README.md)):
- Hints written in a comment after a Lust field (`-- [range 0 10, step 0.5, unit "m/s", tip "…"]`), a Lua field's fifth entry, or a Rust schema's own: sliders, steps, units, tips, groups, several lines, hidden; mistakes and starts outside a range in Problems with their line; never saved in a level.
- Pickers for a field of the project's names (`Sound`, `Clip`, `Picture`, `Prefab`, `Screen`, `Level`, `Tag`): a list with search, thumbnails or icons, ▶ for sounds, and a Library file dropped on it; renamed files followed in the open level. `state Level` and every picture added to `solis/project.lst`. Colours show their hex.
- Lists not built: Lust refuses arrays in a component's fields (asked, below).

**Asked of Lust** (ready to send):

> **Attributes on a component's fields, for engines.** Solis reads hints from a comment after each field today: `float speed = 4.0; -- [range 0 10, step 0.5, unit "m/s", tip "Walking speed"]`. Could Lust take them as real attributes, in the `[ ]` words its screens use?
> 1. `float speed = 4.0 [range 0 10, step 0.5, unit "m/s", tip "Walking speed", group "Moving"];` on a `has` field of a component or an object; bare words too (`[multiline]`, `[hidden]`).
> 2. Checked like any code: a word the engine hasn't declared is a mistake with its line ("speeed isn't an attribute: range, step, unit…"), and each word's values have types (`range float float`, `unit string`). An engine names its words in a host file, so Lust stays general.
> 3. The schema carries them: Lua's `{ key, name, kind, options, attributes }` as a fifth entry (a table by word), and Rust's `Field` gains `attributes`. Nothing else changes how the code runs.
>
> **Arrays in a component's fields.** `vec3 array patrol = [];` refused today ("a component's fields hold bool, int, float, …"). Could a component (and an object) keep an array of the kinds it can already keep: `int`, `float`, `vec2`, `vec3`, a state, `entity`?
> 1. In the schema as `{ key, name, "array", itemKind, options }` (Lua) and `FieldKind::Array(kind)` (Rust), with `to_data` and `set_data` taking a list.
> 2. The engine keeps them in levels and saves and sets them between ticks like any field; `swap` keeps an array whose item kind is unchanged.
> 3. A length the engine can bound (it refuses past 1,024 items when reading files), and `from_data` keeping the starting value for a list of the wrong kind.


### Data tables
Any game's data as rows and typed columns (stage 1's seventh milestone, branch `stage1/data-tables`; [decision 0128](../decisions/0128-data-tables.md); [mock-up](../design/data-tables/README.md); [manual](../manual/data-tables.md)):
- `name.table.lst` (`level::table`, version 1): typed columns (whole number, number, text, yes or no, a choice, the project's names, colour), rows of an ID and cells; 100,000 rows, 64 columns; fuzzed.
- Game code reads `Items:row(Item.IronSword).price` and `Items:rows()` through a Lust file per table (Lua `solis.Items:row("IronSword")`, Rust `cx.host().tables()`), misspellings caught before the game runs; read-only while playing, with the tables' number in saves (version 13) and recordings (version 5).
- The Table tab with the common spreadsheet keys inside the grid, CSV in and out, every change one undo step; 10,000 × 20 opened, scrolled and typed in within 4 ms a frame.
- Not built: screens reading a table, a column of another table's rows, dragging rows and columns, column widths.

### Text layout
Game screens' words wrapped, shaped and cut (stage 1's eighth milestone, branch `stage1/text-layout`; [decision 0129](../decisions/0129-text-layout.md); [study](../reports/text-layout-study.md); [manual](../manual/screen-text.md)):
- **Solis's own layout, not parley** (the study: 0.8 ms against 32 ms for 2,000 changing lines, one new library against nine): Unicode's line breaks (`unicode-linebreak`), runs shaped with `harfrust` and kept by their words, paragraphs kept by text, style and fit; long words broken between letters; "…" when a text's `height`/`maxHeight` (or a `nowrap` text's `width`/`maxWidth`) is limited.
- A project's `.ttf` and `.otf` files are the Library's **Font** kind and `state Font`; screens name them with Lust's `font "fonts/Lora.ttf"`, fallbacks between commas, then Inter; read on a worker, checked, fuzzed, in Problems at their line.
- Line height and letter spacing on the Screen component. Scale: 2,000 changing lines a frame, 3.8 to 5.4 ms (release; target 8).
- Not built: right-to-left text, dictionary line breaks, per-text line height, spacing and "…" words (asked of Lust).

**Asked of Lust** (ready to send):

> **Text words for screens.** Solis's screens are Lust views, and Solis now lays their text out with shaping, wrapping, fallback fonts and "…". Four things only a Lust word can say per text:
> 1. `lineHeight 1.4` and `letterSpacing 2` on a text (or a style), the room between its lines (times the font's own) and after each letter (in units). Today they're the Screen component's, for a whole screen.
> 2. `lines 2`: at most so many lines, the last ending in "…"; and `ellipsis` on a `nowrap` text, cut with "…" when it doesn't fit. Today a text asks by limiting its own `height` or `width`.
> 3. `font` taking a list (`font ["fonts/Lora.ttf", "fonts/NotoSansJP.otf"]`), the first that has a letter drawing it. Today Solis reads `font "a.ttf, b.otf"`, which `lust run` takes for one file it can't find.
>
> Lust's own windows could then lay text out the way Solis's screens do.

### Text effects
Markup inside any game screen's text, letters moved without laying text out again, and text typing out (stage 1's ninth milestone, branch `stage1/text-effects`; [decision 0130](../decisions/0130-text-effects.md); [manual](../manual/text-effects.md)):
- **Markup** (`ui::markup`): `[wave]`, `[shake]`, `[colour=…]`, `[b]`, `[i]`, `[icon=Picture.Ui.Coin]`, `[pause 0.5]`, `[speed 2]`, `[[` for a `[`; mistakes in Problems at their line, shown as written; read after translation; `[button=…]` waits for button pictures.
- **Letters** (`ui::effects`): each letter's shape moved, coloured, slanted or hidden each frame; bold the game's own (thicker for a project's font), italic slanted; icons as boxes the text's height.
- **Typing out** on the game's ticks (the Screen's Typing speed, Typing sound, Sound on), the same at 30 and 144 fps; `TypingFinished(Screen which)` in the next tick's input, `finishTyping(Screen)` to skip; never game state.
- Scale: 2,000 letters waving, shaking and typing out, 0.36 ms a frame (release; target 4 ms).

**Asked of Lust** (ready to send):

> **Typing out, per text.** Solis's screens now type text out letter by letter (a Screen's Typing speed, with `[pause 0.5]` and `[speed 2]` inside the text), and game code hears `TypingFinished` when a screen's text is done. Two things only a Lust word can say per text:
> 1. `typing 40` on a `text` (or a style): this text types out at 40 letters a second, others on the same screen showing at once. Today every `text` on a typing screen types, and a name that shouldn't starts with `[speed 0]`.
> 2. A `typed { … }` block after a `text`, run when it has typed out (as `button`'s block runs when clicked), so a screen can `send LineTyped() to Ui:entity();` for one text rather than the engine's one event for the whole screen.
>
> Lust's own windows could then type text out too; they'd show Solis's markup words as written until then.

### Translations and subtitles

**What it gives** ([report 07](../reports/after-the-rethink/07-shared-5.md), M4, and its questions 2 and 3, the recommended answers taken; the quick sketch `mock-3.png`): the game's text in several languages, with a check for missing lines, and captions for sounds and speech; the foundation the dialogue system (decision 0124) builds its lines, voice and subtitles on: keys made once and never changed, the main language working with no translation files, and the translators' spreadsheet.

**As built** (branch `stage1/translations`; [decision 0131](../decisions/0131-translations-and-captions.md); [manual](../manual/translations.md) and [subtitles and captions](../manual/subtitles-and-captions.md); [mock-up](../design/translations/README.md)):
- **The `translations` crate** (new, layer 1): a key is a group and a name (`Shop.Welcome`), each part a Lust name; each language's lines in Fluent files, `locale/<language>/*.ftl` (a key's dots are dashes there), read with `fluent-syntax` within limits and put together with `fluent-bundle` (plurals by each language's own rules: English, French and Polish tested); the main language (`languages.lst`, `level::languages`) the fallback, then the key itself; the check (lines missing per language, lines nothing uses, values that don't match); the spreadsheet (`translations.csv`, every field quoted) out and in; a line set in a file's text keeping the rest. Fuzzed (`translation_file`).
- **Game code**: `state Text` and `state Language` in `solis/project.lst`; `World:text(Text.Shop.Welcome)`; a method per line with values in `solis/texts.lst` (`ShopText:apples(float count, string who)`, numbers for values that pick plurals); Lua `solis.World:text("Shop.Welcome")` (a misspelt key stops with the nearest) and `solis.ShopText:apples(3, "Mira")`; Rust `Host::text`, `text_with`, `texts`; lent from functions each Luau makes once per text and language (`script::texts`). Screens get the same; `Screens:setLanguage`, `language`, `languageName` and the caption settings.
- **Tables' translated columns** (tables version 2): a cell's key `Items.IronSword.description`, the parts kept through renames (`KeptKeys`); rows read in the player's language (`RowRef::translated` in Rust).
- **The player's language**: theirs, else Windows' (`sys-locale`) when the project has it, else the main one; in their settings file with subtitles, captions, caption size and background; a change shows from the next tick.
- **The editor**: Project settings › Languages (rows with how much is translated, + Language, Make main, Remove, the spreadsheet, the player's caption card); the Translations tab on the tables' grid (`Sheet::Lines`: keys can't change, missing lines marked, Missing only, + Key, typing writes the file, undo writes it back); a sound event's Caption and Speaker; the check in Problems.
- **Captions** (sound events version 2): a sound heard as it starts with a caption shows on Solis's own caption screen (`ui::captions`) while captions are on, a voice line's (with a speaker) while subtitles are on, two to eight seconds, the newest three, as big and dark as the player chose; `Workbench::show_subtitle` and `stop_subtitle` keep a line up for a time or until stopped, for the dialogue.
- **Not built**: right-to-left text; a key picker for a sound's caption (typed); a loop's caption kept while it loops (shown as it starts); renaming a key (keys are made once); `text Text.Shop.Welcome` in a screen (asked of Lust: screens write `text World:text(Text.Shop.Welcome);`).

**Asked of Lust** (ready to send):

> **A text piece showing a line of the game's text by its key.** Solis's games name each line of their text as an option of a state the engine writes (`state Text = [Shop [Welcome, Apples]];`), and `World:text(Text.Shop.Welcome)` gives its words in the player's language. A screen today writes `text World:text(Text.Shop.Welcome);`, since `text` takes only text. Could Lust:
> 1. let an engine mark a state as text, in its host file (for example `state Text as text;` or a host setting), and
> 2. let `text`, `button` and a string's `{…}` take an option of such a state, turning it into words through a native the engine names (`World:text`), checked like any call?
>
> Then a screen writes `text Text.Shop.Welcome;`, a misspelt key is still a mistake before the game runs, and Lust's own windows could show translated words the same way.

## Fixes from making Orc Invasion

The project lead, 2026-11-20 ([decision 0133](../decisions/0133-fixes-from-making-orc-invasion.md)): a session made Orc Invasion, a small top-down action RPG, with Solis and Lust, and wrote down every hurdle ([the report](../reports/orc-invasion-hurdles.md)). Every one is fixed or asked for. **They come now, before dialogue**: two bugs break games that pass the project check, and dialogue leans on reading components and finding entities without strings. Each milestone is planned in detail when it's next, as usual, with UI mocked up first and choices asked.

### Fixes 1: bugs, and a project check that starts the game (done)

- **B1** (High) Screens fail with Lua's 200-locals limit when a screen's program holds many methods. Make each file's methods a table's fields (or one function a file) so the count can't overflow, with a test of a file with hundreds of methods.
- **T2** (High) `--check-project` builds the screens' program too, and `--smoke <ticks>` plays the first level briefly and reports any Console error, so B1's kind of failure is a Problem, not a surprise on Play.
- **B2** (Medium) A colour with transparency draws on screens (or, if it can't yet, is a Problem, never silently dropped).
- **B3** (Low) Spawned copies are named after the prefab's root, as placing one in the editor does.
- **B4** (Low) Opening a project opens its last level, or its only level, never an empty untitled one.
- **B5** (Low) No warnings while the project's names are still loading: game code is checked once they've been read.
- **T4** (Medium) The new-project template keeps small pictures and sounds in Git, not LFS, and the new-project page says how to turn LFS on.
- **T5** (Low) The Linux packages the build needs, in CONTRIBUTING.md.

**As built** (branch `fixes/1-bugs-and-check`; [decision 0134](../decisions/0134-fixes-1-choices.md), choices taken as recommended):
- **B1:** the overflow was in the Lua Lust writes (every top-level name of a program a local of one function), not Solis's, and game code had it too (it failed at the check, with a less clear message). `script::many_names::fit` rewrites Lust's chunk line by line, as `exact_maths` and `states` already do: past the first 120 names, the rest become the program's own globals (its env), a word Lust's runtime uses staying a local; smaller programs are byte for byte as before. Tested at 300 methods (fetched by a screen, by game code, and all called from one method, for upvalues), 300 states, classes, objects and events, a component of 300 fields and a state of 1,200 options; nothing else Lust writes grows with the program (a class's methods and fields are a table's fields).
- **T2:** `script::code::compile` starts the screens as Play does, so a screen that can't start is a mistake in its file in the check and in the editor's Problems. `--check-project <folder> --smoke <ticks>` (`app::check_smoke`, 1 to 36,000 ticks) plays the start level (`app::first_level::start_level`: the last export's, else the first by name) headless and fails on each different Console error or warning, with its first tick and how often.
- **B2:** `ui::view::colour` reads `#rgba` and `#rrggbbaa`, keeping how see-through as the top byte (so a solid colour is unchanged), and every colour a screen draws (background, text, border, bars, shadows, text effects' colours) is drawn with it (`view::alpha`). Nowhere was left that can't draw it, so nothing goes to Problems. A picture test draws `#1a1c2ccc` over the world.
- **B3:** the report's premise was half right: placing in the editor named a copy after the prefab's file too. Both now take the root part's name (`Workbench::copy_name`), or the prefab's when the root has none. The world's checksum includes names, so the two-PC game was recorded again (`record_the_game`, a new seed) and its list written again.
- **B4:** nothing remembered the last level, and no project setting names one. `app::first_level`: the level last opened or saved (`.solis/last level.txt`), else the start level; only when a project is opened as it is (the launcher, Open recent, the command line), not for a sample, a new project or an exported game, which name their own.
- **B5:** the eleven warnings were the first check running before the sprite sheets were read. `GameCode::forget` (a project opening) makes the first check wait until the sheets have been read once (`names_read`, from the window's `follow_sprites`).
- **T4:** new projects' `.gitattributes` mark pictures, sounds, models and fonts `binary` and send only `.psd`, `.kra`, `.blend`, `.fbx`, `.exr`, `.hdr`, `.mp4`, `.mov` and `.webm` through LFS (narrowing decision 0006); the Projects page says when LFS is worth it and how to turn it on.
- **T5:** `libudev-dev`, named in the report, isn't needed: gamepads (gilrs) are Windows only. The list is `build-essential` (Luau is C++), `pkg-config` and `libasound2-dev`.
- **Asked of Lust** (ready to send): "A program's top-level names are all locals of one function in the Lua Lust writes, and Luau allows 200: a program of a few hundred methods, states and classes can't start. Could each file's names be fields of a table, or the ones past a count be written as globals of the program's environment? Solis rewrites the Lua to do the second until then (`script::many_names`)."

### Fixes 2: the missing game-code calls (done)

All in Lust, Rust and Lua, checked against the guidelines (rule 25).
- **M5** (High) Game code reads another entity's component, read-only, as the last tick left it, as screens already do (`entity:knight()`), so a gate can ask whether the knight carries its key.
- **M6** (Medium) A one-off entity is reached without a string: a level's single component of a kind (`World:the(Quest)`) and/or a checked name for named entities. `World:find("…")`'s string use is then retired from the manual.
- **M7** (Medium) Spawning with values: `World:spawn(prefab, place, values)`, or a promise that an event sent to a just-spawned entity reaches it on its first tick.
- **M8** (Medium) `entity:exists()`, and a manual section on the order of updates, events and despawns in a tick.
- **M3** (Medium) A collider turned off and on from game code (`setSolid(false)`), for bodies that fall.
- **M4** (Low) `setVisible(bool)` on an entity.

### Fixes 3: clips by action and direction, and another look (done)

Planned and built in [fixes-3.md](fixes-3.md) (its As built notes), with the project lead's answers (2026-11-23). The project lead chose a sheet cut in the editor over reading direction names from Aseprite tags. As built, game code sets a component's fields rather than calling `setFacing` and `setLook` ([decision 0136](../decisions/0136-engine-calls-make-or-set.md)): `facing = Facing.NorthEast;`, `look = Look.KnightMana;`.
- **M1** (High) A picture cut into cells in the editor (by a grid, or freely, Atlas Crop's way), clips with one, four or eight directions made from its cells, and game code naming the direction (`setFacing(Facing.NorthEast)`, then `animate(Clip.Sprites.Knight.Walk)`). Ends 300-line lookup tables.
- **M2** (Medium) Switching a character to another look while playing: a look is another picture with the same cells (`setLook(Look.Sprites.Knight.Mana)`).
- **D2** (Low) The rule for how a tag becomes a clip's name, in the sprite animation page.

### Fixes 4: Solis's files written down, and playing from the command line (done)

- **D1** (High) A manual page, "Solis's files": one short annotated example of each (Aseprite features read, sidecars and their hash, a level with a placed prefab copy and its changes, prefabs, tilesets, tile maps, how a state field is stored, the 2D components' fields), for anyone writing a game's own tools.
- **D3** (Medium) Files can point at others by path, turned into an ID the first time Solis reads them, and/or `solis --new-id`.
- **D4** (Low) A table of every key and button name in the input page.
- **D5** (Low) A picture in the screens page of how `right` on a column stretches it, and, if Lust agrees, a separate word for "push me to the right" (asked of Lust).
- **T1** (High) Playing a game without a window from the command line: `solis --play <project> <level> --ticks 600 --inputs script.txt --print Quest,Knight`, printing the Console and chosen components, for game makers and a game's own CI.
- **T3** (Low) A plain-text input script that a check recording can be made from, from the command line.

### Asked of Lust

Lust's own rough edges go on the requests for Lust (ready to send, with the others in the As built notes):
- **L1** (Medium) Arrays in components: already asked (field hints).
- **L2** (Low) Common names such as `lock` and `has` allowed where a keyword can't appear (parameter and variable names), or Lust's words listed in one place.
- **L3** (Medium) Names of one or two capitals (`N`, `NE`) allowed, at least as state options, for compass directions.
- **L4** (Low) A result that isn't used is a warning, not a mistake, or `_ = World:spawn(…)` says it isn't needed.
- **D5's word** "push me to the right" (`alignSelf right` or the like) for screens.

