//! Tests for the Aseprite reader, with a small writer that makes files
//! the way Aseprite lays them out (there's no Aseprite on the test
//! machines). `write_test_file` also makes the fuzzer's starting files.

use super::*;
use std::io::Write;

/// A layer to write.
#[derive(Clone)]
pub(crate) struct TestLayer {
    pub(crate) name: &'static str,
    pub(crate) visible: bool,
    pub(crate) group: bool,
    pub(crate) level: u16,
    pub(crate) opacity: u8,
    pub(crate) blend: u16,
    pub(crate) text: Option<&'static str>,
}

impl TestLayer {
    pub(crate) fn new(name: &'static str) -> Self {
        Self {
            name,
            visible: true,
            group: false,
            level: 0,
            opacity: 255,
            blend: 0,
            text: None,
        }
    }
}

/// How a cel is stored.
#[derive(Clone)]
pub(crate) enum TestPixels {
    /// Raw pixels, in the file's colour depth.
    Raw(u32, u32, Vec<u8>),
    /// Compressed pixels.
    Compressed(u32, u32, Vec<u8>),
    /// The same as this layer's cel in that frame.
    Linked(u16),
}

/// A cel to write.
#[derive(Clone)]
pub(crate) struct TestCel {
    pub(crate) layer: u16,
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) opacity: u8,
    pub(crate) z: i16,
    pub(crate) pixels: TestPixels,
    pub(crate) text: Option<&'static str>,
}

/// A tag to write: name, from, to, direction, repeats, user text.
pub(crate) type TestTag = (&'static str, u16, u16, u8, u16, Option<&'static str>);

/// A slice to write: name, keys (frame, x, y, w, h), pivot (applied to
/// every key).
pub(crate) type TestSlice = (
    &'static str,
    Vec<(u32, i32, i32, u32, u32)>,
    Option<[i32; 2]>,
);

/// A file to write.
#[derive(Clone)]
pub(crate) struct TestFile {
    pub(crate) size: [u16; 2],
    pub(crate) depth: u16,
    pub(crate) transparent: u8,
    pub(crate) layers: Vec<TestLayer>,
    /// Each frame's duration and cels.
    pub(crate) frames: Vec<(u16, Vec<TestCel>)>,
    pub(crate) tags: Vec<TestTag>,
    pub(crate) slices: Vec<TestSlice>,
    pub(crate) palette: Vec<[u8; 4]>,
}

fn string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u16).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn chunk(out: &mut Vec<u8>, kind: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32 + 6).to_le_bytes());
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(data);
}

