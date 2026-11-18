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
- **"Area" becomes "Level", everywhere:** the interface, the manual, file names and game code. Saved projects are upgraded when they open (rule 23). Game code that uses an old name stops working right away, with a mistake that names the new one (the project lead, 2026-11-17: "Right away").
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
| R2 | **Level, everywhere** | "Area" becomes "Level" in the interface, the manual, file names, the Lust files the editor writes, and game code. Saved projects are upgraded on opening, with an upgrader and fixtures (rule 23). Old names in game code stop working right away. | 5 |
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


### R2: Level, everywhere

> **Built** (branch `rethink/r2-level`), with these choices:

- **Only the concept changed.** The `area` crate is `level`, and every type, function, file and word named for the thing you build and play follows (`Level`, `LevelFile`, `level_panel.rs`, the Level panel, **New level**, **Add the sample level**, the panel layouts **Level left, Assets bottom** and the rest, the manual's [Saving levels](../manual/saving-levels.md)). Words for a region of the screen or a shape's size keep "area": egui's `Area` and `ScrollArea`, the canvas's **Safe area**, a rectangle's `area()`, the mesh's faces with no area, a tile map's copied area, and area lights in the plans. A check (`crates/editor/src/level_words_tests.rs`) reads every string in the program's code, the manual and the guide's captions, and allows "area" only where its list says why.
- **Levels are format 4** (`level version 4;`), the same layout as format 3 under the new word; both words are read. Assemblies share the format, so only their number moves.
- **Opening a project renames its areas** (`app::older_files`, beside the move to Lust): `x.solarea` becomes `x.sollevel` with only its first line (and the first comment) changed, checked to read exactly as the area did, its sidecar moved, the area kept in the backups. One that can't move stays and still opens as a level (the Library, the Open window and the check still read `.solarea`). New projects' `.gitattributes` names both endings.
- **The project file is format 2**, though nothing in it changes: it says the project's levels are `.sollevel` files, so a build from before refuses the project ("made by a newer version") instead of opening it with no levels. It's saved through the backups once its areas have moved.
- **Everything that names a level** finds it under its new name: recordings are format 4 (`level` line), playtest files format 2 (`level` line; now a format with an upgrader and examples, `tests/fixtures/playtest/`), save games format 9 (`level` section). Saved panel layouts and sizes keep their places (old names and keys still read). The fixtures' folder for levels keeps its name, `tests/fixtures/area/`, since released examples never move.
- **Game code had nothing named for areas** (`docs/game-code-interface.txt` has no such call, type or file), so the project lead's "right away" needed no refusals: nothing that built before stops building. Only two descriptions of `World:find` and `entity:name` (the Level panel's order) changed.
- **Pictures:** the approved pictures and the guide's keep what they show until R6 (no picture test needed them taken again); their names and captions follow (`home-level-*.png`, `level-list.png`).

### R3: one frame on every tab

> **Built** (branch `rethink/r3-frame`) as boards 03, 05, 06 and 09 draw it, with these choices:

