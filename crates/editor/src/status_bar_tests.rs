use super::*;
use crate::test_support::info;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
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
fn an_area_says_whether_it_is_saved() {
    let mut w = fresh();
    assert_eq!(w.status().saved.text, "Not saved yet");
    w.add_point_light();
    assert_eq!(w.status().saved.text, "Unsaved changes");
}

#[test]
fn the_area_is_named_with_2d_or_3d_and_a_2d_games_size() {
    let mut w = fresh();
    assert!(w.status().area.ends_with(" · 3D"), "{}", w.status().area);
    w.viewport.camera.kind = crate::camera::ViewKind::TwoD;
    w.open.pixels.base = [480, 270];
    assert!(
        w.status().area.ends_with(" · 2D · 480 × 270"),
        "{}",
        w.status().area
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

#[test]
fn clicking_the_problems_opens_the_problems_panel() {
    use egui_kittest::kittest::Queryable;
    let mut h = crate::test_support::harness(fresh());
    h.run();
    assert!(!h.state().0.in_front.contains(&Panel::Problems));
    h.get_by_label("No problems").click();
    h.run();
    h.run();
    assert!(h.state().0.in_front.contains(&Panel::Problems));
    // The rest of the bar is there too.
    assert!(h.query_by_label("Not saved yet").is_some());
    assert!(h.query_by_label_contains(" · 3D").is_some());
}
