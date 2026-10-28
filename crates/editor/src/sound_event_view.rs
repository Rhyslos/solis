//! A sound event in Properties (Phase 7, S2): its takes, how each play
//! varies, where it's heard and how many may play at once, edited in
//! place, with Play to hear a new pick each press.
//!
//! Edits are saved to the event's `.solsound` file by the app
//! (`WorkbenchAction::SaveSoundEvent`): once the mouse button is let go,
//! so dragging a number saves once, not every frame. Until then the
//! value being dragged is kept in egui's memory for this event.

use crate::inspector;
use crate::library::{LibraryDrag, LibraryItem, LibraryKind, LibraryRequest, Sounds};
use crate::sound_view::{self, EventState, SOUND_COLOR, SoundState};
use crate::theme;
use egui::{RichText, Ui};
use sound::SoundEvent;
use sound::event::{
    Fade, MAX_AT_ONCE, MAX_DISTANCE_M, MAX_PITCH_VARIATION_ST, MAX_TAKES, MAX_VOLUME_VARIATION_DB,
    PITCH_RANGE_ST, Priority, Space, VOLUME_RANGE_DB,
};

/// The buses there are until the Mixer arrives (S4; decision 0056).
pub(crate) const BUSES: [&str; 6] = ["Master", "Music", "Effects", "Voice", "Ambience", "UI"];

/// The colour of sound events' stripe: a lighter teal than sound files'.
pub(crate) const EVENT_COLOR: egui::Color32 = egui::Color32::from_rgb(0x5d, 0xca, 0xa5);

/// A sound event in Properties. Returns what was asked for.
pub(crate) fn event_properties(
    ui: &mut Ui,
    path: &str,
    sounds: Sounds<'_>,
    library: &[LibraryItem],
) -> Vec<LibraryRequest> {
    let mut requests = Vec::new();
    let name = path.rsplit('/').next().unwrap_or(path);
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| match sounds.events.get(path) {
                    None => {
                        header(ui, name, path, sounds, &mut requests, false);
                        ui.label(RichText::new("Reading it…").color(theme::TEXT_WEAK));
                    }
                    Some(EventState::Failed(why)) => {
                        header(ui, name, path, sounds, &mut requests, false);
                        ui.label(
                            RichText::new(format!(
                                "Solis couldn't read it: {why}. Fix the file in a text editor, \
                                 or delete it and make a new one."
                            ))
                            .color(theme::TEXT_WEAK),
                        );
                    }
                    Some(EventState::Read { event, notes }) => {
                        header(ui, name, path, sounds, &mut requests, true);
                        for note in notes {
                            ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
                        }
                        editor(ui, path, event, sounds, library, &mut requests);
                    }
                });
        });
    requests
}

/// The name, Play or Stop, and what the last play picked.
fn header(
    ui: &mut Ui,
    name: &str,
    path: &str,
    sounds: Sounds<'_>,
    requests: &mut Vec<LibraryRequest>,
    playable: bool,
) {
    let playing = sound_view::played(path, sounds.preview).is_some();
    ui.horizontal(|ui| {
        crate::icons::icon(
            ui.painter(),
            egui::Rect::from_min_size(
                ui.cursor().min + egui::vec2(0.0, 2.0),
                egui::vec2(14.0, 14.0),
            ),
            crate::icons::Icon::Sound,
            SOUND_COLOR,
        );
        ui.add_space(18.0);
        ui.label(RichText::new(name).strong());
        if playable {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if playing { "Stop" } else { "Play" };
                let button = ui.add(
                    egui::Button::new(format!("     {label}")).min_size(egui::vec2(70.0, 0.0)),
                );
                crate::icons::icon(
                    ui.painter(),
                    egui::Rect::from_center_size(
                        egui::pos2(button.rect.left() + 13.0, button.rect.center().y),
                        egui::vec2(11.0, 11.0),
                    ),
                    if playing {
                        crate::icons::Icon::Stop
                    } else {
                        crate::icons::Icon::Play
                    },
                    theme::TEXT_STRONG,
                );
                button.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("{label} the sound event"),
                    )
                });
                let button = button.on_hover_text(if playing {
                    "Stop listening"
                } else {
                    "Plays it as a game would: a different take, pitch and volume each press"
                });
                if button.clicked() {
                    requests.push(if playing {
                        LibraryRequest::StopSound
                    } else {
                        LibraryRequest::PlaySound(path.to_owned())
                    });
                }
            });
        }
    });
    let text = match sounds.preview.filter(|p| p.path == path) {
        Some(preview) => match &preview.detail {
            Some(detail) => format!("Playing {detail}"),
            None => "Playing".to_owned(),
        },
        None => "Each press of Play picks a different take, pitch and volume.".to_owned(),
    };
    ui.label(RichText::new(text).color(theme::TEXT_WEAK).size(11.0));
    ui.add_space(4.0);
}

