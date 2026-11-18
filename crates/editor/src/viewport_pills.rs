//! The pills floating over each view (spec 182; the approved additions,
//! round 2, `02-viewport-pills*.png`):
//!
//! - top left, on every view: the view (Perspective ▾, with how many views
//!   under it), the look (Standard ▾) and what the viewport shows (Show ▾);
//! - top right, on the view in use: the frame time and frame rate;
//! - bottom right, on the view in use: the axis locks, World or Object and
//!   the camera's speed; in the 2D view the X and Y locks, what moves snap
//!   to, the tile grid and the zoom (`viewport_pixel_pills.rs`).
//!
//! Snapping's on and off and its steps are the ribbon's (Home › Snapping).

use super::{ViewLayout, Viewport};
use crate::camera::ViewKind;
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::look::{Look, LookKind};
use crate::tokens::{self, size, text};
use crate::transforming::Snapping;
use crate::widgets::{self, menu, pill::pill};
use egui::{Align, Align2, CornerRadius, Layout, Rect, Response, Sense, Ui, UiBuilder, vec2};

impl Viewport {
    /// The top-left pills of view `rect`; `several` while more than one
    /// view shows. Returns where they are.
    pub(super) fn top_left_pills(&mut self, ui: &mut Ui, rect: Rect, several: bool) -> Rect {
        let mut row = corner(ui, rect, "view pills", Layout::left_to_right(Align::Min));
        row.spacing_mut().item_spacing.x = size::PILL_GAP;
        self.view_pill(&mut row, several);
        // How models are shaded: 3D only (the kind table).
        if crate::kinds::shown(crate::kinds::setting(crate::file_pages::Setting::Look)) {
            let look = &mut self.look;
            let shown = format!("{} ▾", look.kind.name());
            let response = pill(&mut row, &shown);
            drop_down(&response, |ui| look_menu(ui, look));
        }
        let response = pill(&mut row, "Show ▾");
        let overlays = &mut self.overlays;
        drop_down(&response, |ui| {
            menu::caps(ui, "Show in the viewport");
            menu::check(ui, &mut overlays.stats, "Frame time and frame rate");
            menu::check(ui, &mut overlays.snapping, "Axis locks and camera speed");
            menu::check(ui, &mut overlays.tools, "Tools");
            menu::line(ui);
            menu::caps(ui, "While you edit");
            menu::check(ui, &mut overlays.preview_animation, "Preview animation").on_hover_text(
                "Sprites play their clips while you edit. Off: each shows its clip's frame at \
                 its Clip time, and the editor rests. The game always animates.",
            );
        });
        row.min_rect()
    }

    /// The view's pill: which view this is, and how many views there are.
    fn view_pill(&mut self, ui: &mut Ui, several: bool) {
        let kind = self.camera.kind;
        // The 2D view says its zoom too (board 08): "2D · 200% ▾".
        let words = if kind == ViewKind::TwoD {
            format!("{} \u{b7} {:.0}% ▾", kind.name(), self.camera.zoom * 100.0)
        } else {
            format!("{} ▾", kind.name())
        };
        let response = pill(ui, &words);
        let mut chosen = None;
        let mut layout = None;
        let alone = self.maximized == Some(self.active);
        let mut toggle_alone = false;
        let current = self.layout;
        drop_down(&response, |ui| {
            // Only the views the project's kind keeps (the kind table).
            let views = ViewKind::ALL
                .into_iter()
                .filter(|each| crate::kinds::shown(crate::kinds::view(*each)));
            for each in views {
                if each == ViewKind::TwoD && kind_has_3d() {
                    menu::line(ui);
                }
                let label = match each {
                    ViewKind::TwoD => "2D  flat, in pixels",
                    _ => each.name(),
                };
                if menu::choice(ui, label, each == kind)
                    .on_hover_text(view_hint(each))
                    .clicked()
                {
                    chosen = Some(each);
                    ui.close();
                }
            }
            menu::line(ui);
            menu::caps(ui, "Views");
            for each in ViewLayout::ALL {
                if menu::choice(ui, each.name(), current == each).clicked() {
                    layout = Some(each);
                    ui.close();
                }
            }
            if several || alone {
                menu::line(ui);
                let (words, hint) = if alone {
                    ("All views", "Show every view again")
                } else {
                    ("Maximise", "Show this view alone, filling the viewport")
                };
                if menu::item(ui, words, "").on_hover_text(hint).clicked() {
                    toggle_alone = true;
                    ui.close();
                }
            }
        });
        if let Some(kind) = chosen {
            self.camera = self.camera.as_kind(kind);
        }
        if let Some(layout) = layout {
            self.layout = layout;
            self.maximized = None;
        }
        if toggle_alone {
            self.maximized = if alone { None } else { Some(self.active) };
        }
    }

