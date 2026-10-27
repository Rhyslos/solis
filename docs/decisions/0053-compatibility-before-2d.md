# 0053. Compatibility comes first: updates never lose work

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

The project lead is moving their own game to Solis. From then on, an engine update that breaks the game, or worse corrupts it, costs real work. The project lead asked how updates can be made safe for a running game project.

Today:
- Solis writes files atomically: a temporary file is swapped in only when complete, so a crash never leaves a half-written file.
- Area files carry a layout version, and a newer one is refused with a clear message.
- But nothing upgrades an *older* file: there has only been one version so far.
- A newer Solis refuses save games made by an older one. For a shipped game, that would wipe players' progress on every update.
- Nothing guards the names game code calls.
- No real project is tested as a whole before an update ships.

## Options

1. **The full set:**
   - engine builds installed side by side, each project choosing its own;
   - Preview features off unless a project turns them on;
   - behaviour versions, so a project keeps old game behaviour until it opts in;
   - Stable and Preview update channels;
   - plus everything in option 2.

   About a whole phase. Its riskiest part, side-by-side builds, changes the installer and updater in ways GitHub's machines can't fully test.
2. **The light version:** only the parts that work without anyone operating them:
   - a backup before every upgrade;
   - upgraders for every file format;
   - save games that survive updates;
   - the game-code interface guarded;
   - real projects tested before an update ships;
   - breaking changes labelled.

   About half a phase.

## Decision

Option 2, chosen by the project lead, who was sceptical of machinery that has to be used well to protect anything. The guarantee is: **you never lose work**:
- a backup before every change;
- Git underneath;
- files that can't half-write;
- most breakage caught before it reaches the project.

When something does slip through, going back is installing the previous build, and the last ten are kept on the Releases page.

It is Phase 8, before 2D. The plan is [docs/plans/compatibility.md](../plans/compatibility.md).

## Consequences

- Every pull request that changes a file format or what game code can call must bring its upgrader, keep an example of the old version as a test, and say so under "Breaking" in the changelog. A check enforces it.
- Builds side by side, Preview features, behaviour versions and update channels can come later, when many people's games depend on Solis.
