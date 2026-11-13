# world/src

The game's fast storage while it runs (decision 0024). The area (`area::Area`) is the document the editor edits: easy to change, undo and save. A `World` is built from it when Play is pressed: the same entities in the `hecs` library, with the built-in components as typed data, physics, and projectiles, made to step quickly.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | The `World` and the components every entity has, and finding things in it: entities by name or ID, their components and values. Start here; what the world does is in the files below, each an `impl World` of its own. | `World`, `Clock`, `Id`, `Name`, `Parent`, `Global`, `Values` |
| `building.rs` | Building the world from an area, and adding, removing and updating entities in it while the game plays; each entity's components as the world keeps them (`components_of`). | `World::from_area`, `World::rebuild`, `World::update` |
| `clock.rs` | The game's clock in whole ticks, its speed, how far between two ticks, the time of day (decision 0092) and the random seed. | `World::step`, `World::set_speed`, `World::set_time_of_day` |
| `placing.rs` | Where everything is: places worked out from parents', set and changed while playing, settled after physics, and read now or between two ticks for drawing. | `World::set_transform`, `World::settle`, `World::placed_between` |
| `physics_3d.rs` | 3D physics in the running game: bodies joining as entities do (a Solid brush with its own triangles), game code's pushes and moves, a step a tick, and its state for saves. | `World::step_physics`, `World::physics_states` |
| `brush_tests.rs` | Brushes in the running game: a character stands and walks on a Solid one, passes through one that isn't, a brush never falls, and one made Solid while playing joins physics; the scale check of Play on 10,000 solid brushes. | |
| `flat_physics.rs` | 2D physics in the running game (decision 0071): entities with a `Collider2D` join `physics::flat`, their pixels turned into metres with the project's 2D settings; each tile map's solid cells become walls, built again chunk by chunk only where cells changed. When Rapier fails, 2D physics is started again from the entities and maps, at most once every `RESTART_WAIT` steps. Tests in `flat_physics_tests.rs`. | `World::set_pixels`, `set_solid_sets`, `take_flat_broke`, `FlatBroke`, `RESTART_WAIT` |
| `screens_and_sounds.rs` | The screens entities show and the sounds they play, as the running game sets them. | `World::set_screen`, `World::audio_sources` |
| `checksum.rs` | One number for everything the game has set, for telling whether a replay matches. | `World::checksum` |
| `projectiles.rs` | Thousands of small, fast things that aren't entities (lasers, bolts), each with a place, speed, time left and colour, moved in jobs every tick. | `Projectiles`, `Projectile`, `Shot` |
| `animators.rs` | Animators in the running game (Phase 9, D3b): each entity's run stepped every tick before sprites, its sprite's clip and speed set, game code's settings and jumps, values for saves and the checksum. Tests in `animators_tests.rs`, with the scale case of 10,000 animators. | `Animators`, `value_words`, `value_of_words` |
| `tile_maps.rs` | Tile maps in the running game (Phase 9, D4): each entity's copy of its map file, started and restarted from its file, changed cell by cell, in the checksum and saves. Tests in `tile_maps_tests.rs`. | `MapFiles`, `World::set_tile_maps`, `World::tile_map`, `World::set_tile`, `World::set_cell_value` |
| `sprites.rs` | Sprites playing clips (Phase 9, D3): each playing Sprite moved on every tick, the events it passes and the clips that finish handed back, game code's plays and stops. Tests in `sprites_tests.rs`. | `SpriteCue`, `ClipSets` |
| `game_set.rs` | What the running game sets on an entity that the area's data doesn't hold, all of it in one list: taken off an entity before an edit while playing makes it again from the area (`update` for one, `rebuild` for all) and put back after, each part kept while the area still gives what it gave. Tests in `game_set_tests.rs`. | `World::set_by_hand` |
| `tests.rs` | Tests: building worlds from areas, stepping them, and finding things in them. | |
| `api_tests.rs` | Tests of what the editor, save games and drawing rely on: seeds, the clock, the entity list, physics questions, cameras between ticks. Part of `tests.rs`. | |

**Read next:** the crate's [README](../README.md), and [ARCHITECTURE.md](../../../ARCHITECTURE.md), "The area".
