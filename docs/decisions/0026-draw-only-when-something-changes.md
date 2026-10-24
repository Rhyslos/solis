# 0026. The editor draws only when something changes

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

The editor drew a new frame as fast as the screen allows, all the time, even with nothing moving. That kept the processor and graphics card busy while idle, so laptops ran warm and their fans ran. The review of Phase 3's code (decision 0027) flagged it.

## Options

1. **Always draw.** Simple, and the frame-rate counter always has a number. Wastes power while idle.
2. **Draw only when something changes,** and look for changes on disk now and then. What most editors do (Blender, and Godot by default). The frame-rate counter has nothing to measure while idle.

## Decision

Option 2, chosen by the project lead (2026-10-24).

- **Frames back to back** while something moves: the game is running (not paused), the camera flies, a key or mouse button is held, a gizmo or a box is being dragged, or a click's answer is on its way from the graphics card (`Workbench::wants_frames`).
- **Ten frames a second** while background work is waiting: an import, thumbnails, a file window, or a new file waiting for its sidecar.
- **Otherwise, one frame a second,** so changes on disk and messages from background work still show. Any input draws a frame at once, as egui always does.
- **The counter in the viewport's corner says "Idle"** instead of a frame rate. Frames drawn while idle, and the first one after, aren't counted in the frame rate.

## Consequences

- Nothing on screen changes by itself unless one of the cases above says so. Anything new that moves by itself (an animated preview, a playing sound's meter) must add itself to `wants_frames`.
- A file changed outside Solis shows within a second, not at once.
