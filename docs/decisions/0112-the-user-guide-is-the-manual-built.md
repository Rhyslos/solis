# 0112: The user guide is the manual, built

- **Status:** Accepted (the project lead asked for it, 2026-11-14; the six points below are the recommended choices, taken by the lead reviewer while the project lead was away, and can be changed)
- **Date:** 2026-11-14

## Problem

The project lead asked for a **Documentation** button in the top bar that opens the documentation as an HTML page, in a browser the user chooses on Windows or the default one, looking like a reference guide of their own (a top bar with a search, a contents list with how much is read, boxes, numbered steps, keys drawn as keys, a link on every heading, a Top button), and said that the guide must be updated with every change so it is always accurate, which is "vitally important".

Solis already has a user manual (`docs/manual`, 55 pages) that every pull request must update. A second set of words would drift from it.

## Options

1. **A guide written by hand beside the manual:** two texts to keep in step, and nothing to notice when they part.
2. **The guide built from the manual** (recommended): the manual stays the one source; a small tool turns it into the guide; a check fails when they differ.
3. **The manual's pages shown on GitHub:** needs the network, looks like GitHub, and has no search across pages.

## Decision

**Option 2**, in six points:

1. **The guide is built from the user manual,** the single source. `tools/user-guide`, a small Rust program of the workspace with no libraries, turns `docs/manual/*.md` into **one self-contained HTML file**: style, script, fonts and logo inside it, nothing from the network, working offline from the disk. The chapters' order and groups come from the manual's contents page, `docs/manual/SUMMARY.md`, which already listed every page and now has a `##` heading per group.
2. **The look** is the reference guide's layout and behaviour in Solis's identity: a top bar (logo, name, "User guide", where you are, a search opened with `/` or `Ctrl + K`, a light, dark or automatic theme), the contents on the left (numbered chapters in their groups, how much is read, the sections of the chapter being read), and in the text "Good to know" and "Careful" boxes, numbered steps, keys drawn as keys, a `#` link on every heading and a Top button. Dark is the editor's own greys and accent (`tokens.rs`, held by a test) with Inter and JetBrains Mono; light is warm paper. The program's name comes from `branding.rs`. The manual's convention for keys (`` `Ctrl + K` `` in backticks), boxes and steps is written in `docs/manual/_template.md`.
3. **It ships with the program** as `user-guide/index.html` beside the program (the Windows build, decision 0010, and the installer carry it), and a copy is kept in the repository as `docs/user-guide/index.html`.
4. **The Documentation button** is at the right of the tab bar, beside the project's name, on every tab and the File pages and with no project open, drawn as the tab bar's text buttons are. It is a click only (no new key, rule 17). It opens the shipped guide in the chosen browser; if the guide isn't there (a build of one's own), the status bar says so and the Console says how to make it. File › Learn, File › About and a component's help in Properties open the guide too, so the editor has one way to its documentation.
5. **Which browser** is a preference, File › Preferences › General › **Open the guide in**: the default browser (the standard), every browser Windows lists as installed (its `StartMenuInternet` list, read with `reg.exe` and treated as untrusted), or a program the user picks. A browser that is gone falls back to the default, with a note in the status bar. On other systems only the default exists.
6. **Always accurate:** a test in the normal suite (`tools/user-guide/tests/manual.rs`) fails when the kept guide isn't what the manual builds, when a page of the manual is missing from its contents, or when a link in the manual goes nowhere; another (`crates/editor`) fails when the keyboard shortcuts page names a key for an editor action that the editor doesn't bind. CI runs them on every pull request, documentation-only ones too. CLAUDE.md and CONTRIBUTING.md say: the user guide is the manual, built; every change a user would notice updates the manual in the same pull request, and the guide is generated again (`cargo run -p user-guide`); the check fails otherwise.

## Consequences

- Writing the manual is writing the guide: no second text, and a forgotten rebuild fails the checks with the command to run.
- Every pull request that changes the manual also commits the rebuilt `docs/user-guide/index.html` (about 2 MB, most of it the fonts, which don't change; Git folds the file away in pull requests' changes).
- The manual's links are now checked: a link to a page or heading that isn't there fails.
- The online manual on GitHub is no longer what the editor opens; it stays readable there as before.
- A new kind of thing in the manual (a picture, say) needs the tool to know it; until then it stays words.
