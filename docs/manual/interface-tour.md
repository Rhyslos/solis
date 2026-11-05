# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

**The top bar**, from left to right:
- **The logo**, then the **menus:** File, Edit, View, Entity, Play, Window and Help. **Play** has the play buttons, loads [save games](save-games.md), and records a play's input to play back ([Input](input.md)); **Edit > Input actions…** sets the game's controls and **Edit > Settings…** the editor's own ([Settings](settings.md)). **File > New project…**, **Open project…** and **Recent projects** work with [projects](projects-and-the-library.md), and **File > Start page** shows the start page again; **New area**, **Open area…**, **Save area** and **Save area as…** with [area files](saving-areas.md); **File > Import model…** brings a model in (see [Importing models](importing-models.md)); **Edit > Undo** and **Redo** take back changes (see [Undo and redo](undo.md)); **Help > Add the sample area** brings in a ready-made one, and **Help > Make the playground…** makes a project with a level to walk around in ([Characters and cameras](characters-and-cameras.md)). Only what's built is shown: things still to come (building a game to share, playing with two players, layers) appear when they work.
- **The area tab:** the open area ("Untitled area"), and tabs for the animator or tileset open.
- **Search or do anything…**: one search for everything (see below).
- **Play**, **Pause**, **Step** and **Stop**, on the right. They run the area as a game: see [Play mode](play-mode.md).

**The start page** comes up when Solis starts with no project open: **New project…**, **Open project…**, your recent projects, three examples to start from (the playground, the turret sample and the benchmark), four first steps, and a link to this manual. Tick **Don't show this at start** to skip it; **File > Start page** brings it back.

**One search for everything:** press **Space** (with nothing being typed) or click **Search or do anything…** in the top bar. Type part of a name and the list finds:
- every **action** in the menus (Save area, Add point light, Play…), with its shortcut beside it, so you pick up the keys as you go;
- every **file** in the project, which opens it (a model or sprite sheet is shown in the Library instead, ready to drag in);
- every **entity** in the area, which selects it and points the camera at it;
- every **panel**, which comes to the front.

Up and Down choose, **Enter** does it, **Esc** closes. Why? You don't have to remember which menu something is in: type what you want. Actions that can't be done right now (Pause while editing, Delete with nothing selected) aren't listed. In the Frames tab, Space still plays and stops the clip, so point somewhere else first.

**The status bar** along the bottom says what's happening, so you don't have to go looking:
- **Code checked**, or how many mistakes the game's code has. Solis checks the code when the project opens and each time you save a code file; hover to see when it last did.
- **Problems:** how many there are. Click it to open the [Problems](problems.md) panel.
- **Saved**, **Unsaved changes** or **Not saved yet**, for the open area ([Saving areas](saving-areas.md)).
- On the right: **Playing** or **Paused** while the game runs, then the area's name, whether the viewport shows it in 2D or 3D, and for 2D the game's size in pixels (`farm · 2D · 480 × 270`, from **Edit > Settings… > 2D**).

**The panels** start in the places shown in this picture of the plan for the editor: the Area panel on the left, the viewport in the middle, Properties on the right, and the Library, Console, Problems, Mixer, Profiler and Code as tabs along the bottom. Why there? The things you pick (in the Area) and what you change about them (in Properties) sit either side of what you're looking at, and everything else shares the bottom so the viewport stays big. (The rest of the picture, such as the search box and the Hints panel, is still to come.)

| Panel | Shares a space with | What it's for |
|---|---|---|
| Viewport | Game | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Viewport | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | | Every [entity](entities.md) in the area, as a tree: the sun, the [environment](sky-fog-and-exposure.md) (sky and fog), [lights](lights.md) and [imported models](importing-models.md) (each with a × to delete it), with lights from a model's file indented under it. Each icon has its kind's colour, the same as in the Library: models blue, a placed assembly yellow, the environment teal, cameras grey, and each lamp its own light's colour. Click one to see it in Properties. |
| Properties | | The selected entity's name and [components](entities.md), each in a card with its settings, a **?** for its manual page and a **⋯** for rarer actions; the **⋯** beside the name has **Move here**, copying and pasting, and **Delete**. |
| Library | Console, Problems, Mixer, Profiler, Code | The open [project](projects-and-the-library.md)'s files: models, assemblies, pictures and sounds. Double-click a model or assembly, or click its +, to place it. Right-click anywhere in it, or use **+ New**, to make a code file, folder, area, tile map, tileset, animator or sound event. |
| Code | Library, and the rest along the bottom | The project's game code files in Lust, Rust or Lua. See [Game code](game-code.md). |
| Console | Library, and the rest along the bottom | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | Library, and the rest along the bottom | Mistakes found in the area, such as a missing model; a number on the tab says how many. See [Problems](problems.md). |
| Profiler | Library, and the rest along the bottom | Where each frame's time goes: game code, the world, drawing, the editor, and the graphics card. See [The Profiler](profiler.md). |
| Mixer | Library, and the rest along the bottom | The project's buses: a fader, meter, mute and solo each, and ducking. See [The Mixer](mixer.md). |

**Rearranging panels ([docking](concepts.md#the-window)):**
1. Drag a panel by its tab.
2. As you drag, a highlighted area shows where it will land. Drop it on the edge of another panel to put the two side by side, or in the middle to add it to that panel's tab group.
3. Drag the thin line between two panels to make one bigger.
4. The ▼ at the left of a panel's tab row folds the panel away; click it again to unfold.

Your layout is remembered next time. (When this layout became the standard one, every editor started from it once; your own arrangement from before is gone, so drag panels back if you liked them elsewhere.) If the saved layout is ever damaged (a crash while saving, a hand edit), Solis starts with the standard one instead.

## Settings
| Setting | What it does |
|---|---|
| Window > Reset layout | Puts every panel back where it started. Also in **Edit > Settings…** ([Settings](settings.md)). There are no other ready-made layouts: drag panels to make your own, and it's remembered. |
| Edit > Settings… | The editor's own settings, such as its accent colour: see [Settings](settings.md). |
| View | Switches the viewport's corner controls on and off (also in the viewport's ≡ menu). |

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it may be folded away, or behind another tab. **Fix:** look for its tab in each tab group, or use Window > Reset layout.

## Related
- [Concepts](concepts.md): panel, tab, docking, layout, accent colour.
