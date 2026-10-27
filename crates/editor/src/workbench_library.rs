//! The workbench's side of the Library: turning its requests into actions
//! for the app (which changes files), and placing what's dropped on the
//! viewport or the Area panel where it was dropped.

use crate::console::LogLevel;
use crate::library::{LibraryKind, LibraryRequest};
use crate::workbench::{Workbench, WorkbenchAction};
use glam::Vec3;
use reflect::{AssetId, EntityId};
use std::path::PathBuf;

impl Workbench {
    /// Carries out something asked for in the Library.
    pub(crate) fn library_request(
        &mut self,
        request: LibraryRequest,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let Some(assets) = self.project.as_ref().map(|p| p.assets.clone()) else {
            if request == LibraryRequest::Import {
                actions.push(WorkbenchAction::ImportModel);
            }
            return;
        };
        // Every path asked about must be a plain path inside the assets
        // folder: a problem's file comes from an error's text, which game
        // code writes, so it could name a network share or a drive.
        let path_of = |request: &LibraryRequest| match request {
            LibraryRequest::Open(_, path)
            | LibraryRequest::Show(path)
            | LibraryRequest::PlaySound(path) => Some(path.clone()),
            _ => None,
        };
        if let Some(path) = path_of(&request)
            && !path.is_empty()
            && !plain_relative(&path)
        {
            self.log(
                LogLevel::Warning,
                format!(
                    "Didn't open {}: it isn't a file in the project's assets folder.",
                    path.chars().take(200).collect::<String>()
                ),
            );
            return;
        }
        let full = |path: &str| -> PathBuf {
            if path.is_empty() {
                assets.clone()
            } else {
                assets.join(path)
            }
        };
        match request {
            LibraryRequest::Place(id) => {
                // A double-click places it where the camera looks, even if
                // an earlier drop of the same asset never arrived.
                self.place_targets.remove(&id);
                actions.push(WorkbenchAction::PlaceAsset(id));
            }
            LibraryRequest::Open(LibraryKind::Area, path) => {
                self.request(WorkbenchAction::OpenAreaFile(full(&path)), actions);
            }
            LibraryRequest::Open(kind, path) => {
                // Only kinds whose program is known to just show or edit
                // them open in it (a picture viewer, a text editor for
                // Lust); anything else, such as a file called
                // "brick.png.exe" or a .lua file another program might
                // run, is shown in its folder instead, never opened.
                let opens = match kind {
                    LibraryKind::Texture => true,
                    LibraryKind::Code => path.to_lowercase().ends_with(".lst"),
                    _ => false,
                };
                if opens {
                    actions.push(WorkbenchAction::OpenExternally(full(&path)));
                    // Said once a session: which program opens Lust is the
                    // computer's choice, and it should be an editor.
                    if kind == LibraryKind::Code && !self.told_how_lust_opens {
                        self.told_how_lust_opens = true;
                        self.log(
                            LogLevel::Info,
                            format!(
                                "Opening {path} in the program your computer uses for .lst \
                                 files. If it's the wrong one, choose a text or code editor in \
                                 Explorer (right-click the file > Open with)."
                            ),
                        );
                    }
                } else {
                    let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
                    actions.push(WorkbenchAction::OpenExternally(full(folder)));
                    self.log(
                        LogLevel::Info,
                        format!(
                            "Showing {path} in its folder: Solis doesn't open this kind of file \
                             in another program, so a file that isn't what it seems can't run."
                        ),
                    );
                }
            }
            LibraryRequest::Show(path) => {
                actions.push(WorkbenchAction::OpenExternally(full(&path)));
            }
            LibraryRequest::Move(from, to) => actions.push(WorkbenchAction::MoveFile {
                from: from.into(),
                to: to.into(),
            }),
            LibraryRequest::NewFolder(parent) => {
                actions.push(WorkbenchAction::NewFolder(parent.into()));
            }
            LibraryRequest::Delete(path) => actions.push(WorkbenchAction::DeleteFile(path.into())),
            LibraryRequest::Import => actions.push(WorkbenchAction::ImportModel),
            LibraryRequest::PlaySound(path) => {
                actions.push(WorkbenchAction::PlaySound(path.into()));
            }
            LibraryRequest::StopSound => actions.push(WorkbenchAction::StopSound),
        }
    }

    /// Shows a sound file in Properties when one is picked in the Library
    /// (`before` is what was picked before this frame).
    pub(crate) fn follow_sound_selection(&mut self, before: &Option<String>) {
        let picked = &self.library_state.selected;
        if picked != before
            && let Some(path) = picked
            && self
                .library
                .iter()
                .any(|i| &i.path == path && i.kind == LibraryKind::Sound)
        {
            self.sound_shown = Some((path.clone(), self.document.selection.clone()));
        }
    }

    /// Stops showing a sound file in Properties once the area's selection
    /// has changed or the file is gone. Called before the panels are
    /// drawn, so the change shows in the same frame.
    pub(crate) fn drop_stale_sound(&mut self) {
        let stale = self.sound_shown.as_ref().is_some_and(|(path, then)| {
            *then != self.document.selection || !self.library.iter().any(|i| &i.path == path)
        });
        if stale {
            self.sound_shown = None;
        }
    }

    /// The sound files as the app has measured them, by path inside the
    /// assets (`/` between folders).
    pub fn set_sounds(
        &mut self,
        sounds: std::collections::HashMap<String, crate::sound_view::SoundState>,
    ) {
        self.sounds = sounds;
    }

