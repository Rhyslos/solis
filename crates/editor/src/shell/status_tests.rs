use super::*;
use crate::test_support::info;

fn fresh() -> Workbench {
    Workbench::new(info())
}

#[test]
fn before_the_first_check_the_code_isnt_mentioned() {
    let w = fresh();
    let status = w.status();
    assert_eq!(status.code, None);
    assert_eq!(status.problems, 0);
    assert_eq!(status.playing, None);
}

#[test]
fn the_code_is_checked_or_has_mistakes() {
    let mut w = fresh();
    w.set_code_problems(Vec::new(), &[]);
    let code = w.status().code.unwrap();
    assert_eq!(code.text, "Code checked");
    assert_eq!(code.mark, Some(Mark::Tick));
    assert!(code.more.unwrap().contains("last checked at"));
    let mistake = script::CodeProblem {
        file: "main.lst".into(),
        line: 3,
        message: "an unknown name".into(),
        column: 1,
    };
    w.set_code_problems(vec![mistake.clone()], &[]);
    let status = w.status();
    assert_eq!(status.code.unwrap().text, "1 mistake in the code");
    assert_eq!(status.problems, 1, "a code mistake is a problem too");
    w.set_code_problems(vec![mistake.clone(), mistake], &[]);
    let code = w.status().code.unwrap();
    assert_eq!(code.text, "2 mistakes in the code");
    assert_eq!(code.mark, Some(Mark::Warning));
}

#[test]
fn a_level_says_whether_it_is_saved() {
    let mut w = fresh();
    assert_eq!(w.status().saved.text, "Not saved yet");
    w.add_point_light();
    assert_eq!(w.status().saved.text, "Unsaved changes");
}

#[test]
fn the_level_is_named_with_2d_or_3d_and_a_2d_games_size() {
    let mut w = fresh();
    assert!(w.status().level.ends_with(" · 3D"), "{}", w.status().level);
    w.viewport.camera.kind = crate::camera::ViewKind::TwoD;
    w.open.pixels.base = [480, 270];
    assert!(
        w.status().level.ends_with(" · 2D · 480 × 270"),
        "{}",
        w.status().level
    );
}

#[test]
fn playing_and_paused_show_on_the_right() {
    let mut w = fresh();
    w.play();
    let playing = w.status().playing.unwrap();
    assert_eq!(
        (playing.text.as_str(), playing.mark),
        ("Playing", Some(Mark::Playing))
    );
    w.pause();
    assert_eq!(w.status().playing.unwrap().text, "Paused");
    w.stop();
    assert_eq!(w.status().playing, None);
}

/// Save left Home's ribbon (decision 0118): the status line's "Not saved
/// yet" or "Saved" saves with a click, as Ctrl + S does.
#[test]
fn clicking_saved_or_not_saves_the_level() {
    use egui_kittest::kittest::Queryable;
    let mut h = crate::test_support::harness(fresh());
    h.run();
    h.get_by_label("Not saved yet: click to save").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&crate::WorkbenchAction::SaveLevel),
        "{:?}",
        h.state().1.actions
    );
    // The rest of the line is there too; the problems' counts are the
    // bottom panel's now.
    assert!(h.query_by_label_contains(" · 3D").is_some());
    assert!(h.query_by_label("No problems").is_none());
}

#[test]
fn what_is_selected_is_said() {
    let mut w = fresh();
    assert_eq!(w.status().selected, None);
    let one = w.add_entity();
    let name = w.open.document.level().get(one).unwrap().name.clone();
    assert_eq!(w.status().selected, Some(format!("{name} selected")));
    let two = w.add_entity();
    w.open.document.selection.select_all([one, two], two);
    assert_eq!(w.status().selected.as_deref(), Some("2 selected"));
}

#[test]
fn a_click_on_an_unfinished_part_is_said_on_the_left_for_three_seconds() {
    use crate::unbuilt::{self, UnbuiltId};
    use egui_kittest::kittest::Queryable;
    let mut h = crate::test_support::harness(fresh());
    h.run();
    let saved = "Not saved yet: click to save";
    assert!(h.query_by_label(saved).is_some());
    // What a click on the part does, and nothing else.
    unbuilt::say(&h.ctx, UnbuiltId::HomeLightingBake);
    h.step();
    let words = unbuilt::get(UnbuiltId::HomeLightingBake).click_text();
    assert_eq!(words, "Bake isn't built yet (3D 5: Bounced light)");
    assert!(h.query_by_label(&words).is_some());
    // The words it took the place of come back by themselves; the
    // right's stay.
    assert!(h.query_by_label(saved).is_none());
    assert!(h.query_by_label_contains(" · 3D").is_some());
    // A step of the test window is a quarter of a second.
    for _ in 0..10 {
        h.step();
    }
    assert!(
        h.query_by_label(&words).is_some(),
        "still there after 2.5 s"
    );
    for _ in 0..3 {
        h.step();
    }
    assert!(h.query_by_label(&words).is_none(), "gone after 3 s");
    assert!(h.query_by_label(saved).is_some());
    // The Console wasn't told, and the rest of the line never changed.
    assert!(h.query_by_label_contains(" · 3D").is_some());
}
