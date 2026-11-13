# Full review of November 2026: the second pass and the end of Shared 1

## Summary

All 125 problems the review tracked are dealt with: **120 fixed, 5 fixed in part** (big files split, with the rest of each written down). The fixes are in pull requests 228 to 250. The editor's new interface (Shared 1) is built too.

This second pass had three reviewers read everything that changed since the review began: the whole new interface and every fix. Their angles were stability, speed, and how the parts fit together. They found **27 new problems**, all smaller than the review's worst ones. **The most serious:** a game written only in Lua is told its code is missing and can't be played or exported. Four more are at severity 5:
- The area's keys act on an area you can't see.
- Removing a bus in the mixer can remove the next one too.
- Solis no longer opens on Recent projects after the first run.

Only one more is a crash or lost-work risk; most are speed costs and small mismatches. No code was changed in this pass.

The **scale check** ran on a 2-core machine with no graphics card. Most targets are met, and the editor's own cases got faster since 7 November. The slow cases it marks were already slow on this kind of machine, but the renderer's hand-over is 15 to 20 % slower than on 7 November, and one case no longer runs at all (both are below).

## New problems (most serious first)

Severity runs from 1 to 10: 10 is lost work, a crash in normal use or something a hostile file can exploit; 5 is a real bug in an edge case; 2 is small. Each has a proposed fix in the details below.

- **N-01 (6)** A game written only in Lua is told "main.lst… there isn't one": Play is refused and the export says the code has mistakes. *(The fix for games with no code missed this case.)* Fix: check for `main.lst` only when the game has Lust files of its own. **Fixed** (`ee4dfb969`)
- **N-02 (5)** On the Audio, Debug, Build or Library tab, Delete, Ctrl+D and Ctrl+Z still act on the area you can't see (a crate deleted while you meant a bus). Fix: the area's keys rest on any tab without a 3D or 2D view. **Fixed** (`ae1336fd6`)
- **N-03 (5)** Remove a bus and the next bus becomes the picked one; a second click on Remove deletes it too, saved at once with no undo. The Input page has the same problem with its actions. Fix: keep the pick by name, and pick Master after Remove. **Fixed** (`bda9919f6`)
- **N-04 (5)** Solis opens on Home instead of Recent projects every time after the first, because the saved layout is put back after the projects page was chosen (decision 0105). Fix: put the layout back first; a test of the real start order. **Fixed** (`e6d0537fc`)
- **N-05 (4)** Pressing Enter to confirm a project's new name also opens that project. Fix: ignore Enter on the cards while a name is being typed. **Fixed** (`e9b3fc033`)
- **N-06 (4)** Every visit to the File tab (Preferences, About too) reads up to 200 projects' files while the window waits: seconds of freeze with a project on an unplugged drive. Fix: read them on a worker, or only for Recent projects. **Fixed** (`8334c85b0`)
- **N-07 (4)** The new font reader (from the fix of DEP-01) reads the whole font again for every word it measures: thousands of times a frame for a screen full of text. Fix: read the font once and keep what measuring needs. **Fixed** (`36284286c`)
- **N-08 (4)** The Picture tab draws every crop in its list and on the picture every frame (up to 4,096). Fix: draw only the rows and boxes on screen. **Fixed** (`642045a6e`)
- **N-09 (4)** Build's big Export button resets the export form instead of exporting: the typed name is lost and "Open folder" greys out. Fix: make it do what the page's Export button does. **Fixed** (`04cbbe727`)
- **N-10 (4, not yet seen by a reviewer)** Two characters starting on the very same spot and both walking fly apart and sink through the floor (seen by the fixer of NEW-01 in a probe). Fix: confirm with a test, then push overlapping characters apart gently. **Fixed** (`b818cedcb`)
- **N-11 (3)** The renderer hands over sprites, projectiles and models 15–20 % slower than on 7 November, on the same kind of machine, while the editor's own cases got faster. Fix: compare the renderer before and after pull request 245 and find the step.
- **N-12 (3)** The scale case "quick-saving over a big save" no longer runs: its made-up old save is now refused as damaged when saving over it. Fix: write a save of today's version in the case. **Fixed** (`22ff0ddb6`)
- **N-13 (3)** A Library action can freeze the window for over a second while a background scan runs (it now waits for it). Fix: don't wait; send it through the worker. **Fixed** (`eaa399ac0`)
- **N-14 (3)** Three different rules say which folder is the engine's (`solis/`), and the screens write `"solis/screen"` into your code: renaming the program would break screens. Your own file placed in `solis/` now counts as the engine's. Fix: one rule, from branding, listed in RENAMING.md.
- **N-15 (3)** Searching for a panel by name (Space) finds only 10 of them; "Sound events", "Frames" or "Canvas" find nothing. Fix: search every panel.
- **N-16 (3)** The UI canvas copies and lays out its screen twice every frame and matches boxes to pieces the slow way. Fix: keep the result until the screen changes. **Fixed** (`885548588`)
- **N-17 (3)** The Library and the Assets column lowercase every file's name every frame while you search, and the folder tree draws every folder. Fix: keep the lowered names; draw only the visible folders. **Fixed** (`9c9054c4d`)
- **N-18 (3)** With a tileset open on any tab, the whole editor redraws 25 times a second forever; an open clips file sorts every picture's name every frame. Fix: only while its tab is in front. **Fixed** (`a231a78a7`)
- **N-19 (3)** The Area column goes through every entity every frame once anything is hidden or locked. Fix: keep the result until the area changes. **Fixed** (`e99aae5a0`)
- **N-20 (2)** The new folder watcher says nothing while files keep changing faster than every 0.3 s, so the Library doesn't update. Fix: also report every 2 seconds during a long burst. **Fixed** (`8b276d5bb`)
- **N-21 (2)** The search shows no key for the tile tools, the copy axes and the Delete keys of the 2D editors. Fix: take the keys from the tools.
- **N-22 (2)** A contextual tab's shelf (Tileset, Picture) is saved but never put back. Fix: look it up among the contextual tabs too. **Fixed** (`ed68d3789`)
- **N-23 (2)** The manual and a few notes still say some actions are only in the search, that Ctrl+Space opens the Library, and that the window test drags a panel. Fix: correct them.
- **N-24 (2)** A few colours from the old look bypass the theme (the playhead, the unpicked crops): with the orange accent, picked and unpicked crops look alike. Fix: make them tokens.
- **N-25 (2)** The warning "too many model copies" repeats every frame. Fix: say it once, as sprites now do. **Fixed** (`4022219d0`)
- **N-26 (2)** No scale case walks 2D characters, so a slow-down in the new corner check would go unseen. Fix: add one over a tile map's walls. **Fixed** (`a23edb3be`)
- **N-27 (2)** Small leftovers: a test writes `escape.note` into the shared temporary folder; the screen editor and tile-map painting keep their own undo; a few views still use the old pickers; Luau's messages name the program; `tools/storage-bench` still asks for the old maths library.

