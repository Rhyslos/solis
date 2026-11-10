# Full review of the engine, November 2026: what was found

The project lead asked for a review of the whole codebase (10 November 2026): weak code, instability, poor modularity, debt worked around, insecurity, spaghetti code, dependency trouble, flaky tests and outdated documentation. Fourteen reviewers each read one part of the code at commit `50f007a47` (about 170,000 lines of Rust, the shaders, CI, tools and every document), and the whole test suite was run three times to look for tests that fail by chance (all 1,849 passed each time).

They reported 129 findings; with duplicates merged, **107 distinct problems**, listed below. Each is fixed by one agent working on that problem alone, and each fix is reviewed. What was fixed, and a second review of the result, are in [`2026-11-full-review-second-pass.md`](2026-11-full-review-second-pass.md).

**Severity** runs from 1 to 10: 10 is lost work, a crash in normal use or something a hostile file can exploit; 5 is a real bug in an edge case or a serious cost to whoever maintains the code; 2 is small.

## In short

- **The code is careful where it was told to be.** File readers have limits, the game-code sandbox holds, saves are atomic, and no test failed by chance in three runs.
- **The most serious finds:** exported games play no sound; two crafted files can exhaust memory on open (an assembly, a model); a light with a bad cone angle crashes the editor; Keep after Stop writes what the game spawned into the area; Delete over the UI canvas also deletes the area's selection.
- **A pattern:** the editor's central object (`Workbench`) wires each file editor by hand, and most editor bugs are a place where one editor was forgotten.
- **Frame-loop work:** about a dozen places read files or walk everything every frame.
- **Documents:** the manual's detail is right, but narrative written before the editor overhaul and the launcher is stale, and several READMEs lag the 2D work.

## Insecurity (15)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| SEC-01 | 8 | An assembly's "missing part" overrides aren't counted, so a small project can expand to 100 million entities | `crates/area/src/assembly.rs:176,326` | world-sim F1 |
| SEC-02 | 8 | A small .glb can make the importer copy its binary part once per buffer (100 GB asked for) | `crates/assets/src/reader.rs:310` | assets-media F1 |
| SEC-08 | 6 | A screen's styles are copied out of game code with no size cap | `crates/script/src/screens.rs:661` | script F3 |
| SEC-03 | 6 | "Open code" on a Lua component opens the .lua file in the system's program | `crates/editor/src/commands.rs:438, crates/app/src/window.rs:2377` | security F1 |
| SEC-06 | 6 | A copy that wasn't installed still downloads and silently runs the installer | `crates/app/src/updates.rs:165` | app-and-tools F2, security F5 |
| SEC-09 | 5 | A tile map in the Lust layout has no block limit and is grouped quadratically | `crates/tiles/src/map_lust.rs:463` | assets-media F3 |
| SEC-14 | 5 | The Windows build decodes BMP pictures that nothing tests or fuzzes | `crates/assets/src/texture.rs:41, tile_art.rs:59` | dependencies F1 |
| SEC-04 | 5 | A project's own file chooses the export folder, and the dialog probes it on opening (network share) | `crates/app/src/playtest.rs:266, crates/editor/src/export_dialog.rs:250` | security F2 |
| SEC-07 | 5 | getfenv and setfenv are left in the Lua sandbox | `crates/script/src/luau.rs:257` | script F4 |
| SEC-10 | 4 | A sidecar using the biggest part ID makes giving out new IDs quadratic | `crates/asset_db/src/sidecar.rs:251` | assets-media F4 |
| SEC-11 | 4 | A save's lines are split into unbounded word lists before their length is checked | `crates/editor/src/save_game.rs:565,645` | editor-play F4 |
| SEC-05 | 4 | Choosing a placeable file in the search opens it in the system's program | `crates/editor/src/search.rs:623` | security F4 |
| SEC-13 | 4 | Nine copy-pasted save functions, one (sound events) missing its path check | `crates/app/src/project.rs:285` | app-and-tools F9, security F7 |
| SEC-12 | 3 | A collider's centre can reach the physics library unbounded or infinite | `crates/physics/src/lib.rs:616` | world-sim F8 |
| SEC-15 | 3 | The "ordinary file, bounded read" guard exists four times, and sound files have none | `crates/sound/src/read.rs:188, crates/assets/src/reader.rs:168` | assets-media F9 |

