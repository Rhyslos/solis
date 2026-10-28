# script/src

Game code (decision 0028): a project's Lust, checked and turned into Lua by the Lust compiler, run in Luau while the game plays, together with the game's Lua and Rust components. Game code is untrusted: every call has a time limit, memory is capped, and code that misbehaves ends in a failure, never a frozen editor.

**Start reading at** `lib.rs`, then `runtime.rs` (one Luau running components) and `game.rs` (shared across every core).

## Loading code

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers; checks save-slot names. | `valid_slot` |
| `code.rs` | Gathers every `.lst` file in a project's assets within limits and checks and compiles it, on a worker. | `Code`, `Compiled`, `CodeProblem`, `gather`, `check` |
| `describe.rs` | The engine described to Lust: the `solis/*.lst` files a project gets, made from the list of native methods so they always match (decision 0016), plus the ready-made components in [`kit/`](kit/README.md). | `NATIVES`, `Native`, `descriptions` |
| `lust_program.rs` | The game's Lust loaded into Luau, with its components' fields, schemas and events. | |
| `lua_components.rs` | Components written in Lua (decision 0003): each `.lua` file in the assets makes components with `component`. | |
| `rust_component.rs` | Components written in Rust by a game, running in the same ticks and order and sending the same events as Lust's. | `RustComponent`, `Context` |

## Running it

| File | What it does | Main items |
| --- | --- | --- |
| `luau.rs` | The Luau game code runs in, made safe: time limits per call, per tick and per load, a memory cap, and no access to files. | `TIME_LIMIT`, `TICK_LIMIT`, `Clock` |
| `interrupt.rs` | The watchman inside Luau that stops code running past its time limit, set cheaply (decision 0034). | |
| `runtime.rs` | One Luau holding the game's running components, one per entity and component name; swapping in new code while the game runs. | `Runtime`, `Failure`, `Ticked`, `Swapped` |
| `syncing.rs` | Making running components from the area's values, and passing on what changed. | |
| `ticking.rs` | One tick: every component's `update` in one order, then delivering events. | `Order` |
| `events.rs` | Events between components: queued by `send`, delivered after the tick's updates in a fixed order, so a game runs the same way every time. | `Event` |
| `host.rs` | What game code can ask of the engine: the `Host` that answers the native methods, lent to Luau only while game code runs. | `Host`, `NoHost`, `Shot` |
| `convert.rs` | Values between the editor's kinds (`reflect`) and how Lust keeps them in Luau. | |
| `game.rs` | Game code on every core (decision 0033): entities split into shards, each with its own `Runtime`, all seeing the world as it was at the start of the tick. | `Game`, `GameWorld`, `WorldView`, `Change` |

## Tests

`runtime_tests.rs` and the `runtime_*_tests.rs` files test `runtime.rs`: events, swapping code, Lua components, hostile code that once froze or crashed the editor, where the runtime meets the rest (seams), and the speed fixes. `game_tests.rs` tests `game.rs`: the same results on one shard or many.

**Read next:** the crate's [README](../README.md).
