//! The project's sound files in the editor (Phase 7, S1): each measured on
//! a worker (its length, channels and waveform) for the Library and
//! Properties, measured again when it changes, and listened to through
//! the sound device.
//!
//! Listening plays the file straight from disk ([`sound::Clip::streamed`]),
//! so nothing big is held for a sound that's only listed, and the sound
//! starts at once whatever its length.

use asset_db::{FileKind, Jobs, Listing};
use editor::{Preview, SoundInfo, SoundState};
use reflect::{AssetId, Id};
use sound::{ClipInfo, SoundDevice};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// The most sounds measured at once.
const AT_ONCE: usize = 2;
/// Columns kept of each waveform: enough for Properties' width.
const COLUMNS: usize = 512;

/// A file's size and time of last change: it's measured again when they
/// change (sounds aren't assets with IDs, so nothing else says they did).
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// A job number for `path` (the pool tells jobs apart by asset ID).
fn job_id(path: &Path) -> AssetId {
    let mut hasher = DefaultHasher::new();
    ("sound", path).hash(&mut hasher);
    AssetId(Id::from_raw(hasher.finish() | 1).unwrap_or_else(Id::random))
}

/// What measuring gave: what the editor shows, and what playing needs.
type Measured = Result<(SoundInfo, ClipInfo), String>;

/// Every sound file measured, those still to measure, and the one being
/// listened to.
pub struct Sounds {
    /// By path inside the assets (`/` between folders).
    states: HashMap<String, SoundState>,
    /// What playing each one needs.
    clips: HashMap<String, ClipInfo>,
    queued: VecDeque<(String, PathBuf)>,
    /// Files measured or failed, with how they were then.
    tried: HashMap<String, Stamp>,
    /// Goes up when another project opens: results for the last one are
    /// thrown away.
    generation: u64,
    jobs: Jobs<(String, u64, Measured)>,
    /// The sound being listened to, by path inside the assets.
    playing: Option<String>,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            clips: HashMap::new(),
            queued: VecDeque::new(),
            tried: HashMap::new(),
            generation: 0,
            jobs: Jobs::new(1),
            playing: None,
        }
    }
}

impl std::fmt::Debug for Sounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sounds")
            .field("measured", &self.states.len())
            .field("queued", &self.queued.len())
            .field("playing", &self.playing)
            .finish_non_exhaustive()
    }
}

impl Sounds {
    /// Whether sounds are still to be measured or being measured.
    pub fn waiting(&self) -> bool {
        !self.queued.is_empty() || self.jobs.running() > 0
    }

    /// Every sound file's state, for the Library and Properties.
    pub fn states(&self) -> HashMap<String, SoundState> {
        self.states.clone()
    }

    /// Queues every sound file in `listing` not measured yet, or changed
    /// since; forgets those no longer there.
    pub fn want(&mut self, listing: &Listing, assets: &Path) {
        let mut present = Vec::new();
        for file in &listing.files {
            if file.kind != FileKind::Sound {
                continue;
            }
            let path = crate::project::slashed(&file.path);
            let full = assets.join(&file.path);
            let now = stamp(&full);
            present.push(path.clone());
            if self.tried.get(&path) == Some(&now) {
                continue;
            }
            self.tried.insert(path.clone(), now);
            self.states.insert(path.clone(), SoundState::Measuring);
            self.clips.remove(&path);
            self.queued.retain(|(p, _)| *p != path);
            self.queued.push_back((path, full));
        }
        self.states.retain(|path, _| present.contains(path));
        self.clips.retain(|path, _| present.contains(path));
        self.tried.retain(|path, _| present.contains(path));
    }

    /// Forgets every sound (another project was opened), and stops
    /// listening.
    pub fn clear(&mut self, device: &mut SoundDevice) {
        self.stop(device);
        self.states.clear();
        self.clips.clear();
        self.tried.clear();
        self.queued.clear();
        self.generation += 1;
    }

    /// Starts the next measurements and takes in finished ones. Returns
    /// whether anything new was measured (so the Library should be told).
    pub fn poll(&mut self) -> bool {
        while self.jobs.running() < AT_ONCE {
            let Some((path, full)) = self.queued.pop_front() else {
                break;
            };
            let generation = self.generation;
            self.jobs.start(job_id(&full), move || {
                let measured = std::panic::catch_unwind(|| measure(&full))
                    .unwrap_or_else(|_| Err("reading it failed unexpectedly".to_owned()));
                (path, generation, measured)
            });
        }
        let mut new = false;
        for (_, (path, generation, measured)) in self.jobs.finished() {
            // For a project no longer open, or a file gone meanwhile.
            if generation != self.generation || !self.states.contains_key(&path) {
                continue;
            }
            let state = match measured {
                Ok((info, clip)) => {
                    self.clips.insert(path.clone(), clip);
                    SoundState::Read(info)
                }
                Err(why) => SoundState::Failed(why),
            };
            self.states.insert(path, state);
            new = true;
        }
        new
    }

    /// Listens to the sound at `path` (inside `assets`). Says why not, in
    /// plain words, when it can't.
    pub fn play(
        &mut self,
        path: &Path,
        assets: &Path,
        device: &mut SoundDevice,
    ) -> Result<(), String> {
        let key = crate::project::slashed(path);
        let Some(info) = self.clips.get(&key).copied() else {
            return Err(match self.states.get(&key) {
                Some(SoundState::Failed(why)) => format!("Solis couldn't read it: {why}"),
                Some(_) => "it's still being read; try again in a moment".to_owned(),
                None => "it isn't a sound file Solis knows".to_owned(),
            });
        };
        if device.is_silent() {
            return Err("there's no sound card to play it on".to_owned());
        }
        device.preview(&sound::Clip::streamed(&assets.join(path), info))?;
        self.playing = Some(key);
        Ok(())
    }

