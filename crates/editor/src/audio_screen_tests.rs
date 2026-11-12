//! The Audio tab as round 3's `08-audio-mixer.png` draws it (M13): the
//! console with Master last, the fader's drag (Shift for a fine move)
//! saving once, a click on a meter, the Bus group and its switch, the
//! ribbon's commands, and the Sound events shelf tab.

use crate::mixer_view::MixerRequest;
use crate::panel::Panel;
use crate::shell::{RibbonTab, ShellCommand as C};
use crate::test_support::*;
use crate::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;
use sound::mixer::Mixer;

type Harness = egui_kittest::Harness<'static, State>;

/// The editor at the design's size with a project open and Audio in
/// front.
fn audio() -> Harness {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_ui_state(
            |ui, state: &mut State| {
                if !fonts_ready(ui.ctx()) {
                    return;
                }
                let (workbench, picker) = state;
                let actions = workbench.show(ui, picker);
                picker.actions.extend(actions);
            },
            (w, Default::default()),
        );
    h.run();
    h.state_mut().0.show_tab(RibbonTab::Audio);
    h.run();
    h.run();
    h
}

fn saved(h: &mut Harness) -> Vec<Mixer> {
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

/// The left edge of each strip, by the bus's name on its header.
fn strip_left(h: &Harness, bus: &str) -> f32 {
    h.get_all_by_label(bus)
        .map(|n| n.rect())
        .find(|r| (r.height() - crate::tokens::size::STRIP_HEADER_H).abs() < 0.5)
        .unwrap_or_else(|| panic!("{bus}'s strip"))
        .left()
}

#[test]
fn the_mixer_and_the_bus_are_the_audio_tab_s_groups_with_master_last() {
    let h = audio();
    let w = &h.state().0;
    assert!(in_view(w, Panel::Mixer));
    assert!(in_view(w, Panel::Bus));
    assert!(in_view(w, Panel::SoundEvents));
    let master = strip_left(&h, "Master");
    for bus in ["Music", "Effects", "Voice", "Ambience", "UI"] {
        assert!(strip_left(&h, bus) < master, "{bus} before Master");
    }
    // The mixer's tab is its file; the Bus group is headed by the bus.
    assert!(h.query_all_by_label("mixer.solmix").next().is_some());
    assert!(h.query_all_by_label("Master \u{203a} Bus").next().is_some());
}

#[test]
fn a_fader_drag_moves_it_shift_finer_and_saves_once_let_go() {
    let mut h = audio();
    let fader = h.get_by_label("Music's volume").rect();
    drag_for_frames(&mut h, fader.center(), egui::vec2(0.0, 6.0), 5);
    let plain = h.state().0.mixer().buses[1].volume_db;
    assert!(plain < -5.0, "down 30 points is quieter: {plain}");
    assert_eq!(saved(&mut h).len(), 1, "saved once let go");
    // The same drag with Shift moves it a tenth as far.
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    drag_for_frames(&mut h, fader.center(), egui::vec2(0.0, 6.0), 5);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    let fine = h.state().0.mixer().buses[1].volume_db;
    let moved = plain - fine;
    assert!(moved > 0.0 && moved < -plain / 5.0, "{plain} then {fine}");
    // A double-click puts it back to 0 dB, and saves.
    saved(&mut h);
    h.hover_at(fader.center());
    h.run();
    // (Both clicks in one frame: the test window's frames are too far
    // apart in time for two of them to be a double-click.)
    for pressed in [true, false, true, false] {
        h.input_mut().events.push(egui::Event::PointerButton {
            pos: fader.center(),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    h.run();
    assert_eq!(h.state().0.mixer().buses[1].volume_db, 0.0);
    assert!(!saved(&mut h).is_empty());
}

#[test]
fn a_click_on_a_meter_drops_its_peak_line_to_the_level() {
    let mut w = Workbench::new(info());
    w.open.mixer.update_meters(&[1.0, 1.0], &[], 0.0);
    // The level falls; the peak line holds.
    w.open.mixer.update_meters(&[0.0, 0.0], &[], 0.5);
    let (level, held) = w.open.mixer.meter(1);
    assert!(held > level, "{held} over {level}");
    w.mixer_request(MixerRequest::ResetPeak(1), &mut Vec::new());
    assert_eq!(w.open.mixer.meter(1), (level, level));
    // After a second the line falls with the meter.
    w.open.mixer.update_meters(&[1.0, 1.0], &[], 0.0);
    w.open.mixer.update_meters(&[], &[], 0.6);
    w.open.mixer.update_meters(&[], &[], 0.6);
    let (level, held) = w.open.mixer.meter(0);
    assert!(held < 0.0 && held >= level);
}

#[test]
fn the_ribbon_adds_a_bus_inside_the_one_shown_and_removes_it() {
    let mut h = audio();
    let w = &h.state().0;
    assert!(C::RemoveBus.why_not(w).is_some(), "Master stays");
    assert!(C::ClearSolo.why_not(w).is_some(), "nothing soloed");
    h.get_by_label("Music").click();
    h.run();
    run(&mut h, "Add a bus");
    let saves = saved(&mut h);
    assert_eq!(saves.len(), 1);
    let new = saves[0].index_of("Bus").unwrap();
    assert_eq!(saves[0].buses[new].inside, "Music");
    assert_eq!(h.state().0.open.mixer.bus_shown(), new);
    run(&mut h, "Remove the bus");
    assert!(saved(&mut h)[0].index_of("Bus").is_none());
    // Solo, then Clear solo.
    h.get_by_label_contains("Solo Voice").click();
    h.run();
    run(&mut h, "Clear solo");
    assert!(h.state().0.listening().soloed.is_empty());
    run(&mut h, "Mute the game");
    assert!(h.state().0.listening().all_muted);
    for command in [C::AddBus, C::AddDuckingRule, C::MuteGame] {
        let w = &h.state().0;
        let name = command.name(w);
        assert!(w.search_items().iter().any(|i| i.name == name), "{name}");
    }
}

#[test]
fn the_bus_group_switches_to_its_unfinished_effects_and_back() {
    let mut h = audio();
    // The switch's Effects, not the strip or the ribbon's group of that
    // name: the one a segment tall.
    let segment = |h: &Harness, word: &str| {
        h.get_all_by_label(word)
            .map(|n| n.rect())
            .find(|r| (r.height() - crate::tokens::size::SEGMENT_H).abs() < 1.0)
            .unwrap_or_else(|| panic!("the switch's {word}"))
    };
    let effects = segment(&h, "Effects");
    click(&mut h, effects.center(), egui::Modifiers::NONE);
    assert!(h.state().0.open.mixer.effects);
    assert!(h.query_all_by_label("Bus effects").next().is_some());
    let ducking = segment(&h, "Ducking");
    click(&mut h, ducking.center(), egui::Modifiers::NONE);
    assert!(!h.state().0.open.mixer.effects);
}

#[test]
fn sound_events_list_every_event_by_path_and_pick_one() {
    let mut h = audio();
    let read = |bus: &str| crate::sound_view::EventState::Read {
        event: sound::SoundEvent {
            bus: bus.into(),
            ..sound::SoundEvent::default()
        },
        notes: Vec::new(),
    };
    let events = [
        ("sfx/b_step.solsound", read("Effects")),
        ("music/a_theme.solsound", read("Music")),
    ]
    .into_iter()
    .map(|(p, e)| (p.to_owned(), e))
    .collect();
    h.state_mut().0.set_sound_events(events);
    h.run();
    h.run();
    // By path: music/ before sfx/, whatever the map's order.
    let theme = h.get_by_label("a_theme").rect();
    let step = h.get_by_label("b_step").rect();
    assert!(theme.top() < step.top());
    h.get_by_label("b_step").click();
    h.run();
    assert_eq!(
        h.state().0.open.mixer.event.as_deref(),
        Some("sfx/b_step.solsound")
    );
}
