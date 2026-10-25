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
        assert_eq!(files.len(), 2);
        assert!(files[0].1.starts_with("-- Made by Test Engine 9.9.9"));
        files.push((
            "main.lst".into(),
            "get \"solis/world\":[say, time, find];\n\
             get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n\
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
            assert!(["world", "entity"].contains(&native.file));
            assert!(
                native.lust.contains(&format!(" {}(", native.name)),
                "{native:?}"
            );
            assert!(native.lust.ends_with("native;"));
        }
    }
}
