//! The Mixer panel and a bus in Properties (Phase 7, S4; decision 0056),
//! as mocked up with the project lead: a strip per bus (its name, a meter
//! while sound plays, a fader, its volume, mute and solo), Master first;
//! clicking a strip shows the bus in Properties, with what it's inside,
//! its volume and its ducking rules.
//!
//! The buses are the project's (`mixer.solmix`, read and written by the
//! app). An edit is saved once the mouse button is let go
//! (`WorkbenchAction::SaveMixer`), so dragging a fader saves once. Mute
//! and solo aren't saved: they're for listening while working. None of it
//! is an undo step: it's the project's sound, not the area.
//!
//! Colours that mean something keep theirs (decision 0060): mute is red,
//! solo yellow, and meters green, then yellow, then red near full.

use crate::document::Selection;
use crate::theme;
use egui::{Color32, RichText, Ui};
use sound::mixer::{
    Bus, Ducking, Listening, MASTER, MAX_FADE_S, MAX_RULES, MOST_DUCKING_DB, Mixer, SILENT_DB,
    VOLUME_RANGE_DB,
};

/// Mute's colour while on.
const MUTE: Color32 = Color32::from_rgb(0xa3, 0x33, 0x33);
/// Solo's colour while on.
const SOLO: Color32 = Color32::from_rgb(0x9a, 0x86, 0x10);
/// A meter's colours: comfortable, getting loud, too loud.
const METER_LOW: Color32 = Color32::from_rgb(0x4c, 0xaf, 0x6e);
const METER_MID: Color32 = Color32::from_rgb(0xd8, 0xb8, 0x3a);
const METER_HIGH: Color32 = Color32::from_rgb(0xe0, 0x55, 0x4c);
/// How fast a meter falls back, in decibels a second, so a peak can be
/// seen.
const METER_FALL_DB_PER_S: f32 = 24.0;
/// A strip's width.
const STRIP_WIDTH: f32 = 62.0;
/// A fader's height.
const FADER_HEIGHT: f32 = 96.0;

/// The Mixer's state in the editor: the project's buses, mute and solo,
/// what the meters show, and the bus Properties shows, if any.
#[derive(Debug, Clone, Default)]
pub struct MixerState {
    /// The project's buses.
    pub(crate) mixer: Mixer,
    /// Mute and solo, and the Game panel's mute.
    pub(crate) listening: Listening,
    /// What each meter shows, in decibels (falling back after a peak).
    pub(crate) meters: Vec<f32>,
    /// Each bus's ducking now, in decibels, while the game plays.
    pub(crate) ducked: Vec<f32>,
    /// The bus Properties shows, and the area's selection then (a new
    /// selection shows that instead).
    pub(crate) shown: Option<(usize, Selection)>,
}

impl MixerState {
    /// Takes the meters' newest peaks (amplitudes, 1 full scale) and each
    /// bus's ducking, `dt` seconds after the last: a meter jumps up to a
    /// peak and falls back slowly.
    pub(crate) fn update_meters(&mut self, peaks: &[f32], ducked: &[f32], dt: f32) {
        let n = self.mixer.buses.len();
        self.meters.resize(n, SILENT_DB);
        let fall = METER_FALL_DB_PER_S
            * if dt.is_finite() {
                dt.clamp(0.0, 1.0)
            } else {
                0.0
            };
        for (i, shown) in self.meters.iter_mut().enumerate() {
            let peak = peaks.get(i).copied().unwrap_or(0.0);
            let db = if peak.is_finite() && peak > 0.0 {
                (20.0 * peak.log10()).max(SILENT_DB)
            } else {
                SILENT_DB
            };
            *shown = db.max(*shown - fall).max(SILENT_DB);
        }
        self.ducked = ducked.to_vec();
        self.ducked.resize(n, 0.0);
    }

    /// Whether a meter is still moving (so the editor keeps drawing).
    pub(crate) fn meters_moving(&self) -> bool {
        self.meters.iter().any(|m| *m > SILENT_DB) || self.ducked.iter().any(|d| *d < 0.0)
    }
}

