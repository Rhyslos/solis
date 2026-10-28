# 0065. Game code's calls into Solis follow the Lust API guidelines

- **Status:** Accepted (changes 0057's choice 9 and 0064's event names)
- **Date:** 2026-10-28

## Problem

Solis's calls for game code grew one milestone at a time: free functions taking the entity first (`playClip(me, "walk")`, `positionOf(e)`), strings for names the project already knows (clips, input actions, sounds), and no shared shape for names (`positionOf`, `onGround`, `velocityOf`). A typo such as `"wlak"` builds and silently does nothing. The animator was about to add more of the same (`setAnimSwitch(me, "moving", true)`, decision 0057, choice 9).

## Options

- Keep the calls as they are and name new ones the same way.
- Give every call one shape that suits Lust: methods on things, states for fixed choices, mistakes caught before the game runs.

## Decision

The project lead's guidelines, [docs/game-api-guidelines.md](../game-api-guidelines.md), are the source of truth for every call game code makes into Solis. In short:

1. Acting on one entity is a method on it (`me:moveTo(p)`); calls with no owner live on a few engine objects (`Input`, `World`, `Physics`, `Audio`, `Saves`).
2. Fixed choices are states, never strings: the editor writes the project's names (clips, cues, input actions, sounds, assemblies, buses) into `solis/project.lst` as states.
3. **Anything made in an editor reaches code through a file the editor writes:** an animator's states, switches, numbers and triggers (later a behaviour tree, a material, a UI screen) get a Lust file with a state for its names and a class with one method per thing code can set or ask (rule 4, as the project lead rewrote it).
4. Readers are nouns, yes/no questions read after `is`, writers are verbs (`set` + the noun); several answers come back as one class (`RayHit`); variants put the difference at the end (`animateFromStart`); every call is safe every frame; events are past tense and carry states (`ClipFinished(Clip clip)`, `CueReached(Cue cue)`); Lust, Lua and Rust use the same words.

The open questions were answered with the guide's recommendations: `method vec3 entity:position() native;` for engine files, a straight switch (old names become mistakes that name the new call), the five engine objects above, and `me:walk(wanted)`.

## Consequences

- **Every existing call moves over** in a milestone of its own (in a few pull requests, one area each), once Lust has two changes, made in the Lust repository: natives that take and give back states, and native methods on `entity`. Solis then moves its pinned Lust forward. `fire` leaves the engine for the kit (game systems stay out of the engine).
- **No new string-named call is added meanwhile.** The animator (D3b) is built and edited now, but game code reaches it only once its generated Lust file can be written, in that milestone.
- CONTRIBUTING.md gains the guidelines' checklist as a ground rule, so review holds every new call to it.
