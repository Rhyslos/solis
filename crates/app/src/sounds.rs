//! The project's sound files and sound events in the editor (Phase 7, S1
//! and S2): each sound measured on a worker (its length, channels and
//! waveform) and each event read on a worker, for the Library and
//! Properties; again when one changes; and listened to through the sound
//! device.
//!
//! Listening plays the file straight from disk ([`sound::Clip::streamed`]),
//! so nothing big is held for a sound that's only listed, and the sound
//! starts at once whatever its length. Listening to an event picks a take
//! and varies it as a game would ([`sound::event::pick`]), a new pick each
//! press.

use asset_db::{FileKind, Jobs, Listing};
use editor::{EventState, Preview, SoundInfo, SoundState};
use reflect::{AssetId, Id};
use sound::{Clip, ClipInfo, SoundDevice};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// The most files read at once.
const AT_ONCE: usize = 2;
/// Columns kept of each waveform: enough for Properties' width.
const COLUMNS: usize = 512;
/// Sound files up to this size are decoded into memory, ready for the
/// game to play at once; bigger ones are streamed from their file.
const IN_MEMORY_FILE_BYTES: u64 = 32 << 20;
/// The most decoded sound kept in memory in all (decision 0056's limits);
/// past it, sounds are streamed.
pub const MAX_IN_MEMORY_BYTES: usize = 512 << 20;

/// A file's size and time of last change: it's read again when they
/// change.
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

/// What a sound file's measuring gave: what the editor shows, and what
/// playing needs.
type Measured = Result<(SoundInfo, Clip), String>;

/// What reading one file gave.
enum Read {
    /// A sound file, measured.
    Sound(Measured),
    /// A sound event, read.
    Event(EventState),
}

/// Every sound file measured and sound event read, those still to read,
/// and what's being listened to.
pub struct Sounds {
    /// Sound files by path inside the assets (`/` between folders).
    states: HashMap<String, SoundState>,
    /// What playing each sound file needs: its samples in memory, or
    /// where to stream it from.
    clips: HashMap<String, Clip>,
    /// Bytes of samples held in memory in all.
    in_memory: usize,
    /// Sound events by path inside the assets.
    events: HashMap<String, EventState>,
    /// Each sound file's path, by asset ID (events name takes by ID).
    paths: HashMap<AssetId, String>,
    queued: VecDeque<(String, PathBuf, FileKind)>,
    /// Files read or failed, with how they were then.
    tried: HashMap<String, Stamp>,
    /// Goes up when another project opens: results for the last one are
    /// thrown away.
    generation: u64,
    jobs: Jobs<(String, u64, Read)>,
    /// The assets folder, as of the last listing.
    assets: PathBuf,
    /// What's being listened to, by path inside the assets.
    playing: Option<String>,
    /// How long what's playing is, in seconds.
    length: f64,
    /// For an event: which take it picked and how.
    detail: Option<String>,
    /// How many times an event has been listened to (its next pick's
    /// tick), and the take each picked last.
    presses: u64,
    last_take: HashMap<String, AssetId>,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            states: HashMap::new(),
            clips: HashMap::new(),
            in_memory: 0,
            events: HashMap::new(),
            paths: HashMap::new(),
            queued: VecDeque::new(),
            tried: HashMap::new(),
            generation: 0,
            jobs: Jobs::new(1),
            assets: PathBuf::new(),
            playing: None,
            length: 0.0,
            detail: None,
            presses: 0,
            last_take: HashMap::new(),
        }
    }
}

impl std::fmt::Debug for Sounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sounds")
            .field("sounds", &self.states.len())
            .field("events", &self.events.len())
            .field("queued", &self.queued.len())
            .field("playing", &self.playing)
            .finish_non_exhaustive()
    }
}

impl Sounds {
    /// Whether files are still to be read or being read.
    pub fn waiting(&self) -> bool {
        !self.queued.is_empty() || self.jobs.running() > 0
    }

    /// Every sound file's state, for the Library and Properties.
    pub fn states(&self) -> HashMap<String, SoundState> {
        self.states.clone()
    }

    /// Every sound event's state, for the Library and Properties.
    pub fn events(&self) -> HashMap<String, EventState> {
        self.events.clone()
    }

