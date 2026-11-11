# editor/tests/pictures

Solis's own approved pictures of the editor: frames drawn by the picture tests (`crates/editor/src/picture_tests.rs`) that were looked at beside the project lead's design ([`docs/design/g-layout/`](../../../../docs/design/g-layout/README.md)) and approved. Each test draws its frame again and fails if it differs from the picture here by more than a graphics driver's rounding. They catch accidents; they don't prove likeness to the design, which the sampled colours and the measured layout do.

| File | What it shows |
| --- | --- |
| `token-sheet.png` | Every colour of the design as a swatch, in the order of `tokens::sheet()`. |
| `editor.png` | The editor at 1440 x 900 with a small area open and no 3D view: today's layout (top bar, docked panels, status bar) in the design's look. Each milestone that replaces part of the layout replaces this picture, so the change is seen. |
| `launcher.png` | The launcher at 1100 x 700 with no projects. |

**Keep them few and small:** the repository is public and cloned often. A new picture is added only for a whole screen, once, when its milestone is approved.

**To approve a changed picture:** run the test, look at what it drew (the failure says where the picture is) beside the design's screen, and if it is right run the tests once with `SOLIS_UPDATE_PICTURES=1`, on Linux with Mesa's software driver (`WGPU_BACKEND=gl xvfb-run -a cargo test -p editor picture`), which is what the pictures here were drawn with. Other systems compare sampled colours only.
