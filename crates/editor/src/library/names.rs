//! The project's names a Library file stands for (decision 0127): the
//! option game code names it by, as `solis/project.lst` writes it
//! (`sfx/alert.solsound` is `Sound.Sfx.Alert`). A field that picks one of
//! the project's names finds each option's file here, for its thumbnail
//! and to play a sound, and takes a file dropped on it by this name.

use super::{LibraryItem, LibraryKind};
use reflect::NameKind;

/// The kind of name a file of `kind` is, and its path's ending, when it
/// is one; a sprite sheet's or clips file's is `Clip`, which its clips
/// are inside.
fn kind_of(item: &LibraryItem) -> Option<(NameKind, usize)> {
    let path = item.path.to_ascii_lowercase();
    let ending = |endings: &[&str]| endings.iter().find(|e| path.ends_with(*e)).map(|e| e.len());
    match item.kind {
        LibraryKind::SoundEvent => Some((NameKind::Sound, ending(&[".solsound"])?)),
        LibraryKind::Prefab => Some((NameKind::Prefab, ending(&[".prefab.lst", ".solassembly"])?)),
        LibraryKind::Texture => Some((NameKind::Picture, ending(&[".png", ".jpg", ".jpeg"])?)),
        LibraryKind::Level => Some((NameKind::Level, ending(&[".sollevel", ".solarea"])?)),
        LibraryKind::SpriteSheet => Some((NameKind::Clip, ending(&[".aseprite", ".ase"])?)),
        LibraryKind::SpriteClips => Some((NameKind::Clip, ending(&[".solframes"])?)),
        LibraryKind::Code if path.starts_with(script::screens::FOLDER) => {
            Some((NameKind::Screen, 0))
        }
        _ => None,
    }
}

/// The name `item` stands for, as its kind and the option (`Sfx.Alert`),
/// if it stands for one game code can name.
pub(crate) fn name_of(item: &LibraryItem) -> Option<(NameKind, String)> {
    let (kind, ending) = kind_of(item)?;
    if kind == NameKind::Screen {
        let inside = item.path.strip_prefix(script::screens::FOLDER)?;
        return Some((kind, script::screens::screen_name(inside)?));
    }
    let path = &item.path[..item.path.len() - ending];
    Some((kind, script::project_names::option_of(path)?))
}

/// The key a name is found by: options are found again ignoring
/// capitals, as the project file names them.
pub(crate) fn key(kind: NameKind, option: &str) -> (NameKind, String) {
    (kind, option.to_ascii_lowercase())
}

/// The option, among `options`, that `item` stands for when `kind`
/// picks: a file dropped on a field.
pub(crate) fn dropped_option<'a>(
    item: &LibraryItem,
    kind: NameKind,
    options: &'a [String],
) -> Option<&'a String> {
    let (its, option) = name_of(item)?;
    (its == kind && kind != NameKind::Clip)
        .then(|| options.iter().find(|o| o.eq_ignore_ascii_case(&option)))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: LibraryKind, path: &str) -> LibraryItem {
        LibraryItem {
            kind,
            name: path.rsplit('/').next().unwrap_or_default().into(),
            path: path.into(),
            ..LibraryItem::default()
        }
    }

    #[test]
    fn each_kind_of_file_stands_for_its_option() {
        let cases = [
            (
                LibraryKind::SoundEvent,
                "sfx/alert.solsound",
                NameKind::Sound,
                "Sfx.Alert",
            ),
            (
                LibraryKind::Prefab,
                "coin.prefab.lst",
                NameKind::Prefab,
                "Coin",
            ),
            (
                LibraryKind::Prefab,
                "ships/old.solassembly",
                NameKind::Prefab,
                "Ships.Old",
            ),
            (
                LibraryKind::Texture,
                "portraits/guardFace.PNG",
                NameKind::Picture,
                "Portraits.GuardFace",
            ),
            (
                LibraryKind::Level,
                "forest/cave.sollevel",
                NameKind::Level,
                "Forest.Cave",
            ),
            (
                LibraryKind::SpriteSheet,
                "sprites/farmer.aseprite",
                NameKind::Clip,
                "Sprites.Farmer",
            ),
            (
                LibraryKind::Code,
                "ui/pauseMenu.lst",
                NameKind::Screen,
                "PauseMenu",
            ),
        ];
        for (kind, path, names, option) in cases {
            assert_eq!(
                name_of(&item(kind, path)),
                Some((names, option.to_string())),
                "{path}"
            );
        }
        assert_eq!(
            name_of(&item(LibraryKind::Code, "main.lst")),
            None,
            "not a screen"
        );
        assert_eq!(name_of(&item(LibraryKind::Model, "ship.glb")), None);
        assert_eq!(
            name_of(&item(LibraryKind::SoundEvent, "sfx/2bad.solsound")),
            None
        );
    }

    #[test]
    fn a_dropped_file_is_taken_only_by_its_own_kind_among_the_options() {
        let options = vec!["Sfx".to_string(), "Sfx.Alert".to_string()];
        let alert = item(LibraryKind::SoundEvent, "sfx/alert.solsound");
        assert_eq!(
            dropped_option(&alert, NameKind::Sound, &options),
            Some(&options[1])
        );
        assert_eq!(dropped_option(&alert, NameKind::Prefab, &options), None);
        let gone = item(LibraryKind::SoundEvent, "sfx/new.solsound");
        assert_eq!(
            dropped_option(&gone, NameKind::Sound, &options),
            None,
            "not yet an option"
        );
        let sheet = item(LibraryKind::SpriteSheet, "sfx.aseprite");
        assert_eq!(
            dropped_option(&sheet, NameKind::Clip, &options),
            None,
            "a sheet holds many clips"
        );
    }
}
