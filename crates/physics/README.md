# physics

**Layer 1 · Engine**

The game's physics: Rapier in its strict mode ([decision 0036](../../docs/decisions/0036-physics-is-rapier-strict-on-every-pc.md)), so the same game gives exactly the same result on every PC.

## Purpose

Wrap Rapier (`rapier3d`, built with `enhanced-determinism`) so the rest of Solis never meets it: bodies and colliders belong to entities by their area IDs, and places and directions are Solis's (`glam`, Z up), turned into Rapier's at this crate's edge.

- `Physics::add` makes an entity's body (`Body`: dynamic, kinematic or fixed; mass, damping, gravity, upright, fast) and collider (`ColliderSpec`: a box, sphere or capsule, scaled with the entity; friction, bounciness). An entity with a collider and no body gets a fixed one. `remove` takes it out.
- `set_pose` follows game code's moves (a kinematic body moves there over the next step, pushing what's in its way; others jump there); `set_velocity` and `push` (an impulse) change how a body moves.
- `step` moves physics on by one tick; `moved` lists the dynamic bodies awake, in entity order, for the world to follow.
- `State` is everything physics knows, shared (`Arc`) with game code while a tick's phases read it on every core: `velocity`, `pose`, `cast_ray` (the first collider a line meets, passing out of those it starts in), and `checksum` (Phase 6's determinism). It's cloneable and serializable (saves, M4), and changed only between phases, when no one else holds it, so usually without a copy.
- Limits: at most `MAX_BODIES` (100,000; more are counted in `refused`). Nothing that isn't a number reaches Rapier: positions, rotations, velocities and pushes that aren't finite are refused, and odd settings (a NaN mass, a negative size) are made sensible.

Everything is kept in entity order (a `BTreeMap`), never a hash map's, so the same changes always make the same handles. Rapier's own `parallel` mode isn't used: its order of work isn't fixed.

## Must never

- Let a game reach Rapier's types: games use natives in `script`, the world uses this crate's own.
- Use Rapier's `parallel` feature, or anything whose result depends on timing or the number of cores.
- Panic on what game code or a damaged area gives it.

## Depends on

`reflect`, glam, serde and `rapier3d` (Apache-2.0).

## Status

Phase 6, M2: bodies, colliders (box, sphere, capsule), pushes, velocities, rays and checksums. Colliders from a model's own shape, the character mover and snapshots for saves come later in Phase 6.
