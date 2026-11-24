use super::*;

fn id(n: u64) -> AssetId {
    AssetId(reflect::Id::from_raw(n).unwrap())
}

fn timing(ms: u32, event: &str) -> Timing {
    Timing {
        ms,
        event: event.into(),
    }
}

/// A knight: a grid with one cell moved by hand, two looks, a walk with
/// eight directions (the west side mirrored, so five rows) sharing its
/// frames, an attack with four and frames of its own, an idle with one,
/// and a tag's added footstep.
fn knight() -> ClipsFile {
    let way = |facing, cells: &[u32], timings: Vec<Timing>| Way {
        facing,
        cells: cells.to_vec(),
        timings,
    };
    ClipsFile {
        grid: Grid {
            cell: [32, 32],
            between: [2, 2],
            edge: [1, 1],
        },
        first: "idle".into(),
        cells: vec![Cell {
            number: 23,
            rect: [136, 69, 32, 32],
        }],
        looks: vec![
            Look {
                name: "Normal".into(),
                picture: id(1),
            },
            Look {
                name: "Mana".into(),
                picture: id(2),
            },
        ],
        clips: vec![
            Clip {
                name: "walk".into(),
                speed: 1.0,
                playback: Direction::Forward,
                plays: 0,
                directions: 8,
                same_frames: true,
                mirror: true,
                timings: vec![timing(120, "footstep"), timing(120, "")],
                ways: vec![
                    way(Facing::South, &[3, 4], vec![]),
                    way(Facing::SouthEast, &[12, 13], vec![]),
                    way(Facing::East, &[21, 22], vec![]),
                    way(Facing::NorthEast, &[30, 31], vec![]),
                    way(Facing::North, &[39, 40], vec![]),
                ],
            },
            Clip {
                name: "attack".into(),
                speed: 1.5,
                playback: Direction::PingPong,
                plays: 1,
                directions: 4,
                same_frames: false,
                mirror: false,
                timings: vec![],
                ways: vec![
                    way(Facing::North, &[50], vec![timing(80, "swing")]),
                    way(
                        Facing::West,
                        &[51, 52],
                        vec![timing(80, ""), timing(90, "swing")],
                    ),
                ],
            },
            Clip {
                name: "idle".into(),
                speed: 0.5,
                playback: Direction::Reverse,
                plays: 0,
                directions: 1,
                same_frames: true,
                mirror: false,
                timings: vec![timing(200, "")],
                ways: vec![way(Facing::Any, &[0], vec![])],
            },
        ],
        tags: vec![TagChange {
            name: "run".into(),
            speed: 1.25,
            events: vec![(2, "footstep".into())],
        }],
    }
}

#[test]
fn a_knight_writes_and_reads_back_the_same() {
    let text = write(&knight(), "knight.png");
    assert!(text.starts_with("-- Clips of knight.png."), "{text}");
    assert!(text.contains("facing = Facing.NorthEast"), "{text}");
    assert!(text.contains("playback = Playback.PingPong"), "{text}");
    let read = read(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.file, knight());
}

#[test]
fn the_kept_example_reads_and_is_what_solis_writes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/clips/v2/knight.clips.lst");
    let kept = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        kept,
        write(&knight(), "knight.png"),
        "write it again from knight()"
    );
    assert_eq!(read(&kept).unwrap().file, knight());
}

#[test]
fn rows_a_clip_has_no_place_for_are_left_out_with_a_note() {
    let text = write(&knight(), "knight.png")
        // A west row on a mirrored clip, a diagonal on a four-way one, and
        // a row for no clip.
        .replace(
            "lock ClipRow array rows = [\n",
            "lock ClipRow array rows = [\n    new ClipRow { clip = \"walk\", facing = Facing.West, cells = [1], times = [], events = [] },\n    new ClipRow { clip = \"attack\", facing = Facing.NorthEast, cells = [1], times = [80], events = [\"\"] },\n    new ClipRow { clip = \"jump\", facing = Facing.Any, cells = [1], times = [], events = [] },\n",
        );
    let read = read(&text).unwrap();
    assert_eq!(read.file, knight(), "only the bad rows go");
    assert_eq!(read.notes.len(), 3, "{:?}", read.notes);
    assert!(read.notes.iter().any(|n| n.contains("no such clip")));
}

#[test]
fn values_out_of_range_are_put_right_or_left_out() {
    let mut text = write(&knight(), "knight.png");
    text = text
        .replace("cell = [32, 32]", "cell = [0, 99999]")
        .replace("directions = 4", "directions = 3")
        .replace("speed = 0.5", "speed = 99.0")
        .replace(
            "lock Cell array cells = [\n",
            "lock Cell array cells = [\n    new Cell { number = 23, box = [0, 0, 8, 8] },\n",
        );
    let read = read(&text).unwrap();
    assert_eq!(read.file.grid.cell, [16, 16]);
    let attack = read.file.clips.iter().find(|c| c.name == "attack").unwrap();
    assert_eq!(
        attack.directions, 1,
        "with one direction, its four rows have no place"
    );
    assert!(attack.ways.is_empty());
    assert_eq!(
        read.file
            .clips
            .iter()
            .find(|c| c.name == "idle")
            .unwrap()
            .speed,
        1.0
    );
    // The first cell 23 is kept; the second is left out.
    assert_eq!(read.file.cells.len(), 1);
    assert_eq!(read.file.cells[0].rect, [0, 0, 8, 8]);
    assert!(read.notes.len() >= 4, "{:?}", read.notes);
}

