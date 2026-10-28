# input/src

The game's input as named actions (decision 0037): game code asks about "Jump" or "Move", never about keys, so players can rebind, gamepads work without extra code, and a game's input can be recorded and played back.

**Start reading at** `lib.rs`, which explains how the pieces fit, then `bindings.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules, explains the flow, and checks action and input names. | `valid_action_name`, `split_combination` |
| `bindings.rs` | A project's actions and what sets each off (keys, mouse, gamepad), and reading and writing their file (format 2; `FORMAT` upgrades version 1, which wrote the arrow keys `ArrowUp`). | `Bindings`, `Action`, `ActionKind`, `BindingsError`, `FORMAT` |
| `devices.rs` | What the keyboard, mouse and gamepad are doing, sampled into a `Frame` once per tick. | `Devices`, `Pad` |
| `frame.rs` | What game code sees of the input in one tick. | `Frame`, `ActionState` |
| `recording.rs` | A play's input tick by tick, with checksums, so a replay can say at which tick it stopped matching. Text, a line per tick, after a `solis recording N` header. | `Recording`, `Tick`, `RecordingError`, `FORMAT` |
| `tests.rs` | Tests: bindings and their file, sampling, recordings, and damaged files never crashing. | |
| `edge_tests.rs` | Limits at their edge, the stick's dead zone, pad presses and error messages (found untested by mutation testing). Part of `tests.rs`. | |

**Read next:** the crate's [README](../README.md).
