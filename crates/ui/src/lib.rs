//! Game UI (Sungraph UI; Phase 12, brought forward by decision 0070): a
//! screen (the HUD, a pause menu) is a tree of elements, each pinned to a
//! corner, an edge or the middle of its parent, laid out for whatever
//! screen the game is on and turned into shapes ready to draw.
//!
//! - [`screen`]: elements, anchors and a screen's scale rule
//!   ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)).
//! - [`mod@layout`]: the screen laid out on a real screen size, into [`Shape`]s.
//! - [`pictures`]: the pictures screens show, packed into one atlas
//!   ([decision 0089](../../../docs/decisions/0089-atlas-crop-and-the-ui-editor.md)).
//! - [`presets`]: the screen sizes the UI editor previews.
//! - [`text`]: fonts, and letters as distance images in an atlas
//!   ([decision 0084](../../../docs/decisions/0084-game-ui-text.md)).
//! - [`view`]: a screen written in Lust, its view's tree laid out with
//!   Lust's rules ([decision 0087](../../../docs/decisions/0087-screens-through-the-engine.md)).
//!
//! No GPU and no window: `render` draws the shapes, the editor shows them.

pub mod layout;
pub mod pictures;
pub mod presets;
pub mod sample;
pub mod screen;
pub mod text;
pub mod view;

pub use layout::{Shape, ShapeKind, Stretch, UNCLIPPED, layout, layout_at, layout_with};
pub use pictures::Pictures;
pub use screen::{Anchor, Colour, Edge, Element, ElementKind, Fill, ScaleRule, Screen};
pub use text::Fonts;
