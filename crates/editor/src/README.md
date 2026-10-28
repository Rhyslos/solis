# editor/src

The editor's interface, drawn with egui and egui_dock: the top bar, the dockable panels, the viewport and its tools, play mode, and the theme. It never touches the window, the GPU or files: it returns `WorkbenchAction`s for the app to carry out, and draws 3D views through a `ViewportRenderer` the app hands in.

Note: the `symbols_have_glyphs` test in `theme.rs` reads every file in this folder, this README included, and checks that each non-ASCII character can be drawn by the editor's fonts. Keep this file plain ASCII.

**Start reading at** `workbench.rs` (the whole window), then `document.rs` and `commands.rs` (how the area changes).

Tests: files ending `_tests.rs` hold the tests of the file with the same start (`play_tests.rs` tests `play.rs`). Tests of the whole window are drawn with egui_kittest, without a GPU; `test_support.rs` has their shared helpers.

## The window and its state

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `workbench.rs` | The whole editor window: its state between frames, and `show`, which draws a frame and returns the actions. | `Workbench`, `WorkbenchAction`, `AppInfo` |
| `workbench_library.rs`, `workbench_tools.rs` | The workbench's side of the Library (file actions, placing what's dropped) and of the viewport's tools (gizmo, outline, their keys). | |
| `document.rs` | The open area as the editor holds it: the area, the component registry, the selection and undo history. It only changes through `Document::edit`. | `Document`, `Selection` |
| `commands.rs`, `commands_many.rs` | Everything the editor does to the area, each turned into reversible edits and recorded as one undo step; `commands_many.rs` covers several entities at once and copy and paste of values. | |
| `menu_items.rs` | Menu items shared by the top bar and the panels' own menu rows. | `MenuCommand` |
| `top_bar.rs` | The top bar: menus, document tabs, Play and Build. | |
| `panel.rs`, `panels.rs`, `layout.rs` | The list of dockable panels, drawing each one's contents, and the standard layout with checks on a saved layout. | `Panel`, `layout::standard` |
| `theme.rs`, `icons.rs` | The editor's look (greys, accent colour, fonts, text contrast checks) and its icons, drawn in code. | `Accent`, `Icon` |
| `splash.rs`, `updates.rs` | The splash screen while the program starts, and what the editor shows about updates (decisions 0043 and 0045). | `SplashView`, `UpdateNotice` |

## Panels

| File | What it does | Main items |
| --- | --- | --- |
| `area_panel.rs` | The Area panel: entities as a tree, with search, folding, hiding and locking. | |
| `properties.rs`, `inspector.rs` | The Properties panel, and fields drawn from any component's schema, so built-in, Lust and Lua components show the same way. | |
| `library.rs` | The Library panel: the project's files as thumbnails or a list, with a folder tree and search; drag to place or move. | `LibraryItem`, `ProjectInfo` |
| `game_sound.rs` | Game code's sound requests on their way out after each tick, with the tick and seed they're picked by; the listener and entities' places between ticks for the app; the `AudioSource`s turned on, with their places (S5). | `GameSound`, `SoundMode`, `Workbench::take_game_sounds`, `Workbench::audio_sources` |
| `mixer_view.rs` | The Mixer panel (a strip per bus: name, meter, fader, volume, mute and solo), a bus in Properties (inside, volume, ducking rules), and the Game panel's mute button; edits saved once let go. | `MixerState`, `Workbench::set_mixer`, `Workbench::set_bus_meters` |
| `sound_event_view.rs` | A sound event in Properties: its takes (sound files dropped on the list are added), how each play varies, where it's heard, and Play; saved once the mouse is let go. | `event_properties` |
| `sound_view.rs` | Sound files in the Library and Properties: waveform, details, Play and Stop. | `SoundInfo`, `SoundState`, `EventState`, `Preview` |
| `console.rs`, `console_commands.rs` | The Console panel (messages, filters, search) and the commands typed under it. | `LogLine`, `LogLevel` |
| `problems.rs` | The Problems panel: mistakes in game code and in the open area. | `Problem`, `ProblemKind` |
| `profiler.rs` | The Profiler panel: where each frame's time goes, on the CPU (game code, the world, preparing, drawing, sound, the editor) and the GPU. | `Profiler`, `Stage` |
| `game_panel.rs` | The Game panel: the running game through the area's first camera. | |
| `input_window.rs` | Edit > Input actions: the project's actions and what sets each off (decision 0037). | |
| `settings_window.rs` | Edit > Settings…: the editor's settings, a page at a time, with a search box (decision 0061). | |

## The viewport

| File | What it does | Main items |
| --- | --- | --- |
| `viewport.rs` | The viewport panel: the 3D image, its corner controls, and several views (flat views, maximising). | `Viewport`, `ViewportRenderer`, `Tool`, `Views` |
| `area_view.rs` | What the viewport draws, worked out from the area each frame. | `AreaView`, `ViewModel`, `ViewLight` |
| `camera.rs` | The viewport camera and how it flies; `Camera::fly` does the maths so tests can fly it without a window (decision 0009). | `Camera`, `FlyInput`, `ViewKind` |
| `look.rs` | How models are shaded: standard, stylized or unlit. | `Look`, `LookKind` |
| `models.rs` | What the editor knows about imported models and the lights their files bring. | `ModelInfo`, `NodeInfo`, `FileLight` |
| `viewport_tools.rs` | Picking with a click or a box, dragging the gizmo, and what's drawn over the image for them (decision 0025). | `PickAnswer`, `Outline` |
| `picking.rs` | What a click or a box selects. | `PickMode`, `after_click`, `after_box` |
| `gizmo.rs` | The move, rotate and scale gizmo: its handles, which is under the mouse, and what dragging one does. Tests in [`gizmo/`](gizmo/README.md). | `Gizmo`, `GizmoTool`, `Handle` |
| `transforming.rs` | Each selected entity's new transform during a gizmo drag, with snapping. | `Targets`, `Snapping` |
| `screen.rs` | Between the screen and the world: the ray through a point, where a world point appears, how big a pixel is at a depth. | `Ray` |

## Playing

| File | What it does | Main items |
| --- | --- | --- |
| `play.rs` | Play mode: the area running as a game beside the editor, built from a copy of the document. | `PlayState` |
| `game_code.rs` | Game code in the editor: its components in the registry, and the program running against the game's world (decision 0028). | |
| `play_input.rs` | The game's input in the editor: clicking the Game panel gives the game the mouse and keys; recording and replaying input. | |
| `save_game.rs` | Save games: the whole running game, so loading carries on exactly (decision 0038). | |
| `invariant_tests.rs`, `kit_tests.rs`, `scale_levels_tests.rs` | Random games checked against rules that must always hold; the ready-made components played with input; and the scale levels timed (decision 0030). | |

**Read next:** the crate's [README](../README.md), and [ARCHITECTURE.md](../../../ARCHITECTURE.md), "The area".