fn user_data(out: &mut Vec<u8>, text: &str) {
    let mut d = Vec::new();
    d.extend_from_slice(&1u32.to_le_bytes());
    string(&mut d, text);
    chunk(out, 0x2020, &d);
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

/// Writes `file` as Aseprite lays a file out.
pub(crate) fn write_test_file(file: &TestFile) -> Vec<u8> {
    let mut frames = Vec::new();
    for (f, (duration, cels)) in file.frames.iter().enumerate() {
        let mut body = Vec::new();
        let mut chunks = 0u32;
        if f == 0 {
            if !file.palette.is_empty() {
                let mut d = Vec::new();
                d.extend_from_slice(&(file.palette.len() as u32).to_le_bytes());
                d.extend_from_slice(&0u32.to_le_bytes());
                d.extend_from_slice(&(file.palette.len() as u32 - 1).to_le_bytes());
                d.extend_from_slice(&[0; 8]);
                for c in &file.palette {
                    d.extend_from_slice(&0u16.to_le_bytes());
                    d.extend_from_slice(c);
                }
                chunk(&mut body, 0x2019, &d);
                chunks += 1;
            }
            for layer in &file.layers {
                let mut d = Vec::new();
                d.extend_from_slice(&(u16::from(layer.visible) | 2).to_le_bytes());
                d.extend_from_slice(&u16::from(layer.group).to_le_bytes());
                d.extend_from_slice(&layer.level.to_le_bytes());
                d.extend_from_slice(&[0; 4]);
                d.extend_from_slice(&layer.blend.to_le_bytes());
                d.push(layer.opacity);
                d.extend_from_slice(&[0; 3]);
                string(&mut d, layer.name);
                chunk(&mut body, 0x2004, &d);
                chunks += 1;
                if let Some(text) = layer.text {
                    user_data(&mut body, text);
                    chunks += 1;
                }
            }
            if !file.tags.is_empty() {
                let mut d = Vec::new();
                d.extend_from_slice(&(file.tags.len() as u16).to_le_bytes());
                d.extend_from_slice(&[0; 8]);
                for (name, from, to, direction, repeats, _) in &file.tags {
                    d.extend_from_slice(&from.to_le_bytes());
                    d.extend_from_slice(&to.to_le_bytes());
                    d.push(*direction);
                    d.extend_from_slice(&repeats.to_le_bytes());
                    d.extend_from_slice(&[0; 6 + 3 + 1]);
                    string(&mut d, name);
                }
                chunk(&mut body, 0x2018, &d);
                chunks += 1;
                for (.., text) in &file.tags {
                    user_data(&mut body, text.unwrap_or(""));
                    chunks += 1;
                }
            }
            for (name, keys, pivot) in &file.slices {
                let mut d = Vec::new();
                d.extend_from_slice(&(keys.len() as u32).to_le_bytes());
                d.extend_from_slice(&(if pivot.is_some() { 2u32 } else { 0 }).to_le_bytes());
                d.extend_from_slice(&0u32.to_le_bytes());
                string(&mut d, name);
                for (frame, x, y, w, h) in keys {
                    d.extend_from_slice(&frame.to_le_bytes());
                    d.extend_from_slice(&x.to_le_bytes());
                    d.extend_from_slice(&y.to_le_bytes());
                    d.extend_from_slice(&w.to_le_bytes());
                    d.extend_from_slice(&h.to_le_bytes());
                    if let Some([px, py]) = pivot {
                        d.extend_from_slice(&px.to_le_bytes());
                        d.extend_from_slice(&py.to_le_bytes());
                    }
                }
                chunk(&mut body, 0x2022, &d);
                chunks += 1;
            }
        }
        for cel in cels {
            let mut d = Vec::new();
            d.extend_from_slice(&cel.layer.to_le_bytes());
            d.extend_from_slice(&cel.x.to_le_bytes());
            d.extend_from_slice(&cel.y.to_le_bytes());
            d.push(cel.opacity);
            let kind: u16 = match cel.pixels {
                TestPixels::Raw(..) => 0,
                TestPixels::Linked(_) => 1,
                TestPixels::Compressed(..) => 2,
            };
            d.extend_from_slice(&kind.to_le_bytes());
            d.extend_from_slice(&cel.z.to_le_bytes());
            d.extend_from_slice(&[0; 5]);
            match &cel.pixels {
                TestPixels::Raw(w, h, px) => {
                    d.extend_from_slice(&(*w as u16).to_le_bytes());
                    d.extend_from_slice(&(*h as u16).to_le_bytes());
                    d.extend_from_slice(px);
                }
                TestPixels::Compressed(w, h, px) => {
                    d.extend_from_slice(&(*w as u16).to_le_bytes());
                    d.extend_from_slice(&(*h as u16).to_le_bytes());
                    d.extend_from_slice(&zlib(px));
                }
                TestPixels::Linked(frame) => d.extend_from_slice(&frame.to_le_bytes()),
            }
            chunk(&mut body, 0x2005, &d);
            chunks += 1;
            if let Some(text) = cel.text {
                user_data(&mut body, text);
                chunks += 1;
            }
        }
        let mut frame = Vec::new();
        frame.extend_from_slice(&(body.len() as u32 + 16).to_le_bytes());
        frame.extend_from_slice(&0xF1FAu16.to_le_bytes());
        frame.extend_from_slice(&(chunks.min(0xFFFF) as u16).to_le_bytes());
        frame.extend_from_slice(&duration.to_le_bytes());
        frame.extend_from_slice(&[0; 2]);
        frame.extend_from_slice(&chunks.to_le_bytes());
        frame.extend_from_slice(&body);
        frames.push(frame);
    }
    let mut out = Vec::new();
    let total: usize = 128 + frames.iter().map(Vec::len).sum::<usize>();
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&0xA5E0u16.to_le_bytes());
    out.extend_from_slice(&(file.frames.len() as u16).to_le_bytes());
    out.extend_from_slice(&file.size[0].to_le_bytes());
    out.extend_from_slice(&file.size[1].to_le_bytes());
    out.extend_from_slice(&file.depth.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&100u16.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    out.push(file.transparent);
    out.extend_from_slice(&[0; 3]);
    out.extend_from_slice(&(file.palette.len() as u16).to_le_bytes());
    out.extend_from_slice(&[1, 1]);
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(&[0; 84]);
    assert_eq!(out.len(), 128);
    for f in frames {
        out.extend_from_slice(&f);
    }
    out
}

/// `w` × `h` RGBA pixels all `c`.
fn solid(w: u32, h: u32, c: [u8; 4]) -> Vec<u8> {
    c.repeat((w * h) as usize)
}

fn cel(layer: u16, x: i16, y: i16, pixels: TestPixels) -> TestCel {
    TestCel {
        layer,
        x,
        y,
        opacity: 255,
        z: 0,
        pixels,
        text: None,
    }
}

/// A 16 × 24 walker: a body layer and a hat layer, four frames tagged
/// "walk" (ping-pong) and "idle", a feet slice with its pivot, and a
/// footstep event on frame 2.
pub(crate) fn walker() -> TestFile {
    let body = |shade: u8| TestPixels::Compressed(8, 16, solid(8, 16, [shade, 80, 40, 255]));
    let mut frames = Vec::new();
    for f in 0..4u16 {
        let mut cels = vec![cel(0, 4, 8, body(100 + f as u8 * 20))];
        cels.push(cel(
            1,
            4,
            4,
            TestPixels::Raw(8, 4, solid(8, 4, [20, 20, 200, 255])),
        ));
        if f == 2 {
            cels[0].text = Some("footstep");
        }
        frames.push((100 + f * 10, cels));
    }
    TestFile {
        size: [16, 24],
        depth: 32,
        transparent: 0,
        layers: vec![
            TestLayer::new("body"),
            TestLayer {
                text: Some("swappable"),
                ..TestLayer::new("hat")
            },
        ],
        frames,
        tags: vec![
            ("walk", 0, 3, 2, 0, Some("loop me")),
            ("idle", 0, 0, 0, 1, None),
        ],
        slices: vec![
            ("feet", vec![(0, 6, 22, 4, 2)], Some([2, 1])),
            ("hand", vec![(0, 12, 12, 2, 2), (2, 13, 12, 2, 2)], None),
        ],
        palette: Vec::new(),
    }
}

fn px(a: &Aseprite, frame: usize, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * a.size[0] + x) * 4) as usize;
    let p = &a.frames[frame].rgba[at..at + 4];
    [p[0], p[1], p[2], p[3]]
}

