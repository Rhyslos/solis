//! The engine described to Lust ([decision 0016](../../../docs/decisions/0016-how-lust-joins-solis.md)):
//! plain Lust files, `solis/world.lst` and `solis/entity.lst`, whose
//! `native` methods have their bodies in Solis. A game uses them with
//! Lust's ordinary `get "solis/world":[say];`, and Lust checks every call
//! against them. They're made from [`NATIVES`], so they always match the
//! Solis that wrote them.

/// One native method: the file it's in, and its Lust, with a line saying
/// what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Native {
    /// The file it's described in, inside `solis/` (without `.lst`).
    pub file: &'static str,
    /// Its name, as Lust calls it.
    pub name: &'static str,
    /// Whose method it is: `""` for a method on its own (`facingAlong`),
    /// `"entity"` for an entity's (`target:moveTo(place)`, `moveTo(place)`
    /// inside a component), or an engine object's name (`World`, for
    /// `World:say(words)`). Its body's key in `LUST_NATIVES` is
    /// [`Native::key`].
    pub owner: &'static str,
    /// Its Lust: what it takes and gives back.
    pub lust: &'static str,
    /// What it does, in plain words, written above it.
    pub about: &'static str,
}

impl Native {
    /// Its body's key in `LUST_NATIVES`, as Lust's Lua calls it: the name,
    /// `entity_<name>` for an entity's method, `<Object>_<name>` for an
    /// engine object's.
    pub fn key(&self) -> String {
        match self.owner {
            "" => self.name.to_string(),
            owner => format!("{owner}_{}", self.name),
        }
    }

    /// How game code calls it, for messages and the manual:
    /// `World:say(…)`, `target:moveTo(…)`, `facingAlong(…)`.
    pub fn shown(&self) -> String {
        match self.owner {
            "" => format!("{}(…)", self.name),
            "entity" => format!("target:{}(…)", self.name),
            owner => format!("{owner}:{}(…)", self.name),
        }
    }
}

/// A native renamed on purpose, whose old name keeps working until a
/// later phase (Phase 8, C3; decision 0053): game code calling it still
/// runs, the Problems panel warns at each use, and the game-code
/// interface's snapshot lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Renamed {
    /// The file it's described in, inside `solis/` (without `.lst`).
    pub file: &'static str,
    /// The name it had.
    pub old: &'static str,
    /// The name it has now (a [`Native`]'s).
    pub new: &'static str,
    /// When the old name stops working, in plain words ("Phase 12").
    pub until: &'static str,
}

/// An event the engine sends to game code (Phase 9, D3): declared in a
/// description file, brought in with `fetch "solis/<file>":<Name>;`, and
/// handled with `on <Name>(…)` like a game's own events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineEvent {
    /// The file it's declared in, inside `solis/` (without `.lst`).
    pub file: &'static str,
    /// Its name.
    pub name: &'static str,
    /// Its Lust declaration.
    pub lust: &'static str,
    /// When it's sent, in plain words, written above it.
    pub about: &'static str,
}

/// Every event the engine sends.
pub const ENGINE_EVENTS: &[EngineEvent] = &[
    EngineEvent {
        file: "animation",
        name: "CueReached",
        lust: "event CueReached(Cue cue);",
        about: "Sent to an entity when its Sprite's clip reaches a frame with an event (Cue.Footstep), before the tick's other events. A sound event of that name also plays by itself.",
    },
    EngineEvent {
        file: "animation",
        name: "ClipFinished",
        lust: "event ClipFinished(Clip clip);",
        about: "Sent to an entity when its Sprite's clip has played all its times (a clip that plays over and over never finishes).",
    },
];

/// Natives renamed on purpose, with their old names kept working for a
/// while. None yet: the first rename adds a line here.
pub const RENAMED: &[Renamed] = &[];

/// A call game code had that's now written another way (the game API
/// guidelines, decision 0065): a straight switch, so the old way is a
/// mistake that says the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    /// What it was called.
    pub old: &'static str,
    /// How it's written in Lust now.
    pub lust: &'static str,
    /// How it's written in Lua now.
    pub lua: &'static str,
}

/// Classes the engine's files declare for natives to give back (decision
/// 0065, rule 6: one question, one call), as (file, its Lust, a line
/// saying what it is).
pub const ENGINE_CLASSES: &[(&str, &str, &str)] = &[
    (
        "physics",
        "class RayHit {\n    has {\n        bool found = false;\n        float distance = -1;\n        entity entity = nothing;\n        vec3 normal = [0, 0, 0];\n    }\n}",
        "What Physics:castRay found: whether it met a collider, how far along, whose, and which way its surface faces.",
    ),
    (
        "tiles",
        "class MapCell {\n    has {\n        bool inside = false;\n        int x = 0;\n        int y = 0;\n    }\n}",
        "A tile map's cell at a place (farm:cellAt(position())): whether the place is on the map, and the cell's column and row (counted from the top left, even off the map).",
    ),
    (
        "tiles",
        "class MapBounds {\n    has {\n        bool found = false;\n        vec3 low = [0, 0, 0];\n        vec3 high = [0, 0, 0];\n    }\n}",
        "The box a tile map covers in the world (entity:mapBounds()): whether the entity shows one, and its lowest and highest corners.",
    ),
];

