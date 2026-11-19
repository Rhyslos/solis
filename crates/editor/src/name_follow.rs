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
        (".prefab.lst", LibraryKind::Prefab),
        (".solassembly", LibraryKind::Prefab),
        (".png", LibraryKind::Texture),
        (".jpg", LibraryKind::Texture),
        (".jpeg", LibraryKind::Texture),
        (".sollevel", LibraryKind::Level),
        (".solarea", LibraryKind::Level),
        (".aseprite", LibraryKind::SpriteSheet),
        (".ase", LibraryKind::SpriteSheet),
        (".solframes", LibraryKind::SpriteClips),
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
    if old.kind != LibraryKind::Other {
        let (kind, old) = names::name_of(&old)?;
        let (same, new) = names::name_of(&new)?;
        return (kind == same && old != new).then(|| (vec![kind], old, new));
    }
    // A folder: every kind whose names are paths.
    let old = script::project_names::option_of(from)?;
    let new = script::project_names::option_of(to)?;
    let kinds = vec![
        NameKind::Sound,
        NameKind::Clip,
        NameKind::Picture,
        NameKind::Prefab,
        NameKind::Level,
    ];
    (old != new).then_some((kinds, old, new))
}

/// `value` with `old` (or a name inside it) made `new`, if it names it.
fn followed(value: &str, old: &str, new: &str) -> Option<String> {
    if value.eq_ignore_ascii_case(old) {
        return Some(new.to_owned());
    }
    let inside = value.get(old.len()..)?;
    (value[..old.len()].eq_ignore_ascii_case(old) && inside.starts_with('.'))
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
                        .then(|| followed(now, &old, &new))
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
    fn a_file_or_folder_moved_renames_its_names() {
        assert_eq!(
            renamed("sfx/alert.solsound", "sfx/alarm.solsound"),
            Some((vec![NameKind::Sound], "Sfx.Alert".into(), "Sfx.Alarm".into()))
        );
        assert_eq!(
            renamed("coin.prefab.lst", "loot/coin.prefab.lst"),
            Some((vec![NameKind::Prefab], "Coin".into(), "Loot.Coin".into()))
        );
        let (kinds, old, new) = renamed("sfx", "effects").unwrap();
        assert!(kinds.contains(&NameKind::Sound) && kinds.contains(&NameKind::Picture));
        assert_eq!((old.as_str(), new.as_str()), ("Sfx", "Effects"));
        assert_eq!(renamed("a.solsound", "a.png"), None, "it isn't the same kind now");
        assert_eq!(renamed("notes.md", "more.md"), None, "a text file names nothing");
        assert_eq!(
            renamed("ui/hud.lst", "ui/overlay.lst"),
            Some((vec![NameKind::Screen], "Hud".into(), "Overlay".into()))
        );
    }

    #[test]
    fn a_name_and_the_names_inside_it_follow() {
        assert_eq!(followed("Sfx.Alert", "Sfx.Alert", "Sfx.Alarm").as_deref(), Some("Sfx.Alarm"));
        assert_eq!(followed("sfx.alert", "Sfx.Alert", "Sfx.Alarm").as_deref(), Some("Sfx.Alarm"));
        assert_eq!(followed("Sfx.Alert", "Sfx", "Effects").as_deref(), Some("Effects.Alert"));
        assert_eq!(followed("Sfxs.Alert", "Sfx", "Effects"), None, "only whole names");
        assert_eq!(followed("Tools.Axe", "Tools", "Kit").as_deref(), Some("Kit.Axe"), "a picture's crops");
        assert_eq!(followed("Sf", "Sfx", "Effects"), None);
    }
}
