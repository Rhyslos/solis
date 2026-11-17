# 02 · Shared 2: game code, finding and making things

ROADMAP.md: "The fixes the playtest games asked for (steps 2 to 5), and idea 34." Five milestones: tile map questions; tags and questions about the world (ideas 8 and 9); spawning with values; game tests (idea 29); a note for code files nothing fetches (idea 34). **Gate:** "a game finds the nearest enemy by tag, spawns a bullet already moving, and has a test that passes."

Everything here is game-code interface, so every call must pass the checklist in `docs/game-api-guidelines.md` (rule 25), be listed in `docs/game-code-interface.txt` (rule 24), and be shown in Lust, Rust and Lua in the manual (decision 0113).

## What exists that this builds on

- **Generated per-map classes** (`crates/script/src/map_code.rs`, decision 0079): each tile map gets `solis/maps/<path>.lst` with states of its layers, terrains and choices and a class (`FarmMap`) whose methods call the `Maps` object by path and numbers: `width`, `height`, `terrain`, `setTerrain`, a reader and setter per value, `cellAt`, `placeOf` (decision 0082), plus `entity:mapBounds()`. Reads see the map as the tick started; writes land at the tick's end (`Change::MapTile`, `Change::MapValue`).
- **The `tiles` crate** already has `cells_where` ("every watered crop") and `counts` (cells per code, kept up to date), neither reachable from game code.
- **`World:find(name)`** (an index of names to first entity, `world::find`), **`World:spawn(Assembly, place)`** returning an entity ID made from the tick's order (decision 0035), landed as `Change::Spawn` at the end of the tick.
- **`WorldView`** (`script::game`): the read-only snapshot each phase of a tick reads (places, names, time, input, physics, maps). Decision 0033: every shard reads the same start-of-tick world, so anything built once at the tick's start can be shared read-only by all shards.
- **Lust's native rules** (LUST.md §21): natives take and return ints, floats, text, bools, `vec3`, `rotation`, states' options, class things and **arrays of those** (`entity array`, `MapCell array`). Component fields and event payloads hold only `bool`, `int`, `float`, `string`, a state, `vec3`, `rotation`, `entity` (plus `vec2` and `colour` since decision 0106): **no arrays, no class things** (except engine-made parts like `FarmerAnimator anim;`).
- **Testing machinery:** whole-game invariant tests (`crates/editor/src/invariant_tests.rs`), recordings (`input::RECORDING_FORMAT`), checksums, `--check-project`, and `lust test` (Lust's own: runs a program and compares its output with a `.out` file, `LUST_SEED=1`).
- **Which files the program reaches:** `script::reach` follows `area::lust_guard::fetched` from `main.lst`.

## M1. Asking a tile map questions

**What it gives:** "which cells are tilled?", "how many crops are watered?", "what's next to this cell?", "is this cell on the map?". Half of the roadmap line is built already (`terrain` answers "what's at a cell"; `cellAt` and `placeOf` turn places into cells and back). What's missing is the "many cells" questions.

**Ways to build it:**
1. **Expose `cells_where` and `counts` as generated methods** returning `MapCell array` (sorted by row then column, so every shard gets the same order). Cheap: `counts` is already kept as cells change, so `count(FarmTerrain.Tilled)` costs nothing; `cells_where` walks the map, so cap the result (say 65,536 cells) and say so loudly when capped (rule 11 of the guidelines).
2. **An iterator protocol** (game code loops cells one at a time through a native cursor): no big arrays, but a cursor must live across native calls, which doesn't fit the stateless natives and the shards.
3. **Per-chunk summaries** (each chunk keeps counts per code and per value): makes "is any cell here watered?" O(chunks) instead of O(cells). Useful later for very large maps; not needed at 1,000 × 1,000 (a full scan is about 1 M bytes, well under a millisecond).

**Recommendation:** option 1, plus `contains(x, y)` and `neighbours(x, y)` (4 or 8, as a `MapCell array`, cells off the map left out). Names by the guidelines: `cellsOf(FarmTerrain.Tilled)` and `cellsWhere…` per value (`wateredCells()`, `cellsWithCrop(FarmCrop.Wheat)`, `cellsWithGrowthAbove(n)`?). The per-value variants multiply quickly; a cleaner shape is one method per value kind: `cellsWhereWatered(true)`, `cellsWhereCrop(FarmCrop.Wheat)`, `cellsWhereGrowth(atLeast, atMost)`. Ask the lead (question 1).

**How:** `map_code.rs` gains the methods (and adds their names to `OWN`, so no value can clash); `natives/maps.rs` gains `Maps:cellsOf`, `Maps:cellsWhere…`, `Maps:count`, `Maps:neighbours`, reading `WorldView::maps`; `MapCell` already exists as a returned class. The generated class lists stay small.

**Tests:** results sorted and identical on 1 and 4 shards; a change this tick isn't seen until the next (decision 0079); a capped result says so; manual examples in three languages compile (the guide's tests).

