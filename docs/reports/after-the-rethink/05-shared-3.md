# 05 · Shared 3: time and feel

ROADMAP.md (ideas 1 to 7 and 16): **timers** (pause with the game, kept in saves); **tween** (with easing); **steps in order** ("walk there, wait a second, say this"); **states that know when they start and stop**, and stop what was started in them; **screen shake, flash and fade, controller rumble**; **smooth randomness and weighted random** (noise, rare drops, shuffled decks). **Gate:** "a hit shakes the screen and flashes the sprite, a menu slides in, a door opens two seconds after a switch, and a chest drops a rare item now and then."

Suggested milestones: **M1 timers and steps in order**, **M2 tweens made game-grade**, **M3 states with enter and leave**, **M4 shake, flash, fade and rumble** (plus hit-stop and squash moved here from 2D 2, question 5 there), **M5 noise and weighted random**.

## The central problem: saves, replays and blocks of code

Everything time-based in this phase runs a block of code later. Solis's rules are strict: game state is saved, checksummed, the same on 1 core or 16, and replays match exactly (decisions 0033, 0035, 0038). The ideas list already noted the trap: "Hooks aren't saved: 'when done' is an event or a block, never a stored hook, so saves and replays work."

A pending block ("in 2 seconds, open the door") is a closure. Closures can't be written into a save: they hold references to local variables and code locations that change when the code changes. Engines that run scripts as coroutines (yield and resume) can't save them either. So there are three shapes:

| Shape | What it is | Saved? | Today's Lust? |
| --- | --- | --- | --- |
| **Timers as data, finishing with an event** | `startTimer(Timer.Door, 2.0)`; later `on TimerFinished(Timer t)` | Yes: (entity, name, due tick) | Yes |
| **Blocks that capture only fields** | `after 2.0 { open = true; }`, where the block may read and write the component's fields and `this`, but no locals | Yes: (entity, component, block's stable ID, due tick); the block is found again by ID after a reload or a code swap | Needs a Lust change (blocks lifted to hidden methods with stable IDs) |
| **Coroutines** (`wait 1.0` inside a method) | Most readable for sequences | No (a suspended coroutine can't be saved, and a code change breaks it) | Lua has coroutines; Lust's `wait` is for console programs |

**What already exists:** Lust tweens (`tween hp to 0 over 0.5 [ease OutCubic] { … }`, decision 0106) move when Solis calls the chunk's `tickTweens(dt)` once a tick after the game-wide objects. Decision 0106 records three gaps: (1) **done blocks ending in the same tick on different cores keep their order on each core but not between cores**, so what they say, send or spawn can come in another order with another core count; (2) **new code stops the old code's tweens** without their done blocks; (3) tweens are not in saves (the decision doesn't mention saving them, and a closure can't be saved). Lust's own `Random` isn't seeded by the game (game code must use `World:random`).

**Recommendation:** ask Lust for **one rule shared by tweens, timers and steps: a block run later may capture only the component's fields and `this`** (Lust's checker refuses locals in it with a clear message), and give each such block a **stable ID** (file, owner, ordinal) plus a way for the host to list, save and restore pending ones and to say **whose** each is (entity, component). Then Solis:
- orders done blocks by (tick, entity ID, component name, start order) on every core, fixing decision 0106's gap;
- saves pending tweens and timers in save games (a new save format version, rule 23) and the checksum;
- keeps them across code swaps when the block's ID still exists (and drops them with a Console note when it doesn't).

Until Lust has that, **timers as data plus events** works today and is the fallback.

## M1. Timers and steps in order

**What it gives:** "do this in 2 seconds", "every day at 6", "walk there, wait, say this".

**Game code** (guidelines checked):
```
after 2.0 { open = true; }                     -- block form (needs the Lust rule above)
every 5.0 { spawnWave(); }
startTimer(Timer.DoorOpens, 2.0);              -- data form, works today
on TimerFinished(Timer which) { … }
stopTimer(Timer.DoorOpens);
is timerRunning(Timer.DoorOpens)? then { … }
float left = timeLeft(Timer.DoorOpens);
```
`Timer` is a state of names. Where do timer names come from? Recommended: a component declares them as a local state (`state Timer = [DoorOpens, Respawn];`) so no project-wide list is needed; the engine stores them by option name.

- **Game time, not wall time:** timers run on the fixed tick (`world::TICK`), pause when the game pauses, follow `World:setGameSpeed`, and are exact in ticks (2.0 s is 120 ticks, decided by integer tick counts so there is no float drift).
- **"Every day" and clock times:** `atTimeOfDay(6.0) { … }` using the Environment's day clock (decision 0092), firing when the hour passes 6, including after `setTimeOfDay` jumps (fire once if jumped over? recommend: fire, once).
- **Steps in order:** nested `after` blocks give steps written top to bottom:
  ```
  walkTo(well);
  after 1.0 { World:say("Hello"); after 2.0 { walkTo(home); } }
  ```
  A small kit helper (a `Sequence` object of steps by state) makes longer ones readable. Real `wait` inside methods (coroutine style) is not recommended: it can't be saved.
