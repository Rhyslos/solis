# Selecting and moving things

> Click things in the viewport to select them, then move, turn and resize them with the gizmos.

## When to use it

Whenever you build an area: picking a model to change it, arranging crates, turning a lamp to face a wall, making a rock bigger. Clicking in the viewport is the quickest way to pick something you can see; the [Area panel](entities.md) is better for things you can't see (an entity with no model, or one hidden behind another).

## How to use it

**Selecting:**
1. Click a model or a lamp's marker in the [viewport](concepts.md#the-3d-view). It's outlined in the accent colour, and the Area panel and Properties show it.
2. **Ctrl + click** adds another one to the selection, or takes it out if it's already selected.
3. **Drag on empty space** to draw a box: everything you can see inside it is selected. Hold **Ctrl** while dragging to add to what's selected.
4. **Esc** selects nothing; **Ctrl + A** selects everything in the area.

Clicking on empty sky or ground selects nothing.

**Copies of assemblies** are selected whole the first time: clicking any part of a placed lamp post selects the whole lamp post. Click again on a part to select just that part. This stops you from pulling a copy apart by accident. See [Assemblies](assemblies.md).

**Moving, turning and resizing:**
1. Select something.
2. Choose a tool with its key or its button at the bottom of the viewport: **Move (W)**, **Rotate (R)**, **Scale (S)**, or **Transform (T)**, which shows all three. **Select (Q)** shows no gizmo, so clicks only select.
3. A [gizmo](concepts.md#tools) appears on the selection. Drag one of its handles:
   - **Move:** drag an **arrow** to move along that axis, a **square** to move in that flat plane (the blue-sided square slides along the ground), or the **dot in the middle** to move across the screen.
   - **Rotate:** drag a **ring** to turn around that axis (the blue ring turns like a record on a turntable), or the **outer ring** to turn around the line you're looking along.
   - **Scale:** drag a **box** at the end of a line to stretch along that axis, a **square** to grow two axes together, or the **dot in the middle** to grow evenly (right or up grows, left or down shrinks).
4. While you drag, a note by the mouse says how far or how much ("X +1.50 m", "+15.0°", "× 1.30").
5. Let go. The whole drag is **one undo step**: Ctrl + Z puts it back.

To give up on a drag before letting go, press **Esc** or click the **right mouse button**: everything goes back to where it was, and nothing is added to undo.

When several things are selected, the gizmo sits in the middle of them and they move together. Something inside another selected entity (a lamp inside a selected room) moves with it, not twice.

## Settings

These are in the viewport's **bottom-right corner**. Solis remembers them on this computer.

| Setting | What it does |
|---|---|
| **X**, **Y**, **Z** | **Lock** an axis: drags leave it alone, and its handles disappear. The letter turns red while locked; click it or press its key again to free it. *Example:* lock **Z**, then drag the middle dot to slide something around without lifting it off the floor. |
| **World / Object** | Which way the gizmo's arrows point: along the world's axes (X, Y and Z of the whole area), or along the selected entity's own axes (useful for a turned object, to slide it along its own length). |
| **Grid 1 m** | **Snapping** for moving: lit, moves land on the grid lines, so things line up. The ▴ beside it chooses the step: 1 m, 50 cm, 25 cm or 10 cm. With object axes, moves go in steps of that size instead. |
| **Angle 15°** | Snapping for turning: lit, turns go in steps. The ▴ chooses 5°, 15°, 45° or 90°. |
| **Scale 10%** | Snapping for resizing: lit, sizes change in steps of 10%. |
| **Camera** | The camera's speed step (see [Moving around the viewport](navigating-the-viewport.md)). |

**Hold Ctrl while dragging** to flip snapping for that one drag: on if it's off, off if it's on. **Hold Shift** to drag ten times more finely, for small adjustments.

## Shortcuts

| Action | Shortcut |
|---|---|
| Select | Click |
| Add to or take from the selection | Ctrl + click |
| Select with a box | Drag on empty space (Ctrl + drag adds) |
| Select nothing | Esc |
| Select everything | Ctrl + A |
| Select, Move, Rotate, Scale, Transform | Q, W, R, S, T |
| Snapping on or off for one drag | Hold Ctrl while dragging |
| Ten times finer | Hold Shift while dragging |
| Cancel a drag | Esc, or right click |
| Lock or free an axis | X, Y, Z |
| World or object axes | L |

## Tips and common problems

- **Problem:** no gizmo appears. **Why:** the Select tool (Q) is chosen, nothing is selected, or what's selected has no position (the area's Environment, for example). **Fix:** press W, R, S or T, and select something with a Transform.
- **Problem:** an arrow or ring is missing. **Why:** its axis is locked (its letter is red in the corner), or the arrow points almost straight at you, so it's too short to grab. **Fix:** free the axis, or move the camera a little.
- **Problem:** clicking a part of a placed assembly selects the whole thing. **Why:** copies are selected whole first. **Fix:** click the part again.
- **Problem:** a turned box stretches oddly when scaled. **Why:** stretching a turned object along the world's axes would skew it, which a Transform can't hold, so Solis keeps the nearest shape without a skew. **Fix:** press **L** for object axes, so it stretches along its own sides.
- **Tip:** you can't select something hidden behind another model by clicking; select it in the Area panel instead.

## Related
- [Entities and components](entities.md): the Area panel and Properties
- [Undo and redo](undo.md)
- [Moving around the viewport](navigating-the-viewport.md)
- [Keyboard shortcuts](keyboard-shortcuts.md)
- Concepts: [gizmo, snapping, selection](concepts.md#tools)
