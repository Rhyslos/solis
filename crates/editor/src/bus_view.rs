//! The Audio tab's Bus group (Screen 10; round 3, `08-audio-mixer.png`):
//! the bus picked in the mixer (Master until one is), with its name, what
//! it's inside and its volume, then its ducking rules as cards, **+ Add a
//! ducking rule** and **Remove bus**. Its strip's switch (`switch`) turns
//! it to the bus's effects, which aren't built (`audio.effect`).
//!
//! Every change is a `MixerRequest::Changed`, saved once let go, as from
//! the strips.

use crate::icons::Icon;
use crate::mixer_view::{MixerRequest, MixerState, with_bus_shown};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use crate::widgets;
use egui::{Align, Layout, Rect, RichText, Ui, UiBuilder};
use sound::mixer::{Bus, Ducking, MASTER, MAX_FADE_S, MAX_RULES, MOST_DUCKING_DB, VOLUME_RANGE_DB};

/// The width of a name beside its field in the Bus group.
const NAME_W: f32 = 60.0;

/// The Bus group's body. Returns what was asked for.
pub(crate) fn bus(ui: &mut Ui, state: &MixerState) -> Vec<MixerRequest> {
    let mut out = Vec::new();
    if state.effects && !unbuilt::hidden() {
        let answer = unbuilt::page(ui, UnbuiltId::AudioEffect, Icon::Sliders);
        if answer.hide {
            out.push(MixerRequest::HideUnfinished);
        }
        return out;
    }
    let (at, bus) = with_bus_shown(state);
    egui::ScrollArea::vertical()
        .id_salt("bus group")
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(size::PANEL_PAD)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
                    identity(ui, state, at, bus, &mut out);
                    ducking(ui, state, at, bus, &mut out);
                });
        });
    out
}

/// The switch at the right of the Bus group's strip: Ducking, or the
/// bus's Effects (unfinished; with them hidden there is no switch).
pub(crate) fn switch(ui: &mut Ui, strip: Rect, state: &MixerState) -> Option<MixerRequest> {
    if unbuilt::hidden() {
        return None;
    }
    let mut side = ui.new_child(
        UiBuilder::new()
            .id_salt("bus switch")
            .max_rect(strip.shrink2(egui::vec2(size::ROW_PAD, 0.0)))
            .layout(Layout::right_to_left(Align::Center)),
    );
    let mut effects = state.effects;
    let choices = [(false, "Ducking"), (true, "Effects")];
    widgets::segmented::segmented(&mut side, "Ducking or effects", &mut effects, &choices)
        .then_some(MixerRequest::ShowEffects(effects))
}

/// A name in the names' column, then what `field` draws in `width`.
fn named<R>(ui: &mut Ui, name: &str, field: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(NAME_W, size::field_h()), egui::Sense::hover());
        widgets::words(
            ui,
            rect,
            egui::Align2::LEFT_CENTER,
            name,
            text::row(),
            tokens::text_meta(),
        );
        field(ui)
    })
    .inner
}

/// The width each of the two fields on a row gets.
fn half_w(ui: &Ui) -> f32 {
    ((ui.available_width() - 2.0 * NAME_W - 3.0 * size::GAP) / 2.0).max(size::FIELD_MIN_W)
}

/// Name and Inside, then Volume and its range.
fn identity(ui: &mut Ui, state: &MixerState, at: usize, bus: &Bus, out: &mut Vec<MixerRequest>) {
    let width = half_w(ui);
    ui.horizontal(|ui| {
        named(ui, "Name", |ui| name_field(ui, state, at, bus, width, out));
        ui.add_space(size::GAP);
        if at > 0 {
            named(ui, "Inside", |ui| {
                inside_list(ui, state, at, bus, width, out)
            });
        }
    });
    ui.horizontal(|ui| {
        named(ui, "Volume", |ui| {
            let mut volume = bus.volume_db;
            let field = widgets::field::framed(ui, width, None, |ui| {
                ui.add(
                    egui::DragValue::new(&mut volume)
                        .range(VOLUME_RANGE_DB.0..=VOLUME_RANGE_DB.1)
                        .speed(0.1)
                        .fixed_decimals(1)
                        .suffix(" dB"),
                )
            });
            volume_changed(state, at, volume, &field, out);
        });
        ui.add_space(size::GAP);
        ui.label(
            RichText::new(format!(
                "{} dB (silent) to +{} dB",
                VOLUME_RANGE_DB.0, VOLUME_RANGE_DB.1
            ))
            .font(text::row())
            .color(tokens::text_meta()),
        );
    });
}

