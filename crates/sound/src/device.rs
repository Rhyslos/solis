//! The sound device: Kira's audio manager on the sound card (or a test
//! device, or none), and the editor's preview of sound files.
//!
//! Previews play on an **Editor** track of their own, never touched by
//! the game's buses (decision 0056), so muting the game's music never
//! silences a preview in the Library.
//!
//! Opening the sound card can take a moment, so the app opens it on a
//! worker ([`SoundDevice::open`] returns a value that can be sent between
//! threads). No sound card is not an error: [`SoundDevice::silent`] is a
//! device that plays nothing, and the app tries the card again later.

use crate::read::{Clip, ClipInfo, Decoder, ReadError, Samples};
use kira::backend::cpal::CpalBackend;
use kira::backend::mock::{MockBackend, MockBackendSettings};
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, Frame, PlaybackRate, Tween};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a stopped preview takes to fade out.
const STOP_FADE: Duration = Duration::from_millis(30);

/// Which audio manager the device has.
pub(crate) enum Manager {
    /// The sound card.
    Card(Box<AudioManager<CpalBackend>>),
    /// A test device, which plays to nowhere (CI has no sound card).
    Test(Box<AudioManager<MockBackend>>),
}

/// A preview that's playing.
pub(crate) enum Preview {
    /// From memory.
    Memory(StaticSoundHandle),
    /// Streamed from its file.
    Stream(StreamingSoundHandle<ReadError>),
}

impl Preview {
    pub(crate) fn state(&self) -> PlaybackState {
        match self {
            Self::Memory(h) => h.state(),
            Self::Stream(h) => h.state(),
        }
    }

    fn position(&self) -> f64 {
        match self {
            Self::Memory(h) => h.position(),
            Self::Stream(h) => h.position(),
        }
    }

    pub(crate) fn stop(&mut self) {
        self.stop_over(STOP_FADE);
    }

    /// Why it couldn't be played, once: a streamed sound's file is opened
    /// as it starts playing ([`LaterStream`]).
    pub(crate) fn pop_error(&mut self) -> Option<ReadError> {
        match self {
            Self::Memory(_) => None,
            Self::Stream(h) => h.pop_error(),
        }
    }

    /// Stops it with a fade this long.
    pub(crate) fn stop_over(&mut self, fade: Duration) {
        let fade = Tween {
            duration: fade,
            ..Tween::default()
        };
        match self {
            Self::Memory(h) => h.stop(fade),
            Self::Stream(h) => h.stop(fade),
        }
    }
}

/// The sound device, and what the editor is previewing on it.
pub struct SoundDevice {
    /// The audio manager, if there's a device.
    manager: Option<Manager>,
    /// The Editor track previews play on.
    editor: Option<TrackHandle>,
    /// The preview playing, if any.
    preview: Option<Preview>,
    /// The game's sounds: its tracks, the listener and what's playing.
    pub(crate) game: Option<crate::game::GameTracks>,
}

impl std::fmt::Debug for SoundDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let device = match self.manager {
            Some(Manager::Card(_)) => "sound card",
            Some(Manager::Test(_)) => "test device",
            None => "silent",
        };
        f.debug_struct("SoundDevice")
            .field("device", &device)
            .field("previewing", &self.previewing())
            .finish()
    }
}

