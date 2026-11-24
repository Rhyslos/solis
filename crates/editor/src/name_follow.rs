//! A renamed file followed by the fields naming it (decision 0127): a
//! field of the project's names keeps the option's name (`Sfx.Alert`),
//! so when the Library renames or moves `sfx/alert.solsound`, the open
//! level's fields naming it are changed to its new name, as one undo
//! step. A folder moved takes the names inside it along.

use crate::library::{LibraryItem, LibraryKind, names};
use crate::workbench::Workbench;
use level::Edit;
use reflect::{NameKind, Offer, Value};

/// The kinds of file Solis knows by their endings, for a path that has
/// just moved (the Library no longer lists it).
fn item_at(path: &str) -> LibraryItem {
    let lower = path.to_ascii_lowercase();
    let kind = [
        (".solsound", LibraryKind::SoundEvent),
        (".module.lst", LibraryKind::Module),
        (".solassembly", LibraryKind::Module),
        (".png", LibraryKind::Texture),
        (".jpg", LibraryKind::Texture),
        (".jpeg", LibraryKind::Texture),
        (".sollevel", LibraryKind::Level),
        (".solarea", LibraryKind::Level),
        (".aseprite", LibraryKind::SpriteSheet),
        (".ase", LibraryKind::SpriteSheet),
        (".clips.lst", LibraryKind::SpriteClips),
        (".ttf", LibraryKind::Font),
        (".otf", LibraryKind::Font),
        (".lst", LibraryKind::Code),
    ]
    .into_iter()
    .find(|(ending, _)| lower.ends_with(ending))
    .map_or(LibraryKind::Other, |(_, kind)| kind);
    LibraryItem {
        kind,
        path: path.to_owned(),
        ..LibraryItem::default()
    }
}

/// What a move from `from` to `to` (paths inside the project) renames:
/// the kinds it touches (one for a file, every kind with folders for a
/// folder), and the old and new option.
pub(crate) fn renamed(from: &str, to: &str) -> Option<(Vec<NameKind>, String, String)> {
    let (old, new) = (item_at(from), item_at(to));
    // A folder renames nothing: names have one dot (decision 0137).
    if old.kind == LibraryKind::Other {
        return None;
    }
    // A file's name with one dot, by its own name (a clash of two files'
    // names is said when the names are written again).
    let name = |item: &LibraryItem| -> Option<(NameKind, String)> {
        if let Some(screen) = names::screen_of(item) {
            return Some(screen);
        }
        let (kind, stem) = names::stem_of(item)?;
        let file = stem.rsplit('/').next().unwrap_or(&stem);
        let option = script::project_names::one_dot(&[(String::new(), file.to_owned())])
            .pop()
            .flatten()?;
        Some((kind, option))
    };
    let (kind, old) = name(&old)?;
    let (same, new) = name(&new)?;
    (kind == same && old != new).then(|| (vec![kind], old, new))
}

/// `value` with `old` made `new`, if it names it; for a clip or a crop
/// (`parts`), also a name of one of its parts (`KnightWalk` follows
/// `Knight` to `HeroWalk`: decision 0137, names have one dot).
fn followed(value: &str, old: &str, new: &str, parts: bool) -> Option<String> {
    if value.eq_ignore_ascii_case(old) {
        return Some(new.to_owned());
    }
    let inside = value.get(old.len()..)?;
    (parts
        && value[..old.len()].eq_ignore_ascii_case(old)
        && inside.starts_with(|c: char| c.is_ascii_uppercase()))
    .then(|| format!("{new}{inside}"))
}

impl Workbench {
    /// The open level's fields naming what moved from `from` to `to`
    /// (inside the project) changed to its new name, as one undo step.
    /// Says how many; nothing when none name it.
    pub fn follow_renamed_names(&mut self, from: &str, to: &str) {
        let Some((kinds, old, new)) = renamed(from, to) else {
            return;
        };
        let registry = &self.open.document.registry;
        let mut edits = Vec::new();
        for (entity, data) in self.open.document.level().iter() {
            for (component, values) in &data.components {
                let Some(schema) = registry.get(component) else {
                    continue;
                };
                for field in &schema.fields {
                    let Offer::Names(kind) = field.hint.offers else {
                        continue;
                    };
                    let Some(Value::Text(now)) = values.get(&field.name) else {
                        continue;
                    };
                    if let Some(value) = kinds
                        .contains(&kind)
                        .then(|| {
                            let parts = matches!(kind, NameKind::Clip | NameKind::Picture);
                            followed(now, &old, &new, parts)
                        })
                        .flatten()
                    {
                        edits.push(Edit::SetField {
                            entity,
                            component: component.clone(),
                            field: field.name.clone(),
                            value: Some(Value::Text(value)),
                        });
                    }
                }
            }
        }
        if edits.is_empty() {
            return;
        }
        let count = edits.len();
        if self.command("Follow a renamed file", edits, None, None) {
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "{count} setting{} naming {old} now name{} {new}.",
                    if count == 1 { "" } else { "s" },
                    if count == 1 { "s" } else { "" }
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_renamed_renames_its_name_and_a_move_doesnt() {
        assert_eq!(
            renamed("sfx/alert.solsound", "sfx/alarm.solsound"),
            Some((vec![NameKind::Sound], "Alert".into(), "Alarm".into()))
        );
        // Names have one dot: moving a file to another folder keeps it.
        assert_eq!(renamed("coin.module.lst", "loot/coin.module.lst"), None);
        assert_eq!(renamed("sfx", "effects"), None, "a folder renames nothing");
        assert_eq!(
            renamed("a.solsound", "a.png"),
            None,
            "it isn't the same kind now"
        );
        assert_eq!(
            renamed("notes.md", "more.md"),
            None,
            "a text file names nothing"
        );
        assert_eq!(
            renamed("ui/hud.lst", "ui/overlay.lst"),
            Some((vec![NameKind::Screen], "Hud".into(), "Overlay".into()))
        );
    }

    #[test]
    fn a_name_and_its_parts_names_follow() {
        assert_eq!(
            followed("Alert", "Alert", "Alarm", false).as_deref(),
            Some("Alarm")
        );
        assert_eq!(
            followed("alert", "Alert", "Alarm", false).as_deref(),
            Some("Alarm")
        );
        assert_eq!(
            followed("KnightWalk", "Knight", "Hero", true).as_deref(),
            Some("HeroWalk"),
            "a sheet's clips"
        );
        assert_eq!(
            followed("ToolsAxe", "Tools", "Kit", true).as_deref(),
            Some("KitAxe")
        );
        assert_eq!(
            followed("CoinPurse", "Coin", "Gold", false),
            None,
            "a module's alone"
        );
        assert_eq!(
            followed("Knightly", "Knight", "Hero", true),
            None,
            "only whole parts"
        );
        assert_eq!(followed("Kn", "Knight", "Hero", true), None);
    }
}
