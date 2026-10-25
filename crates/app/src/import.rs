//! Importing models without stalling the window: files are read by a
//! small pool of worker threads (through the project's import cache when a
//! project is open), and the window picks up the finished models between
//! frames (CONTRIBUTING.md, rule 10: the frame loop never waits on files).
//! Also the system's Open windows, which run on threads of their own.

use asset_db::Jobs;
use reflect::AssetId;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Instant;

/// A finished import, good or not.
pub struct Finished {
    /// The asset the model is for.
    pub id: AssetId,
    /// The file it came from.
    pub path: PathBuf,
    /// The model, or why it couldn't be read.
    pub result: Result<assets::Model, assets::ImportError>,
    /// How long reading it took, in seconds.
    pub seconds: f32,
    /// Whether it came from the import cache.
    pub from_cache: bool,
}

/// What a folder is being chosen for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderFor {
    /// A new project.
    NewProject,
    /// An existing project to open.
    OpenProject,
    /// A folder to make the sample game in.
    SampleGame,
    /// A folder to make the benchmark in.
    Benchmark,
    /// A folder to make the playground in.
    Playground,
}

/// Starts imports and collects them when they finish.
pub struct Importer {
    jobs: Jobs<Finished>,
    limits: assets::Limits,
    /// The Open window, while it's showing: its choice arrives here.
    dialog: Option<Receiver<Option<PathBuf>>>,
    /// The folder window, while it's showing.
    folder_dialog: Option<(FolderFor, Receiver<Option<PathBuf>>)>,
}

impl Importer {
    /// An importer whose textures are kept within `max_texture_side`
    /// pixels, and whose meshes are split to fit buffers of
    /// `max_buffer_bytes` (what the graphics card can hold).
    pub fn new(max_texture_side: u32, max_buffer_bytes: u64) -> Self {
        let defaults = assets::Limits::default();
        let per_buffer = usize::try_from(max_buffer_bytes / 32).unwrap_or(usize::MAX);
        Self {
            jobs: Jobs::for_this_computer(),
            limits: assets::Limits {
                max_texture_side: max_texture_side.clamp(1, defaults.max_texture_side),
                max_mesh_vertices: per_buffer.clamp(1024, defaults.max_vertices),
                ..defaults
            },
            dialog: None,
            folder_dialog: None,
        }
    }

