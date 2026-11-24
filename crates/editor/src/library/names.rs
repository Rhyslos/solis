//! The project's names a Library file stands for (decision 0127): the
//! option game code names it by, as `solis/project.lst` writes it
//! (`sfx/alert.solsound` is `Sound.Alert`). A field that picks one of
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
        LibraryKind::Module => Some((NameKind::Module, ending(&[".module.lst", ".solassembly"])?)),
        LibraryKind::Texture => Some((NameKind::Picture, ending(&[".png", ".jpg", ".jpeg"])?)),
        LibraryKind::Level => Some((NameKind::Level, ending(&[".sollevel", ".solarea"])?)),
        LibraryKind::SpriteSheet => Some((NameKind::Clip, ending(&[".aseprite", ".ase"])?)),
        LibraryKind::SpriteClips => Some((NameKind::Clip, ending(&[".clips.lst"])?)),
        LibraryKind::Font => Some((NameKind::Font, ending(&[".ttf", ".otf"])?)),
        LibraryKind::Code if path.starts_with(script::screens::FOLDER) => {
            Some((NameKind::Screen, 0))
        }
        _ => None,
    }
}

/// The path `item` stands for as one of the project's names (without its
/// ending: `sfx/alert` for `sfx/alert.solsound`), and its kind, if it is
/// one; a screen's is [`screen_of`]'s.
pub(crate) fn stem_of(item: &LibraryItem) -> Option<(NameKind, String)> {
    let (kind, ending) = kind_of(item)?;
    (kind != NameKind::Screen).then(|| (kind, item.path[..item.path.len() - ending].to_owned()))
}

/// The screen `item` is, by its name (`PauseMenu` for `ui/pauseMenu.lst`).
pub(crate) fn screen_of(item: &LibraryItem) -> Option<(NameKind, String)> {
    let (kind, _) = kind_of(item)?;
    let inside = item.path.strip_prefix(script::screens::FOLDER)?;
    (kind == NameKind::Screen).then(|| script::screens::screen_name(inside).map(|n| (kind, n)))?
}

/// The key a name is found by: options are found again ignoring
/// capitals, as the project file names them.
pub(crate) fn key(kind: NameKind, option: &str) -> (NameKind, String) {
    (kind, option.to_ascii_lowercase())
}

/// The option, among `options`, that `item` stands for when `kind`
/// picks: a file dropped on a field.
pub(crate) fn dropped_option<'a>(
    library: &super::LibraryFiles,
    item: &LibraryItem,
    kind: NameKind,
    options: &'a [String],
) -> Option<&'a String> {
    let (its, option) = library.name_of(item)?;
    (its == kind && kind != NameKind::Clip)
        .then(|| options.iter().find(|o| o.eq_ignore_ascii_case(&option)))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::LibraryFiles;

    fn item(kind: LibraryKind, path: &str) -> LibraryItem {
        LibraryItem {
            kind,
            name: path.rsplit('/').next().unwrap_or_default().into(),
            path: path.into(),
            ..LibraryItem::default()
        }
    }

    /// The Library's files with the names a project with them would have
    /// (decision 0137: one dot, each name's file listed beside it).
    fn library() -> LibraryFiles {
        let mut files: LibraryFiles = vec![
            item(LibraryKind::SoundEvent, "sfx/alert.solsound"),
            item(LibraryKind::Module, "coin.module.lst"),
            item(LibraryKind::Texture, "portraits/guardFace.PNG"),
            item(LibraryKind::Texture, "sprites/tools.png"),
            item(LibraryKind::Level, "forest/cave.sollevel"),
            item(LibraryKind::SpriteSheet, "sprites/farmer.aseprite"),
            item(LibraryKind::Code, "ui/pauseMenu.lst"),
            item(LibraryKind::Code, "main.lst"),
            item(LibraryKind::Model, "ship.glb"),
        ]
        .into();
        let mut names = script::project_names::Names::with_modules(&["coin"])
            .and_sounds(&["sfx/alert"])
            .and_levels(&["forest/cave"])
            .and_clips(&["sprites/farmer/walk"], &[]);
        names.pictures = vec![
            "portraits/guardFace".into(),
            "sprites/tools".into(),
            "sprites/tools/Axe".into(),
        ];
        let (text, _) = script::project_names::project_file("Test 1.0", &names);
        let paths = script::project_names::paths_in(&[("solis/project.lst".into(), text)]);
        files.set_name_paths(std::sync::Arc::new(paths));
        files
    }

    #[test]
    fn each_kind_of_file_stands_for_its_name_with_one_dot() {
        let files = library();
        let name = |kind, path| files.name_of(&item(kind, path)).map(|(_, n)| n);
        assert_eq!(
            name(LibraryKind::SoundEvent, "sfx/alert.solsound").as_deref(),
            Some("Alert")
        );
        assert_eq!(
            name(LibraryKind::Module, "coin.module.lst").as_deref(),
            Some("Coin")
        );
        assert_eq!(
            name(LibraryKind::Texture, "portraits/guardFace.PNG").as_deref(),
            Some("GuardFace")
        );
        assert_eq!(
            name(LibraryKind::Level, "forest/cave.sollevel").as_deref(),
            Some("Cave")
        );
        assert_eq!(
            name(LibraryKind::Code, "ui/pauseMenu.lst").as_deref(),
            Some("PauseMenu")
        );
        assert_eq!(name(LibraryKind::Code, "main.lst"), None);
        assert_eq!(name(LibraryKind::Model, "ship.glb"), None);
    }

    #[test]
    fn a_clip_finds_its_sheet_and_a_crop_its_picture() {
        let files = library();
        let path = |kind, option| files.named(kind, option).map(|i| i.path.clone());
        assert_eq!(
            path(NameKind::Clip, "FarmerWalk").as_deref(),
            Some("sprites/farmer.aseprite")
        );
        assert_eq!(
            path(NameKind::Picture, "ToolsAxe").as_deref(),
            Some("sprites/tools.png")
        );
        assert_eq!(
            path(NameKind::Picture, "Tools").as_deref(),
            Some("sprites/tools.png")
        );
        assert_eq!(
            files.icon_picture("Picture.ToolsAxe").as_deref(),
            Some("sprites/tools.png#Axe")
        );
        assert_eq!(path(NameKind::Clip, "Nothing"), None);
        assert_eq!(files.names_of(NameKind::Sound), ["Alert"]);
    }
}
