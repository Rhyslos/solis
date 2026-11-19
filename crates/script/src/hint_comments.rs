//! Lust fields' hints, read from the comment after each (decision 0127):
//! `float speed = 4.0; -- [range 0 10, unit "m/s"]`. Lust has no
//! attributes on fields yet; once it has, moving to them is taking away
//! the `--`. Also each field's written type, so a field of one of the
//! project's names (`Sound alertSound = Sound.Sfx.Alert;`) gets a picker.
//!
//! Hints change nothing about how the game runs: they're put on the
//! schemas the editor shows, and a mistake in one is listed in Problems
//! with its line, leaving out only that hint.

use crate::code::CodeProblem;
use crate::field_hints::{self, MAX_HINT_TEXT};
use reflect::{ComponentSchema, FieldKind, NameKind, Offer};

/// The most fields read from one file: far more than any component has.
const MOST_FIELDS: usize = 10_000;

/// Puts the hints written in `files` (Lust, path and text) on the Lust
/// components' `schemas`, and the project's names' pickers on fields of
/// their kinds; gives back the mistakes found in the hints.
pub(crate) fn put_on(
    files: &[(String, String)],
    schemas: &mut [ComponentSchema],
) -> Vec<CodeProblem> {
    let mut problems = Vec::new();
    for (path, text) in files {
        let lines: Vec<&str> = text.lines().collect();
        for written in level::lust_fields::fields(text, MOST_FIELDS) {
            let Some(field) = schemas
                .iter_mut()
                .find(|s| s.name == written.component)
                .and_then(|s| s.fields.iter_mut().find(|f| f.name == written.name))
            else {
                continue;
            };
            if let (FieldKind::Choice(_), Some(kind)) =
                (&field.kind, NameKind::of_state(written.kind))
            {
                field.hint.offers = Offer::Names(kind);
            }
            let line = lines
                .get((written.line as usize).saturating_sub(1))
                .copied()
                .unwrap_or_default();
            let Some(hint) = hint_in(line) else {
                continue;
            };
            let problem = |message: String| CodeProblem {
                file: path.clone(),
                line: written.line,
                column: 0,
                message: format!("the hint on {}: {message}", written.name),
            };
            let Some(inside) = hint.strip_suffix(']') else {
                problems.push(problem("its [ has no ] closing it on the same line".into()));
                continue;
            };
            if inside.chars().count() > 4 * MAX_HINT_TEXT {
                problems.push(problem(format!(
                    "it's longer than {} characters",
                    4 * MAX_HINT_TEXT
                )));
                continue;
            }
            let (words, mut mistakes) = field_hints::read(inside);
            mistakes.extend(field_hints::apply(field, words, written.kind));
            problems.extend(mistakes.into_iter().map(problem));
            if let Some(outside) = field_hints::outside_range(field) {
                problems.push(CodeProblem {
                    message: outside,
                    ..problem(String::new())
                });
            }
        }
    }
    problems
}

/// The hint after a field's `;` on its line: the comment's text from its
/// `[`, when the comment starts with one.
pub(crate) fn hint_in(line: &str) -> Option<&str> {
    let comment = comment_in(line)?.trim();
    comment.starts_with('[').then(|| &comment[1..])
}

