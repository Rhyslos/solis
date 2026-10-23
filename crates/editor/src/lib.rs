//! The editor's interface: the top bar, the dockable panels, the viewport's
//! controls and the theme ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! Layer 2 (editor). Games never need this crate. It never touches the
//! window, the GPU or files: it returns actions for the app to carry out,
//! and draws the viewport through a [`ViewportRenderer`] the app provides.
//! See this crate's README for its purpose and limits.

pub mod camera;
pub mod console;
pub mod document;
pub mod icons;
mod inspector;
pub mod layout;
pub mod look;
pub mod models;
pub mod panel;
mod panels;
mod properties;
mod scene_panel;
pub mod scene_view;
pub mod theme;
mod top_bar;
pub mod viewport;
pub mod workbench;

pub use camera::{Camera, FlyInput};
pub use console::{LogLevel, LogLine};
pub use document::{Document, Selection};
pub use egui_dock::DockState;
pub use layout::Preset;
pub use look::{Look, LookKind};
pub use models::{FileLight, FileLightKind, ModelInfo};
pub use panel::Panel;
pub use scene_view::{SceneView, ViewLight};
pub use viewport::{NoRenderer, Overlays, Tool, Viewport, ViewportRenderer, ViewportView};
pub use workbench::{AppInfo, Workbench, WorkbenchAction};
