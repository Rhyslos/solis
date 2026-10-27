# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

**The top bar**, from left to right:
- **Menus:** File, Edit, View, Entity, Play, Build, Window and Help. **Play** has the play buttons, loads [save games](save-games.md), and records a play's input to play back ([Input](input.md)); **Edit > Input actions…** sets the game's controls. **File > New project…**, **Open project…** and **Recent projects** work with [projects](projects-and-the-library.md); **New area**, **Open area…**, **Save area** and **Save area as…** with [area files](saving-areas.md); **File > Import model…** brings a model in (see [Importing models](importing-models.md)); **Edit > Undo** and **Redo** take back changes (see [Undo and redo](undo.md)); **Help > Add the sample area** brings in a ready-made one, and **Help > Make the playground…** makes a project with a level to walk around in ([Characters and cameras](characters-and-cameras.md)). Items that don't work yet are greyed out; point at one to see which phase brings it.
- **The area tab:** the open area ("Untitled area"). Opening more areas, assemblies and code side by side arrives in Phase 2.
- **Play**, **Pause**, **Step**, **Play with two players** and **Stop**, then **Build** and **Build & Play**, on the right. Play, Pause, Step and Stop run the area as a game: see [Play mode](play-mode.md). The others tell you in the Console which phase brings them.

**The panels:**

| Panel | Shares a space with | What it's for |
|---|---|---|
| Viewport | Game | The 3D world, in one, two or four views (flat ones too). See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Viewport | The running game, through its camera, when you press Play. See [Play mode](play-mode.md). |
| Area | Layers | Every [entity](entities.md) in the area, as a tree: the sun, the [environment](sky-fog-and-exposure.md) (sky and fog), [lights](lights.md) and [imported models](importing-models.md) (each with a × to delete it), with lights from a model's file indented under it. Click one to see it in Properties. |
| Layers | Area | The area's layers (Phase 2). |
| Properties | | The selected entity's name and [components](entities.md), each with its settings, plus **Move here** and **Delete**. |
| Library | Code | The open [project](projects-and-the-library.md)'s files: models, assemblies, pictures and sounds. Double-click a model or assembly, or click its +, to place it. |
| Code | Library | Game code in Lust, Rust or Lua (Phase 4). |
| Console | Problems | Messages from the editor and the engine, and a command line. See [The Console](console.md). |
| Problems | Console | Mistakes found in the area, such as a missing model; a number on the tab says how many. See [Problems](problems.md). |
| Profiler | Console | Where each frame's time goes: game code, the world, drawing, the editor, and the graphics card. See [The Profiler](profiler.md). |

**Rearranging panels ([docking](concepts.md#the-window)):**
1. Drag a panel by its tab.
2. As you drag, a highlighted area shows where it will land. Drop it on the edge of another panel to put the two side by side, or in the middle to add it to that panel's tab group.
3. Drag the thin line between two panels to make one bigger.
4. The ▼ at the left of a panel's tab row folds the panel away; click it again to unfold.

Your layout is remembered next time. If the saved layout is ever damaged (a crash while saving, a hand edit), Solis starts with the Default one instead.

## Settings
| Setting | What it does |
|---|---|
| Window > Layouts | **Default:** Library and Console side by side under the viewport. **Wide screen:** Library in a column on the left. **Small screen:** Library, Code, Console, Problems and Profiler as tabs in one space. **Reset layout** goes back to Default. |
| Window > Accent colour | Ten colours, each with a swatch: Petrol blue (the standard), Red, Copper, Orange, Amber, Gold, Olive, Marine green, Blue and Plum. All are dark enough that white text on them reads easily. Remembered next time. |
| View | Switches the viewport's corner controls on and off (also in the viewport's ≡ menu). |

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it may be folded away, or behind another tab. **Fix:** look for its tab in each tab group, or use Window > Layouts > Reset layout.

## Related
- [Concepts](concepts.md): panel, tab, docking, layout, accent colour.
