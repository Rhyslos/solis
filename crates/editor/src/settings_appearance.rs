//! Settings > Appearance, drawn with the design's own controls (the kit,
//! `widgets/`), as the design's Preferences screen lays its settings out:
//! small capitals over each section, and in each row a setting's name
//! and a line about it on the left, its control at the right.
//!
//! The page is a list, made of settings that work and of one the design
//! has that isn't built (its "Code editor split"), which is drawn by
//! `unbuilt` and goes when **Hide unfinished parts** is on. The page
//! stays in today's Settings window until the File pages are built.

use crate::settings_window::{Now, Request, Setting};
use crate::theme::{Accent, Density, Scale};
use crate::tokens::size;
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::button::Button;
use crate::widgets::field::dropdown;
use crate::widgets::rows::{section_caps, setting_row};
use crate::widgets::segmented::segmented;
use crate::widgets::swatch::swatches;
use crate::widgets::toggle::toggle;
use egui::Ui;

/// The page's sections, each with what is in it, in the design's order.
fn sections() -> Vec<(&'static str, Vec<Entry<Setting>>)> {
    use Entry::{Built, Unbuilt};
    vec![
        (
            "Theme",
            vec![
                Built(Setting::Accent),
                Built(Setting::Scale),
                Built(Setting::Density),
            ],
        ),
        (
            "Layout",
            vec![
                Built(Setting::Layout),
                Built(Setting::Hints),
                Built(Setting::HideUnfinished),
                Unbuilt(UnbuiltId::PrefsCodeSplit),
            ],
        ),
    ]
}

/// Draws the page. Returns what a click on it asked for.
pub(crate) fn page(ui: &mut Ui, now: &Now) -> Option<Request> {
    let mut request = None;
    for (index, (caps, entries)) in unbuilt::filter_groups(sections()).into_iter().enumerate() {
        if index > 0 {
            ui.add_space(size::SECTION_GAP);
        }
        section_caps(ui, caps);
        for entry in entries {
            match entry {
                Entry::Built(setting) => {
                    if let Some(asked) = row(ui, setting, now) {
                        request = Some(asked);
                    }
                }
                Entry::Unbuilt(id) => drop(unbuilt::setting_row(ui, id)),
            }
        }
    }
    request
}

/// One of the page's settings as a row (also how it is shown when the
/// search box finds it). Returns what a click on it asked for.
pub(crate) fn row(ui: &mut Ui, setting: Setting, now: &Now) -> Option<Request> {
    // Rows follow one another with no gap: each closes with its own line.
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        setting_row(ui, setting.name(), setting.hint(), |ui| {
            control(ui, setting, now)
        })
    })
    .inner
}

/// A setting's control, at the right of its row.
fn control(ui: &mut Ui, setting: Setting, now: &Now) -> Option<Request> {
    match setting {
        Setting::Accent => {
            let mut accent = now.look.accent;
            let choices = Accent::ALL.map(|accent| (accent, accent.colour(), accent.name()));
            // The gap inside the chosen swatch's ring is the colour the
            // page lies on.
            let page = ui.visuals().window_fill;
            swatches(ui, setting.name(), &mut accent, &choices, page)
                .then_some(Request::Accent(accent))
        }
        Setting::Scale => {
            let mut scale = now.look.scale;
            let names = Scale::ALL.map(|scale| (scale, scale.name()));
            let choices: Vec<(Scale, &str)> = names
                .iter()
                .map(|(scale, name)| (*scale, name.as_str()))
                .collect();
            dropdown(
                ui,
                setting.name(),
                &mut scale,
                &choices,
                size::SETTING_FIELD_W,
            )
            .then_some(Request::Scale(scale))
        }
        Setting::Density => {
            let mut density = now.look.density;
            let choices = Density::ALL.map(|density| (density, density.name()));
            segmented(ui, setting.name(), &mut density, &choices)
                .then_some(Request::Density(density))
        }
        Setting::Layout => Button::plain("Reset layout")
            .show(ui)
            .clicked()
            .then_some(Request::ResetLayout),
        Setting::Hints => {
            let mut on = now.hints;
            toggle(ui, &mut on, "Show hints")
                .changed()
                .then_some(Request::Hints(on))
        }
        Setting::HideUnfinished => {
            let mut on = now.look.hide_unfinished;
            toggle(ui, &mut on, setting.name())
                .changed()
                .then_some(Request::HideUnfinished(on))
        }
        // The project's 2D settings have their own page and controls.
        Setting::BaseResolution | Setting::PixelsPerUnit | Setting::TileSize | Setting::Scaling => {
            None
        }
    }
}