## Instability and weak code (52)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| BUG-01 | 8 | Exported games never get a sound device: they play no sound | `crates/app/src/window.rs:2017, window_playtest.rs:184` | app-and-tools F1 |
| BUG-02 | 8 | A light with a negative or NaN cone crashes the editor (clamp panics) | `crates/editor/src/area_view.rs:370, commands.rs:1190` | editor-chrome F1, editor-workbench F2 |
| BUG-03 | 8 | Keep after Stop also keeps what the game spawned, despawned or loaded | `crates/editor/src/play.rs:422,1018` | editor-play F1 |
| BUG-06 | 7 | Delete and Ctrl+D over the UI canvas or Atlas tab act on the area's selection | `crates/editor/src/workbench.rs:1924` | editor-workbench F1 |
| BUG-12 | 7 | A parent loop in an area file makes world matrices quadratic and overflow | `crates/area/src/tree.rs:106, document.rs:242` | world-sim F2 |
| BUG-17 | 7 | Tileset edits from the Library's Properties skip undo; one click deletes a terrain's art | `crates/editor/src/map_tab.rs:635` | editor-tools F1 |
| BUG-04 | 6 | The pending Keep/Undo question is ignored by quit and survives opening another project | `crates/editor/src/workbench.rs:1800,728` | editor-play F2 |
| BUG-07 | 6 | Opening another project leaves per-project state behind (map undo writes into the new project) | `crates/editor/src/workbench.rs:733` | editor-workbench F3 |
| BUG-13 | 6 | A placed assembly's part moved to the top level is not saved | `crates/area/src/assembly.rs:556` | world-sim F3 |
| BUG-18 | 6 | Dragging a piece on a UI canvas never moves it | `crates/editor/src/canvas_view.rs:723,1122` | editor-tools F2 |
| BUG-25 | 6 | Dragging a number in Input actions saves and re-checks the project every frame | `crates/editor/src/input_window.rs:87` | editor-chrome F2 |
| BUG-46 | 6 | Lua can't reach Window, Screens, Maps or Animators | `crates/script/src/luau.rs:213` | script F1 |
| BUG-47 | 6 | Random numbers and spawned IDs repeat while game speed is 0 | `crates/script/src/game.rs:701` | script F2 |
| BUG-31 | 6 | Pictures and tile maps up to 64 MB are read and decoded on the window thread | `crates/app/src/sprites.rs:161` | app-and-tools F3 |
| BUG-32 | 6 | Rescanning the assets folder hashes whole files and parses every sidecar on the frame loop | `crates/asset_db/src/db.rs:320` | assets-media F2 |
| BUG-05 | 5 | Moving or deleting the open area's file while playing is undone by Stop | `crates/editor/src/play.rs:426` | editor-play F3 |
| BUG-08 | 5 | The clips editor never follows its file on disk, unlike its three siblings | `crates/editor/src/workbench_frames.rs:78` | editor-workbench F4 |
| BUG-09 | 5 | Sheets, maps and clips have two owners, and the app's copy overwrites the editor's | `crates/editor/src/workbench_tiles.rs:216, crates/app/src/window.rs:1138` | editor-workbench F6 |
| BUG-20 | 5 | Dragging a number or colour in the 2D editors makes one undo step and one save per frame | `crates/editor/src/tileset_view.rs:1043` | editor-tools F4 |
| BUG-26 | 5 | Closing the Canvas or Elements tab closes the clips file instead of the screen | `crates/editor/src/panels.rs:657` | editor-chrome F3 |
| BUG-14 | 5 | In a world with no 3D bodies, physics returns before children are settled (one tick behind) | `crates/world/src/lib.rs:1489` | world-sim F4 |
| BUG-53 | 5 | Every change in the assets folder walks the whole folder about five times on the window thread | `crates/app/src/window.rs:2066` | app-and-tools F5 |
| BUG-19 | 5 | Tile caches shared by every view and the Game panel evict each other each frame | `crates/editor/src/tile_view.rs:289` | editor-tools F3 |
| BUG-21 | 5 | The Frames ruler's work has no bound (1.2 million marks a frame) | `crates/editor/src/frames_view.rs:750` | editor-tools F5 |
| BUG-22 | 5 | Tile-map UI scans the whole map every frame | `crates/editor/src/map_tab.rs:296, tile_view.rs:764` | editor-tools F6 |
| BUG-27 | 5 | A Markdown file is re-parsed every frame, and its inline parser is quadratic | `crates/editor/src/text_view.rs:182` | editor-chrome F4 |
| BUG-28 | 5 | Panels walk the whole area every frame | `crates/editor/src/workbench.rs:1257, inspector.rs:168, search.rs:702` | editor-chrome F5 |
| BUG-38 | 5 | Projectiles and unlit models vanish in a 2D picture that has a parallax layer behind it | `crates/render/src/projectiles.rs:78` | render-ui F1 |
| BUG-39 | 5 | Game UI and the pixel stage create GPU buffers and bind groups every frame | `crates/render/src/ui.rs:312, pixel.rs:372` | render-ui F2 |
| BUG-10 | 4 | Caches use a freed pointer's address as their version | `crates/editor/src/workbench_library.rs:427` | editor-workbench F7 |
| BUG-11 | 4 | A screen whose view fails runs the game's screen code again every frame | `crates/editor/src/workbench_screens.rs:145` | editor-workbench F8 |
| BUG-23 | 4 | Scaling with the gizmo can store a NaN rotation | `crates/editor/src/transforming.rs:376` | editor-tools F8 |
| BUG-15 | 4 | 2D physics empties itself silently after a caught panic and is never rebuilt | `crates/physics/src/flat.rs:529` | world-sim F5 |
| BUG-16 | 4 | Edits while playing drop state game code set (shown screens, stopped sounds) | `crates/world/src/lib.rs:598` | world-sim F6 |
| BUG-48 | 4 | Events sent by a component's start are credited to another component | `crates/script/src/runtime.rs:521` | script F6 |
| BUG-49 | 4 | A native's error reaches a Lua author with no line, in the library's words | `crates/script/src/lua_components.rs:70` | script F8 |
| BUG-50 | 4 | A compile that hangs leaves its thread running; a new one starts on every save | `crates/script/src/code.rs:304` | script F9 |
| BUG-33 | 4 | A crash in the game-code worker is silent and leaves an exported game on "Starting" forever | `crates/app/src/game_code.rs:315` | app-and-tools F7 |
| BUG-35 | 4 | Starting a streamed sound opens and scans the whole Ogg file on the frame loop | `crates/sound/src/device.rs:259` | assets-media F5 |
| BUG-40 | 4 | Every viewport target allocates a 16 MiB shadow map, including parallax layer pictures | `crates/render/src/renderer.rs:281` | render-ui F3 |
| BUG-41 | 4 | A full letter atlas wipes itself mid-layout: garbled text and a frame-rate collapse | `crates/ui/src/text.rs:177` | render-ui F4 |
| BUG-42 | 4 | "Matches the shader layout" tests only check Rust against itself | `crates/render/src/frame.rs:312` | render-ui F5 |
| BUG-24 | 3 | Right-click doesn't cancel a box selection in a perspective view | `crates/editor/src/viewport.rs:1218` | editor-tools F10 |
| BUG-29 | 3 | Hard-coded neutral colours left behind by the theme switch | `crates/editor/src/launcher.rs:404, theme.rs:251` | editor-chrome F8 |
| BUG-30 | 3 | The Library's right-click menu shows "Open" twice | `crates/editor/src/library.rs:960` | editor-chrome F9 |
| BUG-51 | 3 | A spawn that can't be made is dropped without a word | `crates/editor/src/play.rs:992` | editor-play F5 |
| BUG-36 | 3 | The import cache hashes a file, then reads it again to import it | `crates/asset_db/src/cache.rs:70` | assets-media F6 |
| BUG-43 | 3 | The letter atlas's revision isn't unique across font sets, and its upload skips the length check | `crates/ui/src/text.rs:57` | render-ui F7 |
| BUG-52 | 2 | Two messages lost their line continuations and show runs of spaces | `crates/editor/src/problems.rs:165, inspector.rs:319` | editor-play note, editor-chrome F10 |
| BUG-37 | 2 | Changing a tile value's kind skips the map's memory limit | `crates/tiles/src/map.rs:655` | assets-media F10 |
| BUG-44 | 2 | Frame statistics count sprites that were not drawn, and warn every frame | `crates/render/src/renderer.rs:1185` | render-ui F9 |
| BUG-45 | 2 | move_piece computes an indent and throws it away | `crates/ui/src/source.rs:981` | render-ui F10 |

