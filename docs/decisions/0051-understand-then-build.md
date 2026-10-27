# 0051. Understand, then build: Solis's own designs for its core systems

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

The project lead wants Solis to be efficient and fast because its systems are understood and designed for it. The aim is not to build what a library or another tool happens to have. For example: not "a particle spawner, because library X has one", but "how should particles be spawned so Solis is fast at its scale?"

## Decision

Chosen by the project lead: **every core system starts by being understood in depth, and is then designed for Solis.**

1. **Study.** A research report in `docs/reports/`. It covers:
   - how the best current methods work and why;
   - where their limits are;
   - what they cost;
   - what Solis's scale target (DESIGN.md §2) and AMD-first rule (decision 0049) mean for them.
2. **Design.** The phase plan states **Solis's own approach**, including where it differs from common practice and why.
3. **Measure.** Where two designs compete, both are prototyped and benchmarked at the scale target, on the reference card. The numbers decide, and they go in the plan.
4. **Build, then keep measuring.** The design's costs become scale cases (decision 0011), so a later change that slows it shows up.

Libraries stay welcome for what isn't Solis's core: reading file formats, talking to the operating system, maths. **What makes Solis special is written by Solis:** its rendering, light, materials, effects, simulation and editors. Code may be adapted from permissively licensed sources (with notices), but only once it is understood.

## Consequences

- Phases start a little later: the study comes first. The light plan (Phase 8) was the first done this way.
- The effects plan (Phase 10) is rewritten from such a study before Phase 10 starts. It goes deep on spawning, simulating, sorting, culling, drawing and budgeting particles on AMD cards.
- Fewer dependencies in the core, which also helps against dependency trouble (see the dependency safeguards).