    /// Queues every sound file and sound event in `listing` not read yet,
    /// or changed since; forgets those no longer there.
    pub fn want(&mut self, listing: &Listing, assets: &Path) {
        self.assets = assets.to_path_buf();
        let mut present = Vec::new();
        self.paths.clear();
        for file in &listing.files {
            if !matches!(file.kind, FileKind::Sound | FileKind::SoundEvent) {
                continue;
            }
            let path = crate::project::slashed(&file.path);
            if file.kind == FileKind::Sound
                && let Some(id) = file.id
            {
                self.paths.insert(id, path.clone());
            }
            let full = assets.join(&file.path);
            let now = stamp(&full);
            present.push(path.clone());
            if self.tried.get(&path) == Some(&now) {
                continue;
            }
            self.tried.insert(path.clone(), now);
            if file.kind == FileKind::Sound {
                self.states.insert(path.clone(), SoundState::Measuring);
                self.forget_clip(&path);
            }
            self.queued.retain(|(p, _, _)| *p != path);
            self.queued.push_back((path, full, file.kind));
        }
        self.states.retain(|path, _| present.contains(path));
        let gone: Vec<String> = self
            .clips
            .keys()
            .filter(|p| !present.contains(p))
            .cloned()
            .collect();
        for path in gone {
            self.forget_clip(&path);
        }
        self.events.retain(|path, _| present.contains(path));
        self.tried.retain(|path, _| present.contains(path));
    }

    /// Forgets every sound (another project was opened), and stops
    /// listening.
    pub fn clear(&mut self, device: &mut SoundDevice) {
        self.stop(device);
        self.states.clear();
        self.clips.clear();
        self.in_memory = 0;
        self.events.clear();
        self.paths.clear();
        self.tried.clear();
        self.queued.clear();
        self.last_take.clear();
        self.generation += 1;
    }

    /// Starts the next reads and takes in finished ones. Returns whether
    /// anything new was read (so the Library should be told).
    pub fn poll(&mut self) -> bool {
        while self.jobs.running() < AT_ONCE {
            let Some((path, full, kind)) = self.queued.pop_front() else {
                break;
            };
            let generation = self.generation;
            self.jobs.start(job_id(&full), move || {
                let read = std::panic::catch_unwind(|| {
                    if kind == FileKind::SoundEvent {
                        Read::Event(read_event(&full))
                    } else {
                        Read::Sound(measure(&full))
                    }
                })
                .unwrap_or_else(|_| Read::Sound(Err("reading it failed unexpectedly".to_owned())));
                (path, generation, read)
            });
        }
        let mut new = false;
        for (_, (path, generation, read)) in self.jobs.finished() {
            // For a project no longer open, or a file gone meanwhile.
            if generation != self.generation || !self.tried.contains_key(&path) {
                continue;
            }
            match read {
                Read::Sound(measured) => {
                    let state = match measured {
                        Ok((info, clip)) => {
                            self.keep_clip(&path, clip);
                            SoundState::Read(info)
                        }
                        Err(why) => SoundState::Failed(why),
                    };
                    self.states.insert(path, state);
                }
                Read::Event(state) => {
                    self.events.insert(path, state);
                }
            }
            new = true;
        }
        new
    }

    /// Takes in a sound event Solis just saved, so Properties shows it at
    /// once (the file is read again when the watcher sees it).
    pub fn saved_event(&mut self, path: &Path, event: sound::SoundEvent) {
        self.events.insert(
            crate::project::slashed(path),
            EventState::Read {
                event,
                notes: Vec::new(),
            },
        );
    }

    /// Listens to the sound file or sound event at `path` (inside
    /// `assets`). Says why not, in plain words, when it can't.
    pub fn play(
        &mut self,
        path: &Path,
        assets: &Path,
        device: &mut SoundDevice,
    ) -> Result<(), String> {
        let key = crate::project::slashed(path);
        if let Some(event) = self.events.get(&key) {
            let EventState::Read { event, .. } = event else {
                return Err("Solis couldn't read it (Properties says why)".to_owned());
            };
            return self.play_event(&key, &event.clone(), assets, device);
        }
        let info = self.clip(&key)?;
        if device.is_silent() {
            return Err("there's no sound card to play it on".to_owned());
        }
        device.preview(&sound::Clip::streamed(&assets.join(path), info))?;
        self.playing = Some(key);
        self.length = info.seconds();
        self.detail = None;
        Ok(())
    }

    /// What playing the sound file at `key` needs, or why it can't play.
    fn clip(&self, key: &str) -> Result<ClipInfo, String> {
        self.clips
            .get(key)
            .map(|c| c.info)
            .ok_or_else(|| match self.states.get(key) {
                Some(SoundState::Failed(why)) => format!("Solis couldn't read it: {why}"),
                Some(_) => "it's still being read; try again in a moment".to_owned(),
                None => "it isn't a sound file Solis knows".to_owned(),
            })
    }

