use super::*;
use crate::shapes::fill;

fn packed(r: IVec2) -> Vec<Packed> {
    let shape = |p: glam::Vec3| Voxel::at(p.z - 40.0 - 3.0 * (p.x / 7.0).sin(), 2);
    all_chunks(r)
        .into_iter()
        .map(|(level, key)| {
            if level == 0 && key.z == 1 && key.x == r.x * 8 + 1 {
                let mut dense = Dense::filled(Voxel::AIR);
                fill(key, &mut dense, shape);
                pack(&Chunk::keep(&dense))
            } else if key.z == 0 {
                Packed::Uniform(Voxel::solid(1))
            } else {
                Packed::Uniform(Voxel::AIR)
            }
        })
        .collect()
}

#[test]
fn a_region_reads_back_chunk_by_chunk_and_whole() {
    let r = IVec2::new(2, -1);
    let packs = packed(r);
    let bytes = write(r, &packs);
    let index = Index::read(&bytes, bytes.len(), r).unwrap_or_else(|e| panic!("{e}"));
    for (i, p) in packs.iter().enumerate() {
        assert_eq!(index.packed(&bytes, i).as_ref(), Ok(p));
    }
    // The table alone is enough to find a chunk.
    let head = &bytes[..Index::most_bytes().min(bytes.len())];
    assert_eq!(Index::read(head, bytes.len(), r), Ok(index));
    let chunks = read(&bytes, r).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(chunks.len(), all_chunks(r).len());
    let mixed = chunks
        .iter()
        .filter(|(_, c)| matches!(c, Chunk::Mixed(_)))
        .count();
    assert_eq!(mixed, 8, "one chunk a row of eight, at level 0");
}

#[test]
fn a_region_holds_every_level_of_its_square() {
    let r = IVec2::new(1, 3);
    assert_eq!(chunks(r, 0).len(), 8 * 8 * 4);
    assert_eq!(chunks(r, 1).len(), 4 * 4 * 2);
    assert_eq!(chunks(r, 3), vec![IVec3::new(1, 3, 0)]);
    for id in all_chunks(r) {
        assert_eq!(region_of(id), r);
    }
}

#[test]
fn another_regions_file_or_a_short_one_is_refused() {
    let r = IVec2::ZERO;
    let bytes = write(r, &packed(r));
    assert!(Index::read(&bytes, bytes.len(), IVec2::ONE).is_err());
    assert!(Index::read(&bytes[..20], bytes.len(), r).is_err());
    assert!(Index::read(&bytes, bytes.len() + 1, r).is_err());
    assert!(read(&bytes[..bytes.len() - 1], r).is_err());
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn damaged_region_files_never_crash_the_reader() {
    let r = IVec2::new(0, 1);
    let good = write(r, &packed(r));
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..3000 {
        let mut bytes = good.clone();
        match next(&mut state) % 3 {
            0 => {
                for _ in 0..1 + next(&mut state) % 8 {
                    let at = (next(&mut state) as usize) % bytes.len();
                    bytes[at] = next(&mut state) as u8;
                }
            }
            1 => bytes.truncate((next(&mut state) as usize) % bytes.len()),
            _ => {
                let at = (next(&mut state) as usize) % bytes.len();
                bytes.insert(at, next(&mut state) as u8);
            }
        }
        let _ = read(&bytes, r);
    }
}

#[test]
fn a_chunk_that_inflates_too_far_is_refused() {
    let mut raw = Vec::new();
    raw.extend_from_slice(&(VOLUME as u32 + 1).to_le_bytes());
    raw.extend(std::iter::repeat_n(0u8, MOST_RUNS_BYTES + 64));
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = encoder.write_all(&raw);
    let bomb = encoder.finish().unwrap_or_default();
    assert!(unpack(&Packed::Runs(bomb)).is_err());
}
