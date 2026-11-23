//! A component's fields as its Lust is written (decision 0127): each
//! field in a component's `has { }`, with the type written before its
//! name and the line its `;` ends on. Read as Lust's own reader splits the
//! text ([`crate::lust_guard`]), so a field in a comment or in text isn't
//! one. Game code's field hints are the comment after that `;`
//! (`float speed = 4.0; -- [range 0 10]`), which `script` reads from the
//! line.

use crate::lust_guard::{Piece, Pieces};

/// One field of a component, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WrittenField<'a> {
    /// The component it's in: `Guard`.
    pub component: &'a str,
    /// The type written before its name: `float`, `Sound`.
    pub kind: &'a str,
    /// Its name: `speed`.
    pub name: &'a str,
    /// The line its `;` is on, from 1.
    pub line: u32,
    /// Its starting value as written, between `=` and `;` and trimmed:
    /// `Sound.Sfx.Alert`, `[0, 0, 1]`; empty when it has none
    /// (`FarmerAnimator anim;`).
    pub start: &'a str,
}

/// Where the reader is in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum At<'a> {
    /// Outside any component.
    Outside,
    /// After `component`, waiting for its name.
    Named,
    /// After `component Name`, waiting for its `{`.
    Opening(&'a str),
    /// Inside the component, `depth` braces deep (1 is its own).
    Inside(&'a str, usize),
    /// After `has` in the component, waiting for its `{`.
    Has(&'a str),
    /// Inside the component's `has { }`.
    Fields(&'a str),
}

/// The fields of every component in `text`, in the order written. At
/// most `most` are given (the text is game code: never trusted to be
/// small).
pub fn fields(text: &str, most: usize) -> Vec<WrittenField<'_>> {
    let mut pieces = Pieces::new(text);
    let mut at = At::Outside;
    let mut found = Vec::new();
    // The words of the field being read, and how deep in brackets its
    // starting value is.
    let mut words: Vec<&str> = Vec::new();
    let mut started = false;
    let mut depth = 0usize;
    // Where the field's starting value begins in the text, after its `=`.
    let mut from = None;
    loop {
        let before = text.len() - pieces.left();
        let Some((line, piece)) = pieces.next() else {
            break;
        };
        if found.len() >= most {
            break;
        }
        at = match (at, piece) {
            (At::Outside, Piece::Word("component")) => At::Named,
            (At::Outside, _) => At::Outside,
            (At::Named, Piece::Word(name)) => At::Opening(name),
            (At::Named, _) => At::Outside,
            (At::Opening(name), Piece::Symbol("{")) => At::Inside(name, 1),
            (At::Opening(_), _) => At::Outside,
            (At::Inside(name, 1), Piece::Word("has")) => At::Has(name),
            (At::Inside(name, n), Piece::Symbol("{")) => At::Inside(name, n + 1),
            (At::Inside(_, 1), Piece::Symbol("}")) => At::Outside,
            (At::Inside(name, n), Piece::Symbol("}")) => At::Inside(name, n - 1),
            (inside @ At::Inside(..), _) => inside,
            (At::Has(name), Piece::Symbol("{")) => {
                words.clear();
                started = false;
                depth = 0;
                from = None;
                At::Fields(name)
            }
            (At::Has(name), _) => At::Inside(name, 1),
            (At::Fields(name), Piece::Symbol("}")) if depth == 0 => At::Inside(name, 1),
            (At::Fields(name), Piece::Symbol(";")) if depth == 0 => {
                let start = from.map_or("", |at| text[at..before].trim());
                if let Some(field) = field_of(name, &words, line, start) {
                    found.push(field);
                }
                words.clear();
                started = false;
                from = None;
                At::Fields(name)
            }
            (At::Fields(name), piece) => {
                match piece {
                    Piece::Symbol("=") if depth == 0 && !started => {
                        started = true;
                        from = Some(text.len() - pieces.left());
                    }
                    Piece::Symbol("{" | "(" | "[") => depth += 1,
                    Piece::Symbol("}" | ")" | "]") => depth = depth.saturating_sub(1),
                    Piece::Word(word) if !started && depth == 0 => words.push(word),
                    _ => {}
                }
                At::Fields(name)
            }
        };
    }
    found
}

/// A field from the words before its `=` or `;`: `lock`, then its type
/// (`Sound`, or `int array`), then its name.
fn field_of<'a>(
    component: &'a str,
    words: &[&'a str],
    line: u32,
    start: &'a str,
) -> Option<WrittenField<'a>> {
    let words = match words.first() {
        Some(&"lock") => &words[1..],
        _ => words,
    };
    let (&name, before) = words.split_last()?;
    let &kind = before.first()?;
    Some(WrittenField {
        component,
        kind,
        name,
        line,
        start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_field_has_its_type_name_and_line() {
        let text = "state Mood = [Calm, Alert];\n\
                    component Guard {\n\
                    \x20   has {\n\
                    \x20       float speed = 4.0; -- [range 0 10]\n\
                    \x20       lock Sound alertSound = Sound.Sfx.Alert;\n\
                    \x20       vec3 home = [0, 0,\n\
                    \x20           1];\n\
                    \x20       string says = \"a } and a ;\";\n\
                    \x20   }\n\
                    \x20   method update(float dt) { float local = 1; }\n\
                    }\n\
                    class NotOne { has { int skipped = 1; } }\n";
        let found = fields(text, 100);
        let shown: Vec<(&str, &str, &str, u32)> = found
            .iter()
            .map(|f| (f.component, f.kind, f.name, f.line))
            .collect();
        let starts: Vec<&str> = found.iter().map(|f| f.start).collect();
        assert_eq!(
            starts,
            [
                "4.0",
                "Sound.Sfx.Alert",
                "[0, 0,\n            1]",
                "\"a } and a ;\""
            ]
        );
        assert_eq!(
            shown,
            [
                ("Guard", "float", "speed", 4),
                ("Guard", "Sound", "alertSound", 5),
                ("Guard", "vec3", "home", 7),
                ("Guard", "string", "says", 8),
            ]
        );
    }

    #[test]
    fn a_field_in_a_comment_or_after_the_component_isn_t_one() {
        let text = "-- component Fake { has { int no = 1; } }\n\
                    component Real { has { FarmerAnimator anim; } }\n\
                    int after = 2;\n";
        let found = fields(text, 100);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].kind, found[0].name), ("FarmerAnimator", "anim"));
        assert_eq!(found[0].start, "");
    }

    #[test]
    fn damaged_text_gives_what_it_can_and_never_more_than_asked() {
        for text in [
            "component",
            "component X {",
            "component X { has {",
            "component X { has { int",
            "}}}};;;",
        ] {
            let _ = fields(text, 10);
        }
        let many = format!("component X {{ has {{ {} }} }}", "int a = 1; ".repeat(50));
        assert_eq!(fields(&many, 7).len(), 7);
    }
}
