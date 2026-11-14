# 0113: Pictures and three languages in the guide

- **Status:** Accepted (the project lead asked for both, 2026-11-14, and chose screenshots taken by a script from the real editor; the rest are the recommended choices, taken by the lead reviewer while the project lead was away, and can be changed)
- **Date:** 2026-11-14

## Problem

The project lead asked to re-do the documentation with two things: **screenshots of the app** for examples, and **a switch between Lust (the standard), Rust and Lua on every code example**, which the guide remembers the next time it is opened.

The user guide is the manual, built ([decision 0112](0112-the-user-guide-is-the-manual-built.md)), so both have to live in the manual's Markdown and be kept accurate by checks, as the words are. Most game-code examples showed Lust only; a few showed Lua or Rust, as fragments.

## Options

For the pictures:

1. **Taken by hand:** quick once, but nobody retakes them when the interface moves, so they drift.
2. **Taken by a script from the real editor** (chosen by the project lead): the same steps each time, so they are retaken whenever the interface changes and stay accurate.
3. **Drawn mock-ups:** never match the editor exactly.

For the examples: a page of each language beside the Lust one (three copies of every page), or **each example in all three, side by side in the Markdown, with a switch in the guide** (recommended).

## Decision

**Pictures:**

1. **A script takes them:** `tools/guide-pictures/take-pictures.sh` starts the real editor on a virtual screen (1440 × 900, Mesa's software driver) in a throwaway home folder, on a new copy of one of File › Learn's samples made from the command line (`--sample <which> <folder>`, new for this), drives it with `xdotool`, and saves each picture cropped to what it shows and reduced to 192 colours, into `docs/manual/pictures/`. Each picture's steps are in one list, `tools/guide-pictures/pictures.list`, with the page that shows it and what it shows; actions are reached through the search where they can be, so a moved button breaks fewer pictures.
2. **In the manual:** `![What it shows](pictures/name.png)` alone on its line, with a caption line under it (`_template.md`). The guide writes each picture into its one file as data (so it still opens offline), as a figure that opens larger when clicked.
3. **Kept accurate:** the guide isn't built when a picture a page shows is missing, a picture is shown by no page, or a picture isn't a PNG or is over 200 KB. And the script writes a stamp, `tools/guide-pictures/interface.txt`: the editor's approved picture-test images (`crates/editor/tests/pictures`) with a hash of each. Those images change exactly when the interface's look changes, so a test in the normal suite (no GPU) fails when they differ from the stamp, saying the pictures are older than the interface and naming the command. CLAUDE.md and CONTRIBUTING.md say to run it.

**Three languages:**

4. **In the manual:** every game-code example is three fenced blocks one after another, `lust`, `rust` and `lua`; code that isn't game code is fenced without a language and has no switch. Each is a whole file, follows the API guidelines ([decision 0065](0065-engine-api-guidelines.md)) and calls only what the engine has for that language. Where a language can't do the same (tweens and game-wide objects are Lust's own, screens are Lust files), its block says how it's done there. A Rust example may leave out the fields' plumbing (`values`, `set`, the `RustKind`), shown once in full on the Game code page.
5. **In the guide:** a switch above each example, **Lust**, **Rust**, **Lua**; choosing one changes every example on the page, and the browser's storage remembers it (a browser that refuses storage starts at Lust). The search finds words in all three, and showing a match in another language switches to it. Without the guide's script, all three show, each named.
6. **Kept accurate:** the guide isn't built when a game-code block isn't part of a whole example, or code fenced without a language reads as Lust. The script crate's tests run every Lust example through Lust with the engine's own description files (and a stand-in project's names), and compile and start every Lua example as Solis does; the guide's tests balance every Rust example's brackets and check that its `host()` and `cx` calls, and the Lua's `solis.` calls, are in `docs/game-code-interface.txt`.

## Consequences

- The guide grows by the pictures (about 0.7 MB, written in as base64), and the repository by the PNGs.
- A pull request that changes the interface's look approves new picture-test images, and then the pictures check fails until the pictures are taken again (`tools/guide-pictures/take-pictures.sh`, which also builds the guide) and committed. A picture whose steps no longer fit the layout shows the wrong thing: its steps in `pictures.list` change with it, and the pictures are looked at before they're committed.
- The floating tool bar, when it comes, retakes them.
- Every new game-code example is written three times, and the checks hold all three to the engine. Rust examples are checked for their brackets and engine calls only, not compiled: Solis can't build a game's Rust yet, and compiling them would need a project of their own.
- `--sample` is a command-line option users can call too; it is in `--help` and on the Projects page.