## What the review fixed

All 125, by kind; *(in part)* means the rest of it is written down in the status file.

**Security**

- SEC-01 (8): An assembly's "missing part" overrides aren't counted, so a small project can expand to 100 million entities
- SEC-02 (8): A small .glb can make the importer copy its binary part once per buffer (100 GB asked for)
- SEC-03 (6): "Open code" on a Lua component opens the .lua file in the system's program
- SEC-04 (5): A project's own file chooses the export folder, and the dialog probes it on opening (network share)
- SEC-05 (4): Choosing a placeable file in the search opens it in the system's program
- SEC-06 (6): A copy that wasn't installed still downloads and silently runs the installer
- SEC-07 (5): getfenv and setfenv are left in the Lua sandbox
- SEC-08 (6): A screen's styles are copied out of game code with no size cap
- SEC-09 (5): A tile map in the Lust layout has no block limit and is grouped quadratically
- SEC-10 (4): A sidecar using the biggest part ID makes giving out new IDs quadratic
- SEC-11 (4): A save's lines are split into unbounded word lists before their length is checked
- SEC-12 (3): A collider's centre can reach the physics library unbounded or infinite
- SEC-13 (4): Nine copy-pasted save functions, one (sound events) missing its path check
- SEC-14 (5): The Windows build decodes BMP pictures that nothing tests or fuzzes
- SEC-15 (3): The "ordinary file, bounded read" guard exists four times, and sound files have none
- SEC-16 (8): The weekly fuzzer found a file that aborts the model reader; the issue has been open since 1 November
- SEC-17 (8): The weekly fuzzer found a sound file that asks for 16 GB of memory; the issue has been open since 1 November
- SEC-18 (5): A streamed Ogg's seek can reach stream headers nothing checked, seek(0) reopens without the check, and two hostile test files pass for the wrong reason
- SEC-19 (7): A fuzzer that had never run found a tileset file that hangs the reader within one minute
- SEC-20 (7): A fuzzer that had never run found an atlas file that crashes its reader within one minute
- SEC-21 (6): Data files are checked by the Lust compiler with no budget: a 160 KB file takes 13 seconds and bigger ones hang

