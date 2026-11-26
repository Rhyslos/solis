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

1. **The "done" list, by games: a guide, not a strict rule** (the project lead, 2026-11-22: "when I don't know myself what done means, it can't be a super strict rule"). Start with a rough first version and change it whenever ideas change. With the project lead, name the games Solis should make well. Suggested start:
   - a top-down adventure like Orc Invasion;
   - a simple platformer;
   - a dialogue-heavy story RPG.
   For each, list what it needs from the engine. Write the list in this file, under "Done means".
   - **How to use it:** work that helps one of these games, or a game the project lead is making or wants to make soon, comes first. Work that helps none of them waits, but it isn't forbidden.
   - The project lead can add a game, drop one, or ask for anything at any time, and the list just changes.
   - When nobody is sure, choose the smaller job that makes a real game easier to make now, and ask the project lead with a question to click.
2. **A "Can I make this?" page** in the manual (`docs/manual/can-i-make-this.md`, in SUMMARY.md near the start). It lists every common need of those games (moving between rooms, characters finding their way, walking behind tall things, inventories, dialogue, saving, menus, directional animation and so on), each marked:
   - **works well**;
   - **works with a workaround**, saying how;
   - **not yet**, saying when.

   It's kept current with every change, so the project lead always knows what they can do.

Then the order was: Fixes 2 to 4 from [Orc Invasion](game-stages.md#fixes-from-making-orc-invasion), then the dialogue system, then the rest of the done list's gaps. Fixes 2 to 4 and the uniformity pass are built; since 2026-11-26 the order is [the target below](#the-target-tiny-versions-cheapest-game-first) (decision 0138).

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

Kept as they are, not extended, until the done list needs them or the project lead asks for them:
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

The project lead's first version (2026-11-26). Three kinds of game; with them, 2D should have most of what any game needs. Work that helps one of them comes first; anything else waits until a game asks for it. A guide, changed whenever ideas change.

### 1. A farming and life game, like Stardew Valley (single player)

Already in Solis: tile maps with values per tile, the day clock and day colours, timers at an hour, trigger zones, tags and "nearest" questions, data tables, game screens with text effects, translations, lights, saves.

Still needed from the engine:
- **Dialogue:** conversations, the dialogue box, the conversation editor, talking to people. Voice lines and the translators' spreadsheets aren't needed for this game.
- **Inventory and hotbar support:** tooltips and drag and drop in screens. The inventory itself is the game's Lust.
- **Houses, town and mines:** rooms, moving between them with a fade, and the Door.
- **Walking behind trees and buildings.**
- **Villagers walking their routes:** pathfinding. Their days are game code on timers.
- **Save slots**, and **tile map questions** ("which cells hold a crop").
- **Polish:** rain and snow, music that follows the season and the hour, festival cutscenes, night lighting with glowing windows.

The game's own code, not the engine's: crops and seasons, shops, crafting, fishing, friendship and gifts, quests and mail.

### 2. A platformer

- **Platformer movement:** jumps set by height and time, coyote time, jump buffering, wall jumps; one-way platforms, slopes and moving platforms.
- **Paths drawn in the editor** for platforms and enemies to follow, and **components that draw in the editor** (an enemy's reach).
- **Tweens kept in saves.**
- **Game feel:** screen shake, flash, hit-stop, squash, rumble.

### 3. A 2D combat game: a platformer with sword fighting, or a side-view shooter like Strike Force Heroes

Everything the platformer needs, and:
- **Melee:** hit boxes that turn on for a few frames of an attack's animation, knockback, hit-stop.
- **Guns:** aiming with the mouse or a stick, many projectiles at once (pools), hits as game state.
- **Effects:** particles, trails, muzzle flashes and sparks.
- **Enemies and bots that move round a level of platforms:** pathfinding with jumps and drops, not just walking.
- **Music that follows the fight.**

### The target: tiny versions, cheapest game first

The project lead, 2026-11-26 ([decision 0138](../decisions/0138-tiny-first-games-in-order.md)): each feature in its smallest version that makes the games possible, grown later when a game needs more; what game code can do reasonably is a Lust recipe in the manual. **Dialogue is the exception, built in full.** Games are made possible cheapest first: "it makes more sense to let me write platformers today, and Stardew Valley in 2 weeks". Sizes are guesses until the check in step 1 confirms them.

1. **The "Can I make this?" check** (1 pull request): the manual page, with each need below marked *works*, *works with a recipe* or *not yet*. Its "not yet" rows become the milestones.
2. **The platformer** (about 3): one-way platforms; being carried by moving platforms; screen shake. Running and jumping are game code already ([the first platformer](../manual/first-platformer.md)); coyote time and jump buffering are recipes; hit-stop is `World.gameSpeed`; paths in the editor wait (marker entities do).
3. **The action platformer** (about 3): aiming with the mouse in the world; one simple particle emitter (sparks, muzzle flashes, later rain); "is there ground ahead" for bots, if it's missing. Melee is a recipe (frame events, `World:within` and tags); bullets are `fire`, with pools only if it's too slow; music follows the fight with the music fade from step 4.
   **Then, removing Rust and Lua as game languages** (about 5 or 6; the project lead, 2026-11-26: "we'll do the platformer update first, and then remove lua and rust later"). Lust stays the only language games are written in; the Lua engine underneath Lust stays. It reaches about 65 code files, 22 manual pages, the turret sample and the two-PC check's game (its Lua and Rust files rewritten in Lust, and the game recorded again). Waiting costs little: Rust and Lua are frozen, so it doesn't grow.
4. **The farming game's small pieces** (about 12):
   - a screen piece that tells game code it was clicked, and tips on hover (drag and drop waits);
   - loading another level at an entry point from game code (the fade is a screen fading, the door a trigger zone recipe; no world map);
   - sorting sprites by their feet on a layer;
   - `walkTo` round solid tiles (schedules are game code on timers);
   - saving and loading by slot number (the menu is the game's own screen);
   - "which cells hold this" on a tile map;
   - a fade from one music track to the next;
   - rain and snow on the particle emitter.
   Cutscenes are game code with timers and `moveTo`, and night lights are the lights there are.
5. **Dialogue, in full** (about 35): [the outline](dialogue/README.md), milestones 2 to 7.

### Not on the list (waits until a game asks for it)

Puzzle and turn-based tools (grid-step movement, game tests, advanced tiling), the renderer rework and materials, palette tricks, mods and Git, multiplayer, and the 3D track.
