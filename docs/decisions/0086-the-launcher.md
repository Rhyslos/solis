# 0086. The launcher: projects and updates inside Solis

- **Status:** Accepted (the project lead, 2026-11-05; grows the start page of decision 0070; builds on 0043 and 0045). Changed by [decision 0099](0099-the-g-layout.md): the launcher becomes the File tab's Recent projects, Learn and Updates pages in Shared 1b; everything it does stands.
- **Date:** 2026-11-05

## Problem

The start page (decision 0070) is a box over the editor with New project, Open project, eight recent projects and the three examples. People with several projects need more: finding one among many, keeping the ones in use at the top, tidying the list, renaming or copying a project, finding projects already on the disk, and seeing which version of Solis each was last saved with before opening it. Updates are only in the Help menu and on the splash screen, with no place to read what a new build brings.

## Options

- **Inside Solis** (chosen): the start page grows into a full page of the editor, like Godot's Project Manager. One program to install and update; the launcher is what Solis shows when no project is open, and File > Projects… brings it back.
- **A separate launcher program**, like Unity Hub or the Epic launcher: it could hold several installed versions of Solis side by side, but it is a second program to install, update and keep in step, for a feature (several versions at once) nobody has asked for.

## Decision

- The launcher is a full page with a sidebar of three parts, as in the mock-up of 5 November ([plan](../plans/launcher.md)):
  - **Projects:** every project Solis knows, pinned ones first; search by name or folder; sort by last opened or name; a star to pin; double-click (or Enter) to open; ⋯ for Open, Show in Explorer, Rename…, Make a copy… and Remove from list; a project whose folder is gone says so, with Locate… and Remove; Scan a folder… finds projects already on the disk; New project and Open project….
  - **Learn:** the start page's three examples, its steps for someone new, and the manual.
  - **Updates:** this build, the newest build and what it brings, Check now, Restart to update, and whether to look when Solis starts.
- **Each project records the build that last saved it** (`saved_by` in the project file, an optional key, so older project files read as before). The list shows it: "this one", "older" (opened and updated as now) or "newer" (a warning: this Solis may not open it). A copy built from source has no build number and records nothing.
- **Remove from list never touches files.** Rename changes the project's name in its project file, not its folder. Make a copy copies the folder (without the import cache) to a new one the user picks.
- The list of known projects replaces the eight recent ones, kept with a pin and the time each was last opened (up to 200).
- Pictures of projects wait: a row shows the project's first letter in a colour of its own. A picture of the last view saved with the project can come later.

## Consequences

- The start page box goes; File > Start page becomes File > Projects….
- Release notes come from each build's release on GitHub: the release text names the pull request merged, instead of the merge's commit line.
- Built in three pull requests: the Projects and Learn pages; rename, copy, show, scan and locate; the Updates page.
