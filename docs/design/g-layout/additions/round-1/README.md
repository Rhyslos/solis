# docs/design/g-layout/additions/round-1

The first round of additions to the project lead's design: parts the G layout doesn't draw, mocked up in its own language on copies of its own screens, and **approved by the project lead on 2026-11-11** ("yeah the mockups look good"). They are part of the source of truth for how the editor looks ([decision 0099](../../../../decisions/0099-the-g-layout.md)), beside the thirteen screens one folder up; the plan that builds them is [`docs/plans/shared-1-ui-remake.md`](../../../../plans/shared-1-ui-remake.md), whose decisions are named below as D2, D4 and so on.

Each picture is a whole screen drawn at 1440 × 900 and saved at twice that size, like the design's own, so a point at (x, y) on the screen is the pixel at (2x, 2y). The icon sheets and the comparison sheet are as wide and as tall as they need. Only the approved, clean pictures are here: the copies with the new parts outlined, the close-ups, the two placeholder looks that weren't chosen and the play tint that was decided against stayed with the mock-ups' source.

| Picture | What it adds to the design | The decision it settles |
| --- | --- | --- |
| `01-placeholder-compare.png` | The three placeholder looks side by side, four cases each (a ribbon, a shelf bar, a setting row, a page). Kept because it records the choice: the project lead chose **look A**, the left column. | D2 |
| `01-placeholder-home-A.png` | Home with its unbuilt parts as placeholders: the Shading and VFX tabs, Volume, the Lighting group, Simulate, four toolbox tools, the Animation shelf tab, the shelf's transport and frame counter, the branch in the status bar. | D2 (look A) |
| `01-placeholder-prefs-A.png` | Preferences › Appearance with one unbuilt setting row (Code editor split) under real ones, two unbuilt sub-tabs and an unbuilt item in the File column. | D2, D11 |
| `01-placeholder-shading-A.png` | A whole unbuilt screen: its ribbon and shelf tabs as placeholders and one centred page card. | D2 (a whole screen) |
| `01-placeholder-hidden.png` | Home with **Hide unfinished parts** on: every placeholder gone and the groups closed up, with no gaps. | D2, the plan's section 3.3 |
| `02-tab-bar.png` | Solis's tabs (File, Home, Library, Shading, Animate, **UI**, VFX, Audio, Debug, Build), the search with its hint **Space**, the path, and Solis's logo in the design's light tile. | D4 (words), D5 (logo), D6 (the UI tab) |
| `03-ribbon-collapsed.png` | The ribbon collapsed to its tabs; the center is 66 px taller. | D10 |
| `03-ribbon-collapsed-dropped.png` | A tab clicked while collapsed: the ribbon dropped over the center. | D10 |
| `04-home-ribbon-stopped.png` | Home's ribbon for Solis: Edit, Create, Snapping, Lighting (unbuilt), Project, Play. Transform is the fifth toolbox button. | D4, Home's groups |
| `04-home-ribbon-playing.png` | While the game runs: Play has become **Stop**, with **Pause** and **Step** beside it. | The Play group |
| `04-home-ribbon-paused.png` | Paused: Pause is lit, Step can be pressed. | The Play group |
| `05-play-tint-b-playing.png` | The game running with **no tint**: nothing changes colour but Play turning into Stop. | D13 (no tint, as designed) |
| `06-game-tab.png` | The running game as a **Game** document tab in front of the area's tab, with its pills. | D14 |
| `07-debug-ribbon.png` | Debug's ribbon with the **Session** group (Load save; Play and record inputs; Replay a recording…). | The Debug ribbon |
| `07-debug-ribbon-hidden.png` | The same with **Hide unfinished parts** on: what is real today. | D2, D12 |
| `08-doc-tab-plus-menu.png` | The **+** at the end of the document tab strip, open: New area, Open area…. | Document tabs; D18 (menus) |
| `08-doc-tab-right-click-menu.png` | A right-click on the area's tab: Save, Save as…, Close, Close others, Show in the Library. | Document tabs; D18 (menus) |
| `09-console-shelf.png` | The Console in front on the shelf: level filters with counts, a search field, Clear and Copy on the shelf bar, the command line along the bottom. | The Console on the shelf |
| `10-dialog-unsaved.png` | The shared dialog card: Unsaved changes (Don't save, Cancel, Save). | The dialog card |
| `10-dialog-keep-changes.png` | The same card for the question after Stop: Keep, Undo all. | The dialog card |
| `11-icons-new-1.png`, `11-icons-new-2.png` | The 69 icons Solis needs that the design doesn't draw, each at 18 px and 12 px with its name. | The icon sheets |
| `11-icons-design.png` | The design's own 58 icons drawn the same way, to compare with. | The icon sheets |

## Reading them

- **Three pictures appear under more than one name**, because one screen answers several questions: `01-placeholder-home-A`, `02-tab-bar` and `04-home-ribbon-stopped` are the same picture, as are `04-home-ribbon-playing`, `05-play-tint-b-playing` and `06-game-tab`. Each name says what to look at.
- **The new icons are built with the design's line ends.** The sheets drew them with round ends; the design's own icons have square ends and sharp corners, and the project lead asked for one style, the design's. The editor's icon table (`crates/editor/src/icons/`) draws all of them that way, so at 12 to 18 px an icon in the editor differs from these sheets by under half a pixel at its line ends.
- **The pictures were drawn before every weight of the fonts was bundled**, so a tab in front, a setting's name and a page's title are a little lighter here than in the design's own pictures. The editor uses the design's weights.
- The names, the crane and the harbour in them are the design's sample content, not features.
- Like the design's pictures, these are **never edited, resized or re-saved with changed pixels**: the editor's picture tests sample colours from them (`crates/editor/src/picture_tests.rs` and the tests beside the placeholder sheet).
