//! The project's names game code had before decision 0118 renamed them,
//! as mistakes that say the new one: a straight switch, as the project
//! lead chose for the rename of areas to levels ("Right away"). Lust
//! would only say the old name isn't there; this says what it is now,
//! before Lust is asked. Words in comments are left alone.
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
    "World:spawn(Module.Ships.Fighter, place), with fetch \"solis/project\":Module;",
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
        assert!(found[0].message.contains("Module.Ships.Fighter"));
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
}