/// The event's settings, edited in place.
fn editor(
    ui: &mut Ui,
    path: &str,
    saved: &SoundEvent,
    sounds: Sounds<'_>,
    library: &[LibraryItem],
    requests: &mut Vec<LibraryRequest>,
) {
    // What's being edited: the value being dragged, kept in memory until
    // the mouse is let go, or the file's.
    let key = egui::Id::new(("sound event edit", path));
    let mut event: SoundEvent = ui
        .data(|d| d.get_temp::<SoundEvent>(key))
        .unwrap_or_else(|| saved.clone());
    let before = event.clone();

    takes(ui, path, &mut event, sounds, library, requests);
    section(ui, "How it sounds");
    inspector::row(
        ui,
        "Volume",
        "How loud, in decibels: 0 is as recorded, -6 about half as loud. Each play is up to the \
         second number louder or quieter.",
        |ui| {
            ui.add(
                egui::DragValue::new(&mut event.volume_db)
                    .range(VOLUME_RANGE_DB.0..=VOLUME_RANGE_DB.1)
                    .speed(0.1)
                    .suffix(" dB"),
            );
            ui.label("±");
            ui.add(
                egui::DragValue::new(&mut event.volume_range_db)
                    .range(0.0..=MAX_VOLUME_VARIATION_DB)
                    .speed(0.1)
                    .suffix(" dB"),
            );
        },
    );
    inspector::row(
        ui,
        "Pitch",
        "How high or low, in semitones: 12 is an octave up. Each play is up to the second number \
         higher or lower, so repeated sounds don't sound copied.",
        |ui| {
            ui.add(
                egui::DragValue::new(&mut event.pitch_st)
                    .range(PITCH_RANGE_ST.0..=PITCH_RANGE_ST.1)
                    .speed(0.05)
                    .suffix(" st"),
            );
            ui.label("±");
            ui.add(
                egui::DragValue::new(&mut event.pitch_range_st)
                    .range(0.0..=MAX_PITCH_VARIATION_ST)
                    .speed(0.05)
                    .suffix(" st"),
            );
        },
    );
    inspector::row(
        ui,
        "Bus",
        "Which group of sounds it belongs to, so a game's options can turn music, effects and \
         voices up or down separately.",
        |ui| {
            egui::ComboBox::from_id_salt(("bus", path))
                .selected_text(event.bus.clone())
                .show_ui(ui, |ui| {
                    for bus in BUSES {
                        ui.selectable_value(&mut event.bus, bus.to_owned(), bus);
                    }
                });
        },
    );
    inspector::row(
        ui,
        "Loop",
        "Plays again from the start when it ends: an engine, a campfire.",
        |ui| {
            ui.checkbox(&mut event.looping, "");
        },
    );

    section(ui, "Where it's heard");
    inspector::row(
        ui,
        "Space",
        "3D sounds come from a place in the world: quieter far away, from their direction. 2D \
         sounds are the same everywhere: music, the interface.",
        |ui| {
            ui.selectable_value(&mut event.space, Space::Flat, "2D");
            ui.selectable_value(&mut event.space, Space::Placed, "3D");
        },
    );
    let placed = event.space == Space::Placed;
    ui.add_enabled_ui(placed, |ui| {
        inspector::row(
            ui,
            "Full until",
            "Full volume up to this far from the listener, in metres.",
            |ui| {
                ui.add(
                    egui::DragValue::new(&mut event.near_m)
                        .range(0.0..=event.far_m)
                        .speed(0.1)
                        .suffix(" m"),
                );
            },
        );
        inspector::row(
            ui,
            "Silent past",
            "Not heard at all past this far from the listener, in metres.",
            |ui| {
                ui.add(
                    egui::DragValue::new(&mut event.far_m)
                        .range(event.near_m.max(0.01)..=MAX_DISTANCE_M)
                        .speed(0.5)
                        .suffix(" m"),
                );
            },
        );
        inspector::row(
            ui,
            "Fade",
            "How it gets quieter between the two distances. Realistic falls quickly at first, as \
             sound does outdoors; Even falls steadily; None stays full until the far distance.",
            |ui| {
                for fade in [Fade::Realistic, Fade::Even, Fade::None] {
                    ui.selectable_value(&mut event.fade, fade, fade.name());
                }
            },
        );
    });

    section(ui, "When many play");
    inspector::row(
        ui,
        "At once",
        "The most plays of this event heard at the same time; more are skipped. Keeps a burst \
         of gunfire from drowning everything out.",
        |ui| {
            ui.add(egui::DragValue::new(&mut event.at_once).range(1..=MAX_AT_ONCE));
            ui.label("of this event");
        },
    );
    inspector::row(
        ui,
        "Priority",
        "When too many sounds play, low ones are dropped first and high ones kept longest.",
        |ui| {
            for priority in [Priority::Low, Priority::Normal, Priority::High] {
                ui.selectable_value(&mut event.priority, priority, priority.name());
            }
        },
    );

    // Saved once the mouse is let go: a drag saves once, not every frame.
    let pointer_down = ui.input(|i| i.pointer.any_down());
    let editing = event != *saved;
    if editing && pointer_down {
        ui.data_mut(|d| d.insert_temp(key, event));
    } else {
        ui.data_mut(|d| d.remove::<SoundEvent>(key));
        if editing && (event != before || !pointer_down) {
            requests.push(LibraryRequest::SaveSoundEvent(path.to_owned(), event));
        }
    }
}

