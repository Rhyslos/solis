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
    /// Its name, as Lust calls it (and as the body's key in
    /// `LUST_NATIVES`).
    pub name: &'static str,
    /// Its Lust: what it takes and gives back.
    pub lust: &'static str,
    /// What it does, in plain words, written above it.
    pub about: &'static str,
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
        name: "AnimationCue",
        lust: "event AnimationCue(string name);",
        about: "Sent to an entity when its Sprite's clip reaches a frame with an event (\"footstep\"), before the tick's other events. A sound event of that name also plays by itself.",
    },
    EngineEvent {
        file: "animation",
        name: "ClipFinished",
        lust: "event ClipFinished(string clip);",
        about: "Sent to an entity when its Sprite's clip has played all its times (a clip that plays over and over never finishes).",
    },
];

/// Natives renamed on purpose, with their old names kept working for a
/// while. None yet: the first rename adds a line here.
pub const RENAMED: &[Renamed] = &[];

/// Every native method Solis gives game code.
pub const NATIVES: &[Native] = &[
    Native {
        file: "world",
        name: "say",
        lust: "method say(string words) native;",
        about: "Writes words in the Console.",
    },
    Native {
        file: "world",
        name: "time",
        lust: "method float time() native;",
        about: "How long the game has been running, in seconds of game time.",
    },
    Native {
        file: "world",
        name: "find",
        lust: "method entity find(string name) native;",
        about: "The first entity with this name, in the Area panel's order, or nothing.",
    },
    Native {
        file: "world",
        name: "fire",
        lust: "method fire(vec3 from, vec3 direction, float speed, float life, vec3 colour) native;",
        about: "Fires a projectile (a laser): from a place, flying a way at a speed in metres a second, for a life in seconds, glowing in a colour (red, green, blue; above 1 glows brighter).",
    },
    Native {
        file: "world",
        name: "spawn",
        lust: "method entity spawn(string file, vec3 place) native;",
        about: "Makes a copy of an assembly (its file in the assets, such as \"ships/fighter.solasm\", or just its name) at a place; its entity, which is in the game from the next tick, or nothing if there's no such assembly.",
    },
    Native {
        file: "world",
        name: "despawn",
        lust: "method despawn(entity which) native;",
        about: "Takes an entity, and everything under it, out of the game at the end of the tick.",
    },
    Native {
        file: "world",
        name: "frameTime",
        lust: "method float frameTime() native;",
        about: "How long the last frame took, in seconds of real time (1/60 at 60 frames a second).",
    },
    Native {
        file: "world",
        name: "random",
        lust: "method float random() native;",
        about: "A random number from 0 up to 1. The same game with the same seed gets the same numbers, on any computer.",
    },
    Native {
        file: "world",
        name: "randomRange",
        lust: "method float randomRange(float low, float high) native;",
        about: "A random number from low up to high.",
    },
    Native {
        file: "world",
        name: "randomInt",
        lust: "method int randomInt(int low, int high) native;",
        about: "A random whole number from low to high, both included (randomInt(1, 6) rolls a die).",
    },
    Native {
        file: "world",
        name: "setSeed",
        lust: "method setSeed(int seed) native;",
        about: "Starts the game's random numbers again from a seed, at the end of the tick: the same seed gives the same numbers again.",
    },
    Native {
        file: "world",
        name: "saveGame",
        lust: "method saveGame(string slot) native;",
        about: "Saves the whole game in a slot (a name such as \"quick\" or \"slot_1\": letters, digits, _ and -), at the end of the tick: every entity, every component's values, the lasers, physics, the clock and the random numbers.",
    },
    Native {
        file: "world",
        name: "loadGame",
        lust: "method loadGame(string slot) native;",
        about: "Loads the game saved in a slot, at the end of the tick: everything is put back exactly as it was when it was saved.",
    },
    Native {
        file: "sound",
        name: "playSound",
        lust: "method playSound(string event) native;",
        about: "Plays a sound event (its file in assets, without .solsound: \"sfx/footstep\"), the same everywhere: music, the interface. At the end of the tick, as a move lands. Nothing comes back, so sound can never change the game.",
    },
    Native {
        file: "sound",
        name: "playSoundAt",
        lust: "method playSoundAt(string event, vec3 place) native;",
        about: "Plays a sound event at a place in the world: quieter far from the camera, and from its direction.",
    },
    Native {
        file: "sound",
        name: "playSoundOn",
        lust: "method playSoundOn(string event, entity on) native;",
        about: "Plays a sound event on an entity, following it as it moves: an engine, a voice.",
    },
    Native {
        file: "sound",
        name: "stopSoundsOn",
        lust: "method stopSoundsOn(entity on) native;",
        about: "Stops the sounds playing on an entity, with a short fade.",
    },
    Native {
        file: "sound",
        name: "setSoundPlaying",
        lust: "method setSoundPlaying(entity on, bool playing) native;",
        about: "Turns an entity's AudioSource (a sound that keeps playing at it: an engine, a campfire) on or off, at the end of the tick.",
    },
    Native {
        file: "sound",
        name: "setBusVolume",
        lust: "method setBusVolume(string bus, float volume) native;",
        about: "Sets a bus's volume (\"Music\", \"Effects\": the Mixer's buses) for the rest of the play, from 0 (silent) to 1 (as set in the Mixer): for an options menu's sliders.",
    },
    Native {
        file: "animation",
        name: "playClip",
        lust: "method playClip(entity on, string clip) native;",
        about: "Plays a clip on an entity's Sprite (\"walk\": one of its sheet's Aseprite tags, or a clip of its clips file), at the end of the tick. Already playing it: it carries on. Another clip, or one that has finished: it starts from its beginning.",
    },
    Native {
        file: "animation",
        name: "stopClip",
        lust: "method stopClip(entity on) native;",
        about: "Stops an entity's Sprite where it is in its clip, at the end of the tick; playClip carries on from there.",
    },
    Native {
        file: "input",
        name: "held",
        lust: "method bool held(string action) native;",
        about: "Whether an action's input is held down this tick (\"Jump\"; an axis such as \"Move\": not at rest). Actions are set up in Edit > Input actions.",
    },
    Native {
        file: "input",
        name: "pressed",
        lust: "method bool pressed(string action) native;",
        about: "Whether an action's input went down since the last tick: true once per press.",
    },
    Native {
        file: "input",
        name: "released",
        lust: "method bool released(string action) native;",
        about: "Whether an action's input came up since the last tick: true once per release.",
    },
    Native {
        file: "input",
        name: "axis",
        lust: "method vec3 axis(string action) native;",
        about: "An axis action's two numbers as x (across: right is +1) and y (up or forward: +1); z is 0. Move gives which way to go, each from -1 to 1; Look gives how far to turn this tick, in degrees.",
    },
    Native {
        file: "input",
        name: "bindingOf",
        lust: "method string bindingOf(string action) native;",
        about: "The input that sets an action off (a key first, then the mouse, then the gamepad), such as \"F\": for telling the player \"Press F\". Empty if it has none.",
    },
    Native {
        file: "input",
        name: "rebind",
        lust: "method rebind(string action, string input) native;",
        about: "Binds an input (\"E\", \"Mouse Right\", \"Pad Y\") to an action from the next tick, in place of its inputs of the same kind: what a controls menu does. Lasts while the game plays.",
    },
    Native {
        file: "physics",
        name: "push",
        lust: "method push(entity which, vec3 impulse) native;",
        about: "Gives an entity's body a push, at the end of the tick: a heavy body moves less than a light one (the push is in kilograms times metres a second).",
    },
    Native {
        file: "physics",
        name: "velocityOf",
        lust: "method vec3 velocityOf(entity which) native;",
        about: "How fast and which way an entity's body moves, in metres a second (nothing moving without a RigidBody).",
    },
    Native {
        file: "physics",
        name: "setVelocity",
        lust: "method setVelocity(entity which, vec3 velocity) native;",
        about: "Sets how fast and which way an entity's body moves, in metres a second, at the end of the tick.",
    },
    Native {
        file: "physics",
        name: "castRay",
        lust: "method float castRay(vec3 from, vec3 direction, float reach) native;",
        about: "Looks along a line from a place, a direction, up to reach metres: how far away the first collider is, or -1 if there's none. Colliders the line starts inside are passed through.",
    },
    Native {
        file: "physics",
        name: "castRayHit",
        lust: "method entity castRayHit(vec3 from, vec3 direction, float reach) native;",
        about: "Looks along a line like castRay: the entity whose collider it meets first, or nothing.",
    },
    Native {
        file: "physics",
        name: "castRayNormal",
        lust: "method vec3 castRayNormal(vec3 from, vec3 direction, float reach) native;",
        about: "Looks along a line like castRay: which way the surface it meets faces (straight up for a floor), or [0, 0, 0] if it meets nothing.",
    },
    Native {
        file: "physics",
        name: "moveCharacter",
        lust: "method moveCharacter(entity which, vec3 wanted) native;",
        about: "Moves a character (an entity with a CharacterBody) this far, in metres, at the end of the tick: it climbs steps and slopes, slides along walls, pushes crates and rides moving platforms, and stops where it can't go. Ask once a tick, with speed times dt.",
    },
    Native {
        file: "physics",
        name: "onGround",
        lust: "method bool onGround(entity which) native;",
        about: "Whether a character stood on something after its last move: for jumping only from the ground, and for falling.",
    },
    Native {
        file: "entity",
        name: "positionOf",
        lust: "method vec3 positionOf(entity which) native;",
        about: "Where an entity is in the world, in metres.",
    },
    Native {
        file: "entity",
        name: "moveTo",
        lust: "method moveTo(entity which, vec3 place) native;",
        about: "Puts an entity at a place in the world, in metres.",
    },
    Native {
        file: "entity",
        name: "rotationOf",
        lust: "method rotation rotationOf(entity which) native;",
        about: "Which way an entity faces in the world.",
    },
    Native {
        file: "entity",
        name: "turnTo",
        lust: "method turnTo(entity which, rotation facing) native;",
        about: "Turns an entity to face this way in the world.",
    },
    Native {
        file: "entity",
        name: "facingAlong",
        lust: "method rotation facingAlong(vec3 direction) native;",
        about: "The upright rotation that faces along a direction (an entity's forward is +Y): turnTo(me, facingAlong(where - positionOf(me))) turns to face a place.",
    },
    Native {
        file: "entity",
        name: "nameOf",
        lust: "method string nameOf(entity which) native;",
        about: "An entity's name, as the Area panel shows it.",
    },
];

