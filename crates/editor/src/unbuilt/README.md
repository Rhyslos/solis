# editor/src/unbuilt

Unfinished parts: the one way the editor shows something the project lead's design has and Solis doesn't do yet ([decision 0101](../../../../docs/decisions/0101-unfinished-parts-as-placeholders.md); the plan's section 3, `docs/plans/shared-1-ui-remake.md`). The module's own file is [`../unbuilt.rs`](../unbuilt.rs): the phases, the table's line type, the filter, what was drawn this frame, and what the status bar says after a click.

| File | What it does |
| --- | --- |
| `table.rs` | The table: one line for each unfinished part, with its name in code (`UnbuiltId`), its key in the plan's inventory, the words shown on it, the roadmap phase that builds it and one plain sentence about what it will do. A test holds it against `docs/plans/shared-1-ui-remake-inventory.md`, so the document and the code can't drift. |
| `draw.rs` | One function for each shape the frame has, each taking the part's id and nothing about looks: `ribbon_big`, `ribbon_row`, `tool`, `tab`, `shelf_tab`, `menu_item`, `sub_tab`, `setting_row`, `field_row`, `chip`, `icon_button`, `status_item`, `link`, `card`, `column` and `page`. The look is the one the project lead approved (`docs/design/g-layout/additions/round-1/`): in place and in shape, words in the lowest readable grey, the icon fainter, a 5 px hollow square mark, no highlight under the pointer. Pointing says what it will do and when; a click only says, in the status bar, that it isn't built. |
| `sheet.rs` | Test only: the placeholder sheet, a small stand-in editor with unfinished parts among working ones in every shape, each list going through the filter. |

Tests are in [`../unbuilt_tests.rs`](../unbuilt_tests.rs) (the table is sound and matches the inventory; every shape draws its part, named and described for screen readers; a click says so for three seconds; hidden, nothing registered is drawn and what is left closes up; the look is the approved one) and in [`../source_rules_tests.rs`](../source_rules_tests.rs) (nothing outside this folder may say that something isn't built, or hold an unfinished part's id except to draw or list it).

## Using it

- **To show an unfinished part:** put it in the list it belongs to as `Entry::Unbuilt(UnbuiltId::..)` beside the working `Entry::Built(..)` ones, pass the list through `unbuilt::filter` (or `filter_groups`) before laying anything out, and draw each `Unbuilt` entry with the function for its shape. Never draw one that the filter didn't hand back: with **Hide unfinished parts** on it must leave no gap.
- **When a part is built:** delete its line from `table.rs`. The compiler points at every place that still draws it. Change its row in the plan's inventory from PLACEHOLDER to EXISTS in the same pull request (the test asks).
- **To add an unfinished part:** add its row to the inventory first, then its line here, in the same order.
