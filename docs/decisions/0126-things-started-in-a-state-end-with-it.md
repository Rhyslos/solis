# 0126. Things started in a state end with it, and the state's change is heard

- **Status:** Accepted (builds on 0033, 0038, 0065, 0106, 0122, 0123 and 0125)
- **Date:** 2026-11-18

## Problem

Stage 1's next milestone is states that know when they start and stop (decision 0122; [report 05](../reports/after-the-rethink/05-shared-3.md), M3): a character that's stunned flashes, plays a dizzy loop and starts a timer to recover, and everything started while stunned should stop by itself when the stun ends, however it ends. The research suggested `on enter Stunned { … }` and `on leave Stunned { … }` handlers, which would be new Lust words.

The project lead's answers (2026-11-18):

1. **No new Lust words** ("can't we use state for this?"): a component's own Lust state (`state = Mood.Calm;` in its `has`) is the tool. Entering is just code: the code that sets `state = Mood.Stunned;` does the entering work on the lines after it.
2. **Things started in a state end with it.** Lust already ends the objects a thing makes in a state when it leaves it (LUST.md §8); Solis does the same for each timer, tween and looping sound a component starts while in an option. A timer ended so doesn't fire `TimerFinished`; a tween stops where it is, without its done block; a looping sound stops. Options inside options count separately, as in Lust. A way to opt out, kept small.
3. **Spawned entities stay:** a state ending never removes one.
4. **A state-changed event:** game code hears when a component's own state changes, so leaving work can be kept in one place, whoever changed it. Noticed as the tick ends, heard in the next tick's first round after `TimerFinished`, once a tick (the first `from`, the last `to`, nothing when it ends where it began), carrying both options, typed.

## What Lust allows today

- **The event can't be one engine event.** An event is one name in the whole program with one set of types (decision 0125 found the same for `TimerFinished`), so a single `StateChanged` can't carry the hero's `Mood` and the guard's `Gate`. An event per state, named after it, can: `MoodChanged(Mood from, Mood to)`. Its declaration must see the state, which is the game's own, so the game declares it beside the state; Solis sends it. Checked against the compiler.
- **Lust's compiled code doesn't say which state a component's own is,** only its options (the schema's `state` field), so Solis reads the state's name from where the component sets its starting option (`state = Mood.Calm;` in its `has`), as decision 0125 reads timer names from their starts.
- **Lust's runtime is where a state changes** (its `setstate`, called for every change however written) and where tweens live (its `tween` and `tickTweens`). Neither tells the engine anything, and a tween can't be stopped from outside. Solis already adjusts Lust's runtime for exact maths (`exact_maths`); it adjusts it the same way here, on the lines that are there, so no line moves.

## Options

For the opt-out:

1. **A variant of every start** (`startTimerKept`, `playSoundKept`): many new calls, and a tween has no call to vary.
2. **Lust's own `keep`:** it takes an object; a timer, tween or sound isn't one.
3. **One call, `keepStarted()`** (chosen): what the component has started in its state so far no longer belongs to it, as Lust's `keep` holds onto an object. Things started after the call belong to the state as usual, so the order of the lines says which is which.

For noticing the change:

1. **Compare every component's state each tick:** simple, but it reads 10,000 states a tick to find the few that changed.
2. **Hear every change as it happens** (chosen): Lust's runtime tells Solis, through a hidden native, kept in the tick's order like any change; the game adds up each component's changes as the tick ends. Nothing is read for components whose state didn't change.

## Decision

- **A Lust component's own state is the tool:** no new Lust words; entering is the code that sets the state.
- **What it starts in an option belongs to that option:** its timers (`startTimer`, `startTimerEvery`, `startTimerAtHour`), its tweens and its sounds (`playSound`, `Audio:play`, `Audio:playAt`), each tagged with (entity, component, option) at the moment it starts, the value the code has just set. Leaving the option ends them at that moment, before any handler runs: a timer silently (a `TimerFinished` already made this tick isn't delivered once the state has changed), a tween where it is without its done block, a sound set to Loop with a short fade (a sound that plays once finishes). Options inside options count separately (`Combat.Melee` to `Combat.Ranged` ends Melee's things; moving out to `Combat` ends nothing; leaving `Combat` ends both). Spawned entities stay. What a component without a state of its own starts, or what's started outside a component's methods (a tween's done block, a game-wide object), belongs to no option.
- **`keepStarted()`** on the component (`solis/states.lst`, called bare) lets what it started so far carry on.
- **The event is named after the state** (`event MoodChanged(Mood from, Mood to);`, declared by the game beside the state): sent only to the component whose state changed, in the next tick's first round after the timers' events, by entity then component; the first `from` and the last `to` of the tick, nothing for a round trip. Only a component that handles it is followed, so one that doesn't listen pays nothing to notice. Declared otherwise, it's a mistake before the game runs, saying how to declare it.
- **Lua and Rust components have no state of their own:** a Lua component's fields are plain values Solis doesn't see change, and a Rust component's are its own. What they start belongs to no option. They have `keepStarted` too, which keeps nothing, so the three languages share the calls.
- **Game state:** each waiting timer's option, and the changes waiting to be heard, are saved (save format 12: ` in OPTION` on a timer's line, a `states` section; version 11 upgrades with none) and in the checksum once one exists, so recordings from before keep their sums. Loading sets components' states without changing them, so nothing is heard twice.

## Consequences

- A component's things end without a line of leaving code, and its leaving work (if any) sits in one `on MoodChanged` handler.
- Solis's adjustment of Lust's runtime is checked on every compile: a Lust whose runtime reads otherwise is a plain mistake ("a bug in Solis"), never a half change. Moving Solis to a newer Lust checks it again.
- A Lust component with a state of its own runs noted as the one running (a small wrapper around its `update` and handlers), so the engine knows its option when it starts something; components without one cost nothing more.
- Tweens still aren't saved (decision 0106), so a tween started in a state is lost with a load, as any tween is.
- Asked of Lust: an engine event that carries the receiving component's own state, typed by it, so one `StateChanged` works for every state without a declaration per state (the plan's As built note).
- 10,000 components with a state of their own, each with a timer and a tween waiting, 1,000 of them changing state each tick: Solis's own part is 4.8 to 6.6 ms a tick (release build, one core), over the 2 ms target, and the scale check asserts the 8 ms reached. All 10,000 changing every tick is reported only (46 to 54 ms). Each change crosses from Luau into Rust once; what Lust would need to do better is asked in the plan. Delivering events got quicker for every event (`TimerFinished` too): their values' strings and lists are made once a phase.
