# mesh

**Layer 0 · Core**

Polygon meshes: the data structure brushes are made of, how its parts connect, the operations that change it, and the shapes a brush starts as ([decision 0111](../../docs/decisions/0111-tools-1-brushes-and-ground.md)).

## Purpose

- **The mesh** (`Mesh`): points joined into faces with any number of corners (n-gons), each point and face with an id that stays the same for its whole life and is never given to another, even after it's deleted. Copies share their parts until one changes, so copying a mesh (as undo does) is cheap.
- **How it connects** (`Topology`): which faces use an edge, which edges meet at a point, neighbours, loops and rings, worked out once per set of faces.
- **Named values on its parts** (`attributes`): layers of values per point, edge, face or corner, such as hard edges and each face's material slot (`material`). Operations carry them over to what they make (`Builder`).
- **Operations as plain functions** (`edit`): moving points, extruding and insetting faces, extruding border edges, setting materials. Each takes the mesh and ids, and works on the ids that are still there.
- **Cutting into triangles** (`triangulate`, `flat`): any face, bent or dented, cut the same way every time; `flat_triangles` gives a brush's triangles to draw flat, `solid_triangles` the same triangles over shared points for physics.
- **Checking** (`MeshReport`): open borders, faces facing the wrong way, edges shared by three faces, faces with no area.
- **Staying whole** (`intact::check_moved`): after some points move, whether any face that uses them would fold over itself, flatten to nothing, or pass through another face (Tools 1a, B3). Only the faces touching what moved are looked at, and passing through is found with a grid of boxes, so it costs the same in a big brush when a few parts move. A face left bent is fine.
- **Shapes** (`shapes`): Box, Ramp, Cylinder, Stairs, Arch, Sphere and Wedge, in metres, each filling the box its size gives, standing on the ground, closed and facing out, every setting within limits (`ShapeSettings::clamped`: 1 cm to 10 km a side, 3 to 64 sides, 1 to 64 steps). The plain primitives the round ones are built on are in `primitives` (in the mesh's own units).

## Must never

- Know about areas, entities, files, the GPU, physics or the editor.
- Change a mesh behind the caller's back: operations take the mesh and say what they did.
- Reuse a deleted point's or face's id.
- Crash or allocate without bound on awkward input (faces with no area, n-gons, open borders, edges shared by three faces): what reads a mesh from a file checks its counts before building it (`area::builtin::Brush`).

## Depends on

No other Solis crate. Outside libraries: `glam` (maths).

## Where it comes from

Copied from Aventail's mesh crate, the project lead's own 3D modelling program, whose owner lets Solis use and relicense it ([decision 0007](../../docs/decisions/0007-mit-licence-and-separate-from-aventail.md)), and trimmed to what brushes need (Tools 1a, B1). Left out: UVs and seams, unwrapping, baking checks, bevels, loop cuts, sliding, slits, grooves, welding and bridging, joining and cleaning up, symmetry, mirroring, bends and stretched extrudes, anchors, stable names, selections and parts. The tests of what was kept came with it.

## Status

Tools 1a, B1: the mesh, its topology, attribute layers and builder, triangulation and shading, the primitives, extrude and inset, and the brushes' shapes. B3: the check that a mesh stays whole after points move.
