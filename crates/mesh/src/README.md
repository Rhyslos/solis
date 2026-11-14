# mesh/src

Polygon meshes, how they connect, the operations on them and the brushes' shapes. See the crate's [README](../README.md).

**Start reading at** `polygon.rs`, then `topology.rs` and `shapes.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `polygon.rs` | The mesh: points and n-sided faces by stable id, hard edges, deleting, rebuilding from saved slots. Its point storage is in [`polygon/`](polygon/README.md). Tests in `polygon_tests.rs`. | `Mesh`, `VertexId`, `FaceId`, `EdgeKey`, `MeshError` |
| `topology.rs` | How points, edges and faces connect, worked out once per set of faces. Tests in `topology_tests.rs`. | `Topology`, `EdgeUse` |
| `attributes.rs` | Named values on points, edges, faces or corners, stored by id. Tests in `attributes_tests.rs`. | `Domain`, `Value`, `AttributeElement`, `HARD` |
| `builder.rs` | How operations make new points and faces so their values come along. Tests in `builder_tests.rs`. | `Builder`, `PointFrom`, `CornerFrom` |
| `material.rs` | Each face's material slot, a face layer. | `MATERIAL`, `slot_of`, `set_material` |
| `edit.rs` | Operations in one place: moving points, and the extrudes, inset, splits, merges and materials re-exported. | `Outcome`, `move_points`, `transform_points` |
| `extrude.rs` | Extruding faces (along their facing, straight, shaped), insetting them, and extruding border edges. Tests in `extrude_tests.rs` and `extrude_shaped_tests.rs`. | `extrude_polygons`, `extrude_shaped`, `inset_polygons`, `extrude_edges`, `Grouping` |
| `split.rs` | Cutting a face in two between two points, a point in the middle of edges, faces cut across their middle along the longer side. Tests in `split_tests.rs`. | `split_face_between`, `split_edges`, `split_faces_across` |
| `merge.rs` | Deleting parts and closing the gap: touching faces merged into one, an edge's or a point's faces merged. Tests in `merge_tests.rs`. | `merge_faces`, `dissolve_edges`, `dissolve_points` |
| `intact.rs` | Whether a brush stays whole after points move: no face folded or flattened, no passing through. Tests in `intact_tests.rs`. | `check_moved`, `Broken` |
| `whole.rs` | Whether a brush is whole after faces are added, removed or rewired: closed, facing out, room inside, four faces or more. Tests in `whole_tests.rs`. | `check_whole`, `room_inside` |
| `repair.rs` | Mending a mesh read from a file: faces turned the right way, a hole with one flat border filled. Tests in `repair_tests.rs`. | `repair`, `why_not_whole` |
| `triangulate.rs` | Cutting any face into triangles, the same way every time. Tests in `triangulate_tests.rs`. | `face_triangles`, `triangulate` |
| `flat.rs` | A mesh as triangles: drawn flat (each face's own facing and slot), or over shared points for physics. | `FlatTriangles`, `flat_triangles`, `solid_triangles` |
| `shading.rs` | Smooth or sharp shading by hard edges or by angle. | `shaded_triangles`, `corner_shading`, `Hardness` |
| `check.rs` | A mesh's health: open borders, flipped faces, faces with no area. | `MeshReport` |
| `bounds.rs` | The upright box around points. | `Bounds` |
| `parallel.rs` | Heavy work split across cores for big meshes. | `in_parallel` |
| `shapes.rs` | The brushes' shapes in metres, filling their box, closed and facing out, settings within limits. Tests in `shapes_tests.rs`. | `Shape`, `ShapeSettings` |
| `primitives.rs` | Plain primitives (cube, plane, cylinder, sphere, disc, tube, cone) in the mesh's own units; the round shapes are built on them. Their builders are in [`primitives/`](primitives/README.md). | `Primitive`, `PrimitiveKind` |
| `samples.rs` | A heavy bumpy ball for the tests. | |
