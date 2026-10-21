# 0009. The viewport camera flies, with Unreal's controls

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

The viewport needs a way to move around. Decision 0004 made fly the only camera mode. Q, W, R, S and T already pick tools, so the camera keys must not clash with them.

## Options

1. **Unreal-style:** hold the right mouse button to look; while it's held, keys move the camera.
2. **A fly-mode toggle:** a key enters fly mode, Esc or a click leaves.
3. **Mouse only:** drags and the wheel, no keys.

## Decision

Option 1, as in Unreal Engine:

| Action | Control |
| --- | --- |
| Look around | Hold right mouse and move |
| Move | While right mouse is held: W A S D forward, left, back, right |
| Down and up | While right mouse is held: Q and E |
| Faster or slower | While right mouse is held: mouse wheel (shown in the Camera box) |
| Move forward or back | Mouse wheel |
| Slide sideways and up/down | Hold middle mouse and move |
| Move faster for a moment | Hold Shift |

Tool keys (Q, W, R, S, T) work only when right mouse isn't held.

## Consequences

- One control scheme to learn, familiar to Unreal users.
- Orbiting around a selection (and F to frame it) can come later, once there is something to select.
