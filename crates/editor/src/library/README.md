# editor/src/library

The Library ([decision 0021](../../../../docs/decisions/0021-asset-database-and-models.md); the rethink's board 04, [decision 0118](../../../../docs/decisions/0118-the-editor-rethink.md)): one browser of the project's own folders and files, drawn the same on Home's bottom panel and on the Library tab, and Home's folder trees. The app keeps the list up to date from the asset database and makes the thumbnails; the Library only shows them and returns `LibraryRequest`s for the app to carry out.

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | The open project (`ProjectInfo`), each file as the editor knows it (`LibraryItem`), the files with their names lowered once for searching (`LibraryFiles`), what the Library shows and what's picked (`LibraryState`: the folder, back, the search, the kind filter, several picked, the name being typed, the trees), what it asks for (`LibraryRequest`), and the entry point, `library`. Start here. | `library`, `LibraryState`, `LibraryRequest`, `LibraryDrag`, `Files`, `moves_into` |
| `browser.rs` | The browser: the folder tree beside back, up, the breadcrumb and the counts, then the files; its tools (search, kind filter, grid or list), which sit in its own header on the Library tab and in the bar above it elsewhere. | `browser`, `tools`, `take_moves` |
| `shown.rs` | What the browser shows: the open folder's folders then files, or every match of the search, through the kind filter; the engine's `solis/` folder left out. | `shown`, `Entry`, `folders_in` |
| `tiles.rs` | The entries as tiles or a list, each thumbnail per kind (a prefab's with the blue prefab mark, and **New** just after it's made, R5), and what a click, a double-click (the picture opens, the name renames), a drag, a drop and a right-click on one ask for; the Code panel's list. | `grid`, `list`, `item_actions`, `paint_thumbnail` |
| `menus.rs` | The right-click menus: on empty space (New in the folder, New prefab among them from R5, Import, Paste, Show in Explorer), on a file or folder (Open, its kind's own, Rename, Duplicate, Delete, Show in Explorer), on several picked. | `space_menu`, `entry_menu` |
| `rename.rs` | Renaming in place: the box, `Enter`, `Esc` and clicking away, and why a name won't do. | `name_box`, `checked_name`, `rename_to` |
| `tree.rs` | The browser's folder tree, and the row every tree draws. | `folder_tree`, `visible_folders`, `draw_row` |
| `trees.rs` | Home's left slot as folder trees, several side by side, each with its own folder, the edges between them dragged. | `folder_trees`, `Trees`, `LeftShows` |
| `kinds.rs` | The kinds of file the Library knows, what each can do, and the groups of kinds its filter shows. | `LibraryKind`, `Filter` |
| `tests.rs` | The parts on their own: what's shown, picking, back and up, names, moves, the trees. | |

**Read next:** the folder's parent, [`../README.md`](../README.md) (the `library/` row).