    /// One play of an event: a take picked and varied as a game would,
    /// a new pick each press, never the same take twice in a row.
    fn play_event(
        &mut self,
        key: &str,
        event: &sound::SoundEvent,
        assets: &Path,
        device: &mut SoundDevice,
    ) -> Result<(), String> {
        // Only takes that are readable sound files in the project.
        let playable = sound::SoundEvent {
            takes: event
                .takes
                .iter()
                .copied()
                .filter(|id| {
                    self.paths
                        .get(id)
                        .is_some_and(|p| self.clips.contains_key(p))
                })
                .collect(),
            ..event.clone()
        };
        if event.takes.is_empty() {
            return Err("it has no takes yet: drop sound files on it".to_owned());
        }
        if device.is_silent() {
            return Err("there's no sound card to play it on".to_owned());
        }
        self.presses += 1;
        let last = self.last_take.get(key).copied();
        let pick = sound::event::pick(&playable, last, 0, self.presses, 0)
            .ok_or("none of its takes is a sound file Solis can read")?;
        let take = self.paths[&pick.take].clone();
        let info = self.clip(&take)?;
        device.preview_varied(
            &sound::Clip::streamed(&assets.join(&take), info),
            pick.volume_db,
            pick.speed(),
        )?;
        self.last_take.insert(key.to_owned(), pick.take);
        self.playing = Some(key.to_owned());
        self.length = info.seconds();
        let name = take.rsplit('/').next().unwrap_or(&take);
        let name = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        self.detail = Some(format!(
            "{name}, {:+.1} st, {:+.1} dB",
            pick.pitch_st, pick.volume_db
        ));
        Ok(())
    }

    /// Stops listening.
    pub fn stop(&mut self, device: &mut SoundDevice) {
        device.stop_preview();
        self.playing = None;
    }

    /// What's being listened to and how far it has got, or `None` once it
    /// has finished.
    pub fn preview(&mut self, device: &SoundDevice) -> Option<Preview> {
        let path = self.playing.as_ref()?;
        match device.preview_position() {
            Some(seconds) => Some(Preview {
                path: path.clone(),
                seconds,
                length: self.length,
                detail: self.detail.clone(),
            }),
            None => {
                self.playing = None;
                None
            }
        }
    }
}

impl Sounds {
    /// Keeps a sound file's clip, in memory while there's room in
    /// [`MAX_IN_MEMORY_BYTES`], else to stream from its file.
    fn keep_clip(&mut self, path: &str, clip: Clip) {
        self.forget_clip(path);
        let bytes = memory_bytes(&clip);
        let clip = if bytes > 0 && self.in_memory + bytes > MAX_IN_MEMORY_BYTES {
            Clip::streamed(&self.assets.join(path), clip.info)
        } else {
            self.in_memory += bytes;
            clip
        };
        self.clips.insert(path.to_owned(), clip);
    }

    fn forget_clip(&mut self, path: &str) {
        if let Some(old) = self.clips.remove(path) {
            self.in_memory = self.in_memory.saturating_sub(memory_bytes(&old));
        }
    }

    /// The sound event called `name`: its path inside the assets, without
    /// `.solsound` (as game code names it, `"sfx/footstep"`), or with it.
    pub fn event_named(&self, name: &str) -> Option<(&str, &sound::SoundEvent)> {
        let key = if name.ends_with(".solsound") {
            name.to_owned()
        } else {
            format!("{name}.solsound")
        };
        let (key, state) = self.events.get_key_value(&key)?;
        match state {
            EventState::Read { event, .. } => Some((key.as_str(), event)),
            EventState::Failed(_) => None,
        }
    }

    /// A sound file's clip, by its asset ID: ready to play.
    pub fn clip_of(&self, take: AssetId) -> Option<&Clip> {
        self.clips.get(self.paths.get(&take)?)
    }

    /// Bytes of decoded sound held in memory now.
    pub fn in_memory(&self) -> usize {
        self.in_memory
    }
}

/// Bytes a clip's samples take in memory (none when streamed).
fn memory_bytes(clip: &Clip) -> usize {
    match &clip.samples {
        sound::Samples::Memory(frames) => std::mem::size_of_val(&**frames),
        sound::Samples::Streamed(_) => 0,
    }
}

/// Reads one sound event file (on a worker).
fn read_event(path: &Path) -> EventState {
    let bytes = match asset_db::read_limited(path, sound::event::MAX_BYTES as u64) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return EventState::Failed(format!(
                "it's bigger than {} KB, too big for a sound event",
                sound::event::MAX_BYTES >> 10
            ));
        }
        Err(e) => return EventState::Failed(format!("couldn't read it ({e})")),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return EventState::Failed("it isn't text".to_owned());
    };
    match sound::event::read(&text) {
        Ok(read) => EventState::Read {
            event: read.event,
            notes: read.notes,
        },
        Err(e) => EventState::Failed(e.to_string()),
    }
}

