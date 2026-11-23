use super::*;
use crate::library::{LibraryItem, LibraryKind};
use crate::test_support::with_a_lamp_post;

#[test]
fn details_name_the_parts_and_count_the_copies_in_every_level() {
    let (mut w, asset, _, _) = with_a_lamp_post();
    w.place_module(asset);
    w.set_library(vec![LibraryItem {
        id: Some(asset),
        kind: LibraryKind::Module,
        name: "lamp post".into(),
        path: "props/lamp post.module.lst".into(),
        thumbnail: None,
    }]);
    w.open.sound_shown = Some((
        "props/lamp post.module.lst".into(),
        w.document().selection.clone(),
    ));
    let details = w.module_details().expect("a module is shown");
    assert_eq!(details.parts.len(), 1);
    assert_eq!(details.parts[0].2, "Lamp");
    assert_eq!(details.placed, None, "not counted yet");
    assert_eq!(placed_words(None), "Counting where it's placed\u{2026}");
    // The app's count: two other levels; the open level's own, as it is.
    w.set_placed(
        asset,
        Placed {
            levels: vec![("meadow.sollevel".into(), 3), ("yard.sollevel".into(), 1)],
        },
    );
    let details = w.module_details().unwrap();
    assert_eq!(details.placed, Some((6, 3)));
    assert_eq!(placed_words(details.placed), "Placed 6 times, in 3 levels");
    assert_eq!(placed_words(Some((1, 1))), "Placed 1 time, in 1 level");
    assert_eq!(placed_words(Some((0, 0))), "Not placed yet");
}