#[test]
fn a_walker_reads_with_its_frames_tags_slices_and_events() {
    let a = read_aseprite(&write_test_file(&walker())).unwrap();
    assert_eq!(a.size, [16, 24]);
    assert_eq!(a.frames.len(), 4);
    assert_eq!(
        a.frames.iter().map(|f| f.duration_ms).collect::<Vec<_>>(),
        [100, 110, 120, 130]
    );
    // Body under, hat over, nothing elsewhere.
    assert_eq!(px(&a, 0, 5, 20), [100, 80, 40, 255]);
    assert_eq!(px(&a, 3, 5, 20), [160, 80, 40, 255]);
    assert_eq!(px(&a, 0, 5, 5), [20, 20, 200, 255]);
    assert_eq!(px(&a, 0, 0, 0), [0, 0, 0, 0]);
    // Tags.
    assert_eq!(a.tags.len(), 2);
    assert_eq!(a.tags[0].name, "walk");
    assert_eq!((a.tags[0].from, a.tags[0].to), (0, 3));
    assert_eq!(a.tags[0].direction, Direction::PingPong);
    assert_eq!(a.tags[0].repeats, 0);
    assert_eq!(a.tags[0].text.as_deref(), Some("loop me"));
    assert_eq!(a.tags[1].repeats, 1);
    assert_eq!(a.tags[1].text, None);
    // Slices, with keys that change and a pivot.
    let feet = &a.slices[0];
    assert_eq!(feet.name, "feet");
    assert_eq!(feet.keys[0].rect, [6, 22, 4, 2]);
    assert_eq!(feet.keys[0].pivot, Some([2, 1]));
    let hand = &a.slices[1];
    assert_eq!(hand.key_at(1).unwrap().rect[0], 12);
    assert_eq!(hand.key_at(3).unwrap().rect[0], 13);
    // Events and layers.
    assert_eq!(a.frames[2].events, ["footstep"]);
    assert!(a.frames[1].events.is_empty());
    assert_eq!(a.layers[1].name, "hat");
    assert_eq!(a.layers[1].text.as_deref(), Some("swappable"));
    assert!(a.notes.is_empty(), "{:?}", a.notes);
}