## Poor modularity, spaghetti and worked-around debt (10)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| DEBT-04 | 5 | Game code's natives are kept in three hand-synced places, one an 860-line function | `crates/script/src/host.rs:486` | script F5 |
| DEBT-05 | 5 | Workbench does everything: 108 fields, impl blocks in 34 files, a 430-line show | `crates/editor/src/workbench.rs:267` | editor-workbench F5 |
| DEBT-01 | 4 | The splash's update question is dead code, and its docs describe it as live | `crates/app/src/startup.rs:96` | app-and-tools F8 |
| DEBT-02 | 4 | Four copies of snapshot undo that have drifted apart | `crates/editor/src/tileset_editor.rs:125, frames_editor.rs:88, atlas_editor.rs:54, animator_editor.rs:99` | editor-tools F7 |
| DEBT-03 | 4 | Six copies of the same drop-down in the inspector | `crates/editor/src/inspector.rs:506` | editor-chrome F7 |
| DEBT-06 | 4 | world/src/lib.rs is one 1,699-line file around one type | `crates/world/src/lib.rs:108` | world-sim F7 |
| DEBT-07 | 4 | run_game is one 485-line function; play.rs and save_game.rs are twice the size guide | `crates/editor/src/play.rs:460` | editor-play F7 |
| DEBT-09 | 4 | viewport.rs (1,742 lines) and the editor views are far past the size guide | `crates/editor/src/viewport.rs:887` | editor-tools F9 |
| DEBT-10 | 4 | render_scene is 312 lines; renderer files far past the size guide | `crates/render/src/renderer.rs:1101` | render-ui F6 |
| DEBT-08 | 4 | The editor's panel files and functions are several times the size limit (launcher, library, search, properties) | `crates/editor/src/library.rs, launcher.rs, search.rs` | editor-chrome F6 |

