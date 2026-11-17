# 01 · Finishing "Usable first": one project file, the rest as Lust (L2 to L6) and the gate

Research for the development plan. Read at `origin/main` e763513 (R1 merged, 2026-11-17). Scope assumes the editor rethink (R1 to R6) is built, so "area" is "level" and assemblies have become (or sit beside) prefabs.

## Where this sits

ROADMAP.md "How the rest is built", step 2: "Finish 'Usable first' (one project file, the rest as Lust) and its gate." Decision 0070 made "one project file" step 2 of Usable first; decision 0081 set the rule (every file Solis makes, except `project.solis`, becomes `<name>.<kind>.lst`, read with Lust's own reader) and the order (base, maps, tilesets; then the 2D camera and UI; then the rest). `docs/plans/lust-data.md` lists what is left: L2 to L6.

**Order note (worth raising):** ROADMAP's step 1 puts Tools 1b *and* the Open worlds phase before this step. Open worlds is a large, purely 3D phase (five milestones, a recipe system, streaming, LOD rings, floating origin). Decision 0097's spirit was "2D first"; decision 0108 brought Tools 1 forward, and decision 0116/0117 then attached Open worlds right behind it. The result is that the project lead's farming-game gate, and all of the remaining 2D, now wait behind about eleven 3D milestones. See `00-overview.md`, question Q1.

## What it is, in plain words

Every file the editor writes for you becomes a Lust data file you can read, diff in Git and (carefully) edit by hand: animators, clips files, sound events, the mixer, 2D settings, input actions, levels, prefabs, sidecars, save games and recordings. The project has one special file (`project.solis`); everything else is plain Lust data.

What it gives the game maker: one language for everything they open; files that merge in Git the same way; and the guarantee that the Lust tools (Ledger, `lust check`) read and colour them.

## What exists

- **The base** (L1, built): `reflect::data_files` (`KINDS = ["atlas", "tilemap", "tileset"]`, `kind_of`, `stem`, `path_stem`), `area::lust_data` (blocks, `fits`: the two reader limits of 12 million values and 1,048,624 things), classes in `solis/data/`, and moving older files when a project opens (with a backup, `asset_db::backup::save`).
- **Formats with versions and upgraders** (`reflect::upgrade::Format<T>`): `animation::frames_file::FORMAT`, `animation::animator_file::FORMAT`, `sound::event::FORMAT`, `sound::mixer::FORMAT`, `area::pixels::FORMAT`, `input::BINDINGS_FORMAT`, `input::RECORDING_FORMAT`, `area::text::FORMAT`, `asset_db::SIDECAR_FORMAT`, the editor's `save_game::FORMAT`. Fixtures per version in `tests/fixtures/<format>/v<N>/`, checked by `crates/app/src/fixtures_tests.rs`.
- **The reader's cost** (from `crates/tiles/README.md`): Lust's own data reader needs about 100 bytes and half a microsecond per value. That is fine for small files and a real risk for big ones (levels with 10,000 entities, save games, recordings).

## The steps and how to build each

| Step | Files | Main risk | Size |
| --- | --- | --- | --- |
| L2 | `.animator.lst`, `.clips.lst` | none: small files, stable IDs per block already | small |
| L3 | `.sound.lst`, `.mixer.lst`, `.pixels.lst`, `.input.lst` | input bindings' key names (format 2 changed `ArrowUp`); keep the same names | small |
| L4 | `.level.lst` (was `.solarea`), `.prefab.lst` (was `.solassembly`) | speed (10,000 entities), merge-friendliness, and R2/R5 already moved these files once | medium to large |
| L5 | `<file>.meta.lst` sidecars | thousands of tiny files read at project open | small to medium |
| L6 | `.save.lst`, `.recording.lst` | size and speed of the Lust reader; saves must never be refused for being older | medium |

### Approaches compared for the big files (L4, L6)

The question is how a large file goes through Lust's reader without blowing the budget.

1. **Everything through Lust's general data reader** (what L1 does). Simplest, one reader. Cost: about 0.5 µs and 100 B per value. A 10,000-entity level with about 20 values each is 200,000 values: about 0.1 s and 20 MB, acceptable. A save game with 100,000 projectiles at 8 values each (800,000 values) is about 0.4 s and 80 MB, borderline; a 10-minute recording at 60 ticks with per-tick input frames could reach millions of values.
2. **Lust's shape, Solis's own fast reader** for the heaviest kinds. The file is still valid Lust (Ledger reads it, `lust check` would accept it), but Solis reads it with a hand-written streaming parser for that one shape, checked against Lust's reader in a test (the same file must give the same values both ways, and a fuzz target runs both). Fast (tens of MB/s), but two readers to keep in step.
3. **A Lust header plus a packed body** for bulk data (recordings' input frames, projectiles in saves): the header is Lust data, the body a base64 or run-length block inside a Lust string. Small and fast, but not readable or mergeable, which saves and recordings don't need anyway.

**Recommendation:** approach 1 for L2, L3 and L5; approach 1 for levels and prefabs (L4) with the scale check proving 10,000 entities open within budget (the plan already asks for it), and approach 2 held in reserve if it doesn't; for L6, approach 3 for the bulk parts (projectiles, input frames) and approach 1 for the rest, because saves and recordings are never merged in Git and must be fast. Decision 0081 says "every file", so approach 3 needs the project lead's yes (question below).

### Ordering against the rethink

R2 renames `.solarea` and the area format, and R5 adds a prefab format, each with a version raise and upgrader. L4 then moves both again. Two moves of the same files in a row means two upgraders and two sets of fixtures. Two options:

- **Do L4 inside R2 and R5** (the level and prefab formats are born as Lust). One move, but R2 and R5 grow, and the rethink is already being built.
- **Keep L4 separate** and accept two upgraders (cheap: each is one `steps` entry).

Recommendation: if R2/R5 are not yet merged when this is read, ask the builder of R2/R5 to write the new formats as Lust data directly; otherwise keep L4 separate. Either way the area/level reader must keep reading every older version (rule 23).

## How to implement in Solis

- **Per step** (as `lust-data.md` says): a Lust shape and classes in `solis/data/` (e.g. `solis/data/animator.lst` declaring the blocks), the reader (older layout still read through the format's `steps`), the writer, a fixture of the new version, the kind added to `reflect::data_files::KINDS` and `app::to_lust`, the fuzz target updated to read the new layout (rule 9: `fuzz/fuzz_targets/animator.rs`, `frames.rs`, etc. already exist), the manual and the changelog with a `### Breaking` line (rule 23).
- **Writing within the reader's limits:** follow `tiles::map_file::write`'s rule: the writer asks `area::lust_data::fits` before writing, and gives the data back unwritten with the reader's reason if it wouldn't read back.
- **Sidecars (L5):** reading thousands of `.meta.lst` at project open should go on the asset database's worker (`asset_db::jobs`), never the frame (rule 10). Measure "open a project with 5,000 assets" before and after.
- **Game code:** no new calls. The generated files in `solis/` don't change.
- **Editor UI:** none new. Files keep opening in their existing tabs.
- **Keys:** none.

## Tests and the scale check

- Fixtures per kind, round trips (write, read, write gives identical bytes), the older layout read and upgraded, a backup made before the first save, fuzz targets per kind.
- Scale cases: a 10,000-entity level opens within the scale check's budget (the plan names it); a save with 100,000 projectiles and a 10-minute recording read and written within budget; a project of 5,000 assets opens without a frame over target.
- A test that every file kind the editor writes is in `KINDS` (so nothing is left in the old format by accident).

## The gate

"The project lead starts their farming game in Solis, in Lust: walk the farm, till, plant and water, crops grow over days, a menu and an inventory, saving, and a build a friend can play." Everything needed is built except the file moves; the gate is the project lead's own time. Two things to check before it: the farming game needs **triggers** (doors, shop counters) and **tags/world questions** ("nearest crop"), which are in 2D 2 and Shared 2. The gate is reachable without them (the API's `cellAt` and tile values carry farming), but the lead will likely ask. Recommend saying so up front.

## Risks and questions for the project lead

1. **Saves and recordings in Lust:** "Every file but `project.solis` becomes Lust" (0081). For saves and recordings, which are big, never merged and never read by people: **recommended**, a Lust header with packed bulk sections (fast, small); alternatives: all plain Lust (slow and large past ~100,000 values), or keep the current text format for these two.
2. **When L4 happens relative to R2/R5** (above): **recommended**, born as Lust in R2/R5 if not yet merged; otherwise a separate step.
3. **Finish "one project file" before or after Tools 1b and Open worlds?** The ROADMAP order says after. **Recommended:** L2 and L3 are small and can slot in any time (each a short PR); do them before Tools 1b so the new ground's text settings file is born as Lust too (decision 0116 asks for "a text file per ground"). Alternative: keep the order.

## Size

L2 small, L3 small, L4 medium (large if R2/R5 already moved them), L5 small to medium, L6 medium. The whole step: medium.
