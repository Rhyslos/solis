# Tile maps

> Lay out a 2D world as a grid of cells (grass, tilled earth, water) in layers under and over your sprites, with values every cell carries, such as what's planted there and whether it's watered.

## When to use it
A farm is a grid. Each square of ground is grass, tilled earth or water, and as you play it changes: the hoe tills it, a seed is planted, the can waters it, the crop grows. A **[tile map](concepts.md#2d)** keeps that grid, and the running game changes it cell by cell.

A tile map stores what each cell *means*, not a picture:
- **[Terrains](concepts.md#2d)** (grass, tilled, water) come from a **[tileset](concepts.md#2d)**, a list shared by every map in the project.
- **[Layers](concepts.md#2d)** stack: Ground, then Decoration (fences, flowers), both drawn **under** sprites, and Above (roofs, treetops), drawn **over** them so the farmer walks behind a roof.
- **[Cell values](concepts.md#2d)** are things every cell has besides its terrain. *crop* is a choice (wheat, corn), *growth* is a number, and *watered* is a switch.

Why meaning rather than pictures? Because the art can change without touching a single map: when tilesets get their art, Solis works out which picture each cell shows from its neighbours. And because "every watered crop" is a question about values, not about how things look.

Why values in the map, rather than an entity per plant? A field of a thousand crops as a thousand entities is a lot to run and save. As values, they're a few bytes each, saved with the map. A plant that needs to animate or think can still be an entity standing on its cell.

For a few things that move or think (a scarecrow that turns, a chicken), use [sprites](sprites.md) instead.

## How to use it
1. **Make a tileset:** in the Library, open the folder you want it in, then **Import > New tileset**. Solis makes `tileset.soltileset` with grass, tilled and water. Click it in the Library: Properties lists its terrains. Change a terrain's colour, its name, or its **code** (the one character the map file writes for it), take one away with **×**, or add one with **+ Terrain**. The first terrain fills new maps.
2. **Make a map:** **Import > New tile map**. Solis makes `map.soltilemap`, 64 × 64 cells of the project's first tileset's first terrain. Click it in the Library to change its **Tileset** and its **Size** (in cells) in Properties.
3. **Place it:** double-click the map in the Library (it goes where the viewport looks), or drag it into the viewport. Its top-left corner lands on a whole tile, and each cell is one tile across (the tile size in [2D settings](2d-pixel-perfect.md)). The **2D** view shows it best.
4. **The Map tab** opens beside Properties while the map's entity is selected (select it in the Area panel). It shows:
   - **Layers**, top first. Each has a small picture of its cells, its name (click to rename), **under sprites** or **over sprites**, an eye that shows or hides it here in the editor only (the game always shows it), and **…** to move it up or down or delete it. **+** adds a layer on top.
   - **Terrains**, the tileset's, each with how many cells use it.
   - **Cell values**. **+** adds one (a switch to start). Change its name, its kind (**number**, **switch** or **choice**) and a choice's options (names with commas between).
   - **The cell under the pointer** in the viewport: each layer's terrain and each value.
5. **See the values on the map:** while the map is selected, the **Values** menu at the top of the viewport lays chosen values over the cells: a watered cell is tinted blue, and a number or a choice is written on its cell when you're zoomed in enough to read it. **Chunks** outlines the map's 32 × 32 chunks.
6. **Play:** each placed map is the game's own copy while it runs, so the game can change it and the file stays as it was. Select the map while playing to watch its cells in the Map tab. [Save games](save-games.md) keep the map as the game left it.

Every change in the Map tab is saved at once. **Ctrl + Z** and **Ctrl + Y** over the tab undo and redo them.

### Painting and art
Painting cells comes with the **tile painter**, and pictures for terrains (with smooth edges worked out from neighbours) come with **tilesets' art**: the next milestones. Until then a terrain is drawn in its colour, and a map's cells come from its file. The file is plain text you can edit by hand: each chunk of a layer is rows of codes (`.` grass, `t` tilled, a space for nothing).

### In game code
Game code can't read or change a map yet. Under Solis's [API guidelines](../game-api-guidelines.md), it will reach each map through a Lust file Solis writes for it, with its terrains, layers and values as names the checker knows, so a misspelt *watered* is caught before the game runs. That arrives with the API milestone, as for [animators](animators.md).

## Settings
The Tile Map component:

| Setting | What it does |
|---|---|
| Map | The tile map file (`.soltilemap`) it shows. Its top-left corner is at the entity. |

A map file's Properties:

| Setting | What it does |
|---|---|
| Tileset | The terrains its cells can be. Giving a map with an empty ground its first tileset fills the ground with that tileset's first terrain. |
| Size | Its width and height in cells, up to 4,096 each. Making it smaller takes away the cells past its new edges. |

Value kinds:

| Kind | Holds | Starts at |
|---|---|---|
| Number | A whole number, such as days of growth | 0 |
| Switch | On or off, such as watered | Off |
| Choice | One of its options, such as wheat or corn | None |

## Shortcuts
| Action | Shortcut |
|---|---|
| Undo or redo a change in the Map tab | Ctrl + Z, Ctrl + Y (pointer over the tab) |

## Tips and common problems
- **Problem:** cells are bright pink. **Why:** their code isn't one of the tileset's terrains (the terrain was taken away, or its code changed). The Map tab says how many. **Fix:** add a terrain with that code, or change the cells.
- **Problem:** clicking the map in the viewport selects what's behind it, or nothing. **Why:** clicks on a map are kept for the tile painter. **Fix:** select the map in the Area panel.
- **Problem:** the Map tab is gone. **Why:** it shows only while a map is selected. **Fix:** select the map's entity.
- **Problem:** numbers don't show on the cells. **Why:** cells are too small to write on, or too many are in view. **Fix:** zoom in.
- **Problem:** a value's numbers are gone after changing its kind. **Why:** a value's cells start again when its kind changes. **Fix:** Ctrl + Z over the Map tab.
- Problems lists a Tile Map with no map chosen, or whose file isn't in the project.

## Related
- [Sprites from Aseprite](sprites.md): what stands on a map.
- [2D: the pixel-perfect view](2d-pixel-perfect.md): the tile size and the 2D view.
- [Save games](save-games.md): saves keep each map as the game changed it.
- [Concepts](concepts.md#2d): tileset, terrain, tile map, layer, cell value, chunk.
