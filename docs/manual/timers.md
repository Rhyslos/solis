# Timers

> Waits your game code starts by name: "open in 2 seconds", "every 5 seconds", "each day at six". When one finishes, the component that started it hears `TimerFinished`.

## When to use it
Whenever something should happen **later**, or **again and again**: a door that opens two seconds after its switch is pulled, a spawner that sends a wave every five seconds, a rooster that crows each morning at six, a trap that resets after a while. A [timer](concepts.md) is a named wait: you start it, game time passes, and when it's done the component that started it is told.

Why not count seconds yourself, in a field? You can (`left = left - dt;`), but a timer does three things that are easy to get wrong:

- **It counts game time, exactly.** A timer's seconds become whole [ticks](concepts.md) once, as it starts, and Solis counts them in whole numbers, so two seconds is always exactly 120 ticks at normal speed: no tiny rounding that builds up over a long game. It follows the game's speed (`World:setGameSpeed`): at half speed it takes twice as many ticks, and while the speed is 0 (paused) it doesn't count at all.
- **It's saved.** A save made while a timer waits keeps it, and the loaded game finishes it on exactly the tick the saved one would have.
- **It's in order.** Timers finishing in the same tick come in a fixed order (see [Order](#order)), so the game comes out the same on any computer and any number of cores.

