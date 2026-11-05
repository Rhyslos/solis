# 0077. Clips and cues as the project's names, and animation on the entity

- **Status:** Accepted (carries out 0065 for clips; builds on 0064 and 0073 to 0076)
- **Date:** 2026-11-05

## Problem

Game code played clips by text (`playClip(me, "walk")`) and heard events with text (`on AnimationCue(string name)`, `on ClipFinished(string clip)`). A misspelt clip did nothing. The API guidelines (decision 0065) make clips and cues the project's names, an entity's calls methods on it, and events carry states.

## Decision

- **On the entity:** `animate(Clip.Sprites.Farmer.Walk)` (was `playClip`), `animateFromStart(…)` (was `replayClip`; a variant puts the difference at the end) and `stopAnimation()` (was `stopClip`).
- **`state Clip`** in `solis/project.lst` lists every clip under its sheet's path: an Aseprite file's tags and a clips file's own clips (`sprites/farmer.aseprite`'s walk is `Clip.Sprites.Farmer.Walk`). **`state Cue`** lists every frame event's name (`Cue.Footstep`).
- **Events carry the names:** `on CueReached(Cue cue)` (was `AnimationCue`: past tense, as the guidelines ask) and `on ClipFinished(Clip clip)`. A cue or clip with no such name isn't sent.
- **Where the names come from:** the editor reads the sheets on a worker, so the app writes the project file again each time they change (`SpriteSheets::clip_names`, `GameCode::set_animation`). `--check-project` reads the sheets before checking the code.
- **Finding the clip:** the native takes the option's last part (`Walk`), and the world plays the Sprite's clip of that name, ignoring capitals (`walk`).
- A straight switch, as for 0073 to 0076.

## Consequences

- Right after a project opens, the code is checked before the sheets have been read, so a game naming clips may show mistakes for a moment, until the sheets arrive and the code is checked again.
- `animate` on another sheet's clip isn't refused yet: it plays nothing, as a missing clip did before.
- Animators (each one's own Lust file, `anim:setMoving(true)`) come next, in their own pull request.
