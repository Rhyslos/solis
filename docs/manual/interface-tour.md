# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

This page follows what you see, in the order you meet it: Recent projects, the File page Solis starts on, then the editor from its tab bar down to its status bar. All of it is drawn in one look: dark neutral greys, lighter the nearer a surface is to you, with one [accent colour](concepts.md#the-window) for what's chosen. The accent, and how large the editor is drawn, are yours to choose in **File › Preferences** ([Settings](settings.md)).

**[Recent projects](launcher.md)** is what Solis always starts on, in its full window: the **File** tab's first page. Your projects are cards (pinned ones first, with **Search projects**, **Sort by**, **Open… ▾** and **New project**); double-click one and the editor opens with it. Under them, **Start from a template**: an empty project or one of the four examples. Why start there? Solis doesn't open the last project by itself, so you always choose what to work on. Until a project is open the other tabs are dimmed; **Esc**, or **Carry on without a project**, opens the editor with none. See [Projects](launcher.md).

**The tab bar** is the dark strip along the very top. From left to right:
- **The logo**, in a small light tile.
- **The [tabs](concepts.md#the-window):** **File**, **Home**, **Library**, **Shading**, **Animate**, **UI**, **VFX**, **Audio**, **Debug** and **Build**. Each is one kind of work, and a click on one changes the [ribbon](concepts.md#the-window) under it and what the editor shows (see **The tabs** below). The tab in front is lit, with a line of the accent colour under it. **Shading** and **VFX** are grey with a small hollow square after their names: those screens aren't built yet ([Unfinished parts](unfinished-parts.md)).
- **A tab that comes with a file** (a [contextual tab](concepts.md#the-window)): while a tileset, a picture's crops or a selected tile map is open, a tab such as **LIBRARY TOOLS · Tileset** stands right after the tab it belongs to, with a ribbon of that file's own tools. Click it to bring that file to the front; it goes when the file closes. A tileset and a picture's crops fill their tab (with **‹ Back to Library** at the right of the document's tab); a tile map's tab is Home with the **Map** shelf tab in front.
- **Search or run a command**, in the middle: one search for everything (see below). Its right end shows its key, **Space**.
- **The project and the area**, as in `Harbour / docks`, with an amber dot after the area's name while it has unsaved changes. Once an update has downloaded, **Restart to update** appears just left of it ([Installing and updating](installing-and-updating.md)).
- **The chevron** at the far right collapses the ribbon (see below).

**The ribbon** is the band under the tab bar: the things you do on the tab in front, as buttons you can see instead of menus you have to open. Why a ribbon? Everything for the job at hand is on screen at once, each with its name under it, and nothing is hidden two menus deep. It is made of **groups**, each with its name underneath:
- **A large button** is an icon over a word. Point at one and it says what it does, its shortcut if it has one, and, when it is greyed, why it can't be used right now.
- **A small row** is a line of words: a switch has a box that is filled while it is on (**Grid · 1** in Snapping), and a row with a small arrow opens a list (**More**).
- **A button that makes one of several kinds** has a small arrow at the end of its name: click the button for the usual kind (**Light** makes a point light), or click the arrow, or right-click the button, for the list (a spotlight).
- **Grey controls with a small hollow square** aren't built yet: they show where things will be ([Unfinished parts](unfinished-parts.md)), and one setting hides them all.

**The Play group** is at the ribbon's right end on every tab: **Play** is the one light button on screen. While the game runs it becomes **Stop**, with **Pause** and **Step** beside it; nothing else in the editor changes colour. See [Play mode](play-mode.md).

**When the window is narrow** the ribbon doesn't wrap: it scrolls sideways. Turn the mouse wheel while pointing at it.

**To give the ribbon's room to the editor,** click the chevron at the tab bar's right end, or double-click a tab: only the tabs stay. Click a tab then and the ribbon drops down over the editor until you choose something on it or move the pointer below it. The chevron, or another double-click, brings the ribbon back for good.

**The tabs, one per job:**
- **Home** is for building [areas](saving-areas.md): the area big in the middle with the toolbox down its left edge, and its entities (the Area list), their Properties and your files (Assets) round it, where the [panel layout](concepts.md#the-window) puts them (see **Home's panel layouts** below). Its ribbon: **Edit** (**Undo** and **Redo**, each named for the step it acts on when you point at it; see [Undo and redo](undo.md)), **Create** (**Empty**, **Model**, **Sprite**, **Light**, **Camera**, **Tile map**, and **More** for a reverb zone or an [assembly](assemblies.md) from the selection), **Snapping** (**Grid**, **Angle** and **Scale**; click the value after "·" for the steps, see [Selecting and moving](selecting-and-moving.md)), and **Project** (**Save** saves the area; **Build** opens [Export for playtesting](playtest-export.md)). With the **Place** shelf tab in front the ribbon takes its build form for [placing](projects-and-the-library.md).
- **Library** is the project's files ([Projects and the Library](projects-and-the-library.md)). Its ribbon: **Add** (Import, Create, Folder), **View** (List, Show folders), **Filter** (Type) and **Use** (Place).
- **Animate** is for animating sprites: a clips file's frames and clips, or an animator's graph and preview, each with a ribbon of its own ([Animating sprites](sprite-animation.md), [Animators](animators.md)). With neither open it shows the Sequencer's page, not built yet.
- **UI** draws game screens on the **Canvas**, with the drawing tools down its left edge; double-click a screen's file in the Library and Solis switches to UI with it open ([The UI editor](ui-editor.md)).
- **Audio** is the [mixer](mixer.md): a strip per bus with **Master** last, the bus you pick beside it with its ducking, and the project's sound events on the shelf. Its ribbon: **Bus** (a new bus), **Add rule**, **Remove**, **Clear solo**, **Mute the game** and **Heard at once**.
- **Debug** is for testing the game: the [Profiler](profiler.md) fills the middle (the frames as bars, the frame you click as a timeline, and Statistics), with the **Game** as a tab beside it, and the [Console](console.md) in front on the shelf. Its ribbon has **CPU** and **GPU** for the Profiler and the **Session** group: **Load save** ([Save games](save-games.md)), **Play and record inputs** and **Replay a recording…** ([Input](input.md)).
- **Build** is [Export for playtesting](playtest-export.md) as a page: the game's name, the area it starts in, the folder and the window, with **Export**, and the Console on the shelf. Its ribbon has **Export** and **Open folder** (the last export's folder); the full export beside them isn't built yet.
- **Shading** and **VFX**, and the **model viewer** (a model's right-click in the Library, **Open in the model viewer**), are screens that aren't built yet: their ribbon and shelf tabs are shown greyed, round one page saying what will be there and when ([Unfinished parts](unfinished-parts.md)).

Each tab remembers its own shelf tab, the shelf's height, the split between its groups and every border you dragged, separately.

**File** is different: it is a page in place of the ribbon and the editor, with a column of items down its left side. **Back to editor** (or **Esc**) returns to the tab you came from.

| Item | What it does |
|---|---|
| Recent projects | Every project Solis knows, as cards, and templates to start from: where Solis starts ([Projects](launcher.md)). |
| New project, Open… | Make a project in a folder, or open one ([Projects and the Library](projects-and-the-library.md)). |
| Save all | Saves the open area. Every other file saves as you change it. |
| Project settings | This project's settings: General, 2D, Input (the game's actions and their keys, see [Input](input.md)) and Build ([Settings](settings.md)). |
| Preferences | The editor's own settings, such as its accent colour ([Settings](settings.md)). |
| Plugins | Not built yet. |
| Learn | Four examples to start from, the first steps and a link to this manual. |
| Updates | The build you have, the newest one, and **Check now** ([Installing and updating](installing-and-updating.md)). |
| About | The program's name and version. |
| Exit | Closes Solis, asking about unsaved work first. |

**One search for everything:** press **Space** (with nothing being typed) or click **Search or run a command** in the tab bar. Type part of a name and the list finds:
- everything the editor can **do** (Save area, Add point light, Play…), each with where it lives and its shortcut beside it when it has one, so you pick up the keys as you go. Going to a tab, collapsing the ribbon, folding the shelf and hiding unfinished parts are there too;
- every **file** in the project, which opens it as a double-click in the [Library](projects-and-the-library.md) would. A file you place rather than open (a model, an assembly, a sprite sheet, a tile map or a picture) is picked in the Library instead, with its folder open, ready to drag in;
- every **entity** in the area, which selects it and points the camera at it;
- a **panel** by its name, which comes to the front.

Up and Down choose, **Enter** does it, **Esc** closes. Why? You don't have to remember which tab something is on: type what you want. Things without a button on the ribbon are found here too, wherever else they are: **New area** and **Open area…** (the **+** after the area's tab), **Save area as…** (a right-click on the area's tab), **Duplicate**, **Delete**, **Rename**, **Select all** and **Select nothing** (a right-click in the Area list), **Reset layout** (**File › Preferences**), **Open the manual ↗** and **Add the sample area** (**File › Learn**), **Check for updates** (**File › Updates**), and showing or hiding the viewport's corner controls (its **Show ▾** pill). Things that can't be done right now (Pause while editing, Delete with nothing selected) aren't listed. In the Frames tab, Space still plays and stops the clip, so point somewhere else first.

**The middle of the window** is the same on every tab: one or two [editor groups](concepts.md#the-window) on top, the [shelf](concepts.md#the-window) under them, the status bar along the bottom; on Home, the Area list, Properties and Assets sit where the panel layout puts them. Why the same everywhere? Each tab of the tab bar is one kind of work, and its screen keeps your things in the same places: what you're working on big in the middle, the tools and lists for it round it. Panels don't move between places; every tab remembers how you sized it.

**Home's panel layouts.** Home has three lists you use all the time: the **Area** tree, **Properties** and **Assets**. Where they sit is a setting, **File › Preferences › Appearance › Panel layout**, and it changes at once, so try each:

| Panel layout | Where the lists are |
|---|---|
| **Area left, Assets bottom** (the standard) | The Area list a tall column at the left of the area, Properties a tall column at the right, Assets a wide strip under the area. |
| **Area and Assets on the left** | The Area list over Assets in one column at the left; Properties at the right. |
| **Area left, Assets right** | The Area list at the left; Properties over Assets at the right. |
| **The shelf (as designed)** | All three side by side on the shelf's **Entity** tab, under the area, as the design first drew them. |

Why choose? Tall columns show many more entities and files at once, and Properties gets room for its cards; the shelf layout gives the area the whole width. In the first three the shelf keeps its other tabs (**Place**, **Console**, **Problems**, **Map** while a tile map is selected) and still folds with **Ctrl + Space**, which gives the columns the whole height. **Placing** is the same in every layout: click **Place** on the shelf's bar. In the side layouts its tab holds how placed things land (**Placement**), and the Assets list, wherever it is, shows what can be placed, with its chips and search. When the window is too narrow for the columns, the area gets narrower first (down to 240 points), then the right column, then the left one (down to 180 each); narrower still, the lists go back on the shelf until the window is wide enough again.

- **Editor groups** hold *documents*: the area, the running game, an animator, a tileset, the mixer. Each group has a strip of tabs along its top (the one in front is lighter, with a thin accent line on top); click one to bring it to the front. A tab for a file has a **×** that closes the file; nothing asks whether to save, because those files save as you change them. When a tab shows two groups side by side, drag the thin line between them to give one more room; when the window is too narrow for both, they sit one above the other.
- **Drag a document's tab** onto the left or right half of the middle and it gets a group of its own there, the rest beside it; drag it onto another group's tabs and the two groups join. **Esc** while dragging puts it back. Each tab keeps what you did while Solis runs.
- **The area's tab** carries a **+**: **New area** and **Open area…**. Right-click the tab for **Save**, **Save as…**, **Close others** and **Show in the Library**. (Close is there but greyed: one area is open at a time, so open or make another in its place.)
- **The shelf** has its own tabs along its bar. Click one to show it; its contents sit side by side as columns, each with a header. Drag the grip at the bar's left (or the bar's top edge) up or down to make the shelf taller or shorter, and press **Ctrl + Space** to fold it down to its bar and back, even while you're typing. At the bar's right are tools for the tab in front: the Console's level filters, search, **Clear** and **Copy** while the Console is there.
- **Every border drags.** Point at the thin line between two parts (two columns, a column and the area, the Assets strip and the area, two lists stacked in a column, two shelf columns, two groups) and the pointer becomes a two-headed arrow: drag to give one side more room. **Double-click** a border to put it back to its standard size. Each part has a least size, so nothing can be dragged away. When the window narrows, the shelf's columns give way too: first the one that takes what's left, then the others from the right; narrower still, they sit under each other.
- **What each tab remembers:** the shelf tab in front, whether the shelf is folded, and every border you dragged (the shelf's height, the split between its groups, the columns), separately for each panel layout, also after closing Solis. **Reset layout** puts the tab you're on back as it started, in every layout.

What each tab shows:

| Tab | Editor groups | Shelf tabs |
|---|---|---|
| Home | The area (its viewport, with the toolbox down its left edge); a **Game** tab beside it, brought to the front, while the game runs. The Area list, Properties and Assets round it, as the panel layout puts them | **Place** (placing: Placement; in the shelf layout also the Area tree and Assets) · **Entity**, in the shelf layout only: the Area tree, Properties (its components as cards, Hints the last) and Assets, side by side · Animation (not built yet) · **Console** · **Problems** (with how many) · **Map**, while an entity with a tile map is selected |
| Library | The Library | **Details**: Properties, showing the file picked · **Code**: the game's code files and what uses each · Import queue and History (not built yet) |
| Library tools · Tileset | The tileset open (its picture, the chosen terrain's slots, Try it) | **Terrains**: the terrains, the picture and the slot picked · **Console** · **Problems** |
| Library tools · Picture | The picture with its crops | **Crops**: the grid and every crop, and the crop picked · **Console** · **Problems** |
| Animate | The viewport while a clips file is open; with an animator, the **Animator** beside it and its **Preview** as a group of its own; with neither, the Sequencer's page (not built yet) | Sequencer (not built yet) · **Frames** (the clips and the chosen clip's frames) and **Clip** (its playback) while a clips file is open, each with a × that closes it · **Parameters** and **Properties** (the state picked) while an animator is · Key (not built yet) · **Console** |
| UI | The **Canvas**, with its toolbox down its left edge (a × closes the screen open on it) | **Screen**: Elements (the screen's pieces), Properties (the piece picked) and Pictures (for the Image tool) · **Console** · **Problems** |
| Audio | The mixer (`mixer.solmix`), and beside it the **Bus** picked (`Music › Bus`), with a switch to its Effects (not built yet) | **Sound events**: every event, and the one picked · Snapshot (not built yet) · **Console** |
| Debug | The **Profiler**, with the **Game** as a tab beside it | **Console** · **Problems** · Hottest functions and Memory snapshot (not built yet) |
| Build | Export for playtesting, as a page | **Console** · **Problems** · Build log (not built yet) |

Where each panel is:

| Panel | Where | What it's for |
|---|---|---|
| Viewport | The area's tab on Home and Animate | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Beside the area while playing; beside the Profiler on Debug | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | Home: the left column; in the shelf layout, Home › Entity and Place, left | Every [entity](entities.md) in the area, as a tree, with its count in the header. The header's magnifier searches by name, the next button folds everything, and **⋯** unfolds everything, shows what's hidden and unlocks what's locked. Point at a row for its eye (hide) and lock; right-click it for Rename, Duplicate, Delete, Add…, Create assembly and Select all or nothing. |
| Properties | Home: the right column; in the shelf layout, Home › Entity, middle; and the Details, Properties and Screen shelf tabs | Along its top the selected entity: its name (click and type to rename it), what it is and what it's under, **+ Component**, and **⋯** for rarer actions. Below, each [component](entities.md) is a card with its settings, side by side as many as fit; a card's **?** opens its manual page and its **⋯** (or a right-click on the card) copies, pastes, resets or removes it. The last card is **Hints**: a few plain sentences about what's selected and what you can do with it. **Turn hints off** on it once you know your way around; **File › Preferences** turns them back on. |
| Assets | Home: the strip under the area, or under the Area list, or under Properties, as the panel layout says; in the shelf layout, Home › Entity, right, and Home › Place | The Library's open folder as tiles (as many across as fit), with **Import** last; while **Place** is in front, only what can be placed, with chips and a search. Drag a tile into the viewport to place it, double-click it to place it where the camera looks, right-click it for the Library's menu. |
| Placement | Home › Place | How placed things land: on the grid, or where you point. |
| Library | The Library tab | The open [project](projects-and-the-library.md)'s files as tiles or a list, with chips for each kind and a search. |
| Code | Library › Code | The project's game code files in Lust, Rust or Lua. See [Game code](game-code.md). |
| Console | A shelf tab on every tab with a shelf | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | A shelf tab on Home, UI, Debug, Build and the Library tools | Mistakes found in the game's code and in the area, such as a missing model. See [Problems](problems.md). |
| Mixer and Bus | Audio | The project's buses, a strip each, and the bus picked with its ducking. See [The Mixer](mixer.md). |
| Sound events | Audio › Sound events | Every sound event in the project, and the one picked to change and hear. See [Sound events](sound-events.md). |
| Profiler | Debug | Where each frame's time goes. See [The Profiler](profiler.md). |
| Export for playtesting | Build | A folder with the game in it, to send. See [Export for playtesting](playtest-export.md). |
| Clips, Frames and Clip, Animator, Parameters and Preview, Map, Tileset and Terrains, Atlas Crop and Crops, Canvas, Elements and Pictures | While their file is open (the Map while a tile map is selected) | See [Animating sprites](sprite-animation.md), [Animators](animators.md), [Painting tiles](painting-tiles.md), [Tilesets and autotiling](tilesets.md), [Atlas Crop](atlas-crop.md) and [The UI editor](ui-editor.md). Opening the file brings its tab to the front. |

**Questions** that stop your work (unsaved changes, a file changed on disk, keeping what you changed while playing) are a card over the dimmed window: the answer that sets your work aside at the left, the main one in the accent colour at the right.

**The status bar** along the bottom says what's happening, so you don't have to go looking:
- On the left, a green dot and **Ready** while the game's code is clean (Solis checks it when the project opens and each time you save a code file; hover to see when it last did), or, in red, how many mistakes it has.
- Beside it, the branch and its changes (not built yet), then the problems as two coloured counts: red for mistakes in the game's code, amber for problems in the area. Click them to open the [Problems](problems.md) tab.
- On the right: **Playing** or **Paused** while the game runs; **Saved**, **Unsaved changes** or **Not saved yet** for the open area ([Saving areas](saving-areas.md)); the area's name, whether the viewport shows it in 2D or 3D, and for 2D the game's size in pixels (`farm · 2D · 480 × 270`, from **File › Project settings**); what moves snap to and which way the gizmo points (**Snap 0.25 · World**, or **Snap off**); then Solis's version, and **Restart to update** once an update has downloaded. For three seconds after you click a part that isn't built yet, the words before the version say so instead ([Unfinished parts](unfinished-parts.md)).

**From an older build:** panels used to be dragged anywhere and the Library could be a drawer. Neither is so now, and an arrangement you made before isn't carried over: every tab starts as the design draws it, and the Console says so once.

## Settings
| Setting | What it does |
|---|---|
| Panel layout (**File › Preferences › Appearance**) | Where Home's Area list, Properties and Assets sit: **Area left, Assets bottom** (the standard), **Area and Assets on the left**, **Area left, Assets right** or **The shelf (as designed)** ([Settings](settings.md)). |
| Reset layout (in the search, and in **File › Preferences**) | Puts the tab you're on back as it started: its shelf tab, unfolded, every border's size in every panel layout, and how its groups were arranged ([Settings](settings.md)). |
| File › Preferences | The editor's own settings, such as its accent colour: see [Settings](settings.md). |
| The viewport's **Show ▾** pill | Switches the frame time, the axis locks and camera speed, and the toolbox on and off (the search does too). |

## Shortcuts
| Action | Shortcut |
|---|---|
| Search for anything to do, any file, entity or panel (with nothing being typed) | `Space` |
| Choose a result in the search list | `Up`, `Down` |
| Do what's chosen | `Enter`, or click a result |
| Close the search | `Esc`, or click outside it |
| Leave the File page for the tab you came from | `Esc` |
| Collapse the ribbon, or bring it back | Double-click a tab, or the chevron |
| Scroll a ribbon that doesn't fit | The mouse wheel, over the ribbon |
| List the other kinds a Create button makes | Its small arrow, or a right-click |
| Fold the shelf to its bar, or open it again (while typing too) | `Ctrl + Space` |
| Make the shelf taller or shorter | Drag its grip, or its bar's top edge |
| Give one of two groups more room | Drag the line between them |
| Make a column, the Assets strip or a stacked list bigger or smaller | Drag the line at its edge |
| Put a border back to its standard size | Double-click it |
| A document in a group of its own, or two groups joined | Drag its tab to a half of the middle, or onto the other group's tabs |
| Put a dragged tab back | `Esc`, while dragging |
| New area, Open area… | The **+** after the area's tab |
| Save, Save as…, Close others, Show in the Library | Right-click the area's tab |

See [Keyboard shortcuts](keyboard-shortcuts.md) for the rest.

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it lives on another tab, on a shelf tab not in front, or the shelf is folded. **Fix:** press **Space** and type its name: choosing it brings its tab and shelf tab to the front. **Ctrl + Space** unfolds the shelf.
- **Problem:** a menu from an older build is gone (File, Edit, Entity, Help). **Why:** the tabs and the ribbon took the menu bar's place. **Fix:** press **Space** and type what you want: the list says where it lives now.
- **Problem:** the panels are where an older build didn't have them. **Why:** the middle of the window is the design's now, and panels don't move. **Fix:** see the table above for where each one lives; **Space** and its name finds it.
- **Problem:** Properties is cramped. **Why:** its column is narrow, or (in the shelf layout) it shares Home's shelf with the Area tree and Assets. **Fix:** drag the border at its edge (or the shelf's grip) for more room, it's remembered; or choose another **Panel layout** in **File › Preferences**.
- **Problem:** Home's lists jumped onto the shelf. **Why:** the window is too narrow for the columns beside the area. **Fix:** widen the window: they go back by themselves.
- **Problem:** a border won't go further. **Why:** each part keeps a least size, and the area keeps room. **Fix:** make the part beside it smaller first, or double-click the border to start again.
- **Problem:** two of a tab's documents are side by side, or one group holds them all. **Why:** a tab was dragged. **Fix:** drag it back, or **Reset layout**.

## Related
- [Projects](launcher.md): Recent projects, the page Solis starts on, and Learn, Updates and About.
- [Projects and the Library](projects-and-the-library.md): the Library.
- [Play mode](play-mode.md): Play, Pause, Step and Stop.
- [Unfinished parts](unfinished-parts.md): what is greyed with a hollow square, and the setting that hides it all.
- [Concepts](concepts.md): panel, tab, ribbon, editor group, shelf, contextual tab, dialog card, layout, workspace, accent colour.
