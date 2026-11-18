# editor/src/file_pages

The File tab's own pages (the design's Screens 8, 12 and 13, and the approved additions of round 2), drawn in the page level beside the File column (`shell/file_page.rs`): what belongs to the open project and the editor. The projects, Learn and Updates moved into the launcher ([decision 0118](../../../../docs/decisions/0118-the-editor-rethink.md), [`../launcher/`](../launcher/README.md)), which File › Close project goes back to. Nothing here reads a file: the app hands over the build and the project's settings, and the pages ask for changes with `WorkbenchAction`s.

| File | What it holds |
| --- | --- |
| `mod.rs` | The pages' state (`FilePages`: the page in front, the settings' sub-tabs and search, the update notice put away), `ProgramFile`, `BuildMark` (a project's build beside this one's), the Rename… rule (`name_problem`), and `file_page_body`, which draws the page in front. |
| `page_parts.rs` | What the pages (and the launcher's) share: `page_head`, `plain`, `sentence`, `button_row`, `section`, and `with_side` (settings beside a column of cards). |
| `about.rs` | About (the name, version and build, the licences, Check for updates). |
| `settings.rs` | What Preferences and Project settings hold: their sub-tabs (`Tab`), every setting with its name and line (`Setting`), each sub-tab's sections with its unfinished rows, and what the search finds. |
| `settings_page.rs` | The page both share: the heading, the search, the sub-tabs, the rows (with the changed dot and Reset), the column beside them. |
| `preferences.rs` | Preferences' controls (General, Appearance, Layout's rows, Viewport), Reset to defaults, the card of an unfinished part and the small picture of the editor; choosing the look. |
| `project_settings.rs` | Project settings' controls (General, with the project's kind, 2D), what has changed from the standard, and the 2D settings themselves (`set_pixels`, `change_pixels`, `fits`). |
| `build.rs` | Project settings › Build: Export for playtesting's rows and the card about the last export (`export_dialog.rs` holds what it asks and the playtest file); and the Build tab's page (`Workbench::build_page`), the same rows in the File pages' pattern with the unfinished targets beside them. |
| `input_page.rs` | Project settings › Input: its state (`InputPage`), the tools over the table and Add action, binding by the next key pressed, Backspace clearing a picked binding, and the clashes to mark. |
| `input_table.rs`, `input_card.rs` | Input's table of actions, and the picked action's card with Device settings and the ready-made actions' links. |
| `shortcuts.rs` | Preferences › Shortcuts: today's keys as a table to read, filtered and searched. |
| `layout_page.rs` | Preferences › Layout (decision 0118, board 06): the layouts as cards three to a row, the Preview box showing the one pointed at, the note with Undo after one is chosen, and the panels' sizes. |
| `layout_miniature.rs` | A layout drawn small, on its card and large in the Preview box with each part named. |

Tests: `tests.rs` (the File column, About, a project's build, renaming), `settings_tests.rs` (Preferences), `input_page_tests.rs` (when Input saves), `kind_tests.rs` (a 2D or 3D project's settings leave out the other kind's, on their sub-tabs and in the search; R6); the 2D tab's are in `two_d_tests.rs`, the export's in `export_dialog_tests.rs`.
