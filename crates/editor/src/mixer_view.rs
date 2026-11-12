//! The Audio tab's state (Phase 7, S4; decision 0056; Screen 10 and
//! round 3's `08-audio-mixer.png`): the project's buses, mute and solo,
//! the meters, the bus the Bus group shows and the sound event picked on
//! the shelf. The console of strips is `mixer_console.rs`, the Bus group
//! `bus_view.rs`, the Sound events shelf tab `sound_events_view.rs`; each
//! draws from this and asks for changes as `MixerRequest`s.
//!
//! The buses are the project's (`mixer.solmix`, read and written by the
//! app). An edit is saved once the mouse button is let go
//! (`WorkbenchAction::SaveMixer`), so dragging a fader saves once. Mute
//! and solo aren't saved: they're for listening while working. None of it
//! is an undo step: it's the project's sound, not the area.

use crate::document::Selection;
use crate::theme;
use egui::Ui;
use sound::mixer::{Listening, MASTER, Mixer, SILENT_DB};

/// How fast a meter falls back, in decibels a second, so a peak can be
/// seen.
const METER_FALL_DB_PER_S: f32 = 24.0;
/// How long a meter's peak line holds before it falls (spec 811: "about
/// a second").
const PEAK_HOLD_S: f32 = 1.0;

/// The Mixer's state in the editor: the project's buses, mute and solo,
/// what the meters show, the bus the Bus group shows and the sound event
/// picked.
#[derive(Debug, Clone, Default)]
pub struct MixerState {
    /// The project's buses.
    pub(crate) mixer: Mixer,
    /// Mute and solo, and the Game panel's mute.
    pub(crate) listening: Listening,
    /// What each meter shows, in decibels (falling back after a peak).
    pub(crate) meters: Vec<f32>,
    /// Each meter's peak line, in decibels, and how long it has held.
    pub(crate) held: Vec<(f32, f32)>,
    /// Each bus's ducking now, in decibels, while the game plays.
    pub(crate) ducked: Vec<f32>,
    /// The bus picked, and the area's selection then (a new selection on
    /// another tab forgets it; the Bus group then shows Master).
    pub(crate) shown: Option<(usize, Selection)>,
    /// The Bus group shows the bus's effects (unfinished) in place of
    /// its ducking.
    pub(crate) effects: bool,
    /// The sound event picked in the Sound events shelf tab (its path).
    pub(crate) event: Option<String>,
}

impl MixerState {
    /// Takes the meters' newest peaks (amplitudes, 1 full scale) and each
    /// bus's ducking, `dt` seconds after the last: a meter jumps up to a
    /// peak and falls back slowly; its peak line holds a second first.
    pub(crate) fn update_meters(&mut self, peaks: &[f32], ducked: &[f32], dt: f32) {
        let n = self.mixer.buses.len();
        self.meters.resize(n, SILENT_DB);
        self.held.resize(n, (SILENT_DB, 0.0));
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fall = METER_FALL_DB_PER_S * dt;
        for (i, (shown, held)) in self.meters.iter_mut().zip(&mut self.held).enumerate() {
            let peak = peaks.get(i).copied().unwrap_or(0.0);
            let db = if peak.is_finite() && peak > 0.0 {
                (20.0 * peak.log10()).max(SILENT_DB)
            } else {
                SILENT_DB
            };
            *shown = db.max(*shown - fall).max(SILENT_DB);
            if *shown >= held.0 {
                *held = (*shown, 0.0);
            } else if held.1 + dt < PEAK_HOLD_S {
                held.1 += dt;
            } else {
                held.0 = (held.0 - fall).max(*shown);
            }
        }
        self.ducked = ducked.to_vec();
        self.ducked.resize(n, 0.0);
    }

    /// Whether a meter is still moving (so the editor keeps drawing).
    pub(crate) fn meters_moving(&self) -> bool {
        self.meters.iter().any(|m| *m > SILENT_DB)
            || self.held.iter().any(|(h, _)| *h > SILENT_DB)
            || self.ducked.iter().any(|d| *d < 0.0)
    }

    /// The meter of bus `at` now, and its peak line, in decibels.
    pub(crate) fn meter(&self, at: usize) -> (f32, f32) {
        let level = self.meters.get(at).copied().unwrap_or(SILENT_DB);
        let held = self.held.get(at).map_or(level, |(h, _)| *h).max(level);
        (level, held)
    }

