//! The project's names game code had before decision 0118 renamed them,
//! as mistakes that say the new one: a straight switch, as the project
//! lead chose for the rename of areas to levels ("Right away"). Lust
//! would only say the old name isn't there; this says what it is now,
//! before Lust is asked. Words in comments are left alone. It also holds
//! the setters a component now sets with `=` on its own entity
//! ([`OWN_SETTERS`], the uniformity pass, U5).
//!
//! Only Lust names the project's states: Lua and Rust name a module by
//! its path as text (`"Ships.Fighter"`, `"ships/fighter"`), which didn't
//! change.

use crate::code::CodeProblem;

/// Each state of `solis/project.lst` renamed, with its new name and how
/// it's written now.
pub const RENAMED_STATES: &[(&str, &str, &str)] = &[(
    "Assembly",
    "Module",
    "new Fighter { position = place }; for Module.Fighter, with fetch \"solis/made\":Fighter;",
)];

/// Each use of a renamed state's old name in the Lust `files`.
pub(crate) fn old_names(files: &[(String, String)]) -> Vec<CodeProblem> {
    let mut found = Vec::new();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    for (file, text) in files {
        if crate::describe::in_engine_folder(file) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let code_part = line.split("--").next().unwrap_or_default();
            for (old, new, written) in RENAMED_STATES {
                let mut from = 0;
                while let Some(at) = code_part[from..].find(old) {
                    let start = from + at;
                    let end = start + old.len();
                    from = end;
                    let before = code_part[..start].chars().next_back();
                    let after = code_part[end..].chars().next();
                    if before.is_some_and(word) || after.is_some_and(word) {
                        continue;
                    }
                    found.push(CodeProblem {
                        file: file.clone(),
                        line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                        column: u32::try_from(code_part[..start].chars().count() + 1)
                            .unwrap_or(u32::MAX),
                        message: format!(
                            "{old} is now {new} (assemblies are called modules): write {written}"
                        ),
                    });
                }
            }
        }
    }
    found
}

/// The setters a component no longer calls on its own entity (the
/// uniformity pass, U5; decision 0137): each is a field the engine reads,
/// set with `=`. On another entity they stay methods (`orc:setVisible(false)`).
/// Each with how it's written now.
pub const OWN_SETTERS: &[(&str, &str)] = &[
    (
        "animate",
        "animation = Clip.FarmerWalk; with Clip animation in the component's has",
    ),
    (
        "animateFromStart",
        "animation = Clip.FarmerWalk; with Clip animation in the component's has, and restartAnimation(); to play it again from its start",
    ),
    (
        "stopAnimation",
        "animation = Clip.None; with Clip animation in the component's has",
    ),
    (
        "setVisible",
        "visible = false; with bool visible = true; in the component's has",
    ),
    (
        "setFlip",
        "flip = Flip.Across; with Flip flip = Flip.None; in the component's has (fetch \"solis/animation\":Flip;)",
    ),
    (
        "setSolid",
        "solid = false; with bool solid = true; in the component's has",
    ),
    (
        "setSoundPlaying",
        "soundPlaying = true; with bool soundPlaying = false; in the component's has",
    ),
    (
        "showScreen",
        "screen = Screen.PauseMenu; with Screen screen = Screen.None; in the component's has",
    ),
    (
        "hideScreen",
        "screen = Screen.None; with Screen screen = Screen.None; in the component's has",
    ),
];

