//! The editor's interface: the top bar, the dockable panels, the viewport's
//! controls and the theme ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! Layer 2 (editor). Games never need this crate. It never touches the
//! window, the GPU or files: it returns actions for the app to carry out,
//! and draws the viewport through a [`ViewportRenderer`] the app provides.
//! See this crate's README for its purpose and limits.

mod area_panel;
pub mod area_view;
pub mod camera;
mod commands;
mod commands_many;
pub mod console;
mod console_commands;
pub mod document;
mod game_code;
mod game_panel;
pub mod gizmo;
pub mod icons;
mod input_window;
mod inspector;
pub mod layout;
pub mod library;
pub mod look;
mod menu_items;
pub mod models;
pub mod panel;
mod panels;
pub mod picking;
pub mod play;
mod play_input;
pub mod problems;
pub mod profiler;
mod properties;
mod save_game;
pub mod screen;
#[cfg(test)]
mod test_support;
pub mod theme;
mod top_bar;
pub mod transforming;
pub mod viewport;
mod viewport_tools;
pub mod workbench;
mod workbench_library;
mod workbench_tools;

pub use area_view::{AreaView, ViewLight, ViewModel};
pub use camera::{Camera, FlyInput, ViewKind};
pub use console::{LogLevel, LogLine};
pub use document::{Document, Selection};
pub use egui_dock::DockState;
pub use layout::Preset;
pub use library::{LibraryItem, LibraryKind, ProjectInfo};
pub use look::{Look, LookKind};
pub use models::{FileLight, FileLightKind, ModelInfo};
pub use panel::Panel;
pub use play::PlayState;
pub use transforming::Snapping;
pub use viewport::{
    DrawReport, GAME_IMAGE, GpuSplit, MAX_IMAGES, NoRenderer, Overlays, Tool, VIEWPORT_IMAGE,
    VIEWS, ViewBolt, ViewLayout, ViewSettings, Viewport, ViewportRenderer, ViewportView, Views,
};
pub use viewport_tools::{Outline, PickAnswer};
pub use workbench::{AppInfo, Workbench, WorkbenchAction};

#[cfg(test)]
mod kit_tests;
#[cfg(test)]
mod scale_levels_tests;