    /// The bus the Bus group shows: the one picked, or Master.
    pub(crate) fn bus_shown(&self) -> usize {
        self.shown
            .as_ref()
            .map(|(at, _)| *at)
            .filter(|at| *at < self.mixer.buses.len())
            .unwrap_or(0)
    }

    /// The buses with ducking working on them now, as "Voice ducking
    /// Music", for the shelf bar while the game plays.
    pub(crate) fn ducking_now(&self) -> Vec<String> {
        self.mixer
            .buses
            .iter()
            .zip(&self.ducked)
            .filter(|(_, d)| **d < -0.05)
            .flat_map(|(bus, _)| {
                bus.ducking
                    .iter()
                    .map(move |r| format!("{} ducking {}", r.while_bus, bus.name))
            })
            .collect()
    }
}

/// What a click or drag in the Audio tab asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MixerRequest {
    /// Show this bus in the Bus group.
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
    /// Unsolo every bus.
    ClearSolo,
    /// Mute or unmute everything (the Game tab's button, and Monitor's
    /// Mute the game).
    MuteAll,
    /// A click on a meter: its peak line drops to the level now.
    ResetPeak(usize),
    /// The Bus group shows the effects side (`true`) or the ducking.
    ShowEffects(bool),
    /// Pick this sound event in the Sound events shelf tab.
    PickEvent(String),
    /// The Effects side's page asked for unfinished parts to be hidden
    /// (the window's own work: it needs the window).
    HideUnfinished,
}

/// The bus a new bus goes inside, a rule is added to, or Remove removes:
/// the one the Bus group shows.
pub(crate) fn with_bus_shown(state: &MixerState) -> (usize, &sound::mixer::Bus) {
    let at = state.bus_shown();
    (at, &state.mixer.buses[at])
}

/// A new bus inside the one shown (or Master), picked: the ribbon's Bus.
pub(crate) fn add_bus(state: &MixerState) -> Vec<MixerRequest> {
    let mut mixer = state.mixer.clone();
    let inside = mixer
        .buses
        .get(state.bus_shown())
        .map_or(MASTER.to_owned(), |b| b.name.clone());
    match mixer.add_bus(&inside) {
        Some(at) => vec![
            MixerRequest::Changed { mixer, save: true },
            MixerRequest::Select(at),
        ],
        None => Vec::new(),
    }
}

impl crate::Workbench {
    /// Uses the project's buses (as read from its `mixer.solmix`, or the
    /// standard ones). Mute and solo stay on buses that are still there.
    pub fn set_mixer(&mut self, mixer: Mixer) {
        let names: Vec<&str> = mixer.buses.iter().map(|b| b.name.as_str()).collect();
        let listening = &mut self.open.mixer.listening;
        listening.muted.retain(|n| names.contains(&n.as_str()));
        listening.soloed.retain(|n| names.contains(&n.as_str()));
        if self
            .open
            .mixer
            .shown
            .as_ref()
            .is_some_and(|(at, _)| *at >= mixer.buses.len())
        {
            self.open.mixer.shown = None;
        }
        self.open.mixer.mixer = mixer;
    }

    /// The project's buses, as the Mixer has them now.
    pub fn mixer(&self) -> &Mixer {
        &self.open.mixer.mixer
    }

    /// Mute and solo, and the Game panel's mute.
    pub fn listening(&self) -> &Listening {
        &self.open.mixer.listening
    }

    /// The meters' newest peaks (amplitudes) and each bus's ducking now,
    /// `dt` seconds after the last call; keeps drawing while they move.
    pub fn set_bus_meters(&mut self, peaks: &[f32], ducked: &[f32], dt: f32, ctx: &egui::Context) {
        self.open.mixer.update_meters(peaks, ducked, dt);
        if self.open.mixer.meters_moving() && self.in_front.contains(&crate::Panel::Mixer) {
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
                self.open.mixer.shown = Some((at, self.open.document.selection.clone()));
                self.open.sound_shown = None;
            }
            MixerRequest::Changed { mixer, save } => {
                self.set_mixer(mixer);
                if save {
                    if self.open.project.is_some() {
                        actions.push(crate::WorkbenchAction::SaveMixer(
                            self.open.mixer.mixer.clone(),
                        ));
                    } else {
                        self.log(
                            crate::LogLevel::Info,
                            "The buses are kept with a project: open or make one to keep these changes.",
                        );
                    }
                }
            }
            MixerRequest::Mute(at) => flip(
                &mut self.open.mixer.listening.muted,
                &self.open.mixer.mixer,
                at,
            ),
            MixerRequest::Solo(at) => flip(
                &mut self.open.mixer.listening.soloed,
                &self.open.mixer.mixer,
                at,
            ),
            MixerRequest::ClearSolo => self.open.mixer.listening.soloed.clear(),
            MixerRequest::MuteAll => {
                self.open.mixer.listening.all_muted = !self.open.mixer.listening.all_muted;
            }
            MixerRequest::ResetPeak(at) => {
                let level = self.open.mixer.meter(at).0;
                if let Some(held) = self.open.mixer.held.get_mut(at) {
                    *held = (level, 0.0);
                }
            }
            MixerRequest::ShowEffects(effects) => self.open.mixer.effects = effects,
            MixerRequest::PickEvent(path) => self.open.mixer.event = Some(path),
            // Done by the frame, which has the window (`Workbench::show`).
            MixerRequest::HideUnfinished => {}
        }
    }
}