## Dependencies (6)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| DEP-01 | 5 | The font library is no longer shared with the editor's, and its unmaintained parser ships on Windows on a false reason | `Cargo.toml:47, deny.toml:16` | dependencies F2 |
| DEP-02 | 4 | The triple_buffer stand-in can make the audio thread wait, which its own README forbids | `patches/triple_buffer/src/lib.rs:17` | dependencies F3 |
| DEP-03 | 4 | The Luau source hold exists only as a comment and a lock line | `Cargo.toml:106` | dependencies F4 |
| DEP-05 | 3 | Solis's glam is one version ahead of every library that speaks glam (compiled twice) | `Cargo.toml:38` | dependencies F6 |
| DEP-06 | 3 | notify-debouncer-mini ships tempfile in the program for an 80-line wrapper | `crates/asset_db/src/watch.rs:7` | dependencies F7 |
| DEP-07 | 2 | log is declared but unused in two crates, and the register test can't see it | `crates/sound/Cargo.toml:25, crates/updater/Cargo.toml:23` | dependencies F8 |

## Flaky tests (4)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| TEST-01 | 5 | A sidecar test must reach its check within 50 ms of writing a file | `crates/asset_db/src/db.rs:783` | flaky-tests F1, assets-media F7 |
| TEST-02 | 4 | Tests race the game code's real-time limits (100 ms a call, 250 ms a tick) | `crates/script/src/runtime_hostile_tests.rs:162, crates/editor/src/game_code_tests.rs:473` | flaky-tests F2, script F7, editor-play F6 |
| TEST-03 | 3 | The window smoke test waits by sleeping fixed times | `tools/smoke/window-smoke.sh:65` | flaky-tests F3 |
| TEST-04 | 2 | Short wall-clock budgets asserted in ordinary tests | `crates/reflect/src/id.rs:297, crates/editor/src/workbench_paint_tests.rs:77, crates/app/src/sprites.rs:539` | flaky-tests F4, world-sim F10, editor-workbench F10 |

