# editor/src/shell

The frame round the editor, as the project lead's design has it ([decision 0099](../../../../docs/decisions/0099-the-g-layout.md); the design is `docs/design/g-layout/`, the plan `docs/plans/shared-1-ui-remake.md`). So far it is the top: the tab bar, the ribbon and the File page. The middle (editor groups, the shelf, the status bar) joins it in the next milestone; until then the docked panels are drawn under the ribbon.

| File | What it does |
| --- | --- |
| `mod.rs` | `RibbonTab` (the ten tabs; which arrangement of panels each shows while the panels are docked) and `Shell`, what the frame remembers between frames: the tab in front, whether the ribbon is collapsed or dropped. It is the editor's one field for all of it (`Workbench::shell`). `Workbench::top` draws the tab bar, then the ribbon, the File page or an unbuilt screen's page. |
| `commands.rs`, `command_words.rs` | `ShellCommand`: the one table of what the editor can do by name. Each command has its name (a switch is named for what choosing it does now), where it lives, its shortcut (`keys`: the one table of shortcuts), whether it can be done now and why not, and what it does. The ribbon's buttons, the File page's items and the search all run these, so a thing has one name and one answer wherever it is asked for. |
| `tab_bar.rs` | The tab bar: the logo in its light tile (the app's picture, never drawn here), the tabs (through the unfinished parts' filter, so a hidden tab leaves no gap), the tabs that come with an open file, the search box with its key, the project and area, the chevron. |
| `ribbon.rs` | The ribbon's table types (`Group`, `Slot`, `Shape`, `Control`), the makers the tables use (`big`, `row`, `switch`, `big_list`, `row_list`, `fact`, `unbuilt_at`), and the layout: every width is known before anything is drawn, the groups go left to right, the Play group is pushed to the right edge, and the band scrolls sideways when it doesn't fit (the wheel over it scrolls it). Also the band itself: in its own strip, or dropped over the center while collapsed. |
| `controls.rs` | How a working control is drawn: the large button and the one light one, the small row, words alone, and the list a control opens from its small arrow or a right-click ([decision 0103](../../../../docs/decisions/0103-todays-keys-stay.md)). |
| [`ribbons/`](ribbons/README.md) | One small table per tab. |
| `play_group.rs` | The Play group: Play, or Stop with Pause and Step while the game runs, and Simulate (unfinished). |
| `file_page.rs` | The File tab: the column of items (through the filter) and the page area, which shows the launcher's page in front. |
| `create.rs` | The two kinds of entity the Create group makes that the editor had no one-step way to make: a sprite and a reverb zone. |
| `tests.rs`, `picture_tests.rs` | The commands' table; the tabs and what each shows; the ribbon's measured layout against the tokens; scrolling, collapsing, lists, the Play group and the File page, used as a person uses them; and the top's colours sampled from a frame drawn for real. `picture_tests.rs` also has an ignored test that writes the top on each screen to a folder, to look at beside the design. |

[`../shell_reach_tests.rs`](../shell_reach_tests.rs) holds the rule that nothing became unreachable: the old menu bar, item by item, each still a command that can be done, listed by the search, and drawn where its row says.

## Adding to it

- **A new thing the editor can do:** add it to `ShellCommand` (and to `ALL`), give it its words in `command_words.rs` and its `why_not` and `run` in `commands.rs`. It is in the search at once. Put it on a ribbon by adding a line to that tab's table.
- **A control the engine can't do yet:** list it in the table as `unbuilt_at(shape, Entry::Unbuilt(UnbuiltId::..))`. Never draw one by hand ([`../unbuilt/`](../unbuilt/README.md)).
- **A new key or mouse action** needs the project lead's answer first (rule 17 of CONTRIBUTING.md).
