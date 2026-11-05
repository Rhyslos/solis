# 0085. The editor overhaul, before the game UI editor

- **Status:** Accepted (the project lead, 2026-11-05; carries out 0070's "the editor's look as a whole gets its own mock-ups, as part of the UI overhaul"; builds on 0004)
- **Date:** 2026-11-05

## Problem

The audit of 3 November (decision 0070) found the editor's surface was what made Solis feel far from usable: the UI "doesn't do it for me". Decision 0070 fixed the everyday basics and left the editor's look as a whole to its own mock-ups. The editor UI research of 4 November studied what people praise and complain about in Unreal, Godot, Unity and GameMaker, turned it into thirteen rules, and drew Solis's editor following them. The project lead confirmed the level editor mock-up as the editor to build (5 November), but it had no place in the build order.

## Options

- **Build it next, before the game UI editor** (chosen): the mock-up puts the UI editor in a UI workspace, so building that editor first in today's look would mean redoing it. Game UI's next step (screens in Lust) waits on a Lust change anyway.
- **After game UI:** finish the game UI editor in today's look, then restyle it with everything else.
- **After the farming gate:** finish all of "Usable first" first.

## Decision

- The level editor mock-up (`docs/plans/editor-overhaul-level.png`) is the editor's target, and the research's rules are its guide ([plan](../plans/editor-overhaul.md)).
- It is built in seven milestones, one pull request each, ahead of the game UI editor in "Usable first" step 7: **O1** the frame (top bar, status bar, panel tabs); **O2** Properties as values, not buttons; **O3** the Library as a drawer; **O4** one search for everything; **O5** playing is unmistakable, and changes made while playing are kept or undone, never lost; **O6** workspaces (Level, Animate, UI, Code); **O7** help in place (hints, the floating tool bar, the cursor readout, the Area panel's kinds).
- Decision 0070's rule stands: nothing is shown before it works, so the mock-up's **Playtest build**, Layers tab and each workspace tab arrive with what they need. New keys are asked about first (the search's Ctrl+K and the drawer's Ctrl+Space, as drawn).
- Docking stays (decision 0004): the mock-up is the standard layout.

## Consequences

- The game UI editor (the game UI plan's M4 onwards) is built inside the UI workspace, after O7. Game UI's M2 (screens in Lust) can still go ahead once Lust has its change.
- The manual's interface tour and screenshots change with each milestone.
