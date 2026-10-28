# Solis engine API guidelines for Lust

Oct 28, 2026 · @Rhy


> **The source of truth for every call game code makes into Solis** ([decision 0065](decisions/0065-engine-api-guidelines.md)). Written by the project lead with the Lust chat, 2026-10-28; rule 4 rewritten by the project lead the same day, and the whole guide revised that evening (abilities called bare inside a component, `this`, editor-made parts as fields, rule 13, the animator's generated file). Every new engine call answers yes to the checklist at the end before it's merged, and the calls Solis had before move over in their own milestone.

## Why these guidelines

Solis's game API today is C written in Lust's spelling. It ignores the three things Lust is built around: methods on things, states for fixed choices, and checks before anything runs.

The animation API shows all three problems:

- **Free functions that take `me` first:** `playClip(me, "walk")`, `setAnimSwitch(me, "moving", moving)`. Inside a component, Lust can call the thing's abilities bare: `animate(...)`.
- **Strings for names the project already knows:** clips, actions, sounds, cues, animator states. A typo like `"wlak"` builds fine and silently does nothing. Lust's whole stance is that a fixed set of choices is a `state`, so a typo is a mistake shown before the game runs.
- **A good state machine reached through strings:** the suggested `setAnimSwitch`, `triggerAnim`, `jumpToAnimState` and `animStateOf(me) == "Use tool"` work like Unity's Animator. The state machine itself is right and stays. The strings are the problem: they are what Unity developers complain about most, because a misspelt name builds fine and does nothing.
- **No shared shape:** readers are `positionOf`, `onGround`, `time`; writers are `moveTo`, `setVelocity`, `push`. One ray takes three calls. Nothing tells a game writer how to guess a name.

The guidelines below give every engine function one predictable shape. A game writer who knows Lust should be able to guess the call before looking it up.

## The big idea: the project's names become states

The editor already knows every name a game uses: its clips, input actions, sounds, cues, assemblies and buses. So the editor writes them into one Lust file as states. Folders become options inside options. Game code `fetch`es them like any other state.

```
-- solis/project.lst, written by the editor whenever the project changes. Never edited by hand.
state Clip = [
    Farmer [Idle, Walk, Run, Swing],
    Cow [Idle, Eat]
];
state Cue = [Footstep, HoeHits, Moo];
state Action = [Move, Jump, Use, Sprint, QuickSave, QuickLoad];
state Sound = [Sfx [Footstep, Hoe], Music [Farm]];
state Assembly = [Ships [Fighter, Freighter], Crops [Wheat]];
state Bus = [Music, Effects, Voices];
pass Clip, Cue, Action, Sound, Assembly, Bus;
end
```

```
fetch "solis/project":[Clip, Action];

animate(Clip.Farmer.Walk);              -- a typo is a mistake before the game runs, with "did you mean"
is Input:pressed(Action.Use)? then { … }
```

What this gives the game writer:

- **Typos are caught.** `Clip.Farmer.Wlak` is a mistake before anything runs, not a silent nothing.
- **Renames are safe.** Rename `walk` in the editor and Lust points at every line that still uses the old name.
- **`check` covers everything.** `check clip { when Farmer.Swing { … } when Farmer { … } when Cow { … } }` must handle every clip. A new clip points at every check that needs it.
- **Folders still mean something.** `is clip == Clip.Farmer?` is true for any farmer clip, because `==` includes the options inside.
- **Nothing new to learn.** It's the same states the Lust manual already teaches.

File names are camelCase and state options are PascalCase, so the editor turns `sfx/footstep.ogg` into `Sound.Sfx.Footstep`. Names that can't become an option (starting with a number, holding a dash) get a clear message in the editor when the file is added, not in game code.

## The guidelines

1. **Inside a component, the thing's abilities are called bare.** `moveTo(place)`, `velocity()`, `playSound(Sound.Sfx.Hoe)`, as if they were the component's own. On any other thing they're methods, called with `:`: `target:push(force)`, `target:position()`. Never `moveTo(this, place)`. `this` is only for handing over the whole thing: `send Died() to this;`, `send Hit(this, 6) to target;`. A variable may share a name with an ability, so `vec3 velocity = velocity();` is fine.
2. **Things with no owner are engine objects.** Whole-game tools live on a few named objects, like Lust's own `Maths:sqrt(x)`: `Input`, `World`, `Physics`, `Audio`, `Saves`. Keep the list short and avoid Lust's own names (`Maths`, `Files`, `Time`, `Json`, `Web`, `Random`).
3. **Fixed choices are states, never strings.** Clips, actions, sounds, cues, assemblies, buses and save slots come from the project file. Strings are only for real text: what a player typed, what the game says, a name shown on screen.
4. **Anything made in an editor reaches code through a file the editor writes.** An animator's states, switches, numbers and triggers; later a behaviour tree, a material's settings, a UI screen. Each gets a Lust file with a state for its names and a class with one method per thing code can set or ask. A new engine feature plugs in the same way, so the engine keeps growing and game code never names anything with a string.
5. **Readers are nouns, writers are verbs.**
   - A value: a noun, no `get` or `Of`: `position()`, `velocity()`, `World:time()`.
   - A yes/no: a word that reads as a question after `is`: `is grounded()?`, `is Input:held(Action.Sprint)?`.
   - A change: a verb. Setting a value the reader names uses `set` + that noun: `setVelocity(v)` goes with `velocity()`. Other changes are plain verbs: `push(force)`, `moveTo(place)`, `despawn()`.
6. **One question, one call.** When the engine works out several answers at once, it gives back one thing holding them all. `RayHit hit = Physics:castRay(from, direction, 10);` then `hit.found`, `hit.distance`, `hit.entity`, `hit.normal`. Never three calls for one ray.
7. **Variants put the difference at the end of the name.** Lust has no optional arguments, so a variant is its own method, named base word first: `animate` / `animateFromStart`, `Audio:play` / `Audio:playAt` / `playSound`. Not `replayClip` next to `playClip`.
8. **Safe to call every frame.** Asking for what is already happening does nothing: `animate(Clip.Farmer.Walk)` every tick keeps walking, it does not restart. Restarting is its own method.
9. **Events say what happened, and carry states.** Past tense, PascalCase, sent to the entity it happened to: `on ClipFinished(Clip clip)`, `on CueReached(Cue cue)`, `on Landed(float speed)`. Payloads use the project's states, never a string name.
10. **The same words in every language.** Lust `moveTo(p)`, Lua `this:moveTo(p)`, Rust `this.move_to(p)`. One table in `describe.rs` names each call once; the Rust name is the same words in snake\_case. The `Host` trait uses those names too (today `position` and `positionOf` differ).
11. **Mistakes are loud and point at the line.** A wrong name is caught before the game runs. A dead entity, a missing component, or a clip the entity's animation file doesn't have stops with a message naming the line, never a silent nothing.
12. **One way to do each job.** When a better call replaces an old one, the old name goes in `RENAMED` with a warning that names the new call, and is removed at the next phase.

13) **Examples are written to be read.** In the manual, the samples and the kit: blank lines between ideas, a one-line comment per group, sums named before they're used (`float speed = velocity:length();`), and every question over several lines. Most people learn by copying, so the examples set how game code looks.

