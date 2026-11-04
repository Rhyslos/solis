# 0072. Lust alpha 59: the engine's folder, `this`, and the compiler's notes

- **Status:** Accepted (builds on 0002, 0065 and the Lust proposal of 2 November)
- **Date:** 2026-11-04

## Problem

Solis pinned Lust at alpha 35. Lust's alphas 49 to 59 were made for Solis's API milestone (docs/game-api-guidelines.md): methods on `entity` (`target:push(force)`), abilities called bare inside a component, `this` for the entity (`me` still works, with a note), fields the engine fills in, data files and objects. Lust's ENGINES.md says what an engine must do for each.

Two of them need Solis to say something to the compiler. Only the engine's own files may give `entity` its methods, so Lust must be told the engine's folder (its **host folder**). And the compiler now gives notes, things that aren't mistakes but are worth saying (`me` is now `this`), which reach no one unless Solis shows them.

The folder is `solis/`, named after the program, and the program's name is never written in the code (it comes from `branding.rs`, in the app crate, which the script crate can't reach).

## Options

1. Pass the folder's name down from the app into every place code is compiled. `Code` is built in about 45 places, so every one would change, and tests would have to invent a name.
2. **Find it in the files themselves.** Solis writes the files describing the engine, and their first line already says so ("-- Made by Solis 0.x: what game code can ask of the engine."). The top folder of each file whose first line says that is the engine's.
3. Treat every folder but the game's as the engine's. A game's own subfolders would then wrongly be able to declare engine methods.

## Decision

Option 2. `script::describe::engine_folders` finds the folders from the files' first lines, and compiling and checking use Lust's `to_lua_with` and `check_with` with them. A game file that copied that first line would make its folder count as the engine's, but it gains nothing: methods it declares are native, so Solis gives them or the call fails.

Moving over showed one gap in Lust: an engine folder's files came with every program, so the kit's components joined every game unasked and would clash with a game's own of the same name. Lust alpha 59 fixes it: a component or object in the engine's folder counts only once the game fetches it, as the manual already said.

The compiler's notes go to the Console as warnings, once each time they change (not on every check).

Solis's kit, samples and manual move from `me` to `this`. `me` keeps working for now with a note; it becomes a mistake in a later Lust once games have had time to move.

## Consequences

- Moving the pin from 35 to 58 brings everything Lust gained between: `check` blocks, `print`, text with `{values}`, `+=`, sized numbers, `lock`, counting loops.
- The rest of ENGINES.md (entity methods, filled fields, data files, objects and their ticking and saving) comes with the API milestone's next pull requests, as Solis starts using each.
- The first line of the engine's description files now matters: changing its wording changes how Solis finds its folder. It's kept in one constant, `ENGINE_FILE`.