/// Every call that moved.
pub const MOVED: &[Moved] = &[
    Moved {
        old: "say",
        lust: "World:say(words)",
        lua: "solis.World:say(words)",
    },
    Moved {
        old: "time",
        lust: "World:time()",
        lua: "solis.World:time()",
    },
    Moved {
        old: "frameTime",
        lust: "World:frameTime()",
        lua: "solis.World:frameTime()",
    },
    Moved {
        old: "find",
        lust: "World:find(name)",
        lua: "solis.World:find(name)",
    },
    Moved {
        old: "fire",
        lust: "World:fire(from, direction, speed, life, colour)",
        lua: "solis.World:fire(from, direction, speed, life, colour)",
    },
    Moved {
        old: "spawn",
        lust: "World:spawn(Assembly.Ships.Fighter, place), with fetch \"solis/project\":Assembly;",
        lua: "solis.World:spawn(\"Ships.Fighter\", place)",
    },
    Moved {
        old: "despawn",
        lust: "target:despawn() (inside a component, despawn() for its own entity)",
        lua: "solis.entity.despawn(target)",
    },
    Moved {
        old: "random",
        lust: "World:random()",
        lua: "solis.World:random()",
    },
    Moved {
        old: "randomRange",
        lust: "World:randomRange(low, high)",
        lua: "solis.World:randomRange(low, high)",
    },
    Moved {
        old: "randomInt",
        lust: "World:randomInt(low, high)",
        lua: "solis.World:randomInt(low, high)",
    },
    Moved {
        old: "setSeed",
        lust: "World:setSeed(seed)",
        lua: "solis.World:setSeed(seed)",
    },
    Moved {
        old: "saveGame",
        lust: "Saves:saveTo(slot)",
        lua: "solis.Saves:saveTo(slot)",
    },
    Moved {
        old: "loadGame",
        lust: "Saves:loadFrom(slot)",
        lua: "solis.Saves:loadFrom(slot)",
    },
    Moved {
        old: "positionOf",
        lust: "target:position() (inside a component, position() for its own entity)",
        lua: "solis.entity.position(target)",
    },
    Moved {
        old: "moveTo",
        lust: "target:moveTo(place) (inside a component, moveTo(place) for its own entity)",
        lua: "solis.entity.moveTo(target, place)",
    },
    Moved {
        old: "rotationOf",
        lust: "target:rotation() (inside a component, rotation() for its own entity)",
        lua: "solis.entity.rotation(target)",
    },
    Moved {
        old: "turnTo",
        lust: "target:turnTo(facing) (inside a component, turnTo(facing) for its own entity)",
        lua: "solis.entity.turnTo(target, facing)",
    },
    Moved {
        old: "nameOf",
        lust: "target:name() (inside a component, name() for its own entity)",
        lua: "solis.entity.name(target)",
    },
    Moved {
        old: "held",
        lust: "Input:held(Action.Jump), with fetch \"solis/input\":Input; and fetch \"solis/project\":Action;",
        lua: "solis.Input:held(\"Jump\")",
    },
    Moved {
        old: "pressed",
        lust: "Input:pressed(Action.Jump), with fetch \"solis/input\":Input; and fetch \"solis/project\":Action;",
        lua: "solis.Input:pressed(\"Jump\")",
    },
    Moved {
        old: "released",
        lust: "Input:released(Action.Jump), with fetch \"solis/input\":Input; and fetch \"solis/project\":Action;",
        lua: "solis.Input:released(\"Jump\")",
    },
    Moved {
        old: "axis",
        lust: "Input:axis(Action.Move), with fetch \"solis/input\":Input; and fetch \"solis/project\":Action;",
        lua: "solis.Input:axis(\"Move\")",
    },
    Moved {
        old: "bindingOf",
        lust: "Input:binding(Action.Jump)",
        lua: "solis.Input:binding(\"Jump\")",
    },
    Moved {
        old: "rebind",
        lust: "Input:rebind(Action.Jump, input)",
        lua: "solis.Input:rebind(\"Jump\", input)",
    },
    Moved {
        old: "push",
        lust: "target:push(impulse) (inside a component, push(impulse) for its own entity)",
        lua: "solis.entity.push(target, impulse)",
    },
    Moved {
        old: "velocityOf",
        lust: "target:velocity() (inside a component, velocity() for its own entity)",
        lua: "solis.entity.velocity(target)",
    },
    Moved {
        old: "setVelocity",
        lust: "target:setVelocity(velocity) (inside a component, setVelocity(velocity) for its own entity)",
        lua: "solis.entity.setVelocity(target, velocity)",
    },
    Moved {
        old: "moveCharacter",
        lust: "target:walk(wanted) (inside a component, walk(wanted) for its own entity)",
        lua: "solis.entity.walk(target, wanted)",
    },
    Moved {
        old: "onGround",
        lust: "target:grounded() (inside a component, grounded() for its own entity)",
        lua: "solis.entity.grounded(target)",
    },
    Moved {
        old: "castRay",
        lust: "RayHit hit = Physics:castRay(from, direction, reach); then hit.found and hit.distance, with fetch \"solis/physics\":[Physics, RayHit];",
        lua: "solis.Physics:castRay(from, direction, reach).distance",
    },
    Moved {
        old: "castRayHit",
        lust: "Physics:castRay(from, direction, reach).entity",
        lua: "solis.Physics:castRay(from, direction, reach).entity",
    },
    Moved {
        old: "castRayNormal",
        lust: "Physics:castRay(from, direction, reach).normal",
        lua: "solis.Physics:castRay(from, direction, reach).normal",
    },
    Moved {
        old: "playSound",
        lust: "Audio:play(Sound.Sfx.Footstep) for sfx/footstep, with fetch \"solis/sound\":Audio; and fetch \"solis/project\":Sound;",
        lua: "solis.Audio:play(\"Sfx.Footstep\")",
    },
    Moved {
        old: "playSoundAt",
        lust: "Audio:playAt(Sound.Sfx.Footstep, place)",
        lua: "solis.Audio:playAt(\"Sfx.Footstep\", place)",
    },
    Moved {
        old: "playSoundOn",
        lust: "target:playSound(Sound.Sfx.Footstep) (inside a component, playSound(…) for its own entity)",
        lua: "solis.entity.playSound(target, \"Sfx.Footstep\")",
    },
    Moved {
        old: "stopSoundsOn",
        lust: "target:stopSounds() (inside a component, stopSounds() for its own entity)",
        lua: "solis.entity.stopSounds(target)",
    },
    Moved {
        old: "setBusVolume",
        lust: "Audio:setVolume(Bus.Music, volume), with fetch \"solis/project\":Bus;",
        lua: "solis.Audio:setVolume(\"Music\", volume)",
    },
    Moved {
        old: "playClip",
        lust: "target:animate(Clip.Sprites.Farmer.Walk) (inside a component, animate(…) for its own entity)",
        lua: "solis.entity.animate(target, \"Sprites.Farmer.Walk\")",
    },
    Moved {
        old: "replayClip",
        lust: "target:animateFromStart(Clip.Sprites.Farmer.Walk) (inside a component, animateFromStart(…))",
        lua: "solis.entity.animateFromStart(target, \"Sprites.Farmer.Walk\")",
    },
    Moved {
        old: "stopClip",
        lust: "target:stopAnimation() (inside a component, stopAnimation())",
        lua: "solis.entity.stopAnimation(target)",
    },
    Moved {
        old: "AnimationCue",
        lust: "on CueReached(Cue cue), with fetch \"solis/animation\":CueReached; and fetch \"solis/project\":Cue;",
        lua: "function C:on_CueReached(cue)",
    },
];