- **Storage:** a per-world timer wheel or a sorted list keyed by (due tick, entity ID, component, order). Each tick, due timers fire in that order during the first round of events. Lives in `world` beside animators; saved in save games; in the checksum.

**Tests:** fires on the exact tick at speeds 0.5, 1, 2; pauses when paused; saved mid-wait then loaded fires on the same tick; 1 and 4 cores identical order; despawning the entity cancels its timers; a code swap keeps timers whose block still exists. **Scale case:** 100,000 pending timers, 10,000 firing in one tick.

**Size:** medium (plus the Lust change).

## M2. Tweens made game-grade

**What it gives:** a value changes smoothly over time with easing (a menu slides in, a door swings, a coin pops). Lust already has the syntax; this milestone makes it safe for games.

- Fix decision 0106's ordering gap (done blocks ordered by entity and component across cores, once Lust says whose each tween is).
- Save and restore running tweens (target field, from, to, duration, elapsed ticks, easing, done-block ID).
- Pause with the game and follow game speed (tween time is tick time; check `tickTweens` gets the tick's game time, not the frame's).
- Keep tweens across a code swap when possible.
- **Two kinds of tween:** game-state tweens (moving a field, saved, deterministic) and **look-only tweens** (squash, a UI wobble) on frame time, never saved, which the renderer or screens own. Screens already have Lust's `animate`; a sprite's look-only scale belongs with M4.
- **Easing set:** the standard family (linear; sine, quad, cubic, quart, quint, expo, circ, back, elastic, bounce; in, out, in-out; Penner's equations) plus a curve asset later (Shared 4's curves, decision 0050's shared curve editor). Evaluate easing with plain arithmetic (no transcendental functions in game state where avoidable, since `sin`/`exp` can differ in the last bit across machines; Lust's "same everywhere" rule for maths must be checked for `sin`).

**Tests:** a tween at speed 2 finishes in half the ticks; saved mid-tween and loaded continues; 1 and 4 cores identical; bit-identical results across runs. **Size:** small to medium.

## M3. States that know when they start and stop

**What it gives:** a character that's "stunned" runs something as it becomes stunned and as it recovers, and **anything started while stunned (a timer, a tween, a sound, a spawned entity) stops by itself when the stun ends**. The ideas list notes no other engine does the second part.

**Lust fit:** Lust already has a component or object's own `state is …;` and "things made in a state end with it" (LUST.md §8) for objects made with `new`. This milestone extends that rule to engine things:
```
on enter Stunned { flash(); startTimer(Timer.Recover, 1.5); playSound(Sound.Sfx.Dizzy); }
on leave Stunned { … }
```
- **Lust side:** `on enter` / `on leave` handlers for a component's own state options (and options inside options), called by Lust when `state is …` changes. A Lust change.
- **Solis side:** every timer, tween, looping sound and spawn started during an option is tagged with (entity, component, state option) and cancelled or despawned when that option ends (unless `keep`), in a fixed order, at the tick's end. Spawned entities "ending with the state" fits Lust's `keep` rule.

**Comparison:** hierarchical state machines with entry/exit actions are the established pattern (statecharts); behaviour trees (Shared 10) are the other. This milestone is the small, code-first version; Shared 10's diagrams can build on it (a drawn state machine compiles to the same enter/leave).

