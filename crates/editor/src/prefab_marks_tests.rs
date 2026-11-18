use crate::properties::PropertyRequest;
use crate::test_support::with_a_lamp_post;
use reflect::Value;

#[test]
fn a_copy_has_a_link_a_changed_one_a_dot_and_a_plain_entity_neither() {
    let (mut w, _, root, lamp) = with_a_lamp_post();
    let plain = w.add_entity();
    let marks = w.prefab_marks();
    assert_eq!(marks.of(root), (true, false), "a copy's root: the link");
    assert_eq!(marks.of(lamp), (false, false), "a part: nothing yet");
    assert_eq!(marks.of(plain), (false, false), "a plain entity: no mark");
    w.apply_property(
        PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(42.0),
        },
        1,
    );
    let marks = w.prefab_marks();
    assert_eq!(
        marks.of(root),
        (true, true),
        "the copy changes some settings"
    );
    assert_eq!(marks.of(lamp), (false, true), "this part is what changes");
    // Put back with ↶: no dot again.
    w.revert_field(lamp, "Light", "strength");
    let marks = w.prefab_marks();
    assert_eq!(marks.of(root), (true, false));
    assert_eq!(marks.of(lamp), (false, false));
}

#[test]
fn marks_are_kept_until_the_level_changes() {
    let (mut w, ..) = with_a_lamp_post();
    let first = w.prefab_marks();
    let again = w.prefab_marks();
    assert!(std::sync::Arc::ptr_eq(&first, &again), "worked out once");
    w.add_entity();
    assert!(!std::sync::Arc::ptr_eq(&first, &w.prefab_marks()));
}
