# 0037. Input as named actions

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Phase 6 gives game code input. Games that read raw keys can't be rebound, don't work with gamepads without extra code, and can't be recorded and replayed simply. And while a game plays inside the editor, its keys (W, S, R, Delete) are also editor keys.

## Decision

Game code reads **named actions**, never keys: `held("jump")`, `pressed("interact")`, `axis("move")`. A project keeps its actions and their bindings (keyboard, mouse, gamepad) in a file; players can rebind them.

The ready-made actions and their default bindings, chosen by the project lead (CONTRIBUTING rule 17):

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | W A S D | left stick |
| Look | mouse | right stick |
| Jump | Space | A |
| Sprint | Shift | left stick click |
| Interact | F | X |

Clicking the Game panel gives the game the mouse and keys: the pointer is hidden and held, and the editor's own keys are off. Esc gives them back. Other choices offered were holding the right mouse button to look (like the editor's fly camera), and arrow keys with no mouse.

Actions are sampled once per tick, before game code, so every core sees the same input for a tick (decision 0033), and a recording is just those samples.

## Consequences

- Esc is the editor's: a game can't use it while playing in the editor. Exported games (Phase 10) can.
- Gamepads come through `gilrs` (Apache-2.0 or MIT).
- A rebinding menu for players needs game UI (Phase 9); until then, natives rebind and the project's Input actions window edits the defaults.
