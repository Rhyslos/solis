//! The mixer's console (Screen 10, spec 762-776; round 3,
//! `08-audio-mixer.png`): a strip per bus in a row that scrolls sideways,
//! Master last (the project lead's answer 30). A strip, top to bottom:
//! its name and where it goes, an insert slot and the pan (unfinished),
//! the fader beside its dB scale and the meter pair, the fader's value,
//! "ducked" while ducking works on it, and Mute and Solo.
//!
//! A drag on the fader moves it, Shift for a fine move (answer 25);
//! a double-click puts it back to 0 dB; it saves once let go. A click
//! on a meter drops its peak line to the level now (answer 25).

use crate::mixer_view::{MixerRequest, MixerState};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use egui::{
    Align2, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    pos2, vec2,
};
use sound::mixer::{SILENT_DB, VOLUME_RANGE_DB};

/// The numbers on a fader's scale, in decibels (round 3).
const SCALE_DB: [f32; 6] = [12.0, 0.0, -12.0, -24.0, -36.0, -48.0];
/// How much a fine (Shift) drag moves the fader, against a plain one.
const FINE: f32 = 0.1;
/// How faint a muted strip is drawn (spec 776: 55%).
const MUTED_OPACITY: f32 = 0.55;

/// The console. Returns what was asked for.
pub(crate) fn console(ui: &mut Ui, state: &MixerState) -> Vec<MixerRequest> {
    let mut requests = Vec::new();
    ui.painter()
        .rect_filled(ui.max_rect(), 0.0, tokens::shelf_bar());
    egui::ScrollArea::horizontal()
        .id_salt("mixer strips")
        .auto_shrink(false)
        .show(ui, |ui| {
            let height = (ui.available_height() - 2.0 * size::STRIP_PAD).max(strip_least_h());
            ui.add_space(size::STRIP_PAD);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = size::STRIP_GAP;
                ui.add_space(size::STRIP_PAD);
                for at in strip_order(state.mixer.buses.len()) {
                    strip(ui, state, at, height, &mut requests);
                }
                ui.add_space(size::STRIP_PAD);
            });
        });
    requests
}

/// The order the strips stand in: every bus as the file lists them, then
/// Master (index 0) last.
pub(crate) fn strip_order(buses: usize) -> Vec<usize> {
    (1..buses).chain((buses > 0).then_some(0)).collect()
}

/// The least height a strip takes: everything but the fader, and the
/// fader's least.
fn strip_least_h() -> f32 {
    size::STRIP_HEADER_H
        + 2.0 * (size::STRIP_SLOT_H + 2.0 * size::ROW_GAP)
        + size::FADER_MIN_H
        + foot_h()
}

/// The height of a strip's foot: the value, the ducking note and M / S.
fn foot_h() -> f32 {
    3.0 * size::ROW_GAP + 2.0 * size::STRIP_SLOT_H + size::STRIP_BUTTON_H + size::STRIP_SLOT_MARGIN
}

