use super::*;
use crate::start::Start;
use crate::store::{Fixed, keys as all_keys};
use glam::UVec3;

fn gentle() -> Ground {
    Start::Gentle {
        height: 40.0,
        seed: 5,
    }
    .ground(UVec3::new(16, 8, 2))
}

#[test]
fn a_region_round_trips_and_a_ground_comes_back_whole() {
    let g = gentle();
    let mut back = Ground::new(Fixed::empty(g.extent()));
    for region in [[0, 0], [1, 0]] {
        let bytes = write(&g, region);
        load_into(&mut back, region, &bytes).unwrap();
    }
    assert_eq!(back, g);
    for key in all_keys(g.extent()) {
        assert!(back.chunk(key).is_some());
    }
}

#[test]
fn levels_one_to_three_have_room_in_the_table() {
    let g = gentle();
    let bytes = write(&g, [0, 0]);
    let mixed = keys([0, 0], 2)
        .iter()
        .filter(|k| !g.chunk(**k).unwrap().is_uniform())
        .count();
    let uniform = 128 - mixed;
    // Levels 1 to 3 of a ground two chunks high: one chunk high each.
    let reserved = 4 * 4 + 2 * 2 + 1;
    let table = uniform * 3 + mixed * 9 + reserved;
    assert!(bytes.len() > HEADER + table);
    assert_eq!(&bytes[..4], b"GRGN");
    assert_eq!(bytes[16], 4);
}

#[test]
fn an_absent_chunk_is_written_as_not_kept_and_read_back_absent() {
    let mut g = gentle();
    let gone = IVec3::new(3, 3, 1);
    g.store_mut().unload(gone);
    let chunks = read(&write(&g, [0, 0]), [0, 0], 2).unwrap();
    let at = keys([0, 0], 2).iter().position(|k| *k == gone).unwrap();
    assert!(chunks[at].is_none());
    assert_eq!(chunks.iter().filter(|c| c.is_none()).count(), 1);
}

#[test]
fn another_regions_file_or_height_is_refused() {
    let g = gentle();
    let bytes = write(&g, [1, 0]);
    assert!(
        read(&bytes, [0, 0], 2)
            .unwrap_err()
            .0
            .contains("region 1_0")
    );
    assert!(read(&bytes, [1, 0], 3).is_err());
}

#[test]
fn damaged_files_are_mistakes_never_crashes() {
    let bytes = write(&gentle(), [0, 0]);
    for cut in (0..bytes.len()).step_by(97) {
        assert!(read(&bytes[..cut], [0, 0], 2).is_err());
    }
    let mut flipped = bytes.clone();
    for i in (0..flipped.len()).step_by(13) {
        flipped[i] ^= 0x5a;
        let _ = read(&flipped, [0, 0], 2);
    }
    assert!(read(b"GRGN", [0, 0], 2).is_err());
    let mut newer = bytes;
    newer[4] = 9;
    assert!(read(&newer, [0, 0], 2).unwrap_err().0.contains("version 9"));
}

#[test]
fn file_names_name_their_region() {
    assert_eq!(file_name([3, 12]), "3_12.region");
    assert_eq!(region_of_name("3_12.region"), Some([3, 12]));
    assert_eq!(region_of_name("3_12.regions"), None);
    assert_eq!(region_of_name("x_1.region"), None);
}

/// Thousands of randomly damaged files (rule 9): none may crash the
/// reader, and none may read back as a chunk of the wrong size.
#[test]
fn thousands_of_damaged_files_never_crash_the_reader() {
    let good = write(&gentle(), [0, 0]);
    let mut seed = 0x9e37_79b9_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for _ in 0..3000 {
        let mut bytes = good.clone();
        for _ in 0..(next() % 8 + 1) {
            let i = next() as usize % bytes.len();
            bytes[i] = next() as u8;
        }
        if next() % 4 == 0 {
            bytes.truncate(next() as usize % bytes.len());
        }
        if let Ok(chunks) = read(&bytes, [0, 0], 2) {
            assert_eq!(chunks.len(), 128);
        }
    }
}
