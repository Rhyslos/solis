//! File › Preferences (Screen 8; round 2's `11-prefs-general`,
//! `-viewport` and `-viewport-2`): the controls of the editor's own
//! settings, kept on this computer, and the column beside them: how an
//! unfinished part looks, a small picture of the editor in the look
//! chosen, and Reset to defaults. Every change applies at once.

use super::settings::{Setting, Tab};
use super::{plain, sentence};
use crate::icons::Icon;
use crate::interface_look::{PanelLayout, ShelfOpensOn};
use crate::look::LookKind;
use crate::theme::{self, Accent, Density, Scale};
use crate::tokens::{self, size, text};
use crate::transforming::{ANGLE_STEPS, GRID_STEPS, PixelSnap};
use crate::unbuilt::{self, UnbuiltId};
use crate::viewport::{Overlays, ViewLayout};
use crate::widgets::{self, button::Button, field, segmented::segmented, slider, swatch, toggle};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{CornerRadius, Rect, Sense, Ui, pos2, vec2};

impl Workbench {
    /// Sets the accent colour for the whole editor (remembered next time).
    pub(crate) fn choose_accent(&mut self, ctx: &egui::Context, accent: Accent) {
        self.choose_interface_look(
            ctx,
            theme::InterfaceLook {
                accent,
                ..self.interface
            },
        );
    }

    /// Sets the accent, scale and density for the whole editor (remembered
    /// next time).
    pub(crate) fn choose_interface_look(
        &mut self,
        ctx: &egui::Context,
        look: theme::InterfaceLook,
    ) {
        self.accent = look.accent;
        self.interface = look;
        theme::restyle(ctx, look);
    }

    /// Hides every unfinished part of the editor, or shows them again
    /// (remembered next time).
    pub(crate) fn hide_unfinished(&mut self, ctx: &egui::Context, hide: bool) {
        self.choose_interface_look(
            ctx,
            theme::InterfaceLook {
                hide_unfinished: hide,
                ..self.interface_look()
            },
        );
    }

    /// Puts the middle of the tab in front back as the design draws it:
    /// its shelf tab, the shelf's fold, and every border's size in every
    /// layout (the shelf's height, the split, the columns).
    pub fn reset_layout(&mut self) {
        // From File › Preferences: the tab File was opened from.
        let tab = match self.shell.tab {
            crate::shell::RibbonTab::File => self.shell.before_file,
            tab => tab,
        };
        self.shell.arrangements.remove(&tab);
        self.shell.sizes.forget_tab(tab);
    }

