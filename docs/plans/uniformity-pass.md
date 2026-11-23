# The uniformity pass

Asked for by the project lead on 2026-11-23, to go over with them before it's built. It comes after Fixes 3 and before dialogue. It's a big rework, and better done now, while there are no projects to move.

## What they asked

1. **One look, built once.** The editor shouldn't style the same kind of element in several ways, and a part made for one place should be usable everywhere else it fits. Their example: the atlas's cutting of a picture into boxes. They expected sprites to use it already.
2. **How the engine's calls read in Lust.** This means Solis's calls, not Lust itself. Long dotted names (`Something.Something.Something`, as in `Clip.Knight.Walk`, `Module.Ships.Fighter` or the planned `Look.Sprites.Knight.Mana`) are not how they want Solis code to look. The calls are to be rethought across the whole engine.

## The project lead's rule for code (2026-11-23)

Code should be **blocky and easy to read**:

- Things list what they hold in a `has` block, and `method` is the one word for what they do.
- A value is made by saying what it is (`something name = "something new"`).
- **`:` and `.` are avoided as far as they can be.** Lust keeps them rare, and Solis's calls are where it goes wrong (`World:spawn(…)`, `Items:row(Item.IronSword).price`, `Clip.Knight.Walk`).

Lust itself was partly shaped by Claude, so the rule isn't always enforced there either. The aim is a middle ground that still works: Solis's calls as close to that blocky style as Lust allows, and where Lust can't reach, a note of what Lust would need, for its own rulebook.

## Settled already (2026-11-23, decision 0136)

Three directions were settled before the audit, so new work follows them now:
- **actions are made or set**: `new Module.Fighter { position = gate };`, `animation = Clip.KnightWalk;`, with a plain call only where neither fits;
- **names have one dot**: `Clip.KnightWalk`;
- **a read is named first**: `Item sword = Item.IronSword;` then `sword.price`.

The audit applies them to every existing call and brings the cases that don't fit to the project lead.

## How it's done

1. **An audit, written down first.** This plan gains two lists:
   - **The interface:** every kind of element (buttons, fields, lists, pop-ups, tabs, headers, pickers, drawing a box on a picture, grids) and each place it's drawn, with the ones drawn differently and the parts built twice (the atlas's box drawing, the tileset's grid guess, the sprite sheet's cells).
   - **Game code:** every engine call and project name in [the interface file](../game-code-interface.txt) and the manual's examples, grouped by shape (dotted names, methods, events, data files' classes), checked against the [guidelines](../game-api-guidelines.md) and Lust's own rulebook.
2. **Proposals as questions.** A recommended shape for each with a before and after, side by side, asked one choice at a time (CLAUDE.md, "Choices come as questions"). The look's proposals are mock-ups first.
3. **A decision**, then the guidelines and `docs/design/g-layout/` are updated, and the rework is built in milestones of the usual size: shared widgets first, then each panel moved onto them; the calls renamed with the manual, the sample games and the interface file in the same pull requests.

## Until then

- New UI reuses an existing part where one fits. Fixes 3's cutting pop-up is built on the atlas's box drawing, as its plan says, not beside it.
- New calls follow the guidelines as they stand. Anything Fixes 3 adds (`Facing.NorthEast`, looks' names) is in the audit's list like everything else.