/// Every native method Solis gives game code.
pub const NATIVES: &[Native] = &[
    Native {
        file: "world",
        name: "say",
        owner: "World",
        lust: "method say(string words) native;",
        about: "Writes words in the Console.",
    },
    Native {
        file: "world",
        name: "time",
        owner: "World",
        lust: "method float time() native;",
        about: "How long the game has been running, in seconds of game time.",
    },
    Native {
        file: "world",
        name: "find",
        owner: "World",
        lust: "method entity find(string name) native;",
        about: "The first entity with this name, in the Area panel's order, or nothing.",
    },
    Native {
        file: "world",
        name: "fire",
        owner: "World",
        lust: "method fire(vec3 from, vec3 direction, float speed, float life, vec3 colour) native;",
        about: "Fires a projectile (a laser): from a place, flying a way at a speed in metres a second, for a life in seconds, glowing in a colour (red, green, blue; above 1 glows brighter).",
    },
    Native {
        file: "world",
        name: "spawn",
        owner: "World",
        lust: "method entity spawn(Assembly which, vec3 place) native;",
        about: "Makes a copy of an assembly (one of the project's, Assembly.Ships.Fighter for ships/fighter) at a place; its entity, which is in the game from the next tick, or nothing if it can't be made.",
    },
    Native {
        file: "world",
        name: "despawn",
        owner: "entity",
        lust: "method entity:despawn() native;",
        about: "Takes the entity, and everything under it, out of the game at the end of the tick.",
    },
    Native {
        file: "world",
        name: "frameTime",
        owner: "World",
        lust: "method float frameTime() native;",
        about: "How long the last frame took, in seconds of real time (1/60 at 60 frames a second).",
    },
    Native {
        file: "world",
        name: "viewSize",
        owner: "World",
        lust: "method vec3 viewSize() native;",
        about: "How much of the world a 2D camera shows, in metres: x across, y up (the project's 2D settings).",
    },
    Native {
        file: "world",
        name: "random",
        owner: "World",
        lust: "method float random() native;",
        about: "A random number from 0 up to 1. The same game with the same seed gets the same numbers, on any computer.",
    },
    Native {
        file: "world",
        name: "randomRange",
        owner: "World",
        lust: "method float randomRange(float low, float high) native;",
        about: "A random number from low up to high.",
    },
    Native {
        file: "world",
        name: "randomInt",
        owner: "World",
        lust: "method int randomInt(int low, int high) native;",
        about: "A random whole number from low to high, both included (World:randomInt(1, 6) rolls a die).",
    },
    Native {
        file: "world",
        name: "setGameSpeed",
        owner: "World",
        lust: "method setGameSpeed(float speed) native;",
        about: "Sets how fast the game runs, at the end of the tick: 0 pauses it (time, physics, projectiles and sprites stop, and dt is 0, but components still run, so a pause menu works), 1 is as usual, up to 4.",
    },
    Native {
        file: "world",
        name: "gameSpeed",
        owner: "World",
        lust: "method float gameSpeed() native;",
        about: "How fast the game runs, as the tick started: 1 as usual, 0 paused.",
    },
    Native {
        file: "world",
        name: "timeOfDay",
        owner: "World",
        lust: "method float timeOfDay() native;",
        about: "The game's hour as the tick started: 0 up to 24 (19.5 is half past seven in the evening). It runs with the game's time, a day as long as the Environment's Day and night says.",
    },
    Native {
        file: "world",
        name: "setTimeOfDay",
        owner: "World",
        lust: "method setTimeOfDay(float hour) native;",
        about: "Sets the game's hour, at the end of the tick, and the day carries on from there: World:setTimeOfDay(6) after a night's sleep. Kept in save games.",
    },
    Native {
        file: "world",
        name: "setSeed",
        owner: "World",
        lust: "method setSeed(int seed) native;",
        about: "Starts the game's random numbers again from a seed, at the end of the tick: the same seed gives the same numbers again.",
    },
    Native {
        file: "world",
        name: "saveTo",
        owner: "Saves",
        lust: "method saveTo(string slot) native;",
        about: "Saves the whole game in a slot (a name such as \"quick\" or \"slot_1\": letters, digits, _ and -), at the end of the tick: every entity, every component's values, the lasers, physics, the clock and the random numbers.",
    },
    Native {
        file: "world",
        name: "loadFrom",
        owner: "Saves",
        lust: "method loadFrom(string slot) native;",
        about: "Loads the game saved in a slot, at the end of the tick: everything is put back exactly as it was when it was saved.",
    },
    Native {
        file: "sound",
        name: "play",
        owner: "Audio",
        lust: "method play(Sound event) native;",
        about: "Plays one of the project's sound events (Sound.Sfx.Footstep for sfx/footstep), the same everywhere: music, the interface. At the end of the tick, as a move lands. Nothing comes back, so sound can never change the game.",
    },
    Native {
        file: "sound",
        name: "playAt",
        owner: "Audio",
        lust: "method playAt(Sound event, vec3 place) native;",
        about: "Plays a sound event at a place in the world: quieter far from the camera, and from its direction.",
    },
    Native {
        file: "sound",
        name: "playSound",
        owner: "entity",
        lust: "method entity:playSound(Sound event) native;",
        about: "Plays a sound event on the entity, following it as it moves: an engine, a voice.",
    },
    Native {
        file: "sound",
        name: "stopSounds",
        owner: "entity",
        lust: "method entity:stopSounds() native;",
        about: "Stops the sounds playing on the entity, with a short fade, at the end of the tick (not its AudioSource: setSoundPlaying does that).",
    },
    Native {
        file: "sound",
        name: "setSoundPlaying",
        owner: "entity",
        lust: "method entity:setSoundPlaying(bool playing) native;",
        about: "Turns an entity's AudioSource (a sound that keeps playing at it: an engine, a campfire) on or off, at the end of the tick.",
    },
    Native {
        file: "sound",
        name: "setVolume",
        owner: "Audio",
        lust: "method setVolume(Bus bus, float volume) native;",
        about: "Sets a bus's volume for the rest of the play, from 0 (silent) to 1 (as set in the Mixer), at the end of the tick: what an options menu's sliders call.",
    },
    Native {
        file: "animation",
        name: "animate",
        owner: "entity",
        lust: "method entity:animate(Clip clip) native;",
        about: "Plays a clip on the entity's Sprite (Clip.Sprites.Farmer.Walk), at the end of the tick: already playing it (or finished it), it carries on as it is, so calling it every tick is safe; another clip starts from its beginning.",
    },
    Native {
        file: "animation",
        name: "animateFromStart",
        owner: "entity",
        lust: "method entity:animateFromStart(Clip clip) native;",
        about: "Plays a clip on the entity's Sprite from its beginning, whatever it was doing.",
    },
    Native {
        file: "animation",
        name: "stopAnimation",
        owner: "entity",
        lust: "method entity:stopAnimation() native;",
        about: "Stops the entity's Sprite where it is in its clip.",
    },
    Native {
        file: "animation",
        name: "setFlip",
        owner: "entity",
        lust: "method entity:setFlip(bool across, bool down) native;",
        about: "Mirrors the entity's Sprite at the end of the tick: across (facing the other way, left for right) and down (upside down). One clip then faces either way: setFlip(true, false) to face left.",
    },
    Native {
        file: "animation",
        name: "setNumber",
        owner: "Animators",
        lust: "method setNumber(entity owner, string name, float value) native;",
        about: "Sets a number of the owner's animator. For the animators' own files (solis/animators/): game code uses the animator's class, anim:setSpeed(…).",
    },
    Native {
        file: "animation",
        name: "setSwitch",
        owner: "Animators",
        lust: "method setSwitch(entity owner, string name, bool on) native;",
        about: "Sets a switch of the owner's animator (for the animators' own files).",
    },
    Native {
        file: "animation",
        name: "fire",
        owner: "Animators",
        lust: "method fire(entity owner, string name) native;",
        about: "Fires a trigger of the owner's animator (for the animators' own files).",
    },
    Native {
        file: "animation",
        name: "setDirection",
        owner: "Animators",
        lust: "method setDirection(entity owner, string name, vec3 direction) native;",
        about: "Sets a direction of the owner's animator, on the ground (for the animators' own files).",
    },
    Native {
        file: "animation",
        name: "stateOf",
        owner: "Animators",
        lust: "method string stateOf(entity owner) native;",
        about: "The state the owner's animator is in, by its name in the graph (for the animators' own files).",
    },
    Native {
        file: "animation",
        name: "jumpTo",
        owner: "Animators",
        lust: "method jumpTo(entity owner, string named) native;",
        about: "Puts the owner's animator straight into a state, by its name in the graph (for the animators' own files).",
    },
    Native {
        file: "animation",
        name: "setPlaybackSpeed",
        owner: "Animators",
        lust: "method setPlaybackSpeed(entity owner, float scale) native;",
        about: "How fast the owner's animator plays its clips: 1 as made (for the animators' own files).",
    },
    Native {
        file: "tiles",
        name: "width",
        owner: "Maps",
        lust: "method int width(string tileMap) native;",
        about: "How many cells across a map is. For the maps' own files (solis/maps/): game code uses the map's class, farm:width().",
    },
    Native {
        file: "tiles",
        name: "height",
        owner: "Maps",
        lust: "method int height(string tileMap) native;",
        about: "How many cells up a map is (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "terrain",
        owner: "Maps",
        lust: "method int terrain(string tileMap, int layer, int x, int y) native;",
        about: "The terrain's code on a layer's cell, 0 for empty (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "setTerrain",
        owner: "Maps",
        lust: "method setTerrain(string tileMap, int layer, int x, int y, int code) native;",
        about: "Paints a layer's cell with a terrain's code, 0 to clear it, at the end of the tick (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "number",
        owner: "Maps",
        lust: "method int number(string tileMap, int value, int x, int y) native;",
        about: "A number value on a cell (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "setNumber",
        owner: "Maps",
        lust: "method setNumber(string tileMap, int value, int x, int y, int to) native;",
        about: "Sets a number value on a cell, at the end of the tick (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "isOn",
        owner: "Maps",
        lust: "method bool isOn(string tileMap, int value, int x, int y) native;",
        about: "A switch value on a cell (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "setSwitch",
        owner: "Maps",
        lust: "method setSwitch(string tileMap, int value, int x, int y, bool on) native;",
        about: "Sets a switch value on a cell, at the end of the tick (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "choice",
        owner: "Maps",
        lust: "method int choice(string tileMap, int value, int x, int y) native;",
        about: "A choice value on a cell, by its option's place, -1 for none (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "setChoice",
        owner: "Maps",
        lust: "method setChoice(string tileMap, int value, int x, int y, int chosen) native;",
        about: "Sets a choice value on a cell, by its option's place, -1 for none, at the end of the tick (for the maps' own files).",
    },
    Native {
        file: "tiles",
        name: "cellAt",
        owner: "Maps",
        lust: "method MapCell cellAt(string tileMap, vec3 place) native;",
        about: "The cell of a map at a place in the world (for the maps' own files: game code uses farm:cellAt(place)).",
    },
    Native {
        file: "tiles",
        name: "placeOf",
        owner: "Maps",
        lust: "method vec3 placeOf(string tileMap, int x, int y) native;",
        about: "The place in the world at a cell's middle (for the maps' own files: farm:placeOf(x, y)).",
    },
    Native {
        file: "tiles",
        name: "mapBounds",
        owner: "entity",
        lust: "method MapBounds entity:mapBounds() native;",
        about: "The box the tile map on this entity covers in the world, as the tick started: a 2D camera stays inside it.",
    },
    Native {
        file: "input",
        name: "held",
        owner: "Input",
        lust: "method bool held(Action action) native;",
        about: "Whether an action's input is held down this tick (\"Jump\"; an axis such as \"Move\": not at rest). Actions are set up in Edit > Input actions.",
    },
    Native {
        file: "input",
        name: "pressed",
        owner: "Input",
        lust: "method bool pressed(Action action) native;",
        about: "Whether an action's input went down since the last tick: true once per press.",
    },
    Native {
        file: "input",
        name: "released",
        owner: "Input",
        lust: "method bool released(Action action) native;",
        about: "Whether an action's input came up since the last tick: true once per release.",
    },
    Native {
        file: "input",
        name: "axis",
        owner: "Input",
        lust: "method vec3 axis(Action action) native;",
        about: "An axis action's two numbers as x (across: right is +1) and y (up or forward: +1); z is 0. Move gives which way to go, each from -1 to 1; Look gives how far to turn this tick, in degrees.",
    },
    Native {
        file: "input",
        name: "binding",
        owner: "Input",
        lust: "method string binding(Action action) native;",
        about: "The input that sets an action off (a key first, then the mouse, then the gamepad), such as \"F\": for telling the player \"Press F\". Empty if it has none.",
    },
    Native {
        file: "input",
        name: "rebind",
        owner: "Input",
        lust: "method rebind(Action action, string input) native;",
        about: "Binds an input (\"E\", \"Mouse Right\", \"Pad Y\") to an action from the next tick, in place of its inputs of the same kind: what a controls menu does. Lasts while the game plays.",
    },
    Native {
        file: "physics",
        name: "push",
        owner: "entity",
        lust: "method entity:push(vec3 impulse) native;",
        about: "Gives the entity's body a push (kilograms times metres a second), made at the next physics step: a kick, an explosion.",
    },
    Native {
        file: "physics",
        name: "velocity",
        owner: "entity",
        lust: "method vec3 entity:velocity() native;",
        about: "How fast and which way the entity's body moves, in metres a second; for a 2D character, how fast its last tick's move went (none without a body or a 2D character).",
    },
    Native {
        file: "physics",
        name: "setVelocity",
        owner: "entity",
        lust: "method entity:setVelocity(vec3 velocity) native;",
        about: "Sets how fast and which way the entity's body moves, at the next physics step.",
    },
    Native {
        file: "physics",
        name: "castRay",
        owner: "Physics",
        lust: "method RayHit castRay(vec3 from, vec3 direction, float reach) native;",
        about: "Looks along a line from a place, a way, up to reach metres, and gives back the first collider it meets: hit.found, hit.distance, hit.entity and hit.normal (which way its surface faces). Nothing found: found is false and distance -1.",
    },
    Native {
        file: "physics",
        name: "walk",
        owner: "entity",
        lust: "method entity:walk(vec3 wanted) native;",
        about: "Walks a character (a CharacterBody, or a Collider2D ticked Character) by wanted metres this tick, as far as it can: climbing steps, sliding along walls.",
    },
    Native {
        file: "physics",
        name: "grounded",
        owner: "entity",
        lust: "method bool entity:grounded() native;",
        about: "Whether a character stood on something after its last move (a 2D character: whether a floor under it stopped it going down): for jumping only from the ground, and for falling.",
    },
    Native {
        file: "entity",
        name: "position",
        owner: "entity",
        lust: "method vec3 entity:position() native;",
        about: "Where the entity is in the world, in metres.",
    },
    Native {
        file: "entity",
        name: "moveTo",
        owner: "entity",
        lust: "method entity:moveTo(vec3 place) native;",
        about: "Puts the entity at a place in the world, in metres.",
    },
    Native {
        file: "entity",
        name: "rotation",
        owner: "entity",
        lust: "method rotation entity:rotation() native;",
        about: "Which way the entity faces in the world.",
    },
    Native {
        file: "entity",
        name: "turnTo",
        owner: "entity",
        lust: "method entity:turnTo(rotation facing) native;",
        about: "Turns the entity to face this way in the world.",
    },
    Native {
        file: "entity",
        name: "facingAlong",
        owner: "",
        lust: "method rotation facingAlong(vec3 direction) native;",
        about: "The upright rotation that faces along a direction (an entity's forward is +Y), for working out a turn. To turn the entity itself, face(direction) does it in one.",
    },
    Native {
        file: "entity",
        name: "face",
        owner: "entity",
        lust: "method entity:face(vec3 direction) native;",
        about: "Turns the entity, upright, to face along a direction (its forward is +Y): face(target:position() - position()) turns to face a place.",
    },
    Native {
        file: "entity",
        name: "name",
        owner: "entity",
        lust: "method string entity:name() native;",
        about: "The entity's name, as the Area panel shows it.",
    },
    Native {
        file: "screens",
        name: "showScreen",
        owner: "entity",
        lust: "method entity:showScreen(Screen which) native;",
        about: "Shows one of the game's screens (a file in ui/) on the entity's Screen, at the end of the tick: showScreen(Screen.PauseMenu). An entity without a Screen is given one. An entity shows one screen at a time: use another entity for another.",
    },
    Native {
        file: "screens",
        name: "hideScreen",
        owner: "entity",
        lust: "method entity:hideScreen(Screen which) native;",
        about: "Hides the screen, at the end of the tick, if the entity's Screen shows it.",
    },
    Native {
        file: "screens",
        name: "screenShown",
        owner: "entity",
        lust: "method bool entity:screenShown(Screen which) native;",
        about: "Whether the entity's Screen shows this screen, as the tick started.",
    },
    Native {
        file: "screens",
        name: "textSize",
        owner: "Screens",
        lust: "method float textSize() native;",
        about: "The player's text size: every screen's text times this (1 as designed, 1.5 half as big again; 0.5 to 3).",
    },
    Native {
        file: "screens",
        name: "setTextSize",
        owner: "Screens",
        lust: "method setTextSize(float size) native;",
        about: "Sets the player's text size, at the end of the tick: every screen's text grows or shrinks, and the pieces round it make room. Kept for the next time the game starts.",
    },
    Native {
        file: "screens",
        name: "uiSize",
        owner: "Screens",
        lust: "method float uiSize() native;",
        about: "The player's UI size: every screen times this (1 as designed; 0.5 to 2).",
    },
    Native {
        file: "screens",
        name: "setUiSize",
        owner: "Screens",
        lust: "method setUiSize(float size) native;",
        about: "Sets the player's UI size, at the end of the tick: every screen grows or shrinks. Kept for the next time the game starts.",
    },
    Native {
        file: "window",
        name: "fullscreen",
        owner: "Window",
        lust: "method bool fullscreen() native;",
        about: "Whether the game fills the screen.",
    },
    Native {
        file: "window",
        name: "setFullscreen",
        owner: "Window",
        lust: "method setFullscreen(bool on) native;",
        about: "Makes the game fill the screen (true) or sit in a window (false), at the end of the tick. Kept for the next time the game starts. While playing in the editor it only says so in the Console.",
    },
    Native {
        file: "window",
        name: "resolution",
        owner: "Window",
        lust: "method vec3 resolution() native;",
        about: "The window's size in pixels when it isn't fullscreen: x across, y down.",
    },
    Native {
        file: "window",
        name: "setResolution",
        owner: "Window",
        lust: "method setResolution(int width, int height) native;",
        about: "Sets the window's size in pixels (each 320 to 16384), at the end of the tick. Kept for the next time the game starts. While playing in the editor it only says so in the Console.",
    },
    Native {
        file: "window",
        name: "useNextResolution",
        owner: "Window",
        lust: "method useNextResolution() native;",
        about: "Switches the window to the next usual size (1280 × 720, 1600 × 900, 1920 × 1080, 2560 × 1440, 3440 × 1440, 3840 × 2160, then round again), at the end of the tick: for a menu's Resolution button.",
    },
];

