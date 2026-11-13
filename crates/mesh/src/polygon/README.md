# mesh/src/polygon

Parts of the mesh (`../polygon.rs`) kept in files of their own.

| File | What it does | Main items |
| --- | --- | --- |
| `points.rs` | Point positions by id, in chunks shared between copies of a mesh until one copy changes them, each with the box around its points. | `Points` |
| `layers.rs` | The mesh's attribute layers: adding, reading and setting values on its points, edges, faces and corners. | `Mesh::add_attribute`, `Mesh::attribute`, `Mesh::set_attribute` |
