# 0075. Physics calls on the entity, and one call for a ray

- **Status:** Accepted (carries out 0065 for physics; builds on 0036, 0071 and 0073)
- **Date:** 2026-11-05

## Problem

Physics was free functions taking the entity first (`push(e, f)`, `velocityOf(e)`, `moveCharacter(e, w)`, `onGround(e)`), and a ray took three calls for its three answers (`castRay`, `castRayHit`, `castRayNormal`), each looking along the line again. The API guidelines (decision 0065) make an entity's abilities methods on it (rule 1), and give several answers worked out at once back as one class (rule 6).

## Decision

- **On the entity:** `push(impulse)`, `velocity()`, `setVelocity(v)`, `walk(wanted)` (was `moveCharacter`, the guidelines' word) and `grounded()` (was `onGround`: a yes/no reads after `is`). Bare for the component's own entity, `target:push(f)` for another. They work for 2D characters too (a Collider2D ticked Character).
- **`Physics:castRay(from, direction, reach)` gives back a `RayHit`:** `found`, `distance` (-1 when nothing), `entity` (nothing when nothing) and `normal`. `RayHit` is declared in `solis/physics.lst` from `describe::ENGINE_CLASSES`; the native gives back a table with those fields. In Lua, `solis.Physics:castRay(…)` gives the same table.
- **The `Host` trait uses the same words** (rule 10): `walk` and `grounded` (were `move_character` and `on_ground`). The world's own `World::move_character` keeps its name: it isn't game code's.
- A component may keep a field with the same name as an ability: the TopDownController's `velocity` field and `velocity()` live side by side, as Lust allows.
- A straight switch, as for 0073 and 0074.

## Consequences

- A ray is worked out once however many of its answers are used.
- Rust components call `cx.host().walk(…)` and `cx.host().grounded(…)`.
- Sound and animation follow.