/// A heading between groups of settings.
fn section(ui: &mut Ui, text: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(text).color(theme::TEXT_WEAK).size(12.0));
}

/// The takes, each with its waveform, length and ×; sound files dropped
/// on the list are added.
fn takes(
    ui: &mut Ui,
    path: &str,
    event: &mut SoundEvent,
    sounds: Sounds<'_>,
    library: &[LibraryItem],
    requests: &mut Vec<LibraryRequest>,
) {
    section(
        ui,
        &format!(
            "Takes ({} of up to {MAX_TAKES}) · drop sound files here",
            event.takes.len()
        ),
    );
    let frame = egui::Frame::new()
        .fill(theme::FIELD)
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(6, 4));
    let (zone, dropped) = ui.dnd_drop_zone::<LibraryDrag, _>(frame, |ui| {
        ui.set_min_width(ui.available_width());
        if event.takes.is_empty() {
            ui.label(
                RichText::new(
                    "No takes yet. Drag sound files here from the Library: each play picks one.",
                )
                .color(theme::TEXT_WEAK),
            );
        }
        let mut remove = None;
        for (i, id) in event.takes.iter().enumerate() {
            let item = library.iter().find(|item| item.id == Some(*id));
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(40.0, 14.0), egui::Sense::hover());
                let state = item.and_then(|item| sounds.states.get(&item.path));
                if let Some(SoundState::Read(info)) = state {
                    sound_view::draw_waveform(ui.painter(), rect, &info.waveform, None);
                }
                match item {
                    Some(item) => {
                        ui.label(&item.name);
                    }
                    None => {
                        ui.label(RichText::new(format!("{id} (missing)")).color(theme::TEXT_WEAK))
                            .on_hover_text(
                                "No sound file in the project has this ID: it was deleted, or its \
                             .solmeta file was lost. Plays skip it.",
                            );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let x = ui
                        .add(egui::Button::new("×").frame(false))
                        .on_hover_text("Take it out of this event (the file stays)");
                    let name = item.map_or_else(|| id.to_string(), |item| item.name.clone());
                    x.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Remove {name}"),
                        )
                    });
                    if x.clicked() {
                        remove = Some(i);
                    }
                    if let Some(SoundState::Read(info)) = state {
                        ui.label(
                            RichText::new(sound_view::length_text(info.seconds))
                                .color(theme::TEXT_WEAK),
                        );
                    }
                });
            });
        }
        if let Some(i) = remove {
            event.takes.remove(i);
        }
    });
    let _ = zone;
    if let Some(dropped) = dropped {
        requests.extend(add_take(path, &dropped.0));
    }
}

/// What dropping `dragged` on the event at `path` asks for: adding it as a
/// take when it's a sound file.
pub(crate) fn add_take(path: &str, dragged: &LibraryItem) -> Option<LibraryRequest> {
    match (dragged.kind, dragged.id) {
        (LibraryKind::Sound, Some(id)) => Some(LibraryRequest::AddTake(path.to_owned(), id)),
        _ => None,
    }
}
