//! Sound for Solis ([decision 0056](../../../docs/decisions/0056-sound-with-kira.md)).
//!
//! Kira does the playing and mixing; this crate wraps it, as `physics`
//! wraps Rapier, so nothing else in Solis sees Kira's types (decision
//! 0052's walls). It reads WAV and Ogg Vorbis files itself, through
//! `hound` and `lewton`, within limits ([`read`]), so every dependency
//! stays permissive and every file is read as if it were hostile.
//!
//! What it holds: reading sound files ([`read`]) and an overview of each
//! for drawing its waveform, the sound device and the editor's preview,
//! sound events ([`event`]), the game's sounds around a listener
//! ([`game`], [`voices`]), buses and ducking ([`mixer`]), and reverb zones
//! ([`reverb`]).
//!
//! Sound never changes the game: nothing here reads or writes game
//! state.

mod buses;
mod device;
pub mod event;
#[cfg(test)]
mod event_tests;
pub mod game;
pub mod mixer;
#[cfg(test)]
mod mixer_tests;
mod overview;
pub mod read;
pub mod reverb;
#[cfg(test)]
mod seek_tests;
#[cfg(test)]
mod tests;
pub mod voices;
mod vorbis_check;
mod vorbis_stream;

pub use device::SoundDevice;
pub use event::{Pick, SoundEvent};
pub use mixer::Mixer;
pub use overview::Overview;
pub use read::{Clip, ClipInfo, Format, ReadError, Samples, measure_file, read_bytes, read_file};
