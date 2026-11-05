# 0074. Input calls on `Input`, with the project's actions as a state

- **Status:** Accepted (carries out 0065 for input; builds on 0037 and 0073)
- **Date:** 2026-11-05

## Problem

Game code asked for input actions by text: `pressed("Jump")`. A misspelt action built fine and was never true. The API guidelines (decision 0065) put whole-game tools on engine objects and make fixed choices states.

## Decision

- **`Input` is an engine object** in `solis/input.lst`: `Input:held(Action.Jump)`, `Input:pressed`, `Input:released`, `Input:axis(Action.Move)`, `Input:binding(Action.Jump)` (was `bindingOf`: a reader is a noun) and `Input:rebind(Action.Jump, key)`.
- **The project's actions are `state Action`** in `solis/project.lst`, from the project's `input.solinput` (or the ready-made actions when it has none). Solis writes the file again when the actions are saved in Edit > Input actions…, and checks the code again, so a renamed action points at every line still using the old name.
- **An action's name is its option, as it is:** it isn't changed to fit, since game code would then ask for it by a name the project doesn't have. An action named so Lust can't take it (`jump`, `Quick save`, all capitals), or two the same but for capitals, is left out of the state with a note in the Console saying to rename it.
- **`_` is allowed in names** (Lust takes `Fire_2`), for assemblies too.
- **Lua:** `solis.Input:held("Jump")`, with the action's name as text (Lua has no states).
- **A straight switch**, as for 0073: `held("Jump")` is a mistake saying `Input:held(Action.Jump)` and the two fetches it needs.

## Consequences

- The kit's components fetch `Action` and use the ready-made actions (Move, Look, Jump, Sprint, Pan, Turn, Zoom, Drag). A project that deletes one of those can't use the component that reads it; the mistake names the missing option.
- Physics, sound and animation follow.
