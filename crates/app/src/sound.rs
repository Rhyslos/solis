//! Opening the sound card, on a worker, and trying again when there's
//! none (Phase 7, S1).
//!
//! Opening the card can take a moment, so it never holds up a frame. Until
//! it's open, and whenever there's no card at all, the editor uses a
//! silent device, so everything keeps working with nothing heard. The
//! Console says once that there's no sound, and again when the card opens
//! after all (a headset plugged in, say).

use editor::LogLevel;
use sound::SoundDevice;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

/// How long to wait before trying the sound card again.
pub const RETRY: Duration = Duration::from_secs(30);

/// What opens a sound device: the real card, or a stand-in in tests.
pub type Opener = fn() -> Result<SoundDevice, String>;

/// The app's sound device, and the work of opening it.
pub struct Sound {
    /// The device in use: silent until the card opens.
    device: SoundDevice,
    /// What opens the device.
    opener: Opener,
    /// The worker opening it, while it works.
    opening: Option<Receiver<Result<SoundDevice, String>>>,
    /// When to try again, after a failure.
    retry_at: Option<Instant>,
    /// How long to wait between tries.
    retry: Duration,
    /// Whether the Console has been told there's no sound.
    told: bool,
}

impl std::fmt::Debug for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sound")
            .field("device", &self.device)
            .field("opening", &self.opening.is_some())
            .finish_non_exhaustive()
    }
}

impl Sound {
    /// Starts opening the sound card.
    pub fn open() -> Self {
        Self::with_opener(SoundDevice::open, RETRY)
    }

    /// Starts opening a device with `opener`, trying again every `retry`
    /// after a failure.
    pub fn with_opener(opener: Opener, retry: Duration) -> Self {
        let mut sound = Self {
            device: SoundDevice::silent(),
            opener,
            opening: None,
            retry_at: None,
            retry,
            told: false,
        };
        sound.start();
        sound
    }

    fn start(&mut self) {
        let (sender, receiver) = channel();
        let opener = self.opener;
        std::thread::spawn(move || {
            let _ = sender.send(opener());
        });
        self.opening = Some(receiver);
        self.retry_at = None;
    }

    /// Checks on the worker, and tries again when it's time. Returns a
    /// note for the Console, when there's news.
    pub fn poll(&mut self, now: Instant) -> Option<(LogLevel, String)> {
        if let Some(receiver) = &self.opening {
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => {
                    Err("opening the sound card stopped unexpectedly".to_owned())
                }
            };
            self.opening = None;
            return match result {
                Ok(device) => {
                    self.device = device;
                    let told = std::mem::replace(&mut self.told, false);
                    told.then(|| (LogLevel::Info, "Sound is on.".to_owned()))
                }
                Err(why) => {
                    self.retry_at = Some(now + self.retry);
                    if std::mem::replace(&mut self.told, true) {
                        None
                    } else {
                        Some((
                            LogLevel::Warning,
                            format!(
                                "No sound: {why}. Everything else works; \
                                 sound will start when a sound card is found."
                            ),
                        ))
                    }
                }
            };
        }
        if self.retry_at.is_some_and(|at| now >= at) {
            self.start();
        }
        None
    }

    /// The device to play on (silent until the card is open).
    pub fn device(&mut self) -> &mut SoundDevice {
        &mut self.device
    }

    /// Whether a device is being opened right now.
    pub fn is_opening(&self) -> bool {
        self.opening.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fails() -> Result<SoundDevice, String> {
        Err("there's no sound card".to_owned())
    }

    fn works() -> Result<SoundDevice, String> {
        Ok(SoundDevice::for_tests())
    }

    /// Polls until the worker answers, and returns its note.
    fn answer(sound: &mut Sound, now: Instant) -> Option<(LogLevel, String)> {
        let give_up = Instant::now() + Duration::from_secs(10);
        while sound.is_opening() && Instant::now() < give_up {
            if let Some(note) = sound.poll(now) {
                return Some(note);
            }
            std::thread::yield_now();
        }
        None
    }

    #[test]
    fn a_working_card_opens_without_a_note() {
        let mut sound = Sound::with_opener(works, RETRY);
        assert!(answer(&mut sound, Instant::now()).is_none());
        assert!(!sound.is_opening());
        assert!(!sound.device().is_silent());
    }

    #[test]
    fn no_card_says_so_once_and_tries_again() {
        let mut sound = Sound::with_opener(fails, Duration::from_secs(5));
        let start = Instant::now();
        let note = answer(&mut sound, start).expect("a note about no sound");
        assert_eq!(note.0, LogLevel::Warning);
        assert!(note.1.contains("there's no sound card"), "{}", note.1);
        assert!(sound.device().is_silent());

        // Not time yet: no new try.
        assert!(sound.poll(start + Duration::from_secs(1)).is_none());
        assert!(!sound.is_opening());

        // Time: it tries again, and stays quiet about failing again.
        let later = start + Duration::from_secs(6);
        assert!(sound.poll(later).is_none());
        assert!(sound.is_opening());
        assert!(answer(&mut sound, later).is_none());
        assert!(!sound.is_opening());
        assert!(sound.device().is_silent());
    }

    #[test]
    fn a_card_found_later_is_announced() {
        let mut sound = Sound::with_opener(fails, Duration::ZERO);
        let now = Instant::now();
        assert!(answer(&mut sound, now).is_some());
        sound.opener = works;
        assert!(sound.poll(now).is_none()); // starts the next try
        let note = answer(&mut sound, now).expect("a note that sound is on");
        assert_eq!(note, (LogLevel::Info, "Sound is on.".to_owned()));
        assert!(!sound.device().is_silent());
    }
}
