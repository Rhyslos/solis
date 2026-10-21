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
}

impl Importer {
    /// An importer whose textures are kept within `max_texture_side`
    /// pixels (what the graphics card can hold).
    pub fn new(max_texture_side: u32) -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver,
            next_id: 1,
            limits: assets::Limits {
                max_texture_side: max_texture_side
                    .clamp(1, assets::Limits::default().max_texture_side),
                ..assets::Limits::default()
            },
            running: 0,
        }
    }

    /// Asks which model to import, with the system's Open window, and
    /// starts importing it. `None` if the window was closed without a
    /// choice.
    pub fn ask_and_start(&mut self) -> Option<(ModelId, PathBuf)> {
        let path = rfd::FileDialog::new()
            .set_title("Import model")
            .add_filter("glTF model", &assets::EXTENSIONS)
            .pick_file()?;
        let id = self.start(&path);
        Some((id, path))
    }

    /// Starts importing `path` on a thread of its own.
    pub fn start(&mut self, path: &Path) -> ModelId {
        let id = ModelId(self.next_id);
        self.next_id += 1;
        self.running += 1;
        let sender = self.sender.clone();
        let limits = self.limits;
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = assets::load(&path, &limits);
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
        let mut importer = Importer::new(4096);
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
    fn a_missing_file_reports_back_instead_of_crashing() {
        let mut importer = Importer::new(4096);
        importer.start(Path::new("/no/such/model.glb"));
        let done = wait(&mut importer);
        assert!(matches!(
            done[0].result,
            Err(assets::ImportError::Unreadable(_))
        ));
    }

    #[test]
    fn every_import_gets_its_own_number() {
        let mut importer = Importer::new(4096);
        let a = importer.start(Path::new("/no/a.glb"));
        let b = importer.start(Path::new("/no/b.glb"));
        assert_ne!(a, b);
        let mut done = Vec::new();
        while done.len() < 2 {
            done.extend(wait(&mut importer));
        }
    }
}
