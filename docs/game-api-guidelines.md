# Solis engine API guidelines for Lust

> **The source of truth for every call game code makes into Solis** ([decision 0065](decisions/0065-engine-api-guidelines.md)). Written by the project lead with the Lust chat, 2026-10-28; rule 4 rewritten by the project lead the same day. Every new engine call answers yes to the checklist at the end before it's merged, and the calls Solis had before move over in their own milestone.

Oct 28, 2026 · @Rhy

## Why these guidelines

Solis's game API today is C written in Lust's spelling. It ignores the three things Lust is built around: methods on things, states for fixed choices, and checks before anything runs.

The animation API shows all three problems:

- **Free functions that take `me` first:** `playClip(me, "walk")`, `setAnimSwitch(me, "moving", moving)`. Lust has methods for this: `me:animate(...)`.
- **Strings for names the project already knows:** clips, actions, sounds, cues, animator states. A typo like `"wlak"` builds fine and silently does nothing. Lust's whole stance is that a fixed set of choices is a `state`, so a typo is a mistake shown before the game runs.
- **A second state machine hidden behind strings:** the suggested `setAnimSwitch`, `triggerAnim`, `jumpToAnimState` and `animStateOf(me) == "Use tool"` copy Unity's Animator. That is the API Unity developers complain about most. Lust already has state machines built in, with `check` making sure every option is handled.
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

me:animate(Clip.Farmer.Walk);           -- a typo is a mistake before the game runs, with "did you mean"
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

1. **Act on a thing with its methods.** Anything that acts on one entity is a method on the entity, called with `:`. `me:moveTo(place)`, not `moveTo(me, place)`. `target:push(force)`, not `push(target, force)`.
2. **Things with no owner are engine objects.** Whole-game tools live on a few named objects, like Lust's own `Maths:sqrt(x)`: `Input`, `World`, `Physics`, `Audio`, `Saves`. Keep the list short and avoid Lust's own names (`Maths`, `Files`, `Time`, `Json`, `Web`, `Random`).
3. **Fixed choices are states, never strings.** Clips, actions, sounds, cues, assemblies, buses and save slots come from the project file. Strings are only for real text: what a player typed, what the game says, a name shown on screen.
4. **Anything made in an editor reaches code through a file the editor writes.** An animator's states, switches, numbers and triggers; later a behaviour tree, a material's settings, a UI screen. Each gets a Lust file with a state for its names and a class with one method per thing code can set or ask. A new engine feature plugs in the same way, so the engine keeps growing and game code never names anything with a string. *(Rewritten by the project lead, 2026-10-28.)*
5. **Readers are nouns, writers are verbs.**
   - A value: a noun, no `get` or `Of`: `me:position()`, `me:velocity()`, `World:time()`.
   - A yes/no: a word that reads as a question after `is`: `is me:grounded()?`, `is Input:held(Action.Sprint)?`.
   - A change: a verb. Setting a value the reader names uses `set` + that noun: `me:setVelocity(v)` goes with `me:velocity()`. Other changes are plain verbs: `me:push(force)`, `me:moveTo(place)`, `me:despawn()`.
6. **One question, one call.** When the engine works out several answers at once, it gives back one thing holding them all. `RayHit hit = Physics:castRay(from, direction, 10);` then `hit.found`, `hit.distance`, `hit.entity`, `hit.normal`. Never three calls for one ray.
7. **Variants put the difference at the end of the name.** Lust has no optional arguments, so a variant is its own method, named base word first: `animate` / `animateFromStart`, `Audio:play` / `Audio:playAt` / `me:playSound`. Not `replayClip` next to `playClip`.
8. **Safe to call every frame.** Asking for what is already happening does nothing: `me:animate(Clip.Farmer.Walk)` every tick keeps walking, it does not restart. Restarting is its own method.
9. **Events say what happened, and carry states.** Past tense, PascalCase, sent to the entity it happened to: `on ClipFinished(Clip clip)`, `on CueReached(Cue cue)`, `on Landed(float speed)`. Payloads use the project's states, never a string name.
10. **The same words in every language.** Lust `me:moveTo(p)`, Lua `me:moveTo(p)`, Rust `me.move_to(p)`. One table in `describe.rs` names each call once; the Rust name is the same words in snake\_case. The `Host` trait uses those names too (today `position` and `positionOf` differ).
11. **Mistakes are loud and point at the line.** A wrong name is caught before the game runs. A dead entity, a missing component, or a clip the entity's animation file doesn't have stops with a message naming the line, never a silent nothing.
12. **One way to do each job.** When a better call replaces an old one, the old name goes in `RENAMED` with a warning that names the new call, and is removed at the next phase.

Naming at a glance:

| What | Shape | Example |
| --- | --- | --- |
| Engine object | PascalCase noun | `Input`, `Physics`, `Audio` |
| Project names | PascalCase state, folders as options | `Clip.Farmer.Walk` |
| Read a value | noun method | `me:velocity()` |
| Ask yes/no | adjective or verb that reads after `is` | `is me:grounded()?` |
| Set a value | `set` + noun | `me:setVelocity(v)` |
| Do something | verb | `me:push(force)` |
| Variant | base word + difference | `animateFromStart` |
| Event | past tense, PascalCase | `ClipFinished` |
| Several answers | a class with fields | `RayHit` |

## Animation, before and after

> **Since rule 4 was rewritten (2026-10-28):** an animator made in the editor (`.solanim`) stays, with its states, switches, numbers and triggers. Game code reaches it through the Lust file the editor writes for it: a state of its state names and a class with one method per switch, number and trigger it has, and one asking which state it's in. The rows below that say "(gone)" for the animator's switches and numbers are replaced by that file's methods; the clip calls below still apply to sprites without an animator.

