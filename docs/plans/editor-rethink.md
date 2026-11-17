# The editor rethink (Shared 1f)

The project lead's review of the remade interface (2026-11-17), the answers they gave, the approved mock-ups, and the six milestones that build it. Decided in [decision 0118](../decisions/0118-the-editor-rethink.md). It comes before Tools 1b (the ground), which waits.

## Why

The remake (Shared 1a to 1e, pull requests 236 to 242 and 252) was "a step in the right direction, but not great". The project lead's nine points:

1. **The launcher is a page inside the editor, not a launcher.** The launcher should organise and show projects; opening one opens the engine.
2. **The main UI is messy and hard to understand at a glance.** The layout options in Preferences don't show what you're switching to, so you have to go in and out.
3. **An assets folder is made automatically, and assets can only be seen on Home's strip.** A full library browser is wanted on Home too.
4. **The bottom bar is confusing and too big.** Collapsing it helps, but doesn't fix it.
5. **"Area" isn't clear,** and there's no reusable thing of its own, made once and placed many times.
6. **A new project doesn't make its own folder.**
7. **Double-click doesn't rename, and right-click doesn't create.**
8. **The bottom bar changes how the asset browser behaves,** and a 2D game sees every 3D-only part.
9. **The panels are divided differently on each tab,** and not every edge can be dragged.

## What was chosen

Every answer came from the project lead, 2026-11-17.