/// The bus's name, renamed once the field is left. Master keeps its name.
fn name_field(
    ui: &mut Ui,
    state: &MixerState,
    at: usize,
    bus: &Bus,
    width: f32,
    out: &mut Vec<MixerRequest>,
) {
    if at == 0 {
        let mut shown = MASTER.to_owned();
        ui.add_enabled_ui(false, |ui| {
            widgets::field::text_field(ui, "Bus name", &mut shown, "", width)
        })
        .response
        .on_disabled_hover_text("Master is the top bus, and keeps its name");
        return;
    }
    let id = ui.id().with(("bus-name", at));
    let mut words = ui
        .data(|d| d.get_temp::<String>(id))
        .unwrap_or_else(|| bus.name.clone());
    let field = widgets::field::text_field(ui, "Bus name", &mut words, "", width);
    if field.changed() {
        ui.data_mut(|d| d.insert_temp(id, words.clone()));
    }
    if field.lost_focus() {
        ui.data_mut(|d| d.remove::<String>(id));
        let mut mixer = state.mixer.clone();
        if words.trim() != bus.name && mixer.rename_bus(at, &words) {
            out.push(MixerRequest::Changed { mixer, save: true });
        }
    }
}

/// The bus it is inside: every bus but itself and those inside it (that
/// would be a loop), and none already as deep as buses go.
fn inside_list(
    ui: &mut Ui,
    state: &MixerState,
    at: usize,
    bus: &Bus,
    width: f32,
    out: &mut Vec<MixerRequest>,
) {
    let choices: Vec<(usize, &str)> = state
        .mixer
        .buses
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            !state.mixer.is_within(*i, at) && state.mixer.depth(*i) < sound::mixer::MAX_DEPTH
        })
        .map(|(i, other)| (i, other.name.as_str()))
        .collect();
    let mut chosen = state.mixer.bus_for(&bus.inside);
    if widgets::field::dropdown(ui, "Inside", &mut chosen, &choices, width) {
        let mut mixer = state.mixer.clone();
        mixer.buses[at]
            .inside
            .clone_from(&state.mixer.buses[chosen].name);
        // Read back through the file's tidying, so the list keeps each
        // bus after its parent.
        if let Ok(read) = sound::mixer::read(&sound::mixer::write(&mixer)) {
            out.push(MixerRequest::Changed {
                mixer: read.mixer,
                save: true,
            });
        }
    }
}

/// A change to bus `at`'s volume from `response`: kept while dragging,
/// saved once let go.
fn volume_changed(
    state: &MixerState,
    at: usize,
    volume: f32,
    response: &egui::Response,
    out: &mut Vec<MixerRequest>,
) {
    let changed = response.changed() && volume != state.mixer.buses[at].volume_db;
    let let_go = response.drag_stopped() || response.lost_focus();
    if !changed && !let_go {
        return;
    }
    let mut mixer = state.mixer.clone();
    mixer.buses[at].volume_db = volume;
    let save = let_go || (changed && !response.dragged() && !response.has_focus());
    out.push(MixerRequest::Changed { mixer, save });
}

/// DUCKING: what it is, the rules as cards, Add a ducking rule and
/// Remove bus.
fn ducking(ui: &mut Ui, state: &MixerState, at: usize, bus: &Bus, out: &mut Vec<MixerRequest>) {
    ui.horizontal(|ui| {
        widgets::rows::section_caps(ui, "Ducking");
        ui.label(
            RichText::new(format!(
                "{} gets quieter while another bus plays: music under voices.",
                bus.name
            ))
            .font(text::row())
            .color(tokens::text_body()),
        );
    });
    let ducked = state.ducked.get(at).copied().unwrap_or(0.0);
    for (r, rule) in bus.ducking.iter().enumerate() {
        rule_card(ui, state, (at, r), rule, ducked, out);
    }
    ui.horizontal(|ui| {
        let room = bus.ducking.len() < MAX_RULES && state.mixer.buses.len() > 1;
        if room && widgets::button::dashed(ui, "+ Add a ducking rule").clicked() {
            out.extend(add_rule(state));
        }
        if at > 0 {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let remove = widgets::button::Button::plain("Remove bus")
                    .icon(Icon::Trash)
                    .show(ui)
                    .on_hover_text(
                        "Buses inside it move up; sound events on it play through Master",
                    );
                if remove.clicked() {
                    out.extend(remove_bus(state));
                }
            });
        }
    });
}

