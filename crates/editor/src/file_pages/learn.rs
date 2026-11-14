//! File › Learn (round 2's `10-file-learn`): the four examples to make
//! and take apart, the first hour's five steps, the user guide, and the
//! sample area.

use super::{examples, page_head, plain, section, sentence};
use crate::icons;
use crate::tokens::{self, size, text};
use crate::widgets::{self, button::Button, keycap};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Rect, Sense, Ui, vec2};

/// The first hour, a step a line. A word between `*` is a name on screen,
/// drawn brighter; `[Space]` is a key.
const FIRST_HOUR: [&str; 5] = [
    "Make a project: a folder for your game's files.",
    "Put art in its assets folder: it shows in the Library.",
    "In the Library, *Create* makes maps, tilesets and code files.",
    "Press *Play* to try it.",
    "Stuck? Tap [Space] and type what you want to do.",
];

impl Workbench {
    pub(super) fn learn_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        page_head(
            ui,
            "Learn",
            "New to making games? Make an example, press Play, then change something and see \
             what happens.",
            |_| {},
        );
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                section(ui, "Examples to open and take apart");
                ui.add_space(size::CARD_INNER_GAP);
                example_cards(ui, asked);
                super::with_side(
                    ui,
                    self,
                    |ui, _| {
                        section(ui, "Your first hour");
                        for (n, step) in FIRST_HOUR.iter().enumerate() {
                            step_row(ui, n + 1, step);
                        }
                    },
                    |ui, w| {
                        section(ui, "Help");
                        if super::button_row(
                            ui,
                            "The user guide",
                            "Every part of the editor, with the why as well as the how.",
                            "Open the user guide",
                            true,
                        )
                        .clicked()
                        {
                            asked.push(WorkbenchAction::OpenGuide(None));
                        }
                        let open = w.open.project.is_some();
                        let add = ui
                            .add_enabled_ui(open, |ui| {
                                super::button_row(
                                    ui,
                                    "The sample area",
                                    "Adds a small ready-made area to the open project, to look \
                                     around in.",
                                    "Add the sample area",
                                    false,
                                )
                            })
                            .inner;
                        let add = if open {
                            add
                        } else {
                            add.on_disabled_hover_text("Open a project first")
                        };
                        if add.clicked() {
                            asked.push(WorkbenchAction::AddSampleArea);
                        }
                    },
                );
            });
    }
}

/// The four examples, side by side: an icon, the name, what it is, and
/// Make it in a folder….
fn example_cards(ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
    let all = examples();
    let gap = size::PROJECT_CARD_GAP;
    let across = super::recent::cards_across(ui.available_width()).min(all.len());
    let width = (ui.available_width() - gap * (across as f32 - 1.0)) / across as f32;
    for row in all.chunks(across) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for example in row {
                let frame = widgets::card::frame(ui)
                    .fill(tokens::panel())
                    .corner_radius(egui::CornerRadius::same(size::radius::PROJECT_CARD))
                    .inner_margin(egui::Margin::same(size::PROJECT_CARD_PAD[1] as i8));
                frame.show(ui, |ui| {
                    let inner = width - 2.0 * (size::PROJECT_CARD_PAD[1] + size::LINE);
                    ui.set_width(inner);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
                        picture_box(ui, inner, example.icon);
                        plain(ui, example.name, text::card_name(), tokens::text_primary());
                        sentence(ui, example.about);
                        let make = Button::plain("Make it in a folder…")
                            .show(ui)
                            .on_hover_text("Made as a new project, in a folder you choose");
                        make.widget_info(|| {
                            let name = format!("Make the {}", example.name);
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                        });
                        if make.clicked() {
                            asked.push(example.make.clone());
                        }
                    });
                });
            }
        });
        ui.add_space(gap);
    }
}

/// The dark box an example's icon sits in.
fn picture_box(ui: &mut Ui, width: f32, icon: icons::Icon) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(width, size::TEMPLATE_BOX_H * 0.75), Sense::hover());
    let radius = egui::CornerRadius::same(size::radius::CARD);
    ui.painter().rect_filled(rect, radius, tokens::shelf_bar());
    widgets::border(ui, rect, size::radius::CARD, tokens::card_border());
    let spot = Rect::from_center_size(
        rect.center(),
        vec2(size::RIBBON_BIG_ICON, size::RIBBON_BIG_ICON),
    );
    icons::icon(ui.painter(), spot, icon, tokens::text_label());
}

/// One step of the first hour: its number, then its words, with names
/// on screen brighter and keys as keycaps, and a hairline under it.
fn step_row(ui: &mut Ui, n: usize, step: &str) {
    let across = ui.available_rect_before_wrap().x_range();
    ui.add_space(size::SETTING_ROW_PAD / 2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::GAP;
        plain(ui, &n.to_string(), text::row(), tokens::text_meta());
        ui.add_space(size::CARD_INNER_GAP);
        for (i, part) in step.split(['[', ']']).enumerate() {
            if i % 2 == 1 {
                keycap::keycap(ui, part, keycap::KeycapState::Bound);
                continue;
            }
            for (j, piece) in part.split('*').enumerate() {
                if piece.is_empty() {
                    continue;
                }
                let ink = if j % 2 == 1 {
                    tokens::text_emphasis()
                } else {
                    tokens::text_body()
                };
                ui.spacing_mut().item_spacing.x = 0.0;
                plain(ui, piece, text::page_subtitle(), ink);
            }
            ui.spacing_mut().item_spacing.x = size::GAP;
        }
    });
    ui.add_space(size::SETTING_ROW_PAD / 2.0);
    let y = ui.cursor().top();
    tokens::hline(ui.painter(), across, y, tokens::row_hover());
    ui.add_space(size::LINE);
}
