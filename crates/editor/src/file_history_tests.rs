use super::*;

fn set(to: u32) -> impl FnOnce(&mut u32) -> bool {
    move |f| {
        *f = to;
        true
    }
}

#[test]
fn a_change_undoes_and_redoes() {
    let mut h = FileHistory::new(10);
    let mut file = 1;
    assert!(h.change(&mut file, set(2)));
    assert!(h.change(&mut file, set(3)));
    assert!(h.undo(&mut file));
    assert_eq!(file, 2);
    assert!(h.undo(&mut file));
    assert_eq!(file, 1);
    assert!(!h.undo(&mut file), "nothing more to undo");
    assert_eq!(file, 1);
    assert!(h.redo(&mut file));
    assert!(h.redo(&mut file));
    assert_eq!(file, 3);
    assert!(!h.redo(&mut file), "nothing more to redo");
}

#[test]
fn a_change_that_changes_nothing_is_no_step() {
    let mut h = FileHistory::new(10);
    let mut file = 1;
    assert!(!h.change(&mut file, set(1)));
    assert!(!h.can_undo());
}

#[test]
fn a_change_that_refuses_is_put_back() {
    // The animator changes a copy as it goes and refuses at a limit; what
    // it did before refusing must not stay.
    let mut h = FileHistory::new(10);
    let mut file = 1;
    assert!(!h.change(&mut file, |f| {
        *f = 7;
        false
    }));
    assert_eq!(file, 1);
    assert!(!h.can_undo());
}

#[test]
fn a_new_change_lets_go_of_redo() {
    let mut h = FileHistory::new(10);
    let mut file = 1;
    h.change(&mut file, set(2));
    h.undo(&mut file);
    assert!(h.can_redo());
    h.change(&mut file, set(5));
    assert!(!h.can_redo());
    assert!(!h.redo(&mut file));
    assert_eq!(file, 5);
}

#[test]
fn only_the_newest_steps_up_to_the_limit_are_kept() {
    let mut h = FileHistory::new(3);
    let mut file = 0;
    for v in 1..=10 {
        h.change(&mut file, set(v));
    }
    let mut undone = Vec::new();
    while h.undo(&mut file) {
        undone.push(file);
    }
    assert_eq!(undone, [9, 8, 7], "the three newest steps, newest first");
    // Redo brings all three back.
    while h.redo(&mut file) {}
    assert_eq!(file, 10);
}

#[test]
fn redo_counts_within_the_limit_once_undone_again() {
    let mut h = FileHistory::new(2);
    let mut file = 0;
    for v in 1..=3 {
        h.change(&mut file, set(v));
    }
    h.undo(&mut file);
    h.undo(&mut file);
    h.redo(&mut file);
    h.redo(&mut file);
    assert_eq!(file, 3);
    assert!(h.undo(&mut file));
    assert!(h.undo(&mut file));
    assert!(!h.undo(&mut file), "still two steps");
    assert_eq!(file, 1);
}

#[test]
fn a_limit_of_nothing_still_keeps_one_step() {
    let mut h = FileHistory::new(0);
    let mut file = 0;
    h.change(&mut file, set(1));
    assert!(h.undo(&mut file));
    assert_eq!(file, 0);
}

#[test]
fn steps_by_hand_undo_redo_and_go_back_when_they_cant_be_used() {
    // The Map tab's way (the second review pass's N-27): a step pushed,
    // taken out to put in force, the file as it was put on the other side,
    // or the step put back when it couldn't be used.
    let mut h = FileHistory::new(3);
    for before in 1..=5 {
        h.push(before);
    }
    assert_eq!(h.steps(), (3, 0), "the oldest went past the limit");
    let undone = h.take(false);
    assert_eq!(undone, Some(5));
    h.put(true, 6);
    assert_eq!(h.steps(), (2, 1));
    // A redo that couldn't be saved goes back where it was.
    let redone = h.take(true).unwrap();
    h.put(true, redone);
    assert_eq!(h.steps(), (2, 1));
    assert_eq!(h.take(true), Some(6));
    // A new step lets go of the redo steps.
    h.put(true, 7);
    h.push(8);
    assert_eq!(h.steps(), (3, 0));
    assert_eq!(h.take(true), None);
}

#[test]
fn the_oldest_steps_go_while_too_much_is_kept_but_never_the_last() {
    let mut h = FileHistory::new(10);
    for before in [40, 30, 20, 10] {
        h.push(before);
    }
    h.drop_oldest_while(|steps| steps.iter().sum::<u32>() > 35);
    assert_eq!(h.steps(), (2, 0), "20 and 10 kept");
    h.drop_oldest_while(|_| true);
    assert_eq!(h.steps(), (1, 0), "the newest stays");
    assert_eq!(h.take(false), Some(10));
}
