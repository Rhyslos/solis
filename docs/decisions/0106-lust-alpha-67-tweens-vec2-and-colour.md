# 0106. Lust alpha 67: tweens, `vec2` and `colour` in the engine

- **Status:** Accepted (builds on 0028, 0033, 0072 and 0080)
- **Date:** 2026-11-12

## Problem

Lust moved from alpha 59 to alpha 67 (the project lead, 2026-11-12: "Lust has had a huge update, so it's important to bring the engine's Lust version up to speed"). Most of it needs nothing from Solis: number helpers, `Random`'s tools, rotation helpers, bitwise operators, vectors in JSON and Lust's own saves. Lust's ENGINES.md asks three things of an engine:

1. **`vec2` and `colour`** cross to the engine in native methods, component fields, objects' kept fields and events: a `vec2` as a table `{ x, y }`, a `colour` as `{ r, g, b, a }`, each part from 0 to 1. Solis turns every field into one of its own kinds (`reflect::FieldKind`), for Properties, area files, save games and the checksum. A field of a kind it doesn't know was left out of all of them, so a game's `vec2` and `colour` fields were never saved.
2. **Tweens** (`tween hp to 0 over 0.5 [ease OutCubic] { … }`) move only when the engine calls the chunk's `tickTweens(dt)`, once a tick, after the game-wide objects and before the components' updates.
3. **Data files** may hold a colour word (`orange5`), read as the new `Fixed::Colour`.

Solis's colours are three numbers (sRGB, 0 to 1), as colour pickers show. Lust's have a fourth, how solid the colour is.

## Options

For `colour`:

1. **Solis's colour, dropping the see-through** (what ENGINES.md suggests). Simple, but a see-through colour's starting value (`colour shadow = [0, 0, 0, 0.5];`) would be solid on every entity, since a component is made from the area's values, and a save would lose it.
2. **A second colour kind with four numbers.** Two ways of holding one thing.
3. **Solis's colour, holding a fourth number when it isn't solid** (`Value::colour`). Every colour Solis has stays three numbers, and nothing that reads one changes: its readers take the first three.

For `vec2`: Solis had no two-number kind, so it gets one, `FieldKind::Vec2`, as `Vec3` is for three.

For tweens on many cores: each core runs its own copy of the game's code, so each moves the tweens of the components it runs. The game-wide objects run on the first core, and their values are copied to the others before the components run (0080).

1. **Open the objects while tweens move.** A tween on an object's field would then change only the copy on the core that started it, so the game would differ with the number of cores.
2. **Keep the objects closed** (0080 as it is): a tween on an object's field stops, saying the object changes only in its own update.

## Decision

- `colour` is Solis's colour, with its see-through kept as a fourth number when it isn't 1 (option 3). Properties' colour picker changes the colour and keeps how solid it is; the Console reads and shows `#rrggbbaa`. Solis's own components' colours stay three numbers, and reach screens as `vec3`s, as they always have.
- `vec2` is `FieldKind::Vec2`: two numbers, shown in Properties as X and Y.
- Both are what Lua components may declare too (`"vec2"`, `"colour"`), so the three languages hold the same kinds. No engine call takes or gives one yet; none is added here.
- Each core calls `tickTweens(dt)` at the start of the components' updates, so after the objects. The objects stay closed (option 2): an object glides its own field in its update (`Maths:moveTowards`, `lerp`). A done block's sends and changes come before every update's.
- A tween that fails (its done block, or moving an object) is said once until the code changes, and stops nothing: Lust doesn't say whose tween it was.
- A data file's colour word is three numbers to the settings layer.

## Consequences

- A game's `vec2` and `colour` fields are now in Properties, area files, save games and the checksum, and are copied between cores. An object's are kept in saves too.
- New code stops the old code's tweens where they are, without their done blocks, as Lust says. Removing an entity doesn't stop its tweens; a `send` from one goes to an entity that's gone, which Solis already ignores.
- Lust doesn't say which component a done block belongs to, so done blocks ending in the same tick on different cores keep their order on each core but not between cores: lines they say, events they send and entities they spawn can come in another order with another number of cores. Asked of Lust (a way to tell the engine whose tween it is).
- Lust's own `Random` (`Random[1, 6]`, `Random:chance`) isn't kept to the game's seed: the first roll seeds it from the clock, which sets the game's seed too. Game code keeps to `World:random` and the rest; asked of Lust.
- The compiler still can't be stopped, and the two hangs Solis guards against are still there at alpha 67 (`script::code`, `area::lust_guard`).
