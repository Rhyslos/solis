# editor/src/shell/ribbons

Each tab's ribbon as a small table of groups: what is on it, in what shape, and which command each control does. Nothing here draws: [`../ribbon.rs`](../ribbon.rs) lays a table out and [`../controls.rs`](../controls.rs) draws its working controls. A control the engine can't do yet is listed in its place as an unfinished part, so the one mechanism draws it and **Hide unfinished parts** takes it out (a group left with nothing goes too).

| File | Tab | What is real on it |
| --- | --- | --- |
| `mod.rs` | | `of(tab, editor)` gives a tab's table; the `group!` macro. File has a page instead of a ribbon; UI has only the Play group until its screen is designed. |
| `home.rs` | Home | Edit (Undo, Redo), Create (Empty, Model, Sprite, Light, Camera, Tile map, More), the three snapping switches, Project (Save, Build). Lighting is unfinished. |
| `library.rs` | Library | Import. The rest of the Library's working controls stay on its panel's own bar until the Library's screen is rebuilt. |
| `animate.rs` | Animate | Nothing yet: the design's sequencer controls, all unfinished. |
| `audio.rs` | Audio | Nothing yet: the design's mixer controls the engine has nothing behind. |
| `debug.rs` | Debug | The Profiler's CPU and GPU switches, how many frames it keeps, and the Session group (Load save, Play and record inputs, Replay a recording…). |
| `build.rs` | Build | Export (for playtesting). |
| `shading.rs`, `vfx.rs` | Shading, VFX | Whole unfinished screens. |

The Play group is not in these tables: it is the same on every tab ([`../play_group.rs`](../play_group.rs)).
