# Game code

> Your game's own rules (how a turret aims, when a door opens) are written as code, as components you put on entities. Solis checks the code as you save, shows its fields in Properties, and runs it when you press Play.

## When to use it
Whenever your game needs to *do* something: react to the player, count score, move things on its own. Areas and assemblies say what's in the world; [game code](concepts.md#game-code) says what happens in it.

Solis games can be written in three languages:
- **[Lust](concepts.md#game-code)**, made for Solis's kind of games and the one this manual teaches first;
- **Lua**, if you know it from Roblox or modding;
- **Rust**, the language Solis itself is written in.

Not sure which? [Which language when](which-language.md) compares them, with numbers.

**Right now:** Lust and Lua components work: they appear in Properties, run in play mode, talk to each other through events, and take new code while the game runs. Rust components can join them, but for now only in a Solis built from source.

## How to use it
1. Open a [project](projects-and-the-library.md). Solis puts a `solis` folder in its `assets`. Everything in it is written by Solis for your code to read, and written again whenever the project opens, so changes you make there don't last:
   - a Lust file for each part of the engine (`world.lst`, `entity.lst`, `input.lst` and so on). They describe what your code can ask of the engine, such as `World:say` (write in the Console) or `moveTo` (put an entity somewhere). [What the engine offers](#what-the-engine-offers) lists every call and the file it's in.
   - `project.lst`, which holds your project's own names, such as its assemblies, written again whenever you add or rename one.
   - folders: `kit` holds the [ready-made components](concepts.md#simulation), `animators` and `maps` hold a file for each animator and tile map you make, and `data` says what the Lust files Solis makes (tile maps, tilesets and the like) hold.

   Why files? Lust checks every call you write against them before the game runs, so a misspelt call is a mistake you see at once, not a surprise while playing.
2. Right-click in the Library and choose **New > Lust code file**. The first one is `main.lst`, at the top of the `assets` folder: your game's code starts there. Lust reads `main.lst` first, and every other code file is fetched from it, or from a file it fetches. The file opens in your code editor for Lust (Ledger), with a component to start from. You can also make the file by hand in any text editor.
3. Write a component. An entity's own abilities, such as `position()` and `moveTo(place)`, need no `get`: they're on every entity. The engine's other parts are objects you `fetch`, such as `fetch "solis/world":World;` for `World:say(…)`:

   ```
   component Spinner {
       has {
           float speed = 2;
       }
   }

   pass Spinner;
   end
   ```

4. Save. Solis checks the code within a moment. Mistakes appear in the [Problems](problems.md) panel, such as "main.lst, line 6:15: speed is a float, but this is text". The Console says how many there are. **Notes** go to the Console too, as warnings: things that aren't mistakes, so the code still runs, but worth changing (writing `me` where Lust now says `this`). Each is said once, until the notes change.
5. Click a mistake to open its file in your code editor, fix it and save. It disappears from Problems by itself.

## Components

A [component](concepts.md#the-area) written in Lust works like Solis's own: put it on an entity, set its fields in Properties, and it does its job when the game runs.

```
component Mover {
    has {
        float speed = 2;
    }

    method update(float dt) {
        vec3 ahead = [1, 0, 0];
        moveTo(position() + ahead * speed * dt);
    }
}

pass Mover;
end
```

The same component in **Lua**, in any `.lua` file in `assets` (here `mover.lua`):

```lua
local Mover = component("Mover", {
    { "speed", "float", 2 },
})

function Mover:update(dt)
    solis.entity.moveTo(self.me, solis.entity.position(self.me) + vec3(1, 0, 0) * self.speed * dt)
end
```

`component` takes the name and the fields in order, each as `{ name, kind, starting value }` (a `state` also lists its options: `{ "mode", "state", "Single", { "Single", "Burst" } }`). The fields are `self.speed` and so on, the entity is `self.me`, the engine's methods are under `solis.`, and `vec3(x, y, z)` makes a vector (a Luau vector, the same kind of value a Lust `vec3` is) you can add, take away and scale, with `v.x` reading a part. A Lua component works the same way as a Lust one from here on, and both can be in the same game.

1. Save the file. When the code has no mistakes, **Mover** appears in Properties' **Add component** menu.
2. Select an entity and add **Mover**. Its `has` fields show in Properties with their starting values (`speed` as **Speed**), and are saved in the area like any other component's.
3. Press **Play**. Every [tick](concepts.md#game-code) (60 times a second), Solis runs each component's `update`, with `dt`, how long a tick is in seconds. `this` is the entity the component is on (older code says `me`, which still works: the Console shows a note saying to write `this`). The Game panel shows the entity moving.
4. Change **Speed** in Properties while it plays: the running Mover takes the new speed at once, and keeps everything else it has (here, where it is). While the game plays, Properties shows a game code component's values as they are right now, not the ones it started with: a turret that has fired shows the ammo it has left. Why? So you can watch what your code is doing without writing anything to the Console. A value you set there is taken by the running component as you set it, even when it's the value it started with: type the turret's full ammo and it has full ammo again. (With several entities selected, it shows the values they started with.)
5. Press **Stop**. Everything the game changed is back as it was before Play. The Speed you changed yourself is a change to the area, so Solis lists it and asks: **Keep** it, or **Undo all** (see [Play mode](play-mode.md)).

What a field can hold: `bool`, `int`, `float`, `string`, a `state` (shown as a choice of its options), `vec3`, `rotation` and `entity` (another entity, or `nothing`).

**Order:** each tick runs entities in the order of their IDs, and an entity's components in the order of their names, so a game runs the same way every time.

**What game code sees during a tick:** the world as it was when the tick started, plus its own changes. Solis runs game code on every core of your processor at once, so components can't see each other's moves while they happen. Instead:

- `position()`, `rotation()`, `name()` and `World:find` read the world as it was at the start of the tick. A component that moves its own entity sees the new place at once; another component sees it next tick.
- `moveTo`, `turnTo`, `face` and `World:say` are kept, and happen when every `update` is done, in the order above. If two components move the same entity, the last one in that order wins.
- Events are delivered after that, a round at a time; each round sees the world as the round before left it.

So a game runs the same way on a computer with 2 cores or 32. Most code never notices: a unit chasing another simply aims at where the other was a sixtieth of a second ago. Code that needs to see a move straight away should move things from one component, or tell the other with an event.

**When something goes wrong while it runs,** such as dividing by zero, that component stops and the Console says which one, on which entity, and where in the Lust: "Divider on Splitter stopped: main.lst, line 8: …". The rest of the game keeps going. A component that runs for more than a tenth of a second without stopping (a loop that never ends) is stopped the same way, so it can't freeze Solis.

**When code changes too much at once:** a loop that calls `moveTo`, `World:fire`, `World:spawn` or `World:say` hundreds of thousands of times in one tick has the changes past 200,000 left out (for each core), and the Console says so once. It's nearly always a loop that never ends.

**When the whole tick is too slow:** if the game's code takes more than a quarter of a second in one tick on any one core (many slow components, say), the rest of that tick waits and the game **pauses**, with a message in the Console saying why. None of the components is to blame on its own, so none stops; make the slow ones faster, then press **Pause** or **Play** to carry on. Without this, the game would try to catch up and the editor would hardly respond.

**What game code says** goes to the Console a line at a time. A very long line is cut short, and past 200 lines in one tick the rest are counted ("Game code said 800 more lines just now"), so a component that talks too much can't drown the Console or fill the computer's memory.

## Changing code while the game runs

Keep the game playing, change your Lust, and save. In about a third of a second the running game has the new code, and nothing starts over:

- **Every running component keeps its values,** field by field, by name: a turret keeps the ammo it has left, a mover stays where it got to. Only `update` and the `on` reactions are new.
- **A field whose kind changed** (an `int` that's now a `float`), **or a new field,** takes its starting value. The Console says which: "Gun's ammo is float now, not int, so it starts at 10."
- **A removed field** is dropped; **a removed component** stops.
- **A component new in the code** starts on every entity that has it.
- **A component that stopped** after a failure runs again, since its code changed.
- **A mistake in the new code** leaves the game running the last code that worked, and the mistake shows in Problems. Fix it and save again.

The Console says what changed each time: "The running game took the new code (Gun changed; Radar new); components keep their values."

## Events

Components tell each other things with [events](concepts.md#game-code): a gun tells a target it was hit, the target's health tells its entity it died. An event says what it carries. A component reacts to it with `on`, and anything sends it to an entity with `send`:

```
fetch "solis/world":World;

event Hit(entity by, float damage);
event Died();

component Health {
    has {
        float hp = 10;
    }

    on Hit(entity by, float damage) {
        is hp > 0? then {
            hp = hp - damage;
            is hp <= 0? then {
                send Died() to this;
            }
        }
    }

    on Died() {
        World:say("down!");
    }
}

component Gun {
    has {
        entity target = nothing;
    }

    method update(float dt) {
        send Hit(this, 2.5) to target;
    }
}

pass Health, Gun;
end
```

- **When they arrive:** after every component's `update` in the tick (and after the moves those made), in the order they were sent (by the sending entity's ID, then component name). On the entity, each component with an `on` for it reacts, in the order of the components' names. An event sent while reacting to another (Hit sends Died) arrives later in the same tick.
- **To something you just spawned:** an event sent to an entity spawned in the same tick (`entity shot = World:spawn(…); send Launch(dir) to shot;`) arrives at the start of the next tick's events, once its components exist. Its first `update` runs just before, so set it moving in the `on`, not in `update`'s first run.
- **An event to `nothing`** goes nowhere, so a gun with no target is harmless.
- **An event sent while starting** goes nowhere too. The lines outside components run before anything plays, and a field's starting value is worked out as its component is made (`int id = register();`, where `register` has a `send` in it): nothing is running yet to take the event. Send from `update` or an `on`.
- **Lust checks** that every `send` carries what the event says, so a misspelt event or a missing value is a mistake in Problems before anything runs.
- **Components that keep sending to each other** forever are stopped after 10,000 events in one tick, with a message in the Console (once, until the code changes), so the game can't freeze. At most 10,000 events can wait at once, holding at most 16 MB of text: past that, the `send` itself fails, so the Console points at the component sending too much.
- **A component whose reaction keeps failing** (an `on` that divides by zero, every time) is said once in the Console; it keeps running, and its later failures with events aren't said again until the code changes.

**In Lua**, `solis.send(to, "Hit", by, 2.5)` sends an event, and a function called `on_` and the event's name reacts to it:

```lua
function Health:on_Hit(by, damage)
    self.hp = self.hp - damage
end
```

When the Lust declares the event, what Lua sends is checked against what it carries: a wrong value means the event isn't sent, and the Console names the Lua component that sent it. Lua can also send events no Lust declares; their numbers arrive as floats (Lua has one kind of number, so `2` and `2.5` arrive alike).

A Lua component's fields can't be called `me` (the entity it's on), `update`, or anything starting with `on_` or `__`: those are its methods, and Lua's own.

**Each Lua file has globals of its own.** A helper a file makes global (`function helper() … end`) is seen by that file only, and no file can change Lua's own libraries (`string`, `table`, `math`) or `solis`, `component` and `vec3`: trying to is a mistake. That way one file can't break another, or the Lust. To share code between Lua files, put it in a component's events, or in Lust.

**What Lua game code can't do.** Game code is sandboxed, so a game you download can't harm your computer: it has no `require`, no files, and no way out of its time and memory limits. It has no `loadstring`, `getfenv` or `setfenv` either: those reach past a file's own globals, into another file's or Solis's own, and calling one is a mistake ("attempt to call a nil value"). `table.move` moves at most a million items at once (a bigger one couldn't be stopped in time), and `collectgarbage` only tells you how much memory is used (`collectgarbage("count")`).

**In Rust**, a component implements `script::RustComponent`, and takes and sends the same events by name. Solis can't build a game's own Rust until Phase 18; until then, the [turret sample](first-game.md)'s Rust component is built into Solis to show how it works.

```rust
fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
    if event.name != "Hit" {
        return false;
    }
    self.dents += 1;
    let me = cx.me();
    cx.send(Some(me), "Dented", vec![Value::Int(self.dents)]);
    true
}
```

Other code files can sit in any folder in `assets`. Fetch them by their path from `assets`, such as `fetch "weapons/gun":Gun;` for `assets/weapons/gun.lst`. The Library's **Code** tab lists them all.

## Projectiles

Lasers, bolts and tracer rounds come in their thousands, so they aren't entities: they're [projectiles](concepts.md#game-code), which Solis moves itself, every tick, on every core.

```
fetch "solis/world":World;

component Turret {
    has {
        int cooldown = 0;
    }

    method update(float dt) {
        cooldown = cooldown - 1;
        is cooldown <= 0? then {
            World:fire(position(), [0, 1, 0], 60, 2, [4, 0.3, 0.2]);
            cooldown = 30;
        }
    }
}

pass Turret;
end
```

`fire` takes where it starts, which way it flies (any length), its speed in metres a second, how long it lives in seconds, and its colour as red, green and blue. Above 1 glows brighter: `[4, 0.3, 0.2]` is a hot red laser. In Lua it's `solis.World:fire(from, direction, speed, life, colour)`, with `vec3(…)` for each.

- It flies in a straight line and disappears when its time is up. It doesn't hit anything: lasers are kept out of [physics](physics.md) so a million can fly. To see what a shot would hit, look along its line with `castRay`.
- It's drawn in the **Game** panel as a glowing streak, as long as it flies in two frames. Streaks add their light where they cross.
- Up to 1,000,000 can fly at once. Past that, new shots aren't fired until some run out, and the Console says so once.
- Like a move, a shot lands after every `update` in the tick is done ([what game code sees](#components)), so it starts flying next tick.

## Making and removing entities

Game code can make copies of [assemblies](assemblies.md) while it plays, and take entities away:

```
fetch "solis/world":World;
fetch "solis/project":Assembly;

component Hangar {
    has {
        int made = 0;
    }

    method update(float dt) {
        is made < 10? then {
            entity ship = World:spawn(Assembly.Ships.Fighter, [made * 5, 0, 10]);
            made = made + 1;
        }
    }
}

pass Hangar;
end
```

- **Which assembly:** one of the project's, named from its file with folders as options inside options: `ships/fighter.solassembly` is `Assembly.Ships.Fighter` (each part's first letter a capital). Solis writes the names into `solis/project.lst` whenever you add, rename or remove an assembly, so a misspelt or deleted one is a mistake before the game runs, listing the real ones. A file whose name can't be a Lust name (it starts with a digit, has a dash or space, is all capitals, or is one of Lust's own objects like `time`) is left out, and the Console says which. `Assembly.None` is there when the project has none, and makes nothing.
- **When:** like a move, it happens after every `update` in the tick ([what game code sees](#components)). `World:spawn` gives back the new entity at once, so you can keep it. It joins the game at the end of the tick, after the tick's moves: its components start then and run from the next tick, even when a slow frame runs several ticks at once. A `despawn` lands at the same moment, in the order they were asked for, so spawning something and taking it out in one tick leaves nothing behind. New copies go at the end of the area's tree, in the order they were spawned.
- **Where it shows:** copies game code makes appear in the Area panel while playing, and are gone when you press **Stop**, like everything else the game did. They aren't undo steps, which is how Solis tells them from your own edits: Stop offers to keep only yours ([Play mode](play-mode.md)), so a change you make to one of these copies goes with it.
- At most 10,000 copies are made in one tick, and an area holds at most a million entities; past that, the Console says so and the rest aren't made.
- **When a copy can't be made:** `World:spawn` still gives back an entity, but nothing appears, and the [Console](console.md) says why in a warning: *Game code spawned "squad", but it places an assembly (…) that isn't in the project, so nothing was made.* The reasons are those of [assemblies](assemblies.md) themselves: the assembly places another whose file is missing or can't be read, assemblies place each other in a loop (the message names them in order), they're placed more than 16 deep, or a copy would be more than a million entities. Each assembly's reason is said once each time you press **Play**, however many times the code asks, so a spawn in `update` doesn't fill the Console. Fix the assembly and press Play again.
- `despawn()` is how a component takes its own entity away, and `target:despawn()` takes another. It ends at the end of the tick.

## Random numbers

Dice, loot, where an enemy wanders next: `random()` gives a number from 0 up to 1, `randomRange(low, high)` one between two numbers, and `randomInt(low, high)` a whole number, both ends included.

```
fetch "solis/world":World;

component Die {
    method update(float dt) {
        World:say("rolled " + World:randomInt(1, 6).text());
    }
}

pass Die;
end
```

Solis makes them from the game's **seed** ([random numbers](concepts.md#game-code)), so the same seed gives the same numbers, on any computer and any number of cores. Each Play has a new seed; type `seed` in the [Console](console.md) to see it, and `seed 42` to start every Play from 42, so you can watch the same game again. `setSeed(n)` starts the numbers again from `n` at the end of the tick. Lua's own `math.random` and `math.randomseed` follow the same seed. The numbers carry on while the game is paused with `World:setGameSpeed(0)`: every tick still rolls new ones, so a pause menu can shuffle or roll like any other code.

## Game-wide objects

Some things belong to the whole game, not to any one entity: the time of day, the player's money, the weather. Write one as a Lust `object` and pass it, and Solis runs it for you ([game-wide object](concepts.md#game-code)):

```
object Clock {
    has {
        int day = 1;
        float hour = 6;
    }

    method update(float dt) {
        hour += dt / 60;
        is hour >= 24? then {
            hour -= 24;
            day += 1;
        }
    }

    method bool isNight() {
        return hour < 6 || hour >= 20;
    }
}

component StreetLamp {
    has {
        bool lit = false;
    }

    method update(float dt) {
        lit = Clock:isNight();
    }
}

pass Clock, StreetLamp;
end
```

- **Ticked first:** each passed object's `update(float dt)` runs once a tick, objects in order of their names, before any entity's components. So every lamp sees the same hour, the one the clock has just moved to.
- **Read anywhere, changed only in its own update.** Components read an object's fields and call its methods (`Clock.hour`, `Clock:isNight()`). Changing one of its fields anywhere else (`Clock.hour = 12;` in a component) stops that component, with a message pointing at the line: "Clock.hour can't be changed here: an object changes only in its own update". An object's `update` may change other objects too. **Why:** Solis runs game code on every core of the computer at once, each core with its own copy of the code. Objects run on the first, and their fields are copied to the others before the components run; if a component on one core could change the clock, the others would never know.
- **Kept in save games:** a save keeps every object's fields, and loading gives them back ([Save games](save-games.md)). New code while the game runs keeps them too, field by field, by name, like a component's.
- **Which fields:** the kinds a component's fields can be (`bool`, `int`, `float`, `string`, a state, `vec3`, `rotation`, `entity`) are copied and kept. A list, a map or a thing in an object isn't saved yet, and each core keeps its own copy, so the Console says so when you press Play. Keep what every part of the game must see in plain fields.
- **An object that fails** in its `update` stops, and the Console says where, like a component; it runs again when the code changes. Components can still read it.
- **Lust only:** objects are written in Lust. Lua and Rust components can't read them yet.
- An object that isn't passed is its file's own helper, and Solis leaves it alone.

## Saving and loading

`Saves:saveTo("quick")` saves the whole game and `Saves:loadFrom("quick")` puts it back exactly as it was: see [Save games](save-games.md). (`save` and `load` on their own are Lust's own words, for its files.)

## Sounds

Game code plays [sound events](sound-events.md) by the project's names for them: Solis writes each event into `solis/project.lst` as `state Sound`, folders as options inside options, so `assets/sfx/footstep.solsound` is `Sound.Sfx.Footstep`. A misspelt or deleted event is a mistake before the game runs. Fetch `Audio` from `solis/sound` and `Sound` from `solis/project`:

```
fetch "solis/sound":Audio;
fetch "solis/project":Sound;

component Walker {
    has {
        int steps = 0;
    }

    method update(float dt) {
        steps = steps + 1;
        is steps % 30 == 0? then {
            Audio:playAt(Sound.Sfx.Footstep, position());
        }
    }
}

pass Walker;
end
```

- `Audio:play(Sound.Music.Theme)` plays it the same everywhere: music, the interface.
- `Audio:playAt(Sound.Sfx.Footstep, place)` plays it at a place in the world: quieter far from the camera, and from its direction (for an event whose Placement is **In the world**).
- `playSound(Sound.Sfx.Engine)` plays it on the component's entity and follows it as it moves: an engine, a voice (`target:playSound(…)` on another). `stopSounds()` stops them, with a short fade, even ones asked for earlier in the same tick; it doesn't stop the entity's AudioSource (use `setSoundPlaying` for that).
- `setSoundPlaying(playing)` turns the entity's [AudioSource](sound-events.md#sounds-that-keep-playing) (a sound that keeps playing: an engine, a campfire) on or off.
- `Audio:setVolume(Bus.Music, volume)` sets a [bus](mixer.md) for the rest of the play (the buses are the Mixer's, as `state Bus` in `solis/project.lst`; the UI bus is `Bus.Ui`, since Lust names aren't all capitals), from 0 (silent) to 1 (as set in the Mixer): what an options menu's volume sliders call. Stop puts the Mixer's back.

In Lua they're `solis.Audio:playAt("Sfx.Footstep", solis.entity.position(self.me))` and so on (the option's name as text; `"sfx/footstep"` works too). In Rust, through the component's host: `cx.host().play_sound("sfx/footstep", SoundAt::Place(place))`, and `cx.host().set_bus_volume("Music", 0.5)`.

**Sound never changes the game.** None of these gives anything back, so code can't ask "is it still playing?": a game plays exactly the same with sound on, off, or with no sound card, and a [recording](concepts.md#simulation) replays the same. Each play picks its take from the game's seed and the tick, so the same game even sounds the same each time. Like a move, a sound lands at the end of the tick.

- The **camera** is what hears (or a [Listener](listeners.md)), and the entity whose code plays a sound is who caused it, for a sound event heard by **Only who caused it**: a sound to its right comes from the right. Pausing the game pauses its sounds, Play carries them on, and Stop fades them out. A Step plays that tick's sounds.
- A sound placed past its event's **Silent past** distance isn't started at all.
- Up to 256 sounds can be asked for in one tick (more are left out, and the Console says so once), and up to 256 play at once.
- If the computer stalls for a moment, only the last few ticks' sounds play, so a hitch doesn't play a pile of footsteps at once.

## Input

Game code reads the player's controls as named actions, never keys: `Input:axis(Action.Move)`, `Input:pressed(Action.Jump)`, `Input:held(Action.Sprint)`, with `Input` from `solis/input` and `Action` from `solis/project`. See [Input](input.md), which also covers gamepads, rebinding and recording a play.

## Physics

Things that fall, collide and get pushed have a **RigidBody** and a **Collider**: see [Physics](physics.md). Game code pushes them (`push`), reads and sets how fast they move (`velocity`, `setVelocity`), and looks along lines with `Physics:castRay`, which gives back a `RayHit`. Characters with a **CharacterBody** are moved with `walk` and asked `grounded`: see [Characters and cameras](characters-and-cameras.md), which also has the ready-made character controller and cameras in `solis/kit/`.

## What the engine offers

| In | Method | What it does |
|---|---|---|
| `solis/world` | `World:say(string words)` | Writes words in the Console. |
| `solis/world` | `float World:time()` | How long the game has been running, in seconds. |
| `solis/world` | `entity World:find(string name)` | The first entity with this name, in the Area panel's order, or `nothing`. |
| `solis/world` | `World:fire(vec3 from, vec3 direction, float speed, float life, vec3 colour)` | Fires a projectile (see Projectiles, below). |
| `solis/world` | `entity World:spawn(Assembly which, vec3 place)` | Makes a copy of one of the project's assemblies at a place (see Making and removing entities, below). |
| `solis/world` | `float World:frameTime()` | How long the last frame took, in seconds of real time (1/60 at 60 frames a second). |
| `solis/world` | `float World:random()` | A random number from 0 up to 1 (see Random numbers, above). |
| `solis/world` | `float World:randomRange(float low, float high)` | A random number from low up to high. |
| `solis/world` | `int World:randomInt(int low, int high)` | A random whole number from low to high, both included. |
| `solis/world` | `World:setGameSpeed(float speed)` | Runs the game at a speed, from the end of the tick: 0 pauses it (time, physics, projectiles and sprites stop and `dt` is 0, but components still run, so a pause menu works), 1 as usual, up to 4. |
| `solis/world` | `World:gameSpeed()` | How fast the game runs, as the tick started. |
| `solis/screens` | `Screens:setTextSize(float size)`, `Screens:textSize()` | The player's text size on every screen, 0.5 to 3 (1 as designed), from the end of the tick ([Game screens](screens.md)). |
| `solis/screens` | `Screens:setUiSize(float size)`, `Screens:uiSize()` | The player's UI size: every screen bigger or smaller, 0.5 to 2. |
| `solis/window` | `Window:setFullscreen(bool on)`, `Window:fullscreen()` | Whether the game fills the screen (Alt+Enter switches it too). |
| `solis/window` | `Window:setResolution(int width, int height)`, `Window:resolution()`, `Window:useNextResolution()` | The window's size in pixels, and the next usual one, for a menu. |
| `solis/window` | `Window:quit()` | Closes the game at the end of the tick, for a menu's Quit button. While playing in the editor, play stops ([Exporting for playtesting](playtest-export.md)). In Lua `solis.Window:quit()`; in Rust `quit`. |
| `solis/world` | `float World:timeOfDay()`, `World:setTimeOfDay(float hour)` | The game's hour, 0 up to 24, from the Environment's Day and night; setting it moves the day on from there, at the end of the tick ([Sky, fog and exposure](sky-fog-and-exposure.md)). In Lua `solis.World:timeOfDay()`; in Rust `time_of_day`. |
| `solis/world` | `World:setSeed(int seed)` | Starts the random numbers again from a seed, at the end of the tick. |
| `solis/world` | `Saves:saveTo(string slot)` | Saves the whole game in a slot, at the end of the tick ([Save games](save-games.md)). |
| `solis/world` | `Saves:loadFrom(string slot)` | Loads the game saved in a slot, at the end of the tick. |
| `solis/project` | `state Assembly` | The project's assemblies, folders as options inside options (`Assembly.Ships.Fighter`). |
| `solis/sound` | `Audio:play(Sound event)` | Plays a sound event the same everywhere (see Sounds, above). |
| `solis/sound` | `Audio:playAt(Sound event, vec3 place)` | Plays a sound event at a place in the world. |
| `solis/sound` | `Audio:setVolume(Bus bus, float volume)` | Sets a bus's volume for the rest of the play, 0 to 1. |
| any entity | `playSound(Sound event)` | Plays a sound event on the entity, following it. |
| any entity | `stopSounds()` | Stops the sounds on the entity, with a short fade. |
| any entity | `setSoundPlaying(bool playing)` | Turns the entity's AudioSource on or off. |
| `solis/project` | `state Sound`, `state Bus` | The project's sound events (`Sound.Sfx.Footstep`) and the Mixer's buses (`Bus.Music`). |
| any entity | `animate(Clip clip)` | Plays a clip on the entity's Sprite; already playing or finished it, it carries on as it is ([Animating sprites](sprite-animation.md)). |
| any entity | `animateFromStart(Clip clip)` | Plays a clip from its beginning, whatever it was doing. |
| any entity | `stopAnimation()` | Stops the entity's Sprite where it is in its clip. |
| any entity | `setFlip(bool across, bool down)` | Mirrors the entity's Sprite: across faces it the other way, down turns it upside down. |
| `solis/animation` | event `CueReached(Cue cue)` | Sent when a sprite's clip reaches a frame with an event (bring it in with `fetch "solis/animation":CueReached;`). |
| `solis/animation` | event `ClipFinished(Clip clip)` | Sent when a sprite's clip has played all its times. |
| `solis/project` | `state Clip`, `state Cue` | The sprites' clips, under their sheets (`Clip.Sprites.Farmer.Walk`), and their frame events (`Cue.Footstep`). |
| any entity | `showScreen(Screen which)` | Shows one of the game's screens on the entity, at the end of the tick ([Game screens](screens.md)). |
| any entity | `hideScreen(Screen which)` | Hides it, if the entity shows it. |
| any entity | `screenShown(Screen which)` | Whether the entity shows that screen, as the tick started. |
| `solis/project` | `state Screen` | The game's screens, by their files in `ui/` (`Screen.PauseMenu` for `ui/pauseMenu.lst`). |
| `solis/animators/<file>` | `class <File>Animator`, `state <File>State` | Each animator's own file: `anim:setMoving(true)`, `anim:current()` ([Animators](animators.md)). |
| `solis/maps/<file>` | `class <File>Map`, its layers, terrains and choices as states | Each tile map's own file: `farm:setWatered(x, y, true)`, `farm:terrain(FarmLayer.Ground, x, y)` ([Tile maps](tile-maps.md)). |
| `solis/input` | `bool Input:held(Action action)` | Whether an action is held this tick ([Input](input.md)). |
| `solis/input` | `bool Input:pressed(Action action)` | True once, the tick an action went down. |
| `solis/input` | `bool Input:released(Action action)` | True once, the tick it came up. |
| `solis/input` | `vec3 Input:axis(Action action)` | An axis action's x and y. |
| `solis/input` | `string Input:binding(Action action)` | The key that sets an action off, for "Press F". |
| `solis/input` | `Input:rebind(Action action, string input)` | Binds a new input to an action, from the next tick. |
| `solis/project` | `state Action` | The project's input actions (`Action.Jump`), from Edit > Input actions…. |
| `solis/physics` | `RayHit Physics:castRay(vec3 from, vec3 direction, float reach)` | What a line meets first: `found`, `distance`, `entity` and `normal` ([Physics](physics.md)). |
| any entity | `vec3 position()` | Where the entity is in the world, in metres. |
| any entity | `moveTo(vec3 place)` | Puts the entity at a place in the world. |
| any entity | `rotation rotation()` | Which way the entity faces. |
| any entity | `turnTo(rotation facing)` | Turns the entity to face this way. |
| any entity | `face(vec3 direction)` | Turns the entity, upright, to face along a direction (its forward is +Y): `face(target:position() - position())` faces a place. |
| any entity | `string name()` | The entity's name. |
| any entity | `despawn()` | Takes the entity, and everything under it, out of the game. |
| any entity | `push(vec3 impulse)` | Gives its body a push ([Physics](physics.md)). |
| any entity | `vec3 velocity()` | How fast and which way its body moves, metres a second; for a 2D character, how fast its last tick's move went. |
| any entity | `setVelocity(vec3 velocity)` | Sets how fast and which way its body moves. |
| any entity | `walk(vec3 wanted)` | Walks a character this far, climbing steps, sliding along walls ([Characters and cameras](characters-and-cameras.md), [Walls and walking](walls-and-walking.md)). |
| any entity | `bool grounded()` | Whether a character stood on something after its last move (3D or 2D). |
| `solis/entity` | `rotation facingAlong(vec3 direction)` | The upright rotation facing along a direction, for working out a turn without making it. |

An entity's abilities are called bare for the component's own entity (`moveTo(place)`) and with `:` on any other (`target:moveTo(place)`). In Lua they're `solis.entity.moveTo(target, place)`, and the objects' `solis.World:say(words)`.

**Calls that changed.** Solis's calls now follow one shape (methods on things, the project's names as states), so some older ones are written differently. Old code is a mistake that says the new way: "positionOf is now written target:position() (inside a component, position() for its own entity)".

| Before | Now |
|---|---|
| `positionOf(e)`, `moveTo(e, p)` | `e:position()`, `e:moveTo(p)` (bare for the component's own entity) |
| `rotationOf(e)`, `turnTo(e, r)`, `nameOf(e)`, `despawn(e)` | `e:rotation()`, `e:turnTo(r)`, `e:name()`, `e:despawn()` |
| `turnTo(this, facingAlong(d))` | `face(d)` |
| `say`, `time`, `frameTime`, `find`, `fire`, `random`, `randomRange`, `randomInt`, `setSeed` | the same names on `World`: `World:say(words)` |
| `spawn("ships/fighter", p)` | `World:spawn(Assembly.Ships.Fighter, p)` |
| `saveGame(slot)`, `loadGame(slot)` | `Saves:saveTo(slot)`, `Saves:loadFrom(slot)` |
| `held("Jump")`, `pressed`, `released`, `axis("Move")` | `Input:held(Action.Jump)` and so on, with `fetch "solis/input":Input;` and `fetch "solis/project":Action;` |
| `bindingOf("Jump")`, `rebind("Jump", key)` | `Input:binding(Action.Jump)`, `Input:rebind(Action.Jump, key)` |
| `push(e, f)`, `velocityOf(e)`, `setVelocity(e, v)` | `e:push(f)`, `e:velocity()`, `e:setVelocity(v)` |
| `moveCharacter(e, w)`, `onGround(e)` | `e:walk(w)`, `e:grounded()` |
| `playSound("sfx/x")`, `playSoundAt("sfx/x", p)` | `Audio:play(Sound.Sfx.X)`, `Audio:playAt(Sound.Sfx.X, p)` |
| `playSoundOn("sfx/x", e)`, `stopSoundsOn(e)`, `setSoundPlaying(e, b)` | `e:playSound(Sound.Sfx.X)`, `e:stopSounds()`, `e:setSoundPlaying(b)` |
| `setBusVolume("Music", v)` | `Audio:setVolume(Bus.Music, v)` |
| `playClip(e, "walk")`, `replayClip`, `stopClip(e)` | `e:animate(Clip.Sprites.Farmer.Walk)`, `e:animateFromStart(…)`, `e:stopAnimation()` |
| `on AnimationCue(string name)`, `on ClipFinished(string clip)` | `on CueReached(Cue cue)`, `on ClipFinished(Clip clip)` |
| `castRay(…)`, `castRayHit(…)`, `castRayNormal(…)` | one call: `RayHit hit = Physics:castRay(…);` then `hit.distance`, `hit.entity`, `hit.normal` |
| Lua `solis.positionOf(e)`, `solis.say(w)`, `solis.held("Jump")` | `solis.entity.position(e)`, `solis.World:say(w)`, `solis.Input:held("Jump")` |

**When an update renames something.** What game code can call doesn't change by accident: Solis's own checks stop any update that changes it without saying so. When a name does change on purpose, the old name keeps working for a few phases, so your game still runs the day you update. Each place your code uses the old name shows in the [Problems](problems.md) panel as a warning, with its file and line, the new name, and when the old one stops working. Change the call to the new name before then.

## Safe to open

Game code runs inside a **sandbox**: it can reach the game through the engine's methods, and nothing else. It can't read or write files, start programs or reach the network, it has 64 MB of memory, and code that runs too long is stopped. Lua's `require` isn't there either (it would read files from anywhere on the computer). So a project you download or are sent can't harm your computer through its code, and Solis never runs the other files in it (see [Projects and the Library](projects-and-the-library.md)).

## Tips and common problems
- **Problem:** Problems says "there's already something called Health". **Why:** two components, classes or objects have the same name (often a copied one not yet renamed), perhaps in two files; the message says where the other is. **Fix:** rename one.
- **Problem:** Problems says checking the code took more than 30 seconds and was given up on. **Why:** something in the code is more than the checker can handle; it shouldn't happen. **Fix:** undo your last change, and please report it with that code.
- **Problem:** Problems says the game's code starts at `main.lst` and there isn't one. **Why:** there are `.lst` files, but none is called `main.lst` in the `assets` folder itself. **Fix:** make `assets/main.lst`, and fetch your other files from it.
- **Problem:** changes I made to `solis/world.lst` (or `solis/kit/…`) disappeared. **Why:** Solis writes the `solis` files again when the project opens, so they always match the engine. **Fix:** put your own code in your own files; to change a ready-made component, copy its file outside `solis/` and rename the component.
- **Problem:** my component isn't in **Add component**. **Why:** the code has a mistake (see Problems), or its Lust file isn't fetched from `main.lst` (directly, or through a file `main.lst` fetches). Lua files are all read, wherever they are in `assets`. **Fix:** fix the mistake, or fetch the file.
- **Problem:** "solis.entity.moveTo isn't something Solis offers" in Lua. **Why:** the name is misspelt, or the method was used outside `update` or an `on_` function, when the game isn't running. **Fix:** check the name against the table above.
- **Problem:** the Console says a call "wants text for words, and was given nothing (nil)" in Lua. **Why:** one of the values given to the call isn't the kind it takes. Nothing (`nil`) usually means a value is missing, a variable's name is misspelt (a name Lua doesn't know is `nil`), or the call is written with a dot where it needs a colon: `solis.World.say("hello")` for `solis.World:say("hello")`. **Fix:** the message names the call, the value and the line; check that value there.
- **Problem:** the Console says "this calls something of Solis's that was kept from earlier" in Lua. **Why:** a call was put in a variable, as `local say = solis.World.say` at the top of a file does. Solis lends its calls to game code only while that code runs, so the one kept stops working as soon as the file has loaded. **Fix:** write the call out where it's used (`solis.World:say("hello")`), or keep the object and call through it: `local World = solis.World`, then `World:say("hello")`.
- **Problem:** a component is called the same as one of Solis's (`Light`) or a Rust one, or a Lust and a Lua component share a name. **Why:** two components can't share a name. The one in your code doesn't run (Solis's own, or the Rust one, does). **Fix:** rename yours; Problems says where it is.
- **Problem:** I saved new code while playing, and the Console says the running game couldn't take it. **Why:** the code's start failed with the game's world (it ran fine with no world while checking). **Fix:** fix the line it points at; the game keeps the last code meanwhile.
- **Problem:** Problems says something is nested too deeply. **Why:** more than 150 levels of brackets, blocks or operators inside each other (`a + b + c …` counts each `+`) is more than Lust takes. **Fix:** split it up with variables or methods.
- **Problem:** Problems says the code stopped while starting. **Why:** Solis runs the code's own start (the lines outside components, and each field's starting value) to read its components, before anything plays and with no world: `World:find` finds nothing then. A start that fails or takes more than a second is a mistake. **Fix:** fix the line Problems points at.
- **Problem:** the Console says a code file was left out. **Why:** each code file can be at most 1 MB, and all of them together 16 MB. Files that aren't text are left out too. **Fix:** split a huge file into smaller ones.
- **Problem:** a component reads another entity's place and gets where it was before this tick's move. **Why:** during a tick, game code sees the world as it was when the tick started (see Components, above), so every core sees the same world. **Fix:** usually nothing: it catches up next tick. If it must be at once, do both in one component, or send an event (events see the moves made before them).
- Commit the `solis` folder with your game, so `lust check` works on other computers too.

## Related
- [Your first game: the turret sample](first-game.md)
- [Problems](problems.md)
- [Projects and the Library](projects-and-the-library.md)
- [Entities and components](entities.md)
- [Play mode](play-mode.md)
- [Physics](physics.md)
- [Input](input.md)
- Concepts: [game code, Lust, native method, tick, event, random number](concepts.md#game-code), [physics](concepts.md#simulation)
- The Lust language itself: [Lust's guide](https://github.com/Rhyslos/Lust/blob/main/LUST.md)