/// The Game tab's mute button, in its top right corner: a speaker,
/// crossed out in amber (mute's colour, spec 784) while everything is
/// muted. Returns whether it was
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
        theme::hover()
    } else {
        theme::overlay()
    };
    painter.rect_filled(rect, 4.0, fill);
    crate::icons::icon(
        painter,
        egui::Rect::from_center_size(rect.center(), egui::vec2(13.0, 13.0)),
        crate::icons::Icon::Sound,
        if muted {
            theme::text_weak()
        } else {
            theme::text_strong()
        },
    );
    if muted {
        painter.line_segment(
            [
                rect.left_bottom() + egui::vec2(6.0, -5.0),
                rect.right_top() + egui::vec2(-6.0, 5.0),
            ],
            egui::Stroke::new(2.0, crate::tokens::WARNING),
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
        let mut w = Workbench::new(info());
        w.set_project(Some(crate::ProjectInfo {
            name: "Test".into(),
            root: std::path::PathBuf::from("/nowhere"),
            assets: std::path::PathBuf::from("/nowhere/assets"),
        }));
        let mut h = harness(w);
        h.run();
        h.state_mut().0.show_panel(Panel::Mixer);
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
    fn the_mixer_is_the_audio_tab_s_document_and_shows_every_bus() {
        let h = with_the_mixer();
        assert_eq!(h.state().0.ribbon_tab(), crate::shell::RibbonTab::Audio);
        assert!(crate::test_support::in_view(&h.state().0, Panel::Mixer));
        for bus in ["Master", "Music", "Effects", "Voice", "Ambience", "UI"] {
            assert!(h.query_all_by_label(bus).next().is_some(), "{bus}");
        }
        assert!(
            h.query_by_label("Meters move while the game plays")
                .is_some()
        );
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
    fn clicking_a_bus_shows_it_in_the_bus_group_and_add_bus_saves() {
        let mut h = with_the_mixer();
        h.get_by_label("Music").click();
        h.run();
        assert_eq!(h.state().0.open.mixer.shown.as_ref().map(|s| s.0), Some(1));
        assert!(h.query_by_label("Quieter while").is_some());
        assert!(h.query_by_label("Remove bus").is_some());
        // A new bus goes inside the one shown (the ribbon's Bus).
        h.get_by_label("Add a bus").click();
        h.run();
        let saves = saved(&mut h);
        assert_eq!(saves.len(), 1);
        let m = &saves[0];
        let new = m.index_of("Bus").unwrap();
        assert_eq!(m.buses[new].inside, "Music");
        assert_eq!(
            h.state().0.open.mixer.shown.as_ref().map(|s| s.0),
            Some(new)
        );
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
        h.state_mut()
            .0
            .mixer_request(MixerRequest::Select(2), &mut Vec::new());
        h.run();
        assert!(h.state().0.open.mixer.shown.is_some());
        h.state_mut()
            .0
            .open
            .document
            .selection
            .select_only(crate_id);
        h.run();
        assert!(h.state().0.open.mixer.shown.is_none());
    }

    #[test]
    fn without_a_project_changes_say_where_they_d_be_kept() {
        let mut w = Workbench::new(info());
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
        let mut w = Workbench::new(info());
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
        let mut h = harness(Workbench::new(info()));
        h.run();
        h.state_mut().0.show_panel(Panel::Game);
        h.run();
        h.get_by_label("Mute the game").click();
        h.run();
        assert!(h.state().0.listening().all_muted);
    }
}
