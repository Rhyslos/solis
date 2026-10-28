# animation

**Layer 1 · Engine**

Animation (Phase 9, D3): clips of frames played by time, with their events, and a sprite's clips file (`.solframes`). The state machine (the next milestone) and 3D skeletons' clips (Phase 16) build on it.

## Purpose

Say which frame a clip shows at a time, and which events it passes between two times, the same on every PC:

- **Clips** (`clip.rs`): frames in order, each shown for its milliseconds and perhaps carrying events, played forward, reverse, ping-pong or ping-pong reversed, over and over or a number of passes, at a speed. `Clip::at` gives the frame at a time; `Clip::events_between` gives every event whose frame starts in a span of time, so stepping a clip tick by tick gives each event exactly once, however the time is cut.
- **Clips files** (`frames_file.rs`): `.solframes`, a settings file in the Lust style. It names the sheet (an Aseprite file), the clip played first, what Solis adds to the sheet's Aseprite tags (speed, events) and clips made in Solis from any of the sheet's frames. Read as a hostile file within limits, values out of range brought in with a note; fuzzed (`fuzz/fuzz_targets/frames.rs`).
- **Clip sets** (`clip_set.rs`): a sheet's tags and its clips file put together into the clips a sprite can play.

## Must never

- Use anything but additions and comparisons on time: the running game steps clips once per tick, and replays, saves and checksums must agree.
- Crash, hang or read past a limit on a damaged clips file.
- Know about the GPU, the window or the editor.

## Depends on

`assets` (sprite sheets and Aseprite's directions), `area` (the settings-file reader), `reflect` (asset IDs and values).

## Status

Phase 9, D3: clips, clips files and clip sets.