/// What a click or drag in the Mixer or a bus's Properties asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MixerRequest {
    /// Show this bus in Properties.
    Select(usize),
    /// The buses changed; `save` once the mouse is let go.
    Changed {
        /// The buses now.
        mixer: Mixer,
        /// Whether to save them now.
        save: bool,
    },
    /// Mute or unmute a bus.
    Mute(usize),
    /// Solo or unsolo a bus.
    Solo(usize),
    /// Mute or unmute everything (the Game panel's button).
    MuteAll,
}

/// The Mixer panel. Returns what was asked for.
pub(crate) fn mixer_panel(ui: &mut Ui, state: &MixerState, playing: bool) -> Vec<MixerRequest> {
    let mut requests = Vec::new();
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            if ui
                .button("Add bus")
                .on_hover_text("A new bus inside the one shown in Properties (or Master)")
                .clicked()
            {
                let mut mixer = state.mixer.clone();
                let inside = state
                    .shown
                    .as_ref()
                    .and_then(|(i, _)| mixer.buses.get(*i))
                    .map_or(MASTER.to_owned(), |b| b.name.clone());
                if let Some(at) = mixer.add_bus(&inside) {
                    requests.push(MixerRequest::Changed { mixer, save: true });
                    requests.push(MixerRequest::Select(at));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(status(state, playing)).color(theme::TEXT_WEAK));
            });
        });
    });
    egui::ScrollArea::horizontal()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for i in 0..state.mixer.buses.len() {
                    strip(ui, state, i, &mut requests);
                    if i == 0 {
                        ui.separator();
                    }
                }
            });
        });
    requests
}

/// The line at the top right: playing or not, and who's ducking whom.
fn status(state: &MixerState, playing: bool) -> String {
    if !playing {
        return "Meters move while the game plays".to_owned();
    }
    let ducking: Vec<String> = state
        .mixer
        .buses
        .iter()
        .zip(&state.ducked)
        .filter(|(_, d)| **d < -0.05)
        .flat_map(|(bus, _)| {
            bus.ducking
                .iter()
                .map(move |r| format!("{} ducking {}", r.while_bus, bus.name))
        })
        .collect();
    if ducking.is_empty() {
        "Playing".to_owned()
    } else {
        format!("Playing · {}", ducking.join(", "))
    }
}

/// One bus's strip.
fn strip(ui: &mut Ui, state: &MixerState, i: usize, requests: &mut Vec<MixerRequest>) {
    let bus = &state.mixer.buses[i];
    let selected = state.shown.as_ref().is_some_and(|(s, _)| *s == i);
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(2, 4))
        .corner_radius(3)
        .fill(if selected {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        });
    frame.show(ui, |ui| {
        ui.set_width(STRIP_WIDTH);
        ui.vertical_centered(|ui| {
            let name = RichText::new(&bus.name).color(if i == 0 {
                theme::TEXT_STRONG
            } else {
                theme::TEXT
            });
            if ui
                .add(
                    egui::Label::new(name)
                        .truncate()
                        .sense(egui::Sense::click()),
                )
                .on_hover_text(&bus.name)
                .clicked()
            {
                requests.push(MixerRequest::Select(i));
            }
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                meter(ui, state.meters.get(i).copied().unwrap_or(SILENT_DB));
                let mut volume = bus.volume_db;
                let fader = ui
                    .add(
                        egui::Slider::new(&mut volume, VOLUME_RANGE_DB.0..=VOLUME_RANGE_DB.1)
                            .vertical()
                            .show_value(false),
                    )
                    .on_hover_text(format!("{}'s volume", bus.name));
                fader_changed(state, i, volume, &fader, requests);
            });
            let mut volume = bus.volume_db;
            let field = ui.add(
                egui::DragValue::new(&mut volume)
                    .range(VOLUME_RANGE_DB.0..=VOLUME_RANGE_DB.1)
                    .speed(0.1)
                    .fixed_decimals(1)
                    .suffix(" dB"),
            );
            fader_changed(state, i, volume, &field, requests);
            let ducked = state.ducked.get(i).copied().unwrap_or(0.0);
            let note = if ducked < -0.05 {
                format!("ducked {ducked:.0}")
            } else {
                String::new()
            };
            ui.label(RichText::new(note).color(theme::WARNING).size(10.5));
            ui.horizontal(|ui| {
                let muted = state.listening.muted.contains(&bus.name);
                if toggle(ui, "M", muted, MUTE, &format!("Mute {}", bus.name)) {
                    requests.push(MixerRequest::Mute(i));
                }
                if i > 0 {
                    let soloed = state.listening.soloed.contains(&bus.name);
                    let hint = format!("Solo {}: hear only it while any bus is soloed", bus.name);
                    if toggle(ui, "S", soloed, SOLO, &hint) {
                        requests.push(MixerRequest::Solo(i));
                    }
                }
            });
        });
    });
}

