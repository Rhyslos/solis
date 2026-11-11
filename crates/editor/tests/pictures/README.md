# editor/tests/pictures

Solis's own approved pictures of the editor: frames drawn by the picture tests (`crates/editor/src/picture_tests.rs`) that were looked at beside the project lead's design ([`docs/design/g-layout/`](../../../../docs/design/g-layout/README.md)) and approved. Each test draws its frame again and fails if it differs from the picture here by more than a graphics driver's rounding. They catch accidents; they don't prove likeness to the design, which the sampled colours and the measured layout do.

| File | What it shows |
| --- | --- |
| `token-sheet.png` | Every colour of the design as a swatch, in the order of `tokens::sheet()`. |
| `editor.png` | The editor at 1440 x 900 with the sample project open (`tests/fixtures/ui-sample/`: its area in the Area panel, its lamp selected in Properties, its files in the Library) and no 3D view: today's layout (top bar, docked panels, status bar) in the design's look. Each milestone that replaces part of the layout replaces this picture, so the change is seen on real content. |
| `launcher.png` | The launcher at 1100 x 700 with no projects. |
| `icon-sheet.png` | Every icon of `crates/editor/src/icons/` at 12, 15 and 18 px on the ribbon's grey: the design's own 58 first, then the 69 made for what the design lacks, so a new one can be looked at beside a design icon of its weight. |
| `widget-sheet.png` | The widget sheet (`crates/editor/src/widgets/sheet.rs`): every control of the kit in each of its states, settings on a File page's grey at the left, a shelf column's cards, chips and tiles at the right. |
| `placeholder-sheet.png` | The placeholder sheet (`crates/editor/src/unbuilt/sheet.rs`): unfinished parts among working ones in every shape the frame has (ribbon buttons and rows, tabs, tools, a File menu item, a setting row, a chip, a link, a field, a card, a shelf column, a status item and a whole page's card), in the look the project lead approved. |

**Keep them few and small:** the repository is public and cloned often. A new picture is added only for a whole screen, or for a sheet that stands for many small things at once (the icons, the kit's controls), once, when its milestone is approved.

**To approve a changed picture:** run the test, look at what it drew (the failure says where the picture is) beside the design's screen, and if it is right run the tests once with `SOLIS_UPDATE_PICTURES=1`, on Linux with Mesa's software driver (`WGPU_BACKEND=gl xvfb-run -a cargo test -p editor picture`), which is what the pictures here were drawn with. Other systems compare sampled colours only.
