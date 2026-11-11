# editor/src/tokens

The parts of the design's tokens that aren't colours. The colours, and what tokens are, are in [`../tokens.rs`](../tokens.rs); the design they come from is [`docs/design/g-layout/`](../../../../docs/design/g-layout/README.md).

| File | What it does |
| --- | --- |
| `size.rs` | Every size of the design, in points: the heights of the tab bar, ribbon, shelf bar and status bar, the sizes of buttons, fields, toggles, tiles and cards, paddings and gaps, and the corner radii (`radius`). Also `Density` (how tall rows, fields and shelf tabs are: Compact, Default, Comfortable) and `Scale` (the interface scale's five steps; any other number becomes the nearest). Nothing that draws writes one of these as a number of its own. |
| `text.rs` | The type styles: a size and a family for each (interface text, tabs, headers, rows, numbers, a File page's title and so on), the family names of the weights (`MEDIUM`, `SEMIBOLD`, `BOLD`, `MONO_BOLD`), and `caps`, which lays out words in capitals with their letters set apart. Which font file a family draws with is `../fonts.rs`. |

Tests for all three files are in [`../tokens_tests.rs`](../tokens_tests.rs): they read the specification and fail when a token and its table disagree.
