//! The Console panel: the editor's messages, with filters by level and a
//! search box, and the command line under them
//! (its commands are in `console_commands.rs`).

use crate::icons::{self, Icon};
use crate::theme;
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
    /// What's typed in the command line.
    pub(crate) input: String,
    /// Commands run before, oldest first.
    history: Vec<String>,
    /// While Up and Down walk the history: which one is shown.
    history_at: Option<usize>,
    /// What was typed before Up was pressed, for Down to bring back.
    draft: String,
    /// Whether the command line takes the keyboard next frame.
    pub(crate) focus: bool,
    /// Whether the cursor goes to the end of the command line next frame
    /// (after Tab or Up changed it).
    pub(crate) cursor_to_end: bool,
    /// Whether the `'` that opened the command line is still to come (as
    /// typed text, a frame after its key), and isn't to be typed.
    pub(crate) swallow_quote: bool,
    /// The choices Tab found, shown above the command line.
    pub(crate) choices: Vec<String>,
    /// The message clicked, shown in full under the list until it's
    /// clicked again (one row is often too short for a whole message).
    pub(crate) reading: Option<LogLine>,
}

/// What the command line asks the editor to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConsoleRequest {
    /// Run this command line (Enter).
    Run(String),
    /// Complete the command line (Tab).
    Complete,
}

/// The most commands the history keeps.
pub const HISTORY_LIMIT: usize = 100;
/// The longest command line kept in the history.
const LONGEST_COMMAND: usize = 1_000;

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

    /// Commands run before, oldest first, for saving.
    pub(crate) fn history(&self) -> &[String] {
        &self.history
    }

    /// Uses `history` (as saved last time), keeping the newest
    /// [`HISTORY_LIMIT`] and leaving out anything unusable.
    pub(crate) fn set_history(&mut self, history: Vec<String>) {
        self.history = Vec::new();
        for line in history {
            self.remember(&line);
        }
    }

    /// Adds `line` to the history (not twice in a row) and leaves the
    /// history walk.
    pub(crate) fn remember(&mut self, line: &str) {
        self.history_at = None;
        let line = line.trim();
        if line.is_empty() || line.len() > LONGEST_COMMAND || line.contains(['\n', '\r']) {
            return;
        }
        if self.history.last().map(String::as_str) != Some(line) {
            self.history.push(line.to_string());
        }
        if self.history.len() > HISTORY_LIMIT {
            let extra = self.history.len() - HISTORY_LIMIT;
            self.history.drain(..extra);
        }
    }

    /// Up: the command run before the one shown.
    pub(crate) fn older(&mut self) {
        let at = match self.history_at {
            None if self.history.is_empty() => return,
            None => {
                self.draft = self.input.clone();
                self.history.len() - 1
            }
            Some(at) => at.saturating_sub(1),
        };
        self.history_at = Some(at);
        self.input = self.history[at].clone();
        self.cursor_to_end = true;
    }

    /// Down: the command run after the one shown, or what was being typed.
    pub(crate) fn newer(&mut self) {
        let Some(at) = self.history_at else {
            return;
        };
        if at + 1 < self.history.len() {
            self.history_at = Some(at + 1);
            self.input = self.history[at + 1].clone();
        } else {
            self.history_at = None;
            self.input = std::mem::take(&mut self.draft);
        }
        self.cursor_to_end = true;
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

/// The console: filters by level, a search box, the messages with their
/// times, and the command line. Returns what the command line asks for.
pub(crate) fn console(ui: &mut Ui, console: &mut Console) -> Option<ConsoleRequest> {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for level in LogLevel::ALL {
                let on = !console.hidden.contains(&level);
                if level_filter(ui, level, console.count(level), on).clicked() {
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
    // The command line along the bottom; the messages fill the rest.
    let request = egui::Panel::bottom("console command row")
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| command_line(ui, console))
        .inner;
    // The message clicked, in full, above the command line.
    if let Some(line) = console.reading.clone() {
        egui::Panel::bottom("console reading")
            .resizable(true)
            .default_size(72.0)
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 4)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&line.time)
                            .monospace()
                            .color(theme::TEXT_TIME),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Close").clicked() {
                            console.reading = None;
                        }
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(line.text.clone());
                        }
                    });
                });
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&line.text)
                                    .monospace()
                                    .color(line.level.color()),
                            )
                            .wrap()
                            .selectable(true),
                        );
                    });
            });
    }
    let shown: Vec<&LogLine> = console.shown().collect();
    let mut clicked = None;
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show_rows(ui, CONSOLE_ROW, shown.len(), |ui, rows| {
                    for line in &shown[rows] {
                        let chosen = console.reading.as_ref() == Some(*line);
                        if console_line(ui, line, chosen).clicked() {
                            clicked = Some((*line).clone());
                        }
                    }
                });
        });
    if let Some(line) = clicked {
        console.reading = if console.reading.as_ref() == Some(&line) {
            None
        } else {
            Some(line)
        };
    }
    request
}

