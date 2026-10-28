# area/src/builtin

The built-in components. Each is a typed Rust struct that turns into values and back (`reflect::Component`); missing or damaged values take their defaults, so an older or damaged area still loads.

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | Lists the components, builds the registry of their schemas, and makes the starter area a new project opens with. Start here. | `registry`, `starter_area` |
| `transform.rs` | Where an entity is: position, rotation and scale relative to its parent. Z is up, units are metres. | `Transform` |
| `model.rs` | A model drawn at the entity: an imported asset, or one node of it. | `Model` |
| `light.rs` | The sun, point lights and spotlights. A light shines along its entity's -Z. | `Light`, `LightKind` |
| `environment.rs` | The sky, fog, exposure and tone mapping, saved with each area (decision 0017). | `Environment`, `Sky`, `ToneMapping` |
| `camera.rs` | What the game shows: field of view, near and far, and its projection. Looks along the entity's -Z. | `Camera` |
| `physics.rs` | Physics components (decision 0036): a body that moves, a shape to bump into, and a character's standing capsule. Plain values; the game's world turns them into physics. | `RigidBody`, `Collider`, `CharacterBody` |

**Adding a component:** write its struct here, implement `reflect::Component`, and add it to `registry` in `mod.rs`. Properties then draws it without any editor code.
