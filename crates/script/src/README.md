# script/src

Game code (decision 0028): a project's Lust, checked and turned into Lua by the Lust compiler, run in Luau while the game plays, together with the game's Lua and Rust components. Game code is untrusted: every call has a time limit, memory is capped, and code that misbehaves ends in a failure, never a frozen editor.

**Start reading at** `lib.rs`, then `runtime.rs` (one Luau running components) and `game.rs` (shared across every core).

## Loading code

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers; checks save-slot names. | `valid_slot` |
| `code.rs` | Gathers every `.lst` file in a project's assets within limits and checks and compiles it, on a worker; finds uses of renamed natives' old names, for warnings. | `Code`, `Compiled`, `CodeProblem`, `gather`, `check`, `compile`, `try_compile`, `check_unfinished`, `renamed_uses` |
| `check_turns.rs` | Checks of the code take turns, each on a thread with a large stack and a time limit; one that never finishes is given up on and kept, and no other starts until its thread ends (the author is told which check that was and what changed since). Tests in `check_turns_tests.rs`, with a stand-in for a compiler that never finishes. | `NotStarted` |
| `describe.rs` | The engine described to Lust: the `solis/*.lst` files a project gets, made from the list of native methods so they always match (decision 0016), plus the ready-made components in [`kit/`](kit/README.md). Natives renamed on purpose keep their old names for a while (`RENAMED`). | `NATIVES`, `Native`, `RENAMED`, `descriptions` |
| `interface.rs` | Everything game code can call as one text, compared by a test with `docs/game-code-interface.txt`, so the interface changes only on purpose (Phase 8, C3). Its Lua names are asked of a real Luau with the natives lent, so it lists what Lua can reach. | `snapshot` |
| `lust_program.rs` | The game's Lust loaded into Luau, with its components' fields, schemas and events, and its game-wide objects. | |
| `objects.rs` | The game-wide objects (decision 0080): each passed Lust `object`'s fields moved into a guarded store, so only the objects' updates change them; their kept values read and written for other cores, saves and new code. | `ObjectValues` |
| `lua_components.rs` | Components written in Lua (decision 0003): each `.lua` file in the assets makes components with `component`. | |
| `screens.rs` | Game screens (decision 0087): the files in `ui/`, their own program (an entry opening each, and `solis/screen.lst` with `Ui` and a reader per Lust component), and `Screens`, which runs it in a runtime of its own, lends the readers through a `ScreenReader`, and reads each view's tree for `ui` within one budget for the whole view (`Reading`: pieces, styles and bytes of text, so nothing game code hands over is copied without a count); `pictures_named` finds each `image "…"` written out in them (decision 0089), with its line; `ScreenCode::sources` keeps their files for the UI editor, and `events_named` lists the game's events for its buttons. Tests in `screens_tests.rs`, and in `screens_reading_tests.rs` for reading a view's tree: long texts, style lists shared or holding themselves, lists of anything else, the budget's edges, and a sweep of made-up trees. | `ScreenCode`, `Screens`, `ScreenReader`, `readers_file` |
| `test_support.rs` | What other crates' tests need of game code, with the `test-support` feature only (TEST-02): games and compiles given limits no busy machine reaches, game by game. | `unhurried`, `calls_unhurried`, `compile_unhurried` |
| `settings.rs` | The player's settings (decision 0091): text size and UI size (`Screens:`), fullscreen and the window's size (`Window:`), kept in their ranges, and the usual window sizes. Not game state: not in saves, recordings or the checksum. Tests in `settings_tests.rs`. | `PlayerSettings`, `Setting` |
| `rust_component.rs` | Components written in Rust by a game, running in the same ticks and order and sending the same events as Lust's. | `RustComponent`, `Context` |

## Running it

| File | What it does | Main items |
| --- | --- | --- |
| `luau.rs` | The Luau game code runs in, made safe: time limits per call, per tick and per load, a memory cap, no access to files, and no way to another file's globals or Solis's own (`sandboxed_lua` takes out `require`, `loadstring`, `getfenv` and `setfenv`; Solis's own Lua is all made in `Luau::new`, before any game code). | `TIME_LIMIT`, `TICK_LIMIT`, `Clock` |
| `interrupt.rs` | The watchman inside Luau that stops code running past its time limit, set cheaply (decision 0034). | |
| `runtime.rs` | One Luau holding the game's running components, one per entity and component name; swapping in new code while the game runs. | `Runtime`, `Failure`, `Ticked`, `Swapped` |
| `syncing.rs` | Making running components from the area's values, and passing on what changed or was set by hand. | |
| `ticking.rs` | One tick: the game-wide objects' `update`s, the tweens game code started (Lust's `tickTweens`), every component's `update` in one order, then delivering events. An `Order` also says which entity's code is running (who caused a sound). | `Order` |
| `events.rs` | Events between components: queued by `send`, delivered after the tick's updates in a fixed order, so a game runs the same way every time. | `Event` |
| `host.rs` | What game code can ask of the engine: the `Host` that answers the native methods, lent to Luau only while game code runs. | `Host`, `NoHost`, `Shot` |
| `native_values.rs` | A native as Lua calls it, reading each value it's given by itself, so one of the wrong kind is said in plain words by the call's name (`solis.World:say(words) wants text for words…`), and what's said when a native kept in a variable is called. Tests in `native_values_tests.rs`. | |
| `convert.rs` | Values between the editor's kinds (`reflect`) and how Lust keeps them in Luau (a `vec3` as a Luau vector, a `vec2` and a `colour` as tables). | |
| `game.rs` | Game code on every core (decision 0033): entities split into shards, each with its own `Runtime`, all seeing the world as it was at the start of the tick. | `Game`, `GameWorld`, `WorldView`, `Change` |

## Tests

`runtime_tests.rs` and the `runtime_*_tests.rs` files test `runtime.rs`: events, swapping code, Lua components, hostile code that once froze or crashed the editor, where the runtime meets the rest (seams), and the speed fixes. `runtime_camera_tests.rs` tests cells at places and the kit's 2D follow camera. `runtime_reach_tests.rs` calls one native of every engine object from Lua, Lust and Rust, and fails when an object has no call to try. `game_tests.rs` tests `game.rs`: the same results on one shard or many, on games made by its `unhurried` and code from its `compiled` (time limits of an hour, so a busy machine can't cut a tick short and make two runs differ); `game_objects_tests.rs` its game-wide objects; `game_clock_tests.rs` random numbers and spawned entities whatever the game's speed (paused too); `game_tweens_tests.rs` what Lust alpha 67 brought (tweens, `vec2` and `colour` fields, in saves and on many shards, and the manual's tween example). `luau_names_tests.rs` holds every name game code can see in `luau.rs`'s Luau to a list, the engine objects under `solis` among them, so a library update or a new engine object that adds one fails there.

**Read next:** the crate's [README](../README.md).
