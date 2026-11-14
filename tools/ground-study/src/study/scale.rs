//! The ground at the scale target, kept each way and meshed each way:
//! memory, file sizes, and what making each chunk costs.

use super::stats::{Spread, Table, count, mb, ms, timed};
use glam::{IVec3, UVec3};
use ground_study::file;
use ground_study::ground::{Ground, Samples};
use ground_study::mesh::{ChunkMesh, Method};
use ground_study::shapes::{busy, fill, rolling_chunk};
use ground_study::store::{Full, Store};
use ground_study::voxel::{Dense, SIZE_I, VOLUME, Voxel};

/// The target ground: 1,024 by 1,024 voxels (a metre each), 128 high.
pub(crate) const ACROSS: UVec3 = UVec3::new(32, 32, 4);
/// Its area in square kilometres.
pub(crate) const KM2: f64 = 1.024 * 1.024;

/// Makes the target ground, kept in full, and says what it is.
pub(crate) fn target_ground() -> Ground<Full> {
    let (time, ground) = timed(|| Ground::build(ACROSS, rolling_chunk));
    let mixed = ground.mixed_keys().len();
    let all = ground.keys().count();
    println!(
        "\nThe target ground: {} chunks, {} of them on the surface; made in {} ms on every core.",
        count(all),
        count(mixed),
        ms(time)
    );
    ground
}

/// The same ground in another store.
pub(crate) fn kept_as<S: Store>(full: &Ground<Full>) -> Ground<S> {
    Ground::build(full.chunks_across(), |key, dense| {
        if let Some(chunk) = full.chunk(key) {
            chunk.decode(dense);
        }
    })
}

/// A store's row: memory and the cost of its own operations.
pub(crate) fn store_row<S: Store>(table: &Table, ground: &Ground<S>) {
    let mixed = ground.mixed_keys();
    let mut scratch = Dense::filled(Voxel::AIR);
    let mut samples = Samples::default();
    let (mut decode, mut encode, mut gather) = (Vec::new(), Vec::new(), Vec::new());
    for key in mixed.iter().step_by(4) {
        let Some(chunk) = ground.chunk(*key) else {
            continue;
        };
        decode.push(timed(|| chunk.decode(&mut scratch)).0);
        encode.push(timed(|| std::hint::black_box(S::encode(&scratch))).0);
        gather.push(timed(|| ground.gather(*key, &mut scratch, &mut samples)).0);
    }
    // Reading single voxels near the surface, as a ray or a question
    // from game code would.
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let reads = 1_000_000;
    let (read_ms, sum) = timed(|| {
        let mut sum = 0i64;
        for _ in 0..reads {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let key = mixed[(state % mixed.len() as u64) as usize];
            let local = IVec3::new(
                (state >> 20) as i32 & 31,
                (state >> 26) as i32 & 31,
                (state >> 32) as i32 & 31,
            );
            sum += i64::from(ground.get(key * SIZE_I + local).distance);
        }
        sum
    });
    std::hint::black_box(sum);
    let bytes = ground.bytes() as f64;
    table.row(&[
        S::NAME.into(),
        mb(bytes),
        mb(bytes / KM2),
        count((bytes / mixed.len() as f64) as usize),
        format!("{:.0} ns", read_ms * 1.0e6 / f64::from(reads)),
        ms(Spread::of(&decode).mean),
        ms(Spread::of(&encode).mean),
        ms(Spread::of(&gather).mean),
    ]);
}

/// The storage table's heads.
pub(crate) fn storage_table(full: &Ground<Full>) -> Table {
    let all = full.keys().count();
    println!(
        "\nEvery chunk in full, nothing folded: {} ({} a square kilometre).",
        mb((all * VOLUME * 2) as f64),
        mb((all * VOLUME * 2) as f64 / KM2)
    );
    Table::start(
        "Storage: the target ground (chunks the same all through kept as one voxel in every store)",
        &[
            "Store",
            "Memory",
            "A km²",
            "Bytes a surface chunk",
            "Read one voxel",
            "Decode a chunk (ms)",
            "Encode a chunk (ms)",
            "Gather a chunk's samples (ms)",
        ],
    )
}

/// Every surface chunk's mesh, made one way, with the time each took.
pub(crate) fn mesh_all(
    ground: &Ground<Full>,
    method: Method,
) -> (Vec<(IVec3, ChunkMesh)>, Vec<f64>) {
    let mut scratch = Dense::filled(Voxel::AIR);
    let mut samples = Samples::default();
    let mut out = Vec::new();
    let mut times = Vec::new();
    for key in ground.mixed_keys() {
        ground.gather(key, &mut scratch, &mut samples);
        let (time, mesh) = timed(|| method.mesh(&samples));
        times.push(time);
        out.push((key, mesh));
    }
    (out, times)
}