/// Measures one sound file (on a worker): a small one is decoded into
/// memory at the same time, ready for the game; a big one only measured,
/// to stream.
fn measure(path: &Path) -> Measured {
    let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= IN_MEMORY_FILE_BYTES);
    let (info, overview, clip) = if small {
        let clip = sound::read_file(path).map_err(|e| e.to_string())?;
        let overview = clip.overview.clone();
        (
            clip.info,
            overview,
            Clip {
                overview: sound::Overview::new(),
                ..clip
            },
        )
    } else {
        let (info, overview) = sound::measure_file(path).map_err(|e| e.to_string())?;
        (info, overview, Clip::streamed(path, info))
    };
    Ok((
        SoundInfo {
            seconds: info.seconds(),
            channels: info.channels,
            sample_rate: info.sample_rate,
            format: info.format.name(),
            streamed: info.seconds() > sound::read::MAX_IN_MEMORY_SECONDS as f64,
            waveform: overview.columns(COLUMNS),
        },
        clip,
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
                    } else if p.ends_with(".solsound") {
                        FileKind::SoundEvent
                    } else {
                        FileKind::Other
                    },
                    id: p.ends_with(".wav").then(|| sound_id(p)),
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

    /// A made-up asset ID for a sound file, from its name.
    fn sound_id(path: &str) -> AssetId {
        job_id(Path::new(path))
    }

    #[test]
    fn an_event_plays_a_different_take_each_press() {
        let dir = folder("event");
        std::fs::write(dir.join("sfx/a.wav"), wav()).unwrap();
        std::fs::write(dir.join("sfx/b.wav"), wav()).unwrap();
        let event = sound::SoundEvent {
            takes: vec![sound_id("sfx/a.wav"), sound_id("sfx/b.wav")],
            pitch_range_st: 2.0,
            ..sound::SoundEvent::default()
        };
        std::fs::write(dir.join("step.solsound"), sound::event::write(&event)).unwrap();
        std::fs::write(
            dir.join("bad.solsound"),
            "sound version 1;\nvolume = \"loud\";",
        )
        .unwrap();
        let files = listing(&["sfx/a.wav", "sfx/b.wav", "step.solsound", "bad.solsound"]);
        let mut s = Sounds::default();
        s.want(&files, &dir);
        settle(&mut s);
        assert!(
            matches!(&s.events()["step.solsound"], EventState::Read { event: e, .. } if *e == event)
        );
        assert!(
            matches!(&s.events()["bad.solsound"], EventState::Failed(why) if why.contains("line 2"))
        );

        let mut device = SoundDevice::for_tests();
        let mut takes = Vec::new();
        for _ in 0..6 {
            s.play(Path::new("step.solsound"), &dir, &mut device)
                .unwrap();
            let preview = s.preview(&device).expect("playing");
            assert_eq!(preview.path, "step.solsound");
            let detail = preview.detail.expect("which take");
            takes.push(detail.split(',').next().unwrap().to_owned());
        }
        assert!(
            takes.windows(2).all(|w| w[0] != w[1]),
            "never the same twice: {takes:?}"
        );

        let bad = s
            .play(Path::new("bad.solsound"), &dir, &mut device)
            .unwrap_err();
        assert!(bad.contains("couldn't read it"), "{bad}");
        // Saved by the editor: shown at once.
        s.saved_event(Path::new("bad.solsound"), sound::SoundEvent::default());
        let empty = s
            .play(Path::new("bad.solsound"), &dir, &mut device)
            .unwrap_err();
        assert!(empty.contains("no takes"), "{empty}");
        let missing = sound::SoundEvent {
            takes: vec![sound_id("gone.wav")],
            ..sound::SoundEvent::default()
        };
        s.saved_event(Path::new("bad.solsound"), missing);
        let none = s
            .play(Path::new("bad.solsound"), &dir, &mut device)
            .unwrap_err();
        assert!(none.contains("none of its takes"), "{none}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_huge_or_garbled_event_file_is_refused() {
        let dir = folder("event-bad");
        std::fs::write(
            dir.join("big.solsound"),
            vec![b' '; sound::event::MAX_BYTES + 1],
        )
        .unwrap();
        std::fs::write(dir.join("bin.solsound"), [0xff, 0xfe, 0x00]).unwrap();
        assert!(
            matches!(read_event(&dir.join("big.solsound")), EventState::Failed(w) if w.contains("KB"))
        );
        assert!(
            matches!(read_event(&dir.join("bin.solsound")), EventState::Failed(w) if w.contains("isn't text"))
        );
        assert!(matches!(
            read_event(&dir.join("none.solsound")),
            EventState::Failed(_)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
