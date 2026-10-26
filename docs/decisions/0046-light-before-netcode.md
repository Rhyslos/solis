# 0046. Light, sky and atmosphere get a phase of their own, and netcode moves later

- **Status:** Proposed (waiting for the project lead's answers in [docs/plans/phase-8-light.md](../plans/phase-8-light.md))
- **Date:** 2026-10-26

## Problem

The project lead wants lighting to be Solis's selling point: "all the fancy lighting solutions working well". In particular the sky, the sun and the colours light can give, inspired by the feeling the best game sunsets give, and never the flat, sterile "Backrooms" look. The project lead also wants the effects suite and the game UI built before netcode.

The roadmap had none of this:
- lighting stopped at Phase 1's sun, 16 lamps, a painted sky and distance fog;
- decision 0008 put better indirect light after 0.1;
- effects and UI shared Phase 9, after netcode in Phase 8.

## Options

1. **Keep the order;** improve light after 0.1. The selling point arrives last.
2. **A phase for light, sky and atmosphere after sound and animation, then effects and UI, each a phase, then netcode.** Light comes before effects, since effects are lit by it and glow through its bloom.
3. **Light first, before sound and animation.** The selling point arrives sooner, but Phase 7's answered plans wait.

## Decision

Option 2, as proposed. The new order:

| Phase | What it covers | Change |
| --- | --- | --- |
| 7 | Sound and animation | unchanged |
| **8** | **Light, sky and atmosphere** | new: physical sky, time of day and night, clouds, shadows near and far, many lamps, ambient occlusion, volumetric fog, reflection and light probes, exposure and the post chain |
| 9 | Effects | the effects suite, which was half of Phase 9 |
| 10 | Game UI (Sungraph UI) | was the other half of Phase 9 |
| **11** | **Netcode** | was 8 |
| 12 | Scripting, export and Git | was 10 |
| 13 | Solis 0.1 | was 11 |

Bloom, white balance and colour grading move from the effects plan into Phase 8. Baked light probes, decision 0008's second source of indirect light, come in Phase 8 rather than after 0.1. Real-time bounced light stays after 0.1.

The effects suite's name is also open in the same plan: "Eclipse" carries trademark risk, and "Solstice" is recommended. That choice gets its own decision when it is made.

## Consequences

- **0.1 comes about two months later than the roadmap's months said,** because one phase is added and Phase 9 is split.
- **Netcode waits safely.** What it needs is built: the fixed tick, seeded randomness, the start-of-tick world, strict physics, state checksums, and save games that hold everything. Effects, UI and now light are designed to stay out of the game's state.
- **A risk while netcode waits:** nothing checks that two different PCs play the same game. A small CI check (a recorded game played on Linux and Windows, its checksum compared with one in the repository) covers it until Phase 11.
- Phase plans for effects and UI keep their content. The UI plan's file is renamed for Phase 10.