#[test]
fn hidden_layers_and_hidden_groups_are_left_out() {
    let mut f = walker();
    f.layers = vec![
        TestLayer::new("body"),
        TestLayer {
            visible: false,
            group: true,
            ..TestLayer::new("extras")
        },
        TestLayer {
            level: 1,
            ..TestLayer::new("hat")
        },
    ];
    for (_, cels) in &mut f.frames {
        cels[1].layer = 2;
    }
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 0, 5, 5), [0, 0, 0, 0], "the hat's group is hidden");
    assert!(!a.layers[2].shown && a.layers[1].group);
    assert_eq!(px(&a, 0, 5, 20), [100, 80, 40, 255]);
}

#[test]
fn opacity_blends_and_z_index_reorders() {
    let mut f = walker();
    f.frames.truncate(1);
    f.layers[1].opacity = 128;
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    let hat_over_nothing = px(&a, 0, 5, 5);
    assert_eq!(hat_over_nothing[3], 128);
    assert_eq!(&hat_over_nothing[..3], &[20, 20, 200]);
    // Half a hat over the body: halfway between.
    let mut f = walker();
    f.frames.truncate(1);
    f.frames[0].1[1] = cel(
        1,
        4,
        10,
        TestPixels::Raw(8, 4, solid(8, 4, [20, 20, 200, 255])),
    );
    f.frames[0].1[1].opacity = 128;
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    let mixed = px(&a, 0, 5, 11);
    assert!(
        (55..=65).contains(&mixed[0]) && mixed[3] == 255,
        "{mixed:?}"
    );
    // A z-index of -1 puts the hat under the body.
    let mut f = walker();
    f.frames.truncate(1);
    f.frames[0].1[1] = cel(
        1,
        4,
        10,
        TestPixels::Raw(8, 4, solid(8, 4, [20, 20, 200, 255])),
    );
    f.frames[0].1[1].z = -1;
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 0, 5, 11), [100, 80, 40, 255]);
}

#[test]
fn linked_cels_draw_the_frame_they_name() {
    let mut f = walker();
    f.frames[3].1[0] = cel(0, 4, 8, TestPixels::Linked(1));
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 3, 5, 20), px(&a, 1, 5, 20));
    // A link to nowhere draws nothing.
    f.frames[3].1[0] = cel(0, 4, 8, TestPixels::Linked(99));
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 3, 5, 20), [0, 0, 0, 0]);
}

#[test]
fn indexed_and_grey_files_read_through_their_palette() {
    let mut f = walker();
    f.depth = 8;
    f.transparent = 0;
    f.palette = vec![[0, 0, 0, 0], [255, 0, 0, 255], [0, 255, 0, 255]];
    f.frames.truncate(1);
    f.frames[0].1 = vec![cel(0, 0, 0, TestPixels::Raw(2, 1, vec![1, 0]))];
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 0, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&a, 0, 1, 0), [0, 0, 0, 0], "the transparent index");
    assert_eq!(a.palette.len(), 3);
    let mut f = walker();
    f.depth = 16;
    f.frames.truncate(1);
    f.frames[0].1 = vec![cel(0, 0, 0, TestPixels::Compressed(1, 1, vec![200, 255]))];
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 0, 0, 0), [200, 200, 200, 255]);
}

