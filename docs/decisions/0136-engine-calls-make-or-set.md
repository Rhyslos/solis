# 0136: Engine calls are made or set, names have one dot, and reads are named first

- **Status:** Accepted (the project lead, 2026-11-23)
- **Date:** 2026-11-23

## Problem

Lust's own style is blocky and easy to read: a `has` block lists what a thing holds, `method` is the one word for what it does, values are made by saying what they are, and `:` and `.` are kept rare. Solis's calls break that most: `World:spawn(Module.Ships.Fighter)`, `Items:row(Item.IronSword).price`, `Clip.Knight.Walk`. The project lead asked for them to be rethought across the engine ([the uniformity pass](../plans/uniformity-pass.md)), and settled three directions before the full audit.

## Options

For actions such as spawning, playing a sound or animating:
- A: plain words with brackets, `spawn(Module.Fighter);`;
- B: `World:` as today;
- C: methods on things, `self.spawn(Module.Fighter);`;
- D: make it or set it, `new Module.Fighter { position = gate };`, `animation = Clip.KnightWalk;`;
- E: words without brackets, `spawn Module.Fighter;`, which needs a change to Lust's grammar.

The project lead turned down A to C for their brackets and dots.

## Decision

1. **Actions are made or set (D).** An action in the world is either making something with `new` (a module spawned, a sound played, a message shown) or setting a value with `=` (what a sprite plays, which way it faces, its look). The values the engine reads from a component are in its `has` block. A plain call stays only where an action is neither; each such case is first tried as an event or a value the engine fills in.
2. **The project's names have one dot.** A name is its kind, then one name: `Clip.KnightWalk`, `Module.Fighter`, `Text.ShopWelcome`, as Lust's own states are (`Mood.Busy`). Folders and files join the one name (`KnightWalk`) only where two names would clash.
3. **A read is named first.** A row or a thing is named, then read once: `Item sword = Item.IronSword;` then `sword.price`, never a chain like `Items:row(Item.IronSword).price`.

## Consequences

- [docs/game-api-guidelines.md](../game-api-guidelines.md) gains these three rules, and new calls follow them from now on, Fixes 3's included (`facing = Facing.NorthEast;`, `look = Look.KnightMana;`).
- The existing calls, the project's names file, the manual's examples, the sample games and [the interface file](../game-code-interface.txt) are moved over in the uniformity pass, after Fixes 3. There are no projects to move.
- Where Lust can't express a call this way yet, the case goes in the pass's audit as something Lust would need, for its own rulebook.