## Checks that don't check (5)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| CI-02 | 6 | The weekly whole-game invariants job cannot fail | `.github/workflows/deep-checks.yml:113` | app-and-tools F4 |
| CI-01 | 5 | Ten of the 23 fuzz targets never run in CI; playtest.txt has no target; fuzz READMEs are behind | `.github/workflows/deep-checks.yml:45, fuzz/` | app-and-tools F6, security F3, docs-developer F1 and F7 |
| CI-03 | 3 | The release pipeline installs unpinned tools | `.github/workflows/windows-exe.yml:98` | app-and-tools F10, security F6 |
| CI-05 | 3 | The fuzzers' lock file is ungoverned and out of step with the main one | `fuzz/Cargo.lock, .github/dependabot.yml` | dependencies F5 |
| CI-04 | 2 | The updater's network tests are ignored and nothing runs them | `crates/updater/src/tests.rs:473` | flaky-tests F5 |

## Outdated documentation (15)

| # | Severity | Problem | Where | Found by |
| --- | --- | --- | --- | --- |
| DOC-08 | 5 | The manual's interface tour still describes the editor from before the overhaul | `docs/manual/interface-tour.md:11-41` | docs-manual F1 |
| DOC-09 | 5 | The manual says Solis reopens the last project; it never does | `docs/manual/projects-and-the-library.md:19` | docs-manual F2 |
| DOC-04 | 4 | Phase numbers retired by decision 0097 are still used, one shown to users | `crates/app/src/sample_game.rs:110, docs/manual (5 pages), ARCHITECTURE.md:100` | docs-developer F5, docs-manual F8 |
| DOC-01 | 4 | Crate READMEs' "Depends on" sections no longer match Cargo.toml | `crates/script, editor, app, area, asset_db READMEs` | docs-developer F2 |
| DOC-02 | 4 | ARCHITECTURE.md has eight statements the code has overtaken | `ARCHITECTURE.md:20,27,59,62,110,112,114,136` | docs-developer F3 |
| DOC-03 | 4 | The README's Status paragraph is stuck at Phase 1, and no top-level folder is described | `README.md:7` | docs-developer F4, docs-manual F6 |
| DOC-10 | 4 | The manual says Properties shows starting values while playing; it shows live values | `docs/manual/game-code.md:73` | docs-manual F3 |
| DOC-11 | 4 | Eleven manual pages still say built features are "coming" | `docs/manual (2d-pixel-perfect, sprites, screens, sounds, importing-models, entities, save-games, tile-maps, looks)` | docs-manual F4 |
| DOC-06 | 3 | RENAMING.md misses five file extensions and one hard-coded name | `RENAMING.md:21, crates/script/src/screens.rs:25` | docs-developer F9 |
| DOC-07 | 3 | Doc comments in source files describe an earlier state of the code | `about 35 places across script, render, world, tiles, editor, app` | docs-developer F10, assets-media F8, render-ui F8, script F10, editor-chrome F10, editor-play F8 |
| DOC-05 | 3 | Source-folder READMEs miss files and name functions that don't exist | `crates/editor/src, script/src, tiles/src, world/src, physics/src, animation/src, area/src/builtin READMEs` | docs-developer F6, F8, F11; world-sim F9; assets-media F8; editor-workbench F9 |
| DOC-12 | 3 | The manual names a menu item and a control that don't exist | `docs/manual/characters-and-cameras.md:23, physics.md:15, play-mode.md:56` | docs-manual F5 |
| DOC-13 | 3 | ROADMAP and plans state built things as begun, and 2D triggers as delivered | `ROADMAP.md:183-187, docs/plans/lust-data.md:7, ideas.md:5` | docs-manual F7 |
| DOC-14 | 2 | The shortcuts page claims every key but omits the newer ones | `docs/manual/keyboard-shortcuts.md` | docs-manual F9 |
| DOC-15 | 2 | The game code page lists three files in the solis folder; there are many more | `docs/manual/game-code.md:18` | docs-manual F10 |

## Not fixed in this round, on purpose

- **DEBT-08** (the editor's panel files are several times the size guide): the editor's interface is about to be remade from scratch (Shared 1, decision 0098), which replaces these files. Splitting them now would be thrown away.
