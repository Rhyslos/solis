//! The Audio tab's Sound events shelf tab (Screen 10, spec 796-806;
//! round 3, `08-audio-mixer.png`): every sound event in the project as a
//! table (its name, its bus, its volume, where it's heard, how many
//! takes), and beside it the one picked, edited in place as in the
//! Library's Details (`sound_event_view::event_properties`), with Play.
//!
//! The events are the ones the app has read (`Sounds::events`): nothing
//! here reads a file. They are listed by path, so the order never moves.

use crate::library::{LibraryItem, LibraryRequest, Sounds};
use crate::mixer_view::{MixerRequest, MixerState};
use crate::sound_view::EventState;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Align2, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

/// The table's columns: their words and their share of its width.
const COLUMNS: [(&str, f32); 5] = [
    ("Event", 0.36),
    ("Bus", 0.18),
    ("Volume", 0.12),
    ("Placement", 0.2),
    ("Takes", 0.14),
];

/// The tab: the table, and the event picked beside it. Returns what was
/// asked of the mixer, and of the Library (the event's edits and Play).
pub(crate) fn sound_events(
    ui: &mut Ui,
    state: &MixerState,
    sounds: Sounds<'_>,
    library: &[LibraryItem],
) -> (Vec<MixerRequest>, Vec<LibraryRequest>) {
    let whole = ui.max_rect();
    let mut events: Vec<&String> = sounds.events.keys().collect();
    events.sort();
    if events.is_empty() {
        crate::panels::empty_note(
            ui,
            "No sound events yet. Make one in the Library (+ New \u{203a} Sound event), then \
             drop sound files on it.",
        );
        return (Vec::new(), Vec::new());
    }
    let picked = state
        .event
        .as_ref()
        .filter(|path| sounds.events.contains_key(*path))
        .unwrap_or(events[0])
        .clone();
    let split = (whole.left() + whole.width() * (1.0 - size::EVENT_DETAIL_SHARE)).round();
    let (table_rect, detail_rect) = (whole.with_max_x(split), whole.with_min_x(split + 1.0));
    tokens::vline(ui.painter(), split, whole.y_range(), tokens::line());
    let mut table_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(table_rect)
            .id_salt("events table"),
    );
    let mixer = table(&mut table_ui, &events, &picked, sounds);
    let mut detail_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(detail_rect)
            .id_salt("event detail"),
    );
    detail_ui.set_clip_rect(detail_rect.intersect(ui.clip_rect()));
    let library_asks =
        crate::sound_event_view::event_properties(&mut detail_ui, &picked, sounds, library);
    (mixer, library_asks)
}

/// The table: its header of small capitals, then a row an event.
fn table(ui: &mut Ui, events: &[&String], picked: &str, sounds: Sounds<'_>) -> Vec<MixerRequest> {
    let mut out = Vec::new();
    let whole = ui.max_rect();
    let header = whole.with_max_y(whole.top() + size::TABLE_HEADER_H);
    ui.painter().rect_filled(header, 0.0, tokens::shelf_bar());
    tokens::hline(
        ui.painter(),
        header.x_range(),
        header.bottom() - 1.0,
        tokens::line(),
    );
    for (word, rect) in COLUMNS.iter().map(|(w, _)| *w).zip(cells(header)) {
        let job = text::caps(word, text::SECTION_CAPS, tokens::text_meta());
        let galley = ui.painter().layout_job(job);
        let at = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, tokens::text_meta());
    }
    let body = whole.with_min_y(header.bottom());
    let mut rows = ui.new_child(UiBuilder::new().max_rect(body).id_salt("event rows"));
    egui::ScrollArea::vertical()
        .id_salt("sound events")
        .auto_shrink(false)
        .show_rows(&mut rows, size::TABLE_ROW_H, events.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for path in &events[range] {
                if event_row(ui, path, *path == picked, sounds) {
                    out.push(MixerRequest::PickEvent((*path).clone()));
                }
            }
        });
    out
}

