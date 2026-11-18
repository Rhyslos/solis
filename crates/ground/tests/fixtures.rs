//! The ground's files as version 1 wrote them (`tests/fixtures/ground/`,
//! rule 23) open with this build: the settings file, and the region file
//! in the folder beside it, which is binary and so read here rather than
//! by the app's text fixtures test.

use ground::{Answer, Fixed, Ground, region, settings};
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/ground")
}

#[test]
fn version_one_opens_whole() {
    let dir = fixtures().join("v1");
    let text = std::fs::read_to_string(dir.join("farm.ground.lst")).unwrap();
    let text = text.replace("\r\n", "\n");
    let read = settings::read(&text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    let s = read.settings;
    assert_eq!(s.regions(), [1, 1]);
    let folder = settings::folder_for("farm.ground.lst");
    let bytes = std::fs::read(dir.join(folder).join(region::file_name([0, 0]))).unwrap();
    let mut ground = Ground::new(Fixed::empty(s.extent()));
    region::load_into(&mut ground, [0, 0], &bytes).unwrap();
    let h = ground.height_at(100.0, 100.0).known().flatten().unwrap();
    assert!((h - 10.5).abs() < 0.02, "{h}");
    assert_eq!(
        ground.kind_at(glam::Vec3::new(5.0, 5.0, 3.0)),
        Answer::Known(Some(0))
    );
}

#[test]
fn nothing_newer_than_this_build_lies_around() {
    assert!(
        !fixtures()
            .join(format!("v{}", settings::VERSION + 1))
            .exists()
    );
}
