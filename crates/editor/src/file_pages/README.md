# editor/src/file_pages

The File tab's own pages (the design's Screens 7, 8, 12 and 13, and the approved additions of round 2), drawn in the page area beside the File column (`shell/file_page.rs`). Solis starts on Recent projects ([decision 0105](../../../../docs/decisions/0105-solis-starts-on-the-file-page.md)). Nothing here reads a file: the app hands over the projects, the build and the release notes, and the pages ask for changes with `WorkbenchAction`s.

| File | What it holds |
| --- | --- |
| `mod.rs` | The pages' state (`FilePages`: the page in front, the projects' search and order, the card picked, the settings' sub-tabs and search), `KnownProject`, `ProgramFile`, the list's order and words (`shown`, `ago`, `BuildMark`), and `file_page_body`, which draws the page in front. |
| `page_parts.rs` | What the pages share: `page_head`, `plain`, `sentence`, `button_row`, `section`, and `with_side` (settings beside a column of cards). |
| `recent.rs` | Recent projects: the heading's search, Sort by, Open… ▾ and New project; the cards in rows and what the four arrows and Enter do to them; the template strip; the Rename… question. |
| `project_card.rs` | One project's card: its letter, name and pin, folder, when and by which build it was last opened, Locate… and Remove for one that's gone, and its right-click menu. |
| `learn.rs`, `updates_page.rs`, `about.rs` | Learn (the examples, the first hour, the user guide, the sample area), Updates (the notice card, this copy, the newest build, the releases page and the changes, what's new) and About (the name, version and build, the licences). |
| `settings.rs` | What Preferences and Project settings hold: their sub-tabs (`Tab`), every setting with its name and line (`Setting`), each sub-tab's sections with its unfinished rows, and what the search finds. |
| `settings_page.rs` | The page both share: the heading, the search, the sub-tabs, the rows (with the changed dot and Reset), the column beside them. |
| `preferences.rs` | Preferences' controls (General, Appearance, Viewport), Reset to defaults, the card of an unfinished part and the small picture of the editor; choosing the look. |
| `project_settings.rs` | Project settings' controls (General, 2D), what has changed from the standard, and the 2D settings themselves (`set_pixels`, `change_pixels`, `fits`). |
| `build.rs` | Project settings › Build: Export for playtesting's rows and the card about the last export (`export_dialog.rs` holds what it asks and the playtest file); and the Build tab's page (`Workbench::build_page`), the same rows in the File pages' pattern with the unfinished targets beside them. |
| `input_page.rs` | Project settings › Input: its state (`InputPage`), the tools over the table and Add action, binding by the next key pressed, Backspace clearing a picked binding, and the clashes to mark. |
| `input_table.rs`, `input_card.rs` | Input's table of actions, and the picked action's card with Device settings and the ready-made actions' links. |
| `shortcuts.rs` | Preferences › Shortcuts: today's keys as a table to read, filtered and searched. |

Tests: `tests.rs` (Recent projects, Learn, Updates, About, starting), `settings_tests.rs` (Preferences), `input_page_tests.rs` (when Input saves); the 2D tab's are in `two_d_tests.rs`, the export's in `export_dialog_tests.rs`.
