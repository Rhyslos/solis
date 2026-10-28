//! Sound for Solis ([decision 0056](../../../docs/decisions/0056-sound-with-kira.md)).
//!
//! Kira does the playing and mixing; this crate wraps it, as `physics`
//! wraps Rapier, so nothing else in Solis sees Kira's types (decision
//! 0052's walls). It reads WAV and Ogg Vorbis files itself, through
//! `hound` and `lewton`, within limits ([`read`]), so every dependency
//! stays permissive and every file is read as if it were hostile.
//!
//! Phase 7, S1: reading sound files, an overview of each for drawing its
//! waveform, the sound device, and the editor's preview. Sound events,
//! game code's sounds, buses and reverb come in later milestones.
//!
//! Sound never changes the game: nothing here reads or writes game
//! state.

mod device;
pub mod event;
#[cfg(test)]
mod event_tests;
pub mod game;
mod overview;
pub mod read;
#[cfg(test)]
mod tests;
mod vorbis_check;

pub use device::SoundDevice;
pub use event::{Pick, SoundEvent};
pub use overview::Overview;
pub use read::{Clip, ClipInfo, Format, ReadError, Samples, measure_file, read_bytes, read_file};