- **The launcher is its own window.** One install: starting the program opens a small launcher window (Projects, Learn, Updates). Opening a project closes it and opens the editor window. File › Close project goes back to the launcher.
- **"Area" becomes "Level", everywhere:** the interface, the manual, file names and game code. Saved projects are upgraded when they open (rule 23). Game code that uses the old name still works for a while, with a note saying what to change.
- **A reusable thing is a "Prefab".** Every thing placed in a level is still an *entity*. A prefab is an entity, with its parts and settings, saved to be placed many times, and each placed copy is an entity linked to it. ("Entity" was the first wish, but Solis already uses it for everything placed in a level, so the project lead's second choice, Prefab, was taken.)
  - **A prefab has its own page:** New prefab, or opening one, puts it in its own tab, where it's built on its own, as a level is.
- **A project is 2D, 3D or both.** This is asked when it's made and can be changed in Project settings. A 2D project hides every 3D-only tab, tool and setting.
- **The bottom panel runs only under the view,** so Level and Properties stay full height.
- **Save and Build leave Home's ribbon.** Save stays on `Ctrl + S`, the File page and the status line; Build stays on the Build tab.
- **The foot has two thin lines:** when collapsed, the bottom panel's line (its names and counts), then the status line.
- **The left side can show the Level panel or the folder tree**, switched with a toggle. **Several folder trees can be open side by side**, to move files between them.
- **Double-click renames, and right-click opens a menu to create things**, in the Library and the Level panel. The project lead asked for both (rule 17). No other new keys; New folder and Import get keys only if asked later.
- The kind reads "2D", "3D" and "2D + 3D" on launcher cards and in the filter. In the New project dialog it's "Both", explained as "2D and 3D".
- The Library tab shows the same browser as Home's bottom panel, with its own folder tree inside. The Library tab's left slot stays empty, so the two places never differ (point 8).

## The approved mock-ups

In [`docs/design/ui-rethink/`](../design/ui-rethink/README.md), approved on 2026-11-17 with the two additions above:

| Board | Shows | Points |
| --- | --- | --- |
| 01 Launcher | The launcher as its own window | 1 |
| 02 New project | The name makes the folder; Kind 2D, 3D or Both; templates | 6, 8 |
| 03 Home | One calm frame: Level, the view, Properties, and the Library under the view | 2, 3, 4, 9 |
| 04 Library | The project's own folders, right-click to create, double-click to rename | 3, 7, 8 |
| 05 Bottom panel | Collapsed to one thin line, and open, beside today's | 4 |
| 06 Layouts | Preferences › Layout as drawn miniatures, applied at once | 2 |
| 07 Prefabs | Make prefab, its card, linked copies, Open and Unlink | 5 |
| 08 2D project | 3D-only parts hidden | 8 |
| 09 Every tab | One frame (left, centre, right, bottom), every edge draggable | 9 |

## The milestones

One pull request each, in this order. Each builds against the mock-ups, updates the manual and the guide, and keeps the editor fast (decision 0108).

| # | Milestone | Gives | Points |
| --- | --- | --- | --- |
| R1 | **The launcher and new projects** | The launcher as its own window, and the editor window opening a project. New project makes its folder, in a remembered parent folder. Kind 2D / 3D / Both is saved in the project. Templates. Close project goes back to the launcher. | 1, 6 |
| R2 | **Level, everywhere** | "Area" becomes "Level" in the interface, the manual, file names, the Lust files the editor writes, and game code. Saved projects are upgraded on opening, with an upgrader and fixtures (rule 23). Old names in game code still work with a note, until a set version. | 5 |
| R3 | **One frame on every tab** | The frame: left, centre, right, and a bottom panel under the view. Every edge draggable and remembered. The same division on every tab. The bottom panel holds Library, Console and Problems as small tabs, and collapses to one thin line above the status line. The command line shows only on the Console tab. Layouts in Preferences as drawn miniatures, applied at once with Undo. The ribbon tidied (Save and Build off Home). | 2, 4, 9 |
| R4 | **One Library browser** | One browser, the same on Home and the Library tab. The project's own folders, with no forced assets folder (and an existing one left alone). Breadcrumb, grid or list, kind filter, search, thumbnails per kind. Right-click on empty space or an item to create, rename, duplicate, delete or show in Explorer. Double-click to rename. Drag to move. The left side's toggle between the Level panel and the folder tree. Several folder trees side by side to move files. | 3, 7, 8 |
| R5 | **Prefabs** | Prefab as its own asset with a format, version and fuzz target. Make prefab… from picked entities. A prefab's own page (its own tab). Placed copies linked and marked, with changed settings shown and resettable. Properties shows "Prefab: Chicken · Open · Unlink". Editing the prefab updates every copy. Game code reaches prefabs through the Lust file the editor writes (decision 0065). | 5 |
| R6 | **2D projects, and the gate** | A 2D project hides every 3D-only tab, tool and setting (the project's kind from R1). Speed checked. The guide's pictures taken again. The gate. | 8 |

## As built

### R1: the launcher and new projects

> **Built** (branch `rethink/r1-launcher`) as boards 01 and 02 draw it, with these choices:

- **One window, two places.** The launcher and the editor are drawn in the one native window, which takes the launcher's size and place (1100 × 700 at first) while it shows and the editor's while a project is open, each remembered on the computer (`launcher_place`, `editor_place`; the editor's earlier `editor_size` is read once). Two real windows were weighed: each would need its own surface on the graphics card and its own copy of what the editor uploads there, and closing the first window eframe opens ends the program, so the launcher's would have to stay open and hidden while the editor works. One window keeps one GPU device, one sound card and one download, with nothing hidden; the window test checks both sizes.
- **Closing the editor's window closes the project** and comes back to the launcher (asking about unsaved work first), as File › Close project does; closing the launcher quits. The editor can't be opened without a project any more ("Carry on without a project" went).
- **File** keeps Back to editor, Save all, Project settings, Preferences, Plugins (unfinished), About (its "Go to Updates" became **Check for updates**), the new **Close project**, and Exit. The status bar's "Build N is out" asks the update question (the launcher's Updates has the notes). The search lost New project, Open project, Recent projects, Learn, Updates and Make the … (the launcher's); **Add the sample area** stays in the search, since it needs an open project (Learn's row for it went).
- **The card's ⋯ menu** is Open, Show in Explorer and Remove from list as drawn, then, under a line, what the launcher before it could do: Pin to the top, Rename… and Make a copy… (nothing became unreachable). A missing folder's card keeps Locate… and Remove. The build that last opened a project is in the card's hover text; arrows and Enter walk and open the cards as before. **Open folder…** opens a project's folder, and looks through any other folder for projects (what Scan a folder… did).
- **A card's picture** is its kind's (a field of tiles, a block, both): projects keep no picture of their last view yet (`recent.card.thumbnail` stays an unfinished part with nothing drawn).
- **Starters** are the empty project and Learn's four examples (all 3D, so greyed for 2D). A sample starter is made under the name typed and the kind chosen. The design's First-person, Top-down and Platformer templates aren't shown (none exists); their unfinished parts stay in the table with nothing drawn.
- **Create in** is remembered when a project is made (not on Browse… alone); the first is `Documents\Solis projects` (`branding::projects_folder`).
- **The kind** is an optional `kind` key in `project.solis` ("2d", "3d", "both"; any case read; anything else refused with why), read as both when missing, like `saved_by`. An older build passes it by and keeps it when it writes the file, so the format's version stays 1 (rule 23 needs no upgrader): a fixture (`tests/fixtures/project/v1/kind-2d.solis`) and a fuzz test cover it, and the fuzz target `project_file` reads it.
- **No forced assets folder:** neither making nor opening a project makes `assets/`. What depended on it: the watcher (the project's own folder is watched, flat, until `assets` appears, then the watcher moves into it), the savers and new files (`Projects::place`, new folders and sound events, imports' `copy_in` make it first), the area's Save and Open windows (they start in it, so it's made then), and the engine's description files for game code (written once it's there). Exports need a saved area, so they need it anyway.
- **A project's folder named on the command line** (`solis <folder>`) opens the editor straight away, as `--sample` does; the window test uses it.

**Gate:**
1. Start the program: the launcher opens.
2. Make a new 2D project by its name; its folder is made.
3. It opens with no 3D-only parts.
4. Arrange the panels by dragging every edge; change the layout from its pictures.
5. In the Library, right-click to make folders, a level and a prefab; rename one by double-clicking; move files between two folder trees.
6. Build a prefab on its own page, place three copies, change the prefab, and see all three change.
7. Close the project: you're back in the launcher.

## Still to settle (asked when its milestone starts)

- R2: the version after which game code's old names stop working.
- R5: which settings a copy may change without unlinking (all, or chosen ones), and what happens to a copy's changes when the prefab changes the same setting.