- **The frame** is one table per tab (`shell/workspaces.rs`: `left_slot`, `right_slot`, the groups, `bottom_tabs`) placed by one function (`shell/layouts.rs`, `place`). Home: Level, the view, Properties, Library · Console · Problems (· Map with a tile map selected). Library: no left slot, the browser, **Details** (Properties), Code · Console · Problems, folded at first so the browser fills the tab. Animate: **Clips** and/or **Parameters** left (both stacked when both files are open), the groups as before, **Properties** right while an animator is open, **Frames** (the timeline) · Clip · the unfinished Animation, Sequencer and Key · Console below. UI: **Elements**, the canvas, **Properties**, Library · Pictures · Console · Problems. Audio: **Sound events**, the mixer, **Bus** (its Ducking/Effects switch moved into the slot's header), Library · Snapshot · Console. Debug: **Running** is a new unfinished part (`debug.running`), since no such panel exists; the board's **Statistics** are the Profiler's own, already beside its timeline, so the right slot stays empty; the Profiler and Game in the centre; Console · Problems below. Build: the Export page, Console · Problems · Build log. A tileset's tab: **Terrains** left, **Slot** right; a picture's: **Crops** left. Shading, VFX and the model viewer keep their page, their bottom line of unfinished tabs now 30 px.
- **Where the old bottom bar's tabs went:** **Place** is the Library's **Place** switch (a chip in the Library's bar on Home, or in its header where a layout puts it beside the view; **Start placing** in the search), and placing shows **Placement** beside the files (over them in a column); **Entity** went (its three lists are slots); the unfinished **Animation**, with its transport and frame counter, went to the Animate tab's bottom panel; **Details**, **Screen**, **Sound events**, **Parameters**, **Properties**, **Terrains** and **Crops** became slots; **Code**, **Console**, **Problems**, **Map**, **Frames** and **Clip** stayed tabs. `shell_reach_tests` proves each panel reachable in every layout and what Home gave up reachable where it went.
- **Edges:** sizes are shares of the middle (`Edge::Left`, `Right`, `Bottom`; the stacked shares and the split as before; the placing column in points), so the window's size keeps them in proportion. Least sizes: 180 points a slot, the centre 240 × 120, the bottom panel 80 under its bar. The three frame edges carry a 28 × 4 grip and take the accent while pointed at or dragged. **Each tab keeps its own panel sizes** (`InterfaceLook::sizes_per_tab`, on) shares the frame's edges under Home when off; the split and placing column stay each tab's.
- **Hiding:** each slot's header has **⋯** (its panel's own items, the Level panel's moved there from its header, then **Hide**). Hidden slots are per tab (`Arrangement::hidden`) and come back from icons on the bottom panel's bar, **Show hidden panels** in the search, showing the panel, or choosing a layout.
- **The bottom panel's line** is 30 px, folded or open (the board's open bar is near enough; the old 34 px shelf bar and grip went). Counts: the Console's warnings and errors (amber and red marks), Problems' count (a dot, red while the code has mistakes). The newest warning or error is kept by the Console as it's pushed, so drawing the line walks nothing. The status line lost its problem counts (they're the Problems tab's) and gained the saved part (a click saves) and what's selected.
- **Layouts:** seven, three of them decision 0109's renamed (Standard, Library on the left, Library on the right; their saved names and size keys kept so builds before still read them), four new: Wide view, **Two views** (the standard frame with the viewport's existing side-by-side views, set on choosing and set back to one on leaving), Tall properties, Just the view (every slot hidden and every bottom panel folded on every tab when chosen). The shelf layout reads as Standard. Choosing one shows every hidden panel again; its Undo puts back the layout, every tab's arrangement and the views.
- **Saved settings:** the format keeps its shape, so no version (as R1's `kind`): the bottom tab and fold keep the shelf's keys, `hidden` is new, and the sizes are under new edge names; the shelf's sizes in points (`shelf_height`, `split`, `left`, `right`, `strip`, `shelf`, `column Place 1`) are read once into their nearest edges as a 1440 × 900 window had them and never written again. Fixture `tests/fixtures/editor-settings/before-frame/side-layouts.ron` (as R2 wrote it), read by the app's test and, as a string, by the editor's; the frame's reader is fuzzed with both.
- **Home's ribbon** (board 03): Edit (Undo and Redo as two small rows), Create (**Entity**, was Empty; Model, Light, Camera, Shape, and **More** holding Sprite, Tile map, Reverb zone, Assembly from selection and the unfinished Trigger zone), the snaps, Lighting; Save and Build left it.

### R4: one Library browser

> **Built** (branch `rethink/r4-library`) as boards 03, 04 and 09 draw it, with these choices:

- **One browser** (`editor::library`, split into `browser.rs`, `shown.rs`, `tiles.rs`, `menus.rs`, `rename.rs`, `tree.rs`, `trees.rs`): the Library tab's centre and Home's bottom panel (and UI's and Audio's, and the Library column of the side layouts) call the same function on the one `LibraryState`, so the folder, back, search, kind filter, view, what's picked and the name being typed are the same in both. Its tools (search, **All kinds ▾**, grid or list) are drawn by one function in the browser's own header on the Library tab ("Library" and the project's name, board 04) and in the bar or header above it elsewhere (board 03). Only placing (Home's **Place** switch) shows the placing form instead, as R3 left it. The Library tab's left slot stays empty and **Details** is its right slot (R3's table).
- **The project's own folder.** `asset_db::Project::files` is the project's folder for every project made from now on: nothing makes `assets/` (imports' `copy_in`, `Projects::place`, new folders and sound events, the level's Save and Open windows, the samples and templates, exports and the playtest check all use it). Solis's own things are left out of the asset database, the watcher, the Library and every save or move (`asset_db::is_own`: hidden folders such as `.solis` and `.git`, `builds`, `saves`, `recordings`, `project.solis` and the settings files beside it). The watcher (`Watcher::new_project`) watches the folder flat and each of its own top folders whole, so the cache, Git's history and builds are never watched; events that only read a file are passed over.
- **An existing `assets` folder.** A project whose top holds only its `assets` folder and Solis's own things (every project from before) keeps `assets` as its files folder, left exactly as it was (`asset_db::files_folder_of`, decided on opening): game code, sprites' clips, tile maps, animators, assemblies, sound events and screens are all named by their path in that folder, so moving the Library's top above it would rename every one and break the game's code. A project with files of its own beside an `assets` folder shows `assets` as an ordinary folder. Moving an older project's files up is left for the project lead to decide (it would rename everything game code names).
- **The engine's `solis/` folder is hidden** (not shown read-only): nothing in it is the person's to change, it's written again whenever the project changes, and the manual says it's there in Explorer.
- **Tiles per kind:** folders, and each kind drawn in its colour with its dot and word; pictures and models show their thumbnails, sounds their waveform, sprite sheets their first frame (as before). **Levels draw their kind for now**: no level thumbnail exists yet, and making one means drawing the level on the GPU on a worker; left for R6's look at speed or later. Material and Prefab aren't in the filter (neither exists); New prefab is an unfinished part until R5.
- **Right-click** as approved: on empty space *New in "<folder>"* (New folder, New level, New prefab (unfinished), New code file, New sound event…, and **More** for New tile map, New tileset and New animator, which were on the old New menu, so nothing became unreachable), Import…, Paste (an unfinished part: no file is copied today, so no `Ctrl + V`), Show in Explorer; on a file Open and/or Place (as its kind offers), its kind's own (Atlas Crop…, the model viewer, Play), Rename (`F2`), Duplicate (new: a copy beside it, `hay copy.png`, given an ID of its own, `AssetDb::duplicate`; folders can't be duplicated yet), Delete (the Recycle Bin), Show in Explorer; on a folder Open and *New in "<folder>"* too; on several picked Duplicate, Delete and Show in Explorer (when they share a folder).
- **Several picked:** `Ctrl` and a click adds or takes away, `Shift` and a click picks a range: the design's own (spec 327, `lib.multi-select`, now built), as the Level panel does already. No other key was added.
- **Renaming** in place (`rename.rs`): double-click a **name**, `F2`, or the menu. The tile's response knows where the pointer was: a double-click inside the name's line renames, anywhere else on the tile (its picture) opens; on a list row, the name's words rename and the rest of the row opens. In Home's folder trees a double-click opens (a file) or makes the folder the tree's own, and renaming is `F2` or the menu. The name is picked out when the box opens. A file keeps its ending (so its kind never changes). A bad name (empty, a dot first or last, `< > : " / \ | ? *`, a control or text-direction character, a device's name, over 100 characters, or taken in the folder in any letter case) is said in red under the box while typed; kept anyway, the old name stays and the Console says why. Renaming and moving go through `AssetDb::move_path` as before, which moves the sidecar, so IDs stay.
- **New … starts in renaming:** the Library picks what's made and starts renaming it as soon as it's listed (`Workbench::show_new_in_library`, `LibraryState::rename_new`): folders and sound events (named by the app), levels and code files, duplicates. **New level and New code file no longer open what they make** (opening a level switches the document, and a code file opens in another program, before it has its name); the first code file, `main.lst`, keeps its name and opens as before. A tile map, tileset or animator opens its own tab as before.
- **Drag to move:** tiles and rows onto a folder tile, a folder in a tree, a part of the breadcrumb or a tree's header; everything picked goes when one of them is dragged. Copying files didn't exist, so `Ctrl` doesn't copy. **Moves aren't undoable** (they weren't before); the manual says to drag back. Dropping onto the view still places.
- **Home's left side:** the slot's title is a toggle, **Level · Folders** (`LibraryState::left`, `OpenNow::folders`); `Panel::Folders` is new, and showing either panel switches the slot to it. The choice is kept while the editor runs, not saved.
- **Several trees** (`trees.rs`): one tree at first; **Open another tree** in a tree's ⋯ menu (or the slot's ⋯ menu) adds one beside it in the same slot, showing the same folder; up to four. Each shows its own folder's folders and files, unfolding in place; double-click a folder to make it the tree's own, **˄** goes up, **Show the project's folder** and **Close this tree** are in its ⋯ menu. The edges between trees drag (each keeps at least 15 %). Their folders are kept for the session (`LibraryState::trees`). No mock-up was made: built from the description, pictured in the report.
- **Speed:** the browser, its tree, the list and Home's trees draw only the rows in view (`show_rows`), and thumbnails are made on workers as before; a scale case draws a folder of 5,000 files (`library_scale_tests.rs`).

**Gate:**
1. Start the program: the launcher opens.
2. Make a new 2D project by its name; its folder is made.
3. It opens with no 3D-only parts.
4. Arrange the panels by dragging every edge; change the layout from its pictures.
5. In the Library, right-click to make folders, a level and a prefab; rename one by double-clicking; move files between two folder trees.
6. Build a prefab on its own page, place three copies, change the prefab, and see all three change.
7. Close the project: you're back in the launcher.

## Still to settle (asked when its milestone starts)

- R5: which settings a copy may change without unlinking (all, or chosen ones), and what happens to a copy's changes when the prefab changes the same setting.
