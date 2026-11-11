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

**The panels** fill the middle of the window. On the Home tab they start with the Area panel on the left, the viewport in the middle, Properties on the right with Hints under it, and the Library, Console, Problems, Mixer, Profiler and Code as tabs along the bottom. Why there? The things you pick (in the Area) and what you change about them (in Properties) sit either side of what you're looking at, and everything else shares the bottom so the viewport stays big.

| Panel | Shares a space with | What it's for |
|---|---|---|
| Viewport | Game | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Viewport | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | | Every [entity](entities.md) in the area, as a tree: the sun, the [environment](sky-fog-and-exposure.md) (sky and fog), [lights](lights.md) and [imported models](importing-models.md) (each with a × to delete it), with lights from a model's file indented under it. Click one to see it in Properties. |
| Properties | | The selected entity's name and [components](entities.md), each in a card with its settings, a **?** for its manual page and a **⋯** for rarer actions; the **⋯** beside the name has **Move here**, copying and pasting, and **Delete**. Some things take its place while you work on them: a sound or sprite sheet chosen in the Library, a bus chosen in the Mixer, or the piece picked on the UI canvas. |
| Hints | (under Properties) | A few plain sentences about what's selected and what you can do with it, such as where a game code component's file is and that saving it in Ledger updates the game at once; with nothing selected, how to start. Turn hints off in **File > Preferences** once you know your way around. |
| Library | Console, Problems, Mixer, Profiler, Code | The open [project](projects-and-the-library.md)'s files: models, assemblies, pictures and sounds. Double-click a model or assembly, or click its +, to place it. Right-click anywhere in it, or use **+ New**, to make a code file, folder, area, tile map, tileset, animator or sound event. |
| Console | Library, and the rest along the bottom | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | Library, and the rest along the bottom | Mistakes found in the game's code and in the area, such as a missing model; a number on the tab says how many. See [Problems](problems.md). |
| Mixer | Library, and the rest along the bottom | The project's buses: a fader, meter, mute and solo each, and ducking. See [The Mixer](mixer.md). |
| Profiler | Library, and the rest along the bottom | Where each frame's time goes: game code, the world, drawing, the editor, and the graphics card. See [The Profiler](profiler.md). |
| Code | Library, and the rest along the bottom | The project's game code files in Lust, Rust or Lua. See [Game code](game-code.md). |

**Panels that come with a file.** Some panels are there only while what they show is open, so they don't take room the rest of the time. Most have a × on their tab to close them; the Map tab has none, and goes when you select something else. Tabs that show the same file close together: the × on **Clips** or **Frames** closes the clips file and both tabs, and the × on **Canvas** or **Elements** closes the screen and both of those, leaving every other open file alone. Nothing asks whether to save, because each change to these files is saved as you make it.

| Panel | When it's there | What it's for |
|---|---|---|
| Clips and Frames | While a clips file is open | A sprite sheet's clips, and the chosen clip's frames along a time ruler. See [Animating sprites](sprite-animation.md). |
| Animator | While an animator is open | Its states and the ways between them, as a graph. See [Animators](animators.md). |
| Map | While an entity with a tile map is selected | The map's layers and the terrains to paint with. See [Painting tiles](painting-tiles.md). |
| Tileset | While a tileset is open | Its terrains, their art and its rules. See [Tilesets and autotiling](tilesets.md). |
| Atlas Crop | While a picture's crops are open | The pictures cut out of one sheet. See [Atlas Crop](atlas-crop.md). |
| Canvas and Elements | On the UI tab, while a screen is open | The screen drawn to pick and change, and its pieces as a tree. See [The UI editor](ui-editor.md). |

**The Library as a drawer:** the Library starts pinned, as a panel along the bottom. Click **Unpin** in it and it leaves the layout, so the viewport gets that room; **Ctrl + Space** then pops it up over the editor whenever you want it, and **Pin** docks it again. With the Library pinned, Ctrl + Space brings it to the front. See [Projects and the Library](projects-and-the-library.md).

**The status bar** along the bottom says what's happening, so you don't have to go looking:
- **Code checked**, or how many mistakes the game's code has. Solis checks the code when the project opens and each time you save a code file; hover to see when it last did.
- **No problems**, or how many there are. Click it to open the [Problems](problems.md) panel.
- **Saved**, **Unsaved changes** or **Not saved yet**, for the open area ([Saving areas](saving-areas.md)).
- On the right: **Playing** or **Paused** while the game runs, then the area's name, whether the viewport shows it in 2D or 3D, and for 2D the game's size in pixels (`farm · 2D · 480 × 270`, from **File > Project settings**). For three seconds after you click a part that isn't built yet, those last words say so instead ([Unfinished parts](unfinished-parts.md)).

**Rearranging panels ([docking](concepts.md#the-window)):**
1. Drag a panel by its tab.
2. As you drag, a highlighted area shows where it will land. Drop it on the edge of another panel to put the two side by side, or in the middle to add it to that panel's tab group.
3. Drag the thin line between two panels to make one bigger.
4. The ▼ at the left of a panel's tab row folds the panel away; click it again to unfold.

Your layout is remembered next time. (When this layout became the standard one, every editor started from it once; your own arrangement from before is gone, so drag panels back if you liked them elsewhere.) If the saved layout is ever damaged (a crash while saving, a hand edit), Solis starts with the standard one instead.

## Settings
| Setting | What it does |
|---|---|
| Reset layout (in the search, and in **File > Preferences**) | Puts this tab's panels back where they started ([Settings](settings.md)). There are no other ready-made layouts: drag panels to make your own, and it's remembered. |
| File > Preferences | The editor's own settings, such as its accent colour: see [Settings](settings.md). |
| The viewport's ≡ menu | Switches the viewport's corner controls on and off (the search does too). |

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
| Pop the Library drawer up or away (or bring the pinned Library to the front) | `Ctrl + Space` |

See [Keyboard shortcuts](keyboard-shortcuts.md) for the rest.

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it may be folded away, or behind another tab. **Fix:** press **Space** and type its name, look for its tab in each tab group, or press **Space** and choose **Reset layout**.
- **Problem:** a menu from an older build is gone (File, Edit, Entity, Help). **Why:** the tabs and the ribbon took the menu bar's place. **Fix:** press **Space** and type what you want: the list says where it lives now.
- **Problem:** the Library isn't among the bottom tabs. **Why:** it's unpinned, so it's a drawer. **Fix:** press **Ctrl + Space** to pop it up, and click **Pin** in it to dock it again.

## Related
- [The launcher](launcher.md): the page Solis starts on.
- [Projects and the Library](projects-and-the-library.md): the Library and its drawer.
- [Play mode](play-mode.md): Play, Pause, Step and Stop.
- [Concepts](concepts.md): panel, tab, ribbon, docking, layout, workspace, accent colour.