**Size:** small.

## M2. Tags and questions about the world

**What it gives:** label entities ("crop", "enemy") with the spelling checked before the game runs, and ask "all crops within 3 m", "the nearest enemy".

**Existing pieces:** none for tags. Project names as states already work (`solis/project.lst`, `project_names.rs`), so `Tag.Crop` is one more state.

### Storing tags

1. **A built-in `Tags` component** holding a small set of the project's tags (stored as the project's stable tag IDs, shown as chips in Properties). Fits "components that describe themselves", saved in levels and prefabs, overridable per prefab copy.
2. **Tags as a field on every entity** (like its name). Simpler UI, but every entity pays for it and it isn't a component like everything else.
3. **Tags made from components** ("everything with a Health is tagged Health"). No new data, but it is a different idea: you can already ask by component.

**Recommendation:** option 1. The project's tag list lives in the project (Project settings › Tags, or a `tags` section of `project.solis` / its own `.tags.lst`), each tag with a stable ID so a rename in the editor keeps every entity's tags (and Lust points at every line still using the old name, as clips do).

### Answering "within" and "nearest" at the scale target

The scale target is 2,000 units (10,000 stress). If every unit asks "nearest enemy" every tick:

| Way | Cost per tick, 2,000 units each asking (estimate) | Notes |
| --- | --- | --- |
| **Brute force** over the tag's members | 2,000 × 2,000 = 4 M distance checks ≈ 4 to 8 ms | Fine for a few askers; far too slow at the target |
| **Uniform grid, rebuilt at the tick's start** (counting sort of members into cells) | build ≈ 0.05 to 0.1 ms; each query visits a few cells: ≈ 0.1 to 0.3 ms total | Simple, deterministic, cache-friendly; cell size must suit the queries |
| **k-d tree or R*-tree rebuilt each tick** (`kiddo`, `rstar`) | build ≈ 0.3 to 1 ms for 10,000; queries fast at any radius | Handles very uneven densities; a library (licence and exit plan, rule 5) |
| **A BVH kept and refitted** (as physics broad phases do) | refit cheap, rebuild occasionally | Most code; wins only for very large radii or ray queries |

Spatial hashing is the standard answer for "fixed-radius near neighbours" among many moving things (see the gamedev.net thread and the spatial-hashing article in the sources); trees win when density is very uneven or radii vary by orders of magnitude.

**Recommendation:** a **uniform grid over the ground plane (X, Y), one per tick, built once at the tick's start** and shared read-only by every shard (decision 0033 makes this free of locking: the snapshot is the same for all). Each cell holds `(entity ID, place, tag bits)` sorted by ID; a tag's members are found by a bitmask test (up to 64 tags per project in a `u64`, or a small bitset). Cell size: a project setting with a sensible default (8 m, or 8 tiles in 2D), plus a fallback to brute force when a radius covers more than about 64 cells. Results: **nearest first, ties by entity ID**, so they are identical on any number of cores. Write the grid ourselves (about 300 lines); `kiddo` or `rstar` (both permissive, to be confirmed in `deny.toml`) stay the exit plan if uneven densities show up in the scale check.

