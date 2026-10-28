# Game code

> Your game's own rules (how a turret aims, when a door opens) are written as code, as components you put on entities. Solis checks the code as you save, shows its fields in Properties, and runs it when you press Play.

## When to use it
Whenever your game needs to *do* something: react to the player, count score, move things on its own. Areas and assemblies say what's in the world; [game code](concepts.md#game-code) says what happens in it.

Solis games can be written in three languages:
- **[Lust](concepts.md#game-code)**, made for Solis's kind of games and the one this manual teaches first;
- **Lua**, if you know it from Roblox or modding;
- **Rust**, the language Solis itself is written in.

**Right now:** Lust and Lua components work: they appear in Properties, run in play mode, talk to each other through events, and take new code while the game runs. Rust components can join them, but for now only in a Solis built from source.

## How to use it
1. Open a [project](projects-and-the-library.md). Solis puts a `solis` folder in its `assets`, holding `world.lst` and `entity.lst`. They describe what your code can ask of the engine, such as `say` (write in the Console) or `moveTo` (put an entity somewhere).
2. Make a file called `main.lst` in the `assets` folder, in any text editor. Your game's code starts there: Lust reads `main.lst` first, and every other code file is fetched from it, or from a file it fetches.
3. Use what the engine offers with Lust's `get`:

   ```
   get "solis/world":[say, time];
   get "solis/entity":[positionOf, moveTo];

   component Spinner {
       has {
           float speed = 2;
       }
   }

   pass Spinner;
   end
   ```

4. Save. Solis checks the code within a moment. Mistakes appear in the [Problems](problems.md) panel, such as "main.lst, line 6:15: speed is a float, but this is text". The Console says how many there are.
5. Click a mistake to open its file in your code editor, fix it and save. It disappears from Problems by itself.

## Components

A [component](concepts.md#the-area) written in Lust works like Solis's own: put it on an entity, set its fields in Properties, and it does its job when the game runs.

```
get "solis/entity":[positionOf, moveTo];

component Mover {
    has {
        float speed = 2;
    }

    method update(float dt) {
        vec3 ahead = [1, 0, 0];
        moveTo(me, positionOf(me) + ahead * speed * dt);
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
    solis.moveTo(self.me, solis.positionOf(self.me) + vec3(1, 0, 0) * self.speed * dt)
end
```

`component` takes the name and the fields in order, each as `{ name, kind, starting value }` (a `state` also lists its options: `{ "mode", "state", "Single", { "Single", "Burst" } }`). The fields are `self.speed` and so on, the entity is `self.me`, the engine's methods are under `solis.`, and `vec3(x, y, z)` makes a vector (a Luau vector, the same kind of value a Lust `vec3` is) you can add, take away and scale, with `v.x` reading a part. A Lua component works the same way as a Lust one from here on, and both can be in the same game.

1. Save the file. When the code has no mistakes, **Mover** appears in Properties' **Add component** menu.
2. Select an entity and add **Mover**. Its `has` fields show in Properties with their starting values (`speed` as **Speed**), and are saved in the area like any other component's.
3. Press **Play**. Every [tick](concepts.md#game-code) (60 times a second), Solis runs each component's `update`, with `dt`, how long a tick is in seconds. `me` is the entity the component is on. The Game panel shows the entity moving.
4. Change **Speed** in Properties while it plays: the running Mover takes the new speed at once, and keeps everything else it has (here, where it is). Properties shows the values the entity started with, not the ones the running game has changed since: a turret that has fired still shows its full ammo there.
5. Press **Stop**. Everything is back as it was before Play.

What a field can hold: `bool`, `int`, `float`, `string`, a `state` (shown as a choice of its options), `vec3`, `rotation` and `entity` (another entity, or `nothing`).

**Order:** each tick runs entities in the order of their IDs, and an entity's components in the order of their names, so a game runs the same way every time.

**What game code sees during a tick:** the world as it was when the tick started, plus its own changes. Solis runs game code on every core of your processor at once, so components can't see each other's moves while they happen. Instead:

- `positionOf`, `rotationOf`, `nameOf` and `find` read the world as it was at the start of the tick. A component that moves its own entity sees the new place at once; another component sees it next tick.
- `moveTo`, `turnTo` and `say` are kept, and happen when every `update` is done, in the order above. If two components move the same entity, the last one in that order wins.
- Events are delivered after that, a round at a time; each round sees the world as the round before left it.

So a game runs the same way on a computer with 2 cores or 32. Most code never notices: a unit chasing another simply aims at where the other was a sixtieth of a second ago. Code that needs to see a move straight away should move things from one component, or tell the other with an event.

**When something goes wrong while it runs,** such as dividing by zero, that component stops and the Console says which one, on which entity, and where in the Lust: "Divider on Splitter stopped: main.lst, line 8: …". The rest of the game keeps going. A component that runs for more than a tenth of a second without stopping (a loop that never ends) is stopped the same way, so it can't freeze Solis.

**When code changes too much at once:** a loop that calls `moveTo`, `fire`, `spawn` or `say` hundreds of thousands of times in one tick has the changes past 200,000 left out (for each core), and the Console says so once. It's nearly always a loop that never ends.

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
get "solis/world":[say];

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
                send Died() to me;
            }
        }
    }

    on Died() {
        say("down!");
    }
}

