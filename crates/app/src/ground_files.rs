//! The ground's files (Tools 1b, G1; decision 0116), read and written on
//! a thread of their own, never on the frame (rule 10): its settings file
//! (`farm.ground.lst`) and the region files in the folder beside it.
//!
//! Saving writes each file beside the old one and swaps it in (rule 11),
//! the region files first, so a settings file never names a ground whose
//! voxels aren't there. A new ground's files are version 1, so there is
//! no older file to keep a copy of yet; a later version's save goes
//! through `app::upgrades::save` like the other formats.

use editor::LoadedGround;
use reflect::AssetId;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// What a thread finished.
enum Finished {
    Loaded(AssetId, Result<LoadedGround, String>),
    Saved(String, Result<(), String>),
}

/// Reads and writes grounds on threads, and hands the answers over.
pub(crate) struct GroundFiles {
    sender: Sender<Finished>,
    receiver: Receiver<Finished>,
}

impl Default for GroundFiles {
    fn default() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }
}

impl GroundFiles {
    /// Reads the ground at `path` (inside `assets`) on a thread.
    pub(crate) fn load(&self, assets: PathBuf, asset: AssetId, path: String) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let read = read(&assets, &path).map(|(settings, ground)| LoadedGround {
                asset,
                path: path.clone(),
                settings,
                ground: Arc::new(ground),
            });
            let _ = sender.send(Finished::Loaded(asset, read));
        });
    }

    /// Writes a new ground's files on a thread.
    pub(crate) fn save(
        &self,
        assets: PathBuf,
        path: String,
        text: String,
        regions: Vec<(String, Vec<u8>)>,
    ) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let saved = write(&assets, &path, &text, &regions);
            let _ = sender.send(Finished::Saved(path, saved));
        });
    }

    /// Hands what finished to the editor.
    pub(crate) fn poll(&self, workbench: &mut editor::Workbench) {
        while let Ok(finished) = self.receiver.try_recv() {
            match finished {
                Finished::Loaded(asset, read) => workbench.ground_loaded(asset, read),
                Finished::Saved(path, Err(why)) => workbench.log(
                    editor::LogLevel::Error,
                    format!("Couldn't save the ground {path}: {why}."),
                ),
                Finished::Saved(..) => {}
            }
        }
    }
}

/// The ground whose settings file is `path` inside `assets`.
pub(crate) fn read(assets: &Path, path: &str) -> Result<(ground::Settings, ground::Ground), String> {
    let file = assets.join(path);
    let text = read_limited(&file, ground::settings::MAX_BYTES)?;
    let text = String::from_utf8(text).map_err(|_| "its settings aren't text".to_string())?;
    let settings = ground::settings::read(&text.replace("\r\n", "\n"))
        .map_err(|e| e.to_string())?
        .settings;
    let folder = assets.join(ground::settings::folder_for(path));
    let mut ground = ground::Ground::new(ground::Fixed::empty(settings.extent()));
    let [across, along] = settings.regions();
    for y in 0..along {
        for x in 0..across {
            let name = ground::region::file_name([x, y]);
            let bytes = read_limited(&folder.join(&name), ground::region::MAX_BYTES)
                .map_err(|why| format!("its region {name}: {why}"))?;
            ground::region::load_into(&mut ground, [x, y], &bytes)
                .map_err(|why| format!("its region {name}: {why}"))?;
        }
    }
    Ok((settings, ground))
}

/// A file's bytes, refused past `limit` before it's read.
fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = reflect::ordinary_file::open(path).map_err(|e| format!("{e:?}"))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("it's bigger than the format allows".into());
    }
    Ok(bytes)
}

/// Writes a ground's files: each region file, then the settings file,
/// each beside the old one and swapped in.
pub(crate) fn write(
    assets: &Path,
    path: &str,
    text: &str,
    regions: &[(String, Vec<u8>)],
) -> Result<(), String> {
    let folder = assets.join(ground::settings::folder_for(path));
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    for (name, bytes) in regions {
        if ground::region::region_of_name(name).is_none() {
            return Err(format!("{name} isn't a region file's name"));
        }
        swap_in(&folder.join(name), bytes)?;
    }
    swap_in(&assets.join(path), text.as_bytes())
}

/// Writes `bytes` beside `path` and swaps them in (rule 11).
fn swap_in(path: &Path, bytes: &[u8]) -> Result<(), String> {
    asset_db::write_atomically(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ground_saved_reads_back_whole() {
        let folder = std::env::temp_dir().join(format!("ground-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("maps")).unwrap();
        let settings = ground::Settings {
            size: [512, 256],
            height: 64,
            ..ground::Settings::default()
        };
        let made = ground::Start::Gentle {
            height: 20.0,
            seed: 2,
        }
        .ground(settings.extent());
        let regions: Vec<(String, Vec<u8>)> = [[0, 0], [1, 0]]
            .into_iter()
            .map(|r| (ground::region::file_name(r), ground::region::write(&made, r)))
            .collect();
        let path = "maps/hills.ground.lst";
        write(&folder, path, &ground::settings::write(&settings), &regions).unwrap();
        assert!(folder.join("maps/hills.ground/1_0.region").is_file());
        let (read_settings, read_ground) = read(&folder, path).unwrap();
        assert_eq!(read_settings, settings);
        assert_eq!(read_ground, made);
        // A region file gone: a mistake that says which, never a crash.
        std::fs::remove_file(folder.join("maps/hills.ground/0_0.region")).unwrap();
        assert!(read(&folder, path).unwrap_err().contains("0_0.region"));
        assert!(write(&folder, path, "x", &[("../evil".into(), Vec::new())]).is_err());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// The asset database's rule for a ground's parts folder and the
    /// ground crate's are the same rule.
    #[test]
    fn the_parts_folder_is_named_the_same_way_everywhere() {
        assert_eq!(asset_db::GROUND_PARTS_ENDING, ground::settings::FOLDER_ENDING);
        assert_eq!(
            asset_db::ground_parts_of(Path::new("maps/farm.ground.lst")),
            Some(PathBuf::from(ground::settings::folder_for("maps/farm.ground.lst")))
        );
    }
}
