# docs/design/g-layout

The project lead's design for the editor's interface, the "G layout": a tab bar, a ribbon, editor groups, a shelf and a status bar, in one dark neutral theme with five accent colours. **This folder is the source of truth for how the editor looks** ([decision 0099](../../decisions/0099-the-g-layout.md)); the plan that builds it is [`docs/plans/shared-1-ui-remake.md`](../../plans/shared-1-ui-remake.md).

| File | What it is |
| --- | --- |
| `specification.md` | The specification, 1,192 lines: the tokens (colours, type, sizes, states), then each screen panel by panel, then what crosses screens and the design's own open questions. Its line numbers match the project lead's document, so "spec 140-146" in a plan or a comment means lines 140 to 146 of this file. Two comparisons to another product were reworded (CONTRIBUTING.md, rule 20); nothing else differs. |
| `01-home-level-editor.png` | Screen 1: Home, the level editor |
| `02-home-level-design-placing.png` | Screen 2: Home, level design (placing) |
| `03-library.png` | Screen 3: Library |
| `04-shading-material.png` | Screen 4: Shading, the material editor |
| `05-animate-sequencer.png` | Screen 5: Animate, the sequencer |
| `06-debug-profiler-and-console.png` | Screen 6: Debug, the profiler and console |
| `07-file-recent-projects.png` | Screen 7: File, Recent projects |
| `08-file-preferences.png` | Screen 8: File, Preferences |
| `09-library-model-viewer.png` | Screen 9: Library, the model viewer |
| `10-audio-mixer.png` | Screen 10: Audio, the mixer |
| `11-vfx-effect-editor.png` | Screen 11: VFX, the effect editor |
| `12-file-project-settings.png` | Screen 12: File, Project settings |
| `13-project-settings-input.png` | Screen 13: File, Project settings, Input |

## Additions the project lead has approved

`additions/` holds the parts the design doesn't draw, once they are mocked up and approved: one folder per round of mock-ups, each with a README naming every picture, what it adds and the decision it settles. They count as the design.

| Folder | Approved | What it adds |
| --- | --- | --- |
| [`additions/round-1/`](additions/round-1/README.md) | 2026-11-11 | A placeholder's look and the hidden state, the tab bar for Solis's tabs, the collapsed ribbon, Home's ribbon stopped, playing and paused, the Game tab, the Debug ribbon's Session group, the document tab's menus, the Console on the shelf, the dialog card, and the sheets of new icons |

## How the pictures are used

- Each picture is a screen drawn at 1440 × 900 and saved at twice that size (2880 pixels wide), so a point at (x, y) on the screen is the pixel at (2x, 2y). Screens 9 and 12 are a little taller than 900.
- The editor's picture tests sample colours from them (`crates/editor/src/picture_tests.rs`): a named point on a flat surface in the picture must be the colour the editor's tokens give that surface. So the pictures are **never edited, resized or re-saved with changed pixels**. (They were re-packed without loss when copied in, which makes the files smaller and leaves every pixel as it was.)
- The names, the crane and the harbour in them are sample content, not features.

## What isn't here

- **The mock-up's HTML and the PDF copies** stay with the project lead: they are large, and the specification and pictures say the same. Where the specification leaves a value out and the HTML had one, the value is written down in the plan's inventory ([`shared-1-ui-remake-inventory.md`](../../plans/shared-1-ui-remake-inventory.md), part d) and in `crates/editor/src/tokens.rs`.
- **Parts the design doesn't draw that are still to be mocked up** (Solis's 2D editors, the Library's chips for Solis's kinds of file, the pages of File the design names but leaves undrawn, and more). Each is drawn in the design's own language on its screens and approved by the project lead before it is built; the plan lists them and records each approval. Once approved, a round's pictures join `additions/`.
