//! The Environment's day colours in Properties (decision 0092): the day as
//! a strip running through its colours, each stop's hour and colour, and
//! adding or taking one away; and the Preview time, which shows any hour
//! in the viewport and is never saved.

use area::builtin::{DayColours, MAX_STOPS};
use egui::{Color32, Ui};
use reflect::Value;

/// Where the viewport's preview hour is kept between frames: the
/// workbench hands it to the viewport.
pub(crate) fn preview_id() -> egui::Id {
    egui::Id::new("day preview hour")
}

/// An hour as a clock shows it: `19:30`.
pub(crate) fn clock(hour: f64) -> String {
    let minutes = (hour.rem_euclid(24.0) * 60.0).round() as u32 % (24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn colour32(c: [f32; 3]) -> Color32 {
    let [r, g, b] = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    Color32::from_rgb(r, g, b)
}

/// The day's strip: its colour at every hour, left to right, with a
/// mark at `now` when there's one.
fn strip(ui: &mut Ui, day: &DayColours, now: Option<f64>) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(240.0), 14.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let steps = 96;
    for i in 0..steps {
        let x0 = rect.left() + rect.width() * i as f32 / steps as f32;
        let x1 = rect.left() + rect.width() * (i + 1) as f32 / steps as f32;
        let hour = 24.0 * (i as f32 + 0.5) / steps as f32;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x0, rect.top()),
                egui::pos2(x1 + 0.5, rect.bottom()),
            ),
            0.0,
            colour32(day.at(hour)),
        );
    }
    for (h, _) in day.stops() {
        let x = rect.left() + rect.width() * h / 24.0;
        painter.line_segment(
            [
                egui::pos2(x, rect.bottom() - 4.0),
                egui::pos2(x, rect.bottom()),
            ],
            egui::Stroke::new(1.0, Color32::from_gray(30)),
        );
    }
    if let Some(now) = now {
        let x = rect.left() + rect.width() * (now.rem_euclid(24.0) as f32) / 24.0;
        painter.line_segment(
            [
                egui::pos2(x, rect.top() - 2.0),
                egui::pos2(x, rect.bottom() + 2.0),
            ],
            egui::Stroke::new(2.0, Color32::WHITE),
        );
    }
}

/// The colours' widget for the text in `value`; the new text when it
/// changed.
pub(crate) fn day_colours(ui: &mut Ui, value: &Value, mixed: bool) -> Option<Value> {
    if mixed {
        ui.label("—");
        return None;
    }
    let text = match value {
        Value::Text(t) => t.as_str(),
        _ => "",
    };
    let day = DayColours::parse(text);
    let mut stops: Vec<(f32, [f32; 3])> = day.stops().to_vec();
    let mut changed = false;
    let preview: Option<f64> = ui.data(|d| d.get_temp(preview_id())).flatten();
    ui.vertical(|ui| {
        strip(ui, &day, preview);
        let mut remove = None;
        let hours: Vec<f32> = stops.iter().map(|(h, _)| *h).collect();
        for (i, (hour, colour)) in stops.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // The stops are sRGB, as the strip shows them.
                let mut c = colour.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                if ui.color_edit_button_srgb(&mut c).changed() {
                    *colour = c.map(|v| f32::from(v) / 255.0);
                    changed = true;
                }
                let mut h = f64::from(*hour);
                let drag = ui.add(
                    egui::DragValue::new(&mut h)
                        .range(0.0..=23.99)
                        .speed(0.05)
                        .custom_formatter(|v, _| clock(v))
                        .custom_parser(|t| {
                            let (a, b) = t.trim().split_once(':').unwrap_or((t.trim(), "0"));
                            Some(a.parse::<f64>().ok()? + b.parse::<f64>().ok()? / 60.0)
                        }),
                );
                if drag.changed() {
                    let mut h = h as f32;
                    // While dragging, a stop stays between its neighbours:
                    // the rows are sorted by hour, so passing one would
                    // hand the drag to it. A typed hour can go anywhere.
                    if drag.dragged() && hours.len() > 1 {
                        let low = if i == 0 { 0.0 } else { hours[i - 1] + 0.05 };
                        let high = hours.get(i + 1).map_or(23.99, |n| n - 0.05);
                        if low <= high {
                            h = h.clamp(low, high);
                        }
                    }
                    *hour = h;
                    changed = true;
                }
                drag.on_hover_text(
                    "When the day is this colour: drag between its neighbours, or type 19:30 to put it anywhere",
                );
                if day.stops().len() > 1
                    && ui
                        .small_button("Remove")
                        .on_hover_text("Take this colour out of the day")
                        .clicked()
                {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            stops.remove(i);
            changed = true;
        }
        if stops.len() < MAX_STOPS
            && ui
                .button("+ Add a colour")
                .on_hover_text("A new colour in the longest stretch of the day, as it is there now")
                .clicked()
        {
            // In the middle of the longest stretch between two stops.
            let mut best = (0.0f32, 12.0f32);
            for (i, (h, _)) in stops.iter().enumerate() {
                let next = stops[(i + 1) % stops.len()].0;
                let gap = (next - h).rem_euclid(24.0);
                let gap = if gap == 0.0 { 24.0 } else { gap };
                if gap > best.0 {
                    best = (gap, (h + gap / 2.0).rem_euclid(24.0));
                }
            }
            let hour = (best.1 * 4.0).round() / 4.0;
            stops.push((hour, day.at(hour)));
            changed = true;
        }
        // The preview time: the editor's, never saved.
        ui.horizontal(|ui| {
            let mut on = preview.is_some();
            let mut h = preview.unwrap_or(12.0);
            if ui
                .checkbox(&mut on, "Preview time")
                .on_hover_text("Shows the area at this hour in the viewport. It isn't saved.")
                .changed()
                || (on
                    && ui
                        .add(
                            egui::Slider::new(&mut h, 0.0..=23.99)
                                .custom_formatter(|v, _| clock(v))
                                .show_value(true),
                        )
                        .changed())
            {
                ui.data_mut(|d| d.insert_temp(preview_id(), on.then_some(h)));
            }
        });
    });
    changed.then(|| Value::Text(DayColours::from_stops(&stops).text()))
}

#[cfg(test)]
#[path = "day_colours_tests.rs"]
mod tests;
