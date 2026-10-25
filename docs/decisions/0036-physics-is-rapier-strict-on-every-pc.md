# 0036. Physics is Rapier, strict on every PC

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Phase 6 brings physics: things that fall, collide, stack and are pushed, and a character that walks into walls and up steps. Which physics, and should it give exactly the same result on every PC?

Physics works with millions of decimal calculations a second, and different processors can round the last digit slightly differently. Physics builds on its own results 60 times a second, so tiny differences grow: after a minute, a crate that stays on a ledge on one PC can fall off it on another, from the same button presses.

## Options

1. **Write our own.** Collision between every kind of shape, finding pairs quickly, a solver that keeps stacks still, joints, fast objects that mustn't pass through walls, sleeping, character movement, and strict determinism: a year or more of hard, solved problems, likely worse than what exists.
2. **Rapier, fast mode.** Quicker physics; the same result only on the same PC. Recordings shared between PCs drift, and lockstep play (Phase 8) wouldn't work without switching later, which could change how tuned games feel.
3. **Rapier, strict mode** (`enhanced-determinism`). The same result on every PC; physics runs somewhat slower (an estimated 20–40%, for physics only).

## Decision

Option 3, chosen by the project lead. Rapier (`rapier3d`, Apache-2.0, maintained by Dimforge, written in Rust) is built with `enhanced-determinism`, for every game: it's a build choice, so it can't differ per game. Solis's target games include online strategy battles, where lockstep is the usual model, and its physics scale (characters, vehicles, a few hundred bodies) leaves room for the cost.

Simple special cases stay cheaper by hand: projectiles fly in straight lines without Rapier (Phase 5), and a game may do rough sphere checks in its own code.

## Consequences

- A new `physics` crate (layer 1) wraps Rapier; game code reaches it only through natives, so Rapier can be updated without changing games.
- The physics step runs once per tick, after game code's changes land, in the tick's fixed order (decision 0033).
- Recordings and lockstep (Phase 8) rely on this; the scale check gains a physics case to watch the cost.
- Rapier's own parallel mode (`parallel`) isn't used: its order of work isn't fixed.