Naming at a glance:

| What | Shape | Example |
| --- | --- | --- |
| Engine object | PascalCase noun | `Input`, `Physics`, `Audio` |
| Project names | PascalCase state, folders as options | `Clip.Farmer.Walk` |
| Read a value | noun method | `velocity()` |
| Ask yes/no | adjective or verb that reads after `is` | `is grounded()?` |
| Set a value | `set` + noun | `setVelocity(v)` |
| Do something | verb | `push(force)` |
| Variant | base word + difference | `animateFromStart` |
| Event | past tense, PascalCase | `ClipFinished` |
| Several answers | a class with fields | `RayHit` |

## Animation, before and after

The animation state machine Solis is building stays exactly as planned (decision 0057): states, switches, numbers, triggers, blend spaces and the graph editor. What changes is how code reaches it. The editor writes a Lust file for each animator, so every name in the graph becomes something Lust checks.

The engine's suggestion:

```
setAnimSwitch(me, "moving", moving);
setAnimNumber(me, "speed", length(velocity));
setAnimDirection(me, "facing", velocity);
if pressed("use") { triggerAnim(me, "swing"); }
if animStateOf(me) == "Use tool" { ... }
jumpToAnimState(me, "Idle");
setAnimSpeed(me, 1.5);
```

What the editor writes from `farmer.solanim`, whenever the graph changes:

```
-- animators/farmer.lst, written by the editor from farmer.solanim. Never edited by hand.
state FarmerState = [Idle, Walk, UseTool [Swing, Water]];   -- the graph's states; groups become options inside options

class FarmerAnimator {
    has { entity owner = nothing; }
    method setMoving(bool on) { … }           -- a switch
    method setSpeed(float value) { … }        -- a number; blend spaces read these
    method setFacing(vec3 direction) { … }    -- a direction
    method swing() { … }                      -- a trigger is a verb
    method FarmerState current() { … }        -- which state the graph is in
    method jumpTo(FarmerState which) { … }
    method setPlaybackSpeed(float scale) { … }
}
pass FarmerState, FarmerAnimator;
end
```

The same farmer, using it:

