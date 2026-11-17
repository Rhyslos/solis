# 0118: The editor rethink: a launcher of its own, Level and Prefab, one frame, one Library

- **Status:** Accepted (the project lead, 2026-11-17)
- **Date:** 2026-11-17

## Problem

The project lead reviewed the remade interface (Shared 1, decision 0099) and listed nine points ([the plan](../plans/editor-rethink.md#why)):
1. the launcher is a page inside the editor;
2. the main interface is messy, and its layouts can't be previewed;
3. an assets folder is forced on the project, and assets can only be seen in one place;
4. the bottom bar is confusing and too big;
5. "Area" is unclear, and there's no reusable thing of its own;
6. a new project doesn't make its folder;
7. double-click doesn't rename, and right-click doesn't create;
8. the asset browser behaves differently when the bottom bar is open, and 2D games see 3D-only parts;
9. the panels are divided differently on each tab, and not every edge drags.

## Options

Each question was asked with a recommended answer and alternatives:
- **The launcher:** its own window, its own program, or its own window that stays open.
- **Area's new name:** Level, Scene, Map or World.
- **The reusable thing's name:** Prefab, Template or Object. The lead also offered "Entity", which clashes with the entity every placed thing already is.
- **2D projects:** a project setting, a 2D tab, or both.
- **The bottom panel:** under the view, full width, or left to the layouts.
- **Save and Build** on Home's ribbon or not.
- **The foot:** two thin lines, or one merged line.
- **The rename:** everywhere, or only what's seen.
- **The order:** the rethink before the ground, or after.

## Decision

- **The launcher is its own window.** One install. Opening a project closes the launcher and opens the editor window, and Close project goes back to it.
- **A new project makes its own folder** from its name, in a remembered parent folder. The project is **2D, 3D or both**, asked then and changeable in Project settings. A 2D project hides every 3D-only part.
- **"Area" is "Level", everywhere:** the interface, the manual, file names and game code. Saved projects are upgraded on opening (rule 23), and game code's old names keep working with a note for a set time.
- **Prefab is a kind of asset:** an entity saved to be placed many times, with its own page (its own tab). Placed copies are entities linked to it.
- **One frame on every tab:**
  - left, centre and right, with a bottom panel under the view;
  - every edge draggable;
  - the bottom panel holds Library, Console and Problems, and collapses to one thin line above the thin status line;
  - layouts are chosen from drawn miniatures;
  - Save and Build leave Home's ribbon.
- **One Library browser,** the same everywhere:
  - the project's own folders, with no forced assets folder;
  - right-click to create, double-click to rename (both asked for by the project lead, rule 17);
  - the left side switches between the Level panel and the folder tree;
  - several folder trees can be open side by side.
- The approved mock-ups are in `docs/design/ui-rethink/`. They amend the G layout (decision 0099) wherever the two differ.
- **The rethink comes before Tools 1b.** It is built as six milestones, R1 to R6 ([the plan](../plans/editor-rethink.md#the-milestones)).

## Consequences

- The launcher code moves out of the editor's File pages into a window of its own. The app opens one window or the other.
- The area format is renamed, with a version raise, an upgrader and fixtures. Game-code calls are renamed, with the old names kept as aliases for a while. The manual and the guide change throughout.
- Prefabs add an asset format (version, fuzz target), linking, and changed settings on copies.
- The G layout's screens stay the lead's own design. Where the rethink differs, the mock-ups in `docs/design/ui-rethink/` and this decision win.
- Tools 1b and the Open worlds phase move after the rethink in ROADMAP.md.
