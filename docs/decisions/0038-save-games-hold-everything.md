# 0038. Save games hold everything

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Phase 6 brings save games. What does a save hold: the whole game, or only what game code marks?

## Options

1. **Everything, automatically:** every entity, every component's fields, the projectiles in flight, the physics, the clock and the randomness. Loading puts the game back exactly as it was; game makers write nothing extra.
2. **Only what game code marks:** smaller files, and game makers choose what's kept, but every field that matters must be marked, and anything forgotten is lost on load.

## Decision

Option 1, chosen by the project lead. Game code calls `saveGame("slot")` and `loadGame("slot")` (`save` and `load` are Lust's own words); both land at the end of the tick, like moves (decision 0033). A loaded game carries on exactly as the saved one would have, checksum and all (decision 0036), so a save is also a way to reproduce a bug.

## Consequences

- Saves are as big as the game's state: fine for the games Solis aims at (an area of thousands of entities is a few megabytes).
- A save names the project and the Solis that made it; one that can't be read is refused with a message, never half loaded.
- In the editor saves go in the project's `saves/` folder (kept out of Git by new projects' `.gitignore`); exported games will use the player's saved-games folder (Phase 10).
- Running components keep the fields their `has` block declares; anything else a component keeps in its own variables isn't saved.
- Open worlds (after Solis 0.1) will need a follow-up: a save holds what's live, plus only what changed in each part of the world.
