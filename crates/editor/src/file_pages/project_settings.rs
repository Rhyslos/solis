//! File › Project settings (Screen 12; round 2's `12-project-settings-*`):
//! the controls of the open project's own settings, which travel with
//! it, and the column beside them: what has changed from the standard,
//! how the game's size fits common screens, and version control (not
//! built yet). Build's own rows are `build.rs`'s; Input is `input_page.rs`.
//!
//! The 2D settings are kept in the project's `pixels.solpix` (Phase 9,
//! D1); a number being dragged is saved once, when it is let go.

use super::settings::{Setting, Tab};
use super::{plain, sentence};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use crate::widgets::{self, field, segmented::segmented};
use crate::workbench::{Workbench, WorkbenchAction};
use area::pixels::{PixelSettings, Scaling};
use egui::Ui;

/// Common base resolutions for a pixel-art game, offered in the list.
pub(crate) const BASE_SIZES: [[u32; 2]; 6] = [
    [320, 180],
    [384, 216],
    [480, 270],
    [640, 360],
    [960, 540],
    [1280, 720],
];

/// The screens the 2D tab says a base resolution scales to.
const SCREENS: [[u32; 2]; 3] = [[1920, 1080], [2560, 1440], [3840, 2160]];

/// How `s`'s base resolution fits each of [`SCREENS`]: "4 times,
/// exactly", "5 times, the rest smoothed" or "5 times, with bars".
pub(crate) fn fits(s: PixelSettings) -> Vec<(String, String)> {
    let [w, h] = s.base;
    SCREENS
        .iter()
        .map(|&[sw, sh]| {
            let whole = (sw / w.max(1)).min(sh / h.max(1));
            let how = if whole >= 1 && whole * w == sw && whole * h == sh {
                format!("{whole} times, exactly")
            } else if s.scaling == Scaling::WholeSteps && whole >= 1 {
                format!("{whole} times, with bars")
            } else {
                let exact = (sw as f32 / w as f32).min(sh as f32 / h as f32);
                format!("{exact:.2} times, the rest smoothed")
            };
            (format!("{sw} × {sh}"), how)
        })
        .collect()
}

impl Workbench {
    /// Uses the project's 2D settings (as read from its `pixels.solpix`,
    /// or the standard ones).
    pub fn set_pixels(&mut self, settings: PixelSettings) {
        self.open.pixels = settings.sanitized();
        self.viewport.pixels = self.open.pixels;
    }

    /// The project's 2D settings.
    pub fn pixels(&self) -> PixelSettings {
        self.open.pixels
    }

    /// Changes the project's 2D settings, saving them when `save`.
    pub(crate) fn change_pixels(
        &mut self,
        settings: PixelSettings,
        save: bool,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        self.open.pixels = settings.sanitized();
        self.viewport.pixels = self.open.pixels;
        if save {
            if self.open.project.is_some() {
                actions.push(WorkbenchAction::SavePixels(self.open.pixels));
            } else {
                self.log(
                    crate::LogLevel::Info,
                    "The 2D settings are kept with a project: open or make one to keep these \
                     changes.",
                );
            }
        }
    }

    /// The project's settings that aren't their standard: each with its
    /// standard value and its value now, in words.
    pub(crate) fn changed_settings(&self) -> Vec<(Setting, String, String)> {
        let now = self.open.pixels;
        let standard = PixelSettings::default();
        let size = |[w, h]: [u32; 2]| format!("{w} × {h}");
        let mut changed = Vec::new();
        let mut note = |setting, from: String, to: String| {
            if from != to {
                changed.push((setting, from, to));
            }
        };
        note(Setting::BaseResolution, size(standard.base), size(now.base));
        note(
            Setting::PixelsPerUnit,
            standard.pixels_per_unit.to_string(),
            now.pixels_per_unit.to_string(),
        );
        note(Setting::TileSize, size(standard.tile), size(now.tile));
        note(
            Setting::Scaling,
            standard.scaling.label().into(),
            now.scaling.label().into(),
        );
        if let Some(form) = &self.open.export {
            let standard = crate::export_dialog::Playtest::default();
            let window = |p: &crate::export_dialog::Playtest| {
                let how = if p.fullscreen {
                    "Fullscreen"
                } else {
                    "Windowed"
                };
                format!("{how} {}", size(p.size))
            };
            note(Setting::Window, window(&standard), window(&form.playtest));
        }
        changed
    }