```
fetch "solis/project":[Action, Cue, Sound];
fetch "solis/engine":Input;
fetch "animators/farmer":[FarmerAnimator, FarmerState];

component Farmer {
    has {
        FarmerAnimator anim;        -- the farmer's animator, filled in by the engine
    }

    method update(float dt) {
        -- How is the farmer moving right now?
        vec3 velocity = velocity();
        float speed = velocity:length();

        -- Tell the farmer's animator.
        anim:setMoving(speed > 0.1);
        anim:setSpeed(speed);
        anim:setFacing(velocity);

        -- The use button swings the hoe.
        is Input:pressed(Action.Use)? then {
            anim:swing();
        }

        -- Stand still while using a tool.
        is anim:current() == FarmerState.UseTool? then {
            setVelocity([0, 0, 0]);
        }
    }

    on CueReached(Cue cue) {
        is cue == Cue.HoeHits? then {
            playSound(Sound.Sfx.Hoe);
        }
    }
}
pass Farmer;
end
```

I checked this shape against Lust alpha 48: a class like `FarmerAnimator`, whose methods call a few general engine calls by number, works today. Rename the `moving` switch in the graph and every `anim:setMoving` becomes a mistake that says so. The `{ … }` bodies are the editor's business; game writers never see them.

Every call, old and new:

| Today or suggested | Under the guidelines | Why |
| --- | --- | --- |
| `setAnimSwitch(me, "moving", m)` | `anim:setMoving(m)` | the switch's name is a method Lust checks |
| `setAnimNumber(me, "speed", s)` | `anim:setSpeed(s)` | same, for a number |
| `setAnimDirection(me, "facing", v)` | `anim:setFacing(v)` | same, for a direction |
| `triggerAnim(me, "swing")` | `anim:swing()` | a trigger is a verb |
| `animStateOf(me) == "Use tool"` | `anim:current() == FarmerState.UseTool` | a state, so `check` and `==` work |
| `jumpToAnimState(me, "Idle")` | `anim:jumpTo(FarmerState.Idle)` | takes a state, not a string |
| `setAnimSpeed(me, 1.5)` | `anim:setPlaybackSpeed(1.5)` | `set` + noun, and can't clash with a `speed` number |
| `playClip(me, "walk")` | `animate(Clip.Door.Open)` | for things with no animator, like a door |
| `replayClip(me, "swing")` | `animateFromStart(...)` | variant: base word, then the difference |
| `stopClip(me)` | `stopAnimation()` | a verb |
| `on AnimationCue(string name)` | `on CueReached(Cue cue)` | past tense; carries a state |
| `on ClipFinished(string clip)` | `on ClipFinished(Clip clip)` | carries a state |

Two rules keep this tidy. An entity with an animator is driven through it: `animate(...)` on such an entity stops with a message saying to use its animator. And the editor refuses a switch, number or trigger whose name would clash with the class's own methods (`current`, `jumpTo`, `setPlaybackSpeed`), when it's added in the graph.

## The rest of today's API

Every call game code has today, under the same rules. Each old name goes in `RENAMED` so existing games keep building, with a warning naming the new call.

| Today | Under the guidelines |
| --- | --- |
| `positionOf(e)` / `moveTo(e, p)` | `e:position()` / `e:moveTo(p)` |
| `rotationOf(e)` / `turnTo(e, r)` | `e:rotation()` / `e:turnTo(r)` |
| `facingAlong(dir)` | `face(dir)` when it's for turning; kept as a plain maths helper otherwise |
| `nameOf(e)` | `e:name()` |
| `find("name")` | `World:find("name")` (a name typed in the editor is real text) |
| `spawn("ships/fighter.solasm", p)` / `despawn(e)` | `World:spawn(Assembly.Ships.Fighter, p)` / `e:despawn()` |
| `time()` / `frameTime()` | `World:time()` / `World:frameTime()` |
| `random()`, `randomRange`, `randomInt`, `setSeed` | `World:random()`, `World:randomRange(a, b)`, `World:randomInt(a, b)`, `World:setSeed(n)` (Lust reserves `Random`) |
| `say(words)` | `World:say(words)` |
| `fire(from, dir, speed, life, colour)` | move to the kit as a component; it's game-specific (CLAUDE.md: game systems stay out of the engine) |
| `saveGame("quick")` / `loadGame("quick")` | `Saves:save("quick")` / `Saves:load("quick")`, with the slot as text, since players name slots |
| `held` / `pressed` / `released("Jump")` | `Input:held(Action.Jump)` / `Input:pressed(...)` / `Input:released(...)` |
| `axis("Move")` | `Input:axis(Action.Move)` |
| `bindingOf("Use")` / `rebind("Use", input)` | `Input:binding(Action.Use)` / `Input:rebind(Action.Use, input)` |
| `push(e, v)` | `e:push(v)` |
| `velocityOf(e)` / `setVelocity(e, v)` | `e:velocity()` / `e:setVelocity(v)` |
| `castRay`, `castRayHit`, `castRayNormal` | one call: `RayHit hit = Physics:castRay(from, dir, reach);` |
| `moveCharacter(e, wanted)` | `e:walk(wanted)` |
| `onGround(e)` | `e:grounded()` |
| `playSound("sfx/x")` / `playSoundAt("sfx/x", p)` / `playSoundOn("sfx/x", e)` | `Audio:play(Sound.Sfx.X)` / `Audio:playAt(Sound.Sfx.X, p)` / `e:playSound(Sound.Sfx.X)` |
| `stopSoundsOn(e)` / `setSoundPlaying(e, b)` | `e:stopSounds()` / `e:setSoundPlaying(b)` |
| `setBusVolume("music", v)` | `Audio:setVolume(Bus.Music, v)` |