/// The description files, and the ready-made components in `kit/`
/// ([`KIT`]), as (path inside `assets`, text), with `/`
/// between folders, in `folder` (the program's name in small letters, from
/// the app's branding: `solis`). `made_by` names the program and version
/// that wrote them, for their first line.
///
/// They include `project.lst`, the project's names as states (here for a
/// project with no assemblies and the ready-made input actions:
/// [`descriptions_for`] gives a project's own).
pub fn descriptions(made_by: &str, folder: &str) -> Vec<(String, String)> {
    descriptions_with(
        made_by,
        folder,
        RENAMED,
        &crate::project_names::Names::standard(),
    )
    .0
}

/// [`descriptions`] for a project with these assemblies (their paths in
/// the assets, without `.solassembly`), with notes on any game code can't
/// name.
pub fn descriptions_for(
    made_by: &str,
    folder: &str,
    names: &crate::project_names::Names,
) -> (Vec<(String, String)>, Vec<String>) {
    descriptions_with(made_by, folder, RENAMED, names)
}

/// [`descriptions_for`], with these natives renamed.
fn descriptions_with(
    made_by: &str,
    folder: &str,
    renamed: &[Renamed],
    names: &crate::project_names::Names,
) -> (Vec<(String, String)>, Vec<String>) {
    let (project, notes) = crate::project_names::project_file(made_by, names);
    // Each file once, in the order they first come.
    let mut names: Vec<&str> = Vec::new();
    for file in NATIVES.iter().map(|n| n.file) {
        if !names.contains(&file) {
            names.push(file);
        }
    }
    let files = names
        .into_iter()
        .map(|file| {
            let mine: Vec<&Native> = NATIVES.iter().filter(|n| n.file == file).collect();
            let mut text = format!(
                "-- Made by {made_by}{ENGINE_FILE} It's written again\n\
                 -- whenever the project opens, so changes made here don't last.\n\n"
            );
            // The project's names it takes (Assembly, say), from the file
            // the editor writes beside these.
            let wanted: Vec<&str> = PROJECT_STATES
                .iter()
                .copied()
                .filter(|state| {
                    let takes = |lust: &str| {
                        lust.contains(&format!("({state} "))
                            || lust.contains(&format!(", {state} "))
                    };
                    mine.iter().any(|n| takes(n.lust))
                        || ENGINE_EVENTS
                            .iter()
                            .any(|e| e.file == file && takes(e.lust))
                })
                .collect();
            if !wanted.is_empty() {
                text.push_str(&format!(
                    "fetch \"{folder}/project\":[{}];\n\n",
                    wanted.join(", ")
                ));
            }
            // Its classes first (what natives give back), then methods on
            // their own and the entity's, then each engine object's in a
            // block of its own.
            let mut names: Vec<&str> = Vec::new();
            for (_, lust, about) in ENGINE_CLASSES.iter().filter(|(f, _, _)| *f == file) {
                text.push_str(&format!("-- {about}\n{lust}\n\n"));
                if let Some(name) = lust
                    .strip_prefix("class ")
                    .and_then(|l| l.split_whitespace().next())
                {
                    names.push(name);
                }
            }
            for native in mine
                .iter()
                .filter(|n| n.owner.is_empty() || n.owner == "entity")
            {
                text.push_str(&format!("-- {}\n{}\n\n", native.about, native.lust));
                if native.owner.is_empty() {
                    names.push(native.name);
                }
            }
            let mut objects: Vec<&str> = Vec::new();
            for owner in mine.iter().map(|n| n.owner) {
                if !owner.is_empty() && owner != "entity" && !objects.contains(&owner) {
                    objects.push(owner);
                }
            }
            for object in objects {
                text.push_str(&format!("object {object} {{\n"));
                for native in mine.iter().filter(|n| n.owner == object) {
                    text.push_str(&format!("    -- {}\n    {}\n\n", native.about, native.lust));
                }
                text.truncate(text.trim_end().len());
                text.push_str("\n}\n\n");
                names.push(object);
            }
            for event in ENGINE_EVENTS.iter().filter(|e| e.file == file) {
                text.push_str(&format!("-- {}\n{}\n\n", event.about, event.lust));
                names.push(event.name);
            }
            // Old names of renamed natives: the same method under its old
            // name, so code using it still checks and runs.
            for renamed in renamed.iter().filter(|r| r.file == file) {
                if let Some(native) = NATIVES.iter().find(|n| n.name == renamed.new) {
                    text.push_str(&format!(
                        "-- Old name of {} (renamed; it stops working in {}).\n{}\n\n",
                        renamed.new,
                        renamed.until,
                        renamed_lust(native.lust, renamed.new, renamed.old)
                    ));
                    names.push(renamed.old);
                }
            }
            if names.is_empty() {
                text.push_str("end\n");
            } else {
                text.push_str(&format!("pass {};\nend\n", names.join(", ")));
            }
            (format!("{folder}/{file}.lst"), text)
        })
        .chain(std::iter::once((format!("{folder}/project.lst"), project)))
        .chain(DATA_CLASSES.iter().map(|(kind, classes)| {
            (
                format!("{folder}/{}/{kind}.lst", area::lust_data::CLASSES_FOLDER),
                format!(
                    "-- Made by {made_by}: what a {kind} file holds, which the files Solis makes\n\
                     -- fetch. It's written again whenever the project opens, so changes made here\n\
                     -- don't last.\n{classes}"
                ),
            )
        }))
        .chain(KIT.iter().map(|(file, body)| {
            (
                format!("{folder}/kit/{file}"),
                format!(
                    "-- Made by {made_by}: a ready-made component, to use as it is or to read and\n\
                     -- copy. It's written again whenever the project opens, so changes made here\n\
                     -- don't last: to change it, copy the file outside {folder}/ and rename its\n\
                     -- components. Use it with fetch \"{folder}/kit/{}\":[…];\n\n{body}",
                    file.trim_end_matches(".lst")
                ),
            )
        }))
        .collect();
    (files, notes)
}