/// One bus's strip, `height` tall.
fn strip(ui: &mut Ui, state: &MixerState, at: usize, height: f32, out: &mut Vec<MixerRequest>) {
    let bus = &state.mixer.buses[at];
    let (rect, _) = ui.allocate_exact_size(vec2(size::STRIP_W, height), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let selected = state.bus_shown() == at && state.shown.is_some();
    let muted = state.listening.muted.contains(&bus.name);
    let mut ui = ui.new_child(UiBuilder::new().max_rect(rect).id_salt(("strip", at)));
    if muted {
        ui.multiply_opacity(MUTED_OPACITY);
    }
    let accent = crate::interface_look::in_use().accent.colors().fill;
    let round = CornerRadius::same(size::radius::CARD);
    ui.painter().rect_filled(rect, round, tokens::panel());
    let header = rect.with_max_y(rect.top() + size::STRIP_HEADER_H);
    strip_header(&mut ui, header, state, at, selected, out);
    let mut y = header.bottom() + size::ROW_GAP;
    let slot = |y: f32| {
        Rect::from_min_size(
            pos2(rect.left() + size::STRIP_SLOT_MARGIN, y),
            vec2(
                rect.width() - 2.0 * size::STRIP_SLOT_MARGIN,
                size::STRIP_SLOT_H,
            ),
        )
    };
    if !unbuilt::hidden() {
        let mut insert = ui.new_child(UiBuilder::new().max_rect(slot(y)).id_salt("insert"));
        unbuilt::chip(&mut insert, UnbuiltId::AudioStripInserts);
        let pan = slot(y + size::STRIP_SLOT_H + size::ROW_GAP);
        let mut pan = ui.new_child(UiBuilder::new().max_rect(pan).id_salt("pan"));
        unbuilt::chip(&mut pan, UnbuiltId::AudioStripPan);
    }
    y += 2.0 * (size::STRIP_SLOT_H + size::ROW_GAP);
    let foot = rect.bottom() - foot_h();
    let fader_area = Rect::from_x_y_ranges(rect.x_range(), y..=foot);
    fader(&mut ui, fader_area, state, at, out);
    strip_foot(&mut ui, rect.with_min_y(foot), state, at, selected, out);
    let line = if selected { accent } else { tokens::line() };
    ui.painter()
        .rect_stroke(rect, round, Stroke::new(1.0, line), StrokeKind::Inside);
    if selected {
        // The accent ring round the strip picked (spec 775).
        ui.painter()
            .rect_stroke(rect, round, Stroke::new(1.0, accent), StrokeKind::Outside);
    }
}

/// The header: the bus's name over where it goes, on the darker fill
/// (the accent's tint while picked). A click picks the bus.
fn strip_header(
    ui: &mut Ui,
    header: Rect,
    state: &MixerState,
    at: usize,
    selected: bool,
    out: &mut Vec<MixerRequest>,
) {
    let bus = &state.mixer.buses[at];
    let accent = crate::interface_look::in_use().accent;
    let fill = if selected {
        tokens::mix(
            accent.colour(),
            tokens::shelf_bar(),
            tokens::share::SELECTED,
        )
    } else {
        tokens::shelf_bar()
    };
    let top = CornerRadius {
        nw: size::radius::CARD,
        ne: size::radius::CARD,
        sw: 0,
        se: 0,
    };
    ui.painter().rect_filled(header, top, fill);
    tokens::hline(
        ui.painter(),
        header.x_range(),
        header.bottom() - 1.0,
        tokens::line(),
    );
    let pad = size::STRIP_SLOT_MARGIN + 2.0;
    let name_ink = if selected {
        tokens::text_emphasis()
    } else {
        tokens::text_primary()
    };
    crate::widgets::cut_off(
        ui,
        Rect::from_min_max(
            pos2(header.left() + pad, header.top() + pad),
            pos2(header.right() - pad, header.center().y + 2.0),
        ),
        &bus.name,
        text::semibold_at(text::UI_SIZE),
        name_ink,
    );
    let routing = if at == 0 {
        "Output".to_owned()
    } else {
        format!("Bus \u{2192} {}", bus.inside)
    };
    crate::widgets::cut_off(
        ui,
        Rect::from_min_max(
            pos2(header.left() + pad, header.center().y + 2.0),
            pos2(header.right() - pad, header.bottom() - size::ROW_GAP),
        ),
        &routing,
        text::strip_routing(),
        tokens::text_meta(),
    );
    let response = ui.interact(header, Id::new(("strip header", at)), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &bus.name));
    if response
        .on_hover_text(format!("{}: show it in the Bus group", bus.name))
        .clicked()
    {
        out.push(MixerRequest::Select(at));
    }
}

/// Where `db` is on a fader travelling `top` to `bottom`.
fn fader_y(db: f32, top: f32, bottom: f32) -> f32 {
    let (low, high) = VOLUME_RANGE_DB;
    let share = ((high - db) / (high - low)).clamp(0.0, 1.0);
    top + (bottom - top) * share
}

/// The fader with its scale, and the meters beside it.
fn fader(ui: &mut Ui, area: Rect, state: &MixerState, at: usize, out: &mut Vec<MixerRequest>) {
    let bus = &state.mixer.buses[at];
    let reach = size::FADER_CAP[1] / 2.0;
    let (top, bottom) = (area.top() + reach, area.bottom() - reach);
    let painter = ui.painter().clone();
    for db in SCALE_DB {
        let y = fader_y(db, top, bottom).round();
        tokens::hline(
            &painter,
            egui::Rangef::new(
                area.left() + size::STRIP_SLOT_MARGIN,
                area.left() + size::FADER_SCALE_X,
            ),
            y,
            tokens::line(),
        );
        let words = if db > 0.0 {
            format!("+{db:.0}")
        } else {
            format!("{db:.0}").replace('-', "\u{2212}")
        };
        painter.text(
            pos2(area.left() + size::FADER_SCALE_X, y),
            Align2::RIGHT_CENTER,
            words,
            text::fader_scale(),
            tokens::text_faint(),
        );
    }
    let x = area.left() + size::FADER_X;
    let track = Rect::from_x_y_ranges(
        x - size::FADER_TRACK / 2.0..=x + size::FADER_TRACK / 2.0,
        top..=bottom,
    );
    painter.rect_filled(track, 1.0, tokens::field());
    let cap_y = fader_y(bus.volume_db, top, bottom);
    let cap = Rect::from_center_size(pos2(x, cap_y), vec2(size::FADER_CAP[0], size::FADER_CAP[1]));
    let reach_rect = Rect::from_x_y_ranges(
        x - size::FADER_CAP[0] / 2.0..=x + size::FADER_CAP[0] / 2.0,
        area.y_range(),
    );
    let response = ui.interact(reach_rect, Id::new(("fader", at)), Sense::click_and_drag());
    let name = format!("{}'s volume", bus.name);
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(egui::WidgetType::Slider, true, &name);
        info.value = Some(f64::from(bus.volume_db));
        info
    });
    fader_cap(&painter, cap);
    let response = response.on_hover_text(format!(
        "{name}: drag (Shift for a fine move); a double-click puts it back to 0 dB"
    ));
    fader_moved(state, at, &response, bottom - top, out);
    let meters = Rect::from_min_max(
        pos2(
            area.left() + size::FADER_SCALE_X + size::METER_GAP * 3.0,
            top,
        ),
        pos2(
            area.left() + size::FADER_SCALE_X + size::METER_GAP * 4.0 + 2.0 * size::METER_W,
            bottom,
        ),
    );
    meter_pair(ui, meters, state, at, out);
}