    /// The frame time and frame rate, top right; the counts on hover.
    pub(super) fn stats_pill(&self, ui: &mut Ui, rect: Rect) -> Rect {
        let mut row = corner(ui, rect, "stats pill", Layout::right_to_left(Align::Min));
        let words = if self.idle {
            // Nothing moves, so nothing is drawn until something does.
            "Idle".to_owned()
        } else {
            format!(
                "{:.1} ms · {:.0} fps",
                self.frame_time * 1000.0,
                self.frames_per_second()
            )
        };
        let s = if self.entities == 1 { "y" } else { "ies" };
        let mut counts = format!("{} entit{s}", crate::models::grouped(self.entities));
        if self.level.triangles > 0 {
            counts.push_str(&format!(
                " · {} triangles",
                crate::models::grouped(self.level.triangles)
            ));
        }
        mono_pill(&mut row, &words).on_hover_text(counts);
        row.min_rect()
    }

    /// The bottom-right pills of the 3D views: the axis locks, World or
    /// Object, and the camera's speed.
    pub(super) fn space_pills(&mut self, ui: &mut Ui, rect: Rect) -> Rect {
        let mut row = corner(ui, rect, "space pills", Layout::right_to_left(Align::Max));
        row.spacing_mut().item_spacing.x = size::PILL_GAP;
        camera_pill(&mut row, self.camera.speed);
        let snapping = &mut self.snapping;
        let space = if snapping.object_axes {
            "Object ▾"
        } else {
            "World ▾"
        };
        let response = pill(&mut row, space)
            .on_hover_text("The gizmo follows the world's axes or the entity's own (L)");
        drop_down(&response, |ui| {
            for (object, words) in [(false, "World"), (true, "Object")] {
                if menu::choice_with(ui, words, "L", snapping.object_axes == object).clicked() {
                    snapping.object_axes = object;
                    ui.close();
                }
            }
        });
        locks_pill(&mut row, snapping, 3);
        row.min_rect()
    }
}

/// Whether the project's kind keeps the 3D views: the 2D view is set
/// apart under a line only when they're there.
fn kind_has_3d() -> bool {
    crate::kinds::shown(crate::kinds::view(ViewKind::Perspective))
}

/// A child of `ui` filling the view's `rect` less the pills' margin, laid
/// out from the corner `layout` starts in.
fn corner(ui: &mut Ui, rect: Rect, salt: &str, layout: Layout) -> Ui {
    let inner = rect.shrink(size::PILL_MARGIN);
    ui.new_child(
        UiBuilder::new()
            .id_salt(("viewport pills", salt))
            .max_rect(inner)
            .layout(layout),
    )
}

/// A pill's list, under it, staying open until a choice is made or a
/// click lands elsewhere.
pub(super) fn drop_down(response: &Response, contents: impl FnOnce(&mut Ui)) {
    egui::Popup::menu(response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.set_min_width(size::PILL_MENU_W);
            contents(ui);
        });
}

/// A pill in the mono font: numbers that change ("6.9 ms · 144 fps").
pub(super) fn mono_pill(ui: &mut Ui, words: &str) -> Response {
    let font = text::mono_small();
    let width = widgets::width_of(ui, words, font.clone()) + 2.0 * size::PILL_PAD;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::PILL_H), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, words));
    if ui.is_rect_visible(rect) {
        pill_box(ui, rect);
        widgets::words(
            ui,
            rect,
            Align2::CENTER_CENTER,
            words,
            font,
            tokens::text_body(),
        );
    }
    response
}

/// The camera's speed: its icon and the bare number.
fn camera_pill(ui: &mut Ui, speed: u8) {
    let number = speed.to_string();
    let font = text::ribbon_small();
    let width = size::ICON_SMALL
        + size::PILL_ICON_GAP
        + widgets::width_of(ui, &number, font.clone())
        + 2.0 * size::PILL_PAD;
    let (rect, response) = ui.allocate_exact_size(vec2(width, size::PILL_H), Sense::hover());
    let name = format!("Camera speed {number}");
    response
        .on_hover_text("Camera speed: hold the right mouse button and turn the wheel")
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &name));
    if ui.is_rect_visible(rect) {
        pill_box(ui, rect);
        let room = rect.shrink2(vec2(size::PILL_PAD, 0.0));
        let spot = Rect::from_min_size(
            egui::pos2(room.left(), room.center().y - size::ICON_SMALL / 2.0),
            vec2(size::ICON_SMALL, size::ICON_SMALL),
        );
        icons::icon(ui.painter(), spot, Icon::Camera, tokens::text_label());
        widgets::words(
            ui,
            room,
            Align2::RIGHT_CENTER,
            &number,
            font,
            tokens::text_body(),
        );
    }
}