/// The meshing table: each way over the whole target ground, and on the
/// worst chunk there is.
pub(crate) fn meshing(full: &Ground<Full>) {
    let table = Table::start(
        "Meshing: every surface chunk of the target ground, one at a time on one core",
        &[
            "Look",
            "Way",
            "Chunk, mean (ms)",
            "95th (ms)",
            "Worst (ms)",
            "Busy chunk (ms)",
            "Triangles a km²",
            "Points a km²",
            "Graphics memory a km²",
            "Busy chunk's triangles",
            "Thin triangles (an angle under 10°)",
        ],
    );
    let busy_ground: Ground<Full> = Ground::build(UVec3::ONE, |key, dense| fill(key, dense, busy));
    let mut scratch = Dense::filled(Voxel::AIR);
    let mut samples = Samples::default();
    busy_ground.gather(IVec3::ZERO, &mut scratch, &mut samples);
    for method in Method::ALL {
        let (meshes, times) = mesh_all(full, method);
        let spread = Spread::of(&times);
        let triangles: usize = meshes.iter().map(|(_, m)| m.triangles()).sum();
        let points: usize = meshes.iter().map(|(_, m)| m.positions.len()).sum();
        let bytes: usize = meshes.iter().map(|(_, m)| m.upload_bytes()).sum();
        let thin: usize = meshes.iter().map(|(_, m)| thin_triangles(m)).sum();
        let busy_times: Vec<f64> = (0..10).map(|_| timed(|| method.mesh(&samples)).0).collect();
        let busy_mesh = method.mesh(&samples);
        table.row(&[
            method.look().into(),
            method.name().into(),
            ms(spread.mean),
            ms(spread.p95),
            ms(spread.worst),
            ms(Spread::of(&busy_times).median),
            count((triangles as f64 / KM2) as usize),
            count((points as f64 / KM2) as usize),
            mb(bytes as f64 / KM2),
            count(busy_mesh.triangles()),
            format!("{:.1} %", 100.0 * thin as f64 / triangles.max(1) as f64),
        ]);
    }
}

/// How many of `mesh`'s triangles have a corner under 10°: slivers,
/// which shade badly and make physics less sure (those with no area at
/// all included).
fn thin_triangles(mesh: &ChunkMesh) -> usize {
    let limit = 10.0_f32.to_radians().cos();
    mesh.indices
        .chunks_exact(3)
        .filter(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            [(a, b, c), (b, c, a), (c, a, b)].iter().any(|(p, q, r)| {
                let (u, v) = ((*q - *p).normalize_or_zero(), (*r - *p).normalize_or_zero());
                u == glam::Vec3::ZERO || v == glam::Vec3::ZERO || u.dot(v) > limit
            })
        })
        .count()
}

/// The file table: the target ground written each way.
pub(crate) fn files(full: &Ground<Full>) {
    let table = Table::start(
        "The ground's file: the target ground",
        &["Form", "Size", "A km²", "Write (ms)", "Read (ms)"],
    );
    let (write_text, text) = timed(|| file::text(full));
    let (read_text, back) = timed(|| file::read_text::<Full>(&text));
    let read_text = if back.is_ok() {
        ms(read_text)
    } else {
        "failed".into()
    };
    let row = |name: &str, size: usize, write: String, read: String| {
        table.row(&[
            name.into(),
            mb(size as f64),
            mb(size as f64 / KM2),
            write,
            read,
        ]);
    };
    row(
        "Every voxel as text (`distance material`)",
        every_voxel_as_text(full),
        "-".into(),
        "-".into(),
    );
    row(
        "Text, runs of equal voxels",
        text.len(),
        ms(write_text),
        read_text,
    );
    let (write_binary, bytes) = timed(|| file::binary(full));
    let (read_binary, back) = timed(|| file::read_binary::<Full>(&bytes));
    let read_binary = if back.is_ok() {
        ms(read_binary)
    } else {
        "failed".into()
    };
    row(
        "Binary, runs of equal voxels",
        bytes.len(),
        ms(write_binary),
        read_binary,
    );
    let (squeeze, squeezed) = timed(|| file::squeezed(&bytes));
    row(
        "Binary runs, deflated",
        squeezed.len(),
        ms(write_binary + squeeze),
        "-".into(),
    );
    let (squeeze_text, squeezed_text) = timed(|| file::squeezed(text.as_bytes()));
    row(
        "Text runs, deflated",
        squeezed_text.len(),
        ms(write_text + squeeze_text),
        "-".into(),
    );
}

/// The length of the ground written voxel by voxel: `distance material`
/// and a space or line end each.
fn every_voxel_as_text(full: &Ground<Full>) -> usize {
    let digits = |n: i32| n.abs().to_string().len() + usize::from(n < 0);
    let mut total = 0;
    let mut dense = Dense::filled(Voxel::AIR);
    for key in full.keys() {
        if let Some(chunk) = full.chunk(key) {
            chunk.decode(&mut dense);
        }
        for v in dense.voxels() {
            total += digits(i32::from(v.distance)) + digits(i32::from(v.material)) + 2;
        }
    }
    total
}