/// A new ducking rule on the bus shown, while another bus plays: the
/// ribbon's Add rule and the group's + Add a ducking rule.
pub(crate) fn add_rule(state: &MixerState) -> Vec<MixerRequest> {
    let (at, bus) = with_bus_shown(state);
    if bus.ducking.len() >= MAX_RULES || state.mixer.buses.len() < 2 {
        return Vec::new();
    }
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
    vec![MixerRequest::Changed { mixer, save: true }]
}

/// Removes the bus shown (never Master): the ribbon's Remove and the
/// group's Remove bus.
pub(crate) fn remove_bus(state: &MixerState) -> Vec<MixerRequest> {
    let at = state.bus_shown();
    if at == 0 {
        return Vec::new();
    }
    let mut mixer = state.mixer.clone();
    mixer.remove_bus(at);
    vec![MixerRequest::Changed { mixer, save: true }]
}

/// One ducking rule as a card: "Quieter while Voice plays", what it does
/// right now, its × and its four settings.
fn rule_card(
    ui: &mut Ui,
    state: &MixerState,
    (at, r): (usize, usize),
    rule: &Ducking,
    ducked: f32,
    out: &mut Vec<MixerRequest>,
) {
    let mut changed: Option<(Ducking, bool)> = None;
    widgets::card::frame(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Quieter while {} plays", rule.while_bus))
                    .font(text::header())
                    .color(tokens::text_primary()),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button::icon(ui, Icon::Close, "Remove this rule").clicked() {
                    let mut mixer = state.mixer.clone();
                    mixer.buses[at].ducking.remove(r);
                    out.push(MixerRequest::Changed { mixer, save: true });
                }
                if ducked < -0.05 {
                    ui.label(
                        RichText::new(format!("Right now: {ducked:.0} dB"))
                            .font(text::row())
                            .color(tokens::WARNING),
                    );
                }
            });
        });
        let width = half_w(ui);
        ui.horizontal(|ui| {
            named(ui, "Quieter while", |ui| {
                let choices: Vec<(usize, &str)> = state
                    .mixer
                    .buses
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != at)
                    .map(|(i, b)| (i, b.name.as_str()))
                    .collect();
                let mut chosen = state.mixer.bus_for(&rule.while_bus);
                if widgets::field::dropdown(ui, "Quieter while", &mut chosen, &choices, width) {
                    let mut new = rule.clone();
                    new.while_bus.clone_from(&state.mixer.buses[chosen].name);
                    changed = Some((new, true));
                }
            });
            ui.add_space(size::GAP);
            named(ui, "By", |ui| {
                let range = MOST_DUCKING_DB..=0.0;
                if let Some((v, save)) = number(ui, "By", rule.by_db, range, " dB", width) {
                    changed = Some((
                        Ducking {
                            by_db: v,
                            ..rule.clone()
                        },
                        save,
                    ));
                }
            });
        });
        ui.horizontal(|ui| {
            let range = || 0.0..=MAX_FADE_S;
            named(ui, "Fade down", |ui| {
                if let Some((v, save)) = number(ui, "Fade down", rule.down_s, range(), " s", width)
                {
                    changed = Some((
                        Ducking {
                            down_s: v,
                            ..rule.clone()
                        },
                        save,
                    ));
                }
            });
            ui.add_space(size::GAP);
            named(ui, "Fade back", |ui| {
                if let Some((v, save)) = number(ui, "Fade back", rule.back_s, range(), " s", width)
                {
                    changed = Some((
                        Ducking {
                            back_s: v,
                            ..rule.clone()
                        },
                        save,
                    ));
                }
            });
        });
    });
    if let Some((new, save)) = changed {
        let mut mixer = state.mixer.clone();
        mixer.buses[at].ducking[r] = new;
        out.push(MixerRequest::Changed { mixer, save });
    }
}

/// A number field for one of a rule's settings. Returns the new value,
/// and whether to save it now, when it changed or was let go.
fn number(
    ui: &mut Ui,
    name: &str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    width: f32,
) -> Option<(f32, bool)> {
    let mut v = value;
    let field = widgets::field::framed(ui, width, None, |ui| {
        let response = ui.add(
            egui::DragValue::new(&mut v)
                .range(range)
                .speed(0.05)
                .fixed_decimals(2)
                .suffix(suffix),
        );
        let name = name.to_owned();
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &name));
        response
    });
    let let_go = field.drag_stopped() || field.lost_focus();
    (field.changed() && v != value)
        .then_some((v, let_go || (!field.dragged() && !field.has_focus())))
        .or_else(|| let_go.then_some((v, true)))
}
