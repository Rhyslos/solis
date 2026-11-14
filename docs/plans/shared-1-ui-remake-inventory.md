# The G layout and Solis: the two-way inventory

Written 2026-11-11 for "Shared 1: The editor's UI, remade", from reading the code and the design before anything was built; brought into the repository with milestone M1. It is the companion of [`shared-1-ui-remake.md`](shared-1-ui-remake.md), "the plan" below. Where the code turns out to differ from a row here, the code wins and the row is corrected.

**Sources.** The design in [`docs/design/g-layout/`](../design/g-layout/README.md): the specification (`specification.md`, 1,192 lines; "spec 140-146" below means its lines 140 to 146) and the thirteen screen pictures. The mock-up's HTML, which is not in the repository: values marked **"from the HTML"** were read from its CSS and are written down here so nothing depends on the file. The engine as it stood on 2026-11-11 (`crates/editor`, `crates/app`, the manual, the decisions): file names with line numbers are from that day and drift as the phase goes on.

**The project lead's answers** are in the plan's section "What the project lead has decided"; this inventory was written after the first three and has been corrected for the later ones where a row said otherwise (the play tint, rows in a.1 and a.6):

1. **Keys:** today's keys stay (Q select, W move, R rotate, S scale, T all three; Space opens the search). The design's keys are used only for things that are new (Ctrl + Space folds the shelf).
2. **Themes:** Standard and Graphite are both removed; only the design's Neutral dark with its five accents remains; decision 0096 is replaced.
3. **Order:** the UI remake starts now as the one line of work, from `main` as it is after the review's "part 6" (script and world crates only, so nothing in this inventory moves).

