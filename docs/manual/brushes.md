# Brushes

> Shapes made in the editor itself: a box, a ramp, stairs, drawn out on the ground, saved in the area and solid in the game.

## When to use it
For blocking out a level: the floors, walls, ramps, steps and platforms you walk on while you work out how a place plays. A **brush** is a shape made of flat faces, kept in the area itself, so there's no model to make in another program first: you can try a layout, play it, change it and play it again in minutes. Once the layout is right, brushes can stay (simple, clean shapes suit many art styles) or be replaced by models made elsewhere.

Use an imported [model](importing-models.md) instead for anything detailed or curved, such as a character, a tree or a statue: a brush is made of flat faces.

## How to use it
A brush is made in two parts: you choose its shape from the **brush list**, then **draw it out** in the view: first its footprint on the ground, then its height.

**Choosing a shape**
1. Rest the pointer over the 3D view (or the 2D view) and press `B`. The brush list opens beside the pointer: Box, Ramp, Cylinder, Stairs, Arch, Sphere and Wedge, each with a small picture.
2. Press `B` again to move the highlight down the list, as many times as you need: after Wedge it comes back to Box. The status bar says which one is highlighted ("Choosing a shape brush: Ramp").
3. Press `Space` to take the highlighted shape, or click any row. `Esc`, or a click anywhere else, closes the list without taking one.

The **Shape** button on **Home**, in the **Create** group (its key, B, is written small beside it), opens the same list. It stays lit while the list is open and while you draw.

**Drawing it out**
1. Press the left mouse button on the ground where one corner of the brush should be, and drag to the opposite corner. This is the brush's **footprint**: its size shows in metres beside two of its edges, and in the steps at the top of the view ("1 Footprint · 4 × 3 m").
2. Let go. Now move the mouse **up** (without pressing anything): the brush rises from the footprint, and an arrow shows its height in metres.
3. Click to keep it. The brush is added to the Area list, named after its shape (**Box_01**, then Box_02; **Ramp_01** for the first ramp) with its shape's picture and name beside it, and picked, so its gizmo and Properties are ready.

Corners and heights land on the grid, in the step set in the ribbon's **Grid** (**Grid · 0.5**: half a metre), so brushes line up with each other; drawing out snaps whether or not Grid is ticked. Hold `Ctrl` while dragging or pulling up to draw freely, off the grid. `Esc` or a right-click gives the brush up at any point, leaving nothing behind. `Ctrl + Z` takes a brush you kept away again in one step, like any change ([Undo](undo.md)).

> **Good to know:** a drag that starts on top of a brush draws the new one on top of it, at that height: that's how you stack a platform on a block, or a step on a floor. Anywhere else it starts on the ground grid.

**The Ramp and the Stairs** climb along the longer side of the footprint, starting low where your drag started. Stairs get a step for every 25 cm of height; a **Cylinder** and a **Sphere** have 16 sides, and an **Arch**'s curve 12 pieces.

**In the 2D view** (and the flat Front and Right views), the footprint is drawn flat in the view itself and there is no height step: letting go makes the brush, one grid step deep. The status bar and Hints say so.

**What you see while drawing:** the steps along the top of the view (the shape, 1 Footprint, 2 Height, the one in hand outlined), the keys for that step along the bottom, the sizes beside the brush, and what's happening in the status bar ("Drawing a Box: height 1.75 m · Click to keep · Esc to give up"). The **Hints** card in Properties ([Interface tour](interface-tour.md)) says how it works too.

Once made, a brush moves, turns and resizes with the gizmos like any entity ([Selecting and moving](selecting-and-moving.md)), and its **Transform's scale** stretches it. Press **Play**: characters stand on it and bump into it, because it is **Solid**.

A brush is drawn **flat**: every face is one shade, so its edges look crisp, as a block-out should.

## Settings
The **Brush** card in Properties:

| Setting | What it does |
|---|---|
| Made as | The shape the brush is. Choosing another makes the brush again as that shape, filling the same box it filled (a 4 × 2 × 1 m box becomes a 4 × 2 × 1 m cylinder), in one undo step. |
| Solid | On: the game's characters and bodies collide with it, as with a floor or a wall. Off: it's only drawn, and things pass through it (a light shaft, a hint of where something goes). A brush never falls: it stays where it's placed. |
| Points, Faces, Materials | The brush's shape itself, made by the editor's tools: Properties only says how many numbers each holds. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Open the brush list (pointer over the 3D or 2D view) | `B` |
| Move down the list (it wraps round) | `B` |
| Take the highlighted shape | `Space`, or click its row |
| Close the list | `Esc`, or click elsewhere |
| Draw the footprint | Drag on the ground |
| Draw freely, off the grid | Hold `Ctrl` while dragging or pulling up |
| Keep the brush | Move the mouse up, then click |
| Give it up | `Esc`, or right-click |

With a tile map picked, `B` is the tile map's Brush instead ([Painting tiles](painting-tiles.md)); the Shape button still opens the list.

## Tips and common problems
- **Problem:** pressing `B` paints tiles instead of opening the list. **Why:** a tile map is picked, and B is its Brush. **Fix:** pick something else (or nothing, `Esc`), or use the **Shape** button.
- **Problem:** "Nothing made: the footprint had no size". **Why:** you clicked without dragging. **Fix:** press and drag to the opposite corner before letting go.
- **Problem:** "Nothing made: the brush had no height". **Why:** you clicked before moving the mouse up. **Fix:** after letting go of the footprint, move the mouse up until the height shows, then click.
- **Problem:** a brush stands in mid-air. **Why:** the drag started on top of another brush, so it was drawn at that height. **Fix:** move it down with the gizmo, or undo and start the drag on the ground.
- **Problem:** a character falls through a brush. **Why:** its **Solid** is off. **Fix:** tick **Solid**.
- **Problem:** a brush gives a body or a collider of its own as well. **Why:** a **Collider** component on the same entity is used instead of the brush's shape. **Fix:** take the Collider off to collide with the brush's own faces.
- **Problem:** a brush from a damaged file shows nothing, and the [Problems](problems.md) list names it. **Why:** its points or faces couldn't be read (the problem says which: a face naming a point that isn't there, say, or more than a brush may hold, 65,536 points or faces). **Fix:** the file's text is kept as it was, so it can be put right by hand; or choose its **Made as** again, which makes it again as that shape at its standard size; or delete it and draw it again.

## Related
- [Entities and components](entities.md): a brush is an entity with a Brush component.
- [Physics](physics.md): colliders and bodies, which a Solid brush works with.
- [Saving areas](saving-areas.md): how a brush is written in the area's file.
- [Keyboard shortcuts](keyboard-shortcuts.md): every key in one place.
- [Concepts](concepts.md#the-area): brush, footprint, mesh, face.