component Gun {
    has {
        entity target = nothing;
    }

    method update(float dt) {
        send Hit(me, 2.5) to target;
    }
}

pass Health, Gun;
end
```

- **When they arrive:** after every component's `update` in the tick (and after the moves those made), in the order they were sent (by the sending entity's ID, then component name). On the entity, each component with an `on` for it reacts, in the order of the components' names. An event sent while reacting to another (Hit sends Died) arrives later in the same tick.
- **An event to `nothing`** goes nowhere, so a gun with no target is harmless.
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

**What Lua game code can't do.** Game code is sandboxed, so a game you download can't harm your computer: it has no `require`, no files, and no way out of its time and memory limits. `table.move` moves at most a million items at once (a bigger one couldn't be stopped in time), and `collectgarbage` only tells you how much memory is used (`collectgarbage("count")`).

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
get "solis/world":[fire];
get "solis/entity":[positionOf];

component Turret {
    has {
        int cooldown = 0;
    }

    method update(float dt) {
        cooldown = cooldown - 1;
        is cooldown <= 0? then {
            fire(positionOf(me), [0, 1, 0], 60, 2, [4, 0.3, 0.2]);
            cooldown = 30;
        }
    }
}

pass Turret;
end
```

`fire` takes where it starts, which way it flies (any length), its speed in metres a second, how long it lives in seconds, and its colour as red, green and blue. Above 1 glows brighter: `[4, 0.3, 0.2]` is a hot red laser. In Lua it's `solis.fire(from, direction, speed, life, colour)`, with `vec3(…)` for each.

