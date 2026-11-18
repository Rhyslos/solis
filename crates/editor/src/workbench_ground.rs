//! The ground in the editor (Tools 1b, G1): asking the app for the
//! level's ground, meshing its chunks on workers ([`GroundChunks`]),
//! making a new one (New ground, its small dialog), and handing the
//! running game its colliders.
//!
//! [`GroundChunks`]: crate::ground_chunks::GroundChunks

use crate::console::LogLevel;
use crate::ground_chunks::{GroundMeshes, LoadedGround};
use crate::workbench::{Workbench, WorkbenchAction};
use ground::{Settings, Start};
use reflect::{AssetId, AssetRef};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

/// The sizes a new ground can be, either way, in metres (decision 0116:
/// 1 × 1 km is the target, 2 × 2 km is tested).
pub(crate) const SIZES: [u32; 2] = [1024, 2048];
/// The heights a new ground can be, in metres.
pub(crate) const HEIGHTS: [u32; 3] = [64, 128, 256];

/// The New ground dialog's choices.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NewGroundForm {
    /// The Library folder it's made in.
    pub(crate) folder: String,
    /// Its size either way, in metres.
    pub(crate) side: u32,
    /// Its height, in metres.
    pub(crate) height: u32,
    /// Gentle hills rather than flat.
    pub(crate) hills: bool,
    /// Where its surface is, in metres above its floor.
    pub(crate) surface: f32,
}

impl NewGroundForm {
    fn new(folder: &str) -> Self {
        Self {
            folder: folder.to_owned(),
            side: SIZES[0],
            height: ground::settings::STANDARD_HEIGHT,
            hills: true,
            surface: 32.0,
        }
    }
}

/// A ground being made on a worker, and where it goes once made.
pub(crate) struct Making {
    path: String,
    answer: Receiver<(Settings, Arc<ground::Ground>, Vec<(String, Vec<u8>)>)>,
    /// Made and saved, waiting for the asset database to give its file an
    /// ID before it's placed in the level.
    made: Option<(Settings, Arc<ground::Ground>)>,
}

/// The New ground dialog and a ground being made.
#[derive(Default)]
pub(crate) struct NewGround {
    pub(crate) form: Option<NewGroundForm>,
    pub(crate) making: Option<Making>,
}

impl Workbench {
    /// One frame's part: asks the app for the level's ground when it's
    /// another, hands its chunks to the workers, places a new one once
    /// its file has an ID, and gives the running game the colliders that
    /// arrived.
    pub(crate) fn ground_frame(&mut self) {
        self.finish_making_ground();
        let shown = crate::kinds::in_use().shows(crate::kinds::PartKind::ThreeD);
        let wanted = self.viewport.level.ground.filter(|_| shown);
        let loaded = self.open.ground.loaded().map(|l| l.asset);
        if wanted != loaded {
            match wanted {
                None => {
                    self.open.ground.asked = None;
                    self.open.ground.set(None);
                }
                Some(asset) if self.open.ground.asked != Some(asset) => {
                    if let Some(path) = self.library_path(asset) {
                        self.open.ground.asked = Some(asset);
                        self.open
                            .outbox
                            .push(WorkbenchAction::LoadGround { asset, path });
                    }
                }
                Some(_) => {}
            }
        }
        let camera = self.viewport.camera.viewpoint();
        self.open.ground.frame(camera);
        if let Some(playing) = &mut self.open.playing {
            let arrived = self.open.ground.take_for_play();
            if !arrived.is_empty() {
                playing.world.add_ground_shapes(arrived);
            }
        }
    }

    /// The Library path of the asset `asset`, if it's listed.
    fn library_path(&self, asset: AssetId) -> Option<String> {
        self.open
            .library
            .iter()
            .find(|i| i.id == Some(asset))
            .map(|i| i.path.clone())
    }