/// A change to bus `i`'s fader from `response`: kept while dragging,
/// saved once let go.
fn fader_changed(
    state: &MixerState,
    i: usize,
    volume: f32,
    response: &egui::Response,
    requests: &mut Vec<MixerRequest>,
) {
    let changed = response.changed() && volume != state.mixer.buses[i].volume_db;
    let let_go = response.drag_stopped() || response.lost_focus();
    if !changed && !let_go {
        return;
    }
    let mut mixer = state.mixer.clone();
    mixer.buses[i].volume_db = volume;
    let save = let_go || (changed && !response.dragged() && !response.has_focus());
    requests.push(MixerRequest::Changed { mixer, save });
}

/// A small on/off button, filled with `color` while on.
fn toggle(ui: &mut Ui, text: &str, on: bool, color: Color32, hint: &str) -> bool {
    let button = egui::Button::new(RichText::new(text).color(if on {
        Color32::WHITE
    } else {
        theme::TEXT_WEAK
    }))
    .min_size(egui::vec2(20.0, 17.0))
    .fill(if on { color } else { theme::FIELD });
    let response = ui.add(button).on_hover_text(hint);
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, hint));
    response.clicked()
}

/// A meter: a bar as tall as the level, green, then yellow from -12 dB,
/// red from -3 dB.
fn meter(ui: &mut Ui, db: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, FADER_HEIGHT), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 1.0, theme::LINE);
    let share = |db: f32| ((db - SILENT_DB) / -SILENT_DB).clamp(0.0, 1.0);
    let top = rect.bottom() - rect.height() * share(db);
    for (from, to, color) in [
        (SILENT_DB, -12.0, METER_LOW),
        (-12.0, -3.0, METER_MID),
        (-3.0, 0.0, METER_HIGH),
    ] {
        let low = rect.bottom() - rect.height() * share(from);
        let high = rect.bottom() - rect.height() * share(to);
        let y_top = high.max(top);
        if y_top < low {
            painter.rect_filled(
                egui::Rect::from_x_y_ranges(rect.x_range(), y_top..=low),
                0.0,
                color,
            );
        }
    }
}

/// A bus in Properties: its name, what it's inside, its volume, and its
/// ducking rules. Returns what was asked for.
pub(crate) fn bus_properties(ui: &mut Ui, state: &MixerState, at: usize) -> Vec<MixerRequest> {
    let mut requests = Vec::new();
    let Some(bus) = state.mixer.buses.get(at) else {
        return requests;
    };
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&bus.name).heading());
                        ui.label(RichText::new("· bus").color(theme::TEXT_WEAK));
                    });
                    ui.add_space(4.0);
                    egui::Grid::new(("bus", at))
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            identity_rows(ui, state, at, bus, &mut requests);
                        });
                    ui.add_space(6.0);
                    ui.separator();
                    ui.label(RichText::new("Ducking").strong());
                    ui.label(
                        RichText::new(format!(
                            "{} gets quieter while another bus plays: music under voices.",
                            bus.name
                        ))
                        .color(theme::TEXT_WEAK)
                        .size(11.0),
                    );
                    for (r, rule) in bus.ducking.iter().enumerate() {
                        rule_rows(ui, state, at, r, rule, &mut requests);
                    }
                    if bus.ducking.len() < MAX_RULES
                        && state.mixer.buses.len() > 1
                        && ui.button("Add a ducking rule").clicked()
                    {
                        let mut mixer = state.mixer.clone();
                        let other = mixer
                            .buses
                            .iter()
                            .find(|b| b.name != bus.name && b.name != MASTER)
                            .map_or(MASTER.to_owned(), |b| b.name.clone());
                        mixer.buses[at].ducking.push(Ducking {
                            while_bus: other,
                            ..Ducking::default()
                        });
                        requests.push(MixerRequest::Changed { mixer, save: true });
                    }
                    let ducked = state.ducked.get(at).copied().unwrap_or(0.0);
                    if ducked < -0.05 {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(format!("Right now: {ducked:.0} dB"))
                                .color(theme::WARNING),
                        );
                    }
                    if at > 0 {
                        ui.add_space(10.0);
                        if ui
                            .button("Remove bus")
                            .on_hover_text(
                                "Buses inside it move up; sound events on it play through Master",
                            )
                            .clicked()
                        {
                            let mut mixer = state.mixer.clone();
                            mixer.remove_bus(at);
                            requests.push(MixerRequest::Changed { mixer, save: true });
                        }
                    }
                });
        });
    requests
}

