# editor/src/library

The Library panel ([decision 0021](../../../../docs/decisions/0021-asset-database-and-models.md)): the project's files as tiles or a list, with a folder tree, the kinds' chips and a search box. The app keeps the list up to date from the asset database and makes the thumbnails; the panel only shows them and returns `LibraryRequest`s for the app to carry out.

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | The panel itself: the open project (`ProjectInfo`), each file as the editor knows it (`LibraryItem`), the folder tree, tiles and list, dragging to place or move, double-clicking to open, the right-click menu and what each of its choices does (`act_on`), and the thumbnails drawn. Start here. | `library`, `LibraryItem`, `LibraryState`, `LibraryRequest`, `act_on` |
| `kinds.rs` | The kinds of file the Library knows, what each can do, and the groups of kinds its chips show. | `LibraryKind`, `Filter` |

`mod.rs` is far past the size guide; it is left as it is on purpose, since the editor's interface is being remade (review finding DEBT-08).

**Read next:** the folder's parent, [`../README.md`](../README.md) (the `library/` row).