    /// Shows the system's folder window, on a thread of its own; the
    /// choice comes back from [`Self::chosen_folder`]. Does nothing if one
    /// is already open.
    pub fn ask_folder(&mut self, purpose: FolderFor) {
        if self.folder_dialog.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let title = match purpose {
                FolderFor::NewProject => "Choose a folder for the new project",
                FolderFor::OpenProject => "Open project",
                FolderFor::SampleGame => "Choose an empty folder for the sample game",
                FolderFor::Benchmark => "Choose an empty folder for the benchmark",
                FolderFor::Playground => "Choose an empty folder for the playground",
            };
            let _ = sender.send(rfd::FileDialog::new().set_title(title).pick_folder());
        });
        self.folder_dialog = Some((purpose, receiver));
    }

    /// The folder chosen in the folder window, and what for, once it's
    /// closed with a choice.
    pub fn chosen_folder(&mut self) -> Option<(FolderFor, PathBuf)> {
        let (purpose, receiver) = self.folder_dialog.as_ref()?;
        let purpose = *purpose;
        match receiver.try_recv() {
            Ok(choice) => {
                self.folder_dialog = None;
                choice.map(|path| (purpose, path))
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.folder_dialog = None;
                None
            }
        }
    }

    /// Shows the system's Open window to choose a model, on a thread of
    /// its own so the editor keeps drawing meanwhile. The choice comes back
    /// from [`Self::chosen`]. Does nothing if the window is already open.
    pub fn ask(&mut self) {
        if self.dialog.is_some() {
            return;
        }
        let (sender, receiver) = channel();
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("Import model")
                .add_filter("glTF model", &assets::EXTENSIONS)
                .pick_file();
            let _ = sender.send(path);
        });
        self.dialog = Some(receiver);
    }

    /// The file chosen in the Open window, once it's closed with a choice.
    pub fn chosen(&mut self) -> Option<PathBuf> {
        let receiver = self.dialog.as_ref()?;
        match receiver.try_recv() {
            Ok(choice) => {
                self.dialog = None;
                choice
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.dialog = None;
                None
            }
        }
    }

    /// Starts reading `path` as the asset `id`, through the import cache
    /// in `cache` if there is one. A newer read of the same asset makes an
    /// older one's result be dropped.
    pub fn start(&mut self, id: AssetId, path: &Path, cache: Option<PathBuf>) {
        let limits = self.limits;
        let path = path.to_path_buf();
        let reading = path.clone();
        self.spawn(id, path, move || {
            asset_db::cache::load(&reading, cache.as_deref(), &limits)
        });
    }

    /// Starts reading the built-in sample area, as if from a file named
    /// `name`, and returns the asset it's given.
    pub fn start_sample(&mut self, name: &str) -> AssetId {
        let limits = self.limits;
        let name = name.to_string();
        let path = PathBuf::from(format!("{name}.glb"));
        let id = AssetId::random();
        self.spawn(id, path, move || {
            assets::load_bytes(&assets::sample::showcase_glb(), None, &name, &limits)
                .map(|m| (m, false))
        });
        id
    }

    /// Runs `read` on a worker and reports what it gave. If it crashes,
    /// that's reported as an error too, so the import never hangs.
    fn spawn(
        &mut self,
        id: AssetId,
        path: PathBuf,
        read: impl FnOnce() -> Result<(assets::Model, bool), assets::ImportError> + Send + 'static,
    ) {
        self.jobs.start(id, move || {
            let started = Instant::now();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(read))
                .unwrap_or_else(|_| {
                    Err(assets::ImportError::Damaged(
                        "reading it failed unexpectedly".into(),
                    ))
                });
            let from_cache = result.as_ref().is_ok_and(|(_, hit)| *hit);
            Finished {
                id,
                path,
                result: result.map(|(model, _)| model),
                seconds: started.elapsed().as_secs_f32(),
                from_cache,
            }
        });
    }

    /// The imports that have finished since last asked.
    pub fn finished(&mut self) -> Vec<Finished> {
        self.jobs.finished().into_iter().map(|(_, f)| f).collect()
    }

    /// Whether any import is still being read.
    pub fn busy(&self) -> bool {
        self.jobs.running() > 0
    }

    /// Whether anything is on its way: an import being read, or an Open
    /// or folder window open (its answer is looked for each frame).
    pub fn waiting(&self) -> bool {
        self.busy() || self.dialog.is_some() || self.folder_dialog.is_some()
    }

    /// The limits models are read with.
    pub fn limits(&self) -> assets::Limits {
        self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn wait(importer: &mut Importer) -> Vec<Finished> {
        for _ in 0..500 {
            let done = importer.finished();
            if !done.is_empty() {
                return done;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Vec::new()
    }

    #[test]
    fn a_model_is_read_in_the_background() {
        let folder = std::env::temp_dir().join(format!("solis-import-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("sample.glb");
        std::fs::write(&path, assets::sample::sample_glb()).unwrap();
        let mut importer = Importer::new(4096, 256 << 20);
        let id = AssetId::random();
        importer.start(id, &path, None);
        assert!(importer.busy());
        let done = wait(&mut importer);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, id);
        assert_eq!(done[0].result.as_ref().unwrap().name, "sample");
        assert!(!importer.busy());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_sample_area_reads_in_the_background() {
        let mut importer = Importer::new(4096, 256 << 20);
        let id = importer.start_sample("Sample area");
        let done = wait(&mut importer);
        assert_eq!(done[0].id, id);
        let model = done[0].result.as_ref().unwrap();
        assert_eq!(model.name, "Sample area");
        assert!(model.meshes.len() > 10);
    }

    #[test]
    fn a_missing_file_reports_back_instead_of_crashing() {
        let mut importer = Importer::new(4096, 256 << 20);
        importer.start(AssetId::random(), Path::new("/no/such/model.glb"), None);
        let done = wait(&mut importer);
        assert!(matches!(
            done[0].result,
            Err(assets::ImportError::Unreadable(_))
        ));
    }

    #[test]
    fn a_crash_while_reading_reports_back_instead_of_hanging() {
        let mut importer = Importer::new(4096, 256 << 20);
        #[allow(clippy::panic)]
        importer.spawn(AssetId::random(), PathBuf::from("crash.glb"), || {
            panic!("a bug in the reader")
        });
        let done = wait(&mut importer);
        assert!(matches!(
            done[0].result,
            Err(assets::ImportError::Damaged(_))
        ));
        assert!(!importer.busy());
    }

    #[test]
    fn meshes_are_split_to_fit_the_graphics_cards_buffers() {
        let importer = Importer::new(4096, 64 << 20);
        assert_eq!(importer.limits().max_mesh_vertices, (64 << 20) / 32);
        // A card reporting a tiny or huge limit still gets sensible ones.
        assert_eq!(Importer::new(4096, 0).limits().max_mesh_vertices, 1024);
        assert_eq!(
            Importer::new(4096, u64::MAX).limits().max_mesh_vertices,
            assets::Limits::default().max_vertices
        );
    }

    #[test]
    fn a_project_import_goes_through_the_cache() {
        let folder =
            std::env::temp_dir().join(format!("solis-import-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("sample.glb");
        std::fs::write(&path, assets::sample::sample_glb()).unwrap();
        let cache = folder.join("cache");
        let mut importer = Importer::new(4096, 256 << 20);
        importer.start(AssetId::random(), &path, Some(cache.clone()));
        assert!(!wait(&mut importer)[0].from_cache);
        importer.start(AssetId::random(), &path, Some(cache));
        let again = wait(&mut importer);
        assert!(again[0].from_cache);
        assert!(again[0].result.is_ok());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
