# physics/src

The game's physics (decision 0036): the Rapier library in its strict mode, so the same game gives exactly the same result on every PC. Bodies and colliders belong to entities by their area IDs; positions and directions use the engine's own types (Z up), which are Rapier's too (both use the same glam), so games never meet Rapier itself.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | 3D physics: the physics world, adding and removing bodies and colliders, stepping it once per tick, characters moving with steps and slopes, and reading back where everything is. Start here. | `Physics`, `Body`, `BodyKind`, `Shape`, `ColliderSpec`, `CharacterSpec`, `CharacterState`, `BodyState`, `Hit`, `State` |
| `guard.rs` | Rapier's work, guarded: a panic in it never crashes Solis. 3D and 2D physics each run Rapier's changes through a `Guard`, which remembers that one failed until asked (`take_broke`), and can be made to fail on purpose for tests (`fail_next`). | `Guard` |
| `flat.rs` | 2D physics (decision 0071): Rapier2D in its strict mode, a world of its own beside the 3D one. Solid things and characters (an entity's `FlatCollider`, a circle or a box) and walls (each tile map chunk's rectangles, from `tiles::walls`); characters moved by Solis's own mover (a shape cast, a small `SKIN` back out, a slide along what's met, and a check of where each piece ends for anything the cast missed), so they never catch on the seams between walls or end inside one. Tests in `flat_tests.rs`. | `Flat`, `FlatState`, `FlatCollider`, `FlatMove` |
| `tests.rs` | Tests: bodies fall and stack, the same every run, and nothing that isn't a number gets in. | |

**Read next:** the crate's [README](../README.md). The physics components themselves (what's saved in an area) are in [`area/src/builtin/physics.rs`](../../area/src/builtin/physics.rs).
