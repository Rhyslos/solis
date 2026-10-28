//! Animation (Phase 9, D3): clips of frames, played by time.
//!
//! - [`clip`]: a [`Clip`] is frames in order, each shown for some
//!   milliseconds and perhaps carrying events, played forward, backward or
//!   ping-pong, over and over or a number of times, at a speed. Where a
//!   clip is at a time ([`Clip::at`]) and which events it passes between
//!   two times ([`Clip::events_between`]) are plain sums, the same on every
//!   PC, so the running game can step clips once per tick and replays and
//!   saves agree.
//! - [`frames_file`]: a sprite's clips file (`.solframes`): which sheet it
//!   plays, and what Solis adds to the sheet's Aseprite tags (speed,
//!   events, the clip played first) plus clips made in Solis from any of
//!   the sheet's frames.
//! - [`clip_set`]: a sheet's tags and its clips file put together into the
//!   clips a sprite can play ([`ClipSet`]).
//!
//! The state machine (the next milestone) plays these clips; 3D
//! skeletons' clips (Phase 16) will use the same time and event rules.

pub mod clip;
pub mod clip_set;
pub mod frames_file;

pub use clip::{Clip, ClipFrame, Direction, Moment};
pub use clip_set::ClipSet;
pub use frames_file::{FramesFile, OwnClip, TagChange};
