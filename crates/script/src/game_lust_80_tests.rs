//! What Lust alpha 75 to 80 changed for a game's code: a comment starts
//! only with `--` (`//` is a mistake), and only a thing's own methods
//! change its state. Each is Lust's own mistake in Problems, on its line,
//! with nothing of Solis's in the way.

use super::states_tests::project;
use crate::code::{CodeProblem, check, gather};

/// Each problem's file, line and words.
fn found(main: &str, name: &str) -> Vec<(String, u32, String)> {
    let folder = project(name, main);
    check(&gather(&folder))
        .iter()
        .map(|p: &CodeProblem| (p.file.clone(), p.line, p.message.clone()))
        .collect()
}

#[test]
fn a_comment_the_old_way_is_a_mistake_saying_to_write_dashes() {
    let main = "// A lamp.\ncomponent Lamp {\n    has {\n        int on = 0; // lit\n    }\n}\npass Lamp;\nend\n";
    let words = "comments start with --, so write -- here instead of //.";
    assert_eq!(
        found(main, "slash-comment"),
        [1, 4].map(|line| ("main.lst".to_string(), line, words.to_string())),
    );
    let fixed = main.replace("//", "--");
    assert_eq!(found(&fixed, "dash-comment"), Vec::new());
}

#[test]
fn a_things_state_is_changed_only_by_its_own_methods() {
    let lamp = |change: &str| {
        format!(
            "state Mood = [Calm, Busy];\n\
             class Bulb {{\n    has {{\n        state = Mood.Calm;\n    }}\n    \
             method rush() {{\n        state = Mood.Busy;\n    }}\n}}\n\
             component Lamp {{\n    method update(float dt) {{\n        \
             Bulb bulb = new Bulb {{}};\n        {change}\n    }}\n}}\n\
             pass Lamp;\nend\n"
        )
    };
    let problems = found(&lamp("bulb.state = Mood.Busy;"), "state-outside");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!((problems[0].0.as_str(), problems[0].1), ("main.lst", 13));
    assert!(
        problems[0].2.starts_with("only Bulb's own methods"),
        "{problems:?}"
    );
    assert_eq!(found(&lamp("bulb:rush();"), "state-method"), Vec::new());
}
