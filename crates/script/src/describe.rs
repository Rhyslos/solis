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
        name: "nameOf",
        lust: "method string nameOf(entity which) native;",
        about: "An entity's name, as the Area panel shows it.",
    },
];

/// The description files, as (path inside `assets`, text), with `/`
/// between folders, in `folder` (the program's name in small letters, from
/// the app's branding: `solis`). `made_by` names the program and version
/// that wrote them, for their first line.
pub fn descriptions(made_by: &str, folder: &str) -> Vec<(String, String)> {
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
            let names: Vec<&str> = mine.iter().map(|n| n.name).collect();
            text.push_str(&format!("pass {};\nend\n", names.join(", ")));
            (format!("{folder}/{file}.lst"), text)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_descriptions_check_cleanly_and_a_game_can_use_them() {
        let mut files = descriptions("Test Engine 9.9.9", "solis");
        assert_eq!(files.len(), 4);
        assert!(files[0].1.starts_with("-- Made by Test Engine 9.9.9"));
        files.push((
            "main.lst".into(),
            "get \"solis/world\":[say, time, find];\n\
             get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n\
             get \"solis/physics\":[push, velocityOf, setVelocity, castRay, castRayHit, castRayNormal];\n\
             get \"solis/input\":[held, pressed, released, axis, bindingOf, rebind];\n\
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
             pass Mover;\n\
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
            assert!(["world", "entity", "physics", "input"].contains(&native.file));
            assert!(
                native.lust.contains(&format!(" {}(", native.name)),
                "{native:?}"
            );
            assert!(native.lust.ends_with("native;"));
        }
    }
}
