# editor/src/icons

The editor's icons: small line drawings kept as data and drawn by Solis's own reader, so they stay sharp at any size and interface scale and take whatever colour they are given. There are no image files and no drawing library.

| File | What it does |
| --- | --- |
| `mod.rs` | `icon(painter, rect, Icon, colour)` draws one, in the design's line style: square line ends, sharp corners (a line that turns straight back on itself is cut there instead, as a corner that sharp would shoot out past the icon), and a line weight that follows the size drawn (`Weight::width`: 1.3 on the grid for the ribbon's 18 px icons, 1.5 for 12 to 15 px, lighter above 20 px). The table is read once, the first time an icon is drawn. |
| `paths.rs` | The table: `Icon`, with each icon's name, weight and parts as path text on a 16 x 16 grid (`lines` stroked, `fills` filled, `dashed`, `faint`). First the project lead's own 58 drawings, as the design's screens have them; then the 69 made for what the design lacks (approved on the icon sheets in `docs/design/g-layout/additions/round-1/`), then the brush list's 8, from its approved mock-up (`docs/design/tools-1/01-brush-list.png`). It is one long file because it is one table: a new icon is a new entry here and nothing else. |
| `reader.rs` | Reads path text (`M3 2.5h8l2.5 2.5v8.5h-10.5z`) into runs of straight lines: move, line, across, down, curve, smooth curve, arc and close, as places or as steps. Text that isn't a path is an error with its place, never a crash. |
| `reader_tests.rs` | The reader's tests: each command, numbers that touch, what isn't a path, and thousands of damaged paths (a fuzz test). |
| `tests.rs` | The table's tests: every path reads, every icon stays inside its square at 12, 15 and 18 px and paints something, filled parts are shapes a plain fill can draw, the weights are the design's, and the icon sheet (every icon at the three sizes) is an approved picture. |

**To add an icon:** draw it on the 16 x 16 grid in the design's style (lines about 1.5 wide, nothing closer than 1 to the edge), add its entry to `paths.rs`, run the tests, look at the icon sheet they draw beside its neighbours, and approve the new sheet (`crates/editor/tests/pictures/README.md` says how). A tool's or a kind's icon is then named where the tool or kind is (`Tool::icon`, `PaintTool::icon`, `CanvasTool::icon`, `Panel::icon`).
