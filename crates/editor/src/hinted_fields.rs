//! What game code's field hints add to Properties (decision 0127): a
//! slider with its number for a field with a range, values kept to the
//! range and step when dragged or typed (never when only shown: game
//! code may set anything), a sentence under the field, a heading over a
//! group, and text over several lines.

use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{Layout, RichText, Ui, vec2};
use reflect::{FieldInfo, FieldKind, Value};

/// `value` kept to `field`'s step and range, as a dragged or typed value
/// is: rounded to a multiple of the step, then within the range.
pub(crate) fn kept(value: f64, field: &FieldInfo) -> f64 {
    let mut value = value;
    if let Some(step) = field.hint.step.filter(|s| *s > 0.0) {
        value = (value / step).round() * step;
        // A multiple of 0.1 as written, not 0.30000000000000004.
        value = (value * 1e9).round() / 1e9;
    }
    if let Some((low, high)) = field.hint.range {
        value = value.clamp(low, high);
    }
    value
}

/// A number of `field`'s kind (a whole number for an int).
fn number_value(value: f64, field: &FieldInfo) -> Value {
    if field.kind == FieldKind::Int {
        Value::Int(value.round() as i64)
    } else {
        Value::Float(value)
    }
}

/// A slider over the field's range, then its number: dragging either,
/// clicking the track or typing gives a value kept to the range and step.
/// A value outside the range (set by game code, or as it starts) is shown
/// as it is, at the slider's end, and changes only when the user moves
/// it. Values that differ between the selected entities show as "—".
pub(crate) fn slider_number(
    ui: &mut Ui,
    field: &FieldInfo,
    value: &Value,
    mixed: bool,
) -> Option<Value> {
    let (low, high) = field.hint.range?;
    let now = value.as_f64().unwrap_or(low);
    let width = ui.available_width();
    let number_w = size::SLIDER_NUMBER_W.min(width / 2.0);
    let slider_w = (width - number_w - size::GAP).max(size::SLIDER_KNOB * 2.0);
    let mut slid = now as f32;
    let slider = ui
        .allocate_ui_with_layout(
            vec2(slider_w, size::field_h()),
            Layout::left_to_right(egui::Align::Center),
            |ui| widgets::slider::slider(ui, &mut slid, low as f32..=high as f32, &field.label),
        )
        .inner;
    let inside = (low..=high).contains(&now);
    let mut out = None;
    // Only a hand on the slider changes the value: drawing it clamps one
    // outside the range, which mustn't count.
    if slider.changed() && (slider.is_pointer_button_down_on() || slider.dragged() || inside) {
        out = Some(kept(f64::from(slid), field));
    }
    let mut typed = now;
    let mut drag = egui::DragValue::new(&mut typed)
        .speed(field.hint.speed.unwrap_or((high - low) / 200.0))
        .max_decimals(3)
        .suffix(&*field.hint.unit)
        .range(low..=high)
        .clamp_existing_to_range(false);
    if field.kind == FieldKind::Int {
        drag = drag.max_decimals(0);
    }
    if mixed {
        drag = drag.custom_formatter(|_, _| "—".to_string());
    }
    if widgets::field::framed(ui, number_w, None, |ui| ui.add(drag)).changed() {
        out = Some(kept(typed, field));
    }
    out.map(|v| number_value(v, field))
        .filter(|v| mixed || v != value)
}

/// Text over several lines, in the design's field box.
pub(crate) fn multiline(
    ui: &mut Ui,
    field: &FieldInfo,
    value: &Value,
    mixed: bool,
) -> Option<Value> {
    let mut words = match value {
        Value::Text(t) if !mixed => t.clone(),
        _ => String::new(),
    };
    let width = ui.available_width();
    let changed = egui::Frame::new()
        .fill(tokens::field())
        .stroke(egui::Stroke::new(size::LINE, tokens::field_border()))
        .corner_radius(egui::CornerRadius::same(size::radius::CONTROL))
        .inner_margin(egui::Margin::symmetric(size::FIELD_PAD as i8, 4))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut words)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .font(text::mono())
                    .desired_rows(size::MULTILINE_ROWS)
                    .desired_width(width - 2.0 * size::FIELD_PAD)
                    .hint_text(if mixed { "—" } else { "" }),
            )
            .on_hover_text(field.label.as_str())
            .changed()
        })
        .inner;
    changed.then_some(Value::Text(words))
}

/// The words under a field: its tip, and a slider's ends ("Walking speed
/// · 0 to 10"); none when it has neither.
pub(crate) fn under_words(field: &FieldInfo) -> Option<String> {
    let ends = field
        .hint
        .range
        .filter(|_| field.hint.slider)
        .map(|(low, high)| {
            format!(
                "{} to {}",
                crate::inspector::short(&Value::Float(low)),
                crate::inspector::short(&Value::Float(high))
            )
        });
    match (field.hint.tip.is_empty(), ends) {
        (true, None) => None,
        (true, Some(ends)) => Some(ends),
        (false, None) => Some(field.hint.tip.to_string()),
        (false, Some(ends)) => Some(format!("{} · {ends}", field.hint.tip)),
    }
}

/// The words under a field, quiet, from where the fields start.
pub(crate) fn under(ui: &mut Ui, words: &str) {
    ui.horizontal(|ui| {
        ui.add_space(size::FIELD_LABEL_W);
        ui.add(
            egui::Label::new(
                RichText::new(words)
                    .font(text::setting_hint())
                    .color(tokens::text_meta()),
            )
            .wrap(),
        );
    });
}

/// A group's heading (game code's `group "Seeing"`), over the fields
/// after it: a line, then its name in spaced capitals.
pub(crate) fn group_heading(ui: &mut Ui, name: &str) {
    ui.add_space(size::ROW_GAP);
    let (line, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), size::LINE), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, tokens::line());
    ui.horizontal(|ui| {
        ui.add_space(size::FIELD_DOT_W);
        ui.label(text::caps(name, text::SECTION_CAPS, tokens::text_meta()));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect::Hint;

    fn speed(hint: Hint) -> FieldInfo {
        FieldInfo::new("speed", "Speed", FieldKind::Float, Value::Float(4.0)).hint(hint)
    }

    #[test]
    fn a_value_is_kept_to_the_step_and_the_range() {
        let field = speed(Hint {
            range: Some((0.0, 10.0)),
            step: Some(0.5),
            ..Hint::default()
        });
        assert_eq!(kept(4.26, &field), 4.5);
        assert_eq!(kept(4.24, &field), 4.0);
        assert_eq!(kept(-3.0, &field), 0.0);
        assert_eq!(kept(12.0, &field), 10.0);
        let tenths = speed(Hint {
            step: Some(0.1),
            ..Hint::default()
        });
        assert_eq!(
            kept(0.29999, &tenths),
            0.3,
            "as written, no 0.30000000000000004"
        );
        assert_eq!(
            kept(123.456, &speed(Hint::default())),
            123.456,
            "no hints, no change"
        );
    }

    #[test]
    fn the_words_under_say_the_tip_and_a_slider_s_ends() {
        let mut field = speed(Hint {
            range: Some((0.0, 10.0)),
            slider: true,
            tip: "Walking speed".into(),
            ..Hint::default()
        });
        assert_eq!(
            under_words(&field).as_deref(),
            Some("Walking speed · 0 to 10")
        );
        field.hint.tip = "".into();
        assert_eq!(under_words(&field).as_deref(), Some("0 to 10"));
        field.hint.slider = false;
        assert_eq!(under_words(&field), None);
        assert_eq!(under_words(&speed(Hint::default())), None);
    }
}
