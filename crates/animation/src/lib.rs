//! Animation (Phase 9, D3): clips of frames, played by time.
//!
//! - [`clip`]: a [`Clip`] is frames in order, each shown for some
//!   milliseconds and perhaps carrying events, played forward, backward or
//!   ping-pong, over and over or a number of times, at a speed. Where a
//!   clip is at a time ([`Clip::at`]) and which events it passes between
//!   two times ([`Clip::events_between`]) are plain sums, the same on every
//!   PC, so the running game can step clips once per tick and replays and
//!   saves agree.
//! - [`clips_file`]: a picture's clips file (`.clips.lst`, Lust): its
//!   looks (the first is the sheet it plays), what Solis adds to an
//!   Aseprite sheet's tags (speed, events), the clip played first, and
//!   clips made in Solis from the sheet's cells, with one, four or eight
//!   directions.
//! - [`frames_file`]: the clips file before version 2 (`.solframes`),
//!   still read and turned into a [`ClipsFile`].
//! - [`clip_set`]: a sheet's tags and its clips file put together into the
//!   clips a sprite can play ([`ClipSet`]).
//! - [`animator_file`]: an animator (`.solanim`, D3b): a state machine of
//!   parameters, states (each playing a clip, or a clip per facing
//!   direction) and transitions with conditions.
//! - [`animator`]: an animator made ready to run, and one entity's place
//!   in it, stepped once a tick.
//!
//! 3D skeletons' clips (3D 1 in ROADMAP.md) will use the same time and event rules
//! and the same state machine.

pub mod animator;
pub mod animator_file;
pub mod atlas;
pub mod clip;
pub mod clip_set;
pub mod clips_file;
pub mod frames_file;

pub use animator::{Animator, ParamValue, Run, SetError};
pub use animator_file::{
    AnimatorFile, Condition, Facing, Parameter, ParameterKind, Plays, State, Transition,
};
pub use clip::{Clip, ClipFrame, Direction, Moment};
pub use clip_set::ClipSet;
pub use clips_file::{ClipsFile, TagChange};