impl SoundDevice {
    /// Opens the sound card. Fails (with a plain reason) when there's
    /// none, or it can't be opened; the caller then uses
    /// [`SoundDevice::silent`] and may try again later.
    pub fn open() -> Result<Self, String> {
        let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("couldn't open the sound card ({e})"))?;
        Ok(Self::with(Manager::Card(Box::new(manager))))
    }

    /// A test device: everything works, and nothing is heard.
    pub fn for_tests() -> Self {
        let settings = AudioManagerSettings {
            backend_settings: MockBackendSettings {
                sample_rate: 48_000,
            },
            ..AudioManagerSettings::default()
        };
        // The test device can't fail to open; if it ever did, a silent
        // device is the safe stand-in.
        AudioManager::<MockBackend>::new(settings).map_or_else(
            |()| Self::silent(),
            |m| Self::with(Manager::Test(Box::new(m))),
        )
    }

    /// A device that plays nothing: when there's no sound card.
    pub fn silent() -> Self {
        Self {
            manager: None,
            editor: None,
            preview: None,
            game: None,
        }
    }

    fn with(mut manager: Manager) -> Self {
        let editor = match &mut manager {
            Manager::Card(m) => m.add_sub_track(TrackBuilder::new()).ok(),
            Manager::Test(m) => m.add_sub_track(TrackBuilder::new()).ok(),
        };
        let game = crate::game::GameTracks::new(&mut manager);
        Self {
            manager: Some(manager),
            editor,
            preview: None,
            game,
        }
    }

    /// Whether it plays nothing (no sound card).
    pub fn is_silent(&self) -> bool {
        self.manager.is_none()
    }

    /// Previews a clip on the Editor track, stopping any preview first.
    /// Silent devices do nothing. A long sound's file is opened as it
    /// starts playing, away from the frame: one that can't be is said by
    /// [`SoundDevice::preview_failure`].
    pub fn preview(&mut self, clip: &Clip) -> Result<(), String> {
        self.preview_varied(clip, 0.0, 1.0)
    }

    /// Previews a clip as one play of a sound event would sound: `volume_db`
    /// louder or quieter, and played `speed` times as fast (which raises
    /// or lowers its pitch with it).
    pub fn preview_varied(
        &mut self,
        clip: &Clip,
        volume_db: f32,
        speed: f64,
    ) -> Result<(), String> {
        self.stop_preview();
        let volume = Decibels(if volume_db.is_finite() {
            volume_db
        } else {
            0.0
        });
        let rate = PlaybackRate(if speed.is_finite() && speed > 0.0 {
            speed
        } else {
            1.0
        });
        let Some(track) = self.editor.as_mut() else {
            return Ok(());
        };
        let preview = match &clip.samples {
            Samples::Memory(frames) => {
                let data = memory_sound(clip.info, frames.clone())
                    .volume(volume)
                    .playback_rate(rate);
                Preview::Memory(
                    track
                        .play(data)
                        .map_err(|e| format!("couldn't play it ({e})"))?,
                )
            }
            Samples::Streamed(path) => {
                let data = StreamingSoundData::from_decoder(LaterStream::new(path, clip.info))
                    .volume(volume)
                    .playback_rate(rate);
                Preview::Stream(
                    track
                        .play(data)
                        .map_err(|e| format!("couldn't play it ({e})"))?,
                )
            }
        };
        self.preview = Some(preview);
        Ok(())
    }

    /// Stops the preview, with a short fade so it doesn't click.
    pub fn stop_preview(&mut self) {
        if let Some(mut preview) = self.preview.take() {
            preview.stop();
        }
    }

    /// Why the preview couldn't be played, if it couldn't, once: a long
    /// sound's file is opened as it starts playing, away from the frame.
    pub fn preview_failure(&mut self) -> Option<String> {
        self.preview
            .as_mut()
            .and_then(Preview::pop_error)
            .map(|why| why.to_string())
    }

    /// Whether a preview is playing.
    pub fn previewing(&self) -> bool {
        self.preview
            .as_ref()
            .is_some_and(|p| p.state() != PlaybackState::Stopped)
    }

    /// How far the preview has played, in seconds, while it plays.
    pub fn preview_position(&self) -> Option<f64> {
        self.preview
            .as_ref()
            .filter(|p| p.state() != PlaybackState::Stopped)
            .map(Preview::position)
    }
}

/// A clip in memory, as Kira's sound data.
pub(crate) fn memory_sound(info: ClipInfo, frames: std::sync::Arc<[Frame]>) -> StaticSoundData {
    StaticSoundData {
        sample_rate: info.sample_rate,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    }
}

/// A long sound, read from its file as it plays.
pub(crate) struct FileStream {
    decoder: Decoder<BufReader<File>>,
    info: ClipInfo,
    /// The frame it has reached.
    at: u64,
}

impl FileStream {
    /// Opens the file measured as `info`, as carefully as when it was
    /// measured (`read::open`). A file changed since (another rate or
    /// channels) is refused: Kira would play it as measured.
    pub(crate) fn open(path: &Path, info: ClipInfo) -> Result<Self, ReadError> {
        let decoder = Decoder::new(crate::read::open(path)?)?;
        let now = decoder.info_with(info.frames);
        if (now.sample_rate, now.channels) != (info.sample_rate, info.channels) {
            return Err(ReadError::Damaged(
                "it changed since it was read: look at it in the Library again".into(),
            ));
        }
        Ok(Self {
            decoder,
            info,
            at: 0,
        })
    }
}