/// What each kind of Lust data file Solis makes holds (decision 0081),
/// written into `solis/data/` so Ledger and `lust check` can follow the
/// files' `fetch`.
pub const DATA_CLASSES: &[(&str, &str)] = &[
    (tiles::map_lust::KIND, tiles::map_lust::CLASSES),
    (animation::atlas::KIND, animation::atlas::CLASSES),
    (tiles::tileset::KIND, tiles::tileset::CLASSES),
];

/// The project's own names as states, written by the editor into
/// `solis/project.lst` ([`crate::project_names::project_file`]): what natives take where a string
/// once named a file.
pub const PROJECT_STATES: &[&str] = &[
    "Assembly", "Action", "Sound", "Bus", "Clip", "Cue", "Screen", "Picture",
];

/// What the first line of each file describing the engine says after who made it: how the
/// compiler knows the engine's own folder (Lust's host folder, decision 0072), without the
/// program's name written into the code.
pub const ENGINE_FILE: &str = ": what game code can ask of the engine.";

/// The engine's own folders among `files`: the top folder of each file describing the engine
/// (its first line says so, [`ENGINE_FILE`]). Lust lets only their files give `entity`
/// methods, and makes them part of every program.
pub fn engine_folders(files: &[(String, String)]) -> Vec<String> {
    let mut folders: Vec<String> = files
        .iter()
        .filter(|(_, text)| {
            text.lines()
                .next()
                .is_some_and(|l| l.starts_with("-- Made by ") && l.contains(ENGINE_FILE))
        })
        .filter_map(|(path, _)| path.split_once('/').map(|(top, _)| top.to_string()))
        .collect();
    folders.sort();
    folders.dedup();
    folders
}