/// The text of the comment ending `line` (after `--` or `//`), past any
/// text in quotes; none for a block comment (`--[[`).
fn comment_in(line: &str) -> Option<&str> {
    let mut in_text = false;
    let mut escaped = false;
    for (at, c) in line.char_indices() {
        if in_text {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_text = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_text = true,
            '-' | '/' if line[at + 1..].starts_with(c) => {
                let rest = &line[at + 2..];
                return (!rest.starts_with("[[")).then_some(rest);
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect::{FieldInfo, Value};

    fn guard() -> ComponentSchema {
        ComponentSchema {
            name: "Guard".into(),
            help: String::new(),
            fields: vec![
                FieldInfo::new("speed", "Speed", FieldKind::Float, Value::Float(4.0)),
                FieldInfo::new(
                    "alertSound",
                    "Alert sound",
                    FieldKind::Choice(vec!["Sfx".into(), "Sfx.Alert".into()]),
                    Value::Text("Sfx.Alert".into()),
                ),
                FieldInfo::new("says", "Says", FieldKind::Text, Value::Text(String::new())),
            ],
        }
    }

    #[test]
    fn the_comment_after_a_field_is_its_hint() {
        assert_eq!(
            hint_in("float speed = 4.0; -- [range 0 10]"),
            Some("range 0 10]")
        );
        assert_eq!(
            hint_in("float speed = 4.0; // [unit \"m\"]"),
            Some("unit \"m\"]")
        );
        assert_eq!(
            hint_in("float speed = 4.0; -- the speed"),
            None,
            "an ordinary comment"
        );
        assert_eq!(hint_in("string s = \"-- [no]\";"), None, "inside text");
        assert_eq!(
            hint_in("string s = \"a \\\" -- [no]\";"),
            None,
            "past an escaped quote"
        );
        assert_eq!(hint_in("int n = 1; --[[ block ]]"), None);
        assert_eq!(hint_in("int n = 1;"), None);
    }

    #[test]
    fn hints_go_on_their_fields_and_mistakes_have_their_lines() {
        let text = "fetch \"solis/project\":Sound;\n\
                    component Guard {\n\
                    \x20   has {\n\
                    \x20       float speed = 4.0; -- [range 0 10, step 0.5, unit \"m/s\", tip \"Walking speed\"]\n\
                    \x20       Sound alertSound = Sound.Sfx.Alert; -- [range 0 1, group \"Noise\"]\n\
                    \x20       string says = \"\"; -- [multiline, colour]\n\
                    \x20   }\n\
                    }\n";
        let mut schemas = vec![guard()];
        let problems = put_on(&[("guard.lst".into(), text.into())], &mut schemas);
        let speed = &schemas[0].fields[0].hint;
        assert_eq!(speed.range, Some((0.0, 10.0)));
        assert!(speed.slider);
        assert_eq!(speed.step, Some(0.5));
        assert_eq!(speed.unit, " m/s");
        assert_eq!(speed.tip, "Walking speed");
        let alert = &schemas[0].fields[1].hint;
        assert_eq!(
            alert.offers,
            Offer::Names(NameKind::Sound),
            "a Sound picks sounds"
        );
        assert_eq!(alert.group, "Noise", "the hint that fits is kept");
        assert_eq!(alert.range, None);
        assert!(schemas[0].fields[2].hint.multiline);
        let said: Vec<(u32, &str)> = problems
            .iter()
            .map(|p| (p.line, p.message.as_str()))
            .collect();
        assert_eq!(
            said,
            [
                (
                    5,
                    "the hint on alertSound: range is for numbers (int, float, vec2 and vec3), and alertSound is a Sound"
                ),
                (
                    6,
                    "the hint on says: colour isn't a hint Solis knows: range, step, unit, tip, group, multiline or hidden"
                ),
            ]
        );
        assert!(problems.iter().all(|p| p.file == "guard.lst"));
    }

    #[test]
    fn a_start_outside_the_range_and_an_unclosed_hint_are_problems() {
        let text = "component Guard { has {\n\
                    float speed = 4.0; -- [range 5 10]\n\
                    string says = \"\"; -- [tip \"open\"\n\
                    } }\n";
        let mut schemas = vec![guard()];
        let problems = put_on(&[("g.lst".into(), text.into())], &mut schemas);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems[0]
                .message
                .contains("starts outside its range (5 to 10)")
        );
        assert_eq!(problems[0].line, 2);
        assert!(problems[1].message.contains("has no ] closing it"));
        assert_eq!(
            schemas[0].fields[0].hint.range,
            Some((5.0, 10.0)),
            "kept all the same"
        );
    }

    #[test]
    fn a_field_without_hints_shows_as_it_always_did() {
        let text = "component Guard { has { float speed = 4.0; } }\n";
        let mut schemas = vec![guard()];
        assert!(put_on(&[("g.lst".into(), text.into())], &mut schemas).is_empty());
        assert_eq!(schemas[0], guard());
    }
}
