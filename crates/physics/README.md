# physics

**Layer 1 · Engine**

The game's physics: Rapier in its strict mode ([decision 0036](../../docs/decisions/0036-physics-is-rapier-strict-on-every-pc.md)), so the same game gives exactly the same result on every PC.

## Purpose

Wrap Rapier (`rapier3d`, built with `enhanced-determinism`) so the rest of Solis never meets it: bodies and colliders belong to entities by their area IDs, and places and directions are Solis's (`glam`, Z up), turned into Rapier's at this crate's edge.

- `Physics::add` makes an entity's body (`Body`: dynamic, kinematic or fixed; mass, damping, gravity, upright, fast) and collider (`ColliderSpec`: a box, sphere or capsule, scaled with the entity; friction, bounciness). An entity with a collider and no body gets a fixed one. `remove` takes it out.
- `set_pose` follows game code's moves (a kinematic body moves there over the next step, pushing what's in its way; others jump there); `set_velocity` and `push` (an impulse) change how a body moves.
- `add_character` makes an entity a character (`CharacterSpec`: a standing capsule, feet at its origin, with step height, steepest slope, how far it keeps to the ground, and mass), a kinematic body. `move_character` (or `move_characters`, for a tick's moves together: each worked out on every core at once with rayon, reading physics only, then made in order, so the result is the same on any number of cores) moves it by a wanted distance with Rapier's `KinematicCharacterController`: it climbs steps (autostep) and slopes, slides along walls, gets out of anything it starts inside, pushes dynamic bodies (`solve_character_collision_impulses`) and is carried by a kinematic body it stands on. Solis snaps it down to the ground itself (`snap`), because Rapier's snapping sinks a resting character a little each tick. `State::character` says whether it stood on something (`CharacterState`), which is saved and in the checksum. Moves are clamped to `MAX_CHARACTER_MOVE`.
- `step` moves physics on by one tick; `moved` lists the dynamic bodies awake and the characters moved since last asked, in entity order, for the world to follow.
- `State` is everything physics knows, shared (`Arc`) with game code while a tick's phases read it on every core: `velocity`, `pose`, `cast_ray` (the first collider a line meets, passing out of those it starts in), and `checksum` (Phase 6's determinism). It's changed only between phases, when no one else holds it, so usually without a copy. A save game keeps only each body's `BodyState` (`State::body_states`: place, speed, spin, asleep, a character's finding), never Rapier's insides: `Physics::clear` and `set_body_state` build physics again from them, so nothing in a file reaches Rapier, and a save's physics is small and survives Rapier updates. Rotations already of unit length are kept exactly, so a body built again is where it was to the bit. A step that panics empties physics instead of crashing Solis (`take_broke`).
- Limits: at most `MAX_BODIES` (100,000; more are counted in `refused`). Nothing that isn't a number reaches Rapier: positions, rotations, velocities and pushes that aren't finite are refused, and odd settings (a NaN mass, a negative size) are made sensible.

Everything is kept in entity order (a `BTreeMap`), never a hash map's, so the same changes always make the same handles. Rapier's own `parallel` mode isn't used: its order of work isn't fixed.

## Must never

- Let a game reach Rapier's types: games use natives in `script`, the world uses this crate's own.
- Use Rapier's `parallel` feature, or anything whose result depends on timing or the number of cores.
- Panic on what game code or a damaged area gives it.

## Depends on

`reflect`, glam and `rapier3d` (Apache-2.0).

## Status

Phase 6, M2: bodies, colliders (box, sphere, capsule), pushes, velocities, rays and checksums. M4: saves (as body states since the Phase 6 review). M5: the character mover. Colliders from a model's own shape are left for later (an issue).
