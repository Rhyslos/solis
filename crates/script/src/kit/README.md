# script/src/kit

Ready-made components written in Lust, built into the program and given to every project as `solis/kit/...` (listed in `KIT` in [`../describe.rs`](../describe.rs)). A game fetches them and uses them as they are, or copies one to change it.

| File | What it holds |
| --- | --- |
| `character.lst` | `CharacterController`: walks with Move, sprints, jumps and falls. Goes on an entity with a CharacterBody, which does the hard part (steps, slopes, walls). |
| `topdown.lst` | `TopDownController`: walks in 8 directions (or 4) with Move, speeding up and slowing down smoothly, never faster diagonally. Goes on an entity with a Collider2D ticked Character, which stops it at walls and solid tiles. Turn `usesInput` off and set `wanted` to walk an NPC from game code. |
| `cameras.lst` | `ThirdPersonCamera`, `FirstPersonCamera` and `RtsCamera`: put one on the entity with the Camera component. Each reads the ready-made input actions, so players can rebind them. |

They are played with input in `crates/editor/src/kit_tests.rs`.
