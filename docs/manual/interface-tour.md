# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

This page follows what you see, in the order you meet it: the launcher Solis starts on, then the editor from its top bar down to its status bar.

**The [launcher](launcher.md)** is what Solis always starts on. **Projects** lists your projects (pinned ones first, with **Search projects** and **Sort by**), beside **+ New project**, **Open project…** and **Scan a folder…**; **Learn** has four examples to start from, the first steps and a link to this manual; **Updates** shows the build you have and the newest one. Double-click a project and the editor opens with it. Why start there? Solis doesn't open the last project by itself, so you always choose what to work on. **File > Projects…** shows the launcher again, and **Esc** leaves it for the editor. See [The launcher](launcher.md).

**The top bar**, from left to right:
- **The logo**, then the **menus**. While the game plays, the items that would change the project are greyed: point at any greyed item or button and it says why.
  - **File:** **New project…**, **Open project…** and **Recent projects** work with [projects](projects-and-the-library.md), and **Projects…** shows the launcher; **New area**, **Open area…**, **Save area** and **Save area as…** work with [area files](saving-areas.md); **Import model…** brings a model in ([Importing models](importing-models.md)); **Export for playtesting…** makes a folder with the game in it to send to someone ([Exporting for playtesting](playtest-export.md)); **Quit** closes Solis.
  - **Edit:** **Undo** and **Redo** take back changes ([Undo and redo](undo.md)); **Duplicate**, **Delete**, **Rename**, **Select all** and **Select nothing** work on the selected [entities](entities.md); **Input actions…** sets the game's controls ([Input](input.md)) and **Settings…** the editor's own ([Settings](settings.md)).
  - **View** switches the viewport's corner controls on and off.
  - **Entity** adds an empty entity, a point light, a spotlight or a camera, and has **Create assembly from selection** ([Assemblies](assemblies.md)).
  - **Play** has the same Play, Pause, Step and Stop as the buttons on the right, **Load a save…** ([Save games](save-games.md)), and **Play and record inputs** and **Replay a recording…** ([Input](input.md)).
  - **Window** has **Reset layout** (see [Settings](#settings) below).
  - **Help:** **Open the manual ↗** opens this manual in your browser; **Add the sample area** brings in a ready-made area; **Make the sample game…**, **Make the UI sample…**, **Make the benchmark…** and **Make the playground…** each make an example project (the same four as the launcher's Learn page); **Check for updates** looks for a newer build ([Installing and updating](installing-and-updating.md)).
- **The workspaces:** **Level**, **Animate**, **Code** and **UI** (see below).
- **The area tab** shows the open area's name ("Untitled area" until it has a file), with a dot after it while it has unsaved changes. An animator or tileset you open gets a tab of its own beside it, with a × to close it: click a tab to bring that file, or the area, to the front.
- **Search or do anything…**: one search for everything (see below).
- **Play**, **Pause**, **Step** and **Stop**, on the right. They run the area as a game: see [Play mode](play-mode.md). Once an update has downloaded, a **Restart to update** button appears just left of them ([Installing and updating](installing-and-updating.md)).

Only what's built is shown. Things that aren't built yet (playing with two players, and layers that group entities) appear when they work.

**[Workspaces](concepts.md#the-window), one per job:** the tabs after the menus switch the whole layout at once, so each kind of work has the panels it needs where it needs them. Each remembers how you arranged it, separately.
- **Level** is for building areas: the panels where **The panels** below says they start.
- **Animate** gives animating sprites room: the Library on the left to pick sheets and animators from, the viewport (an animator opens beside it), and a taller bottom for a clips file's Frames.
- **Code** is a testing layout to keep beside Ledger, where you write game code: the game big in the middle, your code files on the left (each says which of its components entities in the area use, as in `Player · 1`), Properties on the right (game code's values, live while playing), and the Console and Problems side by side along the bottom.
- **UI** draws game screens: the screen open on the **Canvas** in the middle, its pieces in **Elements** on the left, and the piece picked in **Properties** on the right. Double-click a screen's file in the Library and Solis switches to UI with it open. See [The UI editor](ui-editor.md).

The panels that come with an open file or a selected tile map (see the second table below) come with you when you switch, except the Canvas and Elements, which stay in UI. **Window > Reset layout** resets only the workspace you're in.

**One search for everything:** press **Space** (with nothing being typed) or click **Search or do anything…** in the top bar. Type part of a name and the list finds:
- the **actions** in the menus (Save area, Add point light, Play…), each with its shortcut beside it when it has one, so you pick up the keys as you go; switching workspace and pinning the Library are there too;
- every **file** in the project, which opens it (a model or sprite sheet is shown in the Library instead, ready to drag in);
- every **entity** in the area, which selects it and points the camera at it;
- a **panel** by its name, which comes to the front.

Up and Down choose, **Enter** does it, **Esc** closes. Why? You don't have to remember which menu something is in: type what you want. Actions that can't be done right now (Pause while editing, Delete with nothing selected) aren't listed. In the Frames tab, Space still plays and stops the clip, so point somewhere else first.

**The panels** fill the middle of the window. In the Level workspace they start with the Area panel on the left, the viewport in the middle, Properties on the right with Hints under it, and the Library, Console, Problems, Mixer, Profiler and Code as tabs along the bottom. Why there? The things you pick (in the Area) and what you change about them (in Properties) sit either side of what you're looking at, and everything else shares the bottom so the viewport stays big.

| Panel | Shares a space with | What it's for |
|---|---|---|
| Viewport | Game | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Viewport | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | | Every [entity](entities.md) in the area, as a tree: the sun, the [environment](sky-fog-and-exposure.md) (sky and fog), [lights](lights.md) and [imported models](importing-models.md) (each with a × to delete it), with lights from a model's file indented under it. Click one to see it in Properties. |
| Properties | | The selected entity's name and [components](entities.md), each in a card with its settings, a **?** for its manual page and a **⋯** for rarer actions; the **⋯** beside the name has **Move here**, copying and pasting, and **Delete**. Some things take its place while you work on them: a sound or sprite sheet chosen in the Library, a bus chosen in the Mixer, or the piece picked on the UI canvas. |
| Hints | (under Properties) | A few plain sentences about what's selected and what you can do with it, such as where a game code component's file is and that saving it in Ledger updates the game at once; with nothing selected, how to start. Turn hints off in **Edit > Settings… > Appearance** once you know your way around. |
| Library | Console, Problems, Mixer, Profiler, Code | The open [project](projects-and-the-library.md)'s files: models, assemblies, pictures and sounds. Double-click a model or assembly, or click its +, to place it. Right-click anywhere in it, or use **+ New**, to make a code file, folder, area, tile map, tileset, animator or sound event. |
| Console | Library, and the rest along the bottom | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | Library, and the rest along the bottom | Mistakes found in the game's code and in the area, such as a missing model; a number on the tab says how many. See [Problems](problems.md). |
| Mixer | Library, and the rest along the bottom | The project's buses: a fader, meter, mute and solo each, and ducking. See [The Mixer](mixer.md). |
| Profiler | Library, and the rest along the bottom | Where each frame's time goes: game code, the world, drawing, the editor, and the graphics card. See [The Profiler](profiler.md). |
| Code | Library, and the rest along the bottom | The project's game code files in Lust, Rust or Lua. See [Game code](game-code.md). |

**Panels that come with a file.** Some panels are there only while what they show is open, so they don't take room the rest of the time. Most have a × on their tab to close them; the Map tab has none, and goes when you select something else.

| Panel | When it's there | What it's for |
|---|---|---|
| Clips and Frames | While a clips file is open | A sprite sheet's clips, and the chosen clip's frames along a time ruler. See [Animating sprites](sprite-animation.md). |
| Animator | While an animator is open | Its states and the ways between them, as a graph. See [Animators](animators.md). |
| Map | While an entity with a tile map is selected | The map's layers and the terrains to paint with. See [Painting tiles](painting-tiles.md). |
| Tileset | While a tileset is open | Its terrains, their art and its rules. See [Tilesets and autotiling](tilesets.md). |
| Atlas Crop | While a picture's crops are open | The pictures cut out of one sheet. See [Atlas Crop](atlas-crop.md). |
| Canvas and Elements | In the UI workspace, while a screen is open | The screen drawn to pick and change, and its pieces as a tree. See [The UI editor](ui-editor.md). |

**The Library as a drawer:** the Library starts pinned, as a panel along the bottom. Click **Unpin** in it and it leaves the layout, so the viewport gets that room; **Ctrl + Space** then pops it up over the editor whenever you want it, and **Pin** docks it again. With the Library pinned, Ctrl + Space brings it to the front. See [Projects and the Library](projects-and-the-library.md).

**The status bar** along the bottom says what's happening, so you don't have to go looking:
- **Code checked**, or how many mistakes the game's code has. Solis checks the code when the project opens and each time you save a code file; hover to see when it last did.
- **No problems**, or how many there are. Click it to open the [Problems](problems.md) panel.
- **Saved**, **Unsaved changes** or **Not saved yet**, for the open area ([Saving areas](saving-areas.md)).
- On the right: **Playing** or **Paused** while the game runs, then the area's name, whether the viewport shows it in 2D or 3D, and for 2D the game's size in pixels (`farm · 2D · 480 × 270`, from **Edit > Settings… > 2D**).

**Rearranging panels ([docking](concepts.md#the-window)):**
1. Drag a panel by its tab.
2. As you drag, a highlighted area shows where it will land. Drop it on the edge of another panel to put the two side by side, or in the middle to add it to that panel's tab group.
3. Drag the thin line between two panels to make one bigger.
4. The ▼ at the left of a panel's tab row folds the panel away; click it again to unfold.

Your layout is remembered next time. (When this layout became the standard one, every editor started from it once; your own arrangement from before is gone, so drag panels back if you liked them elsewhere.) If the saved layout is ever damaged (a crash while saving, a hand edit), Solis starts with the standard one instead.

## Settings
| Setting | What it does |
|---|---|
| Window > Reset layout | Puts this workspace's panels back where they started. Also in **Edit > Settings…** ([Settings](settings.md)). There are no other ready-made layouts: drag panels to make your own, and it's remembered. |
| Edit > Settings… | The editor's own settings, such as its accent colour: see [Settings](settings.md). |
| View | Switches the viewport's corner controls on and off (also in the viewport's ≡ menu). |

## Shortcuts
| Action | Shortcut |
|---|---|
| Search for any action, file, entity or panel (with nothing being typed) | `Space` |
| Choose a result in the search list | `Up`, `Down` |
| Do what's chosen | `Enter`, or click a result |
| Close the search | `Esc`, or click outside it |
| Pop the Library drawer up or away (or bring the pinned Library to the front) | `Ctrl + Space` |

See [Keyboard shortcuts](keyboard-shortcuts.md) for the rest.

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it may be folded away, or behind another tab. **Fix:** press **Space** and type its name, look for its tab in each tab group, or use Window > Reset layout.
- **Problem:** the Library isn't among the bottom tabs. **Why:** it's unpinned, so it's a drawer. **Fix:** press **Ctrl + Space** to pop it up, and click **Pin** in it to dock it again.

## Related
- [The launcher](launcher.md): the page Solis starts on.
- [Projects and the Library](projects-and-the-library.md): the Library and its drawer.
- [Play mode](play-mode.md): Play, Pause, Step and Stop.
- [Concepts](concepts.md): panel, tab, docking, layout, workspace, accent colour.
