//! Leaving while Keep or Undo all is unanswered (the review, BUG-04):
//! decision 0085 promises the edits made while playing are "kept or
//! undone, never lost", so quitting, closing the window, the launcher,
//! another area or project and a file changed on disk all wait for the
//! answer. Each path is pinned to one outcome here.

use super::*;
use crate::PlayState;
use crate::test_support::{harness, info};
use crate::workbench::WorkbenchAction;
use egui_kittest::kittest::Queryable;
use reflect::EntityId;

/// A workbench whose area is saved (nothing unsaved), playing, with one
/// edit of the user's made while playing: the lamp.
fn playing_with_an_edit() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info());
    w.open.document.mark_saved("harbour.solarea".into());
    w.play();
    let lamp = w.add_point_light();
    (w, lamp)
}

/// As [`playing_with_an_edit`], stopped: the question waits.
fn asked() -> (Workbench, EntityId) {
    let (mut w, lamp) = playing_with_an_edit();
    w.stop();
    assert_eq!(w.play_changes().unwrap(), ["Add Point light 1"]);
    (w, lamp)
}

/// What `w` hands the app with its next frame.
fn handed_over(w: &mut Workbench) -> Vec<WorkbenchAction> {
    std::mem::take(&mut w.open.outbox)
}

/// Every action that puts another area or project in the open one's
/// place, or closes the editor.
fn leaving() -> Vec<WorkbenchAction> {
    vec![
        WorkbenchAction::NewArea,
        WorkbenchAction::OpenArea,
        WorkbenchAction::OpenAreaFile("/p/assets/other.solarea".into()),
        WorkbenchAction::NewProject,
        WorkbenchAction::OpenProject,
        WorkbenchAction::OpenRecent("/other".into()),
        WorkbenchAction::MakeSampleGame,
        WorkbenchAction::MakeUiSample,
        WorkbenchAction::MakeBenchmark,
        WorkbenchAction::MakePlayground,
        WorkbenchAction::Quit,
    ]
}

#[test]
fn quitting_while_playing_asks_about_the_edits_before_it_closes() {
    // File > Exit while playing, after an edit, with a saved area: the
    // editor closed with no question and the edit was gone.
    let (mut w, lamp) = playing_with_an_edit();
    let mut actions = Vec::new();
    w.request(WorkbenchAction::Quit, &mut actions);
    assert_eq!(w.play_state(), PlayState::Editing, "the game stopped");
    assert_eq!(actions, [], "nothing closes yet");
    assert_eq!(w.play_changes().unwrap(), ["Add Point light 1"]);
    assert!(!w.close_requested(), "and the window stays");
    // Undo all: nothing is left to lose, so it closes.
    w.undo_play_changes();
    assert_eq!(handed_over(&mut w), [WorkbenchAction::Quit]);
    assert!(w.close_requested());
    assert!(!w.document().area().contains(lamp));

    // Keep: the edit is in the area and isn't saved, so that is asked
    // next, as for any unsaved change.
    let (mut w, lamp) = playing_with_an_edit();
    let mut actions = Vec::new();
    w.request(WorkbenchAction::Quit, &mut actions);
    w.keep_play_changes();
    assert!(w.document().area().contains(lamp));
    assert_eq!(handed_over(&mut w), [], "not closed");
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));
    assert!(!w.close_requested());
}

#[test]
fn closing_the_window_while_playing_asks_about_the_edits_first() {
    let (mut w, lamp) = playing_with_an_edit();
    assert!(!w.close_requested(), "the close is called off");
    assert_eq!(w.play_state(), PlayState::Editing, "stopped, to ask");
    assert!(w.play_changes().is_some());
    assert!(w.open.unsaved_prompt.is_none(), "one question at a time");
    w.keep_play_changes();
    assert!(w.document().area().contains(lamp));
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));

    let (mut w, _) = playing_with_an_edit();
    assert!(!w.close_requested());
    w.undo_play_changes();
    assert_eq!(handed_over(&mut w), [WorkbenchAction::Quit]);
    assert!(w.close_requested(), "nothing to lose now");
}