- It flies in a straight line and disappears when its time is up. It doesn't hit anything: lasers are kept out of [physics](physics.md) so a million can fly. To see what a shot would hit, look along its line with `castRay`.
- It's drawn in the **Game** panel as a glowing streak, as long as it flies in two frames. Streaks add their light where they cross.
- Up to 1,000,000 can fly at once. Past that, new shots aren't fired until some run out, and the Console says so once.
- Like a move, a shot lands after every `update` in the tick is done ([what game code sees](#components)), so it starts flying next tick.

## Making and removing entities

Game code can make copies of [assemblies](assemblies.md) while it plays, and take entities away:

```
get "solis/world":[spawn, despawn];

component Hangar {
    has {
        int made = 0;
    }

    method update(float dt) {
        is made < 10? then {
            entity ship = spawn("ships/fighter.solassembly", [made * 5, 0, 10]);
            made = made + 1;
        }
    }
}

pass Hangar;
end
```

- **Which assembly:** its file in the project's assets (`ships/fighter.solassembly`), that without the extension (`ships/fighter`), or just its name (`fighter`) when only one assembly has it. A name that isn't an assembly makes nothing, and the Console says so once.
- **When:** like a move, it happens after every `update` in the tick ([what game code sees](#components)). `spawn` gives back the new entity at once, so you can keep it. It joins the game at the end of the tick, after the tick's moves: its components start then and run from the next tick, even when a slow frame runs several ticks at once. A `despawn` lands at the same moment, in the order they were asked for, so spawning something and taking it out in one tick leaves nothing behind. New copies go at the end of the area's tree, in the order they were spawned.
- **Where it shows:** copies game code makes appear in the Area panel while playing, and are gone when you press **Stop**, like everything else changed while playing. They aren't undo steps.
- At most 10,000 copies are made in one tick, and an area holds at most a million entities; past that, the Console says so and the rest aren't made.
- `despawn(me)` is how a component takes its own entity away. It ends at the end of the tick.

## Random numbers

Dice, loot, where an enemy wanders next: `random()` gives a number from 0 up to 1, `randomRange(low, high)` one between two numbers, and `randomInt(low, high)` a whole number, both ends included.

```
get "solis/world":[randomInt, randomRange, say];

component Die {
    method update(float dt) {
        say("rolled " + randomInt(1, 6).text());
    }
}

pass Die;
end
```

Solis makes them from the game's **seed** ([random numbers](concepts.md#game-code)), so the same seed gives the same numbers, on any computer and any number of cores. Each Play has a new seed; type `seed` in the [Console](console.md) to see it, and `seed 42` to start every Play from 42, so you can watch the same game again. `setSeed(n)` starts the numbers again from `n` at the end of the tick. Lua's own `math.random` and `math.randomseed` follow the same seed.

## Saving and loading

`saveGame("quick")` saves the whole game and `loadGame("quick")` puts it back exactly as it was: see [Save games](save-games.md).

## Sounds

Game code plays [sound events](sound-events.md) by name: the event's file in `assets`, without `.solsound`, so `assets/sfx/footstep.solsound` is `"sfx/footstep"`.

```
get "solis/sound":[playSoundAt, playSoundOn, stopSoundsOn];
get "solis/entity":[positionOf];

component Walker {
    has {
        int steps = 0;
    }

    method update(float dt) {
        steps = steps + 1;
        is steps % 30 == 0? then {
            playSoundAt("sfx/footstep", positionOf(me));
        }
    }
}

pass Walker;
end
```

- `playSound(event)` plays it the same everywhere: music, the interface.
- `playSoundAt(event, place)` plays it at a place in the world: quieter far from the camera, and from its direction (for an event whose Placement is **In the world**).
- `playSoundOn(event, entity)` plays it on an entity and follows it as it moves: an engine, a voice. `stopSoundsOn(entity)` stops them, with a short fade.
- `setSoundPlaying(entity, playing)` turns an entity's [AudioSource](sound-events.md#sounds-that-keep-playing) (a sound that keeps playing: an engine, a campfire) on or off.
- `setBusVolume(bus, volume)` sets a [bus](mixer.md) ("Music", "Effects") for the rest of the play, from 0 (silent) to 1 (as set in the Mixer): what an options menu's volume sliders call. Stop puts the Mixer's back.

In Lua they're `solis.playSoundAt("sfx/footstep", solis.positionOf(self.me))` and so on. In Rust, through the component's host: `cx.host().play_sound("sfx/footstep", SoundAt::Place(place))`, and `cx.host().set_bus_volume("Music", 0.5)`.

**Sound never changes the game.** None of these gives anything back, so code can't ask "is it still playing?": a game plays exactly the same with sound on, off, or with no sound card, and a [recording](concepts.md#simulation) replays the same. Each play picks its take from the game's seed and the tick, so the same game even sounds the same each time. Like a move, a sound lands at the end of the tick.

- The **camera** is what hears: a sound to its right comes from the right. Pausing the game pauses its sounds, Play carries them on, and Stop fades them out. A Step plays that tick's sounds.
- A sound placed past its event's **Silent past** distance isn't started at all.
- Up to 256 sounds can be asked for in one tick (more are left out, and the Console says so once), and up to 256 play at once.
- If the computer stalls for a moment, only the last few ticks' sounds play, so a hitch doesn't play a pile of footsteps at once.

## Input

Game code reads the player's controls as named actions, never keys: `axis("Move")`, `pressed("Jump")`, `held("Sprint")`, in `solis/input`. See [Input](input.md), which also covers gamepads, rebinding and recording a play.

## Physics

Things that fall, collide and get pushed have a **RigidBody** and a **Collider**: see [Physics](physics.md). Game code pushes them (`push`), reads and sets how fast they move (`velocityOf`, `setVelocity`), and looks along lines (`castRay`, `castRayHit`, `castRayNormal`), all in `solis/physics`. Characters with a **CharacterBody** are moved with `moveCharacter` and asked `onGround`: see [Characters and cameras](characters-and-cameras.md), which also has the ready-made character controller and cameras in `solis/kit/`.

## What the engine offers

| In | Method | What it does |
|---|---|---|
| `solis/world` | `say(string words)` | Writes words in the Console. |
| `solis/world` | `float time()` | How long the game has been running, in seconds. |
| `solis/world` | `entity find(string name)` | The first entity with this name, in the Area panel's order, or `nothing`. |
| `solis/world` | `fire(vec3 from, vec3 direction, float speed, float life, vec3 colour)` | Fires a projectile (see Projectiles, below). |
| `solis/world` | `entity spawn(string file, vec3 place)` | Makes a copy of an assembly at a place (see Making and removing entities, below). |
| `solis/world` | `despawn(entity which)` | Takes an entity, and everything under it, out of the game. |
| `solis/world` | `float frameTime()` | How long the last frame took, in seconds of real time (1/60 at 60 frames a second). |
| `solis/world` | `float random()` | A random number from 0 up to 1 (see Random numbers, above). |
| `solis/world` | `float randomRange(float low, float high)` | A random number from low up to high. |
| `solis/world` | `int randomInt(int low, int high)` | A random whole number from low to high, both included. |
| `solis/world` | `setSeed(int seed)` | Starts the random numbers again from a seed, at the end of the tick. |
| `solis/world` | `saveGame(string slot)` | Saves the whole game in a slot, at the end of the tick ([Save games](save-games.md)). |
| `solis/world` | `loadGame(string slot)` | Loads the game saved in a slot, at the end of the tick. |
| `solis/sound` | `playSound(string event)` | Plays a sound event the same everywhere (see Sounds, above). |
| `solis/sound` | `playSoundAt(string event, vec3 place)` | Plays a sound event at a place in the world. |
| `solis/sound` | `playSoundOn(string event, entity on)` | Plays a sound event on an entity, following it. |
| `solis/sound` | `stopSoundsOn(entity on)` | Stops the sounds on an entity, with a short fade. |
| `solis/input` | `bool held(string action)` | Whether an action is held this tick ([Input](input.md)). |
| `solis/input` | `bool pressed(string action)` | True once, the tick an action went down. |
| `solis/input` | `bool released(string action)` | True once, the tick it came up. |
| `solis/input` | `vec3 axis(string action)` | An axis action's x and y. |
| `solis/input` | `string bindingOf(string action)` | The key that sets an action off, for "Press F". |
| `solis/input` | `rebind(string action, string input)` | Binds a new input to an action, from the next tick. |
| `solis/physics` | `push(entity which, vec3 impulse)` | Gives a body a push ([Physics](physics.md)). |
| `solis/physics` | `vec3 velocityOf(entity which)` | How fast and which way a body moves, metres a second. |
| `solis/physics` | `setVelocity(entity which, vec3 velocity)` | Sets how fast and which way a body moves. |
| `solis/physics` | `float castRay(vec3 from, vec3 direction, float reach)` | How far along a line the first collider is, or -1. |
| `solis/physics` | `entity castRayHit(vec3 from, vec3 direction, float reach)` | The entity a line meets first, or nothing. |
| `solis/physics` | `vec3 castRayNormal(vec3 from, vec3 direction, float reach)` | Which way the surface a line meets faces. |
| `solis/physics` | `moveCharacter(entity which, vec3 wanted)` | Moves a character this far, climbing steps, sliding along walls ([Characters and cameras](characters-and-cameras.md)). |
| `solis/physics` | `bool onGround(entity which)` | Whether a character stood on something after its last move. |
| `solis/entity` | `vec3 positionOf(entity which)` | Where an entity is in the world, in metres. |
| `solis/entity` | `moveTo(entity which, vec3 place)` | Puts an entity at a place in the world. |
| `solis/entity` | `rotation rotationOf(entity which)` | Which way an entity faces. |
| `solis/entity` | `turnTo(entity which, rotation facing)` | Turns an entity to face this way. |
| `solis/entity` | `rotation facingAlong(vec3 direction)` | The upright rotation facing along a direction (an entity's forward is +Y): `turnTo(me, facingAlong(target - positionOf(me)))` faces a place. |
| `solis/entity` | `string nameOf(entity which)` | An entity's name. |

## Safe to open

Game code runs inside a **sandbox**: it can reach the game through the engine's methods, and nothing else. It can't read or write files, start programs or reach the network, it has 64 MB of memory, and code that runs too long is stopped. Lua's `require` isn't there either (it would read files from anywhere on the computer). So a project you download or are sent can't harm your computer through its code, and Solis never runs the other files in it (see [Projects and the Library](projects-and-the-library.md)).

## Tips and common problems
- **Problem:** Problems says "there's already something called Health". **Why:** two components, classes or objects have the same name (often a copied one not yet renamed), perhaps in two files; the message says where the other is. **Fix:** rename one.
- **Problem:** Problems says checking the code took more than 30 seconds and was given up on. **Why:** something in the code is more than the checker can handle; it shouldn't happen. **Fix:** undo your last change, and please report it with that code.
- **Problem:** Problems says the game's code starts at `main.lst` and there isn't one. **Why:** there are `.lst` files, but none is called `main.lst` in the `assets` folder itself. **Fix:** make `assets/main.lst`, and fetch your other files from it.
- **Problem:** changes I made to `solis/world.lst` (or `solis/kit/…`) disappeared. **Why:** Solis writes the `solis` files again when the project opens, so they always match the engine. **Fix:** put your own code in your own files; to change a ready-made component, copy its file outside `solis/` and rename the component.
- **Problem:** my component isn't in **Add component**. **Why:** the code has a mistake (see Problems), or its Lust file isn't fetched from `main.lst` (directly, or through a file `main.lst` fetches). Lua files are all read, wherever they are in `assets`. **Fix:** fix the mistake, or fetch the file.
- **Problem:** "solis.moveTo isn't something Solis offers" in Lua. **Why:** the name is misspelt, or the method was used outside `update` or an `on_` function, when the game isn't running. **Fix:** check the name against the table above.
- **Problem:** a component is called the same as one of Solis's (`Light`) or a Rust one, or a Lust and a Lua component share a name. **Why:** two components can't share a name. The one in your code doesn't run (Solis's own, or the Rust one, does). **Fix:** rename yours; Problems says where it is.
- **Problem:** I saved new code while playing, and the Console says the running game couldn't take it. **Why:** the code's start failed with the game's world (it ran fine with no world while checking). **Fix:** fix the line it points at; the game keeps the last code meanwhile.
- **Problem:** Problems says something is nested too deeply. **Why:** more than 150 levels of brackets, blocks or operators inside each other (`a + b + c …` counts each `+`) is more than Lust takes. **Fix:** split it up with variables or methods.
- **Problem:** Problems says the code stopped while starting. **Why:** Solis runs the code's own start (the lines outside components, and each field's starting value) to read its components, before anything plays and with no world: `find` finds nothing then. A start that fails or takes more than a second is a mistake. **Fix:** fix the line Problems points at.
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
