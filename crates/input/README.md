# input

**Layer 1 · Engine**

The game's input as named actions ([decision 0037](../../docs/decisions/0037-input-as-named-actions.md)).

## Purpose

Let game code ask about actions ("Jump", "Move"), never keys, so players can rebind, gamepads work without extra code, and a play's input can be recorded and played back exactly.

- `Bindings`: a project's actions (`Action`: a button, or an axis of two numbers) and the inputs that set each off, kept in `input.solinput` beside `project.solis` (TOML, one line per setting, so it merges in Git). `Bindings::default()` is the ready-made set: Move (W A S D, left stick), Look (mouse, right stick), Jump (Space, A), Sprint (Shift, left stick click), Interact (F, X), and the cameras' Pan (arrows, left stick), Turn (Alt + left and right arrows, right stick), Zoom (wheel, Page Up and Down, triggers) and Drag (middle mouse button; decision 0041). `add_missing` adds those a project lacks. `read` refuses anything past the limits (`MAX_ACTIONS`, `MAX_INPUTS`, 1 MB) or not allowed, with why; `rebind` swaps an action's inputs of one kind (what a player's controls menu does).
- `Devices`: what the keyboard, mouse and gamepad (`Pad`) are doing, fed as it happens. `sample` makes one tick's `Frame` (each busy action's held, pressed, released and axis), so every core sees the same input for a tick; a tap between two ticks is still a press and a release. Axes from keys and sticks give which way, each from -1 to 1 (sticks past `DEAD_ZONE`); an axis with `Mouse Motion` or `Mouse Wheel` gives how far to go this tick (the mouse's pixels or wheel notches times `mouse_speed`, sticks and keys at `pad_speed` a second). A key on its own doesn't count while modifiers are held that some binding uses with it.
- `Recording`: each tick's frame with the world's checksum (and game code's every second), the seed and the start's checksum, as text with numbers by their exact bits; `first_difference` finds the tick a replay stopped matching. At most an hour (64 MB).

Inputs are named as text: keys by egui's names (`W`, `Space`, `ArrowUp`) plus `Shift`, `Ctrl` and `Alt`; combinations (`Alt+ArrowLeft`, modifiers in the order Ctrl, Alt, Shift; `split_combination`); `Mouse Left`, `Mouse Right`, `Mouse Middle`, `Mouse Back`, `Mouse Forward`, `Mouse Motion`, `Mouse Wheel` (an axis, in notches) and `Mouse WheelUp` / `Mouse WheelDown` (a notch, as a button); `Pad A` and the rest of `PAD_BUTTONS` (the Xbox layout), whose `LeftStick` and `RightStick` are sticks in an axis and presses in a button.

## Must never

- Know about windows, egui or gamepad libraries: the editor and the app turn their events into input names.
- Panic on a damaged bindings file or recording.
- Let a frame depend on when within a tick something happened, beyond press and release.

## Depends on

`reflect` (checksums), glam, serde and toml.

## Status

Phase 6, M3: actions, bindings and their file, sampling, recordings. M5: combinations, the mouse wheel, the cameras' actions.
