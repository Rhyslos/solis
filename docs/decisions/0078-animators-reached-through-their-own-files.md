# 0078. Animators reached through their own Lust files

- **Status:** Accepted (carries out 0065's rule 4 for animators; builds on 0057, 0077 and Lust alpha 52's filled fields)
- **Date:** 2026-11-05

## Problem

Game code couldn't set an animator's parameters at all. The API guidelines (decision 0065, rule 4) say anything made in an editor reaches code through a file the editor writes: a state for its names, and a class with a method per thing code can set or ask, so no name is ever a string in game code.

## Decision

- **Each animator gets `solis/animators/<path>.lst`**, written by the app whenever the animators are read (`script::animator_code`, from `SpriteSheets::animator_names`). It holds `state FarmerState` (the graph's states, as Lust names: *Use tool* is `UseTool`) and `class FarmerAnimator` with `setMoving(on)` for a switch, `setSpeed(value)` for a number, `setFacing(direction)` for a direction, `swing()` for a trigger, and `current()`, `jumpTo(state)` and `setPlaybackSpeed(scale)`. The file sits in the engine's folder, so it's Lust's host file and its class can be a filled field. A file no longer wanted (an animator deleted or renamed) is taken away.
- **A component holds it as a field the engine fills in** (`FarmerAnimator anim;`, Lust alpha 52): Solis makes one for the entity (`make`, with `owner`) as the component is made, before its first update. It isn't a setting in Properties, and isn't saved (it's made again).
- **The class's bodies call the engine's `Animators` object** with the graph's own names (`Animators:setSwitch(owner, "moving", on)`). Game writers never see them; the names in them come from the file the editor wrote, so they always match. A name that doesn't (the animator changed since the code was checked) sets nothing.
- **Setting lands at the end of the tick**, as moves do (`Change::AnimatorSet`, `AnimatorJump`, `AnimatorSpeed`); `current()` reads the state as the tick started (`WorldView::animator_states`), so every core sees the same.
- A name that can't be a Lust name, a parameter whose method the class has already, two states the same but for capitals and spaces, or two animators with the same file name, is left out with a note in the Console.

## Consequences

- `anim:setMoving(true)` is checked by Lust: renaming *moving* in the graph points at every line still using it.
- The guidelines' rule that `animate(...)` on an entity with an animator is refused isn't built yet.
- `Animators` is visible to game code (it has to be, for the files to call it), but the manual teaches the class.