/// The description files, and the ready-made components in `kit/`
/// ([`KIT`]), as (path inside `assets`, text), with `/`
/// between folders, in `folder` (the program's name in small letters, from
/// the app's branding: `solis`). `made_by` names the program and version
/// that wrote them, for their first line.
pub fn descriptions(made_by: &str, folder: &str) -> Vec<(String, String)> {
    descriptions_with(made_by, folder, RENAMED)
}

/// [`descriptions`], with these natives renamed.
fn descriptions_with(made_by: &str, folder: &str, renamed: &[Renamed]) -> Vec<(String, String)> {
    let mut files: Vec<&str> = NATIVES.iter().map(|n| n.file).collect();
    files.dedup();
    files
        .into_iter()
        .map(|file| {
            let mine: Vec<&Native> = NATIVES.iter().filter(|n| n.file == file).collect();
            let mut text = format!(
                "-- Made by {made_by}: what game code can ask of the engine. It's written again\n\
                 -- whenever the project opens, so changes made here don't last.\n\n"
            );
            for native in &mine {
                text.push_str(&format!("-- {}\n{}\n\n", native.about, native.lust));
            }
            let mut names: Vec<&str> = mine.iter().map(|n| n.name).collect();
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
            text.push_str(&format!("pass {};\nend\n", names.join(", ")));
            (format!("{folder}/{file}.lst"), text)
        })
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
        .collect()
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
    ("cameras.lst", include_str!("kit/cameras.lst")),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_descriptions_check_cleanly_and_a_game_can_use_them() {
        let mut files = descriptions("Test Engine 9.9.9", "solis");
        assert_eq!(files.len(), 8);
        assert!(files[0].1.starts_with("-- Made by Test Engine 9.9.9"));
        files.push((
            "main.lst".into(),
            "get \"solis/world\":[say, time, find];\n\
             get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n\
             get \"solis/physics\":[push, velocityOf, setVelocity, castRay, castRayHit, castRayNormal];\n\
             get \"solis/input\":[held, pressed, released, axis, bindingOf, rebind];\n\
             get \"solis/sound\":[playSound, playSoundAt, playSoundOn, stopSoundsOn, setSoundPlaying, setBusVolume];\n\
             get \"solis/animation\":[playClip, stopClip];\n\
             fetch \"solis/animation\":AnimationCue;\n\
             fetch \"solis/animation\":ClipFinished;\n\
             component Walker {\n\
                 has {\n\
                     int steps = 0;\n\
                 }\n\
                 on AnimationCue(string name) {\n\
                     steps = steps + 1;\n\
                 }\n\
                 on ClipFinished(string clip) {\n\
                     is clip == \"hoe\"? then {\n\
                         playClip(me, \"idle\");\n\
                     }\n\
                     stopClip(me);\n\
                 }\n\
             }\n\
             component Mover {\n\
                 has {\n\
                     float speed = 2;\n\
                 }\n\
                 method update(float dt) {\n\
                     vec3 step = [1, 0, 0];\n\
                     moveTo(me, positionOf(me) + step * speed * dt);\n\
                     turnTo(me, rotationOf(me));\n\
                     is time() > 10? then {\n\
                         say(nameOf(me) + \" has gone far\");\n\
                     }\n\
                     entity other = find(\"Crate\");\n\
                     is held(\"Jump\") && pressed(\"Jump\") || released(\"Jump\")? then {\n\
                         rebind(\"Jump\", \"Space\");\n\
                         say(bindingOf(\"Jump\") + axis(\"Move\").x.text());\n\
                     }\n\
                     push(other, velocityOf(me) + castRayNormal(positionOf(me), step, 10));\n\
                     is castRay(positionOf(me), step, 10) > 0 && castRayHit(positionOf(me), step, 10) == other? then {\n\
                         setVelocity(other, step);\n\
                     }\n\
                 }\n\
             }\n\
             pass Mover, Walker;\n\
             end\n"
                .into(),
        ));
        if let Err(problems) = lust::check("main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
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
        if let Err(problems) = lust::check("main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
    }

    #[test]
    fn every_native_is_in_a_known_file_with_its_own_name() {
        let mut names: Vec<&str> = NATIVES.iter().map(|n| n.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), NATIVES.len(), "no name twice");
        for native in NATIVES {
            assert!(
                ["world", "entity", "physics", "input", "sound", "animation"]
                    .contains(&native.file)
            );
            assert!(
                native.lust.contains(&format!(" {}(", native.name)),
                "{native:?}"
            );
            assert!(native.lust.ends_with("native;"));
        }
    }

    #[test]
    fn an_old_name_still_checks_in_lust() {
        let renamed = [Renamed {
            file: "world",
            old: "spawnAt",
            new: "spawn",
            until: "Phase 12",
        }];
        let mut files = descriptions_with("Test Engine 9.9.9", "solis", &renamed);
        assert!(
            files[0]
                .1
                .contains("method entity spawnAt(string file, vec3 place) native;")
        );
        assert!(files[0].1.contains("stops working in Phase 12"));
        files.push((
            "main.lst".into(),
            "get \"solis/world\":[spawnAt];\n\
             component A {\n\
                 method update(float dt) {\n\
                     entity e = spawnAt(\"ships/fighter\", [0, 0, 0]);\n\
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

