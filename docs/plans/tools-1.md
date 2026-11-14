# Tools 1: shape brushes and world sculpting

**Status:** planned on 2026-11-13 from the project lead's answers; the mock-ups are approved (below). B1 is built (see "As built" below). Decisions [0108](../decisions/0108-shape-brushes-and-sculpting-come-next.md) (the order) and [0111](../decisions/0111-tools-1-brushes-and-ground.md) (what the tools are).

## What the project lead asked

On 13 November 2026: "having brushes to be able to create shapes in engine is important. What I want is to be able to press B to open the brush list. Continuously pressing B traverses the list downwards, and space confirms selection. We also need a world sculpting tool, and I need all of these changes to be performant."

Their answers, the same day:

| Question | Answer |
| --- | --- |
| What a brush is | Shapes edited point by point, edge by edge and face by face, with extruding, as level editors of the brush kind do |
| Placing one | **Draw it out:** drag on the ground for its footprint, move the mouse up for its height, click |
| The brush list | **B** over the 3D or 2D view opens it, each **B** moves down, **Space** confirms while it's open, **Esc** closes; with a tile map picked, B stays the tile Brush (decision 0108) |
| Editing a brush's insides | **Tab** toggles editing the picked brush; while editing, **1** points, **2** edges, **3** faces |
| Extruding | **E**, then move the mouse and click (Esc or right-click gives up); **and** Shift + drag on the move gizmo |
| What sculpting shapes | A **ground with caves and overhangs**, not a height map alone |
| The sculpting mouse | **Left-drag adds, Ctrl + left-drag digs, Shift + left-drag smooths**; **[** and **]** the brush's size, **Shift + [** and **]** its strength |
| The ground's look | **A setting per ground**, so each game picks its art style: Smooth (like clay), Blocky (cubes), Sharp (smooth, with hard edges at cliffs and cuts) |
| Painting materials | **In this phase too** (grass, rock, sand…) |
| Speed | Everything fast, with scale cases (decision 0108) |

## Two phases

Tools 1 is too big for one phase (CLAUDE.md: two to six milestones), so it is two, each with its own gate.

### Tools 1a: brushes

| Milestone | What it gives |
| --- | --- |
| **B1 The shape core** | Aventail's polygon mesh (n-sided faces with full adjacency, stable IDs, operations as pure functions; decision 0007 lets its code be copied in) as a new `mesh` crate, trimmed to what brushes need, with Solis's limits on everything it reads. Generators for the list's shapes (box, ramp, wedge, cylinder, stairs, arch, sphere). A **Brush** component holding its mesh in the area file (a format change: version raised, upgrader, fuzz test; rule 23). Drawn as a mesh made again only when it changes; solid through a triangle-mesh collider (physics gains them). |
| **B2 The list and drawing out** | B, Space and Esc as answered; drawing the footprint on the grid (snapping, sizes shown), then the height, then click; one undo step; the brush in the Area list like any entity. |
| **B3 Editing the insides** | Tab, 1, 2, 3; picking points, edges and faces (click, Ctrl + click, box); the move, rotate and scale gizmos on what's picked; every change through the command stack, so undo works and the area file stays valid. |
| **B4 Extrude and shaping** | E then move and click, and Shift + drag on the gizmo; the other shaping actions the approved mock-up shows (Inset, Split, Delete). A brush made inside out or with holes is refused or repaired, never saved broken. |
| **B5 Materials, speed, the gate** | A material per face; scale cases (10,000 brushes in an area; editing a 5,000-face brush; drawing out with 10,000 brushes around); the manual. **Gate:** a small level blocked out with brushes, edited and extruded, played (walked on, collided with), saved and reopened. |

### Tools 1b: the ground

| Milestone | What it gives |
| --- | --- |
| **G0 The study** (rule 22) | Competing ways to store and draw a ground with caves, built and measured against each other in `docs/reports/`: storage (fixed chunks of voxels; sparse chunks that store only the surface; an octree), and drawing each look (Smooth: surface nets, marching cubes; Sharp: dual contouring, or sharp features added to surface nets; Blocky: cube faces merged). Measured at the scale target: one brush stroke from mouse to screen, a chunk made again, memory and file size per square kilometre, triangles drawn, collision per chunk. The best of each goes into decision records. |
| **G1 The ground, Smooth** | One ground per area, in chunks, its own file (format, version, fuzz test); drawn in the Smooth look, chunks made on workers, never on the frame; solid to physics chunk by chunk; saved and loaded. |
| **G2 Sculpting** | The **HOME TOOLS · Ground** tab: Add, Dig, Smooth, Flatten; the brush's size, strength and falloff; the mouse and keys as answered. A stroke is one undo step, saved when let go; only touched chunks are made again. |
| **G3 Blocky and Sharp** | The other two looks, chosen per ground, switching at once. |
| **G4 Painting** | Materials painted with the same brush, from a palette on the shelf; blended where they meet; drawn without stretching on steep and overhanging ground. |
| **G5 Speed, game code, the gate** | Scale cases (the biggest ground the scale target names, sculpting it, walking 2,000 units over it); game code asking the ground questions (its height, a ray against it), following the game-code rules (decision 0065); the manual. **Gate:** a level of brushes on a sculpted, painted ground with a cave, in each look, played, saved and reopened, at the scale check's targets. |

### As built

- **B1** (2026-11-13): the `mesh` crate (Aventail's mesh trimmed to the n-gon mesh, topology, attribute layers, builder, triangulation, shading, primitives, extrude and inset), the seven shapes (`mesh::Shape`, settings within limits), the `Brush` component (made as, Solid, the mesh as three lists of numbers; counts checked before anything is made; area files' version 3), brushes cut into flat triangles only when the document's brush revision changes and shared when equal, solid as a still triangle surface in physics, and **Create › More › Box brush** until B2's list. Left for B2 onward: the list and drawing out (the other shapes can't be added from the editor yet); changing **Made as** doesn't rebuild the mesh; a damaged brush reads as empty without a Problems entry; each face's material slot is saved but every face draws the plain material (B5); an undo or redo looks at every brush's values again (10,000 boxes took 94 ms in a debug build; release not yet measured), and a brush deleted then brought back by undo is cut into triangles again.

## Mock-ups (approved 2026-11-13)

In [docs/design/tools-1/](../design/tools-1/README.md), drawn in the design's language on Home's standard layout (decision 0109). With the approval, Ctrl turns snapping off for one drag while drawing out or extruding (as for the gizmos), and a Shape button in Home's Create group opens the brush list too.

1. The brush list open (B pressed twice, Ramp highlighted).
2. Drawing out a box (footprint and height).
3. Editing a brush's faces (the HOME TOOLS · Brush tab, a Brush card in Properties).
4. An extrude under way.
5. Sculpting the ground (the HOME TOOLS · Ground tab, the brush's circle, the Ground shelf tab with the palette).
6. One hill and cave in each look: Smooth, Blocky, Sharp.

## Still to settle (asked when their milestone starts)

- The ground's file: text, as areas are, or a binary chunk file (voxel data as text is large); a decision either way.
- How big a ground can be (the scale target for it), and how it ends at its edges.
- Which shapes are in the list at first, and their standard sizes.
- What game code may change on a ground while playing (digging in a game is a game's feature; the engine offers the calls).