**2D and 3D:** a 2D game lies on the ground (decision 0062), so its X and Y are the plane and Z is the layer. The grid is in X and Y for both; distance is 3D (a flying unit 50 m up isn't "within 3 m"). A project setting could make it 2D distance, but recommend not until a game asks.

### Game code (checked against the guidelines)

| Call | Shape |
| --- | --- |
| `is hasTag(Tag.Crop)?` / `target:hasTag(...)` | question after `is` |
| `addTag(Tag.Ripe)`, `removeTag(Tag.Ripe)` | verbs; land at the tick's end like other changes |
| `World:tagged(Tag.Crop)` → `entity array` | noun; in ID order |
| `World:within(Tag.Crop, place, 3.0)` → `entity array` | nearest first |
| `World:nearest(Tag.Enemy, place)` → `entity` or `nothing` | noun |
| `World:nearestWithin(Tag.Enemy, place, 20.0)` | variant at the end (rule 7) |

The ideas list sketched a query chain (`World:tagged(Tag.Crop):where(distance < 3):orderBy(distance):first(1)`), borrowing Lust's database calls. That needs Lust's query words to work on what a native returns, which LUST.md lists as still to design (LustQL). **Recommend the plain calls now**; the chain can come later as sugar over the same grid.

**Editor UI (mock-up first):** a Tags row in Properties (chips, a "+" picker of the project's tags, typing a new one adds it to the project), a Tags page in Project settings (rename, delete with a count of users, colour), and a tag filter in the Level panel's search. No new keys.

**Tests:** tag rename keeps entities' tags and the generated state; queries identical on 1 and 4 shards; a tag added this tick is seen next tick; the grid matches brute force on random worlds (property test); a radius larger than the world falls back correctly; an empty tag gives an empty array. **Scale cases:** 2,000 units each asking `nearest` every tick (target) and 10,000 (stress), timed apart from the rest of the tick.

**Size:** medium.

## M3. Spawning with values

**What it gives:** "spawn a bullet already moving": a spawned entity's fields set as it's made, so its first tick already has them.

**What exists:** `Change::Spawn` lands at the tick's end with an ID decided at once. A placed prefab copy already carries **overrides** (a reference plus changed fields, decision 0020; R5 brings changed settings shown and resettable). Spawning with values is the same thing at run time: a spawn plus overrides.

**Ways to build it:**
1. **A generated class per prefab** (guideline 4: "anything made in an editor reaches code through a file the editor writes"): `solis/prefabs/bullet.lst` holds `class BulletSpawn` with one `set…` method per field a copy may change (from R5's list) and `spawn(place)`:
   ```
   entity shot = BulletSpawn:new():setSpeed(12):setOwner(this):spawn(muzzle);
   ```
   Every name is checked by Lust; renaming a field in the prefab points at every line. Works with today's Lust (natives take class things and arrays). Cost: one generated file per prefab (already the pattern for animators and maps).
2. **A component literal as an argument** (`World:spawnWith(Prefab.Bullet, place, Bullet { speed = 12 })`): reads best, but Lust can't pass a component's values to a native today, so it needs a Lust change, and a prefab with several components needs several literals.
3. **Spawn, then send a set-up event** (`send Launch(12) to shot;`): works today, but every component must handle a set-up event by hand, and the first tick runs before it arrives unless the event is delivered specially.

**Recommendation:** option 1, mirroring decisions 0078 and 0079. Each `set…` records `(component, field, value)` in the Lust object; `spawn(place)` calls one native (`Prefabs:spawn(path, place, names, values)`, arrays) that returns the ID and keeps `Change::Spawn` with the overrides. Values are checked against the prefab's schema when landed; a wrong kind is a loud mistake naming the line (guideline 11). Rotation and velocity deserve first-class setters (`setFacing`, `setVelocity`) because "already moving" is the gate's example and velocity is physics state, not a component field: the spawn must hand the starting velocity to physics as the body is made, or the first tick will be at rest.

**Where it must also work:** Lua (`solis.Prefabs.Bullet:new():setSpeed(12):spawn(p)`, same words) and Rust (`BulletSpawn::new().speed(12).spawn(ctx, p)` or the same method names in snake case). Pools (Shared 6) reuse the same overrides path, so build it so a pool's "take" can apply them too.

**Tests:** the spawned entity's first `update` sees the values; physics velocity applied before the first step; on 1 and 4 shards the same IDs and values; a save made the tick after holds them; overrides for a field the prefab no longer has are a mistake at check time (the generated file no longer has the setter). **Scale case:** 1,000 spawns with values in one tick.

**Size:** medium (it touches script, world, physics hand-over and the generated files).

## M4. Game tests

**What it gives:** small automatic checks the game maker writes ("a crop grows in three days", "the shop sells for the table's price"), run from the editor or the command line, the way Solis tests itself.

**What other engines do:** tests are code files run by a test runner, with a "scene" loaded headless, time advanced, and assertions; or playback tests (record input, compare result). Lust's own `lust test` compares printed output with a saved `.out` file.

**Ways to build it:**
1. **Assertion tests in Lust** (recommended): a test file (`crops.test.lst`, a new data-file-like kind, or a `test` folder) holding tests that open a level headless, advance time, act through the same calls game code uses, and check with `expect`:
   ```
   test "a crop grows in three days" {
       Game:open(Level.Farm);
       entity crop = World:spawn(Prefab.Crops.Wheat, [4, 4, 0]);
       Game:advanceDays(3);              -- or Game:advance(seconds)
       expect(crop:hasTag(Tag.Ripe), "the wheat should be ripe after three days");
   }
   ```
   Needs a Lust word for tests (`test "name" { }`) or a convention (`method test…()` on a `tests` object); `expect` can be a native.
2. **Golden output** (like `lust test`): run the level, compare what `World:say` printed with a `.out` file. No new words; brittle and unhelpful messages.
3. **Recording tests:** a recording plus the checksum or chosen values it must end with. Free to build (recordings and checksums exist), excellent for regressions, but they don't express intent.

**Recommendation:** option 1 for the gate, with option 3 offered as "Turn this recording into a test" (it's nearly free and catches regressions). Runs headless with no GPU and no window (the `world` and `script` crates already tick without `render`), at full speed (no frame pacing: thousands of ticks a second), with the game's fixed seed. Each test gets a fresh world. Results in a **Tests** panel (bottom panel tab after R3) and as `solis --test-project <folder>` for CI, with exit codes, so game makers can put it in their own pipelines.

**Lust work:** a `test` block or convention, and whether `expect` stops the test or records and carries on. This is a Lust change; ask the Lust side early.

**Time:** `Game:advance(seconds)` runs the fixed tick that many times; `advanceDays(n)` uses the Environment's day length (decision 0092). Timers (Shared 3) will make "three days" exact; until then the farming gate uses the day clock.

**Tests of the feature:** a passing and a failing test, an infinite loop hits the time limit and fails cleanly, a test can't reach files (sandbox, as game code), results identical on 1 and 4 shards. **Scale case:** 100 tests of 1,000 ticks each finish in a few seconds.

**Size:** medium (plus a Lust change).

## M5. A note for code files nothing fetches (idea 34)

**What it gives:** a soft hint in Problems: "`enemies/dragon.lst` isn't fetched from `main.lst`, so its components aren't in the game", with a quick fix and a way to silence it.

**How:** `script::reach` already computes what `main.lst` reaches. List the project's `.lst` files that are game code (not data files, `reflect::data_files::kind_of` is none; not in the engine folder, `describe::in_engine_folder`) and not reached. Problems gets a hint-level entry (decision 0101's Problems kinds). **Quick fix:** add `fetch "enemies/dragon":Dragon;` to `main.lst`, listing what the file `pass`es (Lust's checker knows). **Silence:** per file or folder, kept in project settings (a "parked" list), not in a comment, so it survives rewriting the file.

**Speed:** the ideas list asks for a scale case: checking thousands of code files on every save. Reach is computed per check already; the extra work is a set difference. Scale case: 10,000 `.lst` files (the gather limit) within the check's budget.

**Size:** small.

## Dependencies and order

- M2 (tags) before M3's pools reuse and before 2D 1's pathfinding demo ("character finds its way" can use `World:nearest`), and before Shared 6's cheat box (`give seeds 10` finds by tag).
- M3 depends on R5 (prefabs, the per-copy overrides list). If R5 renames `state Assembly` to `state Prefab`, M3's generated file names follow.
- M4 benefits from Shared 3's timers but doesn't need them.
- M1 and M5 are independent.

**Suggested order:** M5 (small, warms up), M1, M2, M3, M4.

## Risks and questions for the project lead

1. **Shape of "cells where a value is…":** one method per value (`wateredCells()`), or one per value kind with a parameter (`cellsWhereWatered(true)`)? **Recommended:** `cellsWhere<Value>(…)` per value, matching the existing `set<Value>`. Alternative: a generic `cellsWhere(FarmValue.Watered, …)` taking a state of the map's values (fewer methods, a little less readable).
2. **Where the tag list lives:** **recommended** in Project settings, saved in its own `tags.lst` data file; alternative: in `project.solis`.
3. **Distance for world questions:** **recommended** 3D distance with a ground-plane grid; alternative: a project setting for flat (X, Y) distance in 2D games.
4. **Spawning with values:** **recommended** the generated per-prefab class with setters (`BulletSpawn:new():setSpeed(12):spawn(p)`); alternatives: a component literal (needs a Lust change), or spawn plus an event.
5. **Game tests' shape:** **recommended** `test "…" { }` blocks in `*.test.lst` files with `expect`; alternatives: golden output like `lust test`; recordings only. Also: may a test press actions (`Test:press(Action.Use)`)? Recommended yes, through the same input frames recordings use.
6. **The query chain** from the ideas list (`:where(...):orderBy(...)`) waits for Lust's query design. Recommended: plain calls now.

## Crates

None needed. `kiddo` (k-d tree) and `rstar` (R*-tree), both MIT or Apache-2.0 per their pages (to confirm in `deny.toml`), are the exit plan for the world-question index if the scale check shows uneven densities; the `pathfinding` crate is irrelevant here.

## Size

Phase: medium (five milestones: small, medium, medium, medium, small).

## Sources

- Fixed-radius near-neighbour search and spatial hashing: [gamedev.net thread](https://gamedev.net/forums/topic/649674-fixed-radius-near-neighbor-search/), [spatial hashing for collision](https://saeed1262.github.io/blog/2025/spatial-hashing-collision), [sparse spatial hash grids](https://dev.to/queelius/sparse-spatial-hash-grids-efficient-n-dimensional-spatial-indexing-438j), [gamedev.net: alternatives to octrees](https://gamedev.net/forums/topic/718178-alternatives-to-octrees/5469585/).
- Crates: [kiddo](https://docs.rs/crate/kiddo/5.2.3/source/README.md), [rstar](https://docs.rs/crate/rstar/latest).
- Lust's native and component rules: `/home/claude/lust/LUST.md` §21 (local copy, alpha 61; Solis pins alpha 67).