/// The name, Inside and Volume rows.
fn identity_rows(
    ui: &mut Ui,
    state: &MixerState,
    at: usize,
    bus: &Bus,
    requests: &mut Vec<MixerRequest>,
) {
    ui.label(RichText::new("Name").color(theme::TEXT_WEAK));
    if at == 0 {
        ui.label(MASTER)
            .on_hover_text("Master is the top bus, and keeps its name");
    } else {
        let id = ui.id().with(("bus-name", at));
        let mut text = ui
            .data(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| bus.name.clone());
        let field = ui.add(egui::TextEdit::singleline(&mut text).desired_width(140.0));
        if field.changed() {
            ui.data_mut(|d| d.insert_temp(id, text.clone()));
        }
        if field.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
            let mut mixer = state.mixer.clone();
            if text.trim() != bus.name && mixer.rename_bus(at, &text) {
                requests.push(MixerRequest::Changed { mixer, save: true });
            }
        }
    }
    ui.end_row();
    if at > 0 {
        ui.label(RichText::new("Inside").color(theme::TEXT_WEAK));
        egui::ComboBox::from_id_salt(("bus-inside", at))
            .selected_text(&bus.inside)
            .show_ui(ui, |ui| {
                for (i, other) in state.mixer.buses.iter().enumerate() {
                    // Not itself, nor a bus inside it (that would be a loop).
                    if state.mixer.is_within(i, at)
                        || (state.mixer.depth(i) >= sound::mixer::MAX_DEPTH)
                    {
                        continue;
                    }
                    if ui
                        .selectable_label(other.name == bus.inside, &other.name)
                        .clicked()
                        && other.name != bus.inside
                    {
                        let mut mixer = state.mixer.clone();
                        mixer.buses[at].inside.clone_from(&other.name);
                        // Read back through the file's tidying, so the
                        // list keeps each bus after its parent.
                        let text = sound::mixer::write(&mixer);
                        if let Ok(read) = sound::mixer::read(&text) {
                            requests.push(MixerRequest::Changed {
                                mixer: read.mixer,
                                save: true,
                            });
                        }
                    }
                }
            });
        ui.end_row();
    }
    ui.label(RichText::new("Volume").color(theme::TEXT_WEAK));
    let mut volume = bus.volume_db;
    let field = ui.add(
        egui::DragValue::new(&mut volume)
            .range(VOLUME_RANGE_DB.0..=VOLUME_RANGE_DB.1)
            .speed(0.1)
            .fixed_decimals(1)
            .suffix(" dB"),
    );
    fader_changed(state, at, volume, &field, requests);
    ui.end_row();
}