/// Each call of an [`OWN_SETTERS`] setter on a component's own entity
/// (written bare, `setVisible(false)`) in the Lust `files`, as a mistake
/// saying the field to set. Calls on another entity (`orc:setVisible`),
/// words in comments and text, and a name the game gives a method of its
/// own are left alone.
pub(crate) fn own_setters(files: &[(String, String)]) -> Vec<CodeProblem> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let own: Vec<&(&str, &str)> = OWN_SETTERS
        .iter()
        .filter(|(name, _)| {
            !files.iter().any(|(file, text)| {
                !crate::describe::in_engine_folder(file)
                    && text.contains(&format!("method {name}("))
            })
        })
        .collect();
    let mut found = Vec::new();
    for (file, text) in files {
        if crate::describe::in_engine_folder(file) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            let code_part = code_of(line);
            for (name, written) in &own {
                let call = format!("{name}(");
                let mut from = 0;
                while let Some(at) = code_part[from..].find(&call) {
                    let start = from + at;
                    from = start + call.len();
                    let before = code_part[..start].trim_end().chars().next_back();
                    if code_part[..start].chars().next_back().is_some_and(word)
                        || before.is_some_and(|c| c == ':' || c == '.')
                    {
                        continue;
                    }
                    found.push(CodeProblem {
                        file: file.clone(),
                        line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                        column: u32::try_from(code_part[..start].chars().count() + 1)
                            .unwrap_or(u32::MAX),
                        message: format!(
                            "{name} is set with = on a component's own entity now: write {written} \
                             (on another entity it stays target:{name}(…))."
                        ),
                    });
                }
            }
        }
    }
    found
}

/// A line of Lust without its comment, and with what's inside quotes
/// blanked out (so a word in text isn't read as a call).
fn code_of(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut quoted = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                quoted = false;
            }
            out.push(if c == '"' { '"' } else { ' ' });
            continue;
        }
        if c == '-' && chars.peek() == Some(&'-') {
            break;
        }
        if c == '"' {
            quoted = true;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lust(text: &str) -> Vec<(String, String)> {
        vec![("main.lst".to_string(), text.to_string())]
    }

    #[test]
    fn the_old_name_is_a_mistake_naming_the_new_one() {
        let found = old_names(&lust(
            "fetch \"solis/project\":Assembly;\n\nentity e = World:spawn(Assembly.Ships.Fighter, p);\n",
        ));
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].line, found[0].column), (1, 23));
        assert_eq!((found[1].line, found[1].column), (3, 24));
        assert!(
            found[0].message.starts_with("Assembly is now Module"),
            "{}",
            found[0]
        );
        assert!(
            found[0]
                .message
                .contains("new Fighter { position = place };")
        );
    }

    #[test]
    fn comments_other_words_and_the_engine_s_files_are_left_alone() {
        let text = "-- Assembly.Ships.Fighter\nAssemblyLine a;\nmyAssembly b;\nassembly Hud {}\n";
        assert!(old_names(&lust(text)).is_empty());
        let engine = vec![("solis/project.lst".to_string(), "Assembly".to_string())];
        assert!(old_names(&engine).is_empty());
    }

    #[test]
    fn a_game_using_the_old_name_is_refused_with_the_new_one() {
        let code = crate::code::Code {
            files: lust("fetch \"solis/project\":Assembly;\nend\n"),
            ..Default::default()
        };
        let problems = crate::code::check(&code);
        assert!(
            problems
                .iter()
                .any(|p| p.message.starts_with("Assembly is now Module")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_setter_called_on_its_own_entity_is_a_mistake_saying_the_field() {
        let text = "component Lamp {\n    method update(float dt) {\n        setVisible(false);\n        \
                    orc:setVisible(false);\n        this:setFlip(true, false);\n        \
                    World:say(\"showScreen(x)\"); -- hideScreen(Screen.Hud)\n        \
                    mySetVisible(true);\n        showScreen(Screen.Hud);\n    }\n}\nend\n";
        let found = own_setters(&lust(text));
        let at: Vec<(u32, u32)> = found.iter().map(|p| (p.line, p.column)).collect();
        assert_eq!(at, [(3, 9), (8, 9)], "{found:#?}");
        assert!(
            found[0].message.starts_with("setVisible is set with ="),
            "{}",
            found[0]
        );
        assert!(found[0].message.contains("visible = false;"));
        assert!(found[1].message.contains("screen = Screen.PauseMenu;"));
    }

    #[test]
    fn a_game_s_own_method_of_that_name_and_the_engine_s_files_are_left_alone() {
        let text = "class Bulb {\n    method setVisible(bool on) {}\n    method show() { setVisible(true); }\n}\nend\n";
        assert!(own_setters(&lust(text)).is_empty());
        let engine = vec![(
            "solis/animation.lst".to_string(),
            "animate(clip);".to_string(),
        )];
        assert!(own_setters(&engine).is_empty());
    }
}
