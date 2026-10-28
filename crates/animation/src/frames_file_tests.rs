//! Clips files: written and read back, put right where they can be,
//! refused where they can't.

use super::*;

fn sample() -> FramesFile {
    FramesFile {
        sheet: Some("1BWHNR3RV8PYY".parse().unwrap()),
        first: "walk".into(),
        tags: vec![TagChange {
            name: "walk".into(),
            speed: 1.5,
            events: vec![(2, "footstep".into())],
        }],
        clips: vec![OwnClip {
            name: "hoe".into(),
            speed: 1.0,
            direction: Direction::PingPong,
            repeats: 1,
            frames: vec![
                ClipFrame {
                    frame: 4,
                    ms: 100,
                    events: Vec::new(),
                },
                ClipFrame {
                    frame: 5,
                    ms: 120,
                    events: vec!["swing".into()],
                },
            ],
        }],
    }
}

#[test]
fn a_clips_file_round_trips() {
    let text = write(&sample());
    assert!(text.contains("frames version 1;"), "{text}");
    assert!(text.contains("tag1 = [\"walk\", 1.5, [[3, \"footstep\"]]];"), "{text}");
    assert!(
        text.contains("clip1 = [\"hoe\", 1.0, \"ping-pong\", 1, [[5, 100, \"\"], [6, 120, \"swing\"]]];"),
        "{text}"
    );
    let read = read(&text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.file, sample());
    // An empty file round trips too.
    assert_eq!(read_ok(&write(&FramesFile::default())), FramesFile::default());
}

fn read_ok(text: &str) -> FramesFile {
    read(text).unwrap().file
}

fn notes(text: &str) -> Vec<String> {
    read(text).unwrap().notes
}

#[test]
fn values_out_of_range_are_brought_in_with_a_note() {
    let text = "frames version 1;\n\
        tag1 = [\"walk\", 99.0, []];\n\
        clip1 = [\"x\", -1.0, \"forward\", -5, [[1, 0, \"\"], [2, 999999, \"\"]]];\n";
    let r = read(text).unwrap();
    assert_eq!(r.file.tags[0].speed, SPEED_RANGE.1);
    let c = &r.file.clips[0];
    assert_eq!(c.speed, 0.0);
    assert_eq!(c.repeats, 0);
    assert_eq!(c.frames[0].ms, 1);
    assert_eq!(c.frames[1].ms, MAX_MS);
    assert_eq!(r.notes.len(), 5, "{:?}", r.notes);
    assert!(r.notes.iter().all(|n| n.starts_with("line ")));
}

#[test]
fn lines_it_can_t_use_are_skipped_with_a_note() {
    let n = notes(
        "frames version 1;\n\
         sheet = \"not an id\";\n\
         colour = 3;\n\
         tag1 = \"walk\";\n\
         clip1 = [\"x\", 1.0, \"sideways\", 0, []];\n\
         clip2 = [\"y\", 1.0, \"forward\", 0, [[0, 100, \"\"]]];\n\
         clip3 = [\"z\", 1.0, \"forward\", 0, []];\n\
         clip4 = [\"z\", 1.0, \"forward\", 0, []];\n\
         tag2 = [\"\", 1.0, []];\n",
    );
    assert_eq!(n.len(), 7, "{n:?}");
    assert!(n[0].contains("line 2") && n[0].contains("ID"));
    assert!(n.iter().any(|m| m.contains("sideways")));
    assert!(n.iter().any(|m| m.contains("from 1")));
    assert!(n.iter().any(|m| m.contains("second clip called \"z\"")));
    // What could be used still is.
    let f = read_ok("frames version 1;\nclip3 = [\"z\", 1.0, \"forward\", 0, []];\nbad = 1;\n");
    assert_eq!(f.clips.len(), 1);
}

#[test]
fn files_it_can_t_read_are_refused_with_their_line() {
    assert!(read("").is_err());
    assert!(read("area version 1;").unwrap_err().message.contains("frames"));
    let newer = read("frames version 2;").unwrap_err();
    assert!(newer.message.contains("newer"));
    assert_eq!(newer.line, 1);
    let big = "-".repeat(MAX_BYTES + 1);
    assert_eq!(read(&big).unwrap_err().line, 0);
    let broken = read("frames version 1;\nclip1 = [\"x\", 1.0,\n").unwrap_err();
    assert!(broken.line >= 2, "{broken}");
}

#[test]
fn limits_are_kept() {
    let mut text = String::from("frames version 1;\n");
    for i in 0..MAX_CLIPS + 3 {
        text.push_str(&format!("clip{i} = [\"c{i}\", 1.0, \"forward\", 0, []];\n"));
    }
    let r = read(&text).unwrap();
    assert_eq!(r.file.clips.len(), MAX_CLIPS);
    assert!(r.notes.iter().any(|n| n.contains("more than")));
    let frames: Vec<String> = (0..MAX_FRAMES + 2).map(|_| "[1, 10, \"\"]".to_owned()).collect();
    let r = read(&format!(
        "frames version 1;\nclip1 = [\"long\", 1.0, \"forward\", 0, [{}]];\n",
        frames.join(", ")
    ))
    .unwrap();
    assert_eq!(r.file.clips[0].frames.len(), MAX_FRAMES);
    let long_name = "n".repeat(500);
    let f = read_ok(&format!(
        "frames version 1;\nclip1 = [\"{long_name}\", 1.0, \"forward\", 0, []];\n"
    ));
    assert_eq!(f.clips[0].name.chars().count(), MAX_NAME);
}

#[test]
fn a_clips_file_is_known_by_its_extension() {
    assert!(is_frames_file(std::path::Path::new("a/walker.solframes")));
    assert!(is_frames_file(std::path::Path::new("WALKER.SOLFRAMES")));
    assert!(!is_frames_file(std::path::Path::new("walker.aseprite")));
}
