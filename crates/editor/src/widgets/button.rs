//! The design's buttons: plain, primary (the accent), light (Play's),
//! dashed ("+ Component"), an icon alone, and a link.

use super::{border, dashed, focus_ring, width_of, words};
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, Rect, Response, Sense, Ui, WidgetInfo, vec2,
};

/// What a button looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Primary,
    Light,
}

/// A button with a word on it: [`Button::plain`], [`Button::primary`] or
/// [`Button::light`], then [`Button::show`].
#[derive(Debug, Clone, Copy)]
pub struct Button<'a> {
    label: &'a str,
    kind: Kind,
    icon: Option<Icon>,
    header: bool,
    wide: bool,
    after: bool,
}

impl<'a> Button<'a> {
    fn new(label: &'a str, kind: Kind) -> Self {
        Self {
            label,
            kind,
            icon: None,
            header: false,
            wide: false,
            after: false,
        }
    }

    /// An ordinary button: a grey fill with a border.
    pub fn plain(label: &'a str) -> Self {
        Self::new(label, Kind::Plain)
    }

    /// The main thing to do on a page: filled with the accent.
    pub fn primary(label: &'a str) -> Self {
        Self::new(label, Kind::Primary)
    }

    /// The light button: Play's look, the one light thing on screen.
    pub fn light(label: &'a str) -> Self {
        Self::new(label, Kind::Light)
    }

    /// With an icon before its word.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// With an icon after its word: "Open the manual ↗".
    pub fn icon_after(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self.after = true;
        self
    }

    /// As tall as the buttons in a File page's header (32 px, not 30).
    pub fn header(mut self) -> Self {
        self.header = true;
        self
    }

    /// As wide as the room it is given.
    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }

    /// Draws the button.
    pub fn show(self, ui: &mut Ui) -> Response {
        let (height, radius) = if self.header {
            (size::PAGE_BUTTON_H + 2.0, size::radius::MENU)
        } else {
            (size::PAGE_BUTTON_H, size::radius::TAB)
        };
        let font = if self.kind == Kind::Plain {
            text::tab_active()
        } else {
            text::semibold_at(text::UI_SIZE)
        };
        let icon_w = self
            .icon
            .map_or(0.0, |_| size::ICON_SMALL + size::CARD_INNER_GAP);
        let content = icon_w + width_of(ui, self.label, font.clone());
        let width = if self.wide {
            ui.available_width()
        } else {
            content + 2.0 * size::PAGE_BUTTON_PAD
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        response.widget_info(|| {
            WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), self.label)
        });
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let accent = interface_look::accent_fill();
        let (fill, ink) = match self.kind {
            Kind::Plain if response.is_pointer_button_down_on() => {
                (tokens::field_border(), tokens::text_primary())
            }
            Kind::Plain if response.hovered() => (tokens::pressed(), tokens::text_primary()),
            Kind::Plain => (tokens::line(), tokens::text_primary()),
            Kind::Primary => (lifted(accent, &response), tokens::ON_ACCENT),
            Kind::Light => (lifted(tokens::PLAY_FILL, &response), tokens::ON_ACCENT),
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(radius), fill);
        if self.kind == Kind::Plain {
            border(ui, rect, radius, tokens::strong_line());
        }
        let mut left = rect.center().x - content / 2.0;
        let icon_left = if self.after {
            left + content - size::ICON_SMALL
        } else {
            left
        };
        if let Some(icon) = self.icon {
            let spot = Rect::from_center_size(
                egui::pos2(icon_left + size::ICON_SMALL / 2.0, rect.center().y),
                vec2(size::ICON_SMALL, size::ICON_SMALL),
            );
            icons::icon(ui.painter(), spot, icon, ink);
            if !self.after {
                left += icon_w;
            }
        }
        let room = Rect::from_min_max(egui::pos2(left, rect.top()), rect.right_bottom());
        words(ui, room, Align2::LEFT_CENTER, self.label, font, ink);
        focus_ring(ui, &response, rect, radius);
        response
    }
}

/// A filled button's colour under the pointer: a touch lighter.
fn lifted(fill: Color32, response: &Response) -> Color32 {
    if response.hovered() {
        tokens::mix(tokens::text_emphasis(), fill, tokens::share::LIFT)
    } else {
        fill
    }
}

/// A dashed "add" button, such as "+ Component": an outline of dashes
/// round quiet words.
pub fn dashed(ui: &mut Ui, label: &str) -> Response {
    dashed_as(ui, label, label)
}

/// A dashed button showing `label` whose name, for screen readers and
/// tests, is `name` ("Add component" for "+ Component").
pub fn dashed_as(ui: &mut Ui, label: &str, name: &str) -> Response {
    let font = text::medium_at(text::SMALL_SIZE);
    let width = width_of(ui, label, font.clone()) + 2.0 * size::DASHED_BUTTON_PAD;
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, size::DASHED_BUTTON_H), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    if ui.is_rect_visible(rect) {
        let ink = if response.hovered() {
            tokens::text_primary()
        } else {
            tokens::text_label()
        };
        dashed::rounded(ui, rect, size::radius::CONTROL, tokens::stronger_line());
        words(ui, rect, Align2::CENTER_CENTER, label, font, ink);
        focus_ring(ui, &response, rect, size::radius::CONTROL);
    }
    response
}

/// A button that is an icon alone, 26 × 26. `name` is what it does: a
/// screen reader says it, and it shows on hover.
pub fn icon(ui: &mut Ui, icon: Icon, name: &str) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(size::ICON_BUTTON, size::ICON_BUTTON), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    if ui.is_rect_visible(rect) {
        let ink = if response.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(size::radius::CONTROL),
                tokens::line(),
            );
            tokens::text_primary()
        } else {
            tokens::text_label()
        };
        let spot = Rect::from_center_size(rect.center(), vec2(size::ICON_SMALL, size::ICON_SMALL));
        icons::icon(ui.painter(), spot, icon, ink);
        focus_ring(ui, &response, rect, size::radius::CONTROL);
    }
    response.on_hover_text(name)
}

/// Words that are a link: the link's blue, lighter under the pointer.
pub fn link(ui: &mut Ui, label: &str) -> Response {
    link_at(ui, label, text::ui())
}

/// A link in `font`, where the words around it aren't interface text's
/// size.
pub fn link_at(ui: &mut Ui, label: &str, font: egui::FontId) -> Response {
    let width = width_of(ui, label, font.clone());
    let height = ui.text_style_height(&egui::TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Link, ui.is_enabled(), label));
    if ui.is_rect_visible(rect) {
        let ink = if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            tokens::LINK_HOVER
        } else {
            tokens::LINK
        };
        words(ui, rect, Align2::LEFT_CENTER, label, font, ink);
        focus_ring(ui, &response, rect.expand(2.0), size::radius::GRIP);
    }
    response
}
