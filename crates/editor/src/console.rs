//! The Console panel: the editor's messages, with filters by level and a
//! search box.

use crate::icons::{self, Icon};
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, RichText, Ui};
use std::collections::HashSet;

/// How serious a console message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Something happened.
    Info,
    /// Something works, but not quite as asked.
    Warning,
    /// Something failed.
    Error,
}

impl LogLevel {
    /// Every level, in the console's filter order.
    pub const ALL: [Self; 3] = [Self::Info, Self::Warning, Self::Error];

    /// Its place in `ALL`.
    fn slot(self) -> usize {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Error => 2,
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Info => Icon::Info,
            Self::Warning => Icon::Warning,
            Self::Error => Icon::Error,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Info => theme::TEXT_CONSOLE,
            Self::Warning => theme::WARNING,
            Self::Error => theme::ERROR,
        }
    }
}

/// One console message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// When it was written, local time, such as "12:04:31".
    pub time: String,
    /// How serious it is.
    pub level: LogLevel,
    /// What it says.
    pub text: String,
}

/// The console's messages and which levels are shown.
#[derive(Debug, Clone, Default)]
pub(crate) struct Console {
    lines: Vec<LogLine>,
    /// Each line's text in lower case, for searching without making new
    /// text every frame.
    lowered: Vec<String>,
    /// How many lines there are of each level (in `LogLevel::ALL` order).
    counts: [usize; 3],
    pub(crate) hidden: HashSet<LogLevel>,
    pub(crate) search: String,
}

/// The most messages the console keeps; older ones are dropped.
pub(crate) const CONSOLE_LIMIT: usize = 2_000;
/// How many of the oldest messages are dropped at once when the console is
/// full, so a full console doesn't shift 2,000 messages for every new one.
const DROP_AT_ONCE: usize = 250;

impl Console {
    pub(crate) fn push(&mut self, level: LogLevel, text: String) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.lowered.push(text.to_lowercase());
        self.lines.push(LogLine { time, level, text });
        self.counts[level.slot()] += 1;
        if self.lines.len() > CONSOLE_LIMIT {
            let extra = self.lines.len() - CONSOLE_LIMIT + DROP_AT_ONCE;
            for gone in self.lines.drain(..extra) {
                self.counts[gone.level.slot()] -= 1;
            }
            self.lowered.drain(..extra);
        }
    }

    /// Every message, oldest first.
    pub(crate) fn lines(&self) -> &[LogLine] {
        &self.lines
    }

    pub(crate) fn clear(&mut self) {
        self.lines.clear();
        self.lowered.clear();
        self.counts = [0; 3];
    }

    pub(crate) fn count(&self, level: LogLevel) -> usize {
        self.counts[level.slot()]
    }

    pub(crate) fn shown(&self) -> impl Iterator<Item = &LogLine> {
        let search = self.search.to_lowercase();
        self.lines
            .iter()
            .zip(&self.lowered)
            .filter(move |(l, lowered)| {
                !self.hidden.contains(&l.level) && (search.is_empty() || lowered.contains(&search))
            })
            .map(|(l, _)| l)
    }
}

/// The console: filters by level, a search box, and the messages with
/// their times.
pub(crate) fn console(ui: &mut Ui, console: &mut Console, accent: Accent) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for level in LogLevel::ALL {
                let on = !console.hidden.contains(&level);
                if level_filter(ui, level, console.count(level), on, accent).clicked() {
                    if on {
                        console.hidden.insert(level);
                    } else {
                        console.hidden.remove(&level);
                    }
                }
            }
            ui.add_space(4.0);
            // Clear at the right edge, and the search box takes the width
            // that is left.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(egui::Button::new("Clear").min_size(egui::vec2(0.0, 20.0)))
                    .clicked()
                {
                    console.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut console.search)
                        .hint_text("Search")
                        .desired_width(ui.available_width().max(40.0)),
                );
            });
        });
    });
    // Only the rows in view are laid out, so a full console (2,000
    // messages) costs no more per frame than a short one.
    let shown: Vec<&LogLine> = console.shown().collect();
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show_rows(ui, CONSOLE_ROW, shown.len(), |ui, rows| {
                    for line in &shown[rows] {
                        console_line(ui, line);
                    }
                });
        });
}

/// The height of one console row, as in the mockup.
pub(crate) const CONSOLE_ROW: f32 = 18.0;

/// One level's filter button: its icon and count, filled with the accent
/// while that level is shown.
pub(crate) fn level_filter(
    ui: &mut Ui,
    level: LogLevel,
    count: usize,
    on: bool,
    accent: Accent,
) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let (text_color, fill) = if on {
        (Color32::WHITE, accent.colors().fill)
    } else {
        (theme::TEXT_WEAK, theme::MENU_ROW)
    };
    let number = ui
        .painter()
        .layout_no_wrap(count.to_string(), font, text_color);
    let icon_size = 11.0;
    let size = egui::vec2(8.0 + icon_size + 5.0 + number.size().x + 8.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_text(format!(
        "{}: {} (click to {} them)",
        level.name(),
        count,
        if on { "hide" } else { "show" }
    ));
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, level.name())
    });
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, fill);
    if !on {
        painter.rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, theme::BUTTON_BORDER),
            egui::StrokeKind::Inside,
        );
    }
    icons::icon(
        painter,
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        ),
        level.icon(),
        text_color,
    );
    painter.galley(
        egui::pos2(
            rect.left() + 8.0 + icon_size + 5.0,
            rect.center().y - number.size().y / 2.0,
        ),
        number,
        text_color,
    );
    response
}

/// One console message: the time, the level's icon and the text.
pub(crate) fn console_line(ui: &mut Ui, line: &LogLine) {
    let (row, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), CONSOLE_ROW),
        egui::Sense::hover(),
    );
    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut row_ui;
    ui.label(
        RichText::new(&line.time)
            .monospace()
            .color(theme::TEXT_TIME),
    );
    let size = 11.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    icons::icon(ui.painter(), rect, line.level.icon(), line.level.color());
    // One row per message: a long one is cut short, and shows in full when
    // pointed at.
    ui.add(
        egui::Label::new(
            RichText::new(&line.text)
                .monospace()
                .color(line.level.color()),
        )
        .truncate(),
    );
}
