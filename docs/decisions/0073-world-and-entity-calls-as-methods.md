# 0073. World and entity calls as methods, and the project's names as states

- **Status:** Accepted (carries out 0065 for the world and entity calls; builds on 0072)
- **Date:** 2026-11-05

## Problem

The game API guidelines (decision 0065) say every call game code makes into Solis takes one shape: an entity's abilities are methods on it, called bare inside a component; calls with no owner live on a few engine objects; fixed choices are the project's names as states, never strings. Lust alpha 59 can do all of it. This is the first area to move: the world (`say`, `spawn`, random numbers, saves) and the entity (`positionOf`, `moveTo` and the rest).

## Decision

- **Entity abilities:** `position()`, `moveTo(place)`, `rotation()`, `turnTo(facing)`, `face(direction)` (new: turns to face along a direction), `name()` and `despawn()`, declared in `solis/entity.lst` as `method vec3 entity:position() native;`. Bare for the component's own entity; `target:moveTo(place)` for another. `facingAlong` stays a plain method, for working out a turn without making it.
- **Engine objects:** `World` (`say`, `time`, `frameTime`, `find`, `fire`, `spawn`, `random`, `randomRange`, `randomInt`, `setSeed`) and `Saves` (`saveTo`, `loadFrom`), as `object World { … }` in `solis/world.lst`, brought in with `fetch "solis/world":[World, Saves];`.
- **`Saves:saveTo` and `Saves:loadFrom`, not `save` and `load`:** the guidelines suggest `Saves:save`, but `save` and `load` are Lust's own words (for its files), so they can't be names. `saveTo` and `loadFrom` read as the same thing.
- **The project's names:** Solis writes `solis/project.lst` with the project's assemblies as `state Assembly`, folders as options inside options (`ships/fighter.solassembly` is `Assembly.Ships.Fighter`). It's written when the project opens and again whenever its assets change, only when the text differs. Each part's first letter is made a capital; a name Lust can't take (starting with a digit, a dash or space, all capitals, one of Lust's objects like `Time`), two names the same but for capitals, or a file beside a folder of the same name is left out with a note in the Console. A project with none has `Assembly.None`, which makes nothing. A native gets the full option (`"Ships.Fighter"`) and finds the file by it, ignoring capitals, as the Library does. `World:spawn` takes an `Assembly`, so a misspelt or deleted assembly is a mistake before the game runs.
- **Lua uses the same words:** `solis.World:say(words)` (the colon, as in Lust) and `solis.entity.moveTo(target, place)`. Lua's `math.random` follows `World:random`.
- **A straight switch** (0065's answer): the old calls are gone. Lust's own "there's no method called positionOf." becomes "positionOf is now written target:position() (inside a component, position() for its own entity).", and a moved call that kept its name (`moveTo(this, place)`) gets the same help after Lust's words. Lua's `solis.positionOf(…)` is found before anything runs, outside comments, and is a mistake saying the new way. The list is `describe::MOVED`.

## Consequences

- Every game's world and entity code needs changing; the Problems panel says how for each line. The kit, the samples and the manual are moved.
- `fire` stays on `World` for now; 0065 moves it into the kit as a component later, since projectiles are a game's system.
- Input, physics, sound and animation follow in their own pull requests, with `Action`, `Sound`, `Bus`, `Clip` and `Cue` joining `project.lst`.
- What game code can call changed on purpose: `docs/game-code-interface.txt` is written again, and CHANGELOG says so under Breaking.