#[test]
fn cels_partly_off_the_canvas_are_cut_and_other_blend_modes_are_noted() {
    let mut f = walker();
    f.frames.truncate(1);
    f.frames[0].1[0] = cel(
        0,
        -4,
        -4,
        TestPixels::Raw(8, 8, solid(8, 8, [9, 9, 9, 255])),
    );
    f.layers[0].blend = 1;
    let a = read_aseprite(&write_test_file(&f)).unwrap();
    assert_eq!(px(&a, 0, 0, 0), [9, 9, 9, 255]);
    assert_eq!(px(&a, 0, 4, 0), [0, 0, 0, 0]);
    assert!(
        a.notes.iter().any(|n| n.contains("blend mode")),
        "{:?}",
        a.notes
    );
}

#[test]
fn damaged_files_are_refused_with_why_and_never_panic() {
    let good = write_test_file(&walker());
    assert!(read_aseprite(&good[..100]).unwrap_err().0.contains("short"));
    let mut wrong = good.clone();
    wrong[4] = 0;
    assert!(
        read_aseprite(&wrong)
            .unwrap_err()
            .0
            .contains("isn't an Aseprite")
    );
    // Cut anywhere: refused or read, never a crash.
    for end in (128..good.len()).step_by(7) {
        let _ = read_aseprite(&good[..end]);
    }
    // Every byte flipped in turn.
    for i in 0..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0xA5;
        let _ = read_aseprite(&bad);
    }
    // A compressed picture that unpacks to less than it claims.
    let mut f = walker();
    f.frames[0].1[0].pixels = TestPixels::Compressed(8, 16, vec![1, 2, 3]);
    assert!(
        read_aseprite(&write_test_file(&f))
            .unwrap_err()
            .0
            .contains("frame 1")
    );
}

#[test]
fn files_past_the_limits_are_refused_before_anything_big_is_made() {
    let good = write_test_file(&walker());
    let tight = AseLimits {
        max_side: 8,
        ..AseLimits::default()
    };
    assert!(
        read_aseprite_within(&good, tight)
            .unwrap_err()
            .0
            .contains("16 × 24")
    );
    let tight = AseLimits {
        max_frames: 2,
        ..AseLimits::default()
    };
    assert!(
        read_aseprite_within(&good, tight)
            .unwrap_err()
            .0
            .contains("4 frames")
    );
    let tight = AseLimits {
        max_pixel_bytes: 100,
        ..AseLimits::default()
    };
    assert!(read_aseprite_within(&good, tight).is_err());
    let tight = AseLimits {
        max_file_bytes: 64,
        ..AseLimits::default()
    };
    assert!(read_aseprite_within(&good, tight).is_err());
    let tight = AseLimits {
        max_layers: 1,
        ..AseLimits::default()
    };
    assert!(
        read_aseprite_within(&good, tight)
            .unwrap_err()
            .0
            .contains("layers")
    );
}

#[test]
fn aseprite_files_are_known_by_extension() {
    use std::path::Path;
    assert!(is_aseprite_file(Path::new("farmer.aseprite")));
    assert!(is_aseprite_file(Path::new("x/Crops.ASE")));
    assert!(!is_aseprite_file(Path::new("farmer.png")));
}

/// Writes the fuzzer's starting files into `$SOLIS_FUZZ_SEEDS/aseprite`.
/// Run by `fuzz/make-seeds.sh`.
#[test]
#[ignore = "writes fuzz seeds: run by fuzz/make-seeds.sh"]
fn write_fuzz_seeds() {
    let Some(root) = std::env::var_os("SOLIS_FUZZ_SEEDS") else {
        return;
    };
    let folder = std::path::Path::new(&root).join("aseprite");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("walker.aseprite"), write_test_file(&walker())).unwrap();
    let mut indexed = walker();
    indexed.depth = 8;
    indexed.palette = vec![[0, 0, 0, 0], [255, 0, 0, 255]];
    indexed.frames = vec![(100, vec![cel(0, 0, 0, TestPixels::Raw(2, 1, vec![1, 0]))])];
    std::fs::write(folder.join("indexed.aseprite"), write_test_file(&indexed)).unwrap();
}
