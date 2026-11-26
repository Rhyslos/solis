# States that start and stop

> A component's own state, such as Calm or Stunned, ends what it started while in it: its timers, tweens and looping sounds stop by themselves when the state changes, and the component hears the change.

## When to use it
Whenever a character, a door or a machine is in one of a few **modes**, and each mode starts things of its own. A stunned character flashes, plays a dizzy sound on a loop and starts a timer to recover. When the stun ends (because the timer finished, or because something knocked it out of the stun early), the flash, the sound and the timer must all stop. Written by hand, that means remembering to stop each one wherever the stun can end, and a forgotten one shows up as a character still flashing, or recovering from a stun it already left.

A Lust component's own [state](concepts.md) does that bookkeeping for you: **what it starts while in an option ends when it leaves that option.** It's the same rule Lust already has for objects a class makes in its state (things made in a state end with it), carried over to what the engine runs: timers, tweens and looping sounds.

> **Good to know:** this is a Lust component's own state, the one written `state = Mood.Calm;` in its `has`. A field that holds a state (`Mood mood = Mood.Calm;`) is an ordinary value: changing it ends nothing. Lua and Rust components have no state of their own (see [In Lua and Rust](#in-lua-and-rust)).

## How to use it

### Enter a state in the code that sets it
There's nothing new to learn to enter a state: the code that sets `state = Mood.Stunned;` does the entering work right there, on the next lines. Whatever it starts (a timer, a tween, a sound) belongs to `Stunned`, because that's the component's state at the moment it starts:

```lust
-- hero.lst: stunned by a hit, recovering a second and a half later.
fetch "solis/world":World;
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":[Recover, Dizzy];

state Mood = [Calm, Stunned];
event Hit(float damage);
event MoodChanged(Mood from, Mood to);

component Hero {
    has {
        state = Mood.Calm;
        float flash = 0;
    }

    on {
        -- A hit stuns: everything started here belongs to Stunned.
        Hit(float damage) {
            state = Mood.Stunned;
            new Recover { after = 1.5 };
            tween flash to 1 over 1.5;
            new Dizzy {};
        }

        -- The stun is over: setting the state ends the rest.
        TimerFinished(Timer which) {
            is which == Timer.Recover? then {
                state = Mood.Calm;
            }
        }

        -- Leaving work in one place, whoever changed the state.
        MoodChanged(Mood from, Mood to) {
            is from == Mood.Stunned? then {
                flash = 0;
                World:say("Back on my feet.");
            }
        }
    }
}
pass Hero, Hit, MoodChanged;
end
```

```rust
// A Rust component has no state of its own, so nothing ends by itself:
// keep the mode in a field, and stop what it started yourself.
use script::{Context, Event, RustComponent, SoundAt, Timing};

#[derive(PartialEq)]
enum Mood {
    Calm,
    Stunned,
}

pub struct Hero {
    mood: Mood,
}

impl Hero {
    /// Leaves the stun: stops what it started.
    fn recover(&mut self, cx: &mut Context<'_>) {
        let me = cx.me();
        self.mood = Mood::Calm;
        cx.host().stop_timer(me, "Recover").expect("its own timer");
        cx.host().stop_sounds_on(me);
        cx.host().say("Back on my feet.");
    }
}

impl RustComponent for Hero {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        let me = cx.me();
        match event.name.as_str() {
            // A hit stuns.
            "Hit" => {
                self.mood = Mood::Stunned;
                cx.host()
                    .start_timer(me, "Recover", Timing::After(1.5))
                    .expect("its own timer");
                cx.host().play_sound("sfx/dizzy", SoundAt::On(me));
                true
            }
            // The stun is over.
            "TimerFinished" => {
                if self.mood == Mood::Stunned {
                    self.recover(cx);
                }
                true
            }
            _ => false,
        }
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
-- A Lua component has no state of its own, so nothing ends by itself:
-- keep the mode in a field, and stop what it started yourself.
local Hero = component("Hero", {
    { "mood", "string", "Calm" },
})

-- Leaves the stun: stops what it started.
function Hero:recover()
    self.mood = "Calm"
    solis.entity.stopTimer(self.me, "Recover")
    solis.entity.stopSounds(self.me)
    solis.World:say("Back on my feet.")
end

-- A hit stuns.
function Hero:on_Hit(damage)
    self.mood = "Stunned"
    solis.entity.startTimer(self.me, "Recover", 1.5)
    solis.entity.playSound(self.me, "Dizzy")
end

-- The stun is over.
function Hero:on_TimerFinished(which)
    if self.mood == "Stunned" then
        self:recover()
    end
end
```