/// The width of the locks' pill with `axes` axes.
pub(super) fn locks_width(ui: &Ui, axes: usize) -> f32 {
    let word = widgets::width_of(ui, "Lock", text::ribbon_small());
    2.0 * size::PILL_PAD
        + word
        + size::PILL_ICON_GAP
        + axes as f32 * (size::LOCK_BUTTON + size::LOCK_GAP)
}

/// "Lock" and the first `axes` of X, Y and Z, each a small button lit
/// while its axis is locked.
pub(super) fn locks_pill(ui: &mut Ui, snapping: &mut Snapping, axes: usize) {
    let font = text::ribbon_small();
    let word = widgets::width_of(ui, "Lock", font.clone());
    let width = locks_width(ui, axes);
    let (rect, _) = ui.allocate_exact_size(vec2(width, size::PILL_H), Sense::hover());
    pill_box(ui, rect);
    let room = rect.shrink2(vec2(size::PILL_PAD, 0.0));
    widgets::words(
        ui,
        room,
        Align2::LEFT_CENTER,
        "Lock",
        font.clone(),
        tokens::text_meta(),
    );
    let mut x = room.left() + word + size::PILL_ICON_GAP;
    for (axis, name) in ["X", "Y", "Z"].into_iter().enumerate().take(axes) {
        let spot = Rect::from_min_size(
            egui::pos2(x, rect.center().y - size::LOCK_BUTTON / 2.0),
            vec2(size::LOCK_BUTTON, size::LOCK_BUTTON),
        );
        x += size::LOCK_BUTTON + size::LOCK_GAP;
        let id = ui.id().with(("lock", axis));
        let response = ui.interact(spot, id, Sense::click());
        let locked = &mut snapping.locked[axis];
        let label = format!("Lock {name}");
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, *locked, &label)
        });
        let response =
            response.on_hover_text(format!("Lock {name}: drags leave it alone ({name})"));
        if response.clicked() {
            *locked = !*locked;
        }
        let radius = CornerRadius::same(size::radius::SMALL);
        let ink = if *locked {
            ui.painter()
                .rect_filled(spot, radius, interface_look::accent_fill());
            tokens::app()
        } else if response.hovered() {
            ui.painter().rect_filled(spot, radius, tokens::hover());
            tokens::text_primary()
        } else {
            tokens::text_body()
        };
        widgets::words(ui, spot, Align2::CENTER_CENTER, name, font.clone(), ink);
    }
}

/// A pill's fill and border.
fn pill_box(ui: &Ui, rect: Rect) {
    let radius = size::radius::CONTROL;
    ui.painter()
        .rect_filled(rect, CornerRadius::same(radius), tokens::pill_fill());
    widgets::border(ui, rect, radius, tokens::pill_border());
}

/// "whole pixels" as "Whole pixels".
pub(super) fn sentence_case(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// What each view's row says on hover.
fn view_hint(kind: ViewKind) -> &'static str {
    match kind {
        ViewKind::Perspective => "As the eye sees: fly with the right mouse button",
        ViewKind::Top => "Flat, from above: right or middle drag slides, the wheel zooms",
        ViewKind::Front => "Flat, from the front (looking north)",
        ViewKind::Right => "Flat, from the right (looking west)",
        ViewKind::TwoD => {
            "Flat, in pixels: for 2D games. Slides like the other flat views; the wheel zooms \
             in whole steps, so pixels stay sharp"
        }
    }
}

/// The look's list: the three looks, and the stylized look's settings.
fn look_menu(ui: &mut Ui, look: &mut Look) {
    for kind in LookKind::ALL {
        if menu::choice(ui, kind.name(), look.kind == kind)
            .on_hover_text(kind.hint())
            .clicked()
        {
            look.kind = kind;
        }
    }
    menu::line(ui);
    menu::caps(ui, "Stylized look");
    let stylized = look.kind == LookKind::Stylized;
    ui.add_enabled_ui(stylized, |ui| {
        let bands = look.bands.to_string();
        let response = menu::item(ui, "Bands of light", &bands)
            .on_hover_text("A click takes the next number of bands");
        if response.clicked() {
            look.bands = if look.bands >= *Look::BANDS.end() {
                *Look::BANDS.start()
            } else {
                look.bands + 1
            };
        }
        menu::check(ui, &mut look.outlines, "Outlines");
    })
    .response
    .on_disabled_hover_text("Choose Stylized to change these");
}
