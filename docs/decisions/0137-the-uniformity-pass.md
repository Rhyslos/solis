# 0137: The uniformity pass: one look for each element, and the engine's calls in Lust

- **Status:** Accepted (the project lead, 2026-11-24)
- **Date:** 2026-11-24

## Problem

The project lead asked for a pass over the whole engine before dialogue ([the plan](../plans/uniformity-pass.md)): the editor draws the same kind of element several ways and builds some parts twice, and Solis's calls in Lust have long dotted names and chains that don't read like Lust. [Decision 0136](0136-engine-calls-make-or-set.md) set three directions; the audit (in the plan) then grouped every call and every kind of element, and each group went to the project lead as a question, with before-and-after code and a Design canvas of mock-ups, [Uniformity pass: one look for each element](https://claude.ai/artifact/TdXzWhTMhtqRuRsR5wgpTZ).

## Decision

**Game code:**
1. **Making is `new`.** A module, a sound, a timer and a shot are made with `new`, through a class Solis writes for each into the files it already writes: `entity ship = new Fighter { position = gate, speed = 9 };`, `new Horn { at = gate };`. `World:spawn`, `spawnWith`, `fire`, `Audio:play`, `playAt`, `playSound` and the `startTimer` calls go.
2. **Setting is `=`, except moving.** Every setter becomes a field the engine reads, as Fixes 3's `facing` and `look` are: `animation = Clip.KnightWalk;`, `flip = true;`, `screen = Screen.Hud;`, `Screens.textSize = 1.2;`. Moving and turning stay calls (`moveTo`, `turnTo`, `face`), since physics moves things too. Another entity's settings stay methods on it until Lust can write to another thing's component.
3. **Reading another entity's component stays a method,** `orc:health()`, read once into a name and never chained.
4. **`orc:despawn()` stays** a call on the entity.
5. **One dot everywhere.** The project's names are their kind and one name, the file's name and the part's joined, folders in front only where two would clash: `Clip.KnightWalk`, `Module.Fighter`, `Sound.Footstep`, `Text.ShopWelcome`. Folders stop being groups in `check`; a group becomes a value of the name (`clip.sheet`).
6. **`World:find` leaves the manual.** It stays for the Console; a one-off entity is reached by `World:the(Component.Gate)` or an entity field set in Properties.
7. **The zone events keep their names:** `Entered`, `Exited`, `EnteredZone`, `LeftZone`.

**The interface:**
1. **One button** for small actions in panels and pop-ups, plain, or filled for the one main step of a place; the ribbon keeps its icon buttons. A link stays a link only where it goes somewhere.
2. **One text box, one number box** (typed, or dragged across, with a slider beside it when the field has a range), and **one way to rename a row** in every list: double-click, the name edits in place, Enter keeps it, Esc leaves it. No new key.
3. **Checkboxes and toggles by weight.** A checkbox for a light, everyday setting in Properties (a component's yes/no field, a screen's Shown, the animator's Plays first); a toggle for a heavier setting that changes how something works, usually with a line explaining it (Preferences, Project settings, a clip's Same frames in every direction). Game code chooses for its own fields with a field hint: `bool open = false; -- [toggle]`.
4. **The name picker** (search, a picture where there is one) for every choice from a list longer than a handful; short lists stay dropdowns or segmented switches. **The facing pad** wherever a facing is chosen: the Sprite in Properties, the clips preview, an animator's direction parameter.
5. **One picture board** for every picture cut into pieces: Atlas Crop, Cut picture… and the Tileset, with the same zoom, moving and picking, and tiles numbered like cells.

## Consequences

- [docs/game-api-guidelines.md](../game-api-guidelines.md) gains these rules, and every new call follows them.
- The rework is built in milestones of the usual size (the plan's list): the shared parts first, then each panel moved onto them; then the calls, with the manual, the sample games and [the interface file](../game-code-interface.txt) in the same pull requests. There are no projects to move, so old names are dropped rather than kept.
- `docs/design/g-layout/` gains the rules for buttons, fields, switches and pickers.
- Asked of Lust: making a state option with `new` (which would remove the classes Solis writes), and writing to another entity's component.
