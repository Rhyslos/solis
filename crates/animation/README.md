# animation

**Layer 1 · Engine**

Animation (Phase 9, D3 and D3b): clips of frames played by time, with their events, a sprite's clips file (`.solframes`), and animators (`.solanim`): state machines that choose which clip plays. 3D skeletons' clips (Phase 16) build on the same clips and the same state machine.

## Purpose

Say which frame a clip shows at a time, and which events it passes between two times, the same on every PC:

- **Clips** (`clip.rs`): frames in order, each shown for its milliseconds and perhaps carrying events, played forward, reverse, ping-pong or ping-pong reversed, over and over or a number of passes, at a speed. `Clip::at` gives the frame at a time; `Clip::events_between` gives every event whose frame starts in a span of time, so stepping a clip tick by tick gives each event exactly once, however the time is cut.
- **Clips files** (`frames_file.rs`): `.solframes`, a settings file in the Lust style. It names the sheet (an Aseprite file), the clip played first, what Solis adds to the sheet's Aseprite tags (speed, events) and clips made in Solis from any of the sheet's frames. Read as a hostile file within limits, values out of range brought in with a note; fuzzed (`fuzz/fuzz_targets/frames.rs`).
- **Clip sets** (`clip_set.rs`): a sheet's tags and its clips file put together into the clips a sprite can play.
- **Animator files** (`animator_file.rs`): `.solanim`, a settings file with a block per parameter, state and transition, each by a stable random ID so two people's changes merge ([decision 0066](../../docs/decisions/0066-animator-state-machine.md)). Parameters are numbers, switches, triggers or directions; a state plays one clip, or a clip per direction (up, right, down, left and the diagonals); a transition from a state (or from any state) is taken when all its conditions hold (`moving`, `not moving`, `speed > 0.5`, `swing`, `finished`). Read as a hostile file within limits; fuzzed (`fuzz/fuzz_targets/animator.rs`).
- **Animators at work** (`animator.rs`): a file made ready to run (`Animator`) and one entity's place in it (`Run`: its state and its parameters' values). `Animator::step` takes the first transition whose conditions hold (any state's first, at most one a tick) and puts triggers out; `Animator::clip` is the state's clip, or its clip for the direction nearest the facing parameter (staying on a tie, so it doesn't flicker); `Animator::speed` multiplies in a number parameter; `Animator::carry` moves a run over to a changed file by name.

## Must never

- Use anything but additions and comparisons on time, or on an animator's values: the running game steps clips and animators once per tick, and replays, saves and checksums must agree.
- Crash, hang or read past a limit on a damaged clips or animator file.
- Know about the GPU, the window or the editor.

## Depends on

`assets` (sprite sheets and Aseprite's directions), `area` (the settings-file reader), `reflect` (asset IDs and values).

## Status

Phase 9, D3: clips, clips files and clip sets. D3b: animator files and the state machine.