/// One ducking rule's rows.
fn rule_rows(
    ui: &mut Ui,
    state: &MixerState,
    at: usize,
    r: usize,
    rule: &Ducking,
    requests: &mut Vec<MixerRequest>,
) {
    let mut changed: Option<(Ducking, bool)> = None;
    egui::Grid::new(("bus-rule", at, r))
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Quieter while").color(theme::TEXT_WEAK));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(("bus-rule-while", at, r))
                    .selected_text(&rule.while_bus)
                    .show_ui(ui, |ui| {
                        for (i, other) in state.mixer.buses.iter().enumerate() {
                            if i == at {
                                continue;
                            }
                            if ui
                                .selectable_label(other.name == rule.while_bus, &other.name)
                                .clicked()
                            {
                                let mut new = rule.clone();
                                new.while_bus.clone_from(&other.name);
                                changed = Some((new, true));
                            }
                        }
                    });
                if ui
                    .small_button("×")
                    .on_hover_text("Remove this rule")
                    .clicked()
                {
                    let mut mixer = state.mixer.clone();
                    mixer.buses[at].ducking.remove(r);
                    requests.push(MixerRequest::Changed { mixer, save: true });
                }
            });
            ui.end_row();
            let number = |ui: &mut Ui, label: &str, value: f32, range, suffix: &str| {
                ui.label(RichText::new(label).color(theme::TEXT_WEAK));
                let mut v = value;
                let field = ui.add(
                    egui::DragValue::new(&mut v)
                        .range(range)
                        .speed(0.05)
                        .fixed_decimals(1)
                        .suffix(suffix),
                );
                ui.end_row();
                let let_go = field.drag_stopped() || field.lost_focus();
                (field.changed() && v != value)
                    .then_some((v, let_go || (!field.dragged() && !field.has_focus())))
                    .or_else(|| let_go.then_some((v, true)))
            };
            if let Some((v, save)) = number(ui, "By", rule.by_db, MOST_DUCKING_DB..=0.0, " dB") {
                changed = Some((
                    Ducking {
                        by_db: v,
                        ..rule.clone()
                    },
                    save,
                ));
            }
            if let Some((v, save)) = number(ui, "Fade down", rule.down_s, 0.0..=MAX_FADE_S, " s") {
                changed = Some((
                    Ducking {
                        down_s: v,
                        ..rule.clone()
                    },
                    save,
                ));
            }
            if let Some((v, save)) = number(ui, "Fade back", rule.back_s, 0.0..=MAX_FADE_S, " s") {
                changed = Some((
                    Ducking {
                        back_s: v,
                        ..rule.clone()
                    },
                    save,
                ));
            }
        });
    if let Some((new, save)) = changed {
        let mut mixer = state.mixer.clone();
        mixer.buses[at].ducking[r] = new;
        requests.push(MixerRequest::Changed { mixer, save });
    }
    ui.add_space(4.0);
}

impl crate::Workbench {
    /// Uses the project's buses (as read from its `mixer.solmix`, or the
    /// standard ones). Mute and solo stay on buses that are still there.
    pub fn set_mixer(&mut self, mixer: Mixer) {
        let names: Vec<&str> = mixer.buses.iter().map(|b| b.name.as_str()).collect();
        let listening = &mut self.mixer.listening;
        listening.muted.retain(|n| names.contains(&n.as_str()));
        listening.soloed.retain(|n| names.contains(&n.as_str()));
        if self
            .mixer
            .shown
            .as_ref()
            .is_some_and(|(at, _)| *at >= mixer.buses.len())
        {
            self.mixer.shown = None;
        }
        self.mixer.mixer = mixer;
    }

    /// The project's buses, as the Mixer has them now.
    pub fn mixer(&self) -> &Mixer {
        &self.mixer.mixer
    }

    /// Mute and solo, and the Game panel's mute.
    pub fn listening(&self) -> &Listening {
        &self.mixer.listening
    }

    /// The meters' newest peaks (amplitudes) and each bus's ducking now,
    /// `dt` seconds after the last call; keeps drawing while they move.
    pub fn set_bus_meters(&mut self, peaks: &[f32], ducked: &[f32], dt: f32, ctx: &egui::Context) {
        self.mixer.update_meters(peaks, ducked, dt);
        if self.mixer.meters_moving() && self.in_front.contains(&crate::Panel::Mixer) {
            ctx.request_repaint();
        }
    }

    /// Carries out what the Mixer or a bus's Properties asked for.
    pub(crate) fn mixer_request(
        &mut self,
        request: MixerRequest,
        actions: &mut Vec<crate::WorkbenchAction>,
    ) {
        match request {
            MixerRequest::Select(at) => {
                self.mixer.shown = Some((at, self.document.selection.clone()));
                self.sound_shown = None;
            }
            MixerRequest::Changed { mixer, save } => {
                self.set_mixer(mixer);
                if save {
                    if self.project.is_some() {
                        actions.push(crate::WorkbenchAction::SaveMixer(self.mixer.mixer.clone()));
                    } else {
                        self.log(
                            crate::LogLevel::Info,
                            "The buses are kept with a project: open or make one to keep these changes.",
                        );
                    }
                }
            }
            MixerRequest::Mute(at) => flip(&mut self.mixer.listening.muted, &self.mixer.mixer, at),
            MixerRequest::Solo(at) => flip(&mut self.mixer.listening.soloed, &self.mixer.mixer, at),
            MixerRequest::MuteAll => {
                self.mixer.listening.all_muted = !self.mixer.listening.all_muted;
            }
        }
    }
}