impl kira::sound::streaming::Decoder for FileStream {
    type Error = ReadError;

    fn sample_rate(&self) -> u32 {
        self.info.sample_rate
    }

    fn num_frames(&self) -> usize {
        usize::try_from(self.info.frames).unwrap_or(usize::MAX)
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        let mut out = Vec::new();
        self.decoder.next_chunk(&mut out)?;
        // Shorter than when it was measured (the file changed): silence up
        // to the measured length, as Kira keeps asking until then and
        // would never stop (the Phase 7 review).
        if out.is_empty() && self.at < self.info.frames {
            let left = (self.info.frames - self.at).min(4096);
            out.resize(usize::try_from(left).unwrap_or(4096), Frame::ZERO);
        }
        self.at += out.len() as u64;
        Ok(out)
    }

    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        let at = self.decoder.seek(index as u64)?;
        self.at = at;
        Ok(usize::try_from(at).unwrap_or(index))
    }
}

/// A long sound, read from its file as it plays: opened when Kira first
/// asks it for frames, on Kira's own decoding thread, not where the sound
/// is started. Opening checks the file as carefully as when it was
/// measured, which for an Ogg file walks every page of it
/// ([`crate::vorbis_check`]): far too slow for the frame (the review,
/// BUG-35). A file that can't be opened says why once (Kira hands it to
/// the sound's handle, and stops the sound) and is silence after.
pub(crate) struct LaterStream {
    path: PathBuf,
    info: ClipInfo,
    state: Opening,
}

/// Where a [`LaterStream`] is.
enum Opening {
    /// Not opened yet: it will start at this frame.
    Waiting(u64),
    Open(FileStream),
    /// It couldn't be opened: silence, from this frame to the end.
    Failed(u64),
}

impl LaterStream {
    /// The file at `path`, measured as `info`, to open when it plays.
    pub(crate) fn new(path: &Path, info: ClipInfo) -> Self {
        Self {
            path: path.to_path_buf(),
            info,
            state: Opening::Waiting(0),
        }
    }

    /// Opens the file at the frame it was asked to start at.
    fn open(&self, from: u64) -> Result<FileStream, ReadError> {
        use kira::sound::streaming::Decoder as _;
        let mut stream = FileStream::open(&self.path, self.info)?;
        if from > 0 {
            stream.seek(usize::try_from(from).unwrap_or(usize::MAX))?;
        }
        Ok(stream)
    }
}

impl kira::sound::streaming::Decoder for LaterStream {
    type Error = ReadError;

    fn sample_rate(&self) -> u32 {
        self.info.sample_rate
    }

    fn num_frames(&self) -> usize {
        usize::try_from(self.info.frames).unwrap_or(usize::MAX)
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        if let Opening::Waiting(from) = self.state {
            match self.open(from) {
                Ok(stream) => self.state = Opening::Open(stream),
                Err(why) => {
                    self.state = Opening::Failed(from);
                    return Err(why);
                }
            }
        }
        match &mut self.state {
            Opening::Open(stream) => stream.decode(),
            // Up to the measured length, as Kira asks until then.
            Opening::Waiting(at) | Opening::Failed(at) => {
                let left = self.info.frames.saturating_sub(*at).min(4096);
                *at += left;
                Ok(vec![Frame::ZERO; usize::try_from(left).unwrap_or(4096)])
            }
        }
    }

    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        match &mut self.state {
            Opening::Open(stream) => stream.seek(index),
            // Kira takes this as where the frames it's handed next start.
            Opening::Waiting(at) | Opening::Failed(at) => {
                *at = (index as u64).min(self.info.frames);
                Ok(usize::try_from(*at).unwrap_or(index))
            }
        }
    }
}

/// The device is opened on a worker and handed to the app, so it must be
/// able to move between threads.
const _: fn() = || {
    fn sendable<T: Send>() {}
    sendable::<SoundDevice>();
};