/// The fader's cap: a dark lozenge, lighter at its top, with a white
/// line across its middle (spec 770).
fn fader_cap(painter: &egui::Painter, cap: Rect) {
    let round = CornerRadius::same(3);
    painter.rect_filled(cap, round, tokens::FADER_FOOT);
    painter.rect_filled(cap.with_max_y(cap.center().y), round, tokens::FADER_TOP);
    painter.rect_stroke(
        cap,
        round,
        Stroke::new(1.0, tokens::text_faint()),
        StrokeKind::Inside,
    );
    tokens::hline(
        painter,
        cap.shrink2(vec2(5.0, 0.0)).x_range(),
        cap.center().y.round(),
        tokens::text_primary(),
    );
}

/// What a drag or double-click on bus `at`'s fader asks for: the volume
/// while dragging, saved once let go.
fn fader_moved(
    state: &MixerState,
    at: usize,
    response: &Response,
    travel: f32,
    out: &mut Vec<MixerRequest>,
) {
    let now = state.mixer.buses[at].volume_db;
    let set = |volume: f32, save: bool, out: &mut Vec<MixerRequest>| {
        let mut mixer = state.mixer.clone();
        mixer.buses[at].volume_db = volume;
        out.push(MixerRequest::Changed { mixer, save });
    };
    if response.double_clicked() {
        set(0.0, true, out);
        return;
    }
    if response.dragged() {
        let (low, high) = VOLUME_RANGE_DB;
        let fine = response.ctx.input(|i| i.modifiers.shift);
        let per_point = (high - low) / travel.max(1.0) * if fine { FINE } else { 1.0 };
        let moved = -response.drag_delta().y * per_point;
        let volume = (now + moved).clamp(low, high);
        if volume != now {
            set(volume, false, out);
        }
    }
    if response.drag_stopped() {
        set(now, true, out);
    }
}

/// The meter pair: the bus's level on both bars (its one level, until
/// left and right are measured apart), grey rising to amber when hot,
/// with the peak line over it. A click drops the peak line to the level.
fn meter_pair(ui: &mut Ui, rect: Rect, state: &MixerState, at: usize, out: &mut Vec<MixerRequest>) {
    let (level, held) = state.meter(at);
    let response = ui.interact(rect, Id::new(("meter", at)), Sense::click());
    let name = format!("{}'s meter", state.mixer.buses[at].name);
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    if response
        .on_hover_text(format!(
            "{name}: a click puts its peak line back to the level now"
        ))
        .clicked()
    {
        out.push(MixerRequest::ResetPeak(at));
    }
    let painter = ui.painter();
    let share = |db: f32| ((db - SILENT_DB) / -SILENT_DB).clamp(0.0, 1.0);
    for bar in 0..2 {
        let left = rect.left() + bar as f32 * (size::METER_W + size::METER_GAP);
        let well = Rect::from_x_y_ranges(left..=left + size::METER_W, rect.y_range());
        painter.rect_filled(well, 1.0, tokens::field());
        let top = well.bottom() - well.height() * share(level);
        if top < well.bottom() {
            let fill = Rect::from_x_y_ranges(well.x_range(), top..=well.bottom());
            painter.rect_filled(fill, 0.0, tokens::METER_LOW);
            let hot = well.bottom() - well.height() * size::METER_HOT;
            let upper = Rect::from_x_y_ranges(well.x_range(), top..=well.center().y.max(top));
            painter.rect_filled(upper, 0.0, tokens::METER_HIGH);
            if top < hot {
                let amber = Rect::from_x_y_ranges(well.x_range(), top..=hot);
                painter.rect_filled(amber, 0.0, tokens::WARNING);
            }
        }
        if held > SILENT_DB {
            let y = well.bottom() - well.height() * share(held);
            let line = Rect::from_x_y_ranges(well.x_range(), y - size::METER_PEAK..=y);
            painter.rect_filled(line, 0.0, tokens::text_emphasis());
        }
    }
}

