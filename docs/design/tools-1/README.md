# docs/design/tools-1

The approved mock-ups of Tools 1: shape brushes and world sculpting ([the plan](../../plans/tools-1.md), [decision 0111](../../decisions/0111-tools-1-brushes-and-ground.md)). The project lead approved them on 2026-11-13, and the floating tools (07 and 08) on 2026-11-14. They are drawn in the G layout's language ([decision 0099](../../decisions/0099-the-g-layout.md)) on Home's standard layout ([decision 0109](../../decisions/0109-layouts-as-a-setting.md)).

| File | What it shows |
| --- | --- |
| `01-brush-list.png` | B pressed twice over the 3D view: the brush list by the pointer, Ramp highlighted, "B next · Space use · Esc close" at its foot. |
| `02-brush-draw.png` | Drawing out a box: its footprint snapped to the grid, the height being pulled up, the steps shown at the top. |
| `03-brush-edit-faces.png` | Editing a brush's faces: the HOME TOOLS · Brush tab (Points 1, Edges 2, Faces 3, Done Tab; Extrude E, Inset, Split, Delete; Material, Solid; Snapping), one face picked, the Brush card in Properties. |
| `04-brush-extrude.png` | An extrude under way: the new part's ghost, the distance, the gizmo. |
| `05-ground-sculpt.png` | The HOME TOOLS · Ground tab (Add, Dig, Smooth, Flatten, Paint; Size, Strength, Falloff; Look), the brush's circle, the Ground shelf tab (materials, mouse and keys, what's under the pointer). |
| `06-ground-looks.png` | One hill and cave in the Smooth, Blocky and Sharp looks. |
| `07-floating-tools-editing.png` | The tools floating at the foot of the view while a brush is edited ([decision 0114](../../decisions/0114-the-tools-float-in-the-view.md), approved 2026-11-14): Select, Move, Rotate, Scale and Transform with their keys small in a corner, a line, then Points 1, Edges 2 and Faces 3; Move and Faces lit; the keys' strip just above, the bottom-right pills beside. |
| `08-floating-tools-plain.png` | The same bar with no brush being edited: the five tools alone. |

## Settled with the approval

- **Ctrl** turns snapping off for one drag while drawing out or extruding, as it does for the gizmos today.
- **A Shape button** in Home's Create group opens the same brush list as B.

## Drawn this way, settled again when each milestone starts

- The list's order: Box, Ramp, Cylinder, Stairs, Arch, Sphere, Wedge.
- In the Area list, a brush shows its shape's name and icon beside it; new ones are named Box_03 and so on.
- While editing a brush's insides, the other entities are dimmed.
  - Ctrl + click adds to the pick, and a drag picks with a box, as for entities today.
- The Brush card shows:
  - what the brush was made as, its size, material and Solid;
  - the counts of its points, edges and faces;
  - what's picked.
- Brush materials are Material assets.
- The ground is a row in the Area list; how a ground is first added is not drawn yet.
- The ground brush's settings:
  - Size is a radius; Strength and Falloff are percentages.
  - Look is both in the ribbon and on the Ground card.
- The ground's size limit is still open; 128 × 128 m in the picture is an example.
- The palette's materials are the ground's own list for now.
- Blocky uses one-metre cubes.
