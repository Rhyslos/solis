//! The editor's interface: the top bar, the dockable panels, the viewport's
//! controls and the theme ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! Layer 2 (editor). Games never need this crate. It never touches the
//! window, the GPU or files: it returns actions for the app to carry out,
//! and draws the viewport through a [`ViewportRenderer`] the app provides.
//! See this crate's README for its purpose and limits.

pub mod camera;
pub mod environment;
pub mod icons;
pub mod layout;
pub mod lighting;
pub mod look;
pub mod models;
pub mod panel;
pub mod theme;
pub mod viewport;
pub mod workbench;

pub use camera::{Camera, FlyInput};
pub use egui_dock::DockState;
pub use environment::{EnvironmentSettings, SkyKind, ToneMapping};
pub use layout::Preset;
pub use lighting::{Lamp, LampFromFile, LampId, LampKind, Selection, SunSettings};
pub use look::{Look, LookKind};
pub use models::{ModelId, ModelInfo};
pub use panel::Panel;
pub use viewport::{NoRenderer, Overlays, Tool, Viewport, ViewportRenderer, ViewportView};
pub use workbench::{AppInfo, LogLevel, LogLine, Workbench, WorkbenchAction};
