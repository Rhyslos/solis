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
            LibraryRequest::Place(id) if self.viewport.sheets.get(id).is_some() => {
                self.place_sprite(id, None);
            }
            LibraryRequest::Place(id) if self.viewport.sheets.tile_map(id).is_some() => {
                self.place_tile_map(id, None);
            }
            LibraryRequest::Place(id) => {
                // A double-click places it where the camera looks, even if
                // an earlier drop of the same asset never arrived.
                self.place_targets.remove(&id);
                actions.push(WorkbenchAction::PlaceAsset(id));
            }
            LibraryRequest::Open(LibraryKind::SpriteClips, path) => self.open_frames(&path),
            LibraryRequest::Open(LibraryKind::Animator, path) => self.open_animator(&path),
            LibraryRequest::Open(LibraryKind::Tileset, path) => self.open_tileset(&path),
            LibraryRequest::NewAnimator(folder) => self.new_animator(&folder),
            LibraryRequest::NewTileset(folder) => self.new_tileset(&folder),
            LibraryRequest::NewTileMap(folder) => self.new_tile_map(&folder),
            LibraryRequest::SaveTileset(path, set) => self.save_tileset(&path, set),
            LibraryRequest::ShapeTileMap(path, tileset, size) => {
                let fill = tileset
                    .and_then(|t| self.viewport.sheets.tileset(t))
                    .and_then(|t| t.terrains.first())
                    .map(|t| t.code as u8);
                self.change_map(&path, |map| {
                    let mut changed = map.tileset != tileset;
                    map.tileset = tileset;
                    // A map with no ground yet takes its first tileset's
                    // first terrain.
                    if changed
                        && let Some(code) = fill
                        && map
                            .layer_cells(0)
                            .is_some_and(|c| c.iter().all(|&v| v == 0))
                    {
                        let [w, h] = map.size();
                        for y in 0..h {
                            for x in 0..w {
                                let _ = map.set_code(0, x, y, code);
                            }
                        }
                    }
                    changed |= map.resize(size);
                    changed
                });
            }
            LibraryRequest::MakeClipsFile(path) => {
                if let Some(id) = crate::frames_editor::id_of(&self.viewport.sheets, &path) {
                    self.make_clips_file(&path, id);
                }
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
            LibraryRequest::NewSoundEvent(folder) => {
                actions.push(WorkbenchAction::NewSoundEvent(folder.into()));
            }
            LibraryRequest::SaveSoundEvent(path, event) => {
                if plain_relative(&path) {
                    actions.push(WorkbenchAction::SaveSoundEvent {
                        path: path.into(),
                        event,
                    });
                }
            }
            LibraryRequest::AddTake(path, take) => self.add_take(&path, take, actions),
        }
    }

    /// Shows a sound file in Properties when one is picked in the Library
    /// (`before` is what was picked before this frame).
    pub(crate) fn follow_sound_selection(&mut self, before: &Option<String>) {
        let picked = &self.library_state.selected;
        if picked != before
            && let Some(path) = picked
            && self.library.iter().any(|i| {
                &i.path == path
                    && matches!(
                        i.kind,
                        LibraryKind::Sound
                            | LibraryKind::SoundEvent
                            | LibraryKind::SpriteSheet
                            | LibraryKind::Tileset
                            | LibraryKind::TileMap
                    )
            })
        {
            self.sound_shown = Some((path.clone(), self.document.selection.clone()));
            self.mixer.shown = None;
        }
    }

    /// Adds `take` to the sound event at `path`, unless it's already there
    /// or the event is full (said in the Console).
    fn add_take(&mut self, path: &str, take: AssetId, actions: &mut Vec<WorkbenchAction>) {
        let Some(crate::sound_view::EventState::Read { event, .. }) = self.sound_events.get(path)
        else {
            self.log(
                LogLevel::Warning,
                format!("Couldn't add the take: {path} isn't a sound event Solis could read."),
            );
            return;
        };
        if event.takes.contains(&take) {
            return;
        }
        if event.takes.len() >= sound::event::MAX_TAKES {
            self.log(
                LogLevel::Warning,
                format!(
                    "{path} already has {} takes, the most a sound event holds.",
                    sound::event::MAX_TAKES
                ),
            );
            return;
        }
        let mut event = event.clone();
        event.takes.push(take);
        if plain_relative(path) {
            actions.push(WorkbenchAction::SaveSoundEvent {
                path: path.into(),
                event,
            });
        }
    }

    /// Picks the file at `path` (inside the assets) in the Library, opens
    /// its folder there, and shows it in Properties if it's a sound or
    /// sound event (the app calls this after making a new sound event).
    pub fn show_in_library(&mut self, path: &str) {
        self.library_state.folder = path.rsplit_once('/').map_or("", |(f, _)| f).to_owned();
        self.library_state.search.clear();
        self.library_state.selected = Some(path.to_owned());
        self.sound_shown = Some((path.to_owned(), self.document.selection.clone()));
    }

    /// The project's sprite sheets as the app has read them (Phase 9, D2).
    pub fn set_sprite_sheets(&mut self, mut sheets: crate::sprite_sheets::SpriteSheets) {
        // The clips each sheet and clips file offers, worked out once here
        // for drawing, Properties and the running game.
        sheets.work_out_clips();
        self.viewport.sheets = std::sync::Arc::new(sheets);
        self.animator_read_again();
        self.tileset_read_again();
    }

    /// The project's sprite sheets.
    pub fn sprite_sheets(&self) -> &crate::sprite_sheets::SpriteSheets {
        &self.viewport.sheets
    }

    /// Makes an egui picture of each sprite sheet's atlas that lacks one
    /// (or whose sheet changed), and lets go of those whose file is gone.
    pub(crate) fn refresh_sheet_pictures(&mut self, ctx: &egui::Context) {
        let sheets = std::sync::Arc::clone(&self.viewport.sheets);
        self.sheet_pictures
            .retain(|path, _| sheets.at(path).is_some());
        self.sheet_pictures_from
            .retain(|path, _| sheets.at(path).is_some());
        for (path, state) in &sheets.by_path {
            let crate::sprite_sheets::SheetState::Read(sheet) = state else {
                continue;
            };
            let from = std::sync::Arc::as_ptr(sheet) as usize;
            if self.sheet_pictures_from.get(path) == Some(&from) {
                continue;
            }
            let [w, h] = sheet.atlas_size.map(|v| v as usize);
            let image = egui::ColorImage::from_rgba_unmultiplied([w, h], &sheet.atlas);
            let texture = ctx.load_texture(
                format!("sheet {path}"),
                image,
                egui::TextureOptions::NEAREST,
            );
            self.sheet_pictures.insert(path.clone(), texture);
            self.sheet_pictures_from.insert(path.clone(), from);
        }
    }

    /// The sound events as the app has read them, by path inside the
    /// assets (`/` between folders).
    pub fn set_sound_events(
        &mut self,
        events: std::collections::HashMap<String, crate::sound_view::EventState>,
    ) {
        self.sound_events = events;
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
        let bus_stale = self.mixer.shown.as_ref().is_some_and(|(at, then)| {
            *then != self.document.selection || *at >= self.mixer.mixer.buses.len()
        });
        if bus_stale {
            self.mixer.shown = None;
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
        if self.viewport.sheets.get(asset).is_some() {
            self.place_sprite(asset, Some((at, parent)));
            return;
        }
        if self.viewport.sheets.tile_map(asset).is_some() {
            self.place_tile_map(asset, Some((at, parent)));
            return;
        }
        self.place_targets.insert(asset, (at, parent));
        self.outbox.push(WorkbenchAction::PlaceAsset(asset));
    }

    /// Places a sprite of sheet `sheet` (Phase 9, D2): at `target` (a point
    /// in the world, under an entity if it was dropped on one), or where
    /// the camera looks; on a whole pixel, standing on the sheet's
    /// "pivot" or "feet" slice if it has one, as drawn (not flipped).
    /// Selected, as one undo step.
    pub(crate) fn place_sprite(
        &mut self,
        sheet: AssetId,
        target: Option<(Vec3, Option<EntityId>)>,
    ) -> Option<EntityId> {
        let info = self.viewport.sheets.get(sheet)?.clone();
        let name = self
            .viewport
            .sheets
            .paths
            .get(&sheet)
            .map_or_else(|| "Sprite".to_owned(), |p| crate::sprite_sheets::name_of(p));
        let (at, parent) = match target {
            Some((at, parent)) => {
                let area = self.document.area();
                let parent = parent.filter(|&p| area.contains(p));
                let local = match parent {
                    Some(p) => area.world_matrix(p).inverse().transform_point3(at),
                    None => at,
                };
                (local, parent)
            }
            None => (self.viewport.camera.focus(), None),
        };
        if !at.is_finite() {
            return None;
        }
        let px = self.pixels.pixel_size();
        let on_pixel = |v: f32| (v / px).round() * px;
        let pivot = ["pivot", "feet"]
            .into_iter()
            .find(|n| info.slice(n).is_some())
            .unwrap_or_default();
        let mut data = area::EntityData::new(&name)
            .with(&area::builtin::Transform {
                position: Vec3::new(on_pixel(at.x), on_pixel(at.y), at.z),
                ..area::builtin::Transform::default()
            })
            .with(&area::builtin::Sprite {
                sheet: Some(reflect::AssetRef {
                    asset: sheet,
                    part: None,
                }),
                pivot: pivot.to_owned(),
                ..area::builtin::Sprite::default()
            });
        data.parent = parent;
        Some(self.spawn_selected(data))
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
