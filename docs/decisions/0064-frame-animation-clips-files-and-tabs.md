# 0064. Frame animation: clips from Aseprite tags and clips files, edited in Clips and Frames tabs

- **Status:** Accepted (builds on 0057 and 0063)
- **Date:** 2026-10-28

## Problem

D3 makes sprites move. It had to settle where a sprite's clips come from and live, how the running game plays them so replays and saves agree, how a frame's event becomes a footstep sound and reaches game code, what the editor's animation tools look like, and whether sprites animate while editing.

## Options

- **Where clips live:** only Aseprite's tags (edit everything in Aseprite); or tags plus clips made in Solis, saved in a file of their own.
- **The editor's tools:** a clip preview in Properties (the first mock-up); a Godot-like panel at the bottom with the clips and the frames side by side (the second); or the clips list beside Properties and the frames at the bottom, appearing with an open file (the third).
- **Playing in the game:** each Sprite's clip moved on by the tick and kept as game state; or worked out from the game's clock when drawn.

## Decision

Chosen by the project lead from three mock-ups drawn on screenshots of the editor:

- **Clips are Aseprite's tags plus clips made in Solis.** A sprite's clips file (`.solframes`, a settings file in the Lust style, one line per clip) names its sheet, the clip played first, what Solis adds to each tag (a speed, events) and clips made here from any of the sheet's frames (their frames, times, direction, repeats and events). A tag's own frames, times and direction stay Aseprite's. A Sprite plays an Aseprite file directly (its tags) or a clips file.
- **Two tabs that come with the file.** Opening a clips file (double-click in the Library, or **Edit clips…** in a sheet's Properties, which makes one beside it) brings a **Clips** tab beside Properties (the clips, each one's speed, direction, repeats, and the one played first) and a **Frames** tab at the bottom: the chosen clip's frames along a time ruler, each as wide as it lasts, the frames a ping-pong plays back faded after them, a blue line where it's playing, and its events in a lane below. Closing either closes both. They're never saved in a layout. Not a copy of Godot's panel: the ruler shows timing at a glance.
- **Controls** (asked first, Godot-like recommended and chosen): click picks a frame, Ctrl+click and Shift+click pick several, dragging reorders a clip made here, Delete takes picked frames out, Space plays and stops, and Ctrl+Z and Ctrl+Y undo and redo the file's changes, all while the pointer is over the tabs. Every change is saved at once.
- **A Sprite's clip, whether it plays and how far in are game state.** The Sprite gains Clip, Playing, Speed and Clip time; each tick the running game moves every playing Sprite's time on by the tick times its speeds, in a fixed order, in the checksum and in saves (`World::step_sprites`). A clip played over and over keeps its time within one pass, so precision never drifts. An edit while playing keeps the game's playback unless it changed the area's.
- **Events:** a frame's event (Aseprite's cel user data, or one added in Solis) plays the sound event of that name on the entity by itself, and reaches game code as the engine's `AnimationCue(string name)` event, ahead of the tick's other events. A clip that finishes sends `ClipFinished(string clip)`. Game code brings them in with `fetch "solis/animation":AnimationCue;` and plays clips with `playClip(entity, clip)` (safe every tick: a clip already playing or finished carries on as it is), `replayClip(entity, clip)` (from its beginning) and `stopClip(entity)`.
- **The viewport animates only with Preview animation on** (the ≡ menu; asked, recommended and chosen), as decision 0057 says for 3D, so the editor rests; the game always animates.

## Consequences

- The state machine (next) chooses a Sprite's clip; it works on the same clips, and a `.solanim` file will sit beside the clips file.
- A clip's time is in seconds; frame lengths are whole milliseconds as Aseprite's are.
- Keeping layers separate and palette swaps still wait (decision 0063).
- Engine events are new in the description files; later engine events (collisions, triggers) use the same `ENGINE_EVENTS` list.