/// The Game panel's mute button, in its top right corner: a speaker,
/// crossed out in red while everything is muted. Returns whether it was
/// clicked.
pub(crate) fn game_mute_button(ui: &mut Ui, panel: egui::Rect, muted: bool) -> bool {
    let size = egui::vec2(26.0, 22.0);
    let rect = egui::Rect::from_min_size(
        egui::pos2(panel.right() - size.x - 8.0, panel.top() + 8.0),
        size,
    );
    let hint = if muted {
        "Sound is muted: click to hear the game again"
    } else {
        "Mute the game's sound"
    };
    let response = ui
        .interact(rect, ui.id().with("game-mute"), egui::Sense::click())
        .on_hover_text(hint);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, muted, "Mute the game")
    });
    let painter = ui.painter();
    let fill = if response.hovered() {
        theme::HOVER
    } else {
        theme::OVERLAY
    };
    painter.rect_filled(rect, 4.0, fill);
    crate::icons::icon(
        painter,
        egui::Rect::from_center_size(rect.center(), egui::vec2(13.0, 13.0)),
        crate::icons::Icon::Sound,
        if muted {
            theme::TEXT_WEAK
        } else {
            theme::TEXT_STRONG
        },
    );
    if muted {
        painter.line_segment(
            [
                rect.left_bottom() + egui::vec2(6.0, -5.0),
                rect.right_top() + egui::vec2(-6.0, 5.0),
            ],
            egui::Stroke::new(2.0, MUTE),
        );
    }
    response.clicked()
}

