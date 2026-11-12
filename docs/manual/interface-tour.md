# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

This page follows what you see, in the order you meet it: the launcher Solis starts on, then the editor from its tab bar down to its status bar. All of it is drawn in one look: dark neutral greys, lighter the nearer a surface is to you, with one [accent colour](concepts.md#the-window) for what's chosen. The accent, and how large the editor is drawn, are yours to choose in [Settings](settings.md).

**The [launcher](launcher.md)** is what Solis always starts on. **Projects** lists your projects (pinned ones first, with **Search projects** and **Sort by**), beside **+ New project**, **Open project…** and **Scan a folder…**; **Learn** has four examples to start from, the first steps and a link to this manual; **Updates** shows the build you have and the newest one. Double-click a project and the editor opens with it. Why start there? Solis doesn't open the last project by itself, so you always choose what to work on. Its pages are also on the **File** tab (see below), and **Esc** leaves the launcher for the editor. See [The launcher](launcher.md).

**The tab bar** is the dark strip along the very top. From left to right:
- **The logo**, in a small light tile.
- **The [tabs](concepts.md#the-window):** **File**, **Home**, **Library**, **Shading**, **Animate**, **UI**, **VFX**, **Audio**, **Debug** and **Build**. Each is one kind of work, and a click on one changes the [ribbon](concepts.md#the-window) under it and what the editor shows (see **The tabs** below). The tab in front is lit, with a line of the accent colour under it. **Shading** and **VFX** are grey with a small hollow square after their names: those screens aren't built yet ([Unfinished parts](unfinished-parts.md)).
- **A tab that comes with a file:** while a tileset, a picture's crops or a selected tile map is open, a tab such as **LIBRARY TOOLS · Tileset** stands right after the tab it belongs to. Click it to bring that file to the front; it goes when the file closes.
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
- **Home** is for building [areas](saving-areas.md), with the panels where **The panels** below says they start. Its ribbon: **Edit** (**Undo** and **Redo**, each named for the step it acts on when you point at it; see [Undo and redo](undo.md)), **Create** (**Empty**, **Model**, **Sprite**, **Light**, **Camera**, **Tile map**, and **More** for a reverb zone or an [assembly](assemblies.md) from the selection), the three snapping switches (**Grid**, **Angle** and **Scale**; their steps are chosen in the viewport's corner, see [Selecting and moving](selecting-and-moving.md)), and **Project** (**Save** saves the area; **Build** opens [Export for playtesting](playtest-export.md)).
- **Library** brings the project's files to the front ([Projects and the Library](projects-and-the-library.md)). Its ribbon has **Import** ([Importing models](importing-models.md)); making files, the view and the kind filter are still in the Library panel's own bar.
- **Animate** gives animating sprites room: the Library on the left to pick sheets and animators from, the viewport (an animator opens beside it), and a taller bottom for a clips file's Frames.
- **UI** draws game screens: the screen open on the **Canvas** in the middle, its pieces in **Elements** on the left, and the piece picked in **Properties** on the right. Double-click a screen's file in the Library and Solis switches to UI with it open. See [The UI editor](ui-editor.md).
- **Audio** brings the [Mixer](mixer.md) to the front.
- **Debug** is for testing the game: the game big in the middle, your code files on the left (each says which of its components entities in the area use, as in `Player · 1`), Properties on the right (game code's values, live while playing), and the Console, Problems and the [Profiler](profiler.md) along the bottom. Its ribbon has two switches for the Profiler (**CPU** and **GPU**) and the **Session** group: **Load save** ([Save games](save-games.md)), **Play and record inputs** and **Replay a recording…** ([Input](input.md)).
- **Build** has **Export**, which opens [Export for playtesting](playtest-export.md).
- **Shading** and **VFX** show one page saying what will be there and when.

Each of Home, Animate, UI and Debug remembers how you arranged its panels, separately. The panels that come with an open file or a selected tile map (see the second table below) come with you when you switch, except the Canvas and Elements, which stay in UI.

**File** is different: it is a page in place of the ribbon and the editor, with a column of items down its left side. **Back to editor** (or **Esc**) returns to the tab you came from.

| Item | What it does |
|---|---|
| Recent projects | Every project Solis knows, to open one ([The launcher](launcher.md) has the same page). |
| New project, Open… | Make a project in a folder, or open one ([Projects and the Library](projects-and-the-library.md)). |
| Save all | Saves the open area. Every other file saves as you change it. |
| Project settings | This project's settings: its 2D sizes ([Settings](settings.md)). |
| Input | The game's actions and the keys for each ([Input](input.md)). |
| Preferences | The editor's own settings, such as its accent colour ([Settings](settings.md)). |
| Plugins | Not built yet. |
| Learn | Four examples to start from, the first steps and a link to this manual. |
| Updates | The build you have, the newest one, and **Check now** ([Installing and updating](installing-and-updating.md)). |
| About | The program's name and version. |
| Exit | Closes Solis, asking about unsaved work first. |

**One search for everything:** press **Space** (with nothing being typed) or click **Search or run a command** in the tab bar. Type part of a name and the list finds:
- everything the editor can **do** (Save area, Add point light, Play…), each with where it lives and its shortcut beside it when it has one, so you pick up the keys as you go. Going to a tab, collapsing the ribbon, pinning the Library and hiding unfinished parts are there too;
- every **file** in the project, which opens it as a double-click in the [Library](projects-and-the-library.md) would. A file you place rather than open (a model, an assembly, a sprite sheet, a tile map or a picture) is picked in the Library instead, with its folder open, ready to drag in;
- every **entity** in the area, which selects it and points the camera at it;
- a **panel** by its name, which comes to the front.

Up and Down choose, **Enter** does it, **Esc** closes. Why? You don't have to remember which tab something is on: type what you want. Some things have no button yet and are done from the search alone (or by their keys): **New area**, **Open area…**, **Save area as…**, **Duplicate**, **Delete**, **Rename**, **Select all**, **Select nothing**, **Reset layout**, **Open the manual ↗**, **Add the sample area**, **Check for updates**, and showing or hiding the viewport's corner controls. Things that can't be done right now (Pause while editing, Delete with nothing selected) aren't listed. In the Frames tab, Space still plays and stops the clip, so point somewhere else first.

**The middle of the window** is the same on every tab: one or two [editor groups](concepts.md#the-window) on top, the [shelf](concepts.md#the-window) under them, the status bar along the bottom. Why the same everywhere? Each tab of the tab bar is one kind of work, and its screen keeps your things in the same places: what you're working on big in the middle, the tools and lists for it on the shelf below. Panels don't move between places; every tab remembers how you sized it.

- **Editor groups** hold *documents*: the area, the running game, an animator, a tileset. Each group has a strip of tabs along its top (the one in front is lighter, with a thin accent line on top); click one to bring it to the front. A tab for a file has a **×** that closes the file; nothing asks whether to save, because those files save as you change them. When a tab shows two groups side by side, drag the thin line between them to give one more room; when the window is too narrow for both, they sit one above the other.
- **The area's tab** carries a **+**: **New area** and **Open area…**. Right-click the tab for **Save**, **Save as…**, **Close others** and **Show in the Library**. (Close is there but greyed: one area is open at a time, so open or make another in its place.)
- **The shelf** has its own tabs along its bar. Click one to show it; its contents sit side by side as columns, each with a header, and fall under each other when the window is narrow. Drag the grip at the bar's left (or the bar's top edge) up or down to make the shelf taller or shorter, and press **Ctrl + Space** to fold it down to its bar and back, even while you're typing. At the bar's right are tools for the tab in front: the Console's level filters, search, **Clear** and **Copy** while the Console is there.
- **What each tab remembers:** the shelf tab in front, the shelf's height and whether it's folded, and the split between its groups, as you left them, also after closing Solis. **Reset layout** puts the tab you're on back as it started.

What each tab shows:

| Tab | Editor groups | Shelf tabs |
|---|---|---|
| Home | The area (its viewport, with the toolbox down its left edge); a **Game** tab beside it, brought to the front, while the game runs | **Entity**: the Area tree, Properties (its components as cards, Hints the last) and Assets, side by side · Animation (not built yet) · **Console** · **Problems** (with how many) · **Map**, while an entity with a tile map is selected |
| Library | The Library; a **Tileset** or a picture's crops (**Atlas Crop**) beside it while one is open | **Details**: Properties, showing the file picked · **Code**: the game's code files and what uses each · Import queue and History (not built yet) |
| Animate | The viewport; an **Animator** beside it while one is open | Sequencer (not built yet) · **Frames** and **Clips** while a clips file is open (each with a × that closes it) · **Properties**: the animator's state picked, or the entity selected · Key (not built yet) · **Console** |
| UI | The **Canvas** (a × closes the screen open on it) | **Screen**: Elements (the screen's pieces) and Properties (the piece picked) · **Console** |
| Audio | The **Mixer** | **Bus**: Properties, showing the bus picked · Snapshot (not built yet) · **Console** |
| Debug | The **Game** (and the viewport behind it) beside the **Profiler** | **Console** · **Problems** · Hottest functions and Memory snapshot (not built yet) |
| Build | As Home | As Home |

The panels inside follow the design on Home; the other tabs' panels are today's, each until its screen is rebuilt to the design:

| Panel | Where | What it's for |
|---|---|---|
| Viewport | The area's tab on Home and Animate | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Beside the area while playing; Debug | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | Home › Entity, left | Every [entity](entities.md) in the area, as a tree, with its count in the header. The header's magnifier searches by name, the next button folds everything, and **⋯** unfolds everything, shows what's hidden and unlocks what's locked. Point at a row for its eye (hide) and lock; right-click it for Rename, Duplicate, Delete, Add…, Create assembly and Select all or nothing. |
| Properties | Home › Entity, middle; and the Details, Properties, Screen and Bus shelf tabs | Along its top the selected entity: its name (click and type to rename it), what it is and what it's under, **+ Component**, and **⋯** for rarer actions. Below, each [component](entities.md) is a card with its settings, side by side as many as fit; a card's **?** opens its manual page and its **⋯** (or a right-click on the card) copies, pastes, resets or removes it. The last card is **Hints**: a few plain sentences about what's selected and what you can do with it. **Turn hints off** on it once you know your way around; **File › Preferences** turns them back on. |
| Assets | Home › Entity, right | The Library's open folder as tiles, with **Import** last. Drag a tile into the viewport to place it, double-click it to place it where the camera looks, right-click it for the Library's menu. |
| Library | The Library tab | The open [project](projects-and-the-library.md)'s files: models, assemblies, pictures and sounds. Double-click a model or assembly, or click its +, to place it. Right-click anywhere in it, or use **+ New**, to make a code file, folder, area, tile map, tileset, animator or sound event. |
| Code | Library › Code | The project's game code files in Lust, Rust or Lua. See [Game code](game-code.md). |
| Console | A shelf tab on Home, Animate, UI, Audio and Debug | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | A shelf tab on Home and Debug | Mistakes found in the game's code and in the area, such as a missing model. See [Problems](problems.md). |
| Mixer | Audio | The project's buses: a fader, meter, mute and solo each, and ducking. See [The Mixer](mixer.md). |
| Profiler | Debug | Where each frame's time goes. See [The Profiler](profiler.md). |
| Clips and Frames, Animator, Map, Tileset, Atlas Crop, Canvas and Elements | While their file is open (the Map while a tile map is selected) | See [Animating sprites](sprite-animation.md), [Animators](animators.md), [Painting tiles](painting-tiles.md), [Tilesets and autotiling](tilesets.md), [Atlas Crop](atlas-crop.md) and [The UI editor](ui-editor.md). Opening the file brings its tab to the front. |

**The status bar** along the bottom says what's happening, so you don't have to go looking:
- On the left, a green dot and **Ready** while the game's code is clean (Solis checks it when the project opens and each time you save a code file; hover to see when it last did), or, in red, how many mistakes it has.
- Beside it, the branch and its changes (not built yet), then the problems as two coloured counts: red for mistakes in the game's code, amber for problems in the area. Click them to open the [Problems](problems.md) tab.
- On the right: **Playing** or **Paused** while the game runs; **Saved**, **Unsaved changes** or **Not saved yet** for the open area ([Saving areas](saving-areas.md)); the area's name, whether the viewport shows it in 2D or 3D, and for 2D the game's size in pixels (`farm · 2D · 480 × 270`, from **File > Project settings**); what moves snap to and which way the gizmo points (**Snap 0.25 · World**, or **Snap off**); then Solis's version, and **Restart to update** once an update has downloaded. For three seconds after you click a part that isn't built yet, the words before the version say so instead ([Unfinished parts](unfinished-parts.md)).

**From an older build:** panels used to be dragged anywhere and the Library could be a drawer. Neither is so now, and an arrangement you made before isn't carried over: every tab starts as the design draws it, and the Console says so once.

## Settings
| Setting | What it does |
|---|---|
| Reset layout (in the search, and in **File > Preferences**) | Puts the tab you're on back as it started: its shelf tab, the shelf's height, unfolded, and the split between its groups ([Settings](settings.md)). |
| File > Preferences | The editor's own settings, such as its accent colour: see [Settings](settings.md). |
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
| New area, Open area… | The **+** after the area's tab |
| Save, Save as…, Close others, Show in the Library | Right-click the area's tab |

See [Keyboard shortcuts](keyboard-shortcuts.md) for the rest.

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it lives on another tab, on a shelf tab not in front, or the shelf is folded. **Fix:** press **Space** and type its name: choosing it brings its tab and shelf tab to the front. **Ctrl + Space** unfolds the shelf.
- **Problem:** a menu from an older build is gone (File, Edit, Entity, Help). **Why:** the tabs and the ribbon took the menu bar's place. **Fix:** press **Space** and type what you want: the list says where it lives now.
- **Problem:** the panels are where an older build didn't have them. **Why:** the middle of the window is the design's now, and panels don't move. **Fix:** see the table above for where each one lives; **Space** and its name finds it.
- **Problem:** Properties is cramped beside the Area tree and the Library. **Why:** they share Home's shelf, until the screen is rebuilt. **Fix:** drag the shelf's grip up for more room; it's remembered.

## Related
- [The launcher](launcher.md): the page Solis starts on.
- [Projects and the Library](projects-and-the-library.md): the Library.
- [Play mode](play-mode.md): Play, Pause, Step and Stop.
- [Concepts](concepts.md): panel, tab, ribbon, editor group, shelf, layout, workspace, accent colour.
