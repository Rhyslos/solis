use super::*;
use crate::shapes::{fill, hill_with_cave};
use crate::store::{Full, Octree};

fn hill() -> Ground<Full> {
    Ground::build(UVec3::new(3, 3, 2), |key, dense| {
        fill(key, dense, hill_with_cave)
    })
}

fn same<S: Store, T: Store>(a: &Ground<S>, b: &Ground<T>) {
    assert_eq!(a.chunks_across(), b.chunks_across());
    let size = a.size();
    for z in 0..size.z {
        for y in 0..size.y {
            for x in 0..size.x {
                let p = glam::IVec3::new(x, y, z);
                assert_eq!(a.get(p), b.get(p), "{p}");
            }
        }
    }
}

#[test]
fn binary_reads_back_the_same_ground() {
    let ground = hill();
    let back: Ground<Octree> = read_binary(&binary(&ground)).unwrap();
    same(&ground, &back);
}

#[test]
fn text_reads_back_the_same_ground() {
    let ground = hill();
    let written = text(&ground);
    assert!(written.starts_with("ground 1\nchunks 3 3 2\nchunk 0 0 0 "));
    let back: Ground<Full> = read_text(&written).unwrap();
    same(&ground, &back);
}

#[test]
fn text_is_bigger_and_squeezing_is_smaller() {
    let ground = hill();
    let (text, bytes) = (text(&ground).len(), binary(&ground));
    assert!(text > bytes.len());
    assert!(squeezed(&bytes).len() < bytes.len());
}

#[test]
fn a_file_saying_too_many_chunks_is_refused_before_anything_is_made() {
    let mut bytes = binary(&hill());
    bytes[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(
        read_binary::<Full>(&bytes)
            .err()
            .unwrap()
            .contains("more than")
    );
    let text = "ground 1\nchunks 100000 100000 100000\n";
    assert!(read_text::<Full>(text).is_err());
}

#[test]
fn runs_that_overflow_a_chunk_are_refused() {
    let text = "ground 1\nchunks 1 1 1\nchunk 0 0 0 2\n32768 0 0 1 0 0\n";
    assert!(read_text::<Full>(text).err().unwrap().contains("add up"));
    let text = "ground 1\nchunks 1 1 1\nchunk 0 0 0 1\n32767 0 0\n";
    assert!(read_text::<Full>(text).err().unwrap().contains("add up"));
}

/// A small random number maker, the same each run.
fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn damaged_files_never_crash_the_readers() {
    let small: Ground<Full> = Ground::build(UVec3::new(1, 1, 1), |key, dense| {
        fill(key, dense, hill_with_cave)
    });
    let (bytes, words) = (binary(&small), text(&small));
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for round in 0..2000 {
        let mut b = bytes.clone();
        let mut t = words.clone().into_bytes();
        for _ in 0..1 + round % 4 {
            let r = next(&mut state);
            let i = (r as usize) % b.len();
            b[i] = (r >> 32) as u8;
            let j = (r as usize >> 8) % t.len();
            t[j] = b"0123456789 -x\n"[(r >> 40) as usize % 14];
        }
        if round % 7 == 0 {
            b.truncate((next(&mut state) as usize) % b.len());
            t.truncate((next(&mut state) as usize) % t.len());
        }
        let _ = read_binary::<Octree>(&b);
        let _ = read_text::<Octree>(&String::from_utf8_lossy(&t));
    }
}
