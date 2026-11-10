//! The editor's interface: the top bar, the dockable panels, the viewport's
//! controls and the theme ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! Layer 2 (editor). Games never need this crate. It never touches the
//! window, the GPU or files: it returns actions for the app to carry out,
//! and draws the viewport through a [`ViewportRenderer`] the app provides.
//! See this crate's README for its purpose and limits.

pub mod animator_editor;
mod animator_view;
mod area_panel;
pub mod area_view;
mod atlas_editor;
mod atlas_view;
pub mod camera;
mod canvas_tools;
mod canvas_view;
mod commands;
mod commands_many;
pub mod console;
mod console_commands;
mod day_colours;
pub mod document;
mod elements_view;
pub mod export_dialog;
pub mod frames_editor;
mod frames_view;
mod game_code;
mod game_panel;
mod game_screens;
pub mod game_sound;
#[cfg(test)]
mod game_sound_tests;
pub mod gizmo;
mod graph_canvas;
pub mod hints;
pub mod icons;
mod input_window;
mod inspector;
pub mod launcher;
pub mod layout;
pub mod library;
pub mod library_drawer;
pub mod look;
mod map_tab;
mod menu_items;
pub mod mixer_view;
pub mod models;
mod palette;
pub mod panel;
mod panels;
pub mod picking;
pub mod play;
pub mod play_changes;
mod play_input;
pub mod play_only;
pub mod problems;
pub mod profiler;
mod properties;
mod save_game;
pub mod screen;
mod screen_doc;
mod screen_pictures;
mod screen_props;
mod screen_ties;
mod search;
mod settings_window;
mod sheet_view;
mod sound_event_view;
pub mod sound_view;
pub mod splash;
pub mod sprite_parallax;
pub mod sprite_sheets;
pub mod status_bar;
#[cfg(test)]
mod test_support;
pub mod text_view;
pub mod theme;
pub mod tile_paint;
pub mod tile_view;
mod tileset_editor;
mod tileset_view;
mod top_bar;
pub mod transforming;
pub mod updates;
pub mod viewport;
mod viewport_tools;
pub mod workbench;
mod workbench_animator;
mod workbench_atlas;
mod workbench_frames;
mod workbench_library;
mod workbench_paint;
mod workbench_screens;
mod workbench_tiles;
mod workbench_tileset;
mod workbench_tools;
pub mod workspace;

pub use area_view::{AreaView, ViewLight, ViewModel};
pub use camera::{Camera, FlyInput, ViewKind};
pub use console::{LogLevel, LogLine};
pub use document::{Document, Selection};
pub use egui_dock::DockState;
pub use game_sound::{GameSound, Hearer, SoundMode};
pub use launcher::KnownProject;
pub use library::{LibraryItem, LibraryKind, ProjectInfo};
pub use look::{Look, LookKind};
pub use mixer_view::MixerState;
pub use models::{FileLight, FileLightKind, ModelInfo};
pub use panel::Panel;
pub use play::PlayState;
pub use save_game::{FORMAT as SAVE_FORMAT, save_read_back, save_reads};
pub use sound_view::{EventState, Preview, SoundInfo, SoundState};
pub use splash::{SplashView, UpdateAnswer, UpdateOffer};
pub use transforming::Snapping;
pub use updates::{ReleaseNotes, UpdateNotice};
pub use viewport::{
    DrawReport, GAME_IMAGE, GpuSplit, MAX_IMAGES, NoRenderer, Overlays, PixelLayer, PixelView,
    Tool, VIEWPORT_IMAGE, VIEWS, ViewBolt, ViewLayout, ViewSettings, Viewport, ViewportRenderer,
    ViewportView, Views,
};
pub use viewport_tools::{Outline, PickAnswer};
pub use workbench::{AppInfo, Workbench, WorkbenchAction};
pub use workbench_library::plain_relative;

#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod kit_tests;
#[cfg(test)]
mod kit_topdown_tests;
#[cfg(test)]
mod open_externally_tests;
#[cfg(test)]
mod scale_levels_tests;