/// Adds bus `at`'s name to `names`, or takes it out.
fn flip(names: &mut Vec<String>, mixer: &Mixer, at: usize) {
    let Some(bus) = mixer.buses.get(at) else {
        return;
    };
    if let Some(i) = names.iter().position(|n| *n == bus.name) {
        names.remove(i);
    } else {
        names.push(bus.name.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panel::Panel;
    use crate::test_support::*;
    use crate::{Workbench, WorkbenchAction};
    use egui_kittest::kittest::Queryable;

    /// A workbench with a project open and the Mixer in front.
    fn with_the_mixer() -> egui_kittest::Harness<'static, State> {
        let mut w = Workbench::new(info(), None);
        w.set_project(Some(crate::ProjectInfo {
            name: "Test".into(),
            root: std::path::PathBuf::from("/nowhere"),
            assets: std::path::PathBuf::from("/nowhere/assets"),
        }));
        let mut h = harness(w);
        h.run();
        h.get_by_label("Mixer").click();
        h.run();
        h
    }

    fn saved(h: &mut egui_kittest::Harness<'static, State>) -> Vec<Mixer> {
        h.state_mut()
            .1
            .actions
            .drain(..)
            .filter_map(|a| match a {
                WorkbenchAction::SaveMixer(m) => Some(m),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_mixer_sits_with_the_console_and_shows_every_bus() {
        let h = with_the_mixer();
        let dock = &h.state().0.dock;
        let path = dock.find_tab(&Panel::Mixer).unwrap();
        let group = dock[path.surface][path.node].tabs().unwrap().to_vec();
        assert!(group.contains(&Panel::Console), "{group:?}");
        for bus in ["Master", "Music", "Effects", "Voice", "Ambience", "UI"] {
            assert!(h.query_by_label(bus).is_some(), "{bus}");
        }
        assert!(h.query_by_label("Meters move while the game plays").is_some());
    }

    #[test]
    fn mute_and_solo_change_what_s_heard_but_aren_t_saved() {
        let mut h = with_the_mixer();
        h.get_by_label("Mute UI").click();
        h.run();
        h.get_by_label_contains("Solo Voice").click();
        h.run();
        let listening = h.state().0.listening().clone();
        assert_eq!(listening.muted, ["UI"]);
        assert_eq!(listening.soloed, ["Voice"]);
        assert!(saved(&mut h).is_empty(), "listening isn't the project's");
        h.get_by_label("Mute UI").click();
        h.run();
        assert!(h.state().0.listening().muted.is_empty());
    }

    #[test]
    fn clicking_a_bus_shows_it_in_properties_and_add_bus_saves() {
        let mut h = with_the_mixer();
        h.get_by_label("Music").click();
        h.run();
        assert_eq!(h.state().0.mixer.shown.as_ref().map(|s| s.0), Some(1));
        assert!(h.query_by_label("Quieter while").is_some());
        assert!(h.query_by_label("Remove bus").is_some());
        // A new bus goes inside the one shown.
        h.get_by_label("Add bus").click();
        h.run();
        let saves = saved(&mut h);
        assert_eq!(saves.len(), 1);
        let m = &saves[0];
        let new = m.index_of("Bus").unwrap();
        assert_eq!(m.buses[new].inside, "Music");
        assert_eq!(h.state().0.mixer.shown.as_ref().map(|s| s.0), Some(new));
        // Removing it saves again, and Properties goes back.
        h.get_by_label("Remove bus").click();
        h.run();
        assert!(saved(&mut h)[0].index_of("Bus").is_none());
    }

    #[test]
    fn selecting_an_entity_takes_properties_back() {
        let (w, crate_id) = with_a_crate();
        let mut h = harness(w);
        h.run();
        h.state_mut().0.mixer_request(MixerRequest::Select(2), &mut Vec::new());
        h.run();
        assert!(h.query_by_label("Quieter while").is_none() || h.state().0.mixer.shown.is_some());
        h.state_mut().0.document.selection.select_only(crate_id);
        h.run();
        assert!(h.state().0.mixer.shown.is_none());
    }

    #[test]
    fn without_a_project_changes_say_where_they_d_be_kept() {
        let mut w = Workbench::new(info(), None);
        let mut actions = Vec::new();
        let mut mixer = Mixer::default();
        mixer.buses[1].volume_db = -3.0;
        w.mixer_request(
            MixerRequest::Changed {
                mixer: mixer.clone(),
                save: true,
            },
            &mut actions,
        );
        assert!(actions.is_empty());
        assert_eq!(w.mixer(), &mixer, "still heard this session");
        assert!(w.log_lines().iter().any(|l| l.text.contains("project")));
    }

    #[test]
    fn a_new_mixer_keeps_mute_and_solo_only_on_buses_still_there() {
        let mut w = Workbench::new(info(), None);
        w.mixer_request(MixerRequest::Mute(5), &mut Vec::new());
        w.mixer_request(MixerRequest::Solo(3), &mut Vec::new());
        let mut fewer = Mixer::default();
        fewer.remove_bus(5);
        w.set_mixer(fewer);
        assert!(w.listening().muted.is_empty());
        assert_eq!(w.listening().soloed, ["Voice"]);
    }

    #[test]
    fn meters_jump_up_and_fall_back_slowly() {
        let mut state = MixerState::default();
        state.update_meters(&[1.0, 0.5, 0.0], &[0.0, -8.0], 1.0 / 60.0);
        assert_eq!(state.meters[0], 0.0);
        assert!((state.meters[1] - (-6.02)).abs() < 0.01);
        assert_eq!(state.meters[2], SILENT_DB);
        assert_eq!(state.ducked[1], -8.0);
        assert!(state.meters_moving());
        state.update_meters(&[], &[], 0.5);
        assert!((state.meters[0] - (-12.0)).abs() < 0.01, "24 dB a second");
        state.update_meters(&[f32::NAN], &[], f32::INFINITY);
        assert!(state.meters.iter().all(|m| m.is_finite()));
        for _ in 0..10 {
            state.update_meters(&[], &[], 1.0);
        }
        assert!(!state.meters_moving());
    }

    #[test]
    fn the_game_panel_mutes_everything() {
        let mut h = harness(Workbench::new(info(), None));
        h.run();
        h.get_by_label("Game").click();
        h.run();
        h.get_by_label("Mute the game").click();
        h.run();
        assert!(h.state().0.listening().all_muted);
    }
}