**Tests:** leaving stops its timers, tweens and sounds; options inside options (leaving `Combat.Melee` for `Combat.Ranged` ends Melee's things only); save inside a state and load: `enter` isn't run again; 1 and 4 cores identical. **Size:** medium (plus the Lust change).

## M4. Screen shake, flash, fade and rumble (and hit-stop, squash)

**Techniques:** Eiserloh's "trauma" model: a trauma value 0 to 1 added by hits, decaying over time; shake = trauma² (or ³) × smooth noise per axis and roll; never random jumps. Shake moves the drawn camera only. Fades and flashes are overlays. Rumble is a two-motor strength over time.

**What's game state and what isn't:** shake, flash, fade, squash and rumble are **drawing and output only**: they never change the game, aren't saved and aren't checksummed (the 2D plan's rule: "camera smoothing, shake, flashes and squash never write back into the game"). Game code **starts** them (a change landed at the tick's end, so the trigger is deterministic) but can't read them back. **Hit-stop is game state** (it stops game time for some ticks), so it goes through the game speed.

**Game code:**
| Call | Notes |
| --- | --- |
| `World:shake(0.5)` / `camera:shake(0.5)` | adds trauma |
| `flash(colour, 0.1)` on an entity | the Sprite has flash already; extend to models through the material (Shared 7) |
| `World:fadeTo(colour, seconds)`, `World:fadeIn(seconds)` | a full-screen overlay drawn over the game and under game UI |
| `Input:rumble(0.6, 0.2, 0.3)` (low motor, high motor, seconds) | through `gilrs`' force feedback, which supports Windows; for the player whose gamepad it is |
| `World:hitStop(0.08)` | game time stopped for some ticks; sounds and look-only effects keep going |
| `squash(1.2, 0.8, 0.15)` | look-only scale that springs back |

**Where:** shake and fade in `render` (a camera offset applied after `camera_between`, a fade quad in the post step); rumble in the app's input loop (`gilrs` is already a workspace dependency, 0.11, whose force feedback works on Windows); hit-stop in `world` (the tick loop counts down stopped ticks).

**Pixel games:** shake in whole pixels of the game's resolution (the pixel stage), optional.

**Tests:** shake never changes checksums; trauma decays to zero; a fade covers the 3D view and the pixel stage but not UI (or UI too, a setting); hit-stop of 0.1 s stops exactly 6 ticks. **Size:** medium.

## M5. Smooth randomness and weighted random

**What it gives:** values that drift (swaying grass, flickering lamps, a drifting camera); "usually common, sometimes rare"; shuffled decks so nothing repeats too soon.

- **Noise:** `Noise[x, y]` (the ideas list's sketch, next to Lust's `Random[1, 6]`) or `World:noise(x, y)`. Must be **bit-identical on every machine** when used for game state, so it's integer-hashed gradient noise (OpenSimplex2 or classic Perlin) computed in Solis's Rust and exposed as a native, never Luau maths. One implementation should serve everything that needs noise: this, the ground's recipe (Open worlds W1/W4), decoration rules (2D 1), materials' noise box (Shared 7, a matching WGSL version for drawing) and effects. Recommend a small layer-0 module or crate (`noise`), written by Solis (about 300 lines) with FastNoise Lite (MIT) as the reference or exit plan.
- **Weighted random:** `World:pick(Rarity, [70, 25, 5])`? The guidelines want states, so: a state's options with weights, `Rarity r = World:weighted(Rarity.Common 70, Rarity.Rare 25, Rarity.Legendary 5);` needs Lust syntax for option-weight pairs; the ideas list wrote `Random[Common 70, Rare 25, Legendary 5]`. A Lust change (and it must use the game's seed, which Lust's own `Random` doesn't, decision 0106). Fallback today: `World:randomInt` plus a table.
- **Shuffle bags:** a deck object (`Deck`) with the game's randomness: draw until empty, reshuffle. Saved like objects.
- **Seeding:** every draw comes from the game seed, the tick and the call's order (as `World:random` does today), so it's the same on any core count.

**Tests:** noise identical in Lust, Lua and Rust and across runs (golden values); weighted picks match their weights statistically and identically per seed; a deck never repeats before it's empty. **Size:** small to medium.

## Dependencies and order

- Lust changes needed: blocks capturing only fields with stable IDs and owners (M1, M2), `on enter`/`on leave` (M3), weighted state picks (M5). **Ask the Lust side at the phase's start**; each milestone has a today's-Lust fallback.
- M4's flash on models waits for Shared 7 materials (sprites have it already).
- M5's noise is wanted earlier by Tools 1b/Open worlds (the recipe) and 2D 1 (decoration): **build the noise module where it is first needed** (the ground's recipe, if Open worlds stays first) and expose it to game code here.
- Shared 10 (behaviour, cutscenes) builds on M1 to M3.

## Risks and questions for the project lead

1. **Blocks run later may use only the component's fields** (no local variables), so they can be saved. **Recommended:** yes (it's what makes timers, tweens and steps saveable). Alternative: allow locals and don't save pending blocks (a loaded game loses them).
2. **Timers by name or by block?** **Recommended:** both: `after`/`every` blocks for convenience, named timers with `TimerFinished` for when code elsewhere must stop or ask about them.
3. **A clock time jumped over** (`setTimeOfDay` past 6:00 with `atTimeOfDay(6)` pending): **recommended** fires once; alternative: skipped.
4. **Shake and fade in pixel games** snapped to whole pixels: **recommended** yes by default.
5. **Rumble:** for all pads or the active one? **Recommended:** the pad that last gave input (the player's).
6. **Weighted random syntax** (needs Lust): **recommended** `Random[Common 70, Rare 25, Legendary 5]` over a state, using the game's seed.

## Crates

`gilrs` 0.11 (already used; Apache-2.0/MIT) for rumble. No new crate needed; FastNoise Lite's Rust port (`fastnoise-lite`, MIT) is a reference.

## Size

Phase: medium to large (five milestones, three needing Lust changes).

## Sources

- Easing: [Robert Penner's easing functions](http://robertpenner.com/easing/).
- Shake: [Eiserloh, Juicing your cameras with math (GDC 2016)](https://gdcvault.com/play/1023146/Math-for-Game-Programmers-Juicing).
- Rumble: [gilrs README (force feedback on Windows)](https://docs.rs/crate/gilrs/0.11.2/source/README.md).
- Noise: [FastNoise Lite (Rust port, MIT)](https://lib.rs/crates/fastnoise-lite), [noise-functions crate](https://docs.rs/crate/noise-functions/0.3.0).
- Statecharts (entry and exit actions): [Harel, Statecharts (1987)](https://www.inf.ed.ac.uk/teaching/courses/seoc/2005_2006/resources/statecharts.pdf).
- Lust tweens and their gaps: decision 0106 in the repository.
