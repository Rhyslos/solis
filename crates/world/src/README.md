# world/src

The game's fast storage while it runs (decision 0024). The area (`area::Area`) is the document the editor edits: easy to change, undo and save. A `World` is built from it when Play is pressed: the same entities in the `hecs` library, with the built-in components as typed data, physics, and projectiles, made to step quickly.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Everything about the world: building it from an area, adding and removing entities while playing, the game's clock, stepping physics, and finding things. Start here. | `World`, `Clock`, `Id`, `Name`, `Parent`, `Global`, `Values` |
| `projectiles.rs` | Thousands of small, fast things that aren't entities (lasers, bolts), each with a place, speed, time left and colour, moved in jobs every tick. | `Projectiles`, `Projectile`, `Shot` |
| `sprites.rs` | Sprites playing clips (Phase 9, D3): each playing Sprite moved on every tick, the events it passes and the clips that finish handed back, game code's plays and stops. Tests in `sprites_tests.rs`. | `SpriteCue`, `ClipSets` |
| `tests.rs` | Tests: building worlds from areas, stepping them, and finding things in them. | |
| `api_tests.rs` | Tests of what the editor, save games and drawing rely on: seeds, the clock, the entity list, physics questions, cameras between ticks. Part of `tests.rs`. | |

**Read next:** the crate's [README](../README.md), and [ARCHITECTURE.md](../../../ARCHITECTURE.md), "The area".
