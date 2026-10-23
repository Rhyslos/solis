# 0025. Picking in the viewport, and the move, rotate and scale gizmos

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

Phase 3's first milestone: select things by clicking them in the viewport, and move, rotate and scale them with gizmos. This needs answers to four questions:
- how the editor finds what's under the mouse;
- what a click selects, especially inside an assembly's copy;
- how a gizmo drag becomes undoable changes;
- what the controls are.

## Controls (chosen by the project lead, 2026-10-23, CONTRIBUTING rule 17)

| Action | Control |
|---|---|
| Select | Click |
| Add to or take from the selection | Ctrl+click |
| Box selection | Drag on empty space; Ctrl+drag adds |
| Select nothing / everything | Esc / Ctrl+A |
| Move, rotate, scale, all three | W, R, S, T (unchanged since Phase 0) |
| Snapping | The buttons in the viewport's bottom-right corner; holding Ctrl flips snapping for one drag |
| Ten times finer | Hold Shift while dragging |
| Cancel a drag | Esc, or a right click |
| Lock an axis (drags leave it alone, its handles disappear) | X, Y, Z, or the buttons in the corner; press again to free it |
| World or object axes | L, or the button in the corner |

These are Aventail's (its decision 0030), so they feel the same in both programs. Aventail needs Shift for the axis locks because plain X and Z are taken there; in Solis they're free.

## Options for finding what's under the mouse

1. **Rays against triangles, on the processor.** Needs every model's triangles kept in the editor and a search structure per model; a box selection would have to test every triangle against the box.
2. **An ID image on the GPU (chosen).** When asked, the placements are drawn once more into a small image where each pixel holds the number of the placement there and its depth. Exactly what's drawn is what's picked, a box is the same question for a bigger rectangle, and the depth gives the world point under the mouse (for dropping things from the Library in M3).

Lamps have no surface, so they're picked by their markers on screen, in the editor.

## Decision

**Picking:**
- `render` draws the ID image only when asked (`ViewportRenderer::request_pick`), only inside the rectangle asked about (a scissor), with its own small shader. The answer is copied back without waiting (rule 10) and collected on a later frame (`ViewportTarget::poll_pick`), one question per viewport at a time.
- The editor gives each drawn model a number (`ViewModel::pick`) and keeps the entity behind each number from when it asked, so an answer that arrives a frame later still means the right entities. A renderer that never answers is given up on after two seconds.

**What gets selected** (`editor::picking`):
- A click selects the entity drawn under the mouse; a click on nothing selects nothing.
- **A copy of an assembly is selected whole first** (its root); once something in the copy is selected, a click selects the entity itself, so a second click goes inside. That is how Unity treats prefabs, and it keeps copies from being pulled apart by accident.
- A box selects everything with a visible pixel inside it, copies whole.

**Gizmos:**
- The maths is Aventail's (`tools::gizmo`, copied with its tests under decision 0007): handles' shapes on screen, which one is under the mouse, and turning mouse movement into a change. The viewport draws the shapes over the image, a fixed size on screen.
- **One drag, one undo step:** each frame sets every moved entity's Transform from where it was when the drag began, merged by the drag's gesture (decision 0022). Cancelling sets them back with the same key, so the step disappears. This is simpler than Aventail's "take back the last step" because Solis's history already merges and drops a step that ends where it started.
- An entity inside another selected one moves with its parent, not twice. The gizmo sits at the middle of the moved entities.
- A move keeps rotation and scale exactly; a turn keeps scale. Scaling a turned entity along other axes can't be shown without a skew, so the nearest shape without one is used (as Aventail does).
- **Snapping:** moves land on the grid lines (1 m, 50 cm, 25 cm or 10 cm); with object axes they move in grid steps. Turns go in 5°, 15°, 45° or 90° steps; scaling in 10% steps. The settings and locks are saved per computer.

## Consequences

- Picking costs nothing while nobody clicks; a click draws the placements once more inside a one-pixel scissor.
- The instance data the GPU reads grows by one number per copy (100 to 104 bytes).
- Snapping to other objects' points and surfaces (Aventail has it) isn't built; moves snap to the grid only. It can come later through the same depth image.
- The viewport's outline of the selection is a box around each model (or a ring around a lamp), drawn over the image; a true outline of the shape would need another pass and can come later.