    /// The sound being listened to and how far it has got, or `None` when
    /// nothing plays. Called by the app every frame.
    pub fn set_preview(&mut self, preview: Option<crate::sound_view::Preview>) {
        self.preview = preview;
    }

    /// Whether a sound is being listened to (the app draws frames for its
    /// playhead).
    pub fn previewing(&self) -> bool {
        self.preview.is_some()
    }

    /// Places `asset`, dropped at `at` in the world (under `parent`, if it
    /// was dropped onto an entity): it goes there once the app has loaded
    /// it.
    pub(crate) fn place_dropped(&mut self, asset: AssetId, at: Vec3, parent: Option<EntityId>) {
        self.place_targets.insert(asset, (at, parent));
        self.outbox.push(WorkbenchAction::PlaceAsset(asset));
    }

    /// Forgets where `asset` was to go: its import failed, so a later
    /// placing of it doesn't land at an old drop point.
    pub fn placing_failed(&mut self, asset: AssetId) {
        self.place_targets.remove(&asset);
    }

    /// Where a model or assembly being placed goes: the point it was
    /// dropped at (made relative to the entity it goes under) and that
    /// entity, or `None` to place it where the camera looks.
    pub(crate) fn take_place_target(&mut self, asset: AssetId) -> Option<(Vec3, Option<EntityId>)> {
        let (at, parent) = self.place_targets.remove(&asset)?;
        let area = self.document.area();
        let parent = parent.filter(|&p| area.contains(p));
        let local = match parent {
            Some(p) => area.world_matrix(p).inverse().transform_point3(at),
            None => at,
        };
        local.is_finite().then_some((local, parent))
    }
}

/// Whether `path` is a plain path inside a folder: parts separated by `/`
/// (or a backslash), none empty, `.` or `..`, and no drive (`:`), so joined to a
/// folder it stays inside it on every system.
pub(crate) fn plain_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(':')
        && path
            .split(['/', '\\'])
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[cfg(test)]
#[path = "workbench_library_tests.rs"]
mod tests;

#[cfg(test)]
mod opening_tests {
    use super::*;
    use crate::test_support::info;

    fn opened(kind: LibraryKind, path: &str) -> PathBuf {
        let mut w = Workbench::new(info(), None);
        let assets = std::env::temp_dir().join("solis-library-open");
        w.project = Some(crate::library::ProjectInfo {
            name: "Opening".into(),
            root: assets.clone(),
            assets: assets.clone(),
        });
        let mut actions = Vec::new();
        w.library_request(LibraryRequest::Open(kind, path.into()), &mut actions);
        match actions.as_slice() {
            [WorkbenchAction::OpenExternally(p)] => p.strip_prefix(&assets).unwrap().to_path_buf(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn only_pictures_and_lust_open_in_another_program_the_rest_show_their_folder() {
        // The security review: a file called "brick.png.exe" looked like a
        // picture, and double-clicking it ran it.
        assert_eq!(
            opened(LibraryKind::Texture, "props/brick.png"),
            PathBuf::from("props/brick.png")
        );
        assert_eq!(
            opened(LibraryKind::Code, "main.lst"),
            PathBuf::from("main.lst")
        );
        assert_eq!(
            opened(LibraryKind::Other, "props/brick.png.exe"),
            PathBuf::from("props")
        );
        assert_eq!(opened(LibraryKind::Other, "run.bat"), PathBuf::from(""));
        assert_eq!(
            opened(LibraryKind::Code, "ai/sparks.lua"),
            PathBuf::from("ai")
        );
    }

    #[test]
    fn a_path_outside_the_assets_folder_is_never_opened() {
        // The third security audit: a problem's file comes from an error's
        // text, so a click could open a network share in Explorer.
        let mut w = Workbench::new(info(), None);
        let assets = std::env::temp_dir().join("solis-library-open");
        w.project = Some(crate::library::ProjectInfo {
            name: "Opening".into(),
            root: assets.clone(),
            assets,
        });
        for path in [
            "//attacker.example/share/a.lua",
            "\\\\attacker\\share",
            "C:/Windows/notepad.exe",
            "C:notepad.exe",
            "../outside.lst",
            "props/../../outside.lst",
            "/etc/passwd",
        ] {
            let mut actions = Vec::new();
            w.library_request(
                LibraryRequest::Open(LibraryKind::Code, path.into()),
                &mut actions,
            );
            w.library_request(LibraryRequest::Show(path.into()), &mut actions);
            assert!(actions.is_empty(), "{path}: {actions:?}");
        }
        assert!(plain_relative("props/brick.png"));
    }

    #[test]
    fn opening_lust_says_once_which_program_opens_it() {
        // The user test: double-clicking main.lst opened an unexpected
        // program, with no word on why.
        let mut w = Workbench::new(info(), None);
        let assets = std::env::temp_dir().join("solis-library-open");
        w.project = Some(crate::library::ProjectInfo {
            name: "Opening".into(),
            root: assets.clone(),
            assets,
        });
        let mut actions = Vec::new();
        for _ in 0..3 {
            w.library_request(
                LibraryRequest::Open(LibraryKind::Code, "main.lst".into()),
                &mut actions,
            );
        }
        let told = w
            .log_lines()
            .iter()
            .filter(|l| l.text.contains("Open with"))
            .count();
        assert_eq!(told, 1);
        assert!(plain_relative("a\\b.lst"));
    }
}
