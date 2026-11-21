# 0135: The scope reset

- **Status:** Accepted (the project lead, 2026-11-21)
- **Date:** 2026-11-21

## Problem

Solis was meant to be a small, simple engine for making good 2D and 3D games. Piece by piece it took on the shape of a big commercial engine:
- three languages for every call;
- determinism and scale targets for every feature;
- twenty runs of every test;
- a decision record for every choice;
- open worlds and a renderer rework in the plan.

A small feature took half a day, long builders were cut off by restarts and spend limits, and the project lead couldn't tell what they could make well.

## Decision

[docs/plans/scope-reset.md](../plans/scope-reset.md) is the plan, and it wins where older rules or plans disagree.
- **The goal:** a small, simple engine judged by the games it makes well. 2D first. **The dialogue system stays**, as Solis's edge for story-driven RPGs.
- **Next:**
  - a "done" list written by games;
  - a "Can I make this?" manual page marking each common need as works well, works with a workaround, or not yet;
  - then Fixes 2 to 4 (trimmed), dialogue, and the done list's gaps.
- **Lust only** for new game code. Lua and Rust are frozen where they are, and a catch-up phase comes at the end of 2D.
- **A light path for most changes:**
  - targeted tests;
  - GitHub's Windows run as the full check;
  - the manual and CHANGELOG only.

  The full path is kept for file formats, saves and big systems.
- **Small milestones,** a fresh chat per milestone, and small jobs done directly.
- **Parked:**
  - new determinism work per feature (what's in the two-PC check stays);
  - per-feature scale targets;
  - open worlds, foliage, the renderer rework, Tools 1b;
  - 3D beyond the done games;
  - three-language examples;
  - routine twenty-times runs.

## Consequences

- This amends:
  - decisions 0002 and 0003 (all three languages taught for every call);
  - 0112 (the guide's three-language examples; its check must allow Lust alone);
  - 0120 and 0122 (the order: the done list now decides it);
  - 0011 (scale checks at a phase's end only).
- CLAUDE.md points to the plan at the top of its rules.
- Work on the stage list continues only where the done list needs it.
