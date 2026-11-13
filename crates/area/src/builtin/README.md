# area/src/builtin

The built-in components. Each is a typed Rust struct that turns into values and back (`reflect::Component`); missing or damaged values take their defaults, so an older or damaged area still loads.

| File | What it does | Main items |
| --- | --- | --- |
| `mod.rs` | Lists the components, builds the registry of their schemas, and makes the starter area a new project opens with. Start here. | `registry`, `starter_area` |
| `transform.rs` | Where an entity is: position, rotation and scale relative to its parent. Z is up, units are metres. | `Transform` |
| `model.rs` | A model drawn at the entity: an imported asset, or one node of it. | `Model` |
| `light.rs` | The sun, point lights and spotlights, and whether a lamp shines only at night (decision 0092). A light shines along its entity's -Z. Its numbers are put right as they're read (`Light::sanitized`: a cone in `CONE_RANGE`, nothing below zero, nothing that isn't a number), so what draws a light never checks them again. | `Light`, `LightKind` |
| `environment.rs` | The sky, fog, exposure and tone mapping, saved with each area (decision 0017); day and night (a day's length, start and colours) and Light sprites (decision 0092). | `Environment`, `Sky`, `ToneMapping` |
| `day.rs` | A day's colours (decision 0092): time→colour stops written as text (`6:00 #e99a6a, …`), the tint at any hour, and whether it's night. Tests in `day_tests.rs`. | `DayColours` |
| `camera.rs` | What the game shows: its View (perspective, or 2D: straight down, pixel-perfect), field of view, near and far, and its projection. Looks along the entity's -Z. | `Camera`, `CameraView` |
| `audio_source.rs` | A sound that keeps playing at the entity: a sound event by name, on or off, and a volume (Phase 7, S5). | `AudioSource` |
| `reverb_zone.rs` | A place where sounds echo: a box or a ball with a soft edge, a preset and a strength (Phase 7, S6). | `ReverbZone`, `ZoneShape` |
| `listener.rs` | Where the game hears from: Hears now and the groups it's in (Phase 7, S8). | `Listener` |
| `physics.rs` | Physics components (decision 0036): a body that moves, a shape to bump into, and a character's standing capsule. Plain values; the game's world turns them into physics. | `RigidBody`, `Collider`, `CharacterBody` |
| `collider_2d.rs` | The 2D collider (decision 0071): a circle or a box on the ground, in pixels, for 2D physics; solid and still, or a character game code walks, which slides along walls, solid tiles and other colliders. Plain values; the game's world turns them into 2D physics. | `Collider2D`, `Shape2D` |
| `animator.rs` | An animator (Phase 9, D3b): the animator file that chooses the Sprite's clip, the state it starts in, and a speed. | `Animator` |
| `tile_map.rs` | A tile map (Phase 9, D4): a tile map file placed at the entity, its top-left corner on the entity's place. | `TileMap` |
| `screen.rs` | A game screen shown (decision 0087): which of the project's screens, whether it's shown, its order, and its look (glow, outline, pixel size; decision 0089). Kept as plain values in the world, so saves and checksums keep it. | `Screen` |
| `parallax.rs` | A parallax layer (decision 0094): how far the entity's Sprite moves with the camera across and up, whether it repeats, and whether it's in front of everything; where it's drawn for a camera. | `Parallax` |
| `sprite.rs` | A sprite (Phase 9, D2): one frame of a sprite sheet lying on the ground at the entity, standing on its pivot slice, with flip, tint, flash, outline and see-through. | `Sprite` |

**Adding a component:** write its struct here, implement `reflect::Component`, and add it to `registry` in `mod.rs`. Properties then draws it without any editor code.
