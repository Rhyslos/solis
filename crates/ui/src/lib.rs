//! Game UI (Sungraph UI; Phase 12, brought forward by decision 0070): a
//! screen (the HUD, a pause menu) is a tree of elements, each pinned to a
//! corner, an edge or the middle of its parent, laid out for whatever
//! screen the game is on and turned into shapes ready to draw.
//!
//! - [`screen`]: elements, anchors and a screen's scale rule
//!   ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)).
//! - [`mod@layout`]: the screen laid out on a real screen size, into [`Shape`]s.
//! - [`presets`]: the screen sizes the UI editor previews.
//! - [`text`]: fonts, and letters as distance images in an atlas
//!   ([decision 0084](../../../docs/decisions/0084-game-ui-text.md)).
//!
//! No GPU and no window: `render` draws the shapes, the editor shows them.

pub mod layout;
pub mod presets;
pub mod sample;
pub mod screen;
pub mod text;

pub use layout::{Shape, ShapeKind, Stretch, layout, layout_at, layout_with};
pub use screen::{Anchor, Colour, Edge, Element, ElementKind, Fill, ScaleRule, Screen};
pub use text::Fonts;
