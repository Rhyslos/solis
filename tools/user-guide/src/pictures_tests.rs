use super::*;

/// A PNG's first 24 bytes: the signature and the header with its size.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut data = PNG_START.to_vec();
    data.extend_from_slice(&13u32.to_be_bytes());
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 6, 0, 0, 0]);
    data
}

#[test]
fn a_png_s_size_is_read_from_its_header() {
    assert_eq!(size(&png(1440, 900)), Some((1440, 900)));
    assert_eq!(size(&png(0, 900)), None, "no width is no picture");
    assert_eq!(size(b"GIF89a and more bytes here"), None);
    assert_eq!(size(&png(10, 10)[..20]), None, "cut short");
    assert_eq!(size(&[]), None);
}

#[test]
fn a_picture_that_isnt_a_png_or_is_too_large_is_a_problem() {
    assert_eq!(problem("a.png", &png(4, 4)), None);
    let not = problem("a.png", b"not a picture at all, just words").unwrap();
    assert!(not.contains("isn't a PNG") && not.contains(RETAKE), "{not}");
    let mut big = png(4, 4);
    big.resize(LARGEST + 1, 0);
    let large = problem("big.png", &big).unwrap();
    assert!(
        large.contains("big.png") && large.contains("200 KB"),
        "{large}"
    );
    big.truncate(LARGEST);
    assert_eq!(
        problem("big.png", &big),
        None,
        "exactly the largest is fine"
    );
}

#[test]
fn the_hash_is_fnv_1a() {
    // The published test values for FNV-1a, 64 bits.
    assert_eq!(hash(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(hash(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(hash(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn a_data_address_is_base64() {
    assert_eq!(data_address(b"Man"), "data:image/png;base64,TWFu");
}

#[test]
fn the_stamp_lists_the_interface_s_pictures_in_order_and_notices_a_change() {
    let root = std::env::temp_dir().join(format!("user-guide-stamp-{}", std::process::id()));
    let folder = root.join(INTERFACE);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::create_dir_all(root.join(STAMP).parent().unwrap()).unwrap();
    std::fs::write(folder.join("b.png"), b"bee").unwrap();
    std::fs::write(folder.join("a.png"), b"ay").unwrap();
    std::fs::write(folder.join("README.md"), b"not a picture").unwrap();
    let stamp = stamp(&root).unwrap();
    let lines: Vec<&str> = stamp.lines().filter(|l| !l.starts_with('#')).collect();
    assert_eq!(lines.len(), 2, "{stamp}");
    assert!(lines[0].starts_with("a.png ") && lines[1].starts_with("b.png "));
    assert!(older_than_interface(&root).unwrap(), "no stamp yet");
    // Kept with Windows line ends, it is the same stamp.
    std::fs::write(root.join(STAMP), stamp.replace('\n', "\r\n")).unwrap();
    assert!(!older_than_interface(&root).unwrap());
    std::fs::write(folder.join("b.png"), b"bee, changed").unwrap();
    assert!(older_than_interface(&root).unwrap(), "a changed picture");
    std::fs::write(root.join(STAMP), super::stamp(&root).unwrap()).unwrap();
    std::fs::write(folder.join("c.png"), b"new").unwrap();
    assert!(older_than_interface(&root).unwrap(), "a new picture");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn only_pngs_are_read_and_no_folder_is_no_pictures() {
    let root = std::env::temp_dir().join(format!("user-guide-read-{}", std::process::id()));
    assert!(read(&root).unwrap().is_empty());
    let folder = root.join(FOLDER);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("one.png"), png(2, 2)).unwrap();
    std::fs::write(folder.join("notes.txt"), b"words").unwrap();
    let pictures = read(&root).unwrap();
    assert_eq!(pictures.keys().collect::<Vec<_>>(), ["one.png"]);
    std::fs::remove_dir_all(&root).unwrap();
}
