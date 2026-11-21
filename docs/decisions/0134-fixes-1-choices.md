# 0134: Fixes 1's choices: many names, the smoke check, the level a project opens on, and Git LFS for big files only

- **Status:** Accepted (taken as recommended while the project lead asked for the fixes to be done without stopping; theirs to change)
- **Date:** 2026-11-20

## Problem

Fixes 1 ([decision 0133](0133-fixes-from-making-orc-invasion.md), [the plan](../plans/game-stages.md#fixes-1-bugs-and-a-project-check-that-starts-the-game-done)) put right the bugs found by making Orc Invasion. Five of them needed a choice:

1. **Too many names.** The Lua that Lust writes keeps every class, method, object and state of a program as a local of one function, and Luau allows 200. Solis may not change Lust (it's pinned), so the fix had to be Solis's.
2. **What `--smoke` plays and what fails it.**
3. **Which level a project opens on**, with nothing in the project's settings naming one.
4. **What a copy of a prefab is called.** The report said placing one in the editor names it after its root part; it named it after the prefab's file, as spawning did.
5. **Git LFS.** New projects sent every picture, sound and model through Git LFS (decision 0006). A game's pixel art is small, LFS needs installing and its own space on most hosts, and a newcomer's push failed for it.

## Options and decisions

1. **Names past the first 120 become the program's own globals** (`script::many_names`), rewriting Lust's Lua line by line as Solis already does for exact maths and states, so no line moves. Each Lust program has a table of globals of its own (never a Lua file's or Solis's), so the code means the same; a word Lust's runtime uses stays a local. Programs with fewer names are left exactly as they were. *Not taken:* fields of a table per file (every use of every name rewritten, which needs Lust's scoping); one Lust program per screen (a single file of 300 methods would still fail). **Asked of Lust:** write a program's top-level names as fields of a table per file, or spill past a count itself, so Solis's rewrite can go.
2. **`--smoke <ticks>` plays the game's start level** (the level its last playtest build started in, else the first level by name), with no input, for 1 to 36,000 ticks, after the other steps. **Every error and warning in the Console fails it**, each different one once with the first tick it was said on and how often. The check's Game code step also starts the screens, as Play does, so a screen that can't start is a mistake in its file without `--smoke`, and the editor's Problems shows it when the code is checked. *Not taken:* warnings as notes (most mean something's wrong; a project can run the check without `--smoke`).
3. **The level last opened or saved there on this computer** (kept in the project's `.solis` folder, beside the last export, so never shared through Git), **else the game's start level** (as above), so the "project settings" step is the playtest build's start level, the only place a project names one today. A project with no levels opens on a new one, as before. A sample, a new project and an exported game open the level they name.
4. **Both placing and spawning name a copy after its prefab's root part** (the name its page shows), or the prefab's name when the root has none. One rule for both, as the report expected.
5. **Only kinds of file that are usually big go through LFS:** `.psd`, `.kra`, `.blend`, `.fbx`, `.exr`, `.hdr`, `.mp4`, `.mov`, `.webm`. Pictures, sounds, models and fonts stay in Git, marked binary so Git never merges them as text. The manual says when LFS is worth it and how to turn it on for another kind. Existing projects' `.gitattributes` are left as they are. This narrows decision 0006 for new projects. *Not taken:* no LFS at all (a `.psd` or a video committed by habit would then bloat the history for good).

## Consequences

- A screen fetching a file of hundreds of methods starts; so does game code with hundreds of names (tested at 300 methods, states, classes, objects and events, a component of 300 fields, a state of 1,200 options, and one method calling 300 others).
- `solis --check-project <folder> --smoke 120` tells a game maker that the game starts, and fails a workflow when it doesn't.
- Two-PC checks and saves keep working: spawned names aren't part of the game's state list, and the copies' IDs are unchanged.
