# 0123. Solis moves to Lust alpha 74: states are set with `=`

- **Status:** Accepted (builds on 0028, 0072 and 0106)
- **Date:** 2026-11-18

## Problem

Lust moved from alpha 67 to alpha 74 (the project lead, 2026-11-18, asked for Solis to follow). Most of it is Lust's own windows (views: tables, pickers, menus, second windows, tray icons, screen readers), which an engine doesn't use. Lust's ENGINES.md asks two things of an engine:

1. **alpha 72:** game code may write `toast "Saved";`, `App:quit()` and `App:notify("Title", "Body")`. In Luau they do nothing unless the host gives `ui_toast`, `app_quit` or `app_notify`, and Solis's doesn't. `App` isn't reserved: a game's own class may still be called `App`.
2. **alpha 74:** a state is set with `=`, like any value (`mood = Mood.Busy;`, `state = Mood.Calm;` in a `has`), and `is` only asks questions. The old way (`mood is Mood.Busy;`) is now a mistake that shows the `=` way. The Lua and Rust Lust writes are unchanged.

Every game that set a state with `is` stops checking until it's changed, so this is a breaking change for game makers.

## Options

For the old way:

1. **Turn `is` into `=` for the author** before Lust sees the code. Old projects would keep checking, but Solis would be changing what the author wrote behind their back, and Lust's own tools (`lust check`, Ledger) would still refuse it.
2. **Let Lust's mistake through as it is** (what ENGINES.md asks): Problems shows "a state is set with =, so write mood = Mood.Busy;", at the line, and the author changes one word.

For `toast` and `App`: show them in the engine (a toast in the Game panel, `App:quit()` stopping the game), or leave them doing nothing, as Lust leaves them.

## Decision

- Solis pins Lust's `main` at rev `9fcd48c` (2.0.0-alpha.74, the merge of Lust's pull request 111). The library's calls Solis uses (`check`, `to_lua_with`, the problems and schemas) are unchanged; nothing in `script` needed adapting, and no other dependency changed.
- The old way is **Lust's own mistake, unchanged** (option 2): one word to change, and Solis, `lust check` and Ledger all say the same.
- Everything Solis writes or ships sets states with `=`: the kit, the samples, the fuzz seeds, the editor's generated files, the manual and the guide. A search found none that set a state at all, only the guide's colouring test of `state is Mood.Calm;`. The guide colours `state = Mood.Calm;` as Ledger does: `state` as a state, the option plain, and `is` as a question word.
- `toast`, `App:quit()` and `App:notify()` do nothing in a Solis game for now, as Lust leaves them; they can show in the engine when Solis's game UI wants them. They are Lust's own, not engine calls, so `docs/game-code-interface.txt` doesn't change.

## Consequences

- An old project's code that sets a state with `is` is refused until each such line says `=`; the CHANGELOG's Breaking part and the manual (Game code, its tips; `concepts.md`, State) say so. Questions (`is mood == Mood.Busy? then`) are untouched.
- What runs is unchanged: Lust writes the same Lua as before for a state set either way, so games, saves and the two-PC check's recording come out as they did.
- The two hangs Solis guards against are still in Lust at alpha 74 (`class_order` tracks classes by name; `Diags::add` compares each mistake with every one before it; neither changed), and the compiler still can't be stopped, so `script::code`'s and `level::lust_guard`'s guards stay, their comments naming the new revision.