/// Where each column's words go in a row `rect`.
fn cells(rect: Rect) -> Vec<Rect> {
    let inside = rect.shrink2(vec2(size::ROW_PAD, 0.0));
    let mut x = inside.left();
    COLUMNS
        .iter()
        .map(|(_, share)| {
            let width = inside.width() * share;
            let cell = Rect::from_x_y_ranges(x..=x + width - size::GAP, rect.y_range());
            x += width;
            cell
        })
        .collect()
}

/// One event's row. Returns whether it was clicked.
fn event_row(ui: &mut Ui, path: &str, picked: bool, sounds: Sounds<'_>) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::TABLE_ROW_H),
        Sense::click(),
    );
    let name = crate::sprite_sheets::name_of(path);
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, picked, &name)
    });
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }
    let accent = crate::interface_look::in_use().accent;
    if picked {
        ui.painter()
            .rect_filled(rect, 0.0, tokens::select_row(accent));
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, tokens::row_hover());
    }
    tokens::hline(
        ui.painter(),
        rect.x_range(),
        rect.bottom() - 1.0,
        tokens::line(),
    );
    let words = row_words(&name, sounds.events.get(path));
    let ink = if picked {
        tokens::text_emphasis()
    } else {
        tokens::text_body()
    };
    let playing = crate::sound_view::played(path, sounds.preview).is_some();
    for (index, (word, cell)) in words.iter().zip(cells(rect)).enumerate() {
        let (font, ink) = match index {
            0 => (
                text::row(),
                if picked { ink } else { tokens::text_primary() },
            ),
            2 => (text::mono(), ink),
            _ => (text::row(), if picked { ink } else { tokens::text_meta() }),
        };
        if index == 0 && playing {
            // A dot before the name while the event is being heard.
            ui.painter().circle_filled(
                pos2(cell.left() - size::ROW_PAD / 2.0, cell.center().y),
                size::STATUS_DOT / 2.0,
                tokens::SUCCESS,
            );
        }
        widgets::words(ui, cell, Align2::LEFT_CENTER, word, font, ink);
    }
    response.on_hover_text(path).clicked()
}

/// What a row says: the name, then the bus, volume, placement and takes
/// as the app read them (or why it couldn't).
fn row_words(name: &str, state: Option<&EventState>) -> [String; 5] {
    match state {
        Some(EventState::Read { event, .. }) => [
            name.to_owned(),
            event.bus.clone(),
            format!("{:.0} dB", event.volume_db).replace('-', "\u{2212}"),
            event.space.name().to_owned(),
            format!("\u{d7}{}", event.takes.len()),
        ],
        Some(EventState::Failed(_)) => [
            name.to_owned(),
            "can't be read".to_owned(),
            String::new(),
            String::new(),
            String::new(),
        ],
        None => [
            name.to_owned(),
            "reading\u{2026}".to_owned(),
            String::new(),
            String::new(),
            String::new(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_says_what_the_event_is() {
        let event = sound::SoundEvent {
            volume_db: -4.0,
            bus: "Footsteps".into(),
            ..sound::SoundEvent::default()
        };
        let read = EventState::Read {
            event,
            notes: Vec::new(),
        };
        let words = row_words("step", Some(&read));
        assert_eq!(words[0], "step");
        assert_eq!(words[1], "Footsteps");
        assert_eq!(words[2], "\u{2212}4 dB");
        assert_eq!(words[3], "In the world");
        assert_eq!(words[4], "\u{d7}0");
        assert_eq!(
            row_words("x", Some(&EventState::Failed("bad".into())))[1],
            "can't be read"
        );
        assert_eq!(row_words("x", None)[1], "reading\u{2026}");
    }

    #[test]
    fn the_columns_fill_the_row_in_order() {
        let shares: f32 = COLUMNS.iter().map(|(_, s)| s).sum();
        assert!((shares - 1.0).abs() < 1e-4);
        let row = Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 26.0));
        let cells = cells(row);
        assert!(cells.windows(2).all(|w| w[0].right() < w[1].left()));
        assert!(cells.last().unwrap().right() <= row.right());
    }
}