    /// Puts one of the project's settings back to its standard value.
    pub(crate) fn reset_setting(&mut self, setting: Setting, asked: &mut Vec<WorkbenchAction>) {
        let standard = PixelSettings::default();
        let mut pixels = self.open.pixels;
        match setting {
            Setting::BaseResolution => pixels.base = standard.base,
            Setting::PixelsPerUnit => pixels.pixels_per_unit = standard.pixels_per_unit,
            Setting::TileSize => pixels.tile = standard.tile,
            Setting::Scaling => pixels.scaling = standard.scaling,
            Setting::Window => {
                if let Some(form) = &mut self.open.export {
                    let standard = crate::export_dialog::Playtest::default();
                    form.playtest.fullscreen = standard.fullscreen;
                    form.playtest.size = standard.size;
                }
                return;
            }
            _ => return,
        }
        self.change_pixels(pixels, true, asked);
    }

    /// The control of one of Project settings' settings. Without a
    /// project they can't be changed.
    pub(super) fn project_control(
        &mut self,
        ui: &mut Ui,
        setting: Setting,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        if self.open.project.is_none() {
            plain(ui, "Open a project first", text::row(), tokens::text_meta());
            return;
        }
        match setting {
            Setting::ProjectName => self.project_name(ui, asked),
            Setting::EngineVersion => {
                if widgets::button::link(ui, "Check now").clicked() {
                    asked.push(WorkbenchAction::CheckForUpdates);
                }
                plain(ui, &self.update.line(), text::row(), tokens::text_meta());
                let build = self
                    .this_build
                    .map_or_else(|| "from source".into(), |b| format!("build {b}"));
                plain(ui, &build, text::mono(), tokens::text_primary());
            }
            Setting::Scripting => drop(plain(
                ui,
                "Your code editor",
                text::ui(),
                tokens::text_primary(),
            )),
            Setting::BaseResolution
            | Setting::PixelsPerUnit
            | Setting::TileSize
            | Setting::Scaling => self.pixel_control(ui, setting, asked),
            _ => self.build_control(ui, setting, asked),
        }
    }

    /// The project's name: a field, renamed when Enter is pressed or the
    /// field is left (its folder keeps its name).
    fn project_name(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some(project) = self.open.project.clone() else {
            return;
        };
        let typed = self
            .file_pages
            .name_typed
            .get_or_insert_with(|| project.name.clone());
        let field = field::text_field(ui, "Project name", typed, "", size::SETTING_FIELD_W);
        if field.lost_focus() {
            let name = typed.trim().to_string();
            self.file_pages.name_typed = None;
            if name != project.name && super::name_problem(&name).is_none() {
                asked.push(WorkbenchAction::RenameProject(project.root, name));
            }
        }
    }

