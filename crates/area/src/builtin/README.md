# area/src/builtin

The built-in components. Each is a typed Rust struct that turns into values and back (`reflect::Component`); missing or damaged values take their defaults, so an older or damaged area still loads.

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | Lists the components, builds the registry of their schemas, and makes the starter area a new project opens with. Start here. | `registry`, `starter_area` |
| `transform.rs` | Where an entity is: position, rotation and scale relative to its parent. Z is up, units are metres. | `Transform` |
| `model.rs` | A model drawn at the entity: an imported asset, or one node of it. | `Model` |
| `light.rs` | The sun, point lights and spotlights. A light shines along its entity's -Z. | `Light`, `LightKind` |
| `environment.rs` | The sky, fog, exposure and tone mapping, saved with each area (decision 0017). | `Environment`, `Sky`, `ToneMapping` |
| `camera.rs` | What the game shows: its View (perspective, or 2D: straight down, pixel-perfect), field of view, near and far, and its projection. Looks along the entity's -Z. | `Camera`, `CameraView` |
| `audio_source.rs` | A sound that keeps playing at the entity: a sound event by name, on or off, and a volume (Phase 7, S5). | `AudioSource` |
| `reverb_zone.rs` | A place where sounds echo: a box or a ball with a soft edge, a preset and a strength (Phase 7, S6). | `ReverbZone`, `ZoneShape` |
| `listener.rs` | Where the game hears from: Hears now and the groups it's in (Phase 7, S8). | `Listener` |
| `physics.rs` | Physics components (decision 0036): a body that moves, a shape to bump into, and a character's standing capsule. Plain values; the game's world turns them into physics. | `RigidBody`, `Collider`, `CharacterBody` |
| `sprite.rs` | A sprite (Phase 9, D2): one frame of a sprite sheet lying on the ground at the entity, standing on its pivot slice, with flip, tint, flash, outline and see-through. | `Sprite` |

**Adding a component:** write its struct here, implement `reflect::Component`, and add it to `registry` in `mod.rs`. Properties then draws it without any editor code.