> **Good to know:** for a value that should *glide* to a new one over time (a door swinging open), use a [tween](game-code.md#tweens). Timers are for *when* something happens; tweens for *how* a value moves.

## How to use it

### Start a timer, and hear it finish
Inside a component, make the timer with `new`, saying how many seconds to wait: `new DoorOpens { after = 2.0 };`. Making something is always `new` in game code, a timer as much as a sound or a shot, and each timer is a class Solis writes into `solis/made.lst`, which you fetch by name. When the time has passed, the component gets the event `TimerFinished`, carrying which timer it was:

```lust
-- door.lst: a door that opens two seconds after its switch is pulled.
fetch "solis/world":World;
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":DoorOpens;

event SwitchPulled();

component Door {
    has {
        bool open = false;
    }

    on {
        -- The switch sends this; the door waits two seconds.
        SwitchPulled() {
            new DoorOpens { after = 2.0 };
            World:say("The door rumbles...");
        }

        -- Two seconds of game time later.
        TimerFinished(Timer which) {
            is which == Timer.DoorOpens? then {
                open = true;
                World:say("The door opens.");
            }
        }
    }
}
pass Door, SwitchPulled;
end
```

```rust
use reflect::Value;
use script::{Context, Event, RustComponent, Timing};

pub struct Door {
    open: bool,
}

impl RustComponent for Door {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        let me = cx.me();
        match event.name.as_str() {
            // The switch sends this; the door waits two seconds.
            "SwitchPulled" => {
                cx.host()
                    .start_timer(me, "DoorOpens", Timing::After(2.0))
                    .expect("a door's timer is its own");
                cx.host().say("The door rumbles...");
                true
            }
            // Two seconds of game time later.
            "TimerFinished" => {
                let which = match event.values.first() {
                    Some(Value::Text(name)) => name.as_str(),
                    _ => "",
                };
                if which == "DoorOpens" {
                    self.open = true;
                    cx.host().say("The door opens.");
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
local Door = component("Door", {
    { "open", "bool", false },
})

-- The switch sends this; the door waits two seconds.
function Door:on_SwitchPulled()
    solis.entity.startTimer(self.me, "DoorOpens", 2.0)
    solis.World:say("The door rumbles...")
end

-- Two seconds of game time later.
function Door:on_TimerFinished(which)
    if which == "DoorOpens" then
        self.open = true
        solis.World:say("The door opens.")
    end
end
```

**Naming a timer.** There's no list to keep anywhere: **the `new` that makes a timer names it.** Solis reads your Lust, finds every `new` of a name that isn't anything else with `after`, `every` or `atHour` in its braces, and writes each as a class into `solis/made.lst` (`new DoorOpens { … }`, fetched with `fetch "solis/made":DoorOpens;`) and as one of the game's `Timer` names into `solis/project.lst` (`Timer.DoorOpens`, for `TimerFinished`, `stopTimer` and the rest, fetched with `fetch "solis/project":Timer;`). A timer's name can't be one the game already uses (a component called `DoorOpens`, say): the Console says so, and you give the timer another. So a name misspelt anywhere else (`stopTimer(Timer.DoorOpnes)`, `is which == Timer.DorOpens?`) is a mistake in [Problems](problems.md) before the game runs. In Lua and Rust a timer is its name as text (`"DoorOpens"`): a capital letter first, then letters, digits and `_`.

**A timer is its component's own.** Two components can each have a timer called `Wave`, even on the same entity, and they never mix: `TimerFinished` goes only to the component that started the timer, and the other components on the entity don't hear it. That's also why a timer is made **inside the component's own code**, and belongs to the entity whose code makes it: made in a game-wide object's update, where no entity's code runs, it's a mistake that stops with its line.

**Starting a timer that's already running starts it again** from the new time: the old wait is forgotten. So start a timer once, when the thing happens (the switch is pulled), not every tick: started every tick, it never finishes.

**How long:** the seconds are rounded to the nearest tick (a 60th of a second). `0` finishes on the next tick, never in the same one. The longest wait is about 115 days of game time.

### Repeat every few seconds
`every` finishes again and again, until you stop it:

```lust
-- spawner.lst: a wave every five seconds, until the tenth.
fetch "solis/world":World;
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":[Wave, Drone];

component Spawner {
    has {
        int waves = 0;
        bool started = false;
    }

    method update(float dt) {
        -- Start once, on the first tick.
        is not started? then {
            new Wave { every = 5.0 };
            started = true;
        }
    }

    on {
        TimerFinished(Timer which) {
            waves = waves + 1;
            entity drone = new Drone { position = position() + [0, 0, 2] };

            -- Ten waves, then no more.
            is waves >= 10? then {
                stopTimer(Timer.Wave);
            }
        }
    }
}
pass Spawner;
end
```

```rust
use script::{Context, Event, RustComponent, Timing};

pub struct Spawner {
    waves: i64,
    started: bool,
}

impl RustComponent for Spawner {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        // Start once, on the first tick.
        if !self.started {
            let me = cx.me();
            cx.host()
                .start_timer(me, "Wave", Timing::Every(5.0))
                .expect("a spawner's timer is its own");
            self.started = true;
        }
    }

    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "TimerFinished" {
            return false;
        }
        let me = cx.me();
        self.waves += 1;
        let place = cx.host().position(me).unwrap_or_default();
        let _drone = cx.host().spawn("drones/drone", place + glam::Vec3::Z * 2.0);

        // Ten waves, then no more.
        if self.waves >= 10 {
            cx.host().stop_timer(me, "Wave").expect("its own timer");
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Spawner = component("Spawner", {
    { "waves", "int", 0 },
    { "started", "bool", false },
})

function Spawner:update(dt)
    -- Start once, on the first tick.
    if not self.started then
        solis.entity.startTimerEvery(self.me, "Wave", 5.0)
        self.started = true
    end
end

function Spawner:on_TimerFinished(which)
    self.waves = self.waves + 1
    local place = solis.entity.position(self.me)
    local drone = solis.World:spawn("Drone", place + vec3(0, 0, 2))

    -- Ten waves, then no more.
    if self.waves >= 10 then
        solis.entity.stopTimer(self.me, "Wave")
    end
end
```

A repeating timer keeps its **rhythm**: each turn is counted from the last turn, not from when your code handled it, so it never drifts. At a game speed so fast that several turns pass in one tick, it finishes once that tick and carries on in step. The shortest repeat is one tick.

### Each day at a time of day
With [day and night](sky-fog-and-exposure.md) on the Environment, the game has a clock. `atHour` finishes each day as that clock passes an hour (`6.5` is half past six), until you stop it:

```lust
-- rooster.lst: crows each morning, and a bed that skips to the morning.
fetch "solis/world":World;
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":Dawn;

component Rooster {
    has {
        bool started = false;
    }

    method update(float dt) {
        is not started? then {
            new Dawn { atHour = 6.0 };
            started = true;
        }
    }

    on {
        TimerFinished(Timer which) {
            World:say("Cock-a-doodle-doo!");
        }
    }
}

component Bed {
    -- Sleeping: the clock jumps to seven the next morning.
    method sleep() {
        World.timeOfDay = 7.0;
    }
}
pass Rooster, Bed;
end
```

```rust
use script::{Context, Event, RustComponent, Timing};

pub struct Rooster {
    started: bool,
}

impl RustComponent for Rooster {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        if !self.started {
            let me = cx.me();
            cx.host()
                .start_timer(me, "Dawn", Timing::AtHour(6.0))
                .expect("a rooster's timer is its own");
            self.started = true;
        }
    }

    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "TimerFinished" {
            return false;
        }
        cx.host().say("Cock-a-doodle-doo!");
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
    // A bed calls cx.host().set_time_of_day(7.0) to skip to the morning.
}
```

```lua
local Rooster = component("Rooster", {
    { "started", "bool", false },
})

function Rooster:update(dt)
    if not self.started then
        solis.entity.startTimerAtHour(self.me, "Dawn", 6.0)
        self.started = true
    end
end

function Rooster:on_TimerFinished(which)
    solis.World:say("Cock-a-doodle-doo!")
end

-- A bed calls solis.World:setTimeOfDay(7.0) to skip to the morning.
```

**When the clock jumps.** Sleeping in a bed (`World.timeOfDay = 7.0` at ten at night) jumps the clock past six: the rooster crows **once**, on the next tick, not once for every hour skipped. Midnight works the same way: a timer at 23:30 or at 0:00 finishes as the clock goes round. Setting the clock always moves the day *forward* to that hour, so setting it from seven back to five is the day going on to five the next morning, which comes before six: the rooster crows when six comes.

### Stop a timer, and ask about one
| Call | What it does |
|---|---|
| `new DoorOpens { after = seconds };` | Finishes once, that many seconds of game time from now. |
| `new DoorOpens { every = seconds };` | Finishes every that many seconds, until stopped. |
| `new DoorOpens { atHour = hour };` | Finishes each day as the day's clock passes that hour, until stopped. |
| `stopTimer(Timer.DoorOpens)` | Stops it, so it doesn't finish. One that isn't running changes nothing. |
| `timerRunning(Timer.DoorOpens)` | Whether it's running: `is timerRunning(Timer.DoorOpens)? then { … }`. |
| `timeLeft(Timer.DoorOpens)` | The seconds of game time left; 0 when it isn't running. |
| `TimerFinished(Timer which)`, in the component's `on { }` | Heard when one of this component's timers finishes. |

A `new` takes exactly one of `after`, `every` and `atHour`: none, or two, stops with its line. Starting and stopping land at the **end of the tick**, as moves do, and `timerRunning` and `timeLeft` answer as the tick started: a timer started this tick is running from the next. In Lua they're `solis.entity.startTimer(self.me, "DoorOpens", 2.0)` and so on; in Rust, through the component's host: `start_timer`, `stop_timer` and `timer_left` (which gives `None` when the timer isn't running).

### Order
Timers that finish in the same tick are heard in the tick's **first round of events**, after the components' `update`s, in a fixed order: the one due earliest first, then by entity (in the order of their IDs), then by component (by name), then in the order they were started. So the same game comes out the same on one core or sixteen.

### Timers, saves and new code
- **Saves:** a [save game](save-games.md) keeps every timer waiting; loaded, each finishes on the tick it would have.
- **When an entity goes:** despawning an entity, or taking the component off it, ends its timers quietly: they never finish.
- **When its state changes:** a timer a Lust component starts while in a state of its own (`state = Mood.Stunned;`) belongs to that state, and ends quietly when the component leaves it: its `TimerFinished` never comes. `keepStarted()` lets it run on. See [States that start and stop](states.md).
- **New code while playing:** a timer keeps running while its component is still in the code and its Lust still starts a timer of that name. Rename the timer, or take the component out, and the timers that no longer fit stop, with one note in the [Console](console.md) saying which.

### Steps in order
"Walk to the well, wait a second, say hello, wait two seconds, walk home": a character doing things one after another, waiting in between. Write it with **one timer** and a **field for the step** it's on. Each wait starts the timer; when it finishes, the code moves on to the next step:

```lust
-- messenger.lst: walks to the well, waits, says hello, waits, walks home.
fetch "solis/world":World;
fetch "solis/project":Timer;
fetch "solis/timers":TimerFinished;
fetch "solis/made":NextStep;

state Step = [WalkThere, WaitThere, WaitAfterHello, WalkHome, Home];

component Messenger {
    has {
        Step step = Step.WalkThere;
        vec3 well = [10, 0, 0];
        vec3 home = [0, 0, 0];
        float speed = 2;
    }

    method update(float dt) {
        -- Walking takes every tick; waiting is the timer's.
        is step == Step.WalkThere && arrived(well, dt)? then {
            step = Step.WaitThere;
            new NextStep { after = 1.0 };
        }
        is step == Step.WalkHome && arrived(home, dt)? then {
            step = Step.Home;
        }
    }

    on {
        -- Each wait ends here, and the next step begins.
        TimerFinished(Timer which) {
            is step == Step.WaitThere? then {
                World:say("Hello!");
                step = Step.WaitAfterHello;
                new NextStep { after = 2.0 };
            } or is step == Step.WaitAfterHello? then {
                step = Step.WalkHome;
            }
        }
    }

    -- Moves a step towards a place; true once it's there.
    method bool arrived(vec3 place, float dt) {
        vec3 way = place - position();
        float far = way:length();
        float stride = speed * dt;
        is far <= stride? then {
            moveTo(place);
            return true;
        }
        moveTo(position() + way * (stride / far));
        return false;
    }
}
pass Messenger;
end
```

```rust
use glam::Vec3;
use script::{Context, Event, RustComponent, Timing};

#[derive(PartialEq)]
enum Step {
    WalkThere,
    WaitThere,
    WaitAfterHello,
    WalkHome,
    Home,
}

pub struct Messenger {
    step: Step,
    well: Vec3,
    home: Vec3,
    speed: f32,
}

impl Messenger {
    /// Moves a step towards a place; true once it's there.
    fn arrived(&self, cx: &mut Context<'_>, place: Vec3, dt: f64) -> bool {
        let me = cx.me();
        let now = cx.host().position(me).unwrap_or_default();
        let way = place - now;
        let stride = self.speed * dt as f32;
        if way.length() <= stride {
            cx.host().move_to(me, place);
            return true;
        }
        cx.host().move_to(me, now + way * (stride / way.length()));
        false
    }
}

impl RustComponent for Messenger {
    fn update(&mut self, cx: &mut Context<'_>, dt: f64) {
        let me = cx.me();
        // Walking takes every tick; waiting is the timer's.
        if self.step == Step::WalkThere && self.arrived(cx, self.well, dt) {
            self.step = Step::WaitThere;
            cx.host()
                .start_timer(me, "NextStep", Timing::After(1.0))
                .expect("its own timer");
        }
        if self.step == Step::WalkHome && self.arrived(cx, self.home, dt) {
            self.step = Step::Home;
        }
    }

    // Each wait ends here, and the next step begins.
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "TimerFinished" {
            return false;
        }
        let me = cx.me();
        if self.step == Step::WaitThere {
            cx.host().say("Hello!");
            self.step = Step::WaitAfterHello;
            cx.host()
                .start_timer(me, "NextStep", Timing::After(2.0))
                .expect("its own timer");
        } else if self.step == Step::WaitAfterHello {
            self.step = Step::WalkHome;
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Messenger = component("Messenger", {
    { "step", "string", "WalkThere" },
    { "well", "vec3", vec3(10, 0, 0) },
    { "home", "vec3", vec3(0, 0, 0) },
    { "speed", "float", 2 },
})

-- Moves a step towards a place; true once it's there.
function Messenger:arrived(place, dt)
    local now = solis.entity.position(self.me)
    local way = place - now
    local far = math.sqrt(way.x * way.x + way.y * way.y + way.z * way.z)
    local stride = self.speed * dt
    if far <= stride then
        solis.entity.moveTo(self.me, place)
        return true
    end
    solis.entity.moveTo(self.me, now + way * (stride / far))
    return false
end

function Messenger:update(dt)
    -- Walking takes every tick; waiting is the timer's.
    if self.step == "WalkThere" and self:arrived(self.well, dt) then
        self.step = "WaitThere"
        solis.entity.startTimer(self.me, "NextStep", 1.0)
    end
    if self.step == "WalkHome" and self:arrived(self.home, dt) then
        self.step = "Home"
    end
end

-- Each wait ends here, and the next step begins.
function Messenger:on_TimerFinished(which)
    if self.step == "WaitThere" then
        solis.World:say("Hello!")
        self.step = "WaitAfterHello"
        solis.entity.startTimer(self.me, "NextStep", 2.0)
    elseif self.step == "WaitAfterHello" then
        self.step = "WalkHome"
    end
end
```

Why a field and not "wait 1 second" in the middle of the code? Because the step is a field, it's **saved**: a save made while the messenger waits at the well loads with it still waiting there, and saying hello a second later. Code paused halfway through a method can't be saved, so Solis has no `wait` for game code.

> **Good to know:** later, Lust will let you write the waits where they happen, as blocks run later: `after 1.0 { World:say("Hello!"); }`. That will make steps in order much shorter. Such a block may use only the component's own fields, so it can be saved just like this one.

## Settings
Timers have no settings of their own: they're started and stopped by game code. A time-of-day timer uses the Environment's **Day and night** clock (**A day lasts**, **Starts at**), as [Sky, fog and exposure](sky-fog-and-exposure.md) describes.

## Shortcuts
Timers have no shortcuts.

## Tips and common problems
- **Problem:** a timer never finishes. **Why:** it's made again every tick (a `new` in `update` with nothing around it), so each start pushes it back. **Fix:** start it once, when the thing happens, or check `timerRunning` first.
- **Problem:** Problems says `Timer.DoorOpnes` isn't a Timer. **Why:** no `new` in the game's Lust makes it: a typo, or the timer was renamed. **Fix:** use the name its `new` gives, or make the timer under that name.
- **Problem:** Problems says a timer's name isn't a `Timer`, though it's set in a variable first (`Timer next = Timer.Later;`). **Why:** only a `new` names a timer. **Fix:** start it by name somewhere (`new Later { after = … }`), even if other code chooses which to start.
- **Problem:** a timer stopped when the code changed while playing. **Why:** its name, or its component, is gone from the new code; the Console says which. **Fix:** start it again, under its new name.
- **Problem:** a timer stopped, and its `TimerFinished` never came, though nothing stopped it. **Why:** it was started while its component was in a state of its own, and the state changed. **Fix:** start it after setting the new state, or call `keepStarted()` after starting it ([States that start and stop](states.md)).
- **Problem:** `timerRunning` says no just after the timer's `new`. **Why:** starts land at the end of the tick, so every core sees the same timers during it. **Fix:** nothing to fix: from the next tick it says yes.
- **Problem:** a timer's `new` stops with "a timer belongs to an entity". **Why:** it was made where no entity's code runs: a game-wide object's update. **Fix:** send an entity an event, and let its own component make the timer.
- **Problem:** Problems says a timer "is in solis/made.lst". **Why:** the file makes it with `new` without fetching it. **Fix:** add `fetch "solis/made":DoorOpens;` at the top, as the message says.

## Related
- [Game code](game-code.md): components, events, the tick, tweens and game-wide objects.
- [States that start and stop](states.md): timers that end with their state.
- [Save games](save-games.md): what a save keeps.
- [Sky, fog and exposure](sky-fog-and-exposure.md): the day and night clock.
- [Concepts](concepts.md): timer, tick, event, state.
