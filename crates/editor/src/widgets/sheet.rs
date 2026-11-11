//! The widget sheet: one page with every control of the kit on it, in
//! each of its states, for the tests to measure, press and take pictures
//! of. Nothing in the editor shows it.
//!
//! The left half lies on a File page's grey, as the settings do; the
//! right half on a shelf column's, as cards and tiles do. So each
//! control is on the surface the design draws it on, and its colours can
//! be held against the design's pictures.

use super::button::{self, Button};
use super::chip::{ChipKind, chip};
use super::field::{Axis, dropdown, number_field, text_field};
use super::keycap::{KeycapState, keycap};
use super::rows::{column_header, key_value, section_caps, setting_row};
use super::tile::{TileText, tile};
use super::{card, checkbox, pill, segmented, slider, swatch, toggle};
use crate::icons::{self, Icon};
use crate::tokens::size::Density;
use crate::tokens::{self, Accent, size};
use egui::{Align, Layout, Rect, Ui, UiBuilder, pos2, vec2};

/// The sheet's size, in points.
pub(crate) const SIZE: [f32; 2] = [1120.0, 600.0];
/// Where the right half starts.
pub(crate) const MIDDLE: f32 = 620.0;
/// The padding round each half's contents.
pub(crate) const PAD: f32 = 20.0;

/// What the sheet's controls hold: each state of each control has a
/// value here, so a test can see a click or a key change it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sheet {
    pub(crate) labels: bool,
    pub(crate) split: bool,
    pub(crate) density: Density,
    pub(crate) accent: Accent,
    pub(crate) opens_on: usize,
    pub(crate) shadows: bool,
    pub(crate) grid: bool,
    pub(crate) volume: f32,
    pub(crate) position: [f64; 3],
    pub(crate) name: String,
    pub(crate) search: String,
    pub(crate) kind: usize,
    /// What was pressed last, by its name.
    pub(crate) pressed: Option<&'static str>,
}

impl Default for Sheet {
    fn default() -> Self {
        Self {
            labels: true,
            split: false,
            density: Density::Default,
            accent: Accent::Blue,
            opens_on: 0,
            shadows: true,
            grid: false,
            volume: 0.35,
            position: [12.5, 0.0, -4.7],
            name: "Dock_Lamp".into(),
            search: String::new(),
            kind: 0,
            pressed: None,
        }
    }
}

/// How wide each of the sheet's cards is, and the fields in it.
const CARD_W: f32 = 224.0;
const FIELD_W: f32 = CARD_W - 2.0 * size::CARD_PAD - 2.0;

/// The choices of the sheet's drop-down.
pub(crate) const OPENS_ON: [(usize, &str); 3] = [(0, "Entity"), (1, "Console"), (2, "Problems")];

/// The kinds the sheet's chips offer, each with its dot's colour.
const KINDS: [(&str, Option<egui::Color32>); 3] = [
    ("All", None),
    ("Model", Some(tokens::KIND_MODEL)),
    ("Picture", Some(tokens::KIND_PICTURE)),
];

/// A `Ui` over `rect`, laid out downwards, with the design's gaps.
fn half(ui: &mut Ui, rect: Rect) -> Ui {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.shrink(PAD))
            .layout(Layout::top_down(Align::Min)),
    );
    child.spacing_mut().item_spacing = vec2(size::GAP, size::CARD_INNER_GAP);
    child
}

impl Sheet {
    /// Draws the whole sheet from the window's top left corner.
    pub(crate) fn show(&mut self, ui: &mut Ui) {
        let whole = Rect::from_min_size(pos2(0.0, 0.0), vec2(SIZE[0], SIZE[1]));
        let (left, right) = whole.split_left_right_at_x(MIDDLE);
        ui.painter().rect_filled(left, 0.0, tokens::shelf_bar());
        ui.painter().rect_filled(right, 0.0, tokens::panel());
        self.settings(&mut half(ui, left));
        self.shelf(&mut half(ui, right));
    }

