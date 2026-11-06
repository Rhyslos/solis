//! A screen open in the UI editor (decision 0089).

use super::*;

pub(crate) const MENU: &str = "fetch \"solis/screen\":Ui;\n\n-- The pause menu.\nassembly PauseMenu {\n    has { int clicks = 0; }\n\n    view {\n        stack [width fill, height fill, center, middle] {\n            column [gap 14, background gray9, pad 24, round 8] {\n                text \"Paused\" [size 28]; -- the title\n                button \"Resume\" [blue] {\n                    clicks = clicks + 1;\n                }\n                button \"Quit\";\n                loop 2 {\n                    text \"made by code\";\n                }\n            }\n        }\n    }\n}\n\npass PauseMenu;\nend\n";

fn doc() -> ScreenDoc {
    ScreenDoc::new("ui/pauseMenu.lst".into(), "PauseMenu".into(), MENU.into())
}

/// The view Lust would make of MENU.
fn view() -> Node {
    let n = |kind: &str, text: &str, children: Vec<Node>| Node {
        kind: kind.into(),
        text: text.into(),
        children,
        ..Default::default()
    };
    n(
        "view",
        "",
        vec![n(
            "stack",
            "",
            vec![n(
                "column",
                "",
                vec![
                    n("text", "Paused", vec![]),
                    n("button", "Resume", vec![]),
                    n("button", "Quit", vec![]),
                    n("text", "made by code", vec![]),
                    n("text", "made by code", vec![]),
                ],
            )],
        )],
    )
}

#[test]
fn the_file_s_pieces_match_the_view_s_until_code_makes_pieces() {
    let d = doc();
    let m = matched(d.block().unwrap(), &view());
    let files: Vec<Vec<usize>> = m.iter().map(|(f, _)| f.clone()).collect();
    assert_eq!(
        files,
        [
            vec![0],
            vec![0, 0],
            vec![0, 0, 0],
            vec![0, 0, 1],
            vec![0, 0, 2]
        ],
        "the loop's rows are the code's"
    );
    // A view that doesn't fit (the file changed) matches as far as it can.
    let mut other = view();
    other.children[0].children[0].children[1].kind = "text".into();
    assert_eq!(matched(d.block().unwrap(), &other).len(), 3);
}

#[test]
fn a_change_is_one_undo_step_shown_at_once_and_only_its_letters_change() {
    let mut d = doc();
    d.view = Some(view());
    d.picked = vec![vec![0, 0, 0]];
    assert!(d.set_style("size", Some("40")));
    assert_eq!(d.text.replace("size 40", "size 28"), MENU);
    let title = &d.view.as_ref().unwrap().children[0].children[0].children[0];
    assert!(
        title
            .styles
            .iter()
            .any(|(k, v)| k == "size" && *v == Style::Number(40.0)),
        "the canvas shows it before the code is checked again"
    );
    assert!(!d.set_style("size", Some("40")), "the same: no step");
    assert!(d.set_words("Taking a break"));
    assert!(
        d.text
            .contains("text \"Taking a break\" [size 40]; -- the title")
    );
    assert!(d.undo());
    assert!(d.undo());
    assert_eq!(d.text, MENU);
    assert!(d.redo());
    assert!(d.text.contains("size 40"));
}

#[test]
fn several_picked_change_together_as_one_step() {
    let mut d = doc();
    d.picked = vec![vec![0, 0, 1], vec![0, 0, 2], vec![0, 0]];
    assert!(d.set_style("round", Some("12")));
    assert!(d.text.contains("button \"Resume\" [blue, round 12]"));
    assert!(d.text.contains("button \"Quit\" [round 12];"));
    assert!(
        d.text
            .contains("column [gap 14, background gray9, pad 24, round 12]")
    );
    assert!(d.undo());
    assert_eq!(d.text, MENU, "one step");
    // Lined up in one step: the old words go.
    d.picked = vec![vec![0]];
    assert!(d.restyle(
        &["center", "middle", "left", "top"],
        &[("right", None), ("bottom", None)]
    ));
    assert!(
        d.text
            .contains("stack [width fill, height fill, right, bottom]"),
        "{}",
        d.text
    );
    assert!(d.undo());
    assert_eq!(d.text, MENU);
}

#[test]
fn pieces_are_deleted_copied_and_moved_and_stay_picked() {
    let mut d = doc();
    d.picked = vec![vec![0, 0, 2]];
    assert!(d.duplicate_picked());
    assert_eq!(d.text.matches("button \"Quit\";").count(), 2);
    assert_eq!(d.picked, vec![vec![0, 0, 3]]);
    assert!(d.behind, "the canvas waits for the code");
    assert!(d.delete_picked());
    assert_eq!(d.text, MENU);
    assert!(d.picked.is_empty());
    d.picked = vec![vec![0, 0, 2]];
    assert!(d.move_picked(&[0, 0], 0));
    assert_eq!(d.picked, vec![vec![0, 0, 0]]);
    assert_eq!(d.piece(&[0, 0, 0]).unwrap().kind, "button");
    // A parent and its child picked: deleting takes the parent with it.
    d.picked = vec![vec![0, 0], vec![0, 0, 1]];
    assert!(d.delete_picked());
    assert!(!d.text.contains("column"));
}

#[test]
fn hidden_and_locked_follow_the_pieces_and_a_change_on_disk_is_a_step() {
    let mut d = doc();
    d.hidden.insert(vec![0, 0, 2]);
    d.picked = vec![vec![0, 0, 2]];
    assert!(d.delete_picked());
    assert!(d.hidden.is_empty(), "gone with it");
    assert!(d.is_own(&d.text.clone()));
    let mut theirs = MENU.replace("Quit", "Leave");
    theirs.push('\n');
    assert!(!d.is_own(&theirs));
    d.taken_in(theirs.clone());
    assert_eq!(d.text, theirs);
    assert!(d.undo(), "taking it in is a step");
}

#[test]
fn a_file_the_canvas_can_t_follow_is_left_alone() {
    let mut d = ScreenDoc::new(
        "ui/x.lst".into(),
        "PauseMenu".into(),
        "assembly PauseMenu { view { text \"a\" } }".into(),
    );
    assert!(d.source.is_err());
    d.picked = vec![vec![0]];
    assert!(!d.set_style("size", Some("3")));
    assert!(!d.delete_picked());
    let gone = ScreenDoc::new("ui/x.lst".into(), "Other".into(), MENU.into());
    assert!(gone.source.unwrap_err().contains("no screen called Other"));
}

#[test]
fn a_word_alone_colour_is_swapped_in_one_step() {
    let mut d = doc();
    d.picked = vec![vec![0, 0, 1]];
    assert!(d.restyle(&["blue", "red"], &[("green", None)]));
    assert!(d.text.contains("button \"Resume\" [green] {"), "{}", d.text);
}
