//! Tilesets' pictures: PNGs and Aseprite files read whole, within limits.

use super::*;

#[test]
fn a_png_is_read_whole_and_written_back() {
    let mut rgba = vec![0u8; 32 * 16 * 4];
    rgba[4..8].copy_from_slice(&[10, 20, 30, 255]);
    let png = png_of([32, 16], &rgba).unwrap();
    let art = read_tile_art(&png, false).unwrap();
    assert_eq!(art.size, [32, 16]);
    assert_eq!(art.frames, [rgba]);
    assert_eq!(art.durations, [100]);
    assert!(png_of([3, 3], &[0; 4]).is_err());
}

#[test]
fn an_aseprite_picture_brings_its_frames_and_times() {
    let bytes = crate::aseprite::tests::write_test_file(&crate::aseprite::tests::walker());
    let art = read_tile_art(&bytes, true).unwrap();
    assert_eq!(art.size, [16, 24]);
    assert_eq!(art.frames.len(), 4);
    assert_eq!(art.durations, [100, 110, 120, 130]);
    assert!(
        art.frames
            .iter()
            .all(|f| f.len() == (art.size[0] * art.size[1] * 4) as usize)
    );
}

#[test]
fn damaged_or_huge_pictures_are_refused_with_why() {
    assert!(
        read_tile_art(b"not a picture", false)
            .unwrap_err()
            .contains("PNG")
    );
    assert!(read_tile_art(b"not aseprite", true).is_err());
    let big = png_of([MAX_SIDE + 1, 1], &vec![0; (MAX_SIDE as usize + 1) * 4]).unwrap();
    assert!(
        read_tile_art(&big, false)
            .unwrap_err()
            .contains("pixels across")
    );
}
