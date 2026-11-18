# 0125. Named timers now, blocks run later when Lust has them

- **Status:** Accepted (builds on 0033, 0035, 0038, 0065, 0092, 0106 and 0122)
- **Date:** 2026-11-18

## Problem

Stage 1's next milestone is timers and steps in order (decision 0122; [report 05](../reports/after-the-rethink/05-shared-3.md), M1): "open in 2 seconds", "every 5 seconds", "each day at six", "walk there, wait, say this". Solis's rules are strict: game state is saved, checksummed, the same on one core or sixteen, and replays match (decisions 0033, 0035, 0038). A block of code run later (`after 2.0 { … }`) is a closure, which can't be saved; a `wait` inside a method is a coroutine, which can't be saved either.

The project lead's answers (2026-11-18):

1. **Named timers now;** blocks run later (`after`, `every`) come in a later milestone, once Lust has them.
2. **Blocks may use only the component's own fields** (and `this`), so saves keep them.
3. **Timer names come from the component's own code,** a local state (`state Timer = [DoorOpens, Respawn];`), with no project-wide list. The engine keeps a timer by (entity, component, name), so renaming one is like renaming a field.
4. **A clock jumped past a timer's hour fires it once,** not once per day skipped.

## What Lust allows today

Answer 3 can't be built with today's Lust (alpha 74), for two reasons, each checked against the compiler:

- **An engine call's types are its own file's.** `method entity:startTimer(Timer which, float seconds) native;` lives in Solis's `solis/timers.lst`; its `Timer` must be a state that file can see. Lust has no "any state" type, and a component's own `state Timer` is private to its file ("Timer is in door.lst, which keeps it to itself").
- **An event is one name for the whole program, carrying fixed types.** `TimerFinished(Timer which)` can be declared once ("another file has an event called TimerFinished too"), so it can carry only one `Timer`.

The other shapes considered:

1. **A file Solis writes for each component file** (`solis/timers/door.lst`, fetching the component's `Timer` back, with a class or methods of its own). Lust allows the two-way fetch, but the event can't be shared, so each component would hear its own `DoorTimerFinished`, and every file would fetch a file Solis made from it. Heavy for someone learning, and a second shape beside every other engine call.
2. **A timer as a field the engine fills in** (`Timer doorOpens;`, `doorOpens:start(2.0)`): names local, renaming truly like a field. But an event can't carry a class, so `TimerFinished` couldn't say which timer finished, and Lua and Rust have no filled-in fields.
3. **A project-wide list** (Project settings › Timers, as tags): the lead said no list; and it needs editor UI, which this milestone doesn't build.
4. **The game's `Timer` state made by Solis from where its Lust starts timers** (chosen): no list anywhere, every call and the event exactly as the research wrote them, and a misspelt name anywhere but the start is a mistake before the game runs.

## Decision

- **Named timers, on the component:** `startTimer(Timer.DoorOpens, 2.0)`, `startTimerEvery(Timer.Wave, 5.0)`, `startTimerAtHour(Timer.Dawn, 6.0)`, `stopTimer(Timer.X)`, `timerRunning(Timer.X)`, `timeLeft(Timer.X)`, and `on TimerFinished(Timer which)`, heard only by the component that started the timer. Called bare inside a component; on another entity, in a game-wide object or in a tween, a mistake with its line. Lua and Rust name a timer by its text (`"DoorOpens"`), as they name the project's other names. The research's names stand: they fit the guidelines' shapes (verbs, a variant's difference at the end, `timerRunning` read after `is`, `timeLeft` a noun, a past-tense event carrying a state).
- **Names come from the code that starts them.** `Timer` is a state in `solis/project.lst`, its options every name the game's Lust gives `startTimer`, `startTimerEvery` or `startTimerAtHour` (`script::timer_names`), gathered as the engine is described, before the code is checked. A timer is kept by (entity, component, name): two components' `Wave`s never mix.
- **Game time, exact in ticks:** seconds become whole ticks once, as a timer starts, rounded to the nearest; 0 finishes on the next tick, never in the same round; a repeating timer waits at least a tick; the longest is 10,000,000 s (about 115 days). Time is counted in whole numbers (65,536 to a tick, each tick adding the game's speed in those), so timers follow `setGameSpeed`, stand still at speed 0 and never drift. A repeating timer's next turn is its last plus its interval; fallen behind (several turns in one tick), it fires once and keeps its rhythm, as Lust's own `every` does.
- **Time of day:** a timer at an hour fires as the day's clock passes it, and once when `setTimeOfDay` jumps past it (answer 4), midnight alike. Setting the clock moves the day forward to that hour, as the world's clock already does, so setting it back from seven to five passes no six.
- **Order:** what's due fires in the tick's first round of events, after the engine's other events, in (due, entity, component name, start order); starts and stops land at the tick's end, so every phase and core asks the timers as the tick started.
- **Kept by the game** (`script::Game`): ended silently with their entity or component, kept by new code while their component exists and (for Lust) its code still starts a timer of that name, else dropped with one Console note. Saved (save format 11, a `timers` section; version 10 upgrades with none), in the checksum and the two-PC check's parts once a timer waits, so older recordings keep their sums.
- **Blocks later:** the store is pending work by (due, entity, component, start order), so blocks run later join it when Lust gives them stable IDs and owners. Until then, steps in order are one timer and a field for the step (the manual's Timers page); there's no kit helper and no `wait`.

## Consequences

- The lead's shape, a local `state Timer` in each component, needs Lust: a native taking the calling component's own state of a name, and an event carrying it. It is asked of Lust with the blocks (the plan's As built note). When Lust has it, games move their timer names into their components and nothing else changes: every call and handler reads the same.
- Only a start call names a timer: code that picks a timer through a variable must still start each by name somewhere. The manual says so.
- A Lua or Rust timer's name isn't a Lust option; `TimerFinished` reaches a Lua component with its name as text whatever Lust's `Timer` holds.
- 100,000 waiting timers with 10,000 finishing and starting again each tick cost the store 1.2 to 1.6 ms a tick in a release build (target 2 ms), plus about 0.9 ms making their events (`script` scale check).
