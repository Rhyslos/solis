//! The splash screen ([decision
//! 0045](../../../docs/decisions/0045-a-splash-screen-that-asks-before-updating.md)):
//! what the window shows while Solis starts and checks for a newer build.
//! The logo, the name and version, a thin bar and what's happening
//! ("Checking for updates…"), on the dark background, as Aventail's
//! splash (decision 0007). The app decides what happens; this only draws.

use crate::theme::{self, Accent};
use crate::workbench::AppInfo;
use egui::{Align2, FontId, Rect, Sense, TextureId, Ui, pos2, vec2};

/// What the splash screen shows this frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SplashView<'a> {
    /// The program's name and version.
    pub info: &'a AppInfo,
    /// The accent colour in use.
    pub accent: Accent,
    /// The logo, if it could be read.
    pub logo: Option<TextureId>,
    /// What's happening ("Checking for updates…").
    pub doing: &'a str,
    /// How far along, from 0 to 1; `None` while it can't be known (the
    /// bar then sweeps).
    pub progress: Option<f32>,
    /// Seconds since the splash appeared, for the sweeping bar.
    pub time: f64,
}

/// The logo's size, in points.
const LOGO: f32 = 128.0;
/// The bar's width.
const BAR_WIDTH: f32 = 260.0;

/// Draws the splash screen over the whole of `ui`.
pub fn show(ui: &mut Ui, view: &SplashView<'_>) {
    let rect = ui.max_rect();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::BACKGROUND);
    let colors = view.accent.colors();
    let middle = rect.center();
    // The logo, the name, the version, the bar and the words, centred.
    if let Some(logo) = view.logo {
        let at = Rect::from_center_size(middle - vec2(0.0, 110.0), vec2(LOGO, LOGO));
        painter.image(
            logo,
            at,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    painter.text(
        middle - vec2(0.0, 16.0),
        Align2::CENTER_CENTER,
        &view.info.name,
        FontId::proportional(32.0),
        theme::TEXT_STRONG,
    );
    painter.text(
        middle + vec2(0.0, 16.0),
        Align2::CENTER_CENTER,
        format!("Version {}", view.info.version),
        FontId::proportional(14.0),
        theme::TEXT_WEAK,
    );
    let bar = Rect::from_center_size(middle + vec2(0.0, 50.0), vec2(BAR_WIDTH, 4.0));
    painter.rect_filled(bar, 2.0, theme::PANEL);
    let done = match view.progress {
        Some(p) => {
            Rect::from_min_size(bar.min, vec2(bar.width() * p.clamp(0.0, 1.0), bar.height()))
        }
        // A short piece sweeping across: working, how long unknown.
        None => {
            let piece = bar.width() * 0.25;
            let along = (view.time * 0.8).fract() as f32 * (bar.width() + piece) - piece;
            Rect::from_min_max(
                pos2((bar.min.x + along).max(bar.min.x), bar.min.y),
                pos2((bar.min.x + along + piece).min(bar.max.x), bar.max.y),
            )
        }
    };
    painter.rect_filled(done, 2.0, colors.fill);
    painter.text(
        pos2(middle.x, bar.max.y + 22.0),
        Align2::CENTER_CENTER,
        view.doing,
        FontId::proportional(13.0),
        theme::TEXT_WEAK,
    );
    // One label for screen readers (and tests): the name, version and
    // what's happening.
    let response = ui.allocate_rect(rect, Sense::hover());
    let text = format!("{} {}: {}", view.info.name, view.info.version, view.doing);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
}

/// A newer build, as the update question shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOffer {
    /// The newer build's number.
    pub latest: u64,
    /// This copy's build number.
    pub current: Option<u64>,
    /// Whether this copy was installed with the installer, so can update
    /// itself; otherwise it can only point to the download page.
    pub installed: bool,
}

/// What the person answered to the update question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAnswer {
    /// Download the newer build, install it and open it.
    UpdateNow,
    /// Open the page to download it by hand (a copy that wasn't
    /// installed).
    OpenDownloadPage,
    /// Carry on with this build. With `skip`, don't ask again about this
    /// build (Skip this version).
    NotNow {
        /// Whether Skip this version was ticked.
        skip: bool,
    },
}

/// How far below the window's middle the question sits on the splash
/// screen: under the logo, name, version and bar, which stay readable.
pub const BELOW_THE_SPLASH: f32 = 170.0;

/// The question "a newer build is out: update?", centred `down` points
/// below the window's middle, with the Skip this version tick in `skip`.
/// Gives the answer once one of its buttons is pressed.
pub fn update_question(
    ctx: &egui::Context,
    name: &str,
    offer: &UpdateOffer,
    skip: &mut bool,
    down: f32,
) -> Option<UpdateAnswer> {
    let mut answer = None;
    egui::Window::new("Update available")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, [0.0, down])
        .show(ctx, |ui| {
            let current = offer
                .current
                .map_or_else(String::new, |n| format!(" (you have build {n})"));
            ui.label(format!(
                "Build {} of {name} is available{current}.",
                offer.latest
            ));
            if !offer.installed {
                ui.label(
                    egui::RichText::new(
                        "This copy wasn't installed with the installer, so it can't update \
                         itself: download the new installer and run it once.",
                    )
                    .color(theme::TEXT_WEAK),
                );
            }
            ui.add_space(6.0);
            ui.checkbox(skip, "Skip this version")
                .on_hover_text("Don't ask about this build again; a newer one asks again");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let update = if offer.installed {
                    ui.button("Update now")
                        .on_hover_text(format!("Downloads it, installs it and opens {name} again"))
                } else {
                    ui.button("Open the download page")
                };
                if update.clicked() {
                    answer = Some(if offer.installed {
                        UpdateAnswer::UpdateNow
                    } else {
                        UpdateAnswer::OpenDownloadPage
                    });
                }
                if ui.button("Open without updating").clicked() {
                    answer = Some(UpdateAnswer::NotNow { skip: *skip });
                }
            });
        });
    answer
}

#[cfg(test)]
#[path = "splash_tests.rs"]
mod tests;
