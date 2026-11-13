use super::*;

fn editor() -> AtlasEditor {
    let mut atlas = Atlas::new("tools.png");
    atlas.grid = [16, 16];
    AtlasEditor::new("tools.atlas.lst".into(), atlas)
}

#[test]
fn a_box_drawn_snaps_to_the_grid_and_stays_in_the_picture() {
    let e = editor();
    assert_eq!(
        e.snapped([3.0, 5.0], [20.0, 9.0], [64, 64]),
        Some([0, 0, 32, 16])
    );
    assert_eq!(
        e.snapped([40.0, 40.0], [2.0, 2.0], [64, 64]),
        Some([0, 0, 48, 48]),
        "either way"
    );
    assert_eq!(
        e.snapped([60.0, 60.0], [99.0, 99.0], [64, 64]),
        Some([48, 48, 16, 16])
    );
    assert_eq!(
        e.snapped([70.0, 0.0], [80.0, 5.0], [64, 64]),
        None,
        "outside"
    );
    assert_eq!(e.snapped([f32::NAN, 0.0], [1.0, 1.0], [64, 64]), None);
}

#[test]
fn crops_are_added_renamed_deleted_and_undone() {
    let mut e = editor();
    assert!(e.add([0, 0, 16, 16]));
    assert!(e.add([16, 0, 16, 16]));
    assert_eq!(e.atlas.crops[1].name, "Crop2");
    assert_eq!(e.picked, Some(1));
    e.naming = "Axe".into();
    assert_eq!(e.rename(), Ok(true));
    e.pick(Some(0));
    e.naming = "axe".into();
    assert!(e.rename().is_err(), "another crop has it");
    e.naming = "2 axes".into();
    assert!(e.rename().is_err(), "not a name");
    assert_eq!(e.crop_at([20.0, 3.0]), Some(1));
    assert_eq!(e.crop_at([40.0, 3.0]), None);
    assert!(e.delete_picked());
    assert_eq!(e.atlas.crops.len(), 1);
    assert!(e.undo());
    assert_eq!(e.atlas.crops.len(), 2);
    assert!(e.redo());
    assert_eq!(e.atlas.crops[0].name, "Axe");
    assert!(!e.set_grid([16, 16]), "no change, no step");
    assert!(e.set_grid([0, 9999]));
    assert_eq!(e.atlas.grid, [1, 4096]);
}

#[test]
fn every_tile_is_one_undo_step_and_so_is_a_file_changed_on_disk() {
    let mut e = editor();
    assert_eq!(e.crop_every_tile([32, 32], &|_| true), 4);
    assert!(e.undo());
    assert!(e.atlas.crops.is_empty());
    // What it saves reads back as what it has: its own save, read again,
    // is no change.
    let on_disk = animation::atlas::read(&e.text()).unwrap().atlas;
    assert_eq!(on_disk, e.atlas);
    let mut other = on_disk.clone();
    other.grid = [8, 8];
    e.taken_in(other);
    assert_eq!(e.atlas.grid, [8, 8]);
    assert!(e.undo(), "taking it in is a step");
    assert_eq!(e.atlas, on_disk);
}

#[test]
fn keeps_the_newest_two_hundred_undo_steps() {
    let mut e = editor();
    for n in 100..350 {
        assert!(e.set_grid([n, n]));
    }
    let mut steps = 0;
    while e.undo() {
        steps += 1;
    }
    assert_eq!(steps, MAX_UNDO);
    assert_eq!(e.atlas.grid, [149, 149], "the oldest steps went first");
}
