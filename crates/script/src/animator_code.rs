//! Each animator's own Lust file ([decision 0078](../../../docs/decisions/0078-animators-reached-through-their-own-files.md)):
//! the game API guidelines' rule 4, anything made in an editor reaches code
//! through a file the editor writes. `farmer.solanim` gives
//! `solis/animators/farmer.lst`: a state of its graph's states
//! (`FarmerState`) and a class with a method per thing code can set or ask
//! (`FarmerAnimator`: `setMoving(on)`, `swing()`, `current()`, `jumpTo(…)`,
//! `setPlaybackSpeed(…)`). A component holds one as a field the engine fills
//! in (`FarmerAnimator anim;`), so `anim:setMoving(true)` is checked by Lust:
//! rename the switch in the graph and every line still using the old name is
//! a mistake.
//!
//! The methods' bodies call the engine's `Animators` object with the
//! graph's own names; game writers never see them.

use crate::project_names::option_name_of;

/// What a parameter of an animator is, for its method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A number: `setSpeed(float value)`.
    Number,
    /// On or off: `setMoving(bool on)`.
    Switch,
    /// Fired once: `swing()`.
    Trigger,
    /// A direction on the ground: `setFacing(vec3 direction)`.
    Direction,
}

/// One animator, as the editor read it: its path in the assets without
/// `.solanim` (`animators/farmer`), its parameters and its states, by their
/// names in the graph.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimatorNames {
    /// Its path in the assets, without the extension.
    pub path: String,
    /// Its parameters: name and kind.
    pub parameters: Vec<(String, ParamKind)>,
    /// Its states' names.
    pub states: Vec<String>,
}

/// The methods every animator's class has, which no parameter may take.
const OWN: &[&str] = &["current", "jumpTo", "setPlaybackSpeed"];

/// The class and state names for an animator at `path` (`FarmerAnimator`,
/// `FarmerState`), or none when its file name can't be a Lust name.
pub fn class_names(path: &str) -> Option<(String, String)> {
    let stem = path.rsplit('/').next()?;
    let base = option_name_of(stem)?;
    Some((format!("{base}Animator"), format!("{base}State")))
}

