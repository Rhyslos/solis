# Tile maps

> Lay out a 2D world as a grid of cells (grass, tilled earth, water) in layers under and over your sprites, with values every cell carries, such as what's planted there and whether it's watered.

## When to use it
A farm is a grid. Each square of ground is grass, tilled earth or water, and as you play it changes: the hoe tills it, a seed is planted, the can waters it, the crop grows. A **[tile map](concepts.md#2d)** keeps that grid, and the running game changes it cell by cell.

A tile map stores what each cell *means*, not a picture:
- **[Terrains](concepts.md#2d)** (grass, tilled, water) come from a **[tileset](concepts.md#2d)**, a list shared by every map in the project.
- **[Layers](concepts.md#2d)** stack: Ground, then Decoration (fences, flowers), both drawn **under** sprites, and Above (roofs, treetops), drawn **over** them so the farmer walks behind a roof.
- **[Cell values](concepts.md#2d)** are things every cell has besides its terrain. *crop* is a choice (wheat, corn), *growth* is a number, and *watered* is a switch.

Why meaning rather than pictures? Because the art can change without touching a single map: Solis works out which picture each cell shows from its neighbours, using the tileset's art ([Tilesets and autotiling](tilesets.md)). And because "every watered crop" is a question about values, not about how things look.

Why values in the map, rather than an entity per plant? A field of a thousand crops as a thousand entities is a lot to run and save. As values, they're a few bytes each, saved with the map. A plant that needs to animate or think can still be an entity standing on its cell.

For a few things that move or think (a scarecrow that turns, a chicken), use [sprites](sprites.md) instead.

## How to use it
1. **Make a tileset:** in the Library, open the folder you want it in, then **New > Tileset** (right-click in the Library, or **+ New**). Solis makes `tileset.tileset.lst` with grass, tilled and water, and opens it in the **Tileset** tab, where terrains get their art (see [Tilesets and autotiling](tilesets.md)). Choose a terrain to change its name, its **code** (the one character the map file writes for it) and its colour in Properties; **+** adds one. Clicking a tileset in the Library shows its terrains in Properties too: change them there, add one with **+ Terrain**, or take one away with **×**. Every change to a tileset is an [undo](undo.md) step of that tileset's, wherever you make it: **Ctrl + Z** with the pointer over Properties while it shows the tileset, or over the Tileset tab, takes it back, so a terrain removed by mistake comes back with its tiles and rules. Dragging a colour is one step, made when you let go. The first terrain fills new maps.
2. **Make a map:** **New > Tile map**. Solis makes `map.tilemap.lst`, 64 × 64 cells of the project's first tileset's first terrain. Click it in the Library to change its **Tileset** and its **Size** (in cells) in Properties.
3. **Place it:** double-click the map in the Library (it goes where the viewport looks), or drag it into the viewport. Its top-left corner lands on a whole tile, and each cell is one tile across (the tile size in [2D settings](2d-pixel-perfect.md)). The **2D** view shows it best.
4. **The Map tab** opens beside Properties while the map's entity is selected (select it in the Area panel). It shows:
   - **Layers**, top first. Each has a small picture of its cells, its name (click to rename), **under sprites** or **over sprites**, an eye that shows or hides it here in the editor only (the game always shows it), and **…** to move it up or down or delete it. **+** adds a layer on top.
   - **Terrains**, the tileset's, each with how many cells use it. They're the [tile painter's](painting-tiles.md) palette: click one to paint with it.
   - **Cell values**. **+** adds one (a switch to start). Change its name, its kind (**number**, **switch** or **choice**) and a choice's options (names with commas between).
   - **The cell under the pointer** in the viewport: each layer's terrain and each value.
5. **See the values on the map:** while the map is selected, the **Values** menu at the top of the viewport lays chosen values over the cells: a watered cell is tinted blue, and a number or a choice is written on its cell when you're zoomed in enough to read it. **Chunks** outlines the map's 32 × 32 chunks.
6. **Play:** each placed map is the game's own copy while it runs, so the game can change it and the file stays as it was. Select the map while playing to watch its cells in the Map tab. [Save games](save-games.md) keep the map as the game left it.

Every change in the Map tab is saved at once. **Ctrl + Z** and **Ctrl + Y** over the tab undo and redo them.

### Art and painting
A terrain is drawn in its colour until its tileset gives it art: then each cell's tile is picked from its neighbours, with smooth edges (see [Tilesets and autotiling](tilesets.md)). Seen from far away, a big map is drawn in its colours again, which is much quicker.

Paint a map's cells with the **tile painter**: while the map is selected, its brush, rectangle, line, fill and eyedropper join the tool bar at the bottom of the viewport, and the Map tab's terrain swatches are its palette (see [Painting tiles](painting-tiles.md)). The file is plain text you can also edit by hand: each chunk of a layer is rows of codes (`.` grass, `t` tilled, a space for nothing), and cells painted with an exact tile are listed in `exact` blocks.

### The file

A tile map is an ordinary Lust file, `farm.tilemap.lst`, and its tileset is another, `ground.tileset.lst`. Open one in Ledger to read it: a list of the map's layers and values, then its cells in chunks of 32 × 32, each layer's rows of terrain codes (a space for an empty cell). It's safe to change by hand, keeping to the shapes Solis writes (`lock` values made of numbers, text in quotes, lists in `[ ]` and `new` things): Solis checks the file against those and then as Lust when it reads it, and a mistake is shown with its line. Chunks keep a change to one cell to one line, so Git shows exactly what changed. (Older maps, `.soltilemap`, move to Lust by themselves when the project opens: see [Backups and upgrades](backups-and-upgrades.md).)

### In game code
Each tile map gets a Lust file of its own, which Solis writes into `solis/maps/` whenever the map or its tileset changes: `maps/farm.tilemap.lst` gives `solis/maps/maps/farm.lst`. In it are [states](concepts.md#game-code) of the map's layers (`FarmLayer`), its tileset's terrains (`FarmTerrain`, with `Empty` for nothing painted) and each choice value's options (`FarmCrop`), and a class with a method for each thing code can ask or change (`FarmMap`). So *watered* misspelt is a mistake before the game runs, and renaming it in the Map tab points at every line still using the old name.

A component holds the map as a field with no starting value, which Solis fills in:

```lust
fetch "solis/maps/maps/farm":[FarmMap, FarmLayer, FarmTerrain, FarmCrop];

component Sprinkler {
    has {
        FarmMap farm;        -- filled in by Solis
        int x = 3;
        int y = 4;
    }

    method update(float dt) {
        -- Water the cell if it's tilled soil and dry.
        bool soil = farm:terrain(FarmLayer.Ground, x, y) == FarmTerrain.Soil;
        is soil && not farm:watered(x, y)? then {
            farm:setWatered(x, y, true);
            farm:setGrowth(x, y, farm:growth(x, y) + 1);
        }

        -- Plant parsnips where nothing grows yet.
        is farm:crop(x, y) == FarmCrop.None? then {
            farm:setCrop(x, y, FarmCrop.Parsnip);
        }
    }
}

pass Sprinkler;
end
```

| In the Map tab | becomes |
|---|---|
| its size | `farm:width()`, `farm:height()` (in cells) |
| a layer's terrain on a cell | `farm:terrain(FarmLayer.Ground, x, y)`, `farm:setTerrain(FarmLayer.Ground, x, y, FarmTerrain.Soil)` |
| a number value *growth* | `farm:growth(x, y)`, `farm:setGrowth(x, y, to)` |
| a switch value *watered* | `farm:watered(x, y)`, `farm:setWatered(x, y, on)` |
| a choice value *crop* | `farm:crop(x, y) == FarmCrop.Parsnip`, `farm:setCrop(x, y, FarmCrop.Corn)` |
| the cell at a place in the world | `farm:cellAt(position())`: a `MapCell` with `inside` (on the map?), `x` and `y` |
| the place at a cell's middle | `farm:placeOf(x, y)` |
| the box a map's entity covers | `ground:mapBounds()`: a `MapBounds` with `found`, `low` and `high` corners |

**Which cell is the player on?** Ask the map with the player's place. `MapCell` comes from `solis/tiles`:

```lust
fetch "solis/maps/maps/farm":[FarmMap, FarmLayer, FarmTerrain];
fetch "solis/tiles":MapCell;

component Hoe {
    has {
        FarmMap farm;
    }

    method update(float dt) {
        MapCell here = farm:cellAt(position());
        is here.inside && farm:terrain(FarmLayer.Ground, here.x, here.y) == FarmTerrain.Grass? then {
            farm:setTerrain(FarmLayer.Ground, here.x, here.y, FarmTerrain.Tilled);
        }
    }
}

pass Hoe;
end
```

It works wherever the map's entity is, even moved or turned. Cells are counted from the map's top left corner, x across and y down, like the Map tab; a place off the map still gets the cell it would be (handy for a tool reaching past the edge), with `inside` false. A 2D camera that stays inside the map is ready-made: **FollowCamera2D** ([Characters and cameras](characters-and-cameras.md)).

- Cells count from 0 at the map's first corner, as the Map tab shows them. A cell off the map reads as empty, 0 or off, and changing it does nothing.
- Changes land at the end of the tick, like moves, and what code reads is the map as the tick started. They're in saves, and a solid terrain painted by code becomes a wall at once.
- The map is the one placed by the first entity showing that file (most games have one per file).

## Settings
The Tile Map component:

| Setting | What it does |
|---|---|
| Map | The tile map file (`.tilemap.lst`) it shows. Its top-left corner is at the entity. |

A map file's Properties:

| Setting | What it does |
|---|---|
| Tileset | The terrains its cells can be. Giving a map with an empty ground its first tileset fills the ground with that tileset's first terrain. |
| Size | Its width and height in cells, up to 4,096 each. Making it smaller takes away the cells past its new edges. A map that big with a lot in it can be too big to open again (see the problems below). |

A terrain's settings (right-click its swatch in the Map tab):

| Setting | What it does |
|---|---|
| Solid | Nothing with a 2D collider walks into its cells: fences, water, walls. A small wall mark shows on its swatch, and the viewport's **Walls** switch shows the walls it makes. See [Walls and walking in 2D](walls-and-walking.md). |

Value kinds:

| Kind | Holds | Starts at |
|---|---|---|
| Number | A whole number, such as days of growth | 0 |
| Switch | On or off, such as watered | Off |
| Choice | One of its options, such as wheat or corn | None |

## Shortcuts
| Action | Shortcut |
|---|---|
| Undo or redo a change in the Map tab, or a painted stroke | Ctrl + Z, Ctrl + Y (pointer over the tab, or a painting tool in use) |
| Undo or redo a change to a tileset (its terrains, **Solid**) | Ctrl + Z, Ctrl + Y (pointer over Properties while it shows the tileset, or over the Tileset tab) |
| A terrain's settings | Right-click its swatch in the Map tab |
| The painting tools | B, U, N, G, I (see [Painting tiles](painting-tiles.md)) |

## Tips and common problems
- **Problem:** cells are bright pink. **Why:** their code isn't one of the tileset's terrains (the terrain was taken away, or its code changed), or their terrain's art is missing the tile they need. The Map tab counts unknown codes, and Problems names missing tiles. **Fix:** add a terrain with that code, change the cells, or fill the slot in the Tileset tab.
- **Problem:** clicking the map in the viewport selects what's behind it, or nothing. **Why:** clicks on a map are kept for the [tile painter](painting-tiles.md). **Fix:** select the map in the Area panel.
- **Problem:** the Map tab is gone. **Why:** it shows only while a map is selected. **Fix:** select the map's entity.
- **Problem:** numbers don't show on the cells. **Why:** cells are too small to write on, or too many are in view. **Fix:** zoom in.
- **Problem:** a value's numbers are gone after changing its kind. **Why:** a value's cells start again when its kind changes. **Fix:** Ctrl + Z over the Map tab.
- **Problem:** on a very big map, **+** adds no layer or value, or a value's kind won't change to number. **Why:** a map's layers and values together take at most 512 MB of memory. A layer, a switch or a choice takes a byte for each cell and a number four, so the biggest map (4,096 × 4,096) has room for 32 bytes a cell: 16 layers and 4 numbers, say. **Fix:** take away a layer or value the map doesn't need, or make the map smaller.
- **Problem:** a very big map won't open: "the map is too big to read" or "it's too big to read". **Why:** a map is kept as text, and reading text takes far more memory than the file's size, so Solis reads a map file of up to 64 MB with up to 12 million numbers, texts and symbols in it (about 2 GB of memory and 10 seconds at worst). The biggest map, 4,096 × 4,096, fits with three layers painted all over; each cell whose value isn't at its start counts too (a number in a million and a half cells is too many), and so does each exact tile. **Fix:** split a very big world into several maps, each on its own entity, and give values only to the cells that need them. Solis doesn't yet stop a map growing past this while you work on it, so open the project again now and then to check that a very big map still opens, and keep it in Git.
- Problems lists a Tile Map with no map chosen, or whose file isn't in the project.

## Related
- [Painting tiles](painting-tiles.md): the brush, rectangle, line, fill and eyedropper.
- [Tilesets and autotiling](tilesets.md): terrains' art.
- [Sprites from Aseprite](sprites.md): what stands on a map.
- [2D: the pixel-perfect view](2d-pixel-perfect.md): the tile size and the 2D view.
- [Save games](save-games.md): saves keep each map as the game changed it.
- [Concepts](concepts.md#2d): tileset, terrain, tile map, layer, cell value, chunk.