The suggested animator and today's clip calls both become a handful of methods on the entity, with clips as states. The game's own questions decide which clip plays.

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

The same farmer under the guidelines:

```
fetch "solis/project":[Clip, Cue, Action, Sound];
fetch "solis/input":Input;

component Farmer {
    method update(float dt) {
        vec3 velocity = me:velocity();
        me:face(velocity);
        is me:animation() == Clip.Farmer.Swing? then {
            -- let the swing finish; ClipFinished says when
        } or is Input:pressed(Action.Use)? then {
            me:animate(Clip.Farmer.Swing);
        } or is velocity:length() > 0.1? then {
            me:animate(Clip.Farmer.Walk);
            me:setAnimationSpeed(velocity:length() / 2);
        } or {
            me:animate(Clip.Farmer.Idle);
        }
    }

    on ClipFinished(Clip clip) {
        is clip == Clip.Farmer.Swing? then { me:animate(Clip.Farmer.Idle); }
    }

    on CueReached(Cue cue) {
        is cue == Cue.HoeHits? then { me:playSound(Sound.Sfx.Hoe); }
    }
}
pass Farmer;
end
```

Every call, old and new:

| Today or suggested | Under the guidelines | Why |
| --- | --- | --- |
| `playClip(me, "walk")` | `me:animate(Clip.Farmer.Walk)` | a method on the thing; the clip is a state |
| `replayClip(me, "swing")` | `me:animateFromStart(Clip.Farmer.Swing)` | variant: base word, then the difference |
| `stopClip(me)` | `me:stopAnimation()` | a verb |
| `on AnimationCue(string name)` | `on CueReached(Cue cue)` | past tense; carries a state |
| `on ClipFinished(string clip)` | `on ClipFinished(Clip clip)` | carries a state |
| `setAnimSwitch(me, "moving", moving)` | (gone) a question in the game: `is velocity:length() > 0.1?` | game logic stays in Lust |
| `triggerAnim(me, "swing")` | `me:animate(Clip.Farmer.Swing)` | a one-shot is a clip marked "plays once" in the editor |
| `animStateOf(me) == "Use tool"` | `me:animation() == Clip.Farmer.Swing` | a noun reader that gives back a state |
| `jumpToAnimState(me, "Idle")` | `me:animate(Clip.Farmer.Idle)` | one way to do each job |
| `setAnimNumber(me, "speed", s)` | (gone) or `me:setAnimationSpeed(s)` | no hidden parameters |
| `setAnimDirection(me, "facing", v)` | `me:face(v)` | sprites pick the matching direction; 3D turns |
| `setAnimSpeed(me, 1.5)` | `me:setAnimationSpeed(1.5)` | `set` + the noun |

Blending between clips (how long a walk-to-run takes) is set per clip pair in the editor's animation tab. If a blend that follows a number is needed later (a walk that becomes a run as speed rises), that is one clip marked "blends by a number" in the editor and one call, `me:setBlend(speed)`. It is not a named parameter.

## The rest of today's API

Every call game code has today, under the same rules. Each old name goes in `RENAMED` so existing games keep building, with a warning naming the new call.

| Today | Under the guidelines |
| --- | --- |
| `positionOf(e)` / `moveTo(e, p)` | `e:position()` / `e:moveTo(p)` |
| `rotationOf(e)` / `turnTo(e, r)` | `e:rotation()` / `e:turnTo(r)` |
| `facingAlong(dir)` | `me:face(dir)` when it's for turning; kept as a plain maths helper otherwise |
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

Two small Lust changes make all of this possible. I tested the rest against Lust alpha 48 and it already works.

| Needed | Works today? | Change |
| --- | --- | --- |
| Engine objects with native methods (`Input:pressed(...)`) | Yes | none |
| A native method giving back a class with fields (`RayHit`) | Yes | none |
| Events carrying a state (`on ClipFinished(Clip clip)`) | Yes | none |
| A native method taking or giving back a state (`me:animate(Clip.Farmer.Walk)`) | **No**: "a native method takes ints, floats, text…" | Lust lets natives take and give back states. Rust gets the option as a number plus its full name (`"Farmer.Walk"`); Lua gets the option as it already has it. |
| Methods on `entity` (`me:position()`) | **No**: `entity` has only `==` and `.text()` | An engine file can give `entity` native methods, checked like any other method. The syntax for this is open question 1 below. |

Both changes are inside Lust, so I'd make them in the Lust repo the usual way: one pull request each, with tests and the rulebook. Solis then moves its API over one area at a time, keeping the old names working with a warning.

## Checklist and open questions

Before any new engine call goes into Solis, it answers yes to all of these:

- [ ] It acts on one entity, so it's a method on `entity`; or it has no owner, so it's on an engine object.
- [ ] Every fixed choice it takes or gives back is a project state, not a string.
- [ ] Anything made in an editor that it touches reaches code through a file the editor writes (rule 4), never a string.
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

**Answers (2026-10-28):** the project lead had no preference on questions 1 and 4, so the recommendations stand: `method vec3 entity:position() native;`, and a straight switch (an old name becomes a mistake that names the new call). Questions 2 and 3 take the recommendations too (`Input`, `World`, `Physics`, `Audio`, `Saves`; `me:walk(wanted)`) unless the project lead says otherwise. The two Lust changes are made in the Lust repository; Solis moves its pinned Lust forward once they land.