    /// The control of one of Preferences' settings.
    pub(super) fn preference_control(
        &mut self,
        ui: &mut Ui,
        setting: Setting,
        asked: &mut Vec<WorkbenchAction>,
    ) {
        let ctx = ui.ctx().clone();
        let look = self.interface_look();
        let name = setting.name();
        match setting {
            Setting::AutoUpdate => {
                let mut on = self.auto_update;
                if toggle::toggle(ui, &mut on, name).changed() {
                    self.auto_update = on;
                    asked.push(WorkbenchAction::SetAutoUpdate(on));
                }
            }
            Setting::Hints => drop(toggle::toggle(ui, &mut self.hints, name)),
            Setting::HideUnfinished => {
                let mut on = look.hide_unfinished;
                if toggle::toggle(ui, &mut on, name).changed() {
                    self.hide_unfinished(&ctx, on);
                }
            }
            Setting::Accent => {
                let mut accent = look.accent;
                let choices = Accent::ALL.map(|a| (a, a.colour(), a.name()));
                if swatch::swatches(ui, name, &mut accent, &choices, tokens::shelf_bar()) {
                    self.choose_accent(&ctx, accent);
                }
            }
            Setting::Scale => {
                let mut scale = look.scale;
                let names = Scale::ALL.map(|s| (s, s.name()));
                let choices: Vec<(Scale, &str)> =
                    names.iter().map(|(s, n)| (*s, n.as_str())).collect();
                if field::dropdown(ui, name, &mut scale, &choices, size::SETTING_FIELD_W) {
                    self.choose_interface_look(&ctx, theme::InterfaceLook { scale, ..look });
                }
            }
            Setting::Density => {
                let mut density = look.density;
                let choices = Density::ALL.map(|d| (d, d.name()));
                if segmented(ui, name, &mut density, &choices) {
                    self.choose_interface_look(&ctx, theme::InterfaceLook { density, ..look });
                }
            }
            Setting::ShelfOpensOn => {
                let mut opens = look.shelf_opens_on;
                let choices = ShelfOpensOn::ALL.map(|s| (s, s.name()));
                if field::dropdown(ui, name, &mut opens, &choices, size::SETTING_FIELD_W) {
                    self.choose_interface_look(
                        &ctx,
                        theme::InterfaceLook {
                            shelf_opens_on: opens,
                            ..look
                        },
                    );
                }
            }
            Setting::Layout => {
                let mut layout = look.layout;
                let choices = PanelLayout::ALL.map(|l| (l, l.name()));
                if field::dropdown(ui, name, &mut layout, &choices, size::SETTING_FIELD_W) {
                    self.choose_interface_look(&ctx, theme::InterfaceLook { layout, ..look });
                }
            }
            Setting::ResetLayout => {
                if Button::plain("Reset layout").show(ui).clicked() {
                    self.reset_layout();
                }
            }
            Setting::Stats => drop(toggle::toggle(ui, &mut self.viewport.overlays.stats, name)),
            Setting::Pills => drop(toggle::toggle(
                ui,
                &mut self.viewport.overlays.snapping,
                name,
            )),
            Setting::Toolbox => drop(toggle::toggle(ui, &mut self.viewport.overlays.tools, name)),
            Setting::PreviewAnimation => {
                drop(toggle::toggle(
                    ui,
                    &mut self.viewport.overlays.preview_animation,
                    name,
                ));
            }
            Setting::GridStep => {
                let words = GRID_STEPS.map(|s| (s, format!("{s} m")));
                steps(ui, name, &mut self.viewport.snapping.grid_step, &words);
            }
            Setting::AngleStep => {
                let words = ANGLE_STEPS.map(|s| (s, format!("{s}°")));
                steps(ui, name, &mut self.viewport.snapping.angle_step, &words);
            }
            Setting::ScaleStep => drop(plain(ui, "10%", text::mono(), tokens::text_primary())),
            Setting::PixelSnap => {
                let choices = [
                    (PixelSnap::Pixels, "Pixels"),
                    (PixelSnap::Tiles, "Tiles"),
                    (PixelSnap::Off, "Off"),
                ];
                segmented(ui, name, &mut self.viewport.snapping.pixel_snap, &choices);
            }
            Setting::CameraSpeed => self.camera_speed(ui, name),
            Setting::Views => {
                let mut layout = self.viewport.layout;
                let choices = ViewLayout::ALL.map(|l| (l, l.name()));
                if field::dropdown(ui, name, &mut layout, &choices, size::SETTING_FIELD_W) {
                    self.viewport.layout = layout;
                    self.viewport.maximized = None;
                }
            }
            Setting::Look => {
                let choices = LookKind::ALL.map(|k| (k, k.name()));
                segmented(ui, name, &mut self.viewport.look.kind, &choices);
            }
            Setting::Bands => {
                let mut bands = f64::from(self.viewport.look.bands);
                let field = field::whole_number_field(ui, name, &mut bands, size::KEY_COLUMN_W);
                if field.changed() {
                    let (low, high) = (
                        *crate::look::Look::BANDS.start(),
                        *crate::look::Look::BANDS.end(),
                    );
                    self.viewport.look.bands = (bands.round() as u8).clamp(low, high);
                }
            }
            Setting::Outlines => drop(toggle::toggle(ui, &mut self.viewport.look.outlines, name)),
            _ => {}
        }
    }

    /// Camera speed: the slider, its number after it.
    fn camera_speed(&mut self, ui: &mut Ui, name: &str) {
        let speed = &mut self.viewport.camera.speed;
        plain(ui, &speed.to_string(), text::mono(), tokens::text_primary());
        let mut value = f32::from(*speed);
        ui.allocate_ui(
            vec2(
                size::SETTING_FIELD_W - size::KEY_VALUE_GAP * 2.0,
                size::SETTING_ROW_MIN_H,
            ),
            |ui| {
                if slider::slider(ui, &mut value, 1.0..=8.0, name).changed() {
                    *speed = value.round().clamp(1.0, 8.0) as u8;
                }
            },
        );
    }

    /// The column beside Preferences' settings.
    pub(super) fn preferences_side(
        &mut self,
        ui: &mut Ui,
        tab: Tab,
        _asked: &mut [WorkbenchAction],
    ) {
        ui.add_space(size::SECTION_GAP);
        match tab {
            Tab::General => unfinished_card(ui),
            Tab::Appearance => miniature(ui, self.accent),
            _ => {}
        }
        if Button::plain("Reset to defaults")
            .wide()
            .show(ui)
            .on_hover_text("Puts this tab's settings back as they were at first")
            .clicked()
        {
            self.reset_preferences(ui.ctx(), tab);
        }
    }