Here the stun ends when `Recover` finishes. But if something else changes the state first (a second hit that knocks the hero out, say, `state = Mood.KnockedOut;`), the stun's timer, its flash and its dizzy loop all stop at that moment, with nothing more to write.

### What ends with the state, and what doesn't
While a component is in an option of its own state, these belong to that option and **end when it leaves it**:

| Started in the state | When the state changes |
|---|---|
| A timer (`new DoorOpens { after = … }`, with `every` or `atHour` too) | It stops, and `TimerFinished` never comes. Even one that was due this very tick isn't heard once the state has changed. |
| A [tween](game-code.md#tweens) (`tween flash to 1 over 1.5;`) | It stops where it is: the value keeps what it had reached, and its done block doesn't run. |
| A sound set to **Loop** ([Sound events](sound-events.md)), made with `new` (`new Engine {};`) | It fades out. A sound that plays once finishes as usual. |

And these **don't**:

- **Spawned entities stay.** A smoke puff [spawned](game-code.md#making-and-removing-entities) while stunned is an entity of its own, and goes on after the stun ends. Despawn it yourself if it should go.
- **What's started outside the component's methods** belongs to no state: in a tween's done block, or in a [game-wide object](game-code.md#game-wide-objects).
- **What a component without a state of its own starts** ends only as it always has.

The moment that counts is the call: a timer belongs to the option the component is in **when the timer starts**, the value its code has just set. Starting things and then setting the state means they belong to the state before.

### Options inside options
A state's options can hold options of their own, and each counts separately, as in Lust:

```lust
-- fighter.lst: a stance inside combat.
fetch "solis/made":[Battle, Swing];

state Mood = [Calm, Combat [Melee, Ranged]];

component Fighter {
    has {
        state = Mood.Calm;
    }

    method enterCombat() {
        state = Mood.Combat;
        new Battle {};
    }

    method closeIn() {
        state = Mood.Combat.Melee;
        new Swing { every = 0.8 };
    }

    method backOff() {
        state = Mood.Combat.Ranged;
    }

    method calmDown() {
        state = Mood.Calm;
    }
}
pass Fighter;
end
```

```rust
// Rust components have no state of their own, so options inside
// options are Lust's: in Rust, stop each thing when its mode ends.
use script::{Context, RustComponent};

pub struct Fighter;

impl RustComponent for Fighter {
    fn update(&mut self, _cx: &mut Context<'_>, _dt: f64) {}

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
-- Lua components have no state of their own, so options inside
-- options are Lust's: in Lua, stop each thing when its mode ends.
local Fighter = component("Fighter", {
    { "mood", "string", "Calm" },
})
```

- `backOff()` (from `Combat.Melee` to `Combat.Ranged`): the swings started in `Melee` stop; the battle music started in `Combat` goes on, since `Ranged` is still inside `Combat`.
- Moving out to `Combat` itself (from `Combat.Melee`) ends nothing: `Melee` is inside it.
- `calmDown()` (leaving `Combat` altogether): what began in `Combat` and in either stance ends.

### Keep something going: `keepStarted()`
Sometimes a thing started in a state should outlive it: a door's slow swing that should finish even after the door is locked, or battle music that should play on into the victory. Call `keepStarted()` after starting it: what the component has started in its state **so far** no longer belongs to the state, and carries on when it changes. Things started after the call belong to the state as usual:

```lust
-- gate.lst: the gate keeps swinging after it's locked; the alarm doesn't.
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":[Alarm, Lock];

state Gate = [Shut, Opening, Locked];

component Guard {
    has {
        state = Gate.Shut;
        float swing = 0;
    }

    method open() {
        state = Gate.Opening;
        tween swing to 90 over 3.0;
        keepStarted();

        -- Started after keepStarted(): ends with Opening.
        new Alarm {};
        new Lock { after = 1.0 };
    }

    on {
        TimerFinished(Timer which) {
            state = Gate.Locked;
        }
    }
}
pass Guard;
end
```

```rust
// Rust components have no state of their own, so what they start never
// ends with one: keep_started() keeps nothing, and is there so the three
// languages share the same calls.
use script::{Context, RustComponent};

pub struct Guard;

impl RustComponent for Guard {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        let me = cx.me();
        cx.host().keep_started(me).expect("its own");
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
-- Lua components have no state of their own, so what they start never
-- ends with one: keepStarted keeps nothing, and is there so the three
-- languages share the same calls.
local Guard = component("Guard", {})

function Guard:update(dt)
    solis.entity.keepStarted(self.me)
end
```

Like timers, `keepStarted()` is called bare, inside the component: it keeps what *this* component started. Called on another entity, or in a game-wide object or a tween, it stops with its line.

### Hear the state change
To hear its own state change, a component's game declares an event for the state, named after it with `Changed` on the end and carrying the option before and the option now:

```text
state Mood = [Calm, Stunned];
event MoodChanged(Mood from, Mood to);
```

Then every component whose own state is a `Mood` hears `MoodChanged` when its state changes, however it changed: in its own `update`, in a handler, or in a tween's done block. It's the place for **leaving work** (put the sword away, stop glowing), kept in one handler instead of copied wherever the state can change (see `on MoodChanged` in the first example).

- **When:** the change is noticed as the tick ends, after everything in it has landed, and heard in the **next tick's first round of events**: after the timers' `TimerFinished`, in order of entity, then component name.
- **Once a tick:** if the state changes several times in one tick, the event says the first `from` and the last `to`. If it ends the tick where it began (stunned and back), nothing is heard.
- **Only the component whose state changed hears it,** not the others on its entity.
- **What ends, ends first.** The timers, tweens and loops of the options it left have already stopped by the time the event arrives.
- **Declared the wrong way** (`event MoodChanged(int from);`), it's a mistake in [Problems](problems.md) before the game runs, saying how to declare it.

Why one event per state, and not one `StateChanged` for all? In Lust an event carries the same kinds everywhere in the game, so one event can't carry a `Mood` for the hero and a `Gate` for the guard. Naming it after its state lets each carry its own, checked before the game runs.

### Saves, and new code while playing
- **Saves:** a [save game](save-games.md) keeps which state each timer belongs to, and a change waiting to be heard. Loaded, the game hears exactly what the saved one would have (never a change twice), and its timers still end when their state changes. Tweens aren't kept in saves yet, whatever their state.
- **Loading or editing values while playing** sets a component's state without "changing" it: nothing ends, and nothing is heard.
- **New code while playing** keeps each component's state where it can. Tweens stop with new code, as they always have.

### In Lua and Rust
Lua and Rust components have no state of their own: a Lua component's fields are plain values Solis doesn't see change, and a Rust component's are its own. So what they start belongs to no state, and ends only when they stop it, or when the entity goes. They have `keepStarted` too (`solis.entity.keepStarted(self.me)`, `cx.host().keep_started(me)`), which keeps nothing for them: it's there so the three languages share the same calls. Their examples above show the same game written by hand, stopping each thing where the mode ends.

## Settings
States have no settings of their own: they're written in game code.

## Shortcuts
States have no shortcuts.

## Tips and common problems
- **Problem:** a timer stopped before it finished, and `TimerFinished` never came. **Why:** the component's state changed after it started, so it ended with it. **Fix:** start it after setting the new state, or call `keepStarted()` after starting it.
- **Problem:** nothing stops when the state changes. **Why:** the "state" is a field (`Mood mood = Mood.Calm;`), not the component's own state. **Fix:** write it `state = Mood.Calm;` in the `has`, and set it with `state = …;`.
- **Problem:** `MoodChanged` never comes. **Why:** the game doesn't declare it, or the state changed and changed back in the same tick. **Fix:** declare `event MoodChanged(Mood from, Mood to);` beside the state.
- **Problem:** a looping sound kept playing after the state changed. **Why:** it was started before the state was set, so it belongs to the state before, or `keepStarted()` came after it. **Fix:** set the state first, then start the sound.
- **Problem:** a tween's value stopped halfway. **Why:** the state changed while it ran, so it stopped where it was. **Fix:** set the value yourself in the leaving work (`on MoodChanged`), or keep the tween with `keepStarted()`.

## Related
- [Timers](timers.md): waits that end with their state.
- [Game code](game-code.md): components, events, tweens and the tick.
- [Sound events](sound-events.md): which sounds loop.
- [Save games](save-games.md): what a save keeps.
- [Concepts](concepts.md): state, timer, tween, event.
