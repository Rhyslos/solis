# 0033. During a tick, game code sees the world as it was when the tick started

- **Status:** Accepted
- **Date:** 2026-10-24

## Problem

Phase 5's first measurement (M1) found that game code is the stress level's biggest cost: 10,000 Lust components take about 22 ms a tick on one core, against a whole frame's CPU budget of 16.7 ms (decision 0030). Making each call cheaper wins perhaps a quarter (Luau's native code generation gave 8%; the safety timer and the crossings between Lust and the engine are most of the rest). Running game code on every core is the way to many times more.

But until now game code has run one component at a time, each seeing every change the ones before it made. Run on several cores at once, a component would see some of the others' changes and not others, differently on every PC and every run.

## Options

1. **The start-of-tick world:** during a tick, game code reads the world as it was when the tick started; what it changes lands at the end, together, in a fixed order. A component sees its own changes at once. The usual answer for games built for many cores, and for networked games, which must run the same everywhere.
2. **Keep one core:** game code sees every change at once, but 10,000 Lust units can't reach the stress level in the editor.
3. **Only Rust components on several cores:** Lust and Lua stay on one.

## Decision

Option 1, chosen by the project lead (2026-10-24).

- **Reading:** `positionOf`, `rotationOf`, `nameOf` and `find` give the world as it was when the tick (or the round of events) started, except for what the component itself changed this tick, which it sees at once.
- **Writing:** `moveTo`, `turnTo`, `say` and `send` are kept, and land when the tick's updates are done, in a fixed order: by the entity that made them (in ID order), then the component (by name), then the order it made them in. Two components moving the same entity: the last in that order wins.
- **Events** are delivered in rounds: the events sent during the updates first, then those sent while reacting to them, and so on, each round reading the world as the round before left it. That's the same order as before (first sent, first delivered).
- The result is the same on one core or sixteen, on every PC, every time.

## Consequences

- Game code runs on every core (Phase 5, M3): each core runs its own share of the entities, in a Luau of its own with the same code.
- A component that reads another entity's place gets where it was at the start of the tick, 1/60 s ago. Games rarely notice; the manual explains it.
- Phase 6's determinism and Phase 8's networking build on "the same result everywhere".
