//! Each tab's ribbon, as a small table of groups: what is on it, in
//! what shape, and which command each control does. `ribbon.rs` lays a
//! table out and draws it; nothing here draws.
//!
//! A control the engine can't do yet is listed in its place as an
//! unfinished part (`Entry::Unbuilt`), so it is drawn by the one
//! mechanism and goes with **Hide unfinished parts**.

pub(crate) mod animate;
mod audio;
mod brush;
mod build;
mod debug;
mod home;
mod library;
mod model;
mod picture;
mod shading;
mod tile_map;
mod tileset;
mod ui;
mod vfx;

use super::RibbonTab;
use super::ribbon::Group;
use crate::workbench::Workbench;

/// The ribbon of `tab`, as the editor is now.
pub(crate) fn of(tab: RibbonTab, w: &Workbench) -> Vec<Group> {
    match tab {
        RibbonTab::File => Vec::new(),
        RibbonTab::Ui => ui::groups(w),
        RibbonTab::Home if w.brush_tools_in_front() => brush::groups(w),
        RibbonTab::Home if w.map_tools_in_front() => tile_map::groups(w),
        RibbonTab::Home => home::groups(w),
        RibbonTab::Library => library::groups(w),
        RibbonTab::Shading => shading::groups(),
        RibbonTab::Animate => animate::groups(w),
        RibbonTab::Vfx => vfx::groups(),
        RibbonTab::Audio => audio::groups(w),
        RibbonTab::Debug => debug::groups(),
        RibbonTab::Build => build::groups(w),
        RibbonTab::Tileset => tileset::groups(w),
        RibbonTab::Picture => picture::groups(w),
        RibbonTab::Model => model::groups(),
    }
}

/// A list of unfinished large buttons or rows under one label: most of
/// an unbuilt screen's ribbon is these.
macro_rules! group {
    ($label:expr, [$($slot:expr),* $(,)?]) => {
        $crate::shell::ribbon::Group {
            label: $label,
            slots: vec![$($slot),*],
        }
    };
}
pub(super) use group;
