# editor/src/shell/ribbons

Each tab's ribbon as a small table of groups: what is on it, in what shape, and which command each control does. Nothing here draws: [`../ribbon.rs`](../ribbon.rs) lays a table out and [`../controls.rs`](../controls.rs) draws its working controls. A control the engine can't do yet is listed in its place as an unfinished part, so the one mechanism draws it and **Hide unfinished parts** takes it out (a group left with nothing goes too).

| File | Tab | What is real on it |
| --- | --- | --- |
| `mod.rs` | | `of(tab, editor)` gives a tab's table (Home's is the tile map's while HOME TOOLS · Tile map is in front); the `group!` macro. File has a page instead of a ribbon. |
| `home.rs` | Home | Edit (Undo, Redo), Create (Empty, Model, Sprite, Light, Camera, Tile map, More), the three snapping switches, Project (Save, Build). Lighting is unfinished. |
| `library.rs` | Library | Import. The rest of the Library's working controls stay on its panel's own bar until the Library's screen is rebuilt. |
| `tile_map.rs` | HOME TOOLS · Tile map | Paint (the five tools), what is in hand (the terrain or copy, Layer, Exact), Show (Map as, Values, Chunks, Walls), Copy (Flip X, Flip Y, Turn, Put away). |
| `tileset.rs` | LIBRARY TOOLS · Tileset | Terrain (Add, Remove), Kind (the list, Save its template…), Picture (the tile size, Zoom). No Play group. |
| `picture.rs` | LIBRARY TOOLS · Picture | Crop (Every tile, Delete), Grid (its size, Zoom). No Play group. |
| `animate.rs` | Animate | By what is open: a clips file's Clip (New, Copy, Delete), Frames (Take out) and playback (Plays, Speed, Direction); an animator's State (Add state, Start here, Delete) and Preview (Start again); then the design's sequencer controls, all unfinished. |
| `ui.rs` | UI | Screen (the size, Text %, Safe area, Snap), Code (Open as code). |
| `audio.rs` | Audio | Mixer (Bus), Bus (Add rule, Remove), Clear solo, Mute the game, Heard at once; Send, Snapshot, Meters, Listen from and Effects unfinished. |
| `debug.rs` | Debug | The Profiler's CPU and GPU switches, how many frames it keeps, and the Session group (Load save, Play and record inputs, Replay a recording…). |
| `build.rs` | Build | Playtest (Export, Open folder); Targets and Full export unfinished. |
| `shading.rs`, `vfx.rs` | Shading, VFX | Whole unfinished screens. |
| `model.rs` | LIBRARY TOOLS · Model | The model viewer, a whole unfinished screen but for Use › Place. |

The Play group is not in these tables: it is the same on every tab ([`../play_group.rs`](../play_group.rs)).
