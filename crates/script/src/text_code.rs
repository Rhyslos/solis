//! The game's text as game code reaches it (stage 1, translations;
//! [decision 0131](../../../docs/decisions/0131-translations-and-captions.md)):
//! each line of the main language is an option of `Text` in
//! `solis/project.lst` (`World:text(Text.ShopWelcome)`), and each line
//! that takes values is a method of its group's object in `solis/texts.lst`
//! (the game API guidelines' rule 4), its values in order:
//!
//! ```text
//! object ShopText {
//!     -- Shop.Welcome: Welcome, { $name }! You have { $gold } gold.
//!     method string welcome(string name, float gold) native;
//! }
//! ```
//!
//! So a misspelt key or a value left out is a mistake before the game
//! runs. The methods' bodies are lent by `crate::texts`.

use translations::ValueKind;

/// The file the lines with values are in, inside the engine's folder.
pub const FILE: &str = "texts.lst";

/// A line that takes values, as its method is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    /// Its key (`Shop.Welcome`).
    pub key: String,
    /// Its group's object (`ShopText`).
    pub object: String,
    /// Its method (`welcome`).
    pub method: String,
    /// Its values as Fluent names them (`player-name`), in order, each
    /// with its Lust name (`playerName`) and how it's used.
    pub values: Vec<(String, String, ValueKind)>,
    /// Its text in the main language, for the comment above it.
    pub text: String,
}

/// What the project's text gives game code: its keys, its languages'
/// options and its lines with values.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextNames {
    /// The main language's keys (`Shop.Welcome`), for `Text`.
    pub keys: Vec<String>,
    /// The languages' options (`En`, `Nb`), the main one first.
    pub languages: Vec<String>,
    /// The lines that take values, each a method.
    pub methods: Vec<Method>,
}

impl TextNames {
    /// The names `translations` give game code, and notes on lines whose
    /// methods can't be made. `taken` holds names that are another's
    /// already (the engine's objects, the tables').
    pub fn of(translations: &translations::Translations, taken: &[String]) -> (Self, Vec<String>) {
        let mut notes = Vec::new();
        let keys = translations.code_keys();
        let languages = translations
            .languages()
            .iter()
            .map(translations::Language::option)
            .collect();
        let mut methods: Vec<Method> = Vec::new();
        for key in &keys {
            let line = match translations.line(translations.main(), key) {
                Some(line) if !line.values.is_empty() => line,
                _ => continue,
            };
            let group = key.split('.').next().unwrap_or_default();
            let object = translations::keys::group_object(group);
            let method = translations::keys::method_name(key);
            if taken.iter().any(|t| t.eq_ignore_ascii_case(&object)) {
                notes.push(format!(
                    "{key} takes values, but its group's object {object} is another's name \
                     already: rename the group to reach it from game code."
                ));
                continue;
            }
            if let Some(other) = methods
                .iter()
                .find(|m| m.object == object && m.method.eq_ignore_ascii_case(&method))
            {
                notes.push(format!(
                    "{key} would be {object}:{method}, which {} is already: rename one.",
                    other.key
                ));
                continue;
            }
            let mut values = Vec::with_capacity(line.values.len());
            for (name, kind) in &line.values {
                match translations::keys::value_name(name)
                    .filter(|n| level::table::field_problem(n).is_none() && *n != method)
                {
                    Some(lust) if !values.iter().any(|(_, l, _)| *l == lust) => {
                        values.push((name.clone(), lust, *kind));
                    }
                    _ => {
                        notes.push(format!(
                            "{key}'s value ${name} can't be named in game code: give it a name of \
                             letters and digits that isn't one of Lust's own words."
                        ));
                        values.clear();
                        break;
                    }
                }
            }
            if values.is_empty() {
                continue;
            }
            methods.push(Method {
                key: key.clone(),
                object,
                method,
                values,
                text: line.text.clone(),
            });
        }
        (
            Self {
                keys,
                languages,
                methods,
            },
            notes,
        )
    }

    /// The objects the methods are on, each once, in order.
    pub fn objects(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for m in &self.methods {
            if !out.contains(&m.object.as_str()) {
                out.push(&m.object);
            }
        }
        out
    }
}

/// `solis/texts.lst`'s text, or none when no line takes values. `made_by`
/// names the program and version, for the first line.
pub fn texts_file(made_by: &str, names: &TextNames) -> Option<String> {
    let objects = names.objects();
    if objects.is_empty() {
        return None;
    }
    let mut out = format!(
        "-- Made by {made_by} from the main language's text (locale/): a method for each line\n\
         -- that takes values. It's written again whenever the text changes, so changes made\n\
         -- here don't last.\n"
    );
    for object in &objects {
        out.push_str(&format!("\nobject {object} {{\n"));
        for m in names.methods.iter().filter(|m| m.object == *object) {
            let first: String = m
                .text
                .lines()
                .next()
                .unwrap_or_default()
                .chars()
                .take(100)
                .collect();
            let params: Vec<String> = m
                .values
                .iter()
                .map(|(_, lust, kind)| {
                    let ty = match kind {
                        ValueKind::Text => "string",
                        ValueKind::Number => "float",
                    };
                    format!("{ty} {lust}")
                })
                .collect();
            out.push_str(&format!(
                "    -- {}: {first}\n    method string {}({}) native;\n",
                m.key,
                m.method,
                params.join(", ")
            ));
        }
        out.push_str("}\n");
    }
    out.push_str(&format!("\npass {};\nend\n", objects.join(", ")));
    Some(out)
}

#[cfg(test)]
#[path = "text_code_tests.rs"]
mod tests;
