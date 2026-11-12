# 0105: Solis starts on the File page

- **Status:** Accepted (the project lead, 2026-11-11, answers 22 and 23 of the Shared 1 plan; replaces [0090](0090-always-through-the-launcher.md)'s small launcher window and keeps its two promises; carries out [0099](0099-the-g-layout.md)'s File tab)
- **Date:** 2026-11-12

## Problem

Decision 0090 made Solis start on the launcher, in a small window of its own (1100 × 700) that grew to the editor's size when a project opened, and came back to it when the editor's window was closed. The project lead's design (the G layout, Screen 7) draws Recent projects as a page of the File tab, in the editor's own window, with Learn, Updates and About as pages beside it. The launcher had to become that page, and the question was whether its small window should stay.

## Options

1. **The design's page at the editor's size** (recommended, `09-recent-projects-start`): Solis opens its full window on File › Recent projects; the other tabs are dimmed until a project is open; "Carry on without a project" stands where Back to editor does.
2. **Keep the small size for the page** until a project opens, then grow, as today.

## Decision

**Option 1.** The project lead chose it on 2026-11-11 (answer 23), and with it the templates of `09-recent-projects-a` (answer 22).

- **Solis starts in its full window, on File › Recent projects**, every time, at the size the window was last left (remembered on the computer), centred. The small launcher window and the growing are gone.
- **0090's two promises stay:** the projects page is the first thing shown at every start, and no project is reopened by itself. Closing the editor's window with a project open still comes back to Recent projects, the project open behind it; closing the window while a File page is in front, File › Exit and Restart to update quit, asking about unsaved work first.
- **Until a project is open** the other ribbon tabs are dimmed (they still work: a click carries on without a project), the tab bar says "No project open", and Back to editor reads "Carry on without a project".
- **The projects are cards** (pin, the build that last opened each, a folder that's gone with Locate… and Remove, a right-click menu); all four arrow keys walk them and Enter opens the one picked (answer 25).
- **Templates:** the blank project and Solis's four examples are real (each makes a project in a folder you choose); the design's First-person, Top-down and Platformer, and All templates, are unfinished parts. VR / XR is left out (Solis is for Windows desktops).
- **Learn, Updates and About are File pages.** A newer build is no longer a bar over the projects: Updates is marked **New**, the status bar says "Build N is out" (a click goes to Updates), and Updates has the notice with Not now and the next step. About replaces the small About window, with the licence and the libraries' notices.

## Consequences

- One window size to remember instead of two; the app no longer resizes the window when a project opens.
- The launcher's own code (`launcher.rs`, 1,569 lines) is gone; its state and helpers live in `crates/editor/src/file_pages/`, split by page, each file under 500 lines.
- Saved settings are unchanged: the list of known projects keeps its format, and the editor's remembered size is the same key.