**Bugs**

- BUG-01 (8): Exported games never get a sound device: they play no sound
- BUG-02 (8): A light with a negative or NaN cone crashes the editor (clamp panics)
- BUG-03 (8): Keep after Stop also keeps what the game spawned, despawned or loaded
- BUG-04 (6): The pending Keep/Undo question is ignored by quit and survives opening another project
- BUG-05 (5): Moving or deleting the open area's file while playing is undone by Stop
- BUG-06 (7): Delete and Ctrl+D over the UI canvas or Atlas tab act on the area's selection
- BUG-07 (6): Opening another project leaves per-project state behind (map undo writes into the new project)
- BUG-08 (5): The clips editor never follows its file on disk, unlike its three siblings
- BUG-09 (5): Sheets, maps and clips have two owners, and the app's copy overwrites the editor's
- BUG-10 (4): Caches use a freed pointer's address as their version
- BUG-11 (4): A screen whose view fails runs the game's screen code again every frame
- BUG-12 (7): A parent loop in an area file makes world matrices quadratic and overflow
- BUG-13 (6): A placed assembly's part moved to the top level is not saved
- BUG-14 (5): In a world with no 3D bodies, physics returns before children are settled (one tick behind)
- BUG-15 (4): 2D physics empties itself silently after a caught panic and is never rebuilt
- BUG-16 (4): Edits while playing drop state game code set (shown screens, stopped sounds)
- BUG-17 (7): Tileset edits from the Library's Properties skip undo; one click deletes a terrain's art
- BUG-18 (6): Dragging a piece on a UI canvas never moves it
- BUG-19 (5): Tile caches shared by every view and the Game panel evict each other each frame
- BUG-20 (5): Dragging a number or colour in the 2D editors makes one undo step and one save per frame
- BUG-21 (5): The Frames ruler's work has no bound (1.2 million marks a frame)
- BUG-22 (5): Tile-map UI scans the whole map every frame
- BUG-23 (4): Scaling with the gizmo can store a NaN rotation
- BUG-24 (3): Right-click doesn't cancel a box selection in a perspective view
- BUG-25 (6): Dragging a number in Input actions saves and re-checks the project every frame
- BUG-26 (5): Closing the Canvas or Elements tab closes the clips file instead of the screen
- BUG-27 (5): A Markdown file is re-parsed every frame, and its inline parser is quadratic
- BUG-28 (5): Panels walk the whole area every frame
- BUG-29 (3): Hard-coded neutral colours left behind by the theme switch
- BUG-30 (3): The Library's right-click menu shows "Open" twice
- BUG-31 (6): Pictures and tile maps up to 64 MB are read and decoded on the window thread
- BUG-32 (6): Rescanning the assets folder hashes whole files and parses every sidecar on the frame loop
- BUG-33 (4): A crash in the game-code worker is silent and leaves an exported game on "Starting" forever
- BUG-35 (4): Starting a streamed sound opens and scans the whole Ogg file on the frame loop
- BUG-36 (3): The import cache hashes a file, then reads it again to import it
- BUG-37 (2): Changing a tile value's kind skips the map's memory limit
- BUG-38 (5): Projectiles and unlit models vanish in a 2D picture that has a parallax layer behind it
- BUG-39 (5): Game UI and the pixel stage create GPU buffers and bind groups every frame
- BUG-40 (4): Every viewport target allocates a 16 MiB shadow map, including parallax layer pictures
- BUG-41 (4): A full letter atlas wipes itself mid-layout: garbled text and a frame-rate collapse
- BUG-42 (4): "Matches the shader layout" tests only check Rust against itself
- BUG-43 (3): The letter atlas's revision isn't unique across font sets, and its upload skips the length check
- BUG-44 (2): Frame statistics count sprites that were not drawn, and warn every frame
- BUG-45 (2): move_piece computes an indent and throws it away
- BUG-46 (6): Lua can't reach Window, Screens, Maps or Animators
- BUG-47 (6): Random numbers and spawned IDs repeat while game speed is 0
- BUG-48 (4): Events sent by a component's start are credited to another component
- BUG-49 (4): A native's error reaches a Lua author with no line, in the library's words
- BUG-50 (4): A compile that hangs leaves its thread running; a new one starts on every save
- BUG-51 (3): A spawn that can't be made is dropped without a word
- BUG-52 (2): Two messages lost their line continuations and show runs of spaces
- BUG-53 (5): Every change in the assets folder walks the whole folder about five times on the window thread
- BUG-54 (4): Typing a field's starting value into Properties while playing never reaches the game
- BUG-55 (8): The editor can save a tile map it cannot reopen (the format's limits allow maps bigger than the reader can read)
- BUG-56 (6): A placed assembly part moved out from under another part of the same copy is lost on reopen once its old parent part is deleted

**Tests**

- TEST-01 (5): A sidecar test must reach its check within 50 ms of writing a file
- TEST-02 (4): Tests race the game code's real-time limits (100 ms a call, 250 ms a tick)
- TEST-03 (3): The window smoke test waits by sleeping fixed times
- TEST-04 (2): Short wall-clock budgets asserted in ordinary tests
- TEST-05 (5): The game-code checksum test sometimes differs between 1 and 3 shards on a busy machine
- TEST-06 (2): Tests leave their temporary folders behind (over 10,000 in /tmp after a day of runs)
- TEST-07 (3): The window smoke test's import step looks for the model on a tab with no 3D view, and its Don't save click misses
- TEST-08 (3): The window smoke test counts two empty (all-black) screens as settled

**Dependencies**

- DEP-01 (5): The font library is no longer shared with the editor's, and its unmaintained parser ships on Windows on a false reason
- DEP-02 (4): The triple_buffer stand-in can make the audio thread wait, which its own README forbids
- DEP-03 (4): The Luau source hold exists only as a comment and a lock line
- DEP-05 (3): Solis's glam is one version ahead of every library that speaks glam (compiled twice)
- DEP-06 (3): notify-debouncer-mini ships tempfile in the program for an 80-line wrapper
- DEP-07 (2): log is declared but unused in two crates, and the register test can't see it

**CI**

- CI-01 (5): Ten of the 23 fuzz targets never run in CI; playtest.txt has no target; fuzz READMEs are behind
- CI-02 (6): The weekly whole-game invariants job cannot fail
- CI-03 (3): The release pipeline installs unpinned tools
- CI-04 (2): The updater's network tests are ignored and nothing runs them
- CI-05 (3): The fuzzers' lock file is ungoverned and out of step with the main one
- CI-06 (6): The weekly whole-game invariants job has never really run: it cannot build, and reported success

**Debt**

- DEBT-01 (4): The splash's update question is dead code, and its docs describe it as live
- DEBT-02 (4): Four copies of snapshot undo that have drifted apart
- DEBT-03 (4): Six copies of the same drop-down in the inspector
- DEBT-04 (5): Game code's natives are kept in three hand-synced places, one an 860-line function *(in part)*
- DEBT-05 (5): Workbench does everything: 108 fields, impl blocks in 34 files, a 430-line show *(in part)*
- DEBT-06 (4): world/src/lib.rs is one 1,699-line file around one type
- DEBT-07 (4): run_game is one 485-line function; play.rs and save_game.rs are twice the size guide
- DEBT-08 (4): The editor's panel files and functions are several times the size limit (launcher, library, search, properties) *(in part)*
- DEBT-09 (4): viewport.rs (1,742 lines) and the editor views are far past the size guide *(in part)*
- DEBT-10 (4): render_scene is 312 lines; renderer files far past the size guide *(in part)*

**Documentation**

- DOC-01 (4): Crate READMEs' "Depends on" sections no longer match Cargo.toml
- DOC-02 (4): ARCHITECTURE.md has eight statements the code has overtaken
- DOC-03 (4): The README's Status paragraph is stuck at Phase 1, and no top-level folder is described
- DOC-04 (4): Phase numbers retired by decision 0097 are still used, one shown to users
- DOC-05 (3): Source-folder READMEs miss files and name functions that don't exist
- DOC-06 (3): RENAMING.md misses five file extensions and one hard-coded name
- DOC-07 (3): Doc comments in source files describe an earlier state of the code
- DOC-08 (5): The manual's interface tour still describes the editor from before the overhaul
- DOC-09 (5): The manual says Solis reopens the last project; it never does
- DOC-10 (4): The manual says Properties shows starting values while playing; it shows live values
- DOC-11 (4): Eleven manual pages still say built features are "coming"
- DOC-12 (3): The manual names a menu item and a control that don't exist
- DOC-13 (3): ROADMAP and plans state built things as begun, and 2D triggers as delivered
- DOC-14 (2): The shortcuts page claims every key but omits the newer ones
- DOC-15 (2): The game code page lists three files in the solis folder; there are many more

**Found while fixing**

- NEW-01 (4): Characters moved together end somewhere else than moved one by one (the property test found a case): one standing in another was carried by it
- NEW-06 (5): The save_game fuzz target no longer compiled (it still passed the old second argument to `Workbench::new`), so the weekly fuzzing of save games would fail
- NEW-09 (4): A 2D character can end inside a wall's corner (the property test rolled a case: 0.208 from the corner, its radius 0.25): Rapier's shape cast missed the wall
- NEW-10 (5): A game with no code of its own can't be played exported: the engine's own `assets/solis/*.lst` were taken for game code ("main.lst is missing"), and `Workbench::code_ready` waited for a program there's none of

## The scale check

Run with `tools/scale/scale-check.sh` on 2026-11-13, at `main` after pull request 250, on a 2-core Linux machine with Mesa's software renderer (no graphics card). Cases "judged on 4+ cores" and the viewport's times are reported, not judged.

| Case | 7 November | Now | Target |
| --- | --- | --- | --- |
| The target level's CPU frame (2,000 units, 20,000 lasers) | 9.39 ms SLOW | 8.07 ms SLOW | 8.0 ms |
| Batching 100,000 sprites | 22.28 ms SLOW | 16.60 ms SLOW | 4 ms |
| Editor frame while playing and dragging, 2,000 entities | 4.81 ms SLOW | 4.32 ms SLOW | 4 ms |
| A Markdown note of 78 KB in Properties | – | 5.33 ms SLOW | 4 ms |
| 10,000 moving units, a tenth in view (GPU test) | 31.79 ms SLOW | 36.48 ms SLOW | 8 ms |
| Handing over 100,000 projectiles (GPU test) | 73.82 ms SLOW | 87.73 ms SLOW | 2 ms |
| Handing over 100,000 sprites (GPU test) | 84.42 ms SLOW | 100.18 ms SLOW | 2 ms |
| 10,000 detailed units, most far away (GPU test) | 986.94 ms SLOW | 1,119.56 ms SLOW | 8 ms |
| Quick-saving over a big save | – | **fails to run** (N-12) | 1,000 ms |

Every other case meets its target, among them: the editor frame with 10,000 entities listed and searched 0.84 ms (8), dragging 2,000 entities 4.55 ms (8), a change in an assets folder of 10,000 files 18.7 ms (50; it was 53.0 before the review), 10,000 animators 0.885 ms a tick (1.5), a Lust edit reaching a running game 326 ms (2,000), 2,000 AudioSources' sound 0.801 ms (1), and the tile-map cases (all well inside). The renderer's GPU times on a software renderer say little about a real card; their rise against 7 November on the same kind of machine is N-11.

## Details for fixing (for the AI)

Each entry: where, the evidence the reviewer quoted (checked by the lead reviewer where marked ✓), the failure, the proposed fix, and what it came from. Line numbers are at `49ed8c28a`.

### N-01 ✓ Lua-only games: `check_entry` still runs (6, fix of NEW-10)
- **Where:** `crates/script/src/code.rs:408-411` (`compile_here`: `if !code.files.is_empty() { let mut missing = check_entry(code); …`), the same gate for screens at `:488`, `check_entry` at `:530`.
- **Cause:** `code.files` always holds the engine's `solis/*.lst`, so the gate is always open. `check_entry` only asks whether `main.lst` exists.
- **Fix:** work out the game's own Lust files once (`files` minus `describe::engine_folders`) and gate both blocks on that.
- **Tests:** `check()` on the engine files plus one `.lua` gives no problems; an exported Lua-only game starts.

### N-02 Area keys with no area on screen (5, remake)
- **Where:** `crates/editor/src/key_owner.rs:111-147`, `workbench_keys.rs:150-184`, `key_owner_tests.rs:244-258`.
- **Cause:** `covers_center()` is true only for File and unbuilt tabs; everything else without an owner falls to `KeyOwner::Area`. The `OWNERS` table asserts that Ctrl+D, Esc and Ctrl+A over `Panel::Library` act on the area, and it has no rows for Mixer, Bus, Profiler or Export.
- **Fix:** return `KeyOwner::Page` when no group of the tab in front holds `Panel::Viewport` (`workspaces::groups(tab, open)`).
- **Tests:** rows for Mixer, Bus, Profiler and Export; the Library row changed.

### N-03 ✓ Picks kept by position (5, remake)
- **Where:** `crates/editor/src/bus_view.rs:279-287` (`remove_bus` saves at once); `mixer_view.rs:188-196` (`set_mixer` forgets the pick only when it's past the end); `:100-106` (`bus_shown`); `file_pages/input_page.rs:49-51,318-392` (`picked`, `listening` by index).
- **Failure:** a second click on Remove deletes the next bus and the ducking rules that name it. Reloading `mixer.solmix` with the buses reordered moves edits to another bus. On the Input page, the next key binds to another action.
- **Fix:** keep picks by name (bus name, action name), or clear them whenever the list changes. Pick Master after Remove.

### N-04 ✓ Start-up order (5, remake; this was NEW-02)
- **Where:** `crates/app/src/window.rs:257-258` (`start_on_recent_projects()` then `saved.settle()`), `window_saved.rs:118-121`, `crates/editor/src/shell/saved.rs:133-150,169-171` (`restored` never gives File).
- **Fix:** settle first, then go to Recent projects; or let `restore_shell` keep File in front (the restored tab becomes `before_file`).
- **Tests:** an app-level test through both steps in the real order. Today's tests call only one of them.

### N-05 Enter on the project cards (4, remake)
- **Where:** `crates/editor/src/file_pages/recent.rs:193-200` (`cards()`: Enter with no focus or dialog check), `project_card.rs:91-93`, `file_pages/mod.rs:435-442` (rename confirmed on `lost_focus` plus Enter in the same frame).
- **Fix:** ignore Enter while `renaming.is_some()` or a widget has focus, as `Walk::pressed` already does.

### N-06 Project list read on every File visit (4, remake)
- **Where:** `crates/editor/src/shell/file_page.rs:83-87` (`ListProjects` on every entry), `crates/app/src/window.rs:992,268-271`, `known_projects.rs:200-235` (`rows()`: `Project::summary`, `is_file`, `is_dir` for up to 200 projects).
- **Fix:** read the rows on a worker and hand them back like the scans do; or re-read only when Recent projects is shown, keeping the rows otherwise.

### N-07 ✓ Font read per measure (4, fix of DEP-01)
- **Where:** `crates/ui/src/font.rs:49-60` (`at()` builds `FontRef`, the charmap, metrics, kern and `outline_glyphs()` on each call); callers `text.rs:293-298,342`, `view.rs:675-681,699-716` (one `text_width` per word), `layout.rs:547-556`.
- **Fix:** read the font once. Keep a char → (glyph, advance) cache and the kern lookups in `Font`, and build the outlines only in `Fonts::make`.
- **New scale case:** the 4,096-letter paragraph wrapped with a real `Fonts`. Today's case uses `Fixed`.

### N-08 ✓ Every crop drawn every frame (4, remake)
- **Where:** `crates/editor/src/atlas_view.rs:135-174` (a row per crop, three texts and two `format!` each, no `show_rows`), `:298-321` (every crop's box and name on the picture).
- **Fix:** list the crops with `ScrollArea::show_rows`; on the picture, skip boxes outside the clip and names below a zoom.
- **New scale case:** an atlas of 4,096 crops with the Picture tab in front.

### N-09 Build's Export resets the form (4, remake)
- **Where:** `crates/editor/src/shell/ribbons/build.rs:20` → `MenuCommand::ExportPlaytest` → `open_export()` + `prepare_export()` (`export_dialog.rs:245-248,296-302`). This puts back a fresh `ExportForm` with `done: None` and `busy: false`.
- **Fix:** on the Build tab, send `ExportPlaytest(request)` from the current form. `open_export` prepares only when no form is open.

### N-10 Two characters at one spot, both walking (4?, older; not reviewed)
- **Reported:** by the fixer of NEW-01 from a probe: two characters at exactly one place that both move go sideways about 70 m in 2 s and sink through the floor.
- **First step:** a test (`crates/physics/src/tests.rs`, 3D and 2D). Then resolve overlaps between characters with a bounded push, not a free slide.

### N-11 ✓ Renderer hand-over slower than on 7 November (3, likely pull request 245)
- **Evidence:** on the same kind of machine (2 cores, llvmpipe), the GPU cases are 15–20 % slower:
  - handing over 100,000 sprites: 84 → 100 ms;
  - handing over 100,000 projectiles: 74 → 88 ms;
  - 10,000 detailed units, most far away: 987 → 1,120 ms;
  - 10,000 moving units: 32 → 36 ms.
- **Against noise:** the CPU-only editor cases got faster on the same run, and a second run on a quiet machine gave the same numbers.
- **Suspects:** BUG-38 (the layered projectile pipeline), BUG-44 (counting from `write_sprites`) and DEBT-10's passes split.
- **Fix:** bisect `cargo test --release -p render --test gpu scale_check` between `50f007a47` and `main`.

### N-12 ✓ A scale case that no longer runs (3, fix of a save finding)
- **Where:** `crates/app/src/fixtures_tests.rs:286-292`. The case writes `"solis save 3\n" + 64 MB of x` and saves over it. That now fails with "the upgraded file didn't read back the same (it's damaged), so the old one was kept", because saving over an older save upgrades and checks it first.
- **Fix:** write a save of today's version (or a real old one) so the case times what it was made for. `tools/scale/scale-check.sh` already fails on it.

### N-13 Sync rescans wait on the worker's scan (3, fix of BUG-32)
- **Where:** `crates/asset_db/src/db.rs:289-292,439-440` (`Scan::run` holds `looking` for the walk). Callers: `crates/app/src/window.rs` around 1396 (`rescan_now`) and 1486, and `project.rs` around 566 and 760.
- **Fix:** `try_lock` and fall back to asking the worker; or route these through the worker and take the new file's ID when the result comes.

### N-14 The engine folder by three rules (3, older)
- **Where:**
  - `crates/script/src/code.rs:46` (a first-line marker, `engine_folders`);
  - `crates/app/src/game_code.rs:167` (`branding::CODE_FOLDER`);
  - `crates/script/src/screens.rs:380` (`starts_with("solis/")`);
  - `crates/editor/src/screen_ties.rs:85,344,411` (writes `fetch "solis/screen"`);
  - `crates/script/src/luau.rs:285,303` (messages naming the program).
- **Fix:** one rule, `area::lust_data::ENGINE_FOLDER`, used everywhere; the names listed in RENAMING.md.

### N-15 Panel search (3, remake)
- **Where:** `crates/editor/src/search/list.rs:103-116` (a hand-written list of 10 panels).
- **Fix:** `Panel::ALL` chained with `Panel::OPEN_WITH_A_FILE`, keeping the `tab_for(..).is_some()` filter.

### N-16 UI canvas per frame (3, older)
- **Where:** `crates/editor/src/canvas_view.rs:258,267-268,282,297,328,343-353`: two clones of the view, `doc.matches()` rebuilt, the screen laid out twice, and a linear `find` per box.
- **Fix:** keep the drawn view, its matches (a HashMap from seen path to file path) and its boxes, keyed by the text revision, the hidden set, zoom and size.

### N-17 Library search and folder tree per frame (3, remake and older)
- **Where:** `crates/editor/src/library/mod.rs:407` and `assets_column.rs:43-57` (`to_lowercase()` per item per frame); `library/mod.rs:478` (the tree cloned and drawn whole).
- **Fix:** a lowered name kept per `LibraryItem` (made in `set_library`); `show_rows` for the tree.
- **New scale case:** 10,000 library items with a search typed.

### N-18 Open files cost on other tabs (3, older; common since the remake)
- **Where:**
  - `crates/editor/src/workbench_frame.rs:134-156`: a 40 ms repaint while any tileset is open and any tile animates;
  - `workbench_frames.rs:34-36`: `frames_ui` built every frame;
  - `sprite_sheets.rs:564-585`: `listed()` formats and sorts every picture's name.
- **Fix:** gate the repaint on the Tileset tab being in front and its own tileset animating; build the UI states only when their panels are drawn, or cache them by revision.

### N-19 Area column's hidden and locked sets (3, older; noted under BUG-28)
- **Where:** `crates/editor/src/area_panel.rs:402` (`in_effect` every frame), `:85-104`.
- **Fix:** keep the sets in `AreaPanelState`, keyed by the tree, its revision and the hidden and locked sets; or reuse the ones `prepare_frame` makes for the viewport.
- **Scale case:** extend the 10,000-entity case with a hidden top-level parent.

### N-20 Watcher starved by constant writes (2, fix of DEP-06)
- **Where:** `crates/asset_db/src/watch.rs:49-57,74-82`.
- **Fix:** also report when a burst began more than about 2 s ago.

### N-21 Shortcut table (2, remake)
- **Where:** `crates/editor/src/shell/command_words.rs:305-328`.
- **Fix:** take `UseTool` and `PaintWith` from `Tool::key()` and `PaintTool::key()`; add `TurnCopy`, `MapMeaning`, `CropDelete`, `ClipDelete`, `FramesTakeOut` and `StateDelete`.

### N-22 Contextual tabs' saved layout (2, remake)
- **Where:** `crates/editor/src/shell/saved.rs:121-127,134-142` (names are looked up in `RibbonTab::ALL` only).
- **Fix:** look them up in `ALL` and `CONTEXT`.

### N-23 Stale words (2, remake)
- **Where:** `docs/manual/interface-tour.md:69` and `keyboard-shortcuts.md:254` (the "search alone" lists); `crates/editor/src/shell_reach_tests.rs:45-53` (`Place::Search` for New, Open and Save area as); `key_owner.rs:13` (Ctrl+Space); `CONTRIBUTING.md:29` (the window test).

### N-24 Colours outside the tokens (2, remake)
- **Where:** `crates/editor/src/frames_view.rs:35-36` (`PLAYHEAD`, whose comment says it is Play's colour), `graph_canvas.rs:29` (`LIVE`), `atlas_view.rs:305` and `canvas_view.rs:384,519` (the old amber).
- **Fix:** named tokens in `tokens.rs`.

### N-25 ✓ Model copies warning (2, older)
- **Where:** `crates/render/src/passes.rs:174-179`.
- **Fix:** a per-target flag, as `sprites_dropped` does.

### N-26 No 2D characters scale case (2, fix of NEW-09)
- **Where:** `crates/physics/src/flat.rs:286-293,330-380`.
- **Fix:** add `scale_check_500_flat_characters_walking` over a tile map's walls.

### N-27 Leftovers (2)
- `crates/asset_db/src/backup.rs:497` ✓ writes `/tmp/escape.note`, shared between test runs: use a folder of the test's own.
- `screen_doc.rs` and `workbench_tiles.rs` (`MapEdits`) keep their own undo with `Vec::remove(0)`: move them onto `FileHistory`.
- Plain pickers remain in `sound_event_view.rs`, `screen_props`, `tile_properties`, `frames_view`, `map_tab`, `tileset_view` and `animator_view`.
- `tools/storage-bench/Cargo.toml:19` ✓ asks for glam 0.34.

### What the fixes left (the "in part" rows)
- **DEBT-04:** the natives and their published list aren't yet built from one table.
- **DEBT-05:** the five file editors have no single owner (`OpenFiles`), which needs a decision.
- **DEBT-08:** `library/mod.rs` is still 1,423 lines, with `library()` 113 lines and `item_actions` 96. The remake split the launcher and the search.
- **DEBT-09:** `tileset_view`, `canvas_view`, `animator_view`, `tile_view` and `frames_view` are 885 to 1,244 lines.
- **DEBT-10:** `gpu_model.rs`, ui's `view.rs` and `source.rs` are left.
- **Also over the size guide:** `physics/src/lib.rs` (1,133 lines), `viewport_tools.rs` (800) and `library_request` (221 lines).

### What the reviewers checked and found sound
- **Stability:** saved shell state (names, finite sizes, limits); the widgets' index math; the profiler's ring; the Console's caps; `patches/triple_buffer`; the atlas's `make_room`; `code_ready`; NEW-09's `missed`; the save readers' caps; `external.rs`'s paths; the unsaved and Keep dialogs.
- **Speed:** the status bar, tab bar, ribbon, shelf, groups and `why_not`; icons; the profiler and mixer meters; the search; the watcher; the GPU buffers (BUG-39 holds); saving the shell state.
- **Parts meeting:** the play tick's order after its split; `screen_ties`'s move; "hide unfinished parts" everywhere; the model viewer's tab; contextual tabs settling; `FileHistory`'s four users; ARCHITECTURE.md's editor section.