The engine files can stay as they are (`world`, `entity`, `physics`, `input`, `sound`, `animation`). With methods on `entity` and a few objects, most games need one line, `fetch "solis/engine":[Input, World, Physics, Audio, Saves];`, plus the project file.

## What Lust needs

Four Lust changes make all of this possible. I tested the rest against Lust alpha 48 and it already works.

| Needed | Works today? | Change |
| --- | --- | --- |
| Engine objects with native methods (`Input:pressed(...)`) | Yes | none |
| A native method giving back a class with fields (`RayHit`) | Yes | none |
| Events carrying a state (`on ClipFinished(Clip clip)`) | Yes | none |
| A native method taking or giving back a state (`animate(Clip.Farmer.Walk)`) | **No**: "a native method takes ints, floats, text…" | Lust lets natives take and give back states. Rust gets the option as a number plus its full name (`"Farmer.Walk"`); Lua gets the option as it already has it. |
| Abilities on `entity` (`target:position()`) | **No**: `entity` has only `==` and `.text()` | An engine file can give `entity` native methods, checked like any other method. The syntax is open question 1 below. |
| Abilities called bare inside a component, and `this` | **No**: today it's `me:position()` | Inside a component, the thing's abilities are called bare; a variable may share a name with one; `this` names the whole thing. `me` keeps working for a while with a message pointing to the new way. |
| Editor-made parts as fields (`FarmerAnimator anim;`) | **No**: component fields can't hold classes, and every field needs a starting value | A component can list an editor-made part with no starting value; the engine fills it in, and the editor shows it's connected. |

Both changes are inside Lust, so I'd make them in the Lust repo the usual way: one pull request each, with tests and the rulebook. Solis then moves its API over one area at a time, keeping the old names working with a warning.

## Checklist and open questions

Before any new engine call goes into Solis, it answers yes to all of these:

- [ ] It acts on one entity, so it's a method on `entity`; or it has no owner, so it's on an engine object.
- [ ] Every fixed choice it takes or gives back is a project state, not a string.
- [ ] Anything made in an editor reaches code through the file the editor writes, never names in strings.
- [ ] Its name follows the shapes: noun reader, question-word yes/no, `set` + noun, verb, variant at the end.
- [ ] Several answers come back as one class.
- [ ] Calling it every frame is safe.
- [ ] Any event it sends is past tense and carries states.
- [ ] Lust, Lua and Rust use the same words.
- [ ] A mistake stops with a message naming the line.
- [ ] The user manual shows it with a before-and-after if it replaces something.

Open questions for the project lead:

1. **How an engine file gives `entity` methods.** Only Solis's own files use this; game writers just call `me:position()`. My recommendation: `method vec3 entity:position() native;`, which reads as "entity's position". The alternative is a block, `extend entity { method vec3 position() native; }`, which groups them but adds a new word.
2. **The engine object names:** `Input`, `World`, `Physics`, `Audio`, `Saves`. Any you'd rather call something else? (`Sound` is taken by the project's sounds, and Lust reserves `Time` and `Random`.)
3. **`me:walk(wanted)` for character movement**, replacing `moveCharacter`. Or `me:moveCharacter(wanted)` if "walk" feels wrong for flying or swimming.
4. **Should Solis keep the old names working** for one phase with a warning, or switch straight over? There are few games yet, so a straight switch is cheaper.

**Answers (2026-10-28):** the project lead had no preference on questions 1 and 4, so the recommendations stand: `method vec3 entity:position() native;`, and a straight switch (an old name becomes a mistake that names the new call). Questions 2 and 3 take the recommendations too (`Input`, `World`, `Physics`, `Audio`, `Saves`; `walk(wanted)` inside a component) unless the project lead says otherwise. The four Lust changes are made in the Lust repository; Solis moves its pinned Lust forward once they land.