/// The command line's widget ID.
pub(crate) fn command_line_id() -> egui::Id {
    egui::Id::new("console command line")
}

/// The command line: `›`, the text box, and Tab's choices above it. Enter
/// runs it, Up and Down walk the history, Tab completes.
fn command_line(ui: &mut Ui, console: &mut Console) -> Option<ConsoleRequest> {
    let id = command_line_id();
    let mut request = None;
    if console.swallow_quote {
        ui.input_mut(|i| {
            if let Some(at) = i
                .events
                .iter()
                .position(|e| matches!(e, egui::Event::Text(_)))
            {
                if matches!(&i.events[at], egui::Event::Text(t) if t == "'") {
                    i.events.remove(at);
                }
                console.swallow_quote = false;
            }
        });
    }
    if ui.memory(|m| m.has_focus(id)) {
        // Taken before the text box sees them: Tab would move the
        // keyboard away, Up and Down would do nothing.
        let (tab, up, down) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Tab),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            )
        });
        if tab {
            request = Some(ConsoleRequest::Complete);
        }
        if up {
            console.older();
        }
        if down {
            console.newer();
        }
    }
    if !console.choices.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(console.choices.join("   "))
                        .monospace()
                        .color(theme::TEXT_WEAK),
                )
                .truncate(),
            );
        });
    }
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new("›").monospace().color(theme::TEXT_WEAK));
            let output = egui::TextEdit::singleline(&mut console.input)
                .id(id)
                .font(egui::TextStyle::Monospace)
                .hint_text("Type a command (help lists them); ' jumps here")
                .desired_width(ui.available_width())
                .show(ui);
            let response = output.response;
            if response.changed() {
                console.choices.clear();
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let line = std::mem::take(&mut console.input);
                console.choices.clear();
                if !line.trim().is_empty() {
                    console.remember(&line);
                    request = Some(ConsoleRequest::Run(line));
                }
                // Stay in the command line for the next one.
                console.focus = true;
            }
            if std::mem::take(&mut console.focus) {
                response.request_focus();
            }
            if std::mem::take(&mut console.cursor_to_end) {
                let mut state = output.state;
                let end = egui::text::CCursor::new(console.input.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
            }
        });
    });
    request
}

/// The height of one console row, as in the mockup.
pub(crate) const CONSOLE_ROW: f32 = 18.0;

/// One level's filter button: its icon and count, filled with the level's
/// own colour while that level is shown (decision 0060: grey, amber, red,
/// whatever the accent).
pub(crate) fn level_filter(ui: &mut Ui, level: LogLevel, count: usize, on: bool) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let (text_color, fill) = if on {
        let fill = match level {
            LogLevel::Info => theme::level::INFO,
            LogLevel::Warning => theme::level::WARNING,
            LogLevel::Error => theme::level::ERROR,
        };
        (Color32::WHITE, fill)
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
/// Clickable (to read it in full), and marked when it's the one being
/// read.
fn console_line(ui: &mut Ui, line: &LogLine, chosen: bool) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), CONSOLE_ROW),
        egui::Sense::click(),
    );
    if chosen || response.hovered() {
        ui.painter().rect_filled(
            row,
            2.0,
            ui.visuals()
                .widgets
                .hovered
                .weak_bg_fill
                .gamma_multiply(if chosen { 1.0 } else { 0.5 }),
        );
    }
    let response = response.on_hover_text("Click to read it in full");
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
        .truncate()
        .selectable(false),
    );
    response
}
