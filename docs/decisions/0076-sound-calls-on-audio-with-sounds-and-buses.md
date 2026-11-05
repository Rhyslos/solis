# 0076. Sound calls on `Audio` and the entity, with the project's sounds and buses as states

- **Status:** Accepted (carries out 0065 for sound; builds on 0056, 0073 and 0074)
- **Date:** 2026-11-05

## Problem

Game code named sound events and buses by text: `playSound("sfx/footstep")`, `setBusVolume("Music", 0.5)`. A misspelt one built fine and was silent. The API guidelines (decision 0065) put whole-game calls on an engine object, an entity's on the entity, and make fixed choices states.

## Decision

- **`Audio` is an engine object** in `solis/sound.lst`: `Audio:play(Sound.Music.Theme)`, `Audio:playAt(Sound.Sfx.Footstep, place)` and `Audio:setVolume(Bus.Music, 0.5)` (was `setBusVolume`).
- **On the entity:** `playSound(Sound.Sfx.Engine)` (was `playSoundOn(event, e)`), `stopSounds()` (was `stopSoundsOn`) and `setSoundPlaying(playing)`.
- **The project's sound events are `state Sound`** in `solis/project.lst`, folders as options inside options, from every `.solsound` file in the assets. **Its buses are `state Bus`**, from the project's `mixer.solmix` (or a new project's buses), written again when the Mixer is saved.
- **Names in all capitals are folded rather than left out:** Lust doesn't take all-capital names, so the `UI` bus is `Bus.Ui` and a `UFO.solassembly` is `Assembly.Ufo`. This replaces 0073's rule of leaving them out; the engine finds the bus, the sound event and the assembly ignoring capitals, as the Library already did for assemblies. Input actions stay exact (0074), since game code asks for them by their exact names.
- **An option back to its file** makes each part's first letter small (`Sound.Sfx.Footstep` asks for `sfx/footstep`), as files are mostly named, and the lookup ignores capitals for the rest.
- **Lua:** `solis.Audio:play("Sfx.Footstep")` (the option's name as text; a path works too) and `solis.entity.playSound(e, "Sfx.Engine")`.
- A straight switch, as for 0073 to 0075.

## Consequences

- AudioSources and animation cues still name their event as text in the editor: they're the editor's own data, not game code, and will move with the project file milestone.
- Animation and tiles follow.
