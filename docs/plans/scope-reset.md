# The scope reset: a small, simple engine again

**Read this first.** The project lead, 2026-11-21 ([decision 0135](../decisions/0135-the-scope-reset.md)). Where this plan and an older rule or plan disagree, this plan wins until it's been carried into them.

## Why

Solis was meant to be **a small, simple engine for 2D and 3D games**:
- a simple editor;
- the tools to make a game: picking frames for animations, animation players, a scene to move around in and place things;
- room for large worlds if wanted;
- above all, making good games without restrictions.

Piece by piece it grew into something shaped like a big commercial engine:
- every call in three languages;
- the same result on every core and every PC for each new feature;
- scale targets on every feature;
- each test run 20 times;
- a decision record for each choice;
- open worlds, foliage and a renderer rework in the plan.

Each step made sense on its own. Together they made a small button take half a day, and long builders got cut off by restarts and spend limits. The project lead also couldn't tell which features help and where they'd fight the engine.

## What Solis is for now

- **A small, simple engine** with the tools a game needs, judged by **the games it lets you make well**, not by how much it has.
- **2D first.** 3D gets what the chosen games need, later.
- **The dialogue system stays** (the project lead: "it gives the engine a unique edge for those wanting to make story-driven RPGs with lots and lots of dialogue right out of the box"). It's built as planned in [dialogue/](dialogue/README.md), with the light process below.

## The first two jobs (the next chat starts here)

1. **The "done" list, by games.** With the project lead, name the games Solis must make well. Suggested start:
   - a top-down adventure like Orc Invasion;
   - a simple platformer;
   - a dialogue-heavy story RPG.
   For each, list what it needs from the engine. **Anything none of them needs is parked.** Write the list in this file, under "Done means".
2. **A "Can I make this?" page** in the manual (`docs/manual/can-i-make-this.md`, in SUMMARY.md near the start). It lists every common need of those games (moving between rooms, characters finding their way, walking behind tall things, inventories, dialogue, saving, menus, directional animation and so on), each marked:
   - **works well**;
   - **works with a workaround**, saying how;
   - **not yet**, saying when.

   It's kept current with every change, so the project lead always knows what they can do.

Then the order is:
1. Fixes 2 to 4 from [Orc Invasion](game-stages.md#fixes-from-making-orc-invasion), trimmed to what the done list needs.
2. The dialogue system.
3. The rest of the done list's gaps, smallest first.

Fixes 2's unfinished start is saved on the branch `fixes/2-game-code-calls`.

## The lighter process

**Lust only from now on.** New game-code calls are built, tested and documented for Lust only, and the manual shows Lust only.
- What Lua and Rust can already do keeps working and keeps its tests, frozen where it is.
- A **catch-up phase** adds the missing calls to Lua (cheap, since Lust becomes Lua) and Rust (real work) at the end of 2D.
- New calls still follow [the guidelines](../game-api-guidelines.md) (methods on things, no string names), so the catch-up fits.
- **First light-path job:** the user guide's check that every game-code example has Lust, Rust and Lua blocks must allow Lust alone.

**Two kinds of change.**

| | Light path | Full path |
|---|---|---|
| **For** | Most things: UI, editor tools, small features, game-code calls, fixes | File formats, saves, the checksum, big new systems (the dialogue system's core) |
| **Tests** | Targeted tests for what changed, run once; picture tests when UI changes | The full suite locally, fuzz for anything read from files, scale cases where it matters |
| **Checks** | fmt and clippy on the crates touched, then GitHub's Windows run as the full check | Everything in CLAUDE.md's Builds |
| **Docs** | The manual and CHANGELOG, plus the "Can I make this?" page if it changes | Also the decision record, the tracker, ARCHITECTURE.md and the CLAUDE.md sentence |

**Also:**
- **UI mock-ups:** still first for new panels and screens. Small changes to existing UI (a button, a fold) don't need one.
- **Milestones stay small:** one feature each, so a builder finishes in well under an hour. If a job looks bigger, split it.
- **A fresh chat for each milestone,** or each few, so the long conversation isn't re-read on every step.
- **Small jobs are done directly,** without a separate builder, when that's quicker.

## Parked

Kept as they are, not extended, until the done list needs them:
- **New determinism work per feature.**
  - The two-PC check and the checksum keep running for what's already in them.
  - New features don't have to join the check, run on several cores, or be measured against 1 and 4 cores, unless they're saved game state on the full path.
- **Scale targets per feature.** The scale check runs at the end of a phase only.
- **Open worlds, foliage, the renderer rework (Shared 7's start), Tools 1b,** and the 3D track beyond what the done games need.
- **Three-language examples** in the manual: Lust only, as above.
- **Running each new test 20 times:** only for tests that time things or race.

## What doesn't change

- One milestone at a time, merged when GitHub's checks are green, with the merge message and the testing doc.
- No name-dropping, and readable code.
- The user manual describes what's in main.
- Asking about keys and mouse actions, and choices as questions.

## Done means

*(Written with the project lead in the next chat.)*
