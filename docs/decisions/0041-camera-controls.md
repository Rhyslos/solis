# 0041. The ready-made cameras' controls

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Phase 6's milestone 5 gives game makers ready-made cameras (RTS, first person, third person, chosen by the project lead) and a playground for the phase's gate. They add mouse and key actions, so the project lead chooses them (CONTRIBUTING rule 17). Two of the choices need things input didn't have yet: a key with a modifier held (Alt + arrow) and the mouse wheel.

## Options

Each control was asked about with a recommended choice and alternatives; the project lead answered in their own words, quoted in the plan (`docs/plans/phase-6.md`, M5).

## Decision

Chosen by the project lead:

| Action | Keyboard and mouse | Gamepad | Used by |
|---|---|---|---|
| Pan | arrow keys | left stick | RTS camera: moves over the map |
| Drag | hold the middle mouse button (and move the mouse) | | RTS camera: drags the map |
| Turn | Alt + left and right arrows | right stick | RTS camera |
| Zoom | mouse wheel, Page Up and Page Down | right and left triggers | RTS camera; third-person camera when allowed |
| QuickSave | F6 | | the playground |
| QuickLoad | F7 | | the playground |

- The **third-person camera** keeps a fixed distance; a **Wheel zooms** tick box among its fields in Properties lets the wheel (Zoom) change it.
- The gamepad bindings were suggested with the controls and not changed.
- Pan, Turn, Zoom and Drag join the ready-made actions (decision 0037) in every new project; QuickSave and QuickLoad are the playground's own.
- Input gains **combinations** (`Alt+Left`, written with Ctrl, Alt and Shift in that order) and the **mouse wheel** (`Mouse Wheel` as an axis, counted in notches; `Mouse WheelUp` and `Mouse WheelDown` as buttons). A key on its own doesn't count while modifiers are held that some action binds with it, so Alt + left turns without also panning.
- Zoom, like Look, says how far to go this tick: a notch is one step, and Page Up or Down held is eight steps a second.

## Consequences

- Projects made before this lack the camera actions; the Input actions window offers to add them, changing nothing else.
- Esc stays the editor's (decision 0037), which is why the saves aren't on it.
- The cameras and the character controller are Lust in `solis/kit/`, written into every project like the engine's descriptions, so game makers can read and copy them.
