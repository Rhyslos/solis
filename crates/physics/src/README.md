# physics/src

The game's physics (decision 0036): the Rapier library in its strict mode, so the same game gives exactly the same result on every PC. Bodies and colliders belong to entities by their area IDs; positions and directions use the engine's own types (Z up) and are converted at this crate's edge, so games never meet Rapier itself.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Everything: the physics world, adding and removing bodies and colliders, stepping it once per tick, characters moving with steps and slopes, and reading back where everything is. Start here. | `Physics`, `Body`, `BodyKind`, `Shape`, `ColliderSpec`, `CharacterSpec`, `CharacterState`, `BodyState`, `Hit`, `State` |
| `tests.rs` | Tests: bodies fall and stack, the same every run, and nothing that isn't a number gets in. | |

**Read next:** the crate's [README](../README.md). The physics components themselves (what's saved in an area) are in [`area/src/builtin/physics.rs`](../../area/src/builtin/physics.rs).