#[test]
fn closing_the_window_with_the_question_waiting_waits_for_the_answer() {
    let (mut w, _) = asked();
    assert!(!w.close_requested());
    assert!(!w.close_requested(), "however often it is tried");
    assert!(w.play_changes().is_some(), "still asked");
    w.undo_play_changes();
    assert_eq!(
        handed_over(&mut w),
        [WorkbenchAction::Quit],
        "asked for once"
    );
    assert!(w.close_requested());
}

#[test]
fn nothing_that_leaves_the_area_goes_ahead_until_the_question_is_answered() {
    for action in leaving() {
        // Undo all: the action goes ahead once answered.
        let (mut w, _) = asked();
        let mut actions = Vec::new();
        w.request(action.clone(), &mut actions);
        assert_eq!(actions, [], "{action:?} waits");
        assert!(w.open.unsaved_prompt.is_none(), "{action:?}");
        assert!(w.play_changes().is_some(), "{action:?}: still asked");
        w.undo_play_changes();
        assert_eq!(
            handed_over(&mut w),
            std::slice::from_ref(&action),
            "then it goes"
        );
        // Keep: the edits are unsaved, so the action asks about that.
        let (mut w, lamp) = asked();
        let mut actions = Vec::new();
        w.request(action.clone(), &mut actions);
        w.keep_play_changes();
        assert!(w.document().area().contains(lamp), "{action:?}: kept");
        assert_eq!(handed_over(&mut w), [], "{action:?}");
        assert_eq!(w.open.unsaved_prompt, Some(action));
    }
}

#[test]
fn what_doesn_t_leave_the_area_goes_ahead_while_the_question_waits() {
    let (mut w, _) = asked();
    for action in [
        WorkbenchAction::ImportModel,
        WorkbenchAction::CheckForUpdates,
        WorkbenchAction::ListProjects,
    ] {
        let mut actions = Vec::new();
        w.request(action.clone(), &mut actions);
        assert_eq!(actions, [action]);
    }
    assert!(w.play_changes().is_some());
}

#[test]
fn a_quit_handed_straight_to_the_app_while_playing_waits_too() {
    let (mut w, _) = playing_with_an_edit();
    let mut actions = vec![WorkbenchAction::Quit];
    w.hold_while_playing(&mut actions);
    assert_eq!(w.play_state(), PlayState::Editing);
    assert_eq!(actions, [], "not while the edits are unanswered");
    w.undo_play_changes();
    assert_eq!(handed_over(&mut w), [WorkbenchAction::Quit]);
    // With no edit of the user's it closes as before.
    let mut w = Workbench::new(info());
    w.play();
    let mut actions = vec![WorkbenchAction::Quit];
    w.hold_while_playing(&mut actions);
    assert_eq!(actions, [WorkbenchAction::Quit]);
}

#[test]
fn the_question_is_asked_over_recent_projects() {
    // Closing the editor's window with a project open goes back to
    // Recent projects, stopping the game: the question must be drawn
    // over the page, never left unseen while another project opens.
    let (mut w, lamp) = playing_with_an_edit();
    w.show_recent_projects();
    assert!(w.file_page_open());
    assert_eq!(w.play_state(), PlayState::Editing);
    let mut h = harness(w);
    h.run();
    let _ = h.get_by_label("Keep the changes made while playing?");
    // Another project asked for meanwhile (Enter on the list) waits.
    let other = WorkbenchAction::OpenRecent("/other".into());
    let mut actions = Vec::new();
    h.state_mut().0.request(other.clone(), &mut actions);
    assert_eq!(actions, []);
    h.get_by_label("Keep").click();
    h.run();
    let (w, picker) = h.state();
    assert!(w.play_changes().is_none());
    assert!(w.document().area().contains(lamp), "kept");
    assert!(
        !picker.actions.contains(&other),
        "the kept edits aren't saved"
    );
    assert_eq!(w.open.unsaved_prompt, Some(other));
}