**Words.** The names of the design's own elements (the Object shelf tab, the Scene and Inspector columns, the Mesh button) are written here as the design has them, so they can be found in the spec; what they are called on screen is the plan's decision D4, which lists every difference (settled 2026-11-11: Solis's words). The spec's sample words differ from Solis's. This inventory uses Solis's words and gives the spec's in brackets the first time: area (level, scene), entity (object), assembly (prefab), Properties (inspector), model (mesh), picture (texture), sound event (sound event; "audio" as a kind), game code (script), Lust (the sample code is in another language).

**Milestones** named M1 to M16 are the plan's.

**The three marks used in part (b):**

- **EXISTS**: Solis has the feature; the row names what drives it. "(shell)" after it means the element is pure interface that this phase builds itself and needs nothing new from the engine.
- **PARTLY**: part works today; the row says which part, and names the piece that becomes a placeholder.
- **PLACEHOLDER**: the engine has no such feature. The element is registered once (its id is the row's id), drawn by the one placeholder function, and hidden by **Hide unfinished parts**. "Filled in" names the roadmap phase from `ROADMAP.md`; "not on the roadmap" means no phase covers it and the lead must say whether it gets one (the plan, decision D12).

---

## (a) Every piece of today's editor UI, and where it goes

"Where it goes" names the ribbon tab, ribbon group, center group, shelf tab and column, File page, status bar slot or contextual tab. **NEEDS MOCK-UP** marks a place the lead hasn't designed: it is drawn and approved before it is built (CLAUDE.md, "UI is mocked up first").

### a.1 The top bar (`crates/editor/src/top_bar.rs`, 671 lines; drawn from `workbench.rs:880`)

| Today | Where it goes | Notes |
| --- | --- | --- |
| The logo (18 px, `Workbench::logo`) | Tab bar, left: the 20 × 20 logo tile (spec 132) | Solis's own logo from `branding/`, not the mock-up's cube. Decision D5. |
| Menu bar: File, Edit, View, Entity, Play, Window, Help | Gone as a bar. Each item is placed below (a.2). | The design has no menu bar at all. |
| Workspace tabs Level, Animate, Code, UI (`workspace.rs:332`) | The ribbon tabs: Level → **Home**; Animate → **Animate**; UI → a new **UI** ribbon tab (**NEEDS MOCK-UP**, a.6); Code → no tab of its own: its parts go to **Debug** (the game big, Console and Problems) and **Library** (code files) | Each ribbon tab is a workspace that remembers its shelf tab, shelf height and split (spec 1131-1138). |
| The area tab (name, dot when unsaved) and the tabs of an open animator or tileset (`document_tab`, `top_bar.rs:581`) | Document tabs of the center's editor group (spec 150, 182): `farm.area` with the amber unsaved dot (spec 124); an animator, tileset, atlas or screen is a document tab in its own tab's center | "Several levels open at once" (spec 182) is not Solis today: one area at a time. Row `shell.groups.several-areas` in (b). |
| Search box "Search or do anything…" (`search.rs`) | Tab bar, centre: the command search (spec 134) | The key hint shows **Space**, not the mock-up's hint (the lead's answer 1). |
| Restart to update button (`updates.rs:194`) | Status bar, right: the "Update available" slot (spec 616), which becomes **Restart to update** once downloaded; also File › Updates | |
| Play, Pause, Step one tick, Stop (`play_and_build`, `top_bar.rs:475`) | Ribbon, far right: the **Play** group (spec 145). Play is the one light button and turns into **Stop** while running (spec 1129). | **Pause** and **Step** have no place in the design: proposed as two small rows in the Play group while the game runs. **NEEDS MOCK-UP** (small). |
| The play tint (the window's edge and bars turn blue, amber when paused; `play_changes.rs`, decision 0085) | **Removed** (the project lead, 2026-11-11: no tint, as designed). Playing shows as Play turned into Stop, with Pause and Step beside it, and the Game tab in front. Goes with the top bar in M3 and the status bar in M4. | Decision D13, settled against the plan's first recommendation; decision 0099 records the change to 0085. |

### a.2 Every menu item (`top_bar.rs:105-470`, `menu_items.rs`)

| Menu › item (key) | Where it goes |
| --- | --- |
| File › New project… | File › **New project** (menu column item 3, spec 548) and the **New project** button on Recent projects (spec 567) |
| File › Open project… | File › **Open…** (spec 549) and the page's **Open…** button (spec 566) |
| File › Projects… (the launcher) | The **File** tab itself: File › Recent projects (Screen 7) |
| File › Recent projects › (list) | The cards on File › Recent projects |
| File › New area (Ctrl + N) | A **+** at the end of the viewport group's document tab strip, and command search. **NEEDS MOCK-UP** (small). |
| File › Open area… (Ctrl + O) | Double-click an area in the Library; command search; the **+** menu above |
| File › Save area (Ctrl + S) | Home › **Project** › **Save** (spec 177) and File › **Save all** (spec 550) |
| File › Save area as… (Ctrl + Shift + S) | Right-click the area's document tab › Save as…; command search. **NEEDS MOCK-UP** (the document tab's right-click menu: Save, Save as…, Close, Show in the Library). |
| File › Import model… | Library › **Add** › **Import** (spec 297); the **Import** tile in the shelf's Assets column (spec 216) |
| File › Export for playtesting… | The **Build** ribbon tab (undesigned, spec 1183) and Home › Project › **Build** (spec 177), which opens it. **NEEDS MOCK-UP.** |
| File › Quit | File › **Exit** (spec 557) |
| Edit › Undo, Redo (Ctrl + Z, Ctrl + Y) | Keys and command search; proposed: two icon buttons at the left of the Home ribbon in a small **Edit** group, their hover text naming the step ("Undo Move"). **NEEDS MOCK-UP.** |
| Edit › Duplicate, Delete, Rename (Ctrl + D, Delete, F2) | Right-click a row in the shelf's Scene column (the Area panel's menu today); keys; command search |
| Edit › Select all, Select nothing (Ctrl + A, Esc) | Keys; command search; the Scene column's right-click menu |
| Edit › Input actions… | File › Project settings › **Input** (Screen 13) |
| Edit › Settings… | File › **Preferences** (Screen 8) for the editor's page; File › **Project settings** (Screen 12) for the project's 2D page |
| View › Viewport tools; Frame rate and entity count | The viewport's **Show ▾** pill (spec 260) |
| Entity › Add entity | Home › **Create**: a fifth entry **Empty** in the long-press list of Mesh, and command search. **NEEDS MOCK-UP** (the Create group for Solis's kinds: Empty, Model, Sprite, Tile map, Light, Camera, Volume). |
| Entity › Add point light, Add spotlight | Home › Create › **Light** (click: point light; long press: the list; spec 174) |
| Entity › Add camera | Home › Create › **Camera** |
| Entity › Create assembly from selection | Right-click the entity in the Scene column; Library › Add › **Create** › Assembly from selection |
| Play › Play, Pause, Step one tick, Stop | The Play group (a.1) |
| Play › Load a save… | Debug ribbon: proposed group **Session** (Load a save…, Play and record inputs, Replay a recording…). **NEEDS MOCK-UP.** |
| Play › Play and record inputs; Replay a recording… | Debug › Session (as above) |
| Window › Reset layout | File › Preferences › Appearance › Layout: a **Reset this workspace** row, beside the design's **Reset to defaults** (spec 663) |
| Help › Open the manual ↗ | File › **Learn** (spec 556) |
| Help › Add the sample area | File › Learn |
| Help › Make the sample game…, UI sample…, benchmark…, playground… | File › Learn (the four examples, as on the launcher's Learn page today) |
| Help › Check for updates; "Check for updates when starting" (`updates.rs`) | File › **Updates** (a page added to the File menu column after Learn; **NEEDS MOCK-UP**, a.8); File › Project settings › General › Engine version › **Check now** (spec 958) |
| Help › About | File menu column, last block: **About** (proposed, with Updates). Shows the name from `AppInfo`, never written out. |

### a.3 The docked panels (`panel.rs` `Panel::ALL`, drawn by `panels.rs` `PanelViewer::ui`)

| Panel (file) | Where it goes |
| --- | --- |
| **Viewport** (`viewport.rs`, `viewport_tools.rs`, `gizmo.rs`, `camera.rs`) | Home › center, left group: the viewport document (spec 180-186). Also the "Viewport" document tab on Animate (spec 404). |
| **Game** (`game_panel.rs`) | A **Game** document tab in the viewport's group, there while the game runs (the pattern of spec 404, a camera tab beside "Viewport"). Its notes ("Click to give the game the mouse and keys", "The game has the mouse and keys · Esc gives them back", "Game speed", "No camera in the area…") become pills over it. The Mute button goes to the Play group's small rows. **NEEDS MOCK-UP** (small). Decision D14 (the spec says the game plays "in the viewport", spec 178). |
| **Area** (`area_panel.rs`): tree, search, fold and unfold all, hide, lock, × to remove, right-click Edit and Entity menus | Shelf › **Object** and **Place** › **Scene** column (spec 205, 270). Search, fold all and "show everything hidden / unlock everything" go in the column header as icon buttons; hide and lock stay on the row, shown on hover. |
| **Layers** (never shown, `Panel::NOT_BUILT`) | Removed from `Panel`. Not in the design. |
| **Properties** (`properties.rs`, `inspector.rs`): name, ⋯ entity menu, a card per component with ? and ⋯, fields with dot and ↺, + Add component…, assembly copies (Revert, Apply, Unpack), live values while playing | Shelf › Object › **Inspector** column (spec 206-214): header (icon, name, kind · parent, **+ Component**), cards that wrap. The ? (manual page) and ⋯ stay in each card's title row; the spec's right-click menu (spec 214) is added beside ⋯. |
| Properties showing something else: a sound (`sound_view.rs`), a sound event (`sound_event_view.rs`), a sprite sheet (`sheet_view.rs`), a text file (`text_view.rs`), a tileset's terrains and a map file (`map_tab.rs`), a bus (`mixer_view.rs`), an animator's state or transition (`animator_view.rs`), a screen's piece (`screen_props.rs`), day colours and Preview time (`day_colours.rs`, on the Environment's card) | Files picked in the Library: Library › shelf › **Details** (spec 316-322). A bus: Audio › shelf (a.7). Animator, tileset, map, screen piece: the shelf of their own tab (a.6). Day colours: stay on the Environment's card in the Inspector. |
| **Hints** (`hints.rs`) | Proposed: a last, quieter card in the Inspector column ("About this"), off with File › Preferences › General › Show hints. **NEEDS MOCK-UP** (small). |
| **Library** (`library.rs`, 1,665 lines): folder tree, breadcrumb, thumbnails or list, kind filter, search, + New ▾, Import model…, right-click (New, Place, Open, Show in Explorer, Rename…, Delete), drag to place or move, ▶ on sounds, Pin and Unpin | The **Library** ribbon tab (Screen 3) for managing; the shelf's **Assets** column (spec 207, 272) for placing. The folder tree has no place in the design (breadcrumb and folder tiles only): kept as a dropdown from the breadcrumb's current folder (spec 305). |
| The Library as a drawer (Ctrl + Space, `library_drawer.rs`) | Gone: the shelf's Assets column is always under the viewport. **Ctrl + Space now folds and unfolds the shelf** (spec 1078; the lead's answer 1). Pin and Unpin go. |
| **Code** (the Library filtered to code, with each file's uses: `workspace.rs` `code_uses`) | Library with the type chip **Code**; uses shown in Library › shelf › **Used by** |
| **Console** (`console.rs`, `console_commands.rs`): lines, Info / Warnings / Errors filters with counts, search, Clear, Copy, the command line (' jumps to it) | Shelf › **Console** tab on Home, Animate, Audio and Debug (spec 224-226, 506-519). Filters at the right of the shelf bar (spec 504). Search, Clear and Copy: icon buttons beside the filters. **NEEDS MOCK-UP** (small: the design shows no search or Clear). |
| **Problems** (`problems.rs`) | Shelf › **Problems · N** tab (spec 228-230); the status bar's counts open it (spec 164) |
| **Profiler** (`profiler.rs`) | **Debug** › center (Screen 6): frame-time graph, frame timeline, Statistics |
| **Mixer** (`mixer_view.rs`) | **Audio** › center, left group (Screen 10) |

### a.4 Panels that come with a file (`Panel::OPEN_WITH_A_FILE`): none has a place in the design

| Panel (files) | Proposed place, following the design's patterns | Mark |
| --- | --- | --- |
| **Map** (`map_tab.rs`, `tile_view.rs`, `tile_paint.rs`, `workbench_paint.rs`, `workbench_tiles.rs`): layers, terrains, cell values, the hovered cell; in the viewport: Brush, Rectangle, Line, Fill, Eyedropper, Art or Meaning, Values, Chunks, Walls, the copy bar (flip X, Y, turn Z) | A contextual tab **HOME TOOLS · Tile map** right after Home while an entity with a tile map is selected (the pattern of spec 138, 682). Ribbon groups: **Paint** (Brush, Rectangle, Line, Fill, Eyedropper; the design's own Paint group, spec 246), **Show** (Art / Meaning, Values ▾, Chunks, Walls; small rows), **Copy** (Flip, Flip, Turn), **Play**. Shelf opens on a **Map** tab: columns Layers, Terrains (tiles of min 92 px, like Place tiles), Cell values, Cell. | **NEEDS MOCK-UP** |
| **Tileset** (`tileset_view.rs`, `tileset_editor.rs`, `workbench_tileset.rs`): terrains, the picture and its grid, slots, templates, rules, the try-it patch | Contextual tab **LIBRARY TOOLS · Tileset** (opened from the Library, like the model viewer, Screen 9). Center: the picture (left group, about 60%), the chosen terrain's slots or rules (right group). Ribbon: **Terrain** (Add, Remove), **Kind** (dropdown, Save template…), **Picture** (Choose…, Tile size, Zoom), **Use** (Place a map with it). Shelf tabs: **Terrains**, **Slot** (the slot's tiles, weights, animation), **Rules**, **Try it**. Document tabs and **‹ Back to Library** as spec 695-698. | **NEEDS MOCK-UP** |
| **Atlas Crop** (`atlas_view.rs`, `atlas_editor.rs`, `workbench_atlas.rs`) | Contextual tab **LIBRARY TOOLS · Picture**. Center: the picture with grid and crops. Ribbon: **Crop** (Crop every tile, Delete crop), **Grid** (size), **View** (Zoom, Fit). Shelf: **Crops** (list, Name, Rename). | **NEEDS MOCK-UP** |
| **Clips** and **Frames** (`frames_view.rs`, `frames_editor.rs`, `workbench_frames.rs`) | The **Animate** tab, with a clips file as its document. Center: the sprite playing (left), the sheet's frames to add (right). Shelf: a tall **Frames** tab in the Sequencer's own shapes (spec 418-435): the Tracks column becomes the **Clips** list (+ Clip, search), the dope sheet becomes the frame ruler with events as clips on a lane below, the playhead as designed; a **Clip** tab for speed, direction, start and end. Transport in the shelf bar (spec 416) drives Play and Stop. Ribbon: the design's Playback group (Loop, speed), **Frames** (Add frames ▾, Delete), **Events** (Sound events ▾). | **NEEDS MOCK-UP** |
| **Animator** (`animator_view.rs`, `animator_editor.rs`, `graph_canvas.rs`, `workbench_animator.rs`) | The **Animate** tab, with an animator as its document. Center: the graph (left, about 75%) and the preview (right), exactly Shading's split (spec 348-370), since both use the one shared graph editor (rule 21). Ribbon: **State** (Add state, Make transition, Start here), **Graph** (Fit), **Preview** (Play, Start again). Shelf tabs: **Parameters** (cards), **State · name** or **Transition** (what Properties shows today), Console. | **NEEDS MOCK-UP** |
| **Canvas** and **Elements** (`canvas_view.rs`, `canvas_tools.rs`, `elements_view.rs`, `screen_props.rs`, `screen_doc.rs`, `workbench_screens.rs`) | A new ribbon tab **UI** between Animate and VFX (decision D6; the other choice is a contextual tab **LIBRARY TOOLS · Screen**). Center: the canvas. A toolbox column like the viewport's (spec 183) holds Select, Box, Ellipse, Line, Pen, Text, Image, Button. Ribbon: **Screen** (size ▾, Safe area, Snap, Text %), **Preview** (Preview values, Back to the starting values), **Code** (Open as code), **Play**. Shelf tab **Screen**: columns **Elements** (the tree, where Scene sits), **Piece** (cards: Tied to the game, When clicked, Size and place, Look, All its settings), **Pictures** (tiles, where Assets sits). | **NEEDS MOCK-UP** |

### a.5 The viewport's own controls (`viewport.rs:1345-1560`, `viewport_tools.rs:750-1048`)

| Today | Where it goes |
| --- | --- |
| The tool bar at the bottom centre: Select, Move, Rotate, Scale, Transform (Q, W, R, S, T) | The **toolbox** column (spec 183), 34 × 34 buttons. **Transform** (all three) is not in the design: added as a fifth button after Scale. In Level design (placing) the three go to the ribbon's Transform group (spec 247). |
| ≡ menu: Tools; Frame rate and entity count; Preview animation; Views (One, Two side by side, Two one above the other, Four); Maximise / All views | The **Show ▾** pill (spec 260) for the switches; the views choice at the end of the **Perspective ▾** pill's menu. Several views are not in the design but need no new surface. |
| View kind ▴ (Perspective, Top, Front, Right, 2D) | The **Perspective ▾** pill (spec 186). **2D** stays the last entry, after a divider, as today. |
| Look (Standard, Stylized, Unlit; bands of light, outlines) | The **Lit ▾** pill (spec 186). Words: decision D4. |
| Stats (fps · ms, entities, triangles, "Idle") | The mono pill top right (spec 186): "6.9 ms · 144 fps"; entities and triangles in its hover text |
| Snapping row: Grid step, Angle, Scale 10%, Snap ▴, World or Object (L), axis locks X Y Z, Camera speed; in 2D: Grid px, Zoom ▴ | Ribbon › **Snapping** (spec 175): "Grid · 0.25", "Angle · 15°", and a third row "Scale · 10%". World / Object and the axis locks: the status bar's right slot shows them ("Snap 0.25 · World", spec 234) and clicking it switches. Camera speed and 2D zoom: a pill at the viewport's bottom right. **NEEDS MOCK-UP** (small: the third snapping row and the bottom-right pill). |
| The cursor readout while dragging a gizmo ("× 0.25 0.00 0.00") | A label box by the cursor in the placing label's look (spec 258) |
| "Drop to place {name}" while a file is dragged over the viewport | The ghost, footprint and label of Screen 2 (spec 253-258) |
| Selection outline, colliders, reverb zones, the 2D game-screen frame, tile overlay | Unchanged drawings, recoloured from the tokens (outline 1.5 px accent, spec 184) |

### a.6 Windows, dialogs and overlays

| Today (file) | Where it goes | Mark |
| --- | --- | --- |
| **Settings** window (`settings_window.rs`): Editor › Appearance (Theme, Rounded panels, Accent colour, Hints, Panel layout); This project › 2D (Base resolution, Pixels per unit, Tile size, Scaling to the screen); Search settings; Reset this page | File › **Preferences** (Screen 8) and File › **Project settings** (Screen 12). Per setting: a.9. | |
| **Input actions** window (`input_window.rs`) | File › Project settings › **Input** (Screen 13) | |
| **Export for playtesting** window (`export_dialog.rs`): Game's name, Starts in, Into the folder, Window (Windowed / Fullscreen), Export, Open folder | The **Build** tab's center page (the design's Build tab is undesigned, spec 1183); its remembered values also in File › Project settings › Build | **NEEDS MOCK-UP** |
| **About** window (`workbench.rs:1762`) | File › About (a.2) | |
| **Unsaved changes** (Save, Don't save, Cancel; `workbench.rs:1643`), **Changed on disk** (Load the file, Keep mine; `workbench.rs:1691`), **Keep the changes made while playing?** (Keep, Undo all; `play_changes.rs:216`), **Rename** a file (`library.rs:1484`), **Rename the project** (`launcher.rs:1196`) | The same questions, as one shared dialog card. The design draws no dialog: proposed look is the placing label's box grown up (surface #181818, border #333333, radius 10, title 14 px / 600, buttons 30 px: the main one accent-filled). Rename becomes in-place on the tile or row (spec 327). | **NEEDS MOCK-UP** (one picture covers all) |
| **Update available** on the splash (`splash.rs:140`; dead code today, review finding DEBT-01) and the splash itself (`splash.rs`) | The splash keeps its job (the program starting) in the new tokens. The update question is already the launcher's bar: it becomes the status bar slot and File › Updates. | |
| The **search list** (`search.rs:732`) | The command palette under the command search (spec 136) | |
| The **files dragged over the window** hint (`panels.rs` `drop_hint`) | The status bar's "Drag files anywhere to import" (spec 333) and an accent inner outline on the Library's grid | |
| The **playing border** round the viewport (`panels.rs` `playing_border`) | **Removed** in M4 (a.1, decision D13: no tint) | |
| An exported game's window (`play_only.rs`) | Not editor UI: unchanged | |

### a.7 The Mixer's and sound's parts with no place in the design

| Today | Proposed place | Mark |
| --- | --- | --- |
| A bus in Properties: Name, Inside (its parent), Volume, **Ducking** rules (Quieter while, By, Fade down, Fade back, Right now), Remove bus | Audio › center, right group (where the design shows a bus effect, spec 784-791): a **Bus** header with a switch **Ducking / Effects**; Ducking shows the rules as cards, Effects is the placeholder EQ | **NEEDS MOCK-UP** |
| The Game panel's Mute button; "Mute the game" | Audio › Monitor group, a small row "Mute the game" | |
| A sound event's takes list (drop sound files), How it sounds, Where it's heard, Heard by (groups), When many play, Priority, Loop | Audio › shelf › Sound events › the detail column (spec 813), with a **Takes** block above the fields | **NEEDS MOCK-UP** (small) |
| Reverb zones (a component) | Stay an Inspector card; Home › Create › Volume makes one | |

### a.8 The launcher (`launcher.rs`, 1,551 lines; decisions 0086 and 0090)

| Today | Where it goes | Mark |
| --- | --- | --- |
| A page of its own in a small window, shown at every start | File › **Recent projects** in the one window (Screen 7). Solis still starts there every time (decision 0090's "always choose the project" kept); the other ribbon tabs are dimmed until a project is open. Decision D7 (the small window goes). | |
| Sidebar: Projects, Learn, Updates (with a **New** mark), "← Back to {project}" / "Carry on without a project", the update note at the bottom | The File menu column (spec 542-557): Back to editor, Recent projects, New project, Open…, Save all, Project settings, Preferences, Plugins, Learn, Exit; plus **Updates** and **About** in the last block. "Carry on without a project" = Back to editor with no project. | **NEEDS MOCK-UP** (the two added items) |
| Projects: rows (first letter, name, folder, build mark "build 57 · this one / older / newer", last opened), PINNED and ALL PROJECTS headings, ☆ pin, ⋯ (Open, Show in Explorer, Rename…, Make a copy…, Remove from list), Locate… and Remove for a missing folder, a note of what was done | Project **cards** (spec 571-589). Pinned cards first with a small pin on the card; the build mark is the card's version line ("build 52 · upgrade" in amber, spec 582); a missing folder is a dimmed card with **Locate…** and **Remove**. The ⋯ menu is the card's right-click menu (spec 608), with Solis's extra entries. | |
| Search projects; Sort by (Last opened, Name); + New project; Open project…; **Scan a folder…** | The page header (spec 559-567). **Sort by** and **Scan a folder…** are not in the design: a small dropdown beside the search, and an entry in **Open…**'s dropdown. | **NEEDS MOCK-UP** (small) |
| The update bar over the projects (Update now, See what's new, Not now) | The status bar's "Update available" (spec 616), and a notice card at the top of File › Updates | |
| Learn: four examples (Playground, Turret sample, UI sample, Benchmark; "Make it in a folder…"), first steps, Open the manual ↗ | File › **Learn** (undesigned): the examples as template tiles (spec 591-603), first steps as a list, the manual link. They could also fill the "Start from a template" strip: decision D8. | **NEEDS MOCK-UP** |
| Updates: this build, the newest and its notes, Check now, Download, Restart to update, Open the releases page ↗, "Look for updates when {name} starts", Every change build by build ↗ | File › **Updates** (undesigned), in the setting-row pattern (spec 632-634) with the notes as a card | **NEEDS MOCK-UP** |

### a.9 Every setting (`settings_window.rs`, `app/src/window.rs` keys)

| Setting today | Where it goes |
| --- | --- |
| Appearance › Theme (Standard, Graphite) | Removed (the lead's answer 2). Preferences › Appearance › **Base theme** shows the one choice, "Neutral dark" (spec 640). |
| Appearance › Rounded panels | Removed with the dock. |
| Appearance › Accent colour (ten) | Preferences › Appearance › **Accent color** (five, spec 641). Old choices are mapped (the plan, "Themes, layouts and settings"). |
| Appearance › Hints (Show hints) | Preferences › **General** › Show hints |
| Appearance › Panel layout (Reset layout) | Preferences › Appearance › Layout › Reset this workspace |
| 2D › Base resolution, Pixels per unit, Tile size, Scaling to the screen (project, `pixels.solpix`) | Project settings › a **2D** sub-tab added after Rendering (or inside Rendering): decision D9. **NEEDS MOCK-UP.** |
| Search settings; Reset this page | The page's search (spec 626, 924) and **Reset to defaults** (spec 663) |
| Kept without a control today: viewport overlays, snapping steps, look, camera, views, console history, known projects, auto-update, skipped build, window size (`window.rs:32-76`) | Stay as they are; overlays, snapping steps and camera speed get rows in Preferences › **Viewport** (undesigned: **NEEDS MOCK-UP**); "Look for updates when starting" in Preferences › General and File › Updates |
| New in this phase | Preferences › General › **Hide unfinished parts**; Appearance › Interface scale, Density, Show ribbon labels, Shelf opens on (spec 642-651) |

### a.10 The status bar (`status_bar.rs`)

| Today | Where it goes |
| --- | --- |
| "Code checked" or the count of mistakes (hover: when last checked) | Left: the dot + word (spec 162): green "Ready" when the code is checked and clean; amber "Checking…"; red "N mistakes" |
| "No problems" or the count (click opens Problems) | Left: error and warning counts as coloured dots + numbers (spec 162, 164) |
| Saved / Unsaved changes / Not saved yet | Right context text, and the unsaved dot on the document tab (spec 124) |
| Playing / Paused | Right context text; the Play button as Stop |
| The area's name · 2D or 3D · the 2D size ("farm · 2D · 480 × 270") | The area's name is in the tab bar's path (spec 135); "2D · 480 × 270" stays in the right context text |
| (nothing today) | "main · 4 changes" (placeholder, Shared 11); the version in mono at the far right (spec 163), from `AppInfo` |

### a.11 What is removed outright

`Panel::Layers`; the dock itself (drag a panel to an edge, floating panels, the ▼ fold on a tab row); Pin and Unpin; the Rounded panels switch; the two themes; the launcher's small window; `panels::placeholder` keeps its job (a quiet note such as "No problems found") but is renamed `empty_note`, so "placeholder" means one thing in the code.

---

## (b) Every element of the design, and what stands behind it

One row is one thing the **Hide unfinished parts** setting can switch off. The id is the registry id (the plan, "The placeholder mechanism"). Where a whole container is a placeholder (a ribbon tab, a shelf tab), its children are listed too, because with the setting off each is drawn as its own placeholder; with the setting on, the container goes and takes them with it. A **PARTLY** row stays on screen either way; the piece of it that is unbuilt has its own id, given in the row as "unbuilt piece:".

Counts are at the end of this part.

### b.0 The shell on every screen (spec 126-164)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `shell.logo` | Logo tile, 20 × 20 (spec 132) | EXISTS | `Workbench::logo` (the app hands in `branding::LOGO_PNG`) | |
| `shell.tab.file` | **File** tab (spec 133, 533) | EXISTS | (shell) opens the File pages; today's launcher and settings behind it | |
| `shell.tab.home` | **Home** tab | EXISTS | Today's Level workspace (`workspace.rs`) | |
| `shell.tab.library` | **Library** tab | EXISTS | `library.rs` | |
| `shell.tab.shading` | **Shading** tab | PLACEHOLDER | The tab, its ribbon (b.4) and a placeholder page | Shared 7: Materials and shaders |
| `shell.tab.animate` | **Animate** tab | PARTLY | Sprite clips, frames and animators are real (a.4); the sequencer (b.5) is not | Shared 10 |
| `shell.tab.vfx` | **VFX** tab | PLACEHOLDER | The tab, its ribbon (b.11) and a placeholder page | Shared 8: Effects |
| `shell.tab.audio` | **Audio** tab | PARTLY | The Mixer and sound events are real; effects, sends and snapshots are not (b.10) | not on the roadmap |
| `shell.tab.debug` | **Debug** tab | PARTLY | The Profiler and Console are real; capture, debugger and memory are not (b.6) | Shared 6 |
| `shell.tab.build` | **Build** tab (undesigned, spec 1183) | PARTLY | Export for playtesting (`export_dialog.rs`) is real; targets, platforms and the build log are not. **NEEDS MOCK-UP.** | Shared 11 |
| `shell.search` | Command search (spec 134, 136) | PARTLY | `search.rs` finds actions, files, entities and (now) shelf tabs. Unbuilt piece: `shell.search.settings` (settings as results) is built in M9, so no placeholder is needed. Key hint: Space. | |
| `shell.path` | Project / area path (spec 135) | EXISTS | Project name (`ProjectInfo`), `Document::name` | |
| `shell.ribbon.collapse` | Collapse-ribbon chevron (spec 135-136) | EXISTS | (shell). How it collapses is the spec's open question (spec 1184): decision D10. | |
| `shell.contextual` | Contextual "TOOLS" tabs (spec 138, 682) | EXISTS | (shell) driven by what is open or selected: a tile map (a.4), a tileset, a picture's crops | |
| `shell.ribbon` | Ribbon: groups, large buttons, small rows, sideways scroll (spec 140-146) | EXISTS | (shell) | |
| `shell.play.play` | **Play**, turning into Stop (spec 145, 1129) | EXISTS | `play.rs`, `top_bar.rs` `play_and_build` | |
| `shell.play.simulate` | **Simulate** (spec 145, 178) | PLACEHOLDER | Placeholder button | not on the roadmap |
| `shell.groups` | Editor groups with document tabs and a draggable split (spec 148-151) | EXISTS | (shell); documents are the area, the Game, an animator, a tileset, a picture's crops, a screen | |
| `shell.groups.drag-to-split` | Drag a document tab to split or merge groups (spec 151, 1121) | EXISTS | (shell), built last (M15); a new mouse action: decision D15 | |
| `shell.groups.several-areas` | Several areas open as tabs (spec 182) | PARTLY | One area is open at a time (`document.rs`). Unbuilt piece: `shell.groups.second-area`, the second tab ("Docks_Day.level" in the mock-up), which simply isn't drawn (no placeholder: nothing to show). | not on the roadmap |
| `shell.shelf` | Shelf: bar, grip, tabs, columns that wrap, resize (spec 153-158) | EXISTS | (shell) | |
| `shell.shelf.fold` | Ctrl + Space folds the shelf (spec 157) | EXISTS | (shell) | |
| `shell.shelf.opens-on` | Selecting opens the shelf on the chosen tab (spec 157, 1111) | EXISTS | (shell), from `Selection` changes | |
| `shell.status.ready` | Dot + "Ready" (spec 162) | EXISTS | `status_bar.rs` `Status`: code checked, mistakes | |
| `shell.status.branch` | "main · 4 changes" (spec 162, 164) | PLACEHOLDER | Placeholder status item | Shared 11: the Git panel |
| `shell.status.counts` | Error and warning counts, click opens Problems (spec 162, 164) | EXISTS | `problems.rs`, `status_bar.rs` | |
| `shell.status.context` | Context text per screen (spec 163) | EXISTS | `status_bar.rs` right-hand parts, extended per tab | |
| `shell.status.version` | Version in mono (spec 163) | EXISTS | `AppInfo::version` | |

### b.1 Screen 1: Home, the level editor (spec 166-234)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `home.create.mesh` | Create › **Mesh** (spec 174) | EXISTS | Places a model from the project or runs Import model… (`WorkbenchAction::PlaceAsset`, `ImportModel`). Ready-made shapes are shape brushes (Tools 1a): the long-press list's "Shape brush…" opens the brush list, as Create › Shape does. Word: "Model" (D4). | |
| `home.create.light` | Create › **Light** | EXISTS | `MenuCommand` Add point light, Add spotlight (`menu_items.rs`) | |
| `home.create.camera` | Create › **Camera** | EXISTS | `MenuCommand` Add camera | |
| `home.create.volume` | Create › **Volume** | PARTLY | Adds an entity with a ReverbZone, the one volume Solis has. Unbuilt piece: `home.create.trigger` (a trigger zone) in the long-press list. | 2D 2: triggers |
| `brush.shape.extrude` | HOME TOOLS · Brush › Shape › **Extrude** (Tools 1, mock-up 3) | PLACEHOLDER | A ribbon button, while a brush's insides are edited | Tools 1a, B4 |
| `brush.shape.inset` | HOME TOOLS · Brush › Shape › **Inset** | PLACEHOLDER | A ribbon button | Tools 1a, B4 |
| `brush.shape.split` | HOME TOOLS · Brush › Shape › **Split** | PLACEHOLDER | A ribbon button | Tools 1a, B4 |
| `brush.shape.delete` | HOME TOOLS · Brush › Shape › **Delete** | PLACEHOLDER | A ribbon button; Delete and Backspace say it comes with the next update | Tools 1a, B4 |
| `brush.ribbon.material` | HOME TOOLS · Brush › Brush › "Material ▾" | PLACEHOLDER | A ribbon row | Tools 1a, B5 |
| `brush.card.material` | The Brush card's **Material** row (mock-up 3) | PLACEHOLDER | A field row | Tools 1a, B5 |
| `home.create.long-press` | Long press opens a type list (spec 174) | EXISTS | (shell); a new mouse action: decision D15 | |
| `home.snap.grid` | Snapping › "Grid · 0.25" (spec 175) | EXISTS | `transforming::Snapping`, `viewport_tools.rs` | |
| `home.snap.angle` | Snapping › "Angle · 15°" | EXISTS | The same | |
| `home.lighting.bake` | Lighting › **Bake** (spec 176) | PLACEHOLDER | Placeholder large button | 3D 5: Bounced light |
| `home.lighting.quality` | Lighting › "Quality: Preview ▾" | PLACEHOLDER | Placeholder small row | 3D 5 |
| `home.lighting.lightmap-view` | Lighting › "Light map view" | PLACEHOLDER | Placeholder small row | 3D 5 |
| `home.project.save` | Project › **Save** (spec 177) | EXISTS | `WorkbenchAction::SaveArea` (the other files save as they change, `own_files.rs`) | |
| `home.project.build` | Project › **Build** | PARTLY | Opens the Build tab with Export for playtesting. Unbuilt piece: build targets (`build.targets`, b.14). | Shared 11 |
| `home.docs` | Viewport group's document tabs (spec 182) | EXISTS | (shell) `Document::name`, `is_dirty` | |
| `home.tool.select` | Toolbox › Select (spec 183) | EXISTS | `viewport::Tool::Select` (Q) | |
| `home.tool.move` | Toolbox › Move | EXISTS | `Tool::Move` (W) | |
| `home.tool.rotate` | Toolbox › Rotate | EXISTS | `Tool::Rotate` (R) | |
| `home.tool.scale` | Toolbox › Scale | EXISTS | `Tool::Scale` (S) | |
| `home.tool.pose` | Toolbox › Pose bones | PLACEHOLDER | Not drawn for now: the toolbox column went, the tools float in the view (decision 0114) | 3D 1: 3D animation |
| `home.tool.motion-path` | Toolbox › Motion path | PLACEHOLDER | Not drawn for now: the toolbox column went, the tools float in the view (decision 0114) | Shared 4: paths drawn in the editor |
| `home.tool.add-key` | Toolbox › Add key | PLACEHOLDER | Not drawn for now: the toolbox column went, the tools float in the view (decision 0114) | Shared 10: the cutscene timeline |
| `home.tool.measure` | Toolbox › Measure | PLACEHOLDER | Not drawn for now: the toolbox column went, the tools float in the view (decision 0114) | not on the roadmap |
| `home.view.scene` | The viewport's picture, outline and gizmo (spec 184-185) | EXISTS | `ViewportRenderer`, `viewport_tools::Outline`, `gizmo.rs`; axis colours from the tokens | |
| `home.view.motion-curve` | The dashed animation path over the selection (spec 184) | PLACEHOLDER | Nothing is drawn (a sample drawing, not a control); registered so it is counted | Shared 10 |
| `home.pill.view` | "Perspective ▾" (spec 186) | EXISTS | `camera::ViewKind` (Perspective, Top, Front, Right, 2D), and the views count | |
| `home.pill.shading` | "Lit ▾" | EXISTS | `look::Look` (Standard, Stylized, Unlit) | |
| `home.pill.stats` | "6.9 ms · 144 fps" | EXISTS | `Viewport::stats` | |
| `home.code` | The code editor group: tabs, breadcrumb, code, the error pinned to its line (spec 188-195) | PLACEHOLDER | Not drawn by default, as the spec says (spec 29, 190): the viewport has the full width. With **Code editor split** on (itself a placeholder setting), a placeholder page. Code opens in the user's code editor as today. Decision D11. | not on the roadmap (a later feature) |
| `home.shelf.object` | Shelf tab **Object** (spec 199) | EXISTS | (shell) | |
| `home.shelf.animation` | Shelf tab **Animation**: Tracks, dope sheet, Key details (spec 218-222) | PLACEHOLDER | Placeholder shelf tab with a placeholder page | Shared 10 |
| `home.shelf.console` | Shelf tab **Console** (spec 224-226) | EXISTS | `console.rs` | |
| `home.shelf.problems` | Shelf tab **Problems · N** (spec 228-230) | EXISTS | `problems.rs` | |
| `home.shelf.transport` | Previous key, play animation, next key (spec 199) | PLACEHOLDER | Three placeholder icon buttons | Shared 10 |
| `home.shelf.frame-counter` | "0096 / 0240" | PLACEHOLDER | Placeholder mono text "---- / ----" | Shared 10 |
| `home.object.scene` | **Scene** column: tree, count, selection (spec 205) | EXISTS | `area_panel.rs` `area_tree` | |
| `home.object.inspector` | **Inspector** column: header, + Component, cards (spec 206-214) | EXISTS | `properties.rs`, `inspector.rs` | |
| `home.object.model-material` | The Model card's **Material** row (approved additions, round 2, `04-properties-cards`) | PLACEHOLDER | Placeholder field row: a Model has one setting today, the model itself (found while mocking up, round 2) | Shared 7: Materials and shaders |
| `home.object.model-shadows` | The Model card's **Shadows** row | PLACEHOLDER | Placeholder field row: only the sun casts shadows today, switched on its Light card | not on the roadmap |
| `home.object.edited-border` | A just-edited value's accent border (spec 121, 211) | PARTLY | The focused field gets it; "just edited" is new and pure interface, built in M5 | |
| `home.object.label-scrub` | Drag a field's label to scrub (spec 214) | PARTLY | Today the value itself is dragged (`settled.rs`). Label dragging is a new mouse action: decision D15. | |
| `home.object.card-menu` | Right-click a card: reset, copy, remove (spec 214) | EXISTS | The card's ⋯ menu today (`properties.rs:726`) | |
| `home.object.assets` | **Assets** column: tiles of the current folder, Import (spec 207, 216) | EXISTS | `library.rs` (thumbnails), `ImportModel` | |
| `home.object.assign-drop` | Drag a tile onto a card's field to assign it (spec 216, 1118) | PARTLY | Sound files dropped on a sound event's takes work (`sound_event_view.rs`). Dropping on a model, sprite sheet or animator field is new, built in M5 (the fields already take an asset id). | |
| `home.console.links` | A log line's file:line link (spec 226, 1098) | PARTLY | Problems opens a mistake's file (`ProblemClick::File`). Console lines carry no file or line. Unbuilt piece: `home.console.line-link`. | Shared 6: seeing inside the game |
| `home.status.cursor` | "Ln 84, Col 9" (spec 234) | PLACEHOLDER | Not drawn (part of the code editor) | not on the roadmap |
| `home.status.snap` | "Snap 0.25 · World" | EXISTS | `Snapping`, the world or object axes switch (L) | |

### b.2 Screen 2: Home, level design (placing) (spec 236-287)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `place.mode` | The Home ribbon in its build form (Place, Snapping, Paint, Transform, Play) while the Place shelf tab is in front (spec 240-248) | EXISTS | (shell). How the mode is entered isn't written down: decision D16. | |
| `place.snap.surface` | Snapping › "Snap to surface" (spec 245) | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `place.paint.brush` | Paint › **Brush** (assets along a stroke; spec 246) | PLACEHOLDER | Placeholder large button. (With a tile map selected, the tile painter's real Brush is on the contextual tab, a.4.) | not on the roadmap (2D 1 has scatter brushes for tiles) |
| `place.paint.scatter` | Paint › **Scatter** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `place.transform` | Transform › Move, Rotate, Scale (spec 247) | EXISTS | `viewport::Tool` | |
| `place.ghost` | Ghost, footprint, snap guides, drag trail (spec 253-256) | PARTLY | Dragging a file in places it where pointed, with the words "Drop to place" (`viewport.rs`). The ghost is new and pure drawing from the model's bounds (`ModelInfo::min`, `max`), built in M6; the trail is a cut candidate. | |
| `place.label` | Name and live coordinates by the cursor (spec 258) | PARTLY | The name is shown today; the coordinates and snap source are added in M6 | |
| `place.hints` | Hint strip: Release, Shift, Alt, R, Esc (spec 259) | PARTLY | Release places and Esc cancels today. Unbuilt pieces, each a placeholder chip until its key is agreed (rule 17, decision D15): `place.keep-placing` (Shift), `place.free` (Alt), `place.rotate-ghost` (R). | not on the roadmap |
| `place.pill.show` | "Show ▾" (spec 260) | EXISTS | The viewport's ≡ menu (`Overlays`) | |
| `place.pill.code-hidden` | "Code editor hidden while building · Show" | PLACEHOLDER | Not drawn (part of the code editor) | not on the roadmap |
| `place.shelf.place` | Shelf tab **Place** and its right-hand hint (spec 264) | EXISTS | (shell) | |
| `place.scene.placing-row` | "+ name (placing)" row while dragging (spec 270) | EXISTS | (shell), from the drag in progress | |
| `place.scene.target` | "New objects go into ▾" (spec 270) | PLACEHOLDER | Placeholder footer row. (Today: drop onto an entity in the tree to put it under that one.) | not on the roadmap |
| `place.align` | Placement › Align to: Surface / Grid / Free (spec 271) | PARTLY | Grid and Free are snapping on and off. Unbuilt piece: `place.align.surface`. | not on the roadmap |
| `place.rotation` | Placement › Rotation "Keep · 0° ▾" | PLACEHOLDER | Placeholder field | not on the roadmap |
| `place.random-yaw` | Placement › Random yaw | PLACEHOLDER | Placeholder toggle | not on the roadmap |
| `place.scale` | Placement › Scale | PLACEHOLDER | Placeholder field | not on the roadmap |
| `place.physics` | Placement › Physics "Settle on drop ▾" | PLACEHOLDER | Placeholder field | not on the roadmap |
| `place.chip.favorites` | Assets › **★ Favorites** chip, and the ★ on tiles (spec 272, 274) | PLACEHOLDER | Placeholder chip; no stars | not on the roadmap |
| `place.chip.recent` | Assets › **Recent** chip | PLACEHOLDER | Placeholder chip | not on the roadmap |
| `place.chip.folder` | Assets › folder chip "Harbor ▾" | EXISTS | `LibraryState` folder | |
| `place.chip.lights` | Assets › **Lights** chip, with tiles that add a light | EXISTS | (shell) tiles run Add point light, Add spotlight | |
| `place.search` | "Find something to place" | EXISTS | `library.rs` search | |
| `place.open-library` | "Open Library" link (spec 272, 1096) | EXISTS | (shell) switches to the Library tab, same folder | |
| `place.tiles` | Place tiles, min 92 px (spec 274) | EXISTS | `library.rs` thumbnails; the "dragging" badge is (shell) | |
| `place.click-to-place` | Click a tile, then click in the scene (spec 282) | PLACEHOLDER | Not drawn (a behaviour): today double-click places where the camera looks. A new mouse action: decision D15. | not on the roadmap |
| `place.status` | "Placing name · Snap 0.25 · Surface" (spec 287) | EXISTS | (shell), from the drag and `Snapping` | |

### b.3 Screen 3: Library (spec 289-333)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `lib.add.import` | Add › **Import** (spec 297) | PARTLY | Import model… (`ImportModel`); files dropped on the window (`files_hovering`). Other kinds are copied into the folder by hand and picked up by the watcher. Unbuilt piece: `lib.import.any-kind` (one picker for every kind). | not on the roadmap |
| `lib.add.create` | Add › **Create** | EXISTS | The Library's + New ▾ (folder, area, Lust code file, tile map, tileset, animator, sound event) | |
| `lib.add.folder` | Add › **Folder** | EXISTS | `WorkbenchAction::NewFolder` | |
| `lib.view.grid` | View › Grid / List (spec 298) | EXISTS | `LibraryState::list` (Thumbnails, List) | |
| `lib.view.folders` | View › Show folders | EXISTS | (shell): folders as tiles is new drawing over the folders the Library already has | |
| `lib.filter.type` | Filter › "Type: All ▾", and the type chips (spec 299, 305) | EXISTS | The Library's kind filter, with Solis's kinds (D4, D17) | |
| `lib.filter.tags` | Filter › "Tags ▾"; tag chips and "+ Tag" in Details (spec 299, 321) | PLACEHOLDER | Placeholder small row; no tag chips | not on the roadmap |
| `lib.filter.missing` | Filter › "Missing refs"; the "!" badge on a tile (spec 299, 310) | PLACEHOLDER | Placeholder small row; no badges. (Problems already lists a missing model or sheet per entity.) | not on the roadmap |
| `lib.source.check-in` | Source control › **Check in** (spec 300) | PLACEHOLDER | Placeholder large button | Shared 11 |
| `lib.source.history` | Source control › **History**; shelf tab **History** (spec 300, 314) | PLACEHOLDER | Placeholder large button and shelf tab | Shared 11 |
| `lib.use.place` | Use › **Place** (spec 301, 1093) | EXISTS | `PlaceAsset`; switches to Home with the Place tab in front | |
| `lib.use.show-in-level` | Use › "Show in level" (spec 301, 1094) | PLACEHOLDER | Placeholder small row | not on the roadmap (small) |
| `lib.nav` | Back, Up one folder, breadcrumb with a dropdown (spec 305) | EXISTS | The Library's breadcrumb and folders; Back is (shell) | |
| `lib.search` | "Search Harbor" | EXISTS | `library.rs` search | |
| `lib.grid` | The tile grid: thumbnail, name, type line, selection (spec 306-309) | EXISTS | `library.rs`, `app/src/thumbnails.rs` | |
| `lib.shelf.details` | Shelf tab **Details** (spec 316-322) | EXISTS | What Properties shows for a picked file today (`sound_view.rs`, `sheet_view.rs`, `text_view.rs`, `sound_event_view.rs`) | |
| `lib.details.preview` | Preview box with Lit, Wire, UV pills and orbit (spec 320) | PARTLY | A model's thumbnail, a sound's waveform, a sheet's frames, a text file's words. Unbuilt piece: `lib.preview.modes` (the three pills and orbiting). | not on the roadmap |
| `lib.details.facts` | Name, kind line and key-value facts (spec 321) | PARTLY | Path, kind, and per kind what Solis knows (a sound's length, channels and rate; a model's triangles, levels of detail and bounds from `ModelInfo`; a sheet's size, tags and layers). Vertices, materials, disk size and "modified" are left out, not placeheld, until something knows them. | |
| `lib.shelf.used-by` | Shelf tab **Used by · N** and the Used by column (spec 314, 322) | PARTLY | Code files' uses (`workspace.rs` `code_uses`) and "shared by" (`properties.rs`). Unbuilt piece: `lib.used-by.all-kinds` (which areas and assemblies use a model, picture or sound). | not on the roadmap |
| `lib.shelf.import-queue` | Shelf tab **Import queue · N** and the progress bar (spec 314, 322) | PLACEHOLDER | Placeholder shelf tab. (Imports already run on workers, `app/src/import.rs`, with no list.) | not on the roadmap (small) |
| `lib.count` | "48 items · 1 selected" (spec 314) | EXISTS | (shell) | |
| `lib.multi-select` | Ctrl / Shift click selects several (spec 327) | PLACEHOLDER | Not drawn (a behaviour): one file is selected at a time (`LibraryState::selected`) | not on the roadmap |
| `lib.rename` | F2 renames in place (spec 327) | EXISTS | Rename… (`library.rs:1484`), made in-place; F2 already means Rename | |
| `lib.delete-warns` | Delete warns when something uses the file (spec 327) | PARTLY | Delete moves to the Recycle Bin with its ID file. The warning needs `lib.used-by.all-kinds`. | not on the roadmap |
| `lib.open` | Double-click opens in its editor (spec 328, 1095) | EXISTS | `workbench_library.rs`: animator, tileset, clips, screen, area, picture's crops; code and text open in their program | |
| `lib.drag-to-home` | Drag a tile onto the Home tab (spec 329) | PLACEHOLDER | Not drawn (a behaviour); a new mouse action: decision D15 | not on the roadmap |
| `lib.status` | "Drag files anywhere to import" (spec 333) | EXISTS | `files_hovering`; the words say "models" until `lib.import.any-kind` | |

### b.4 Screen 4: Shading, the material editor (spec 335-386). All Shared 7.

The whole tab is `shell.tab.shading`. Solis's shared graph editor (`graph_canvas.rs`, rule 21) will drive the graph when Shared 7 builds it.

| id | Element | Mark | What the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `shading.material.apply` | Material › **Apply** (spec 343) | PLACEHOLDER | Placeholder large button | Shared 7 |
| `shading.material.save` | Material › **Save** | PLACEHOLDER | Placeholder large button | Shared 7 |
| `shading.add.texture` | Add node › **Texture** (spec 344) | PLACEHOLDER | Placeholder large button | Shared 7 |
| `shading.add.math` | Add node › **Math** | PLACEHOLDER | Placeholder large button | Shared 7 |
| `shading.add.param` | Add node › **Param** | PLACEHOLDER | Placeholder large button | Shared 7 |
| `shading.graph.auto-layout` | Graph › "Auto-layout" (spec 345) | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.graph.comment` | Graph › "Comment frame" | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.graph.wires` | Graph › "Wires: Curved ▾" | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.preview.shape` | Preview › "Shape: Sphere ▾" (spec 346) | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.preview.lit` | Preview › "Lit" | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.preview.live` | Preview › "Live update" | PLACEHOLDER | Placeholder small row | Shared 7 |
| `shading.graph` | Center left: the node graph, its document tabs, comment frames, nodes, pins, wires, hint pill (spec 348-365) | PLACEHOLDER | One placeholder page for the whole center | Shared 7 |
| `shading.preview` | Center right: the preview and its "Compiled" footer (spec 367-370) | PLACEHOLDER | Part of the same page | Shared 7 |
| `shading.shelf.parameters` | Shelf tab **Parameters**: parameter cards, Pin types legend (spec 376-382) | PLACEHOLDER | Placeholder shelf tab | Shared 7 |
| `shading.shelf.node` | Shelf tab **Node · name** (spec 374) | PLACEHOLDER | Placeholder shelf tab | Shared 7 |
| `shading.shelf.palette` | Shelf tab **Palette** | PLACEHOLDER | Placeholder shelf tab | Shared 7 |
| `shading.shelf.instances` | Shelf tab **Instances · N** | PLACEHOLDER | Placeholder shelf tab | Shared 7 |

### b.5 Screen 5: Animate, the sequencer (spec 388-451)

The tab is real because sprite clips and animators live there (a.4). Everything the sequencer itself needs is Shared 10 (the cutscene timeline); skeletons are 3D 1.

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `anim.keys.add` | Keys › **Add key** (spec 396) | PLACEHOLDER | Placeholder large button | Shared 10 |
| `anim.keys.auto` | Keys › **Auto key** | PLACEHOLDER | Placeholder large button | Shared 10 |
| `anim.interp.step` | Interpolation › Step (spec 397) | PLACEHOLDER | Placeholder small row | Shared 10 |
| `anim.interp.linear` | Interpolation › Linear | PLACEHOLDER | Placeholder small row | Shared 10 |
| `anim.interp.smooth` | Interpolation › Smooth | PLACEHOLDER | Placeholder small row | Shared 10 |
| `anim.tracks.track` | Tracks › **Track** (spec 398) | PLACEHOLDER | Placeholder large button | Shared 10 |
| `anim.tracks.curves` | Tracks › **Curves** | PLACEHOLDER | Placeholder large button | Shared 10 |
| `anim.playback.loop` | Playback › "Loop" (spec 399) | PARTLY | Real for a sprite clip (its direction and "once", `frames_view.rs`); a placeholder with no clips file open | Shared 10 |
| `anim.playback.fps` | Playback › "30 fps ▾" | PARTLY | A clip's speed, for a clips file; otherwise a placeholder | Shared 10 |
| `anim.playback.range` | Playback › "Range 0–240" | PARTLY | A clip's start and end, for a clips file; otherwise a placeholder | Shared 10 |
| `anim.camera` | Center left: the shot camera's tab, cinema frame, timecode (spec 402-406) | PLACEHOLDER | Placeholder page. The "Viewport" tab beside it is real (the area's viewport). | Shared 10 |
| `anim.curves` | Center right: the curve editor (spec 408-412) | PLACEHOLDER | Placeholder page | Shared 10 |
| `anim.shelf.sequencer` | Shelf tab **Sequencer**: tracks, dope sheet, ruler, clips, keys, playhead (spec 418-435) | PLACEHOLDER | Placeholder shelf tab. (The sprite Frames tab is built in these same shapes, a.4.) | Shared 10 |
| `anim.shelf.key` | Shelf tab **Key · name** (spec 437-439) | PLACEHOLDER | Placeholder shelf tab | Shared 10 |
| `anim.shelf.console` | Shelf tab **Console** (spec 416) | EXISTS | `console.rs` | |
| `anim.shelf.transport` | Previous key, play, next key and the frame counter (spec 416) | PARTLY | Play and Stop, and "Frame n of m", for a clips file (`frames_view.rs`); previous and next key are placeholders | Shared 10 |
| `anim.status` | "30 fps · 240 frames" (spec 451) | PARTLY | A clip's speed and frame count, for a clips file | Shared 10 |

### b.6 Screen 6: Debug, the profiler and console (spec 453-529)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `debug.capture.record` | Capture › **Record** (spec 461) | PLACEHOLDER | Placeholder large button. (The Profiler always keeps the last frames, `profiler.rs` `KEPT`.) | Shared 6 |
| `debug.capture.pause` | Capture › **Pause** | PLACEHOLDER | Placeholder large button | Shared 6 |
| `debug.capture.save` | Capture › **Save** | PLACEHOLDER | Placeholder large button | Shared 6 |
| `debug.target.source` | Target › "Play-in-editor ▾" (spec 462) | PLACEHOLDER | Placeholder small row | Shared 11 (a built game to profile) |
| `debug.target.frames` | Target › "Frames: last 300" | EXISTS | `profiler.rs` `KEPT` (240 frames today), shown as it is ("Frames: last 240"), not yet changeable | |
| `debug.target.deep` | Target › "Deep profile" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `debug.show.cpu` | Show › CPU (spec 463) | EXISTS | (shell) hides the CPU stages of `profiler::Stage` | |
| `debug.show.gpu` | Show › GPU | EXISTS | (shell) hides the GPU's times | |
| `debug.show.memory` | Show › Memory | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `debug.tools.debugger` | Tools › **Debugger** (spec 464) | PLACEHOLDER | Placeholder large button | not on the roadmap (DESIGN.md §13 names stepping through Lust) |
| `debug.tools.memory` | Tools › **Memory** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `debug.graph` | Frame-time graph: bars, budget lines, the frame label (spec 467-473) | PARTLY | `profiler.rs:305` draws the kept frames as bars with the 60 fps line. Selecting a frame or a range is new: unbuilt piece `debug.graph.select`, built in M14 if the samples allow it (they do: `Profiler::history`). | |
| `debug.timeline` | Frame timeline by thread (spec 475-489) | PARTLY | One lane of the CPU's stages (game code, world, preparing, drawing, sound, editor) and one of the GPU's. Unbuilt piece: `debug.timeline.threads` (Physics, Render and worker lanes, named blocks). | not on the roadmap |
| `debug.stats` | Statistics: CPU, GPU, draw calls, memory (spec 491-500) | PARTLY | CPU and GPU times, draw calls (counted today: round 3 found it; M14 took the piece out of the table), triangles, entities, components, projectiles, levels of detail. Unbuilt piece: `debug.stats.memory`. | not on the roadmap |
| `debug.shelf.console` | Shelf tab **Console**, level filters with counts, command line (spec 504-519) | EXISTS | `console.rs`, `console_commands.rs` (history with Up and Down, Tab finishes a name) | |
| `debug.console.hint` | The completions shown at the right of the command line (spec 518) | PARTLY | Tab completes today; showing the choices is new, built in M14 | |
| `debug.shelf.hottest` | Shelf tab **Hottest functions** (spec 504) | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `debug.shelf.memory` | Shelf tab **Memory snapshot** | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `debug.status` | "Recording · 300 frames" (spec 529) | PLACEHOLDER | Not drawn; the slot shows "Last 240 frames" | Shared 6 |

### b.7 Screen 7: File, Recent projects (spec 531-616)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `file.back` | **Back to editor** (spec 546) | EXISTS | `launcher.rs` "Back to", Esc | |
| `file.recent` | **Recent projects** | EXISTS | `launcher.rs`, `app/src/known_projects.rs` | |
| `file.new` | **New project** | EXISTS | `WorkbenchAction::NewProject` | |
| `file.open` | **Open…** | EXISTS | `WorkbenchAction::OpenProject` | |
| `file.save-all` | **Save all** (spec 550) | EXISTS | `SaveArea` (every other file is saved as it changes) | |
| `file.project-settings` | **Project settings** | EXISTS | b.12, b.13 | |
| `file.preferences` | **Preferences** | EXISTS | b.8 | |
| `file.plugins` | **Plugins** (spec 554) | PLACEHOLDER | Placeholder menu item and page | Shared 11: scripting and mods |
| `file.learn` | **Learn** (spec 556) | EXISTS | The launcher's Learn page. Page undesigned: **NEEDS MOCK-UP**. | |
| `file.exit` | **Exit** | EXISTS | `WorkbenchAction::Quit` | |
| `recent.search` | "Search projects" (spec 565) | EXISTS | `launcher.rs` search | |
| `recent.cards` | Project cards: name, folder, last opened, version, the open one's accent border (spec 571-582) | EXISTS | `KnownProject`: name, folder, when opened, the build that last opened it | |
| `recent.card.thumbnail` | The card's 140 px picture (spec 575) | PLACEHOLDER | The project's first letter, large, on the thumbnail's surface (what the launcher's rows show today) with the placeholder mark | not on the roadmap |
| `recent.card.badge` | "3D" / "2D" badge (spec 576) | PLACEHOLDER | No badge. Solis's projects aren't one or the other. | not on the roadmap (decision D8) |
| `recent.card.outdated` | An older version in amber with "upgrade" (spec 582, 609) | EXISTS | The build mark (older, newer); files upgrade on save with a backup (decision 0053); "upgrade a copy first" is Make a copy… | |
| `recent.card.menu` | Right-click: Pin, Show in folder, Remove from list, Upgrade (spec 608) | EXISTS | `PinProject`, `ShowProject`, `ForgetProject`, plus Rename…, Make a copy…, Locate… | |
| `recent.template.blank-3d` | Template **Blank 3D** (spec 597) | PARTLY | Makes an empty project (`NewProject`), as Blank 2D does: Solis has one kind of empty project. Decision D8. | Shared 13: templates |
| `recent.template.blank-2d` | Template **Blank 2D** | PARTLY | The same, with the first area's view set to 2D | Shared 13 |
| `recent.template.first-person` | Template **First-person** | PLACEHOLDER | Placeholder template tile | Shared 13 |
| `recent.template.top-down` | Template **Top-down** | PLACEHOLDER | Placeholder template tile | Shared 13 |
| `recent.template.platformer` | Template **Platformer** | PLACEHOLDER | Placeholder template tile | 2D 2 (the controller), Shared 13 |
| `recent.template.vr` | Template **VR / XR** | PLACEHOLDER | Placeholder template tile | not on the roadmap (decision D8: leave it out?) |
| `recent.template.all` | "All templates" link (spec 593) | PLACEHOLDER | Placeholder link | Shared 13 |
| `recent.status.update` | "Update [X.Y] available" (spec 616) | EXISTS | `updates.rs` `UpdateNotice` ("Build N is available") | |

### b.8 Screen 8: File, Preferences (spec 618-674)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `prefs.search` | "Search all settings", across sub-tabs, words highlighted (spec 626, 668) | EXISTS | `settings_window.rs` `Setting::matches`; highlighting is (shell) | |
| `prefs.tab.general` | Sub-tab **General** (undesigned) | EXISTS | (shell): Show hints, Look for updates when starting, **Hide unfinished parts**. **NEEDS MOCK-UP.** | |
| `prefs.tab.appearance` | Sub-tab **Appearance** | EXISTS | b.8 rows below | |
| `prefs.tab.viewport` | Sub-tab **Viewport** (undesigned) | EXISTS | `Overlays`, `Snapping` steps, camera speed, `Look`. **NEEDS MOCK-UP.** | |
| `prefs.tab.input` | Sub-tab **Input** (undesigned; the editor's own mouse and keys) | PLACEHOLDER | Placeholder sub-tab and page | not on the roadmap |
| `prefs.tab.shortcuts` | Sub-tab **Shortcuts** (undesigned; "all shortcuts are editable", spec 1087) | PARTLY | A read-only list from the one table of shortcuts (`search.rs`, the test of rule 21). Unbuilt piece: `prefs.shortcuts.edit` (changing a key). **NEEDS MOCK-UP.** | not on the roadmap |
| `prefs.tab.source-control` | Sub-tab **Source control** | PLACEHOLDER | Placeholder sub-tab and page | Shared 11 |
| `prefs.base-theme` | Base theme "Neutral dark" (spec 640) | EXISTS | The one palette (tokens) | |
| `prefs.accent` | Accent color, five swatches (spec 641) | EXISTS | `theme::Accent`, cut to the five | |
| `prefs.scale` | Interface scale (spec 642) | EXISTS | (shell) egui's zoom factor | |
| `prefs.density` | Density: Compact, Default, Comfortable (spec 643) | EXISTS | (shell) row and field heights from the tokens | |
| `prefs.ribbon-labels` | Show ribbon labels (spec 649) | EXISTS | (shell) | |
| `prefs.shelf-opens-on` | Shelf opens on (spec 650) | EXISTS | (shell) | |
| `prefs.code-split` | Code editor split (spec 651) | PLACEHOLDER | Placeholder setting row (decision D11) | not on the roadmap |
| `prefs.preview` | The live miniature (spec 653-661) | EXISTS | (shell); a cut candidate | |
| `prefs.reset` | Reset to defaults, asking which scope (spec 663, 669) | EXISTS | "Reset this page" today | |

### b.9 Screen 9: Library › the model viewer (spec 676-743)

The contextual tab is real (b.0); this user of it is not. Double-clicking a model keeps placing it, as today; the viewer's placeholder is reached from the tile's right-click **Open**. Solis knows a model's triangles, nodes, lights and levels of detail (`models.rs` `ModelInfo`, decision 0032), which is where the real version starts.

| id | Element | Mark | What the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `mesh.tab` | Contextual tab **LIBRARY TOOLS · Mesh** (spec 682) | PLACEHOLDER | The tab, drawn with the placeholder mark | not on the roadmap |
| `mesh.view.lit` | View › **Lit** (spec 688) | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.view.unlit` | View › **Unlit** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.view.wire` | View › **Wire** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.view.normals` | View › **Normals** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.view.uv-check` | View › **UV check** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.show.bounds` | Show › Bounds (spec 689) | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.show.pivot` | Show › Pivot | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.show.collision` | Show › Collision | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.light.preset` | Lighting › "Studio ▾" (spec 690) | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.light.rotate` | Lighting › "Rotate light" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.light.background` | Lighting › "Background: Gray ▾" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.lod.auto` | LOD › Auto (spec 691) | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.lod.pick` | LOD › "LOD 0 ▾" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.lod.density` | LOD › "Show tri density" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `mesh.tools.reimport` | Tools › **Reimport** (spec 692) | PLACEHOLDER | Placeholder large button. (Models already reload when their file changes.) | not on the roadmap |
| `mesh.tools.pivot` | Tools › **Pivot** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.tools.shot` | Tools › **Shot** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `mesh.use.place` | Use › **Place** (spec 693) | EXISTS | `PlaceAsset` | |
| `mesh.use.show-in-level` | Use › "Show in level" | PLACEHOLDER | Placeholder small row (the same feature as `lib.use.show-in-level`) | not on the roadmap |
| `mesh.viewport` | Center left: the studio viewport, bounds, dimension lines, pivot, pills, camera presets (spec 700-712) | PLACEHOLDER | One placeholder page for the center | not on the roadmap |
| `mesh.uv` | Center right: the UV layout (spec 714-718) | PLACEHOLDER | Part of the same page | not on the roadmap |
| `mesh.shelf.overview` | Shelf tab **Overview**: Levels of detail, Material slots, Import (spec 724-730) | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `mesh.shelf.lods` | Shelf tab **LODs** | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `mesh.shelf.materials` | Shelf tab **Materials** | PLACEHOLDER | Placeholder shelf tab | Shared 7 |
| `mesh.shelf.collision` | Shelf tab **Collision** | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `mesh.shelf.sockets` | Shelf tab **Sockets · N** | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |
| `mesh.shelf.import` | Shelf tab **Import settings** | PLACEHOLDER | Placeholder shelf tab | not on the roadmap |

### b.10 Screen 10: Audio, the mixer (spec 745-826)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `audio.mixer.bus` | Mixer › **Bus** (spec 753) | EXISTS | `mixer_view.rs` Add bus (`SaveMixer`) | |
| `audio.mixer.send` | Mixer › **Send**; the Rev send knob on a strip (spec 753, 765) | PLACEHOLDER | Placeholder large button; the strip leaves the knob's space empty, as the spec allows | not on the roadmap |
| `audio.mixer.snapshot` | Mixer › **Snapshot**; shelf tab **Snapshots · N** (spec 753, 795) | PLACEHOLDER | Placeholder large button and shelf tab | not on the roadmap |
| `audio.monitor.clear-solo` | Monitor › "Clear solo" (spec 754) | EXISTS | (shell) over the buses' solo switches | |
| `audio.monitor.meters` | Monitor › "Meters: Peak + RMS ▾" | PLACEHOLDER | Placeholder small row | not on the roadmap |
| `audio.monitor.listen-from` | Monitor › "Listen from: Camera ▾" | PARTLY | Shows who hears now (the first Listener, or the camera: `game_sound.rs` `Hearer`); choosing isn't built: unbuilt piece `audio.listen.choose` | not on the roadmap |
| `audio.fx.eq` | Effects › **EQ** (spec 755) | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `audio.fx.compress` | Effects › **Compress** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `audio.fx.reverb` | Effects › **Reverb** | PLACEHOLDER | Placeholder large button. (Reverb exists only as zones in the world.) | not on the roadmap |
| `audio.fx.limiter` | Effects › **Limiter** | PLACEHOLDER | Placeholder large button | not on the roadmap |
| `audio.console` | The mixer console: a strip per bus with name, routing, fader, dB scale, readout, M and S; the selected and muted looks (spec 758-772) | EXISTS | `mixer_view.rs` `MixerState`, `sound::Mixer` | |
| `audio.strip.inserts` | A strip's insert slots (spec 764) | PLACEHOLDER | One placeholder "+ Insert" row | not on the roadmap |
| `audio.strip.pan` | A strip's pan (spec 766) | PLACEHOLDER | Placeholder track | not on the roadmap |
| `audio.strip.meters` | Stereo meters with a peak-hold line, amber when hot (spec 768) | PARTLY | One meter per bus that falls back after a peak (`update_meters`). Drawn as the pair with the same level; unbuilt piece `audio.meters.stereo`. | not on the roadmap |
| `audio.effect` | Center right: the bus effect (EQ graph, bands) (spec 784-791) | PLACEHOLDER | Placeholder page behind the **Effects** side of the switch; the **Ducking** side is real (a.7) | not on the roadmap |
| `audio.shelf.events` | Shelf tab **Sound events**: the table and the detail column (spec 797-813) | PARTLY | Sound events are files with a bus, volume and where they're heard (`sound_event_view.rs`); the table lists them. "Used by" in the detail: unbuilt piece `audio.event.used-by`. | not on the roadmap |
| `audio.shelf.console` | Shelf tab **Console** | EXISTS | `console.rs` | |
| `audio.voices` | "14 / 64 voices" (spec 795, 826) | PARTLY | Sounds playing now are known ("Playing · n"); the limit and "audio CPU" come from the Profiler's Sound stage | |

### b.11 Screen 11: VFX, the effect editor (spec 828-907). All Shared 8.

| id | Element | Mark | What the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `vfx.emitter.add` | Emitter › **Emitter** (spec 836) | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.emitter.duplicate` | Emitter › **Duplicate** | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.module.spawn` | Add module › **Spawn** (spec 837) | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.module.velocity` | Add module › **Velocity** | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.module.color` | Add module › **Color** | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.module.size` | Add module › **Size** | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.module.collide` | Add module › **Collide** | PLACEHOLDER | Placeholder large button | Shared 8 |
| `vfx.sim.loop` | Simulation › "Loop" (spec 838) | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.sim.speed` | Simulation › "Speed: 1× ▾" | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.sim.bounds` | Simulation › "Show bounds" | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.preview.background` | Preview › "Background: Scene ▾" (spec 839) | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.preview.time` | Preview › "Time of day: Night ▾" | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.preview.follow` | Preview › "Follow emitter" | PLACEHOLDER | Placeholder small row | Shared 8 |
| `vfx.preview` | Center left: the effect preview, its pills and playback bar (spec 842-849) | PLACEHOLDER | One placeholder page for the center | Shared 8 |
| `vfx.stack` | Center right: the emitter stack (spec 851-877) | PLACEHOLDER | Part of the same page | Shared 8 |
| `vfx.shelf.module` | Shelf tab **Module · name**: the gradient editor (spec 883-889) | PLACEHOLDER | Placeholder shelf tab | Shared 8 |
| `vfx.shelf.parameters` | Shelf tab **Parameters · N** | PLACEHOLDER | Placeholder shelf tab | Shared 8 |
| `vfx.shelf.performance` | Shelf tab and column **Performance** (spec 891-895) | PLACEHOLDER | Placeholder shelf tab | Shared 8 |

### b.12 Screen 12: File, Project settings (spec 909-987)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `proj.changes-pill` | "N changed from defaults" pill, the dot and **Reset** on a changed row, the Changed from defaults card (spec 923, 934, 964) | EXISTS | (shell) over each real setting's standard value (as Properties' dot and ↺ do today) | |
| `proj.search` | "Search project settings" | EXISTS | `settings_window.rs` search | |
| `proj.tab.general` | Sub-tab **General** | EXISTS | Rows below | |
| `proj.tab.rendering` | Sub-tab **Rendering** (undesigned) | PARTLY | Holds Solis's 2D settings (`area::pixels::PixelSettings`) unless they get a 2D sub-tab (decision D9); nothing else to set yet. **NEEDS MOCK-UP.** | Shared 9, 3D 2 |
| `proj.tab.physics` | Sub-tab **Physics** (undesigned) | PLACEHOLDER | Placeholder sub-tab and page | not on the roadmap |
| `proj.tab.input` | Sub-tab **Input** | EXISTS | b.13 | |
| `proj.tab.audio` | Sub-tab **Audio** (undesigned) | PLACEHOLDER | Placeholder sub-tab and page | not on the roadmap |
| `proj.tab.platforms` | Sub-tab **Platforms** (undesigned) | PLACEHOLDER | Placeholder sub-tab and page | Shared 11 |
| `proj.tab.build` | Sub-tab **Build** (undesigned) | PARTLY | What Export for playtesting remembers: the game's name, the area it starts in, windowed or fullscreen. **NEEDS MOCK-UP.** | Shared 11 |
| `proj.tab.plugins` | Sub-tab **Plugins** | PLACEHOLDER | Placeholder sub-tab and page | Shared 11 |
| `proj.name` | Project name (spec 940) | EXISTS | `WorkbenchAction::RenameProject` | |
| `proj.company` | Company | PLACEHOLDER | Placeholder setting row | Shared 11: the full export |
| `proj.version` | Version (the game's) | PLACEHOLDER | Placeholder setting row | Shared 11 |
| `proj.icon` | Project icon | PLACEHOLDER | Placeholder setting row | Shared 11 |
| `proj.game-start` | Game start level (spec 949) | EXISTS | The export's "Starts in" area (`export_dialog.rs` `usable_area`) | |
| `proj.editor-start` | Editor start level | PLACEHOLDER | Placeholder setting row | not on the roadmap (small) |
| `proj.game-mode` | Default game mode | PLACEHOLDER | Placeholder setting row. Solis has no game modes (game-wide objects do that job): decision D12 may drop it. | not on the roadmap |
| `proj.frame-rate` | Target frame rate | PLACEHOLDER | Placeholder setting row | not on the roadmap |
| `proj.engine-version` | Engine version, "up to date", Check now (spec 958) | EXISTS | `AppInfo`, `updates.rs` | |
| `proj.scripting` | Scripting "Ledger" (spec 959) | PARTLY | Shows where game code opens (the computer's program for the file, `app/src/external.rs`); there is nothing to choose. Decision D11. | not on the roadmap |
| `proj.units` | Units and axes | PLACEHOLDER | Placeholder setting row showing Solis's fixed "Metres · Z up" | not on the roadmap |
| `proj.version-control` | Version control card: Check in, Review (spec 965) | PLACEHOLDER | Placeholder card | Shared 11 |
| `proj.status` | "Saved · not checked in" (spec 987) | PARTLY | "Saved" is real; "not checked in" belongs to `proj.version-control` | Shared 11 |

### b.13 Screen 13: File, Project settings › Input (spec 989-1062)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `input.contexts` | Context chips and "+ Context"; "Block lower contexts" (spec 995, 1040, 1055) | PLACEHOLDER | One placeholder chip row; no toggle. Solis's actions are one flat list. | not on the roadmap |
| `input.device-filter` | All devices / Keyboard & mouse / Gamepad (spec 996) | EXISTS | (shell) over `input::Bindings` | |
| `input.search` | "Find action or key" (spec 997) | EXISTS | (shell) | |
| `input.add` | **Add action** (spec 998) | EXISTS | `input_window.rs` Add button, Add axis | |
| `input.table` | The bindings table: Action, Type, Keyboard & mouse, Gamepad; keycaps, pad chips, "+" (spec 1000-1022) | EXISTS | `input_window.rs`, `input::bindings::Action` | |
| `input.trigger` | The Trigger column and "Fires on" (spec 1003, 1038) | PLACEHOLDER | Column not drawn; placeholder row in the details card. Game code asks pressed, held or released itself. | not on the roadmap |
| `input.groups` | Group header rows MOVEMENT, INTERACTION… (spec 1006) | PLACEHOLDER | Not drawn: one flat list | not on the roadmap |
| `input.listening` | The listening keycap; Esc cancels (spec 1025, 1053) | EXISTS | `input_window.rs` `listening` (`KeyOwner::Binding`) | |
| `input.conflict` | Amber keycaps, the warning triangle, the banner (spec 1026-1027) | EXISTS | (shell): the same input on two actions is found from `Bindings`. The banner's fix link needs contexts and is left out. | |
| `input.details` | Action details card: name, type, bindings, + Add binding (spec 1031-1037) | EXISTS | `input_window.rs` | |
| `input.buffer` | Buffer (spec 1039) | PLACEHOLDER | Placeholder row | not on the roadmap |
| `input.dead-zone` | Device settings › Stick dead zone (spec 1044) | PLACEHOLDER | Placeholder row | not on the roadmap |
| `input.sensitivity` | Device settings › Mouse sensitivity | PARTLY | Per axis action today: "Mouse speed" and "Stick and key speed" (`input_window.rs`), shown in the details card | |
| `input.invert-y` | Device settings › Invert look Y | PLACEHOLDER | Placeholder row | not on the roadmap |
| `input.players-rebind` | Device settings › Players can rebind | PLACEHOLDER | Placeholder row. (A game's own pause menu can already rebind, decision 0091.) | not on the roadmap |
| `input.test` | Test input card (spec 1049) | EXISTS | (shell) over `Workbench::devices` and `pad_now` | |
| `input.status` | "11 actions · 4 contexts · 1 conflict" (spec 1062) | EXISTS | (shell); contexts left out | |

### b.14 The Build tab (undesigned, spec 1183) and what crosses screens (spec 1064-1146)

| id | Element | Mark | Driven by, or what the placeholder shows | Filled in |
| --- | --- | --- | --- | --- |
| `build.export` | Export for playtesting (Solis's own) | EXISTS | `export_dialog.rs`, `app/src/playtest.rs`. **NEEDS MOCK-UP.** | |
| `build.targets` | Build targets, platforms, the build log (spec 1183) | PLACEHOLDER | A card beside Export for playtesting on the Build tab (round 3, `10-build-tab`) | Shared 11 |
| `build.target.windows` | Build › Targets › Windows (round 3) | PLACEHOLDER | A ribbon button (M15 gave Build's unfinished parts ids of their own) | Shared 11 |
| `build.target.platforms` | Build › Targets › Platforms (round 3) | PLACEHOLDER | A ribbon button | Shared 11 |
| `build.full` | Build › Full export › Build (round 3) | PLACEHOLDER | A ribbon button | Shared 11 |
| `build.installer` | Build › Full export › Installer (round 3) | PLACEHOLDER | A ribbon button | Shared 11 |
| `build.rust-code` | Build › Full export › Your Rust code (round 3) | PLACEHOLDER | A ribbon row to tick | Shared 11 |
| `build.log` | Build's Build log shelf tab (round 3) | PLACEHOLDER | A shelf tab; what an export says goes to the Console | Shared 11 |
| `cross.show-in-library` | A placed entity › Show in library (spec 1097) | EXISTS | `Workbench::pick_in_library` | |
| `cross.code-links` | Every link that would open code opens it in the user's code editor (spec 1105) | EXISTS | `WorkbenchAction::OpenExternally`, `app/src/external.rs`; the line number isn't passed (unbuilt piece `home.console.line-link`) | |
| `cross.bake-now` | Problems › "Bake now" (spec 1099) | PLACEHOLDER | Not drawn (the problem it belongs to can't arise) | 3D 5 |
| `cross.selection` | One selection everywhere (spec 1107-1113) | PARTLY | Entities: `document::Selection`, shared by the viewport and the Scene column. A file picked in the Library is its own selection (`LibraryState::selected`), and stays so. | |
| `cross.play-changes` | Changes while playing are temporary; "Keep changes" (spec 1127) | EXISTS | `play_changes.rs`: asked at Stop, Keep or Undo all | |
| `cross.error-pins` | Runtime errors pinned to a code line with the frame number (spec 1128) | PLACEHOLDER | Not drawn (part of the code editor); errors still land in Console and Problems | not on the roadmap |
| `cross.persistence` | Each workspace remembers its shelf tab and height, split, documents, ribbon state (spec 1131-1138) | EXISTS | (shell), saved with the settings | |
| `cross.responsive` | Ribbon scrolls; toolbars, columns and cards wrap; groups stack; grids reflow (spec 1140-1146) | EXISTS | (shell) | |

### b.15 Counts

338 elements: **125 EXISTS** (of which 40 are pure shell, built in this phase), **43 PARTLY**, **170 PLACEHOLDER**. By screen:

| Screen | EXISTS | PARTLY | PLACEHOLDER |
| --- | --- | --- | --- |
| The shell | 18 | 6 | 4 |
| 1 Home, level editor | 24 | 6 | 19 |
| 2 Home, placing | 11 | 4 | 12 |
| 3 Library | 14 | 5 | 8 |
| 4 Shading | 0 | 0 | 17 |
| 5 Animate | 1 | 5 | 11 |
| 6 Debug | 4 | 4 | 11 |
| 7 File, Recent projects | 14 | 2 | 8 |
| 8 File, Preferences | 12 | 1 | 3 |
| 9 Model viewer | 1 | 0 | 27 |
| 10 Audio | 4 | 4 | 10 |
| 11 VFX | 0 | 0 | 18 |
| 12 File, Project settings | 7 | 4 | 12 |
| 13 File, Input | 9 | 1 | 7 |
| Build tab and cross-screen | 6 | 1 | 3 |

Of the 164 placeholders, 62 are three whole unbuilt screens (Shading 17, the model viewer 27, VFX 18). **83 placeholders have no roadmap phase** ("not on the roadmap"): the model viewer (26), placing's options (12), the mixer's effects, sends, pan and snapshots (10), input contexts and device settings (7), the Library's tags, used-by and import queue (6), the Debug tab's debugger and memory tools (6), project settings such as frame rate and units (6), and ten smaller ones. The other 81 name their phase: Shared 7 (19), Shared 8 (19), Shared 10 (16), Shared 11 (13), 3D 5 (4), Shared 6 (4), Shared 13 (3), and one each for 3D 1, Shared 4 and 2D 2. the plan's decision D12 asks what to do with those.

---

## (c) Keys and mouse actions

The spec's table is spec 1070-1087; keys and mouse actions named elsewhere in the spec are added with their line. "Today" is `docs/manual/keyboard-shortcuts.md`. **Settled** means the project lead's answer 1 decides it: today's keys stay, the design's keys are used only for new things. Every row marked **ASK** is a new key or mouse action that needs the lead's answer before the milestone that builds it (CONTRIBUTING.md rule 17); the plan's decision D15 gathers them.

### c.1 The spec's shortcut table

| # | Action in the design | Design's key | Today in Solis | Verdict | What each choice would break |
| --- | --- | --- | --- | --- | --- |
| 1 | Command search | Ctrl K (spec 1074) | **Space**, with nothing being typed (decision 0085); Ctrl + K is unused | CLASH, **settled: Space** | Taking the design's key would retire a key chosen by the lead and learnt; keeping Space costs nothing, since Ctrl + K is free either way. The search box's hint reads "Space". |
| 2 | Select / Move / Rotate / Scale | Q / W / E / R (spec 1075, 183) | **Q / W / R / S**, and **T** for all three | CLASH, **settled: Q W R S T** | The design's E and R would move Rotate off R and Scale off S, lose T, and collide with E as "fly up" (right mouse held) and E as the UI editor's tool ring. Keeping today's keys means the toolbox's hover texts show Q, W, R, S, T, and the toolbox gets a fifth button (Transform). |
| 3 | Frame selection | F, in viewports and the node graph (spec 1076) | F points the camera at the selection; F fits the Animator's graph | SAME | |
| 4 | Add node | Tab, in a node graph (spec 1077, 365) | Tab shows a tile map's art or meaning while painting; Tab finishes a name in the Console. The Animator's graph adds a state by right-click. | NEW, **not built now** | The only node graph in the design is Shading's, a placeholder. When Shared 7 builds it the key applies to every graph (rule 21), so the Animator would gain it too: ask then. No clash with painting's Tab, since `key_owner.rs` gives keys to the tab under the pointer. |
| 5 | Fold or unfold the shelf | Ctrl + Space (spec 1078) | Ctrl + Space pops the Library drawer up or away | CLASH in meaning, **settled: folds the shelf** | The Library drawer goes (a.3): the shelf's Assets column is what it showed, so the key still "brings the files up". `library_drawer.rs` and its tests are removed. |
| 6 | Place and keep the asset loaded | Shift + release (spec 1079) | Nothing while dropping a file. Elsewhere Shift means "ten times finer" on a gizmo drag and "a run" in lists. | NEW, **ASK** | Recommended: as designed. No existing meaning during a drop. |
| 7 | Place without snapping | Alt while dragging (spec 1080) | Solis's key for "flip snapping for this drag" is **Ctrl** (gizmo drags). Alt + click picks the terrain under the pointer when painting. | CLASH of convention, **ASK** | Recommended: **Ctrl**, so one key means "flip snapping" everywhere (rule 21). Taking Alt would give Solis two keys for one idea; the hint strip's chip would read "Ctrl". |
| 8 | Rotate the ghost 15° | R while dragging (spec 1081) | R is the Rotate tool, but no tool key acts during a drag. A held tile copy is turned with Z. | NEW, **ASK** | Recommended: as designed (R is "rotate" in both senses). The step should be the ribbon's Angle step, not always 15°. |
| 9 | Cancel placement | Esc (spec 1082) | Esc cancels a box or a gizmo drag | SAME | |
| 10 | Rename an asset | F2, in the Library (spec 1083) | F2 renames the selected **entity**, wherever the pointer is; a file is renamed from its right-click menu | CLASH, **ASK** | Recommended: F2 renames the picked file while the pointer rests over the Library grid or the shelf's Assets column, and the entity otherwise: the rule the editor already has ("where the pointer rests decides", `key_owner.rs` gets a `Library` owner). Leaving it as today means F2 over the Library renames an entity the user isn't looking at. **Delete** has the same question (spec 327). |
| 11 | Select several | Ctrl / Shift + click, in the Library, scene list and dope sheet (spec 1084) | Scene list and Frames tab: the same. Library: one file at a time. | SAME for the scene list; NEW for the Library (`lib.multi-select`, a placeholder) | |
| 12 | Play animation, previous key, next key | Space, and shelf bar buttons (spec 1085, 443) | Space opens the search, except over the Frames tab, where it plays the clip | CLASH, **settled: Space is the search**; over the Frames tab (and later the sequencer) it plays, as today | Making Space "play" everywhere would take the search's key. The existing rule already gives Space to the timeline under the pointer. |

### c.2 Keys and mouse actions named elsewhere in the spec

| # | Action (spec line) | Today | Verdict |
| --- | --- | --- | --- |
| 13 | Long press a Create button for a list of kinds (174) | Menus list the kinds (Entity menu) | NEW mouse action, **ASK**. Recommended: click makes the usual kind, and a small ▾ on the button (or a right-click) opens the list; a long press is slow and can't be found by looking. |
| 14 | Click the value in "Grid · 0.25" to edit the step (175) | "Choose the step" opens a list | SAME idea; the list stays |
| 15 | Drag a field's label to scrub its value (214) | Drag the value itself | NEW, **ASK**. Recommended: both. |
| 16 | Right-click a card for reset, copy, remove (214) | The card's ⋯ menu | SAME actions, one more way in; ⋯ stays |
| 17 | Click a Place tile, then click in the scene; "a shortcut drops the selected asset under the mouse" (282) | Double-click a file places it where the camera looks | NEW, **ASK**: the spec names no key. Recommended: no new key now; click-then-click stays a placeholder (`place.click-to-place`). |
| 18 | Double-click an asset opens its editor; a mesh opens the model viewer (328, 678) | Double-click **places** a model, assembly, sprite sheet or tile map, and opens every other kind | CLASH, **ASK**. Recommended: in the shelf's Assets and Place columns double-click places, as today; in the Library tab it only selects (the Library "is not where objects are placed", spec 291) and **Place** places. |
| 19 | Drag a tile onto the Home tab (329) | Nothing | NEW, **ASK** (`lib.drag-to-home`). Recommended: later; Place does it. |
| 20 | Drag tiles onto a folder tile or a breadcrumb part (326) | Drag onto a folder in the tree | SAME idea, new targets |
| 21 | Node graph: middle-drag pans, wheel zooms, left-drag selects with a box (351) | The Animator's graph: the same (decision 0057) | SAME |
| 22 | Drag the shelf's grip to resize it (157, 1122) | Drag the line between two panels | NEW, replaces the old one; obvious |
| 23 | Drag a document tab to an edge to split or merge groups (151, 1121) | Drag a panel by its tab to dock it | NEW, replaces docking. **ASK** only for where a drop may land (recommended: left or right half of the center, or onto another group's tab strip). Built last (M15). |
| 24 | Ribbon scrolls sideways (146) | No ribbon | NEW: the wheel over the ribbon scrolls it. **ASK** (small). |
| 25 | Click the status bar's counts (164) | The same | SAME |
| 26 | Esc or Back to editor leaves the File view (612) | Esc leaves the launcher | SAME |
| 27 | Right-click a project card (608) | The row's ⋯ | SAME actions, one more way in |
| 28 | Console: Tab completes, Up and Down walk history (519) | The same; and ' jumps to the command line | SAME; ' stays |
| 29 | Mixer: double-click a fader resets it (822) | The same | SAME |
| 30 | Mixer: Alt-drag fine-tunes a fader (822) | Solis's key for "finer" is **Shift** (gizmo drags) | CLASH of convention, **ASK**. Recommended: Shift. |
| 31 | Mixer: click a meter to reset its peak; drag a routing line to another strip; drag an event row onto a strip (820-821) | Routing is the bus's "Inside" choice | NEW, **ASK** before M13; recommended: the first now, the two drags later |
| 32 | Input: Backspace clears a binding (1053) | × beside each input | NEW, **ASK**. Recommended: × stays; Backspace only on a keycap that is picked and not listening (while listening, Backspace is a key to bind). |
| 33 | Sequencer: click the ruler, arrow keys step a frame, Up and Down jump between keys, drag to retime, Alt-drag duplicates, drag clip edges (443-445) | Frames tab: click, Ctrl + click, Shift + click, drag to move, Delete, Space | For the sequencer: not built now. For the Frames tab re-housed in its look (M11): the arrow keys would be NEW, **ASK**. |
| 34 | Curve editor: drag keys and handles, Alt-drag breaks tangents (412) | Nothing | Not built now (Shared 10) |
| 35 | Profiler: click a bar, drag a range, click a block, double-click a script block, scroll to zoom time (473, 489) | Nothing is clickable | NEW, **ASK** before M14 (recommended: click a bar now; the rest with the features) |
| 36 | Model viewer: drag orbits, middle-drag pans, wheel zooms, Shift-drag turns the light (734) | The area viewport flies with the right mouse button | Not built now. When it is: a second way to move a camera, to be asked then. |
| 37 | VFX: drag module cards, Delete removes (899-900) | Nothing | Not built now (Shared 8) |

### c.3 Solis's keys the design doesn't mention: all kept

Undo and redo (Ctrl + Z, Ctrl + Y, Ctrl + Shift + Z); New, Open, Save, Save as (Ctrl + N, O, S, Shift + S); Delete; Duplicate (Ctrl + D); Select all (Ctrl + A); axis locks X, Y, Z; world or object axes (L); the fly camera (right mouse with W A S D Q E, Shift, wheel); the tile painter (B, U, N, G, I, Tab, Alt + click, Ctrl, X Y Z on a held copy); the UI editor (1 to 7, E for the ring, Enter, Backspace); the Console's '; Alt + Enter; Esc giving the keys back from the game; Enter and the arrows in the search list and on the projects page.

Two things to carry over with care:

- **Whose key it is** stays "the panel the pointer rests over" (`key_owner.rs`, review fix BUG-06). The shell must keep reporting which `Panel` is under the pointer for shelf columns and center groups, exactly as `PanelViewer::ui` does today (`FramesOut::pointer_over`).
- **The projects page** is a grid of cards, not a list: Left and Right join Up and Down (small, **ASK** with M8).

The manual's "Panels" table (drag a panel by its tab; drag the line between two panels) is replaced by rows 22 and 23.

---

## (d) Design tokens, ready to become constants

Proposed home: one new file, `crates/editor/src/tokens.rs` (colours, sizes, radii, type), which takes the place of `palette.rs` and of the neutral parts of `theme.rs`. `source_rules_tests.rs` then allows colours written as numbers only there. Names are proposals. **"from the HTML"** marks a value the spec doesn't give, read from the mock-up's own CSS; where the HTML and the spec disagree the row says so, and the plan's decision D18 (settled 2026-11-11, as recommended) says which wins. The values as built are `crates/editor/src/tokens.rs`; a test keeps that file and the specification's tables the same.

### d.1 Surfaces and lines (spec 35-51)

| Constant | Hex | Used for |
| --- | --- | --- |
| `APP` | #0E0E0E | Window root, tab bar, document tab strips, segmented wells, the Console's command line, dark text and icons on accent or light fills |
| `FIELD` | #121212 | Text and number fields, the Console, graph panels inside a group |
| `PREVIEW` | #111111 | Preview boxes (Library details, Shading and VFX previews, the camera preview) (spec 320, 367, 402) |
| `SHELF_BAR` | #141414 | Shelf bar, node-graph canvas, File pages, toolbox column, inactive document tab |
| `CODE` | #161616 | Code group, status bar, table header strips, the Library's browser, dope sheet |
| `CURVES` | #151515 | Curve editor, emitter stack (spec 408, 851) |
| `RIBBON` | #181818 | Ribbon body, the active ribbon tab, command search fill, File menu column, label boxes |
| `PANEL` | #1A1A1A | Shelf columns, project cards, mixer strips, tables |
| `VIEWPORT` | #1C1C1C | Viewport backdrop, the active document tab, a project card's thumbnail |
| `GROUP_GAP` | #1E1E1E | 1 px between editor groups; track and lane lines; File menu hover and active item |
| `CARD` | #1F1F1F | Cards in the shelf, module cards, band cards |
| `ROW_HOVER` | #222222 | Hovered rows; setting-row dividers; node body; insert slots |
| `HOVER` | #242424 | Hovered buttons; tile thumbnails |
| `LINE` | #262626 | Panel borders, 1 px separators, the active shelf tab, plain buttons, keycaps, chips |
| `CARD_BORDER` | #2A2A2A | Card borders; node headers; type badges |
| `FIELD_BORDER` | #2C2C2C | Field borders; slider tracks; the active segment; thumbnail borders; shortcut chips |
| `PILL_BORDER` | #2E2E2E | Floating pills |
| `STRONG_LINE` | #333333 | Toggle off; label-box, gradient-bar and plain-button borders; comment frames |
| `STRONGER_LINE` | #3A3A3A | Dashed "add" borders, chip and keycap borders, the shelf grip, checkbox outline |
| `NODE_BORDER` | #363636 | Node border (spec 353) |
| `PAD_BORDER` | #444444 | Gamepad chip border (spec 1008) |
| `PILL_FILL` | #121212 at 85% | Floating pills (spec 110); 90% for the camera presets and playback bar (spec 711, 849) |
| `LABEL_BOX_FILL` | #181818 at 95% | The placing label (spec 258); 94% for the hint strip (spec 259) |
| Viewport scene | sky #141414 → #2A2A2A to the horizon at 44% height; floor #202020; grid lines white at 5% | The viewport with no sky of its own (spec 184). Solis draws the area's real sky; these replace `theme::viewport::{sky_top, horizon, ground}`. |

### d.2 Text (spec 53-64)

| Constant | Hex | Used for |
| --- | --- | --- |
| `TEXT_EMPHASIS` | #FFFFFF | Selected row, active tab, the name in the Inspector header |
| `TEXT_PRIMARY` | #E6E6E6 | Values, headings, field contents; the document tab in front (from the HTML: the spec says white, spec 150) |
| `TEXT_TILE` | #D0D0D0 | Tile names, gamepad chip text (from the HTML) |
| `TEXT_BODY` | #C8C8C8 | Tree and list rows, ribbon button labels, column headers |
| `TEXT_LABEL` | #A6A6A6 | Field labels, inactive tabs, breadcrumbs, status bar |
| `TEXT_META` | #8C8C8C | Counts, kinds, hints, ribbon group labels, section caps. The lowest grey for readable text. |
| `TEXT_FAINT` | #6E6E6E | Log times, dB scale, "0 voices", empty slots |
| `TEXT_LINE_NUMBER` | #5E5E5E | Code line numbers |
| `TEXT_SEPARATOR` | #5A5A5A | The path's slash |

The spec promises 4.5:1 for all text on its surface (spec 64). Today's contrast tests (`theme.rs` `text_reads_on_every_surface`) are kept and pointed at these pairs. Measured: `TEXT_META` passes everywhere it is used (4.5:1 on `LINE`, 4.6 on `HOVER`, 5.3 on `RIBBON`), and dark text on every accent passes easily (7.0 to 13.0). Three greys are below the promise: `TEXT_FAINT` (3.4:1 on `PANEL`, 3.7 on `FIELD`), `TEXT_LINE_NUMBER` (2.8 on `CODE`) and `TEXT_SEPARATOR` (2.8 on `APP`). The spec itself calls #8C8C8C "the lowest gray used for readable text", so these three are for marks that repeat what something else says (a log line's time, a line number, a slash); the test lists them as the only allowed exceptions, and they are never used for a placeholder's name (decision D18).

### d.3 Accent and colours that mean something (spec 66-81)

| Constant | Hex | Rule |
| --- | --- | --- |
| `Accent::Blue` (default) | #5C9CFF | Selection, the active tool, active tab underline, focus, toggles on, playhead, progress |
| `Accent::Grey` | #D4D4D4 | "No colour" |
| `Accent::Orange` | #F2994A | |
| `Accent::Teal` | #4FC2A8 | |
| `Accent::Purple` | #B48CFF | |
| `ON_ACCENT` | #0E0E0E | Text and icons on a solid accent fill (weight 600). Today's rule was white on the accent (`white_text_reads_on_every_accent`): the test flips. |
| `LINK`, `LINK_HOVER` | #7FB2FF, #A9CBFF | Inline links |
| `SUCCESS`, `WARNING`, `ERROR` | #4FC28A, #E8B04A, #F06A6A | Dots, levels, icons; always with a word or icon |
| `ERROR_TEXT` | #F08A84 | Error text; Record and Auto key labels; broken references |
| `SUCCESS_TEXT` | #8FD4A8 | "up to date", "None" (spec 730, 958) |
| `AXIS_X`, `AXIS_Y`, `AXIS_Z` | #E5534B, #6CC24A, #4C8DFF | Gizmo, pivot. Today #E8545A, #58C26A, #4A7CFF (`theme::axis`). |
| `AXIS_X_TEXT`, `AXIS_Y_TEXT`, `AXIS_Z_TEXT` | #F08A84, #8FD47A, #7FB2FF | The small letters in number fields (10 px / 700 mono, from the HTML) |
| Asset kinds | Folder #C9A35A, Mesh #5FC4D6, Material #6CC28A, Texture #E57F6A, Animation #B48CFF, Audio #E8B04A, Prefab #7FB2FF | 7 × 7 px dot, radius 2. Solis has more kinds than seven (area, code, sprite sheet, clips, animator, tileset, tile map, picture's crops, screen, sound event, text): decision D17. |
| Pin kinds | Float #9A9A9A, Vector3 #E8C95A, Color #C38CFF, Texture #E57F6A | For Shared 7; unused now |
| Code colours | keyword #8FB8F0, string #B8CF9A, comment #7A7A7A, number #E0B878, default #D4D4D4, type #C8C8C8 / 500 (from the HTML) | For the code editor; unused now |
| `PLAY_FILL`, `PLAY_TEXT` | #E6E6E6, #0E0E0E | The one light button. Today Play is blue (`theme::GO`, decision 0060). |
| Console rows | warning row: #E8B04A at 6%; error row: #F06A6A at 8% (spec 514-515) | |
| Conflict keycap | fill #2E2617, border #8A6A2A, text #F2D08A; banner fill #1F1B12, top line #3A3020, text #E8D2A2 (spec 1026-1027) | |
| Problem badge | fill #4A2422, text #F08A84, 16 px circle, "!" 10 px / 700 (from the HTML) | |
| Profiler | bars #4A4A4A, over budget #A6A6A6, budget lines #4A4A4A and #3A3A3A; GC block #4A2A2A with #FFD6D3 text (spec 470-481) | |
| Mixer | meter gradient #6E6E6E → #9A9A9A, hot #E8B04A, effect-on dot #4FC28A; fader cap #5A5A5A → #3A3A3A, border #6E6E6E (spec 764-768) | |
| VFX sample colours | #FFF8E0, #FFF2C8, #F2C46D, #E57F6A (spec 846, 887) | Sample content; unused now |

### d.4 Accent mixes (spec 71 gives "16-24%"; the exact values are from the HTML)

One helper: `tokens::mix(accent, over, share)`, a straight blend of the two colours' stored values, which is what the mock-up's CSS does. Mixed in code against the known surface, never by drawing a see-through colour, so the result is the same on every backend.

| Constant | Share of accent, over | Used for |
| --- | --- | --- |
| `SELECT_ROW` | 22% over the row's surface (`PANEL`) | Selected rows of a tree or list: the scene list. *Corrected in M1: first written here as 16%; the design's first screen and the mock-up both have 22%, and a test now samples it.* |
| `SELECT_ITEM` | 16% over `PANEL` | Selected table rows (LOD rows, the sound-event table) and cards; 18% for a selected track |
| `SELECT_TILE` | 16% over the tile's surface; plus a 2 px accent inner outline on the thumbnail | Selected tiles |
| `ACTIVE_BUTTON` | 22% over `RIBBON` | The active large ribbon button (white text) |
| `SELECT_HEADER` | 16% over `RIBBON` | Selected emitter header, selected strip's header |
| `SELECT_LANE` | 8% over the lane | The selected lane; the selected module (8% over `CARD`) |
| `SELECT_INPUT_ROW` | 12% over `PANEL`, with a 2 px accent bar at the left | The selected row in the input table |
| `LISTENING` | 20% over `PANEL`, dashed accent border | A keycap that is listening |
| `CHANGES_PILL` | 14% over `SHELF_BAR` | "N changed from defaults"; selected band card (14% over `CARD`) |
| `OPEN_CARD_BORDER` | 60% over `CARD_BORDER` | The open project's card |
| `CULPRIT_BLOCK` | 45% over #222222 | The picked block in the frame timeline |
| `CONTEXT_TAB_LINE` | 55%, a 2 px line inside the tab's top edge | Contextual tabs |
| Ghost | fill 22%, footprint 16% with an 80% outline, guides 55%, bounds box 80% | Placing (spec 253-255, 705) |
| Graph fills | 8% under the EQ curve, 14% under the particle count, 10% in UV islands | Unused now |

### d.5 Type (spec 83-96)

| Constant | Font | Size / weight | Notes |
| --- | --- | --- | --- |
| `UI` | Inter | 12 / 400 | The default |
| `TAB`, `TAB_ACTIVE` | Inter | 12 / 400, 12 / 500 | Ribbon tabs, shelf tabs |
| `RIBBON_LABEL` | Inter | 10 / 400 under an 18 px icon | Play's label is 600 (from the HTML) |
| `RIBBON_SMALL` | Inter | 11 / 400 | Small rows (from the HTML) |
| `GROUP_LABEL` | Inter | 10 / 400, letter-spacing 0.04 em | `TEXT_META` |
| `HEADER` | Inter | 11.5 / 600 | Column and panel headers, card titles |
| `ROW` | Inter | 11.5 / 400 | Rows, labels, document tabs |
| `MONO` | JetBrains Mono | 11 / 400 | Numbers in fields, log lines, timecodes |
| `MONO_SMALL` | JetBrains Mono | 10 to 10.5 / 400 | Paths, keycaps, shortcut chips, rulers; 9 to 9.5 for dB and frequency scales |
| `MONO_BOLD` | JetBrains Mono | 10 / 700 | Axis letters, the playhead's flag |
| `CODE_TEXT` | JetBrains Mono | 11.5 / 400, line 19 px | Unused now |
| `PAGE_TITLE` | Inter | 22 / 700 | File pages |
| `PAGE_SUBTITLE` | Inter | 12.5 / 400 | `TEXT_META` |
| `SECTION_CAPS` | Inter | 10.5 / 600, upper case, letter-spacing 0.08 em | Settings sections, table headers |
| `SETTING_NAME`, `SETTING_HINT` | Inter | 12.5 / 500; 11.5 / 400, line 1.45 | Setting rows |
| `MENU_ITEM` | Inter | 13 / 500 | File menu column |
| `CONTEXT_PREFIX` | Inter | 9.5 / 600, upper case, letter-spacing 0.06 em | "LIBRARY TOOLS" |
| `CARD_NAME` | Inter | 13 / 600 | Project cards, template headers; 14 / 600 for a details title |
| `TILE_NAME`, `TILE_KIND` | Inter | 11 / 400 (10.5 in the shelf); 10.5 / 400 | Cut off with "…" |
| `BADGE` | Inter | 9.5 to 10.5 / 500 to 600 | "3D", "GPU", "dragging" |

**Fonts.** The design uses Inter at 400, 500, 600 and 700 and JetBrains Mono at 400 and 700. Solis bundles `Inter-Regular.ttf`, `Inter-SemiBold.ttf` and `JetBrainsMonoNL-Regular.ttf` (`crates/editor/assets/fonts/`, read in `theme.rs:240-243`), with their licence texts in `licenses/fonts/Inter-OFL-1.1.txt` and `JetBrainsMono-OFL-1.1.txt`, listed in `licenses/README.md`. So:

- **To add:** `Inter-Medium.ttf`, `Inter-Bold.ttf`, `JetBrainsMonoNL-Bold.ttf` (about 0.4 MB, 0.4 MB and 0.3 MB). egui has no notion of weight: each weight is a named family, as `theme::SEMIBOLD` is today, so M1 adds `MEDIUM`, `BOLD` and `MONO_BOLD` families.
- **Licence:** both families are under the SIL Open Font License 1.1, which allows bundling with any software as long as the fonts aren't sold by themselves and the licence text travels with them. The mock-up's own file states the same licence for the fonts packed into it. The texts already in `licenses/fonts/` cover the new weights (same families, same copyright lines); `licenses/README.md`'s two rows change to name the weights ("regular, medium, semibold and bold"; "regular and bold"). `cargo deny` covers code libraries only and needs nothing. The fonts must come from the families' own releases, unchanged, at the versions already bundled.
- **RENAMING.md:** nothing: no font file or licence carries the program's name. The only new place a name could appear is the logo tile, which already uses `branding::LOGO_PNG` (item 8 of the checklist).
- **Game UI is separate:** `app/src/gpu.rs` keeps its own Inter regular and semibold for game screens (review finding DEP-01 is about that library): the editor's new weights must not be added to the game's font set.
- `theme.rs`'s `symbols_have_glyphs` test keeps running over every source file, and now over the new families too.

### d.6 Sizes and spacing (spec 98-113, with the HTML's exact paddings)

| Constant | Value | Notes |
| --- | --- | --- |
| `TAB_BAR_H` | 34 | Padding 0 × 10, gap 4 between its parts (from the HTML) |
| `RIBBON_TAB` | 30 tall, 12 side padding, bottom-aligned, 6 px top corners, 2 px accent underline | |
| `LOGO_TILE` | 20 × 20, radius 5, 8 after it | Radius from the HTML |
| `SEARCH` | 24 tall, 380 wide, at most 36% of the width, radius 6, padding 0 × 10, gap 8; icon 12 px; hint 10 px mono | |
| `RIBBON_H` | 66; without labels 46 ("save 20 px", spec 649) | |
| `RIBBON_GROUP` | Padding 4 top, 10 sides, 2 bottom; 1 px `LINE` at its right; 2 between buttons | From the HTML |
| `RIBBON_BIG` | 50 × 44, radius 6, icon 18, 4 between icon and label | |
| `RIBBON_SMALL_ROW` | 20 tall, padding 0 × 8, gap 6, radius 4 | |
| `DOC_TAB_H` | 30; padding 0 × 12; gap 7; 1 px `APP` at its right; 1 px accent line on top when in front | |
| `TOOLBOX` | 44 wide column; buttons 34 × 34, radius 7, icon 15; divider 22 wide | |
| `SHELF_BAR_H` | 34; padding 0 × 10; gap 4 | |
| `SHELF_GRIP` | 36 × 4, radius 2 | |
| `SHELF_TAB` | 26 tall, padding 0 × 12, radius 6 | |
| `SHELF_H` | Bar + content of about 290 to 360 (spec 128); the sequencer's about 360 | Default per workspace: decision D18 |
| `COLUMN_HEADER_H` | 30; padding 0 × 12; gap 8 | |
| `ROW_H` | 24 (22 compact); padding 0 × 12 (0 × 10 on Screen 1: the HTML has both; use 12) | Tree children indent 28 (spec 205); track children 18 (spec 423) |
| `TRACK_H`, `LANE_H` | 26 | |
| `FIELD` | 24 tall, radius 5, padding 0 × 7, gap 6 | |
| `ICON_BUTTON` | 26 × 26, radius 5 | Hover fill: the spec says #242424 (spec 118), the HTML #262626 |
| `PILL` | 24 tall, radius 5, padding 0 × 8, gap 6, 11 px text | |
| `STATUS_BAR_H` | 24; padding 0 × 12; gap 14; dot 6 px | |
| `CARD` | Radius 8, padding 10, gap 8 inside, 10 between cards, at least 180 wide (grows from 190) | |
| `TILE` | Square, radius 6; grid gap 4; at least 108 wide (Library) or 92 (Place); 4 columns in the Object tab's Assets column | |
| `KV` | Key column 96, gaps 7 down and 10 across | From the HTML |
| `TOGGLE` | 32 × 18, radius 9, knob 14 with 2 px inset (spec 122 allows 28 to 32 × 16 to 18) | |
| `CHECKBOX` | 12 × 12, radius 3 (spec 123 allows 12 to 14) | |
| `SEGMENTED` | Well padding 2, radius 6, 1 px `LINE` border; segments 20 tall, padding 0 × 9, radius 4, 11 px / 500 | From the HTML |
| `CHIP` | 24 tall, radius 12, padding 0 × 9 (Library kind chips); 22 tall, radius 11, padding 0 × 10 (Place filters); 26 tall, radius 13 (input contexts) | |
| `SWATCH` | 22 × 22, radius 6, gap 8; chosen: a 2 px ring of the page colour, then a 2 px ring of its own colour | |
| `SLIDER` | Track 4 tall, radius 2; knob 12 | |
| `KEYCAP` | 22 tall, at least 22 wide, padding 0 × 7, radius 5, 2 px bottom border | |
| `SHORTCUT_CHIP` | Mono 10 px, padding 1 × 5, radius 3, fill `FIELD_BORDER` | From the HTML |
| `DASHED_BUTTON` | 22 tall, padding 0 × 10, radius 5, 1 px dashed `STRONGER_LINE`, 11 px / 500 `TEXT_LABEL` | From the HTML ("+ Component") |
| `PLAIN_BUTTON`, `PRIMARY_BUTTON` | 30 or 32 tall, radius 6 or 7; plain: fill `LINE`, border `STRONG_LINE`, 500; primary: accent fill, `ON_ACCENT`, 600 | |
| `FILE_MENU` | Column 220 to 240 wide, padding 16 × 10; items 36 tall, padding 0 × 14, radius 7, icon 15, gap 10; separator 1 px with 8 × 6 margin | |
| `FILE_PAGE` | Padding 28 top and bottom, 36 sides; 22 between blocks; settings column at most 760; control column 260; row padding 12; right column 260 to 330 | |
| `PROJECT_CARD` | At least 240 wide, gap 16, radius 10, thumbnail 140 tall, body padding 12 × 14, gap 5 | |
| `TEMPLATE` | 150 wide, thumbnail 84 tall, radius 8, icon 24, gap 14 | |
| `LABEL_BOX` | 208 × 52, radius 7 (placing); hint strip 30 tall, radius 8 | |
| Shadows | Node: 0 6 18 black 35%; selected node: 1 px accent ring + 0 8 24 black 45%; fader cap: 0 2 4 black 40% | From the HTML. Nothing else has a shadow (spec 33). |
| Lines | Every border and separator is 1 px; selection outline in the viewport 1.5; gizmo arrows 2.5; wires 2 (2.5 from the picked node); playhead 2 | |

### d.7 States (spec 115-124)

| State | Rule |
| --- | --- |
| Hover | Buttons: fill `HOVER`, text `TEXT_PRIMARY`. Rows: fill `ROW_HOVER`. File menu items: fill `GROUP_GAP`. |
| Selected | Accent-mixed fill (d.4) + `TEXT_EMPHASIS`; tiles add the 2 px inner outline |
| Active tool, main action | Solid accent fill, `ON_ACCENT` icon or label |
| Focused or just-edited field | Border turns accent |
| Toggle | On: accent track, `APP` knob at the right. Off: `STRONG_LINE` track, `TEXT_META` knob at the left. |
| Checkbox | On: accent fill and border. Off: `STRONGER_LINE` outline. |
| Unsaved | A 6 to 7 px `WARNING` dot after a document's name |
| Muted strip | The whole strip at 55% |
| Being dragged | The tile at 55% with a dashed accent border and a "dragging" badge |
| Disabled | Not in the spec. Proposed: text `TEXT_FAINT`, no hover (decision D18). The placeholder look is a different thing (the plan). |
| Icons | 16 px grid, stroke 1.3 (18 px ribbon icons), 1.5 (12 to 15 px), up to 1.8 to 2 for the smallest chevrons and the logo (from the HTML); drawn 11 to 18 px; current text colour |

### d.8 What the spec leaves out altogether

Density's numbers (what Compact and Comfortable change), the interface scale's steps, dialogs, tooltips, scroll bars, text selection colour, disabled controls, menus and dropdown lists (their fill, border and row height), the look of a placeholder, focus rings for keyboard use. Each is in the plan's decision D18 with a proposal.

---

## (e) What egui doesn't give for free

Solis draws with egui 0.36.2 on wgpu 30 (through eframe), docks with egui_dock 0.21.1, and tests whole windows with egui_kittest 0.36.2 at 1440 × 900, the mock-ups' own size (`test_support.rs:99`). "Cost" is new code without tests, as a rough count of lines.

### e.1 Docking: can egui_dock be styled into editor groups plus a shelf?

**How panels are docked today.** `Workbench::dock` is an `egui_dock::DockState<Panel>`; `workbench.rs:1147` draws it with `DockArea::new(..).show_inside(ui, &mut PanelViewer {..})`; `panels.rs` implements `TabViewer` (the tab's title, its icon painted in `on_tab_button`, its contents in `ui`); `theme.rs:458` `dock_style` sets the colours; `layout.rs` and `workspace.rs` build the standard trees; the trees are saved under the settings keys `panel_layout2` and `workspaces` (`app/src/window.rs:32-35`).

**What egui_dock 0.21 can be styled to do** (its `Style`: `tab_bar.height`, `bg_fill`, `hline_color`; `tab.active` / `inactive` fills, text colours and corner radii; `separator.width` and colours; `DockArea::allowed_splits`, `draggable_tabs`, `show_close_buttons`, and no floating windows): the **center's document tab strips** would come out close to the design (30 px strip on #0E0E0E, tabs #141414 and #1C1C1C, the 1 px accent top line painted in `on_tab_button`, a 1 px #1E1E1E split), and drag-to-split comes with it.

**What it cannot do:**

1. **The shelf.** Pill tabs with a grip before them and tools after them on the same bar, and content laid out as columns that wrap: none of it is a tab group. The shelf has to be Solis's own code whatever the center uses.
2. **Things at the right end of a tab strip** ("‹ Back to Library", spec 698): `TabViewer` has no hook for it (only an add button).
3. **Groups with a header instead of tabs** ("Preview", "Curves · Rotation Y", "UV layout", "Emitters 3", "SFX › EQ"): a leaf always has a tab bar; the header would have to be faked as a lone tab.
4. **Stacking groups vertically when the window is narrow** (spec 1145), a split ratio remembered per workspace as one number, and a fixed number of groups: the tree is free-form, and every one of these means fighting it (today's `layout.rs` already spends 440 lines checking saved trees).
5. **A saved format Solis owns.** `DockState` is the library's own shape; what the design needs to remember (spec 1131-1138) is four small values per workspace.

**Recommendation: replace it.** Write `shell/groups.rs`: one or two groups (three for the VFX and Animate screens' needs at most), each a strip of document tabs or a header, a draggable 1 px divider with a remembered ratio, stacking when narrow. About 450 lines; drag-a-tab-to-split another 250, built last (M15) because it is the only part that is hard and nothing else waits for it. This is the exit `docs/dependencies.md` already names for egui_dock ("write docking into the editor"), and it removes a dependency rather than adding one. `Panel` stays as the unit of content: `PanelViewer::ui(ui, panel)` already draws any panel into any `Ui`, and `key_owner.rs` keeps working because it only asks which `Panel` the pointer is over.

**Cost of the other choice** (keep egui_dock for the center only): about 200 lines of style and workarounds for points 2 to 4, two layout systems, and the saved-layout checks kept.

### e.2 The rest, one by one

| Thing in the design | What egui gives | How to do it in Solis | Cost |
| --- | --- | --- | --- |
| **The ribbon** (spec 140-146) | Nothing like it | A table per tab (`shell/ribbon.rs`): tab → groups → controls (`Big`, `Check`, `Drop`, `Row`), each either a command or an unbuilt id. Drawn in a horizontal `ScrollArea` with its bar hidden, so it scrolls instead of wrapping. A large button is one `allocate_response` of 50 × 44 with the icon and label painted. Being data is what makes "hide unfinished parts" close the gaps: unbuilt controls are filtered out before layout. | 400, plus about 30 lines of table per tab |
| **Tab bar and contextual tabs** (spec 130-138) | `Panel::top`; per-corner radii (`CornerRadius`) | One row: logo, tabs, search, path, chevron. A contextual tab is one more entry, worked out each frame from what is open, with the prefix as a second text run and the 55% line painted. | 250 |
| **The shelf's wrapping columns** (spec 156-158, 1144) | `horizontal_wrapped` wraps widgets, not columns with their own headers and scrolling | A small layout function: given each column's least and most width and which one stretches, give each a rect in a row; when the least widths don't fit, start a new row and let the shelf scroll. The grip is an `interact` with a drag sense that changes the shelf's height. | 300 |
| **Inspector cards that wrap** (spec 206, 1144) | Wrapping doesn't cope with framed groups of different heights | Work out how many columns of at least 180 px fit, then fill them row by row | 120 |
| **The File full-page view** (spec 533-540) | `Panel::left` + `CentralPanel`: `launcher.rs:517-526` already draws exactly this shape | When the File tab is in front, skip the ribbon, center and shelf, and draw the menu column and a scrolling page. The page stacks under the column when narrow. | 200 for the frame |
| **Toggle, segmented control, chip, pill, swatch, keycap, slider, dashed button** in the design's exact look | `Checkbox`, `Button`, `Slider`, `ComboBox` in egui's own look; restyling them through `Visuals` gets colours but not these shapes | Solis's own small widgets in `crates/editor/src/widgets/`, each an `allocate_response` plus painting, each giving a `WidgetInfo` so screen readers and the 467 label look-ups in the tests keep working (spec 117: every icon-only button has a text label) | 40 to 90 each, about 900 in all |
| **Fields with an axis letter; dropdown fields** | `DragValue`, `TextEdit`, `ComboBox` | Keep egui's editing inside; draw the frame (24 px, radius 5, accent border when focused) and the letter around it. Dropdown lists use `Popup::menu`, which the editor uses already, with the popup's frame taken from the tokens. Today's "counts once let go" fields (`settled.rs`) wrap the same way. This is also where the review's DEBT-03 (six copies of one drop-down in `inspector.rs`) ends. | 250 |
| **Inner outlines** (2 px accent inside a tile; the accent line inside a tab's top edge) | `Painter::rect_stroke` takes `StrokeKind::Inside` | Free | 0 |
| **Mixed accent fills** (d.4) | No colour mixing | `tokens::mix`, a few lines; always mixed against the known surface | 20 |
| **Dashed borders** (add buttons, blank templates, the ghost, the listening keycap) | `Shape::dashed_line` for a path of points, not for a rounded rectangle | A helper that walks a rounded rectangle's outline and dashes it | 50 |
| **Gradients** (sky, the model viewer's backdrop, fader caps, meters, the VFX gradient bar) | No gradient fill | A `Mesh` with a colour per corner for straight ones; a fan of triangles for round ones. Only the straight ones are needed in this phase. | 40 |
| **Shadows** (nodes, fader caps) | `epaint::Shadow` (offset, blur, spread, colour) | Free | 0 |
| **1 px lines at fractional scales** (Interface scale, spec 642) | Sizes are in points; at 125% a 1-point line is 1.25 pixels and blurs, and a line at a fractional position smears over two rows | Two rules in `tokens.rs`: a hairline's width is one whole pixel (`1.0 / pixels_per_point`, rounded up to whole pixels as the scale grows), and every separator's position goes through `Painter::round_to_pixel_center`. Offer scale steps that keep text sharp (proposed 100, 125, 150, 175, 200%; decision D18). The picture tests run at 100% and 150%. | 60, and care everywhere |
| **Interface scale and density** | `Context::set_zoom_factor` scales everything | Scale is the zoom factor. Density is a small table of heights (row, field, tab) read through `tokens::row_h()` and friends, so nothing writes 24.0 itself. | 80 |
| **Type weights** (400, 500, 600, 700) | A family per font file (`theme::SEMIBOLD` today). egui 0.36 can also set a variable font's weight per run of text (`TextFormat::coords`). | Static files and named families, as today: every widget takes a `FontId`. The variable route is one file instead of four but needs the weight set on every piece of text by hand; not worth it now. | 40 |
| **Letter-spacing and upper-case caps** | `TextFormat::extra_letter_spacing`; no upper-casing | A `tokens::caps(text)` helper building the text job | 20 |
| **Names cut off with "…"** | `Label::truncate` | Free | 0 |
| **Icons: 16 px line icons, about 120** | No icon set (only a symbol font as fallback) | See e.3 | 300 + data |
| **The command palette** | Nothing ready-made | `search.rs` already draws its own (an `Area` under the box); restyle | 100 |
| **Dialogs** | `Window`, `Modal` | Keep `Modal` (used by `play_changes.rs` and the launcher) with one shared frame from the tokens | 80 |
| **Tooltips, scroll bars, text selection** | Styled through `Visuals` and `Spacing` | Set once in `theme::visuals` from the tokens | 40 |
| **Label scrubbing, long press** (spec 174, 214) | `Sense::drag` on any rect; no long press | Both are small; both wait for decision D15 | 60 |
| **Ghost, footprint, guides over the viewport** (spec 253-256) | The viewport already paints over its image (`viewport_tools.rs`) | Project the model's bounds (`models.rs` `ModelInfo::min` and `max`) through `screen.rs` and paint dashed lines | 200 |
| **Graphs** (frame-time bars, timeline blocks, the Frames ruler, meters) | The painter; no plot widget is used today | As today (`profiler.rs:305`, `frames_view.rs`, `mixer_view.rs`), repainted with the tokens. Each must draw only what is in view (review findings BUG-21 and BUG-22). | within their milestones |

### e.3 Icons

**Today:** every icon is drawn in code in `crates/editor/src/icons.rs` (489 lines, no image files): 27 in `Icon`, 5 tool icons, 5 tile-painting icons, and the 8 canvas tools' icons in `canvas_tools.rs`. That is the right idea (sharp at any size, takes any colour) in the wrong form for 120 icons: each is a hand-written function.

**What the design needs:** the thirteen screens draw **60 different icons**, every one an inline drawing on a 16 × 16 grid made of plain path, circle and rectangle parts, written out in the mock-up's HTML. They are the project lead's own drawings, part of their design, so using them copies nothing from anyone (`CONTRIBUTING.md`, "Third-party material"). Several are used for two meanings (the three-block icon is both Scatter and "assembly"), which is fine. Solis's own editors need about 45 more that the mock-up doesn't have (the five tile tools, the eight canvas tools, Transform, hide, lock, pin, the kinds of file Solis has, Pause and Step, undo and redo, and so on), which today's `icons.rs` already draws: about 105 to 120 in all.

**How:** keep "drawn in code", change the form to data.

- `icons.rs` becomes a table: `Icon::Save => "M3 2.5h8l2.5 2.5v8.5h-10.5z M5.5 2.5v3.5h5V2.5"`, the path text copied from the mock-up, on the 16 grid.
- One small reader turns that text into lines (move, line, horizontal, vertical, curve, arc, close; plus circles and rounded rectangles written as paths), scaled to the size asked for, and paints them with the stroke the design gives (d.7). About 300 lines with its tests; the reader is fuzzed like every reader, though it only ever reads Solis's own table.
- The 45 icons the mock-up lacks are redrawn by hand in the same grid and stroke, from Solis's present drawings, and shown to the lead on one sheet (**NEEDS MOCK-UP**: the icon sheet). Nothing is taken from any icon library or other program.
- A test draws every icon at 12, 15 and 18 px and checks it stays inside its square and isn't empty; the icon sheet is one of the picture tests.

The other route, an SVG-drawing library turning files into small pictures, was looked at and not recommended: a new dependency (rule 5), blurry between scale steps unless re-made per scale, and a tint per icon instead of a stroke.

### e.4 Picture tests

**Today** no test compares pictures of the editor. The whole-window tests drive the editor through egui_kittest without a GPU and ask the accessibility tree for labels and rectangles. Real pixels are checked in two places only: `crates/render/tests/gpu` (frames from the renderer) and `tools/smoke/window-smoke.sh`, which opens the real program under a virtual screen, takes screenshots and samples a few colours at fixed positions for the standard layout (those positions change in M3 and M4; its fixed sleeps are review finding TEST-03).

**What egui_kittest can do:** with its `wgpu` feature (off today) a harness draws a real frame and hands back the picture (`Harness::render`). That needs a GPU or Mesa's software driver, which CI already has for the render tests. Its `snapshot` feature adds comparing with a kept PNG, at the price of three more test-only libraries (a picture comparer, a file opener, temporary files). **Recommended:** turn on `wgpu` only, and write the comparison in Solis: a per-pixel difference with a threshold, saving a difference picture when it fails, about 80 lines over the `image` library already in the tree. Two of the three checks below are Solis's own anyway, and rule 5 asks why a library is needed for what is small. Either way the change is noted in `docs/dependencies.md`.

**How close to the mock-ups is realistic?** Not pixel-close across a whole screen, for three reasons: the mock-ups were drawn by a web browser, whose text comes out different from egui's letter by letter (spacing, smoothing, a different line height by a fraction of a pixel); they are full of sample content Solis doesn't have (Harbor_Prototype, the crane); and their viewports are drawings. So the plan uses three kinds of check, each honest about what it proves:

1. **Measured layout, against the spec** (no GPU): the rectangles of the bands and controls, read from the accessibility tree, against the spec's numbers to within 1 px: tab bar 0 to 34, ribbon 34 to 100, large buttons 50 × 44, shelf bar 34, status bar 24, fields 24, toolbox buttons 34 × 34 and so on. This is the check that the screen *is* the design.
2. **Sampled colours, against the mock-up pictures** (GPU): for each screen a short list of named points on flat surfaces away from text (the ribbon's body, a shelf column, a card, a selected row, the active tab's underline), each read from the mock-up PNG (which is twice the size, so the point is doubled) and required to match the editor's frame exactly, or within 1 step of 255 for mixed colours. About 15 to 25 points a screen.
3. **Solis's own approved pictures** (GPU): once a screen has been compared by eye beside its mock-up and approved, its frame is kept as the reference and later changes are compared with it at a small threshold. This catches accidents; it doesn't prove likeness.

For the lead's eye, each milestone's pull request carries a **side-by-side sheet**: the mock-up on the left, the editor's frame on the right, made by a test from a small sample project named like the mock-up's (`tests/fixtures/ui-sample/`), so the two can be compared at a glance.
