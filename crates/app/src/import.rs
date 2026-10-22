//! Importing models without stalling the window: each file is read on a
//! thread of its own, and the window picks up the finished models between
//! frames (CONTRIBUTING.md, rule 10: the frame loop never waits on files).

use editor::ModelId;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

/// A finished import, good or not.
pub struct Finished {
    /// The number the model was given when the import started.
    pub id: ModelId,
    /// The file it came from.
    pub path: PathBuf,
    /// The model, or why it couldn't be read.
    pub result: Result<assets::Model, assets::ImportError>,
    /// How long reading it took, in seconds.
    pub seconds: f32,
}

/// Starts imports and collects them when they finish.
pub struct Importer {
    sender: Sender<Finished>,
    receiver: Receiver<Finished>,
    next_id: u64,
    limits: assets::Limits,
    running: usize,
    /// The Open window, while it's showing: its choice arrives here.
    dialog: Option<Receiver<Option<PathBuf>>>,
}

impl Importer {
    /// An importer whose textures are kept within `max_texture_side`
    /// pixels, and whose meshes are split to fit buffers of
    /// `max_buffer_bytes` (what the graphics card can hold).
    pub fn new(max_texture_side: u32, max_buffer_bytes: u64) -> Self {
        let (sender, receiver) = channel();
        let defaults = assets::Limits::default();
        let per_buffer = usize::try_from(max_buffer_bytes / 32).unwrap_or(usize::MAX);
        Self {
            sender,
            receiver,
            next_id: 1,
            limits: assets::Limits {
                max_texture_side: max_texture_side.clamp(1, defaults.max_texture_side),
                max_mesh_vertices: per_buffer.clamp(1024, defaults.max_vertices),
                ..defaults
            },
            running: 0,
            dialog: None,
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

    /// Starts importing `path` on a thread of its own.
    pub fn start(&mut self, path: &Path) -> ModelId {
        let limits = self.limits;
        let path = path.to_path_buf();
        let reading = path.clone();
        self.spawn(path, move || assets::load(&reading, &limits))
    }

    /// Starts reading the built-in sample scene, as if from a file named
    /// `name`.
    pub fn start_sample(&mut self, name: &str) -> ModelId {
        let limits = self.limits;
        let name = name.to_string();
        let path = PathBuf::from(format!("{name}.glb"));
        self.spawn(path, move || {
            assets::load_bytes(&assets::sample::showcase_glb(), None, &name, &limits)
        })
    }

    /// Runs `read` on a thread of its own and reports what it gave. If it
    /// crashes, that's reported as an error too, so the import never
    /// hangs.
    fn spawn(
        &mut self,
        path: PathBuf,
        read: impl FnOnce() -> Result<assets::Model, assets::ImportError> + Send + 'static,
    ) -> ModelId {
        let id = ModelId(self.next_id);
        self.next_id += 1;
        self.running += 1;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(read))
                .unwrap_or_else(|_| {
                    Err(assets::ImportError::Damaged(
                        "reading it failed unexpectedly".into(),
                    ))
                });
            // The window may have closed meanwhile; then nobody is waiting.
            let _ = sender.send(Finished {
                id,
                path,
                result,
                seconds: started.elapsed().as_secs_f32(),
            });
        });
        id
    }

    /// The imports that have finished since last asked.
    pub fn finished(&mut self) -> Vec<Finished> {
        let done: Vec<Finished> = self.receiver.try_iter().collect();
        self.running = self.running.saturating_sub(done.len());
        done
    }

    /// Whether any import is still being read.
    pub fn busy(&self) -> bool {
        self.running > 0
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
        let id = importer.start(&path);
        assert!(importer.busy());
        let done = wait(&mut importer);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].id, id);
        assert_eq!(done[0].result.as_ref().unwrap().name, "sample");
        assert!(!importer.busy());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_sample_scene_reads_in_the_background() {
        let mut importer = Importer::new(4096, 256 << 20);
        let id = importer.start_sample("Sample scene");
        let done = wait(&mut importer);
        assert_eq!(done[0].id, id);
        let model = done[0].result.as_ref().unwrap();
        assert_eq!(model.name, "Sample scene");
        assert!(model.meshes.len() > 10);
    }

    #[test]
    fn a_missing_file_reports_back_instead_of_crashing() {
        let mut importer = Importer::new(4096, 256 << 20);
        importer.start(Path::new("/no/such/model.glb"));
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
        importer.spawn(PathBuf::from("crash.glb"), || panic!("a bug in the reader"));
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
    fn every_import_gets_its_own_number() {
        let mut importer = Importer::new(4096, 256 << 20);
        let a = importer.start(Path::new("/no/a.glb"));
        let b = importer.start(Path::new("/no/b.glb"));
        assert_ne!(a, b);
        let mut done = Vec::new();
        while done.len() < 2 {
            done.extend(wait(&mut importer));
        }
    }
}
