//! The editor's interface: the frame (tab bar, ribbon, editor groups,
//! shelf and status bar, [decision 0099](../../../docs/decisions/0099-the-g-layout.md)),
//! the panels inside it, the viewport's controls and the theme.
//!
//! Layer 2 (editor). Games never need this crate. It never touches the
//! window, the GPU or files: it returns actions for the app to carry out,
//! and draws the viewport through a [`ViewportRenderer`] the app provides.
//! See this crate's README for its purpose and limits.

pub mod animator_editor;
mod animator_view;
mod assets_column;
mod atlas_editor;
mod atlas_view;
#[cfg(test)]
mod audio_screen_tests;
pub mod brushes;
mod brushing;
mod bus_view;
pub mod camera;
mod canvas_tools;
mod canvas_view;
mod commands;
mod commands_many;
pub mod console;
mod console_commands;
mod day_colours;
#[cfg(test)]
mod debug_screen_tests;
pub mod document;
mod document_diff;
mod elements_view;
pub mod export_dialog;
mod field_rows;
mod file_history;
pub mod file_pages;
mod fonts;
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
mod guide;
#[cfg(test)]
mod guide_keys_tests;
mod hinted_fields;
pub mod hints;
pub mod icons;
mod inspector;
pub mod interface_look;
mod key_owner;
mod kinds;
mod languages_page;
pub mod launcher;
mod level_panel;
pub mod level_view;
pub mod library;
pub mod locale;
mod look;
mod made_from;
pub mod make_prefab;
mod map_tab;
mod menu_items;
mod mixer_console;
pub mod mixer_view;
pub mod models;
mod name_follow;
mod name_picker;
mod own_files;
pub mod panel;
mod panels;
pub mod picking;
mod placement;
pub mod play;
pub mod play_changes;
mod play_input;
pub mod play_only;
pub mod prefab_details;
mod prefab_marks;
mod prefab_page;
pub mod problems;
pub mod profiler;
mod profiler_view;
mod project_state;
mod properties;
mod properties_cards;
mod properties_header;
mod save_game;
pub mod screen;
mod screen_doc;
mod screen_fonts;
mod screen_pictures;
mod screen_props;
mod screen_ties;
mod screen_typing;
mod search;
mod settled;
mod sheet_view;
pub mod shell;
mod sound_event_view;
mod sound_events_view;
pub mod sound_view;
pub mod splash;
pub mod sprite_parallax;
pub mod sprite_sheets;
mod standard;
mod state_dump;
pub mod state_list;
mod table_editor;
#[cfg(test)]
mod table_scale_tests;
#[cfg(test)]
mod table_tab_tests;
mod table_view;
pub mod tags;
#[cfg(test)]
mod test_support;
pub mod text_view;
pub mod theme;
pub mod tile_paint;
mod tile_properties;
pub mod tile_view;
mod tileset_editor;
mod tileset_view;
pub mod tokens;
mod toolbox;
pub mod transforming;
mod translations_tab;
pub mod unbuilt;
#[cfg(test)]
mod unbuilt_screens_tests;
pub mod updates;
pub mod viewport;
mod viewport_tools;
pub mod widgets;
pub mod workbench;
mod workbench_animate_tools;
mod workbench_animator;
mod workbench_atlas;
mod workbench_audio_tools;
mod workbench_brush_edit;
mod workbench_brush_parts;
mod workbench_brush_shaping;
mod workbench_brushes;
mod workbench_frames;
mod workbench_library;
mod workbench_library_tools;
mod workbench_paint;
mod workbench_screens;
mod workbench_table;
mod workbench_tile_tools;
mod workbench_tiles;
mod workbench_tileset;
mod workbench_tools;
mod workbench_zones;
pub mod zones;

pub use brushes::{BrushModel, ViewBrush};
pub use camera::{Camera, FlyInput, ViewKind};
pub use console::{LogLevel, LogLine};
pub use document::{Document, Selection};
pub use file_pages::ProgramFile;
pub use game_sound::{GameSound, Hearer, SoundMode};
pub use guide::{FoundBrowser, GuideBrowser};
pub use interface_look::InterfaceLook;
pub use launcher::{FolderState, KnownProject, NewProjectPlan, ProjectKind, Starter};
pub use level_view::{LevelView, ViewLight, ViewModel};
pub use library::{LibraryItem, LibraryKind, ProjectInfo};
pub use look::{Look, LookKind};
pub use make_prefab::{MakePrefabPlan, SwapPlan};
pub use mixer_view::MixerState;
pub use models::{FileLight, FileLightKind, ModelInfo};
pub use own_files::OwnFile;
pub use panel::Panel;
pub use play::PlayState;
pub use prefab_details::Placed;
pub use save_game::{FORMAT as SAVE_FORMAT, save_read_back, save_reads};
pub use shell::{BottomTab, RibbonTab, ShellState};
pub use sound_view::{EventState, Preview, SoundInfo, SoundState};
pub use splash::{SplashView, UpdateAnswer, UpdateOffer};
pub use state_list::{PartDifference, StateList, StateListError};
pub use tags::TagUse;
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
mod field_hints_tests;
#[cfg(test)]
mod game_set_tests;
#[cfg(test)]
mod hint_picture_tests;
#[cfg(test)]
mod home_screen_tests;
#[cfg(test)]
mod invariant_tests;
#[cfg(test)]
mod kind_picture_tests;
#[cfg(test)]
mod kit_tests;
#[cfg(test)]
mod kit_topdown_tests;
#[cfg(test)]
mod level_words_tests;
#[cfg(test)]
mod library_browser_tests;
#[cfg(test)]
mod library_picture_tests;
#[cfg(test)]
mod library_scale_tests;
#[cfg(test)]
mod library_screen_tests;
#[cfg(test)]
mod open_externally_tests;
#[cfg(test)]
mod picture_support;
#[cfg(test)]
mod picture_tests;
#[cfg(test)]
mod placing_tests;
#[cfg(test)]
mod prefab_picture_tests;
#[cfg(test)]
mod prefab_scale_tests;
#[cfg(test)]
mod scale_levels_tests;
#[cfg(test)]
mod shell_reach_tests;
#[cfg(test)]
mod source_rules_tests;
#[cfg(test)]
mod tag_picture_tests;
#[cfg(test)]
mod tags_play_tests;
#[cfg(test)]
mod timers_play_tests;
#[cfg(test)]
mod tools_1a_gate_tests;
#[cfg(test)]
mod two_d_screen_tests;
#[cfg(test)]
mod zone_picture_tests;
#[cfg(test)]
mod zones_play_tests;