    /// Puts one of Preferences' sub-tabs back as it was at first.
    fn reset_preferences(&mut self, ctx: &egui::Context, tab: Tab) {
        let standard = theme::InterfaceLook::default();
        let look = self.interface_look();
        match tab {
            Tab::General => {
                self.hints = true;
                self.hide_unfinished(ctx, standard.hide_unfinished);
            }
            Tab::Appearance => self.choose_interface_look(
                ctx,
                theme::InterfaceLook {
                    accent: standard.accent,
                    scale: standard.scale,
                    density: standard.density,
                    shelf_opens_on: standard.shelf_opens_on,
                    layout: standard.layout,
                    ..look
                },
            ),
            Tab::Viewport => {
                let standard = crate::transforming::Snapping::default();
                let snapping = &mut self.viewport.snapping;
                snapping.grid_step = standard.grid_step;
                snapping.angle_step = standard.angle_step;
                snapping.pixel_snap = standard.pixel_snap;
                self.viewport.overlays = Overlays::default();
                self.viewport.look = crate::look::Look::default();
                self.viewport.layout = ViewLayout::One;
                self.viewport.maximized = None;
            }
            _ => {}
        }
    }
}

/// A step's drop-down: the steps there are, in their words.
fn steps<const N: usize>(ui: &mut Ui, name: &str, value: &mut f32, words: &[(f32, String); N]) {
    let choices: Vec<(u32, &str)> = words
        .iter()
        .map(|(s, w)| (s.to_bits(), w.as_str()))
        .collect();
    let mut chosen = value.to_bits();
    if field::dropdown(ui, name, &mut chosen, &choices, size::SETTING_FIELD_W) {
        *value = f32::from_bits(chosen);
    }
}

/// The card showing what an unfinished part looks like: a built button,
/// an unfinished one and an unfinished row, as the ribbon draws them.
fn unfinished_card(ui: &mut Ui) {
    if unbuilt::hidden() {
        return;
    }
    widgets::card::card(ui, "How an unfinished part looks", |ui| {
        ui.set_width(ui.available_width());
        widgets::card::frame(ui)
            .fill(tokens::ribbon())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(size::RIBBON_BIG[0], size::RIBBON_BIG[1]),
                        Sense::hover(),
                    );
                    let spot = Rect::from_center_size(
                        rect.center() - vec2(0.0, 6.0),
                        vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
                    );
                    crate::icons::icon(ui.painter(), spot, Icon::Save, tokens::text_body());
                    let label = Rect::from_min_max(pos2(rect.left(), spot.bottom()), rect.max);
                    widgets::words(
                        ui,
                        label,
                        egui::Align2::CENTER_CENTER,
                        "Save",
                        text::ribbon_label(),
                        tokens::text_body(),
                    );
                    unbuilt::ribbon_big(ui, UnbuiltId::HomeLightingBake, Icon::Sun);
                });
            });
        sentence(
            ui,
            "A fainter label and a small hollow square. Hover one to read what it will do and \
             when it is planned; a click says so in the status bar.",
        );
    });
}

/// A small picture of the editor in the accent chosen: the tab bar with
/// its line, the ribbon, the viewport with a selected thing, and the
/// shelf's cards (Screen 8's Preview).
fn miniature(ui: &mut Ui, accent: Accent) {
    widgets::rows::section_caps(ui, "Preview");
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, width * 0.58), Sense::hover());
    let painter = ui.painter();
    let round = CornerRadius::same(size::radius::CARD);
    painter.rect_filled(rect, round, tokens::app());
    widgets::border(ui, rect, size::radius::CARD, tokens::card_border());
    let band = |from: f32, to: f32| {
        Rect::from_min_max(
            pos2(rect.left() + 1.0, rect.top() + rect.height() * from),
            pos2(rect.right() - 1.0, rect.top() + rect.height() * to),
        )
    };
    painter.rect_filled(band(0.08, 0.2), 0.0, tokens::ribbon());
    let tab = Rect::from_min_size(
        pos2(rect.left() + 8.0, rect.top() + rect.height() * 0.06),
        vec2(18.0, 2.0),
    );
    painter.rect_filled(tab, 0.0, accent.colour());
    let view = band(0.2, 0.68);
    painter.rect_filled(
        view.with_max_x(view.left() + view.width() * 0.6),
        0.0,
        tokens::viewport(),
    );
    let thing = Rect::from_center_size(
        pos2(view.left() + view.width() * 0.25, view.center().y),
        vec2(8.0, view.height() * 0.55),
    );
    painter.rect_stroke(
        thing,
        0.0,
        egui::Stroke::new(1.0, accent.colour()),
        egui::StrokeKind::Inside,
    );
    let shelf = band(0.7, 0.98);
    painter.rect_filled(shelf, 0.0, tokens::shelf_bar());
    for n in 0..3 {
        let w = (shelf.width() - 16.0) / 3.0;
        let card = Rect::from_min_size(
            pos2(shelf.left() + 4.0 + n as f32 * (w + 4.0), shelf.top() + 6.0),
            vec2(w, shelf.height() - 12.0),
        );
        painter.rect_filled(
            card,
            CornerRadius::same(size::radius::SMALL),
            tokens::panel(),
        );
    }
}