/// The strip's foot: the fader's value, the ducking now, Mute and Solo
/// (Master has Mute only).
fn strip_foot(
    ui: &mut Ui,
    foot: Rect,
    state: &MixerState,
    at: usize,
    selected: bool,
    out: &mut Vec<MixerRequest>,
) {
    let bus = &state.mixer.buses[at];
    let painter = ui.painter();
    let mut y = foot.top() + size::ROW_GAP + size::STRIP_SLOT_H / 2.0;
    let value = format!("{:.1} dB", bus.volume_db).replace('-', "\u{2212}");
    let ink = if selected {
        tokens::text_emphasis()
    } else {
        tokens::text_primary()
    };
    painter.text(
        pos2(foot.center().x, y),
        Align2::CENTER_CENTER,
        value,
        text::mono(),
        ink,
    );
    y += size::STRIP_SLOT_H;
    let ducked = state.ducked.get(at).copied().unwrap_or(0.0);
    if ducked < -0.05 {
        painter.text(
            pos2(foot.center().x, y),
            Align2::CENTER_CENTER,
            format!("ducked {ducked:.0}"),
            text::strip_routing(),
            tokens::WARNING,
        );
    }
    let buttons = Rect::from_min_size(
        pos2(
            foot.left() + size::STRIP_SLOT_MARGIN,
            foot.bottom() - size::STRIP_SLOT_MARGIN - size::STRIP_BUTTON_H,
        ),
        vec2(
            foot.width() - 2.0 * size::STRIP_SLOT_MARGIN,
            size::STRIP_BUTTON_H,
        ),
    );
    let muted = state.listening.muted.contains(&bus.name);
    if at == 0 {
        if strip_button(
            ui,
            buttons,
            "M",
            muted,
            tokens::WARNING,
            &format!("Mute {}", bus.name),
        ) {
            out.push(MixerRequest::Mute(at));
        }
        return;
    }
    let half = (buttons.width() - size::STRIP_SLOT_MARGIN) / 2.0;
    let mute = buttons.with_max_x(buttons.left() + half);
    let solo = buttons.with_min_x(buttons.right() - half);
    if strip_button(
        ui,
        mute,
        "M",
        muted,
        tokens::WARNING,
        &format!("Mute {}", bus.name),
    ) {
        out.push(MixerRequest::Mute(at));
    }
    let soloed = state.listening.soloed.contains(&bus.name);
    let accent = crate::interface_look::in_use().accent.colors().fill;
    let hint = format!("Solo {}: hear only it while any bus is soloed", bus.name);
    if strip_button(ui, solo, "S", soloed, accent, &hint) {
        out.push(MixerRequest::Solo(at));
    }
}

/// A strip's M or S button in `rect`, filled with `on_fill` while on.
/// Returns whether it was clicked.
fn strip_button(
    ui: &mut Ui,
    rect: Rect,
    word: &str,
    on: bool,
    on_fill: egui::Color32,
    name: &str,
) -> bool {
    let response = ui.interact(rect, Id::new(("strip button", name)), Sense::click());
    response.widget_info(|| WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, name));
    let (fill, ink) = if on {
        (on_fill, tokens::ON_ACCENT)
    } else if response.hovered() {
        (tokens::hover(), tokens::text_primary())
    } else {
        (tokens::line(), tokens::text_label())
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::radius::SMALL), fill);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        word,
        text::strip_button(),
        ink,
    );
    crate::widgets::focus_ring(ui, &response, rect, size::radius::SMALL);
    response.on_hover_text(name).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_stands_last() {
        assert_eq!(strip_order(4), [1, 2, 3, 0]);
        assert_eq!(strip_order(1), [0]);
        assert!(strip_order(0).is_empty());
    }

    #[test]
    fn the_fader_runs_from_its_loudest_at_the_top_to_silent_at_the_foot() {
        let (low, high) = VOLUME_RANGE_DB;
        assert_eq!(fader_y(high, 10.0, 82.0), 10.0);
        assert_eq!(fader_y(low, 10.0, 82.0), 82.0);
        // Twelve decibels a sixth of the way down, as the scale's lines.
        assert!((fader_y(0.0, 0.0, 72.0) - 12.0).abs() < 1e-4);
        assert_eq!(fader_y(f32::NAN.max(high + 40.0), 0.0, 72.0), 0.0);
    }
}