#[test]
fn a_file_changed_on_disk_waits_for_the_answer() {
    let theirs = || {
        let mut area = area::builtin::starter_area();
        let id = area.spawn(area::EntityData::new("Theirs"));
        (area, id)
    };
    // Changed while the question waits. Undo all: nothing is unsaved, so
    // the file is loaded.
    let (mut w, _) = asked();
    let (area, theirs_id) = theirs();
    w.offer_reload(area);
    assert!(!w.document().area().contains(theirs_id), "not yet");
    assert!(w.open.pending_reload.is_none());
    w.undo_play_changes();
    assert!(w.document().area().contains(theirs_id), "loaded");
    // Keep: the kept edits are unsaved, so Keep mine or Load the file is
    // asked, where Keep used to put the old area back over the new file.
    let (mut w, lamp) = asked();
    let (area, theirs_id) = theirs();
    w.offer_reload(area);
    w.keep_play_changes();
    assert!(w.document().area().contains(lamp));
    assert!(!w.document().area().contains(theirs_id));
    assert!(w.open.pending_reload.is_some(), "asked which to keep");
    // Changed while playing: offered after Stop, so after the answer too.
    let (mut w, _) = playing_with_an_edit();
    let (area, theirs_id) = theirs();
    w.offer_reload(area);
    w.stop();
    assert!(w.play_changes().is_some());
    assert!(!w.document().area().contains(theirs_id));
    assert!(w.open.pending_reload.is_none());
    w.undo_play_changes();
    assert!(w.document().area().contains(theirs_id));
}

#[test]
fn another_area_put_in_place_takes_the_question_with_the_old_one() {
    // Asking comes before `new_area` and `open_area` (`request`); if one
    // is called all the same, Keep must not put the old area back over
    // the one just opened.
    let (mut w, lamp) = asked();
    w.open_area(area::builtin::starter_area(), "/p/assets/b.solarea".into());
    assert!(w.play_changes().is_none());
    w.keep_play_changes();
    assert!(!w.document().area().contains(lamp));
    assert_eq!(
        w.document().file(),
        Some(std::path::Path::new("/p/assets/b.solarea"))
    );
    let (mut w, lamp) = asked();
    w.new_area();
    assert!(w.play_changes().is_none());
    w.keep_play_changes();
    assert!(!w.document().area().contains(lamp));
}

#[test]
fn edits_made_while_the_unsaved_question_was_up_are_asked_about_too() {
    // Closing the window while playing with unsaved changes from before
    // Play asks about those and leaves the game running. An edit made
    // then, and the question answered: the edit was lost with the game.
    let mut w = Workbench::new(info());
    w.add_spotlight();
    w.play();
    assert!(!w.close_requested());
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::Quit));
    assert_eq!(w.play_state(), PlayState::Playing, "Cancel would go on");
    let lamp = w.add_point_light();
    let mut h = harness(w);
    h.run();
    h.get_by_label("Don't save").click();
    h.run();
    let (w, picker) = h.state();
    assert_eq!(w.play_state(), PlayState::Editing);
    assert!(!picker.actions.contains(&WorkbenchAction::Quit), "not yet");
    assert_eq!(w.play_changes().unwrap(), ["Add Point light 1"]);
    // Keep, then the question about saving again, now about everything.
    h.get_by_label("Keep").click();
    h.run();
    h.run();
    assert!(h.state().0.document().area().contains(lamp));
    assert!(!h.state().1.actions.contains(&WorkbenchAction::Quit));
    h.get_by_label("Don't save").click();
    h.run();
    h.run();
    assert!(h.state().1.actions.contains(&WorkbenchAction::Quit));
    assert!(h.state_mut().0.close_requested());
}

#[test]
fn the_sample_projects_ask_about_unsaved_changes_like_any_other_project() {
    // They put another project and area in the open one's place, so they
    // go through the same check.
    for action in leaving() {
        let mut w = Workbench::new(info());
        w.add_point_light();
        let mut actions = Vec::new();
        w.request(action.clone(), &mut actions);
        assert_eq!(actions, [], "{action:?}");
        assert_eq!(w.open.unsaved_prompt, Some(action));
    }
}
