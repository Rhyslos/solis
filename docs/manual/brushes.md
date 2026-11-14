# Brushes

> Shapes made in the editor itself: a box, a ramp, stairs, saved in the area and solid in the game.

## When to use it
For blocking out a level: the floors, walls, ramps, steps and platforms you walk on while you work out how a place plays. A **brush** is a shape made of flat faces, kept in the area itself, so there's no model to make in another program first: you can try a layout, play it, change it and play it again in minutes. Once the layout is right, brushes can stay (simple, clean shapes suit many art styles) or be replaced by models made elsewhere.

Use an imported [model](importing-models.md) instead for anything detailed or curved, such as a character, a tree or a statue: a brush is made of flat faces.

**For now, a brush is added as a box** (Create › More › Box brush), and shaped by moving, turning and stretching it.

## How to use it
1. On **Home**, in the **Create** group, open **More** and choose **Box brush**. A box 2 m on each side stands where the camera looks, named **Box_01** (the next is Box_02), and is selected.
2. Move, turn and resize it with the gizmos, as any entity ([Selecting and moving](selecting-and-moving.md)). Its **Transform's scale** stretches it: a scale of 5, 5, 0.25 makes a floor 10 m square and half a metre thick.
3. Press **Play**: characters stand on it and bump into it, because it is **Solid**.
4. **Ctrl + Z** takes the box away again, like any change ([Undo](undo.md)).

A brush is drawn **flat**: every face is one shade, so its edges look crisp, as a block-out should.

## Settings
The **Brush** card in Properties:

| Setting | What it does |
|---|---|
| Made as | The shape the brush was made as. It's a note of where the brush started: changing it doesn't change the brush's shape. |
| Solid | On: the game's characters and bodies collide with it, as with a floor or a wall. Off: it's only drawn, and things pass through it (a light shaft, a hint of where something goes). A brush never falls: it stays where it's placed. |
| Points, Faces, Materials | The brush's shape itself, made by the editor's tools: Properties only says how many numbers each holds. |

## Tips and common problems
- **Problem:** a character falls through a brush. **Why:** its **Solid** is off. **Fix:** tick **Solid**.
- **Problem:** a brush gives a body or a collider of its own as well. **Why:** a **Collider** component on the same entity is used instead of the brush's shape. **Fix:** take the Collider off to collide with the brush's own faces.
- **Problem:** a brush from a damaged file shows nothing. **Why:** its points or faces couldn't be read (a face naming a point that isn't there, or more than a brush may hold: 65,536 points or faces). **Fix:** the file's text is kept as it was, so it can be put right by hand, or the brush deleted and made again.

## Related
- [Entities and components](entities.md): a brush is an entity with a Brush component.
- [Physics](physics.md): colliders and bodies, which a Solid brush works with.
- [Saving areas](saving-areas.md): how a brush is written in the area's file.
- [Concepts](concepts.md#the-area): brush, mesh, face.