    /// A 2D setting's control; a change is kept at once and saved, unless
    /// a number is still being dragged.
    fn pixel_control(&mut self, ui: &mut Ui, setting: Setting, asked: &mut Vec<WorkbenchAction>) {
        let s = self.open.pixels;
        let mut new = s;
        let mut changed = false;
        let mut dragging = false;
        let mut number = |ui: &mut Ui, value: &mut u32, range: (u32, u32), name: &str| {
            let mut v = f64::from(*value);
            let r =
                field::whole_number_field(ui, name, &mut v, size::KEY_COLUMN_W - size::GAP * 4.0);
            *value = (v.round() as u32).clamp(range.0, range.1);
            changed |= r.changed() || r.drag_stopped();
            dragging |= r.dragged() && !r.drag_stopped();
        };
        let times = |ui: &mut Ui| drop(plain(ui, "×", text::row(), tokens::text_meta()));
        let px = |ui: &mut Ui| drop(plain(ui, "px", text::row(), tokens::text_meta()));
        match setting {
            Setting::BaseResolution => {
                number(
                    ui,
                    &mut new.base[1],
                    area::pixels::BASE_RANGE,
                    "Base height",
                );
                times(ui);
                number(ui, &mut new.base[0], area::pixels::BASE_RANGE, "Base width");
                let words = BASE_SIZES.map(|[w, h]| ([w, h], format!("{w} × {h}")));
                let choices: Vec<([u32; 2], &str)> =
                    words.iter().map(|(s, w)| (*s, w.as_str())).collect();
                let mut base = new.base;
                if field::dropdown(
                    ui,
                    "Base resolution",
                    &mut base,
                    &choices,
                    size::KEY_COLUMN_W + size::GAP * 3.0,
                ) {
                    new.base = base;
                    changed = true;
                }
            }
            Setting::PixelsPerUnit => {
                px(ui);
                number(
                    ui,
                    &mut new.pixels_per_unit,
                    area::pixels::PIXELS_PER_UNIT_RANGE,
                    "Pixels per unit",
                );
            }
            Setting::TileSize => {
                px(ui);
                number(
                    ui,
                    &mut new.tile[1],
                    area::pixels::TILE_RANGE,
                    "Tile height",
                );
                times(ui);
                number(ui, &mut new.tile[0], area::pixels::TILE_RANGE, "Tile width");
            }
            Setting::Scaling => {
                let choices = Scaling::ALL.map(|s| (s, s.label()));
                changed |= segmented(ui, "Scaling to the screen", &mut new.scaling, &choices);
            }
            _ => {}
        }
        if changed && (new != s || !dragging) {
            // Saved once the drag ends, not every step of it.
            self.change_pixels(new, !dragging, asked);
        }
    }

    /// The column beside Project settings' settings.
    pub(super) fn project_side(&mut self, ui: &mut Ui, tab: Tab) {
        ui.add_space(size::SECTION_GAP);
        let changed = self.changed_settings();
        if !changed.is_empty() {
            widgets::card::card(ui, "Changed from defaults", |ui| {
                ui.set_width(ui.available_width());
                for (setting, from, to) in &changed {
                    ui.horizontal(|ui| {
                        plain(ui, "•", text::row(), crate::interface_look::accent_fill());
                        plain(ui, setting.name(), text::row(), tokens::text_primary());
                        if setting.tab() != tab {
                            plain(ui, setting.tab().name(), text::row(), tokens::text_meta());
                        }
                        plain(
                            ui,
                            &format!("{from} → {to}"),
                            text::mono_small(),
                            tokens::text_meta(),
                        );
                    });
                }
            });
        }
        match tab {
            Tab::TwoD => {
                let [w, h] = self.open.pixels.base;
                widgets::card::card(ui, &format!("How {w} × {h} fits a screen"), |ui| {
                    ui.set_width(ui.available_width());
                    for (screen, how) in fits(self.open.pixels) {
                        ui.horizontal(|ui| {
                            plain(ui, &screen, text::row(), tokens::text_meta());
                            plain(ui, &how, text::row(), tokens::text_primary());
                        });
                    }
                });
            }
            Tab::Build => self.export_card(ui),
            _ => {}
        }
        if !unbuilt::hidden() {
            unbuilt::card(ui, UnbuiltId::ProjVersionControl);
        }
        sentence(
            ui,
            "A dot before a setting marks a value changed from its standard. Settings that are \
             yours alone, such as the accent colour, are in Preferences.",
        );
    }
}