#[test]
fn a_newer_or_broken_file_is_refused_and_names_are_checked() {
    let newer = write(&knight(), "k.png").replace("version = 2", "version = 9");
    assert!(read(&newer).unwrap_err().to_string().contains("newer"));
    assert!(read("not lust").is_err());
    assert!(read(&"-".repeat(MAX_BYTES + 1)).is_err());
    assert!(good_name("walk") && good_name("walk left") && good_name("9walk"));
    assert!(!good_name("") && !good_name(" walk") && !good_name("a\tb"));
    assert!(!good_name(&"w".repeat(MAX_NAME + 1)));
    assert_eq!(file_for("sprites/knight.png"), "sprites/knight.clips.lst");
}

fn cells_of(clip: &Clip, facing: Facing) -> Vec<u32> {
    clip.ways
        .iter()
        .find(|w| w.facing == facing)
        .map(|w| w.cells.clone())
        .unwrap_or_default()
}

#[test]
fn from_eight_to_four_keeps_the_four_straight_rows_and_back_copies() {
    let mut walk = knight().clips[0].clone();
    walk.set_directions(4);
    let facings: Vec<Facing> = walk.ways.iter().map(|w| w.facing).collect();
    assert_eq!(
        facings,
        [Facing::South, Facing::East, Facing::North],
        "mirrored: no West row"
    );
    assert_eq!(cells_of(&walk, Facing::East), [21, 22]);
    walk.set_directions(8);
    assert_eq!(
        cells_of(&walk, Facing::SouthEast),
        [3, 4],
        "a new row copies the south one"
    );
    assert_eq!(cells_of(&walk, Facing::North), [39, 40], "kept");
    walk.set_directions(1);
    assert_eq!(walk.ways.len(), 1);
    assert_eq!(
        (walk.ways[0].facing, cells_of(&walk, Facing::Any)),
        (Facing::Any, vec![3, 4])
    );
    walk.set_directions(4);
    assert_eq!(
        cells_of(&walk, Facing::North),
        [3, 4],
        "every new row from the one"
    );
}

#[test]
fn a_mirrored_west_side_is_drawn_from_the_east_flipped() {
    let knight = knight();
    let (walk, attack) = (&knight.clips[0], &knight.clips[1]);
    assert_eq!(
        walk.drawn_from(Facing::NorthWest),
        (Facing::NorthEast, true)
    );
    assert_eq!(walk.drawn_from(Facing::North), (Facing::North, false));
    assert_eq!(
        attack.drawn_from(Facing::West),
        (Facing::West, false),
        "not mirrored"
    );
    assert_eq!(walk.facings().len(), 8);
    assert_eq!(
        Facing::of_directions(4),
        [Facing::South, Facing::East, Facing::North, Facing::West]
    );
}

#[test]
fn each_row_s_frames_are_its_own_with_shared_or_own_times() {
    let knight = knight();
    let walk = &knight.clips[0];
    let east = walk.frames_of(Facing::East, |_| 1);
    assert_eq!(
        east.iter().map(|f| (f.frame, f.ms)).collect::<Vec<_>>(),
        [(21, 120), (22, 120)]
    );
    assert_eq!(east[0].events, ["footstep"], "the shared times and events");
    let attack = &knight.clips[1];
    let west = attack.frames_of(Facing::West, |_| 1);
    assert_eq!(
        west.iter().map(|f| (f.frame, f.ms)).collect::<Vec<_>>(),
        [(51, 80), (52, 90)]
    );
    assert!(
        attack.frames_of(Facing::South, |_| 1).is_empty(),
        "no such row"
    );
    let mut attack = attack.clone();
    attack.edit_frames_of(Facing::South, |frames| {
        frames.push(ClipFrame {
            frame: 7,
            ms: 50,
            events: Vec::new(),
        })
    });
    assert_eq!(cells_of(&attack, Facing::South), [7], "made when edited");
}

#[test]
fn turning_either_setting_off_and_on_again_gives_the_clip_back() {
    let knight = knight();
    let walk = knight.clips[0].clone();
    let mut changed = walk.clone();
    changed.set_same_frames(false);
    assert!(changed.timings.is_empty());
    assert!(
        changed.ways.iter().all(|w| w.timings.len() == 2),
        "each row its own copy"
    );
    changed.set_same_frames(true);
    assert_eq!(changed, walk);
    changed.set_mirror(false);
    assert_eq!(
        cells_of(&changed, Facing::NorthWest),
        [30, 31],
        "a copy of NorthEast"
    );
    assert_eq!(
        changed.drawn_from(Facing::West),
        (Facing::West, false),
        "its own row now"
    );
    changed.set_mirror(true);
    assert_eq!(changed, walk);
}