    /// Stops listening.
    pub fn stop(&mut self, device: &mut SoundDevice) {
        device.stop_preview();
        self.playing = None;
    }

    /// The sound being listened to and how far it has got, or `None` once
    /// it has finished.
    pub fn preview(&mut self, device: &SoundDevice) -> Option<Preview> {
        let path = self.playing.as_ref()?;
        match device.preview_position() {
            Some(seconds) => Some(Preview {
                path: path.clone(),
                seconds,
            }),
            None => {
                self.playing = None;
                None
            }
        }
    }
}

/// Measures one sound file (on a worker).
fn measure(path: &Path) -> Measured {
    let (info, overview) = sound::measure_file(path).map_err(|e| e.to_string())?;
    Ok((
        SoundInfo {
            seconds: info.seconds(),
            channels: info.channels,
            sample_rate: info.sample_rate,
            format: info.format.name(),
            streamed: info.seconds() > sound::read::MAX_IN_MEMORY_SECONDS as f64,
            waveform: overview.columns(COLUMNS),
        },
        info,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-second 16-bit mono WAV at 8 kHz, rising and falling.
    fn wav() -> Vec<u8> {
        let rate = 8_000_u32;
        let samples: Vec<i16> = (0..rate).map(|i| ((i % 80) as i16 - 40) * 500).collect();
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend(b"RIFF");
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16_u32.to_le_bytes());
        out.extend(1_u16.to_le_bytes());
        out.extend(1_u16.to_le_bytes());
        out.extend(rate.to_le_bytes());
        out.extend((rate * 2).to_le_bytes());
        out.extend(2_u16.to_le_bytes());
        out.extend(16_u16.to_le_bytes());
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    fn listing(paths: &[&str]) -> Listing {
        Listing {
            folders: Vec::new(),
            files: paths
                .iter()
                .map(|p| asset_db::FileEntry {
                    path: p.into(),
                    kind: if p.ends_with(".wav") {
                        FileKind::Sound
                    } else {
                        FileKind::Other
                    },
                    id: None,
                })
                .collect(),
        }
    }

    fn settle(s: &mut Sounds) {
        for _ in 0..500 {
            s.poll();
            if !s.waiting() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("measuring never finished");
    }

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("solis-sounds-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sfx")).unwrap();
        dir
    }

    #[test]
    fn sounds_are_measured_once_and_again_when_they_change() {
        let dir = folder("measure");
        std::fs::write(dir.join("sfx/step.wav"), wav()).unwrap();
        std::fs::write(dir.join("bad.wav"), b"not a sound").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hello").unwrap();
        let files = listing(&["sfx/step.wav", "bad.wav", "notes.txt"]);
        let mut s = Sounds::default();
        s.want(&files, &dir);
        assert_eq!(s.states().len(), 2, "only sound files");
        assert_eq!(s.states()["sfx/step.wav"], SoundState::Measuring);
        settle(&mut s);
        let SoundState::Read(info) = &s.states()["sfx/step.wav"] else {
            panic!("not read: {:?}", s.states()["sfx/step.wav"]);
        };
        assert!((info.seconds - 1.0).abs() < 1e-9);
        assert_eq!(
            (info.channels, info.sample_rate, info.format),
            (1, 8_000, "WAV")
        );
        assert!(!info.streamed);
        assert_eq!(info.waveform.len(), COLUMNS);
        assert!(info.waveform.iter().any(|(lo, hi)| *lo < -0.5 && *hi > 0.5));
        assert!(matches!(s.states()["bad.wav"], SoundState::Failed(_)));

        s.want(&files, &dir);
        assert!(!s.waiting(), "unchanged: not measured again");
        let mut longer = wav();
        longer.extend([0; 2]); // a different size
        std::fs::write(dir.join("sfx/step.wav"), longer).unwrap();
        s.want(&files, &dir);
        assert!(s.waiting(), "changed: measured again");
        settle(&mut s);

        s.want(&listing(&["bad.wav"]), &dir);
        assert!(!s.states().contains_key("sfx/step.wav"), "gone");
    }

    #[test]
    fn listening_plays_from_the_file_and_says_why_not() {
        let dir = folder("play");
        std::fs::write(dir.join("sfx/step.wav"), wav()).unwrap();
        std::fs::write(dir.join("bad.wav"), b"not a sound").unwrap();
        let mut s = Sounds::default();
        s.want(&listing(&["sfx/step.wav", "bad.wav"]), &dir);
        let mut device = SoundDevice::for_tests();
        let early = s.play(Path::new("sfx/step.wav"), &dir, &mut device);
        assert!(early.unwrap_err().contains("still being read"));
        settle(&mut s);

        s.play(Path::new("sfx/step.wav"), &dir, &mut device)
            .unwrap();
        let preview = s.preview(&device).expect("playing");
        assert_eq!(preview.path, "sfx/step.wav");
        s.stop(&mut device);
        assert!(s.preview(&device).is_none());

        let bad = s.play(Path::new("bad.wav"), &dir, &mut device).unwrap_err();
        assert!(bad.contains("couldn't read it"), "{bad}");
        let unknown = s.play(Path::new("x.wav"), &dir, &mut device).unwrap_err();
        assert!(unknown.contains("isn't a sound file"), "{unknown}");
        let mut silent = SoundDevice::silent();
        let none = s
            .play(Path::new("sfx/step.wav"), &dir, &mut silent)
            .unwrap_err();
        assert!(none.contains("no sound card"), "{none}");

        // Another project: nothing of this one is kept, or plays.
        s.play(Path::new("sfx/step.wav"), &dir, &mut device)
            .unwrap();
        s.clear(&mut device);
        assert!(s.states().is_empty());
        assert!(s.preview(&device).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