    /// The left half: what a settings page is made of.
    fn settings(&mut self, ui: &mut Ui) {
        section_caps(ui, "Layout");
        // Setting rows follow one another with no gap.
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.setting_rows(ui);
        });
        ui.add_space(size::SECTION_GAP);
        section_caps(ui, "Buttons");
        self.buttons(ui);
    }

    fn setting_rows(&mut self, ui: &mut Ui) {
        setting_row(
            ui,
            "Show ribbon labels",
            "Turn off to show icons only and save 20 px of height.",
            |ui| toggle::toggle(ui, &mut self.labels, "Show ribbon labels"),
        );
        setting_row(
            ui,
            "Code beside the viewport",
            "A switch that is off.",
            |ui| toggle::toggle(ui, &mut self.split, "Code beside the viewport"),
        );
        setting_row(
            ui,
            "Density",
            "Compact fits more rows; comfortable is easier to click.",
            |ui| {
                let choices = Density::ALL.map(|density| (density, density.name()));
                segmented::segmented(ui, "Density", &mut self.density, &choices)
            },
        );
        setting_row(
            ui,
            "Accent colour",
            "Used for selection, the active tool and focus.",
            |ui| {
                let choices = Accent::ALL.map(|accent| (accent, accent.colour(), accent.name()));
                swatch::swatches(
                    ui,
                    "Accent",
                    &mut self.accent,
                    &choices,
                    tokens::shelf_bar(),
                )
            },
        );
        setting_row(
            ui,
            "Shelf opens on",
            "Which shelf tab comes to the front when you select something.",
            |ui| {
                dropdown(
                    ui,
                    "Shelf opens on",
                    &mut self.opens_on,
                    &OPENS_ON,
                    size::SETTING_FIELD_W,
                )
            },
        );
    }

    fn buttons(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
            if Button::plain("Open…").header().show(ui).clicked() {
                self.pressed = Some("Open…");
            }
            if Button::primary("New project")
                .header()
                .icon(Icon::Plus)
                .show(ui)
                .clicked()
            {
                self.pressed = Some("New project");
            }
            if Button::light("Play").icon(Icon::Play).show(ui).clicked() {
                self.pressed = Some("Play");
            }
            if button::link(ui, "All templates").clicked() {
                self.pressed = Some("All templates");
            }
        });
        ui.scope(|ui| {
            ui.set_width(260.0);
            if Button::plain("Reset to defaults").wide().show(ui).clicked() {
                self.pressed = Some("Reset to defaults");
            }
        });
        ui.horizontal(|ui| {
            for (key, state) in [
                ("W", KeycapState::Bound),
                ("Left Shift", KeycapState::Bound),
                ("E", KeycapState::Conflict),
                ("Press a key…", KeycapState::Listening),
            ] {
                if keycap(ui, key, state).clicked() {
                    self.pressed = Some(key);
                }
            }
            if pill::pill(ui, "Perspective ▾").clicked() {
                self.pressed = Some("Perspective ▾");
            }
        });
    }

    /// The right half: what a shelf column is made of.
    fn shelf(&mut self, ui: &mut Ui) {
        // The header runs edge to edge, over the half's padding.
        let across = ui.max_rect().expand2(vec2(PAD, 0.0));
        let mut strip = ui.new_child(UiBuilder::new().max_rect(across));
        column_header(&mut strip, "Properties", |ui| {
            if button::icon(ui, Icon::More, "More").clicked() {
                self.pressed = Some("More");
            }
            if button::dashed(ui, "+ Component").clicked() {
                self.pressed = Some("+ Component");
            }
        });
        ui.add_space(size::COLUMN_HEADER_H);
        ui.horizontal(|ui| {
            for (index, (label, dot)) in KINDS.iter().enumerate() {
                if chip(ui, ChipKind::Filter, label, self.kind == index, *dot).clicked() {
                    self.kind = index;
                }
            }
            chip(ui, ChipKind::Quiet, "Recent", false, None);
            chip(ui, ChipKind::Quiet, "Favourites", true, None);
            chip(ui, ChipKind::Large, "Gameplay", true, None);
            chip(ui, ChipKind::Large, "Menus", false, None);
        });
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = size::CARD_GAP;
            let column = |ui: &mut Ui, width: f32, card: &mut dyn FnMut(&mut Ui)| {
                ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(width);
                    card(ui);
                });
            };
            column(ui, CARD_W, &mut |ui| {
                card::card(ui, "Transform", |ui| self.transform(ui));
            });
            column(ui, CARD_W, &mut |ui| {
                card::card(ui, "Sound", |ui| self.sound(ui));
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::TILE_GAP;
            for (name, kind, colour, chosen, icon) in [
                (
                    "Crane_Arm_02",
                    "Model",
                    tokens::KIND_MODEL,
                    false,
                    Icon::Model,
                ),
                (
                    "Dock_Planks_Wet_Long_Name",
                    "Picture",
                    tokens::KIND_PICTURE,
                    true,
                    Icon::Picture,
                ),
                (
                    "Harbour",
                    "Folder",
                    tokens::KIND_FOLDER,
                    false,
                    Icon::Library,
                ),
            ] {
                let said = TileText {
                    name,
                    kind,
                    kind_colour: colour,
                };
                let pressed = tile(
                    ui,
                    size::TILE_MIN_W,
                    said,
                    chosen,
                    tokens::panel(),
                    |p, r| {
                        icons::icon(
                            p,
                            Rect::from_center_size(r.center(), vec2(44.0, 44.0)),
                            icon,
                            colour,
                        );
                    },
                )
                .clicked();
                if pressed {
                    self.pressed = Some(name);
                }
            }
        });
    }

    fn transform(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::ROW_GAP;
            for (value, axis) in self.position.iter_mut().zip([Axis::X, Axis::Y, Axis::Z]) {
                let width = (FIELD_W - 2.0 * size::ROW_GAP) / 3.0;
                number_field(ui, value, Some(axis), 0.1, width);
            }
        });
        text_field(ui, &mut self.name, "Name", FIELD_W);
        text_field(ui, &mut self.search, "Find something to place", FIELD_W);
        ui.spacing_mut().item_spacing.y = size::KEY_VALUE_ROW_GAP;
        key_value(ui, "Triangles", "12,480");
        key_value(ui, "File", "Props/Harbour/Crane_Arm_02.glb");
    }

    fn sound(&mut self, ui: &mut Ui) {
        slider::slider(ui, &mut self.volume, 0.0..=1.0, "Volume");
        checkbox::checkbox(ui, &mut self.shadows, "Casts shadows");
        checkbox::checkbox(ui, &mut self.grid, "Show the grid");
    }
}