    /// The app read the ground `asset` asked for with
    /// [`WorkbenchAction::LoadGround`]: its settings and voxels, or why
    /// it couldn't. Its chunks are made from the next frame, nearest the
    /// camera first.
    pub fn ground_loaded(&mut self, asset: AssetId, read: Result<LoadedGround, String>) {
        if self.open.ground.asked != Some(asset) {
            return;
        }
        match read {
            Ok(loaded) => {
                for note in ground_notes(&loaded) {
                    self.log(LogLevel::Info, note);
                }
                self.open.ground.set(Some(loaded));
            }
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't open the ground: {why}."));
                self.open.ground.set(None);
            }
        }
    }

    /// The ground's chunks made since the app last asked, to upload.
    pub fn take_ground_meshes(&mut self) -> GroundMeshes {
        self.open.ground.take_fresh()
    }

    /// Whether the ground still has chunks on their way: the app keeps
    /// drawing frames until they've arrived.
    pub fn ground_busy(&self) -> bool {
        self.open.ground.left() > 0
    }

    /// How long the ground took to arrive whole, once it has (the scale
    /// check's "every chunk drawn").
    pub fn ground_arrived_in(&self) -> Option<std::time::Duration> {
        self.open.ground.timing.and_then(|(_, done)| done)
    }

    /// New ground (Home's Create, the Library's New): opens its dialog,
    /// unless the level has a ground already (one per level).
    pub(crate) fn ask_new_ground(&mut self, folder: &str) {
        if self.level_ground().is_some() {
            self.log(LogLevel::Warning, level::builtin::ONE_PER_LEVEL);
            return;
        }
        self.open.new_ground.form = Some(NewGroundForm::new(folder));
    }

    /// The entity holding the level's Ground, if it has one.
    pub(crate) fn level_ground(&self) -> Option<reflect::EntityId> {
        self.open
            .document
            .level()
            .with::<level::builtin::Ground>()
            .map(|(id, _)| id)
            .next()
    }

    /// Makes a ground from `form` on a worker: its voxels, its region
    /// files and its settings file, saved by the app.
    pub(crate) fn make_ground(&mut self, form: &NewGroundForm) {
        let path = crate::workbench_tiles::free_path(&form.folder, "Ground", "ground.lst", |p| {
            self.open.library.iter().any(|i| i.path == p)
        });
        let settings = Settings {
            size: [form.side; 2],
            height: form.height,
            ..Settings::default()
        };
        let surface = form.surface.clamp(1.0, form.height as f32 - 1.0);
        let start = if form.hills {
            Start::Gentle {
                height: surface,
                seed: 1,
            }
        } else {
            Start::Flat { height: surface }
        };
        let (send, answer) = channel();
        let made_settings = settings.clone();
        rayon::spawn(move || {
            let made = Arc::new(start.ground(made_settings.extent()));
            let [across, along] = made_settings.regions();
            let regions = (0..along)
                .flat_map(|y| (0..across).map(move |x| [x, y]))
                .map(|r| (ground::region::file_name(r), ground::region::write(&made, r)))
                .collect();
            let _ = send.send((made_settings, made, regions));
        });
        self.log(
            LogLevel::Info,
            format!("Making {path} ({} × {} m)…", settings.size[0], settings.size[1]),
        );
        self.open.new_ground.making = Some(Making {
            path,
            answer,
            made: None,
        });
    }

    /// A ground made on a worker is saved; once its file has an ID, it's
    /// placed in the level as one undo step.
    fn finish_making_ground(&mut self) {
        let Some(making) = &mut self.open.new_ground.making else {
            return;
        };
        if making.made.is_none()
            && let Ok((settings, made, regions)) = making.answer.try_recv()
        {
            let text = ground::settings::write(&settings);
            self.open.outbox.push(WorkbenchAction::SaveGround {
                path: making.path.clone(),
                text,
                regions,
            });
            making.made = Some((settings, made));
        }
        let path = making.path.clone();
        let Some(asset) = self
            .open
            .library
            .iter()
            .find(|i| i.path == path)
            .and_then(|i| i.id)
        else {
            return;
        };
        let Some(Making {
            made: Some((settings, made)),
            ..
        }) = self.open.new_ground.making.take()
        else {
            return;
        };
        self.open.ground.asked = Some(asset);
        self.open.ground.set(Some(LoadedGround {
            asset,
            path: path.clone(),
            settings,
            ground: made,
        }));
        if self.level_ground().is_some() {
            self.log(LogLevel::Warning, level::builtin::ONE_PER_LEVEL);
            return;
        }
        let data = level::EntityData::new("Ground").with(&level::builtin::Ground {
            ground: Some(AssetRef { asset, part: None }),
            solid: true,
        });
        self.spawn_selected(data);
        self.log(LogLevel::Info, format!("Made the ground {path}."));
    }

    /// The New ground dialog, while it's open.
    pub(crate) fn new_ground_dialog(&mut self, ctx: &egui::Context) {
        let Some(form) = &mut self.open.new_ground.form else {
            return;
        };
        use crate::widgets::dialog::{Answer, dialog};
        let answers = [Answer::right("Cancel"), Answer::main("Create")];
        let picked = dialog(
            ctx,
            "new ground",
            "New ground",
            |ui| crate::ground_dialog::form(ui, form),
            &answers,
        );
        match picked {
            Some(1) => {
                if let Some(form) = self.open.new_ground.form.take() {
                    self.make_ground(&form);
                }
            }
            Some(_) => self.open.new_ground.form = None,
            None => {}
        }
    }

    /// For a new play: gives the game the level's ground, and the
    /// colliders made so far.
    pub(crate) fn ground_for_play(&mut self, world: &mut world::World) {
        let Some(loaded) = self.open.ground.loaded().cloned() else {
            return;
        };
        let level = self.open.document.level();
        let Some((entity, g)) = level
            .with::<level::builtin::Ground>()
            .find(|(_, g)| g.ground.map(|r| r.asset) == Some(loaded.asset))
        else {
            return;
        };
        let (needed, ready) = self.open.ground.for_new_play();
        world.set_ground(
            Arc::clone(&loaded.ground),
            (entity, loaded.asset, g.solid),
            &needed,
            ready,
        );
    }
}

/// Notes about a ground just read, for the Console.
fn ground_notes(loaded: &LoadedGround) -> Vec<String> {
    if loaded.settings.recipe.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "{} has a recipe, which grounds can't use yet (the Open worlds phase): its voxels \
             are drawn as saved.",
            loaded.path
        )]
    }
}