/// `lust`, a native's method line, with its name `new` changed to `old`.
fn renamed_lust(lust: &str, new: &str, old: &str) -> String {
    lust.replacen(&format!(" {new}("), &format!(" {old}("), 1)
}

/// The ready-made components (Phase 6, M5), written into every project
/// beside the descriptions: Lust a game uses as it is, or reads and
/// copies.
pub const KIT: &[(&str, &str)] = &[
    ("character.lst", include_str!("kit/character.lst")),
    ("topdown.lst", include_str!("kit/topdown.lst")),
    ("cameras.lst", include_str!("kit/cameras.lst")),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_descriptions_check_cleanly_and_a_game_can_use_them() {
        let mut files = descriptions("Test Engine 9.9.9", "solis");
        assert_eq!(files.len(), 16);
        assert!(files.iter().any(|(p, _)| p == "solis/project.lst"));
        assert!(files.iter().any(|(p, _)| p == "solis/data/tilemap.lst"));
        assert!(files[0].1.starts_with("-- Made by Test Engine 9.9.9"));
        files.push((
            "main.lst".into(),
            "fetch \"solis/world\":[World, Saves];\n\
             fetch \"solis/project\":[Assembly, Action, Sound, Bus, Clip, Cue, Screen, Picture];\n\
             fetch \"solis/input\":Input;\n\
             get \"solis/entity\":facingAlong;\n\
             fetch \"solis/physics\":[Physics, RayHit];\n\
             fetch \"solis/sound\":Audio;\n\
             fetch \"solis/animation\":[CueReached, ClipFinished];\n\
             -- A game's own class may share a name with a data file's class.\n\
             class Layer {\n\
                 has {\n\
                     int depth = 0;\n\
                 }\n\
             }\n\
             component Walker {\n\
                 has {\n\
                     int steps = 0;\n\
                 }\n\
                 on CueReached(Cue cue) {\n\
                     is cue != Cue.None? then {\n\
                         steps = steps + 1;\n\
                     }\n\
                 }\n\
                 on ClipFinished(Clip clip) {\n\
                     is clip == Clip.None? then {\n\
                         animate(Clip.None);\n\
                         animateFromStart(clip);\n\
                     }\n\
                     stopAnimation();\n\
                 }\n\
             }\n\
             component Mover {\n\
                 has {\n\
                     float speed = 2;\n\
                 }\n\
                 method update(float dt) {\n\
                     vec3 step = [1, 0, 0];\n\
                     moveTo(position() + step * speed * dt);\n\
                     turnTo(rotation());\n\
                     face(step);\n\
                     turnTo(facingAlong(step));\n\
                     is World:time() > 10 && World:frameTime() > 0? then {\n\
                         World:say(name() + \" has gone far\");\n\
                         Saves:saveTo(\"quick\");\n\
                         Saves:loadFrom(\"quick\");\n\
                     }\n\
                     entity other = World:find(\"Crate\");\n\
                     entity made = World:spawn(Assembly.None, other:position());\n\
                     made:moveTo(other:position() + [0, 0, 1]);\n\
                     made:turnTo(other:rotation());\n\
                     World:say(made:name());\n\
                     made:despawn();\n\
                     World:setSeed(World:randomInt(1, 6));\n\
                     float r = World:random() + World:randomRange(1, 2);\n\
                     World:fire(position(), step, 10, 1, [1, 0, 0]);\n\
                     Audio:play(Sound.None);\n\
                     Audio:playAt(Sound.None, position());\n\
                     playSound(Sound.None);\n\
                     other:stopSounds();\n\
                     setSoundPlaying(true);\n\
                     Audio:setVolume(Bus.Music, 0.5);\n\
                     is Input:held(Action.Jump) && Input:pressed(Action.Jump) || Input:released(Action.Jump)? then {\n\
                         Input:rebind(Action.Jump, \"Space\");\n\
                         World:say(Input:binding(Action.Jump) + Input:axis(Action.Move).x.text());\n\
                     }\n\
                     RayHit hit = Physics:castRay(position(), step, 10);\n\
                     other:push(velocity() + hit.normal);\n\
                     is hit.found && hit.distance > 0 && hit.entity == other? then {\n\
                         other:setVelocity(step);\n\
                         setVelocity(other:velocity());\n\
                         walk(step);\n\
                         is grounded() || other:grounded()? then {\n\
                             other:walk(step);\n\
                         }\n\
                     }\n\
                 }\n\
             }\n\
             pass Mover, Walker;\n\
             end\n"
                .into(),
        ));
        let engine = lust::Engine {
            folders: engine_folders(&files),
        };
        assert_eq!(engine.folders, ["solis"]);
        if let Err(problems) = lust::check_with(&engine, "main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
    }

    #[test]
    fn the_engines_folder_is_found_from_its_files_first_lines() {
        let mut files = descriptions("Test Engine 9.9.9", "engine");
        assert_eq!(engine_folders(&files), ["engine"]);
        // A game's own files, a top-level one with the line, and one where it isn't first:
        // none of them makes a folder the engine's.
        files.push(("game/a.lst".into(), "end\n".into()));
        files.push((
            "loose.lst".into(),
            format!("-- Made by me{ENGINE_FILE}\nend\n"),
        ));
        files.push((
            "other/b.lst".into(),
            format!("end\n-- Made by me{ENGINE_FILE}\n"),
        ));
        assert_eq!(engine_folders(&files), ["engine"]);
        assert!(engine_folders(&[]).is_empty());
    }

    #[test]
    fn the_ready_made_components_check_cleanly() {
        let mut files = descriptions("Test Engine 9.9.9", "solis");
        assert!(files.iter().any(|(p, _)| p == "solis/kit/character.lst"));
        files.push((
            "main.lst".into(),
            "fetch \"solis/kit/character\":[CharacterController];\n\
             fetch \"solis/kit/cameras\":[ThirdPersonCamera, FirstPersonCamera, RtsCamera];\n\
             end\n"
                .into(),
        ));
        let engine = lust::Engine {
            folders: engine_folders(&files),
        };
        assert_eq!(engine.folders, ["solis"]);
        if let Err(problems) = lust::check_with(&engine, "main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
    }

    #[test]
    fn every_native_is_in_a_known_file_with_its_own_name() {
        let mut keys: Vec<String> = NATIVES.iter().map(Native::key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), NATIVES.len(), "no key twice");
        for native in NATIVES {
            assert!(
                [
                    "world",
                    "entity",
                    "physics",
                    "input",
                    "sound",
                    "animation",
                    "tiles",
                    "screens",
                    "window"
                ]
                .contains(&native.file)
            );
            let named = match native.owner {
                "entity" => format!(" entity:{}(", native.name),
                _ => format!(" {}(", native.name),
            };
            assert!(native.lust.contains(&named), "{native:?}");
            assert!(native.lust.ends_with("native;"));
        }
    }

    #[test]
    fn an_old_name_still_checks_in_lust() {
        let renamed = [Renamed {
            file: "entity",
            old: "facingTowards",
            new: "facingAlong",
            until: "Phase 12",
        }];
        let mut files = descriptions_with(
            "Test Engine 9.9.9",
            "solis",
            &renamed,
            &crate::project_names::Names::standard(),
        )
        .0;
        let entity = &files
            .iter()
            .find(|(p, _)| p == "solis/entity.lst")
            .unwrap()
            .1;
        assert!(entity.contains("method rotation facingTowards(vec3 direction) native;"));
        assert!(entity.contains("stops working in Phase 12"));
        files.push((
            "main.lst".into(),
            "get \"solis/entity\":facingTowards;\n\
             component A {\n\
                 method update(float dt) {\n\
                     turnTo(facingTowards([0, 1, 0]));\n\
                 }\n\
             }\n\
             pass A;\nend\n"
                .into(),
        ));
        let code = crate::Code {
            files,
            lua: Vec::new(),
            notes: Vec::new(),
        };
        assert!(crate::check(&code).is_empty(), "{:?}", crate::check(&code));
    }
}
