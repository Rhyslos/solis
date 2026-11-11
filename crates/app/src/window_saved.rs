//! The settings saved last time, read back as the window opens.
//!
//! The editor's settings are keys and values in eframe's storage (one
//! file on this computer). Each key is read by itself, and one that is
//! missing or can't be read gives its standard value, never an error: the
//! editor always starts.
//!
//! **Keys are never given a new meaning.** The editor's look is saved
//! under its own key since the design's one theme and five accents
//! (decision 0100). The keys an earlier build wrote for the theme, rounded
//! panels and the accent are read once more, only while the new key is
//! missing, to carry the accent over; they are never written again and
//! never removed, so an earlier build still finds what it saved.

use super::{
    ACCENT_KEY, AUTO_UPDATE_KEY, CAMERA_KEY, CONSOLE_HISTORY_KEY, FIRST_SIZE, HINTS_KEY,
    INTERFACE_LOOK_KEY, KNOWN_PROJECTS_KEY, LAYOUT_KEY, LIBRARY_PINNED_KEY, LOOK_KEY,
    OLD_ACCENT_KEY, OVERLAYS_KEY, RECENT_PROJECTS_KEY, SKIPPED_BUILD_KEY, SNAPPING_KEY, VIEWS_KEY,
    WORKSPACES_KEY, saved_editor_size,
};
use crate::known_projects::KnownProjects;
use editor::interface_look::AccentBefore;
use editor::{AppInfo, Camera, DockState, InterfaceLook, Look, Overlays, Panel, Workbench};
use std::path::PathBuf;

/// Everything the editor remembers between runs.
pub(super) struct Saved {
    /// The interface's accent, scale and density.
    pub(super) look: InterfaceLook,
    /// The editor window's size while a project is open.
    pub(super) editor_size: [f32; 2],
    /// Every project the launcher knows.
    pub(super) known: KnownProjects,
    /// Whether newer builds are looked for as Solis starts.
    pub(super) auto_update: bool,
    /// The build skipped with Skip this version.
    pub(super) skipped: Option<u64>,
    layout: Option<DockState<Panel>>,
    workspaces: Option<editor::workspace::Workspaces>,
    library_pinned: bool,
    hints: bool,
    camera: Camera,
    viewport_look: Look,
    overlays: Overlays,
    snapping: editor::Snapping,
    views: Option<editor::Views>,
    console_history: Vec<String>,
}

/// The interface's look: as saved, or the first time after the design's
/// five accents arrived, the accent an earlier build saved, mapped onto
/// them.
fn saved_look(storage: &dyn eframe::Storage) -> InterfaceLook {
    eframe::get_value(storage, INTERFACE_LOOK_KEY).unwrap_or_else(|| {
        let chosen: Option<AccentBefore> = eframe::get_value(storage, ACCENT_KEY);
        let oldest: Option<AccentBefore> = eframe::get_value(storage, OLD_ACCENT_KEY);
        InterfaceLook::from_before(chosen, oldest)
    })
}

impl Saved {
    /// Reads what was saved; with no storage at all (a first run, or a
    /// disk that can't be written), everything is standard.
    pub(super) fn read(storage: Option<&dyn eframe::Storage>) -> Self {
        fn get<T: serde::de::DeserializeOwned>(
            storage: Option<&dyn eframe::Storage>,
            key: &str,
        ) -> Option<T> {
            storage.and_then(|s| eframe::get_value(s, key))
        }
        let recent: Vec<PathBuf> = get(storage, RECENT_PROJECTS_KEY).unwrap_or_default();
        Self {
            look: storage.map(saved_look).unwrap_or_default(),
            editor_size: storage.map_or(FIRST_SIZE, saved_editor_size),
            known: KnownProjects::from_saved(get(storage, KNOWN_PROJECTS_KEY), recent),
            auto_update: get(storage, AUTO_UPDATE_KEY).unwrap_or(true),
            skipped: get::<Option<u64>>(storage, SKIPPED_BUILD_KEY).flatten(),
            layout: get(storage, LAYOUT_KEY),
            workspaces: get(storage, WORKSPACES_KEY),
            library_pinned: get(storage, LIBRARY_PINNED_KEY).unwrap_or(true),
            hints: get(storage, HINTS_KEY).unwrap_or(true),
            camera: get(storage, CAMERA_KEY).unwrap_or_default(),
            viewport_look: get(storage, LOOK_KEY).unwrap_or_default(),
            overlays: get(storage, OVERLAYS_KEY).unwrap_or_default(),
            snapping: get(storage, SNAPPING_KEY).unwrap_or_default(),
            views: get(storage, VIEWS_KEY),
            console_history: get(storage, CONSOLE_HISTORY_KEY).unwrap_or_default(),
        }
    }

    /// The editor as it was left: its panels, its look, its viewport and
    /// its console's history.
    pub(super) fn workbench(&mut self, info: AppInfo) -> Workbench {
        let mut workbench = Workbench::new(info, self.layout.take())
            .with_interface_look(self.look)
            .with_overlays(self.overlays)
            .with_snapping(self.snapping)
            .with_camera(self.camera)
            .with_look(self.viewport_look)
            .with_console_history(std::mem::take(&mut self.console_history));
        workbench.set_auto_update(self.auto_update);
        // Saved before views existed: the one camera and look above.
        if let Some(views) = &self.views {
            workbench = workbench.with_views(views);
        }
        workbench
    }

    /// Puts back what only applies once the editor (not an exported game)
    /// is up: the Library's pin, hints, and every workspace's panels.
    pub(super) fn settle(&mut self, workbench: &mut Workbench) {
        if !self.library_pinned {
            workbench.set_library_pinned(false);
        }
        workbench.set_hints_on(self.hints);
        if let Some(workspaces) = self.workspaces.take() {
            workbench.set_workspaces(workspaces);
        }
    }
}

#[cfg(test)]
#[path = "window_saved_tests.rs"]
mod tests;