/// The Lust file for one animator (its path, `solis/animators/<path>.lst`,
/// and text; none when the animator can't be named at all), and notes on
/// anything game code can't reach.
pub fn animator_file(
    made_by: &str,
    folder: &str,
    animator: &AnimatorNames,
) -> (Option<(String, String)>, Vec<String>) {
    let path = &animator.path;
    let mut notes = Vec::new();
    let Some((class, state)) = class_names(path) else {
        notes.push(format!(
            "The animator {path} can't be reached from game code: its file name must start with a \
             letter and hold only letters, digits and _. Rename it to use it from code."
        ));
        return (None, notes);
    };
    // The states, as options, each with the graph's name for it.
    let mut options: Vec<(String, &str)> = Vec::new();
    for name in &animator.states {
        // "Use tool" is UseTool: each word's first letter a capital.
        let compact: String = name
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                c.next().map_or_else(String::new, |f| {
                    f.to_ascii_uppercase().to_string() + c.as_str()
                })
            })
            .collect();
        match option_name_of(&compact) {
            Some(option) if !options.iter().any(|(o, _)| o.eq_ignore_ascii_case(&option)) => {
                options.push((option, name));
            }
            _ => notes.push(format!(
                "The state \"{name}\" in {path} can't be named in game code (or has the same name \
                 as another but for capitals and spaces); rename it to reach it from code."
            )),
        }
    }
    let mut text = format!(
        "-- Made by {made_by}: {path}.solanim's names, for game code. It's written again\n\
         -- whenever the animator changes, so changes made here don't last.\n\
         -- Hold one in a component as a field the engine fills in: {class} anim;\n\n\
         fetch \"{folder}/animation\":Animators;\n\n\
         -- The animator's states.\n\
         state {state} = [{}];\n\n\
         class {class} {{\n    has {{\n        entity owner = nothing;\n    }}\n",
        if options.is_empty() {
            crate::project_names::NO_OPTION.to_string()
        } else {
            options
                .iter()
                .map(|(o, _)| o.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    let mut taken: Vec<String> = OWN.iter().map(|m| (*m).to_string()).collect();
    for (name, kind) in &animator.parameters {
        let Some(upper) = option_name_of(name) else {
            notes.push(format!(
                "The parameter \"{name}\" in {path} can't be named in game code; rename it to set \
                 it from code."
            ));
            continue;
        };
        let lower = upper[..1].to_ascii_lowercase() + &upper[1..];
        let method = match kind {
            ParamKind::Trigger => lower,
            _ => format!("set{upper}"),
        };
        if taken.iter().any(|t| t.eq_ignore_ascii_case(&method)) {
            notes.push(format!(
                "The parameter \"{name}\" in {path} would be the method {method}, which the \
                 animator's class has already; rename it to set it from code."
            ));
            continue;
        }
        taken.push(method.clone());
        let quoted = lust_text(name);
        let line = match kind {
            ParamKind::Number => format!(
                "    -- The number {name}.\n    method {method}(float value) {{\n        Animators:setNumber(owner, {quoted}, value);\n    }}\n"
            ),
            ParamKind::Switch => format!(
                "    -- The switch {name}.\n    method {method}(bool on) {{\n        Animators:setSwitch(owner, {quoted}, on);\n    }}\n"
            ),
            ParamKind::Trigger => format!(
                "    -- The trigger {name}.\n    method {method}() {{\n        Animators:fire(owner, {quoted});\n    }}\n"
            ),
            ParamKind::Direction => format!(
                "    -- The direction {name}.\n    method {method}(vec3 direction) {{\n        Animators:setDirection(owner, {quoted}, direction);\n    }}\n"
            ),
        };
        text.push_str(&line);
    }
    // Which state the graph is in, and jumping to one.
    text.push_str(&format!(
        "    -- Which state the animator is in.\n    method {state} current() {{\n        string now = Animators:stateOf(owner);\n"
    ));
    for (option, name) in options.iter().skip(1) {
        text.push_str(&format!(
            "        is now == {}? then {{\n            return {state}.{option};\n        }}\n",
            lust_text(name)
        ));
    }
    text.push_str(&format!(
        "        return {state}.{};\n    }}\n",
        options
            .first()
            .map_or(crate::project_names::NO_OPTION, |(o, _)| o.as_str())
    ));
    text.push_str(&format!(
        "    -- Puts the animator straight into a state, its clip started at once.\n    method jumpTo({state} which) {{\n"
    ));
    if options.is_empty() {
        text.push_str("        Animators:jumpTo(owner, \"\");\n");
    } else {
        text.push_str("        check which {\n");
        for (option, name) in &options {
            text.push_str(&format!(
                "            when {option} {{\n                Animators:jumpTo(owner, {});\n            }}\n",
                lust_text(name)
            ));
        }
        text.push_str("        }\n");
    }
    text.push_str(
        "    }\n    -- How fast its clips play: 1 as made, 2 twice as fast.\n    method setPlaybackSpeed(float scale) {\n        Animators:setPlaybackSpeed(owner, scale);\n    }\n}\n\n",
    );
    text.push_str(&format!("pass {state}, {class};\nend\n"));
    (
        Some((format!("{folder}/animators/{path}.lst"), text)),
        notes,
    )
}

/// `text` as a Lust text literal: quotes and backslashes escaped, and a
/// brace before a name kept as a brace.
fn lust_text(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn farmer() -> AnimatorNames {
        AnimatorNames {
            path: "animators/farmer".into(),
            parameters: vec![
                ("moving".into(), ParamKind::Switch),
                ("speed".into(), ParamKind::Number),
                ("facing".into(), ParamKind::Direction),
                ("swing".into(), ParamKind::Trigger),
                ("playback speed".into(), ParamKind::Number),
                ("current".into(), ParamKind::Trigger),
            ],
            states: vec![
                "Idle".into(),
                "Walk".into(),
                "Use tool".into(),
                "use Tool".into(),
            ],
        }
    }

    #[test]
    fn an_animator_becomes_a_state_and_a_class_with_a_method_each() {
        let (file, notes) = animator_file("Test 1.0", "solis", &farmer());
        let (path, text) = file.unwrap();
        assert_eq!(path, "solis/animators/animators/farmer.lst");
        assert!(
            text.contains("state FarmerState = [Idle, Walk, UseTool];"),
            "{text}"
        );
        for method in [
            "method setMoving(bool on)",
            "method setSpeed(float value)",
            "method setFacing(vec3 direction)",
            "method swing()",
            "method FarmerState current()",
            "method jumpTo(FarmerState which)",
            "method setPlaybackSpeed(float scale)",
        ] {
            assert!(text.contains(method), "{method}: {text}");
        }
        assert!(
            text.contains("Animators:jumpTo(owner, \"Use tool\");"),
            "{text}"
        );
        // A name with a space, one clashing with the class's own methods,
        // and a state the same as another but for capitals: notes.
        assert_eq!(notes.len(), 3, "{notes:?}");
    }

    #[test]
    fn the_file_checks_in_lust_and_a_component_can_hold_one() {
        let (path, text) = animator_file("Test 1.0", "solis", &farmer()).0.unwrap();
        let mut files = crate::describe::descriptions("Test 1.0", "solis");
        files.push((path, text));
        files.push((
            "main.lst".into(),
            "fetch \"solis/animators/animators/farmer\":[FarmerAnimator, FarmerState];\n\
             component Farmer {\n    has {\n        FarmerAnimator anim;\n    }\n\
             \x20   method update(float dt) {\n        anim:setMoving(true);\n        anim:swing();\n\
             \x20       is anim:current() == FarmerState.UseTool? then {\n            anim:jumpTo(FarmerState.Idle);\n        }\n\
             \x20       anim:setPlaybackSpeed(1.5);\n    }\n}\npass Farmer;\nend\n"
                .into(),
        ));
        let engine = lust::Engine {
            folders: crate::describe::engine_folders(&files),
        };
        if let Err(problems) = lust::check_with(&engine, "main.lst", &files) {
            let text: Vec<String> = problems.iter().map(ToString::to_string).collect();
            panic!("{}", text.join("\n"));
        }
    }

    #[test]
    fn an_animator_with_no_states_or_a_bad_name_is_still_safe() {
        let empty = AnimatorNames {
            path: "cow".into(),
            parameters: Vec::new(),
            states: Vec::new(),
        };
        let (_, text) = animator_file("Test 1.0", "solis", &empty).0.unwrap();
        assert!(text.contains("state CowState = [None];"));
        let bad = AnimatorNames {
            path: "2cow".into(),
            ..empty
        };
        let (file, notes) = animator_file("Test 1.0", "solis", &bad);
        assert!(file.is_none());
        assert!(notes[0].contains("2cow"), "{notes:?}");
        assert_eq!(lust_text("a \"b\" {c} \\"), "\"a \\\"b\\\" \\{c} \\\\\"");
    }
}
