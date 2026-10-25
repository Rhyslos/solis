# 0030. Scale is tested at three levels: target, stress and ceiling

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

DESIGN.md §2 aims Solis at "a few thousand units at peak, and tens of thousands of projectiles", and the roadmap's Phase 5 gate tested one number: 2,000 units and 20,000 projectiles at 60 fps. Testing only at the target says nothing about what happens just past it, and games will go past it: two players each summoning hundreds of fighter squads of seven planes, or a horde of 6,000 zombies. Solis will reach a limit somewhere; we need to know where, and see it coming.

## Options

1. **Keep one number** (2,000 and 20,000).
2. **Raise the one number** (say 10,000 units at 60 fps). Harder to meet, and still says nothing past it.
3. **Three levels:** the target at 60 fps, a stress level that must stay playable, and a ceiling run that finds where 60 fps ends and reports it.

## Decision

Option 3, chosen by the project lead (2026-10-24):

| Level | Units | Projectiles | Must hold |
|---|---|---|---|
| Target | 2,000 | 20,000 | 60 fps |
| Stress | 10,000 | 100,000 | 30 fps, no stutter |
| Ceiling | keeps adding | keeps adding | reported: where 60 fps ends |

The benchmark's units run game code (a Lust component), so the levels time game code as well as drawing.

## Consequences

- Phase 5's gate checks the target and stress levels on the project lead's PC, and reports the ceiling.
- The scale check reports all three levels at the end of every phase from Phase 5 on, so a drop shows before a game finds it. Phase 7 keeps them with animated units.
- Most of what makes this fast (drawing many copies in one go, culling, LODs, using every core) helps every kind of game, open worlds included. Game-specific systems still stay in games.
