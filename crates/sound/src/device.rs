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
use kira::{AudioManager, AudioManagerSettings, Frame, Tween};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

/// How long a stopped preview takes to fade out.
const STOP_FADE: Duration = Duration::from_millis(30);

/// Which audio manager the device has.
enum Manager {
    /// The sound card.
    Card(Box<AudioManager<CpalBackend>>),
    /// A test device, which plays to nowhere (CI has no sound card).
    Test(Box<AudioManager<MockBackend>>),
}

/// A preview that's playing.
enum Preview {
    /// From memory.
    Memory(StaticSoundHandle),
    /// Streamed from its file.
    Stream(StreamingSoundHandle<ReadError>),
}

impl Preview {
    fn state(&self) -> PlaybackState {
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

    fn stop(&mut self) {
        let fade = Tween {
            duration: STOP_FADE,
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
        }
    }

    fn with(mut manager: Manager) -> Self {
        let editor = match &mut manager {
            Manager::Card(m) => m.add_sub_track(TrackBuilder::new()).ok(),
            Manager::Test(m) => m.add_sub_track(TrackBuilder::new()).ok(),
        };
        Self {
            manager: Some(manager),
            editor,
            preview: None,
        }
    }

    /// Whether it plays nothing (no sound card).
    pub fn is_silent(&self) -> bool {
        self.manager.is_none()
    }

    /// Previews a clip on the Editor track, stopping any preview first.
    /// Silent devices do nothing.
    pub fn preview(&mut self, clip: &Clip) -> Result<(), String> {
        self.stop_preview();
        let Some(track) = self.editor.as_mut() else {
            return Ok(());
        };
        let preview = match &clip.samples {
            Samples::Memory(frames) => {
                let data = memory_sound(clip.info, frames.clone());
                Preview::Memory(
                    track
                        .play(data)
                        .map_err(|e| format!("couldn't play it ({e})"))?,
                )
            }
            Samples::Streamed(path) => {
                let stream = FileStream::open(path, clip.info).map_err(|e| e.to_string())?;
                let data = StreamingSoundData::from_decoder(stream);
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
fn memory_sound(info: ClipInfo, frames: std::sync::Arc<[Frame]>) -> StaticSoundData {
    StaticSoundData {
        sample_rate: info.sample_rate,
        frames,
        settings: StaticSoundSettings::default(),
        slice: None,
    }
}

/// A long sound, read from its file as it plays.
struct FileStream {
    decoder: Decoder<BufReader<File>>,
    info: ClipInfo,
}

impl FileStream {
    fn open(path: &Path, info: ClipInfo) -> Result<Self, ReadError> {
        let decoder = Decoder::new(BufReader::new(File::open(path)?))?;
        Ok(Self { decoder, info })
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
        Ok(out)
    }

    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        let at = self.decoder.seek(index as u64)?;
        Ok(usize::try_from(at).unwrap_or(index))
    }
}

/// The device is opened on a worker and handed to the app, so it must be
/// able to move between threads.
const _: fn() = || {
    fn sendable<T: Send>() {}
    sendable::<SoundDevice>();
};
