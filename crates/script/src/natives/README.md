# script/src/natives

The native methods game code calls (`solis.World:say`, `entity.moveTo`), each made as a Lua function over the `Host` and lent into the `LUST_NATIVES` table by `host::with_natives` for as long as game code runs. Each native is registered under the key `describe::NATIVES` describes it by (`World_say`, `entity_moveTo`); the test `every_native_is_lent_to_lust_and_lua` in `interface.rs` lends them to a real Luau and checks the table against that list, both ways. A native that takes several values is made with `native_values::native`, so a value of the wrong kind is a mistake that names the call.

**Start reading at** `mod.rs`, whose `lend` calls each file's own `lend`; each file's natives are lent in small groups (`lend_words`, `lend_facing`, …).

| File | What it does |
| --- | --- |
| `mod.rs` | `lend`: every native, file by file. |
| `world.rs` | `World` (what game code says and finds, firing and spawning, random numbers and their seed, the clock, speed and time of day, the view's size), `Physics:castRay` and `Saves`. |
| `entity.rs` | An entity's own methods: where it is, its name and despawning it; which way it faces (and `facingAlong`); pushing and walking it; the screens it shows; the clip its sprite plays. |
| `sound.rs` | `Audio` (playing at a place, a bus's volume) and the sounds an entity plays. |
| `animators.rs` | `Animators`: an entity's animator, its values, state and speed, reached by its graph's names. |
| `maps.rs` | `Maps`: a tile map reached by its path, its cells at places, its size, terrains and values; an entity's map bounds. |
| `settings.rs` | The player's settings (decision 0091): `Screens`' text and UI sizes and `Window`'s fullscreen, size and quit. A screen's own code gets a read-only set of these instead (`runtime.rs`, `run_screen`). |
| `input.rs` | `Input`: actions held, pressed and released, axes, and bindings. |
| `states.rs` | A component's own state (stage 1, decision 0126): `keepStarted`, on the component's own entity, and the hidden native Lust's runtime calls when an object's own state changes (only a component's, one with an entity, reaches the host). |
| `timers.rs` | A component's timers (stage 1, decision 0125): `startTimer`, `startTimerEvery`, `startTimerAtHour`, `stopTimer`, `timerRunning` and `timeLeft`, on the component's own entity; a timer named by its option's text, and a mistake (nothing to call it on, a name a timer can't have) said by the call's Lua name. |

**Read next:** the folder's [README](../README.md).
