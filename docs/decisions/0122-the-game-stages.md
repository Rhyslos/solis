# 0122: The game stages: 2D ordered by the kind of game it lets you make

- **Status:** Accepted (the project lead, 2026-11-18); amends 0097 and 0120's order of the 2D and shared phases
- **Date:** 2026-11-18

## Problem

After decision 0120 the roadmap ran about 120 milestones, phase by phase, by system. The project lead said they felt unsure about the plan:
- it was too long to hold in their head;
- it didn't show what they'd be able to make at each step;
- the 3D world tools coming forward and going back again shook their trust in the order;
- it didn't start from what they want Solis for now, which is learning to make every kind of 2D game, with 3D later because 2D features are easier to build first.

## Options

- **Stages by kind of game**, each pulling in the general features that kind needs.
- **The phases by system as before**, just planned one milestone at a time.

Asked separately:
- how far ahead to plan in detail: answered "the next milestone, unless it's huge";
- whether the remaining files-as-Lust work ("Usable first", L2 to L6) comes first or later: answered "later".

## Decision

- **The 2D and shared work is ordered in four stages by the kind of game it lets you make:**
  1. a top-down adventure;
  2. a platformer;
  3. a puzzle or turn-based game;
  4. an action game or shooter.

  Then come finishing and sharing games (polish, the finished picture, translations, debugging tools, cutscenes and behaviour, the rethink, scripting, export, Git and mods). Then 3D, as decisions 0120 and 0121 left it.
- **The features are unchanged.** They are the general tools ROADMAP.md's phase sections describe, and a stage only decides when each is built. The project lead's games are made with Solis, never built into it.
- **Only the next milestone is planned in detail.** The stages are the outline, revisited at each stage's end.
- **Each stage ends with a sample game of its kind,** made with Solis's tools and offered in Learn. The project lead doesn't have to make a game to finish a stage. "Usable first"'s gate goes with it.
- **The remaining files-as-Lust work (L2 to L6) moves to "later".**
- The two-PC replay check (decision 0120) comes early in stage 1, before timers put more maths into game state. The renderer's restructure still comes first in materials and shaders, now in stage 4.
- **The first milestone is triggers.**

## Consequences

- [docs/plans/game-stages.md](../plans/game-stages.md) holds the stages and the next milestone's plan. ROADMAP.md's order points to it, and its phase sections become the catalogue of features.
- Phase gates for 2D and shared phases give way to each stage's sample. The project lead's big testing pass is still at the rethink.
- How each feature is built is still in [the plan after the rethink](../plans/after-the-rethink.md).
