//! Tests for moving about in a sound as it's decoded (`Decoder::seek`):
//! an Ogg always starts again from its first page, through the header
//! check, and is read forward to the frame asked for.

use crate::ReadError;
use crate::read::Decoder;
use crate::tests::{data, test_dir};
use crate::vorbis_stream::VorbisStream;
use kira::Frame;
use proptest::prelude::*;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};

/// Every frame the decoder has left.
fn rest<R: Read + Seek>(decoder: &mut Decoder<R>) -> Vec<Frame> {
    let mut all = Vec::new();
    let mut chunk = Vec::new();
    while decoder.next_chunk(&mut chunk).unwrap() {
        all.append(&mut chunk);
    }
    all
}

#[test]
fn an_ogg_sought_part_way_carries_on_from_that_very_frame() {
    // Regression (SEC-18): part-way seeks went through the Ogg reader's
    // own search, which starts in the middle of the file, lands a page or
    // so early, and takes any bytes that look like a page for one.
    for name in [
        "tone_mono_44k.ogg",
        "tone_stereo_48k.ogg",
        "tone_six_channels.ogg",
        "tone_many_pages.ogg",
    ] {
        let bytes = std::fs::read(data(name)).unwrap();
        let mut decoder = Decoder::new(Cursor::new(&bytes)).unwrap();
        let whole = rest(&mut decoder);
        let len = whole.len();
        // The frames around where the pages of `tone_many_pages.ogg` end
        // (21,888, 44,416 and 66,944), among others.
        let near_pages = [21_887, 21_888, 21_889, 44_416, 50_000, 66_944, 66_945];
        let indices = [1, 100, 4095, 4096, 4097, len / 2, len - 1];
        for index in indices.into_iter().chain(near_pages) {
            if index >= len {
                continue;
            }
            assert_eq!(decoder.seek(index as u64).unwrap(), index as u64, "{name}");
            assert!(
                rest(&mut decoder) == whole[index..],
                "{name}: the sound from frame {index} on"
            );
        }
        // Past the end is the end, and it still starts again after.
        assert_eq!(decoder.seek(len as u64 + 10).unwrap(), len as u64);
        assert!(rest(&mut decoder).is_empty());
        assert_eq!(decoder.seek(len as u64).unwrap(), len as u64);
        assert!(rest(&mut decoder).is_empty());
        assert_eq!(decoder.seek(0).unwrap(), 0);
        assert!(rest(&mut decoder) == whole, "{name}: from the start again");
    }
}

#[test]
fn good_oggs_decode_to_the_samples_lewtons_own_reader_gives() {
    // Solis reads the packets itself now: the sound must be the same.
    for name in [
        "tone_mono_44k.ogg",
        "tone_stereo_48k.ogg",
        "tone_six_channels.ogg",
        "tone_many_pages.ogg",
        "silence_22k.ogg",
    ] {
        let bytes = std::fs::read(data(name)).unwrap();
        let mut theirs = lewton::inside_ogg::OggStreamReader::new(Cursor::new(&bytes)).unwrap();
        let mut expected = Vec::new();
        while let Some(packet) = theirs.read_dec_packet_itl().unwrap() {
            expected.extend(packet);
        }
        let (mut ours, at) = VorbisStream::open(Cursor::new(&bytes), 0).unwrap();
        assert_eq!(at, 0);
        let mut samples = Vec::new();
        while let Some(packet) = ours.next_packet().unwrap() {
            samples.extend(packet);
        }
        assert!(samples == expected, "{name}");
    }
}

#[test]
fn a_packet_of_another_stream_is_refused_not_read_as_a_new_sound() {
    // The second half of a chained Ogg, met while reading: the check
    // refuses such a file, and so does reading, should one get past it
    // (a file changed after the check). lewton's own reader would read
    // the second stream's headers here, unchecked.
    let good = std::fs::read(data("tone_mono_44k.ogg")).unwrap();
    let other = std::fs::read(data("tone_stereo_48k.ogg")).unwrap();
    let dir = test_dir("seek-second-stream");
    let path = dir.join("music.ogg");
    std::fs::write(&path, &good).unwrap();
    let mut decoder =
        Decoder::new(BufReader::with_capacity(64, File::open(&path).unwrap())).unwrap();
    // Longer now, after the check: a second sound follows the first.
    let mut chained = good.clone();
    chained.extend(&other);
    std::fs::write(&path, &chained).unwrap();
    let mut chunk = Vec::new();
    let err = loop {
        chunk.clear();
        match decoder.next_chunk(&mut chunk) {
            Ok(true) => {}
            Ok(false) => panic!("it read to the end"),
            Err(err) => break err,
        }
    };
    assert!(err.to_string().contains("more than one sound"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_ogg_changed_under_a_playing_stream_is_checked_again() {
    // Regression (SEC-18): starting again opened the file afresh without
    // the header check, so a file swapped after it was first opened was
    // read by lewton unchecked.
    let dir = test_dir("seek-changed");
    let path = dir.join("music.ogg");
    let good = std::fs::read(data("tone_mono_44k.ogg")).unwrap();
    // A file the header check refuses, though its first headers are good.
    let refused = std::fs::read(data("chained_after_stray_byte.ogg")).unwrap();
    for index in [0, 1_000] {
        std::fs::write(&path, &good).unwrap();
        let mut decoder = Decoder::new(BufReader::new(File::open(&path).unwrap())).unwrap();
        let mut chunk = Vec::new();
        assert!(decoder.next_chunk(&mut chunk).unwrap());
        std::fs::write(&path, &refused).unwrap();
        let err = decoder.seek(index).unwrap_err();
        assert!(err.to_string().contains("stray bytes"), "{index}: {err}");
        // Lost from now on, for the same reason, wherever it's moved to.
        assert_eq!(decoder.next_chunk(&mut chunk).unwrap_err(), err);
        assert_eq!(decoder.seek(0).unwrap_err(), err);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_ogg_with_another_rate_or_channels_since_is_refused_on_starting_again() {
    // The frames were being cut by the channels it was opened with.
    let dir = test_dir("seek-other-sound");
    let path = dir.join("music.ogg");
    std::fs::write(&path, std::fs::read(data("tone_mono_44k.ogg")).unwrap()).unwrap();
    let mut decoder = Decoder::new(BufReader::new(File::open(&path).unwrap())).unwrap();
    std::fs::write(&path, std::fs::read(data("tone_stereo_48k.ogg")).unwrap()).unwrap();
    let err = decoder.seek(0).unwrap_err();
    assert!(matches!(err, ReadError::Damaged(_)), "{err}");
    assert!(err.to_string().contains("changed since"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `tone_many_pages.ogg` written again by the `ogg` crate, so with right
/// checksums: its packets on pages that end where `breaks` says, on the
/// frames `granules` says (both gone round as often as needed).
fn paged_again(breaks: &[bool], granules: &[u64]) -> Vec<u8> {
    let bytes = std::fs::read(data("tone_many_pages.ogg")).unwrap();
    let mut reader = ogg::PacketReader::new(Cursor::new(&bytes));
    let packets: Vec<_> = std::iter::from_fn(|| reader.read_packet().unwrap()).collect();
    let mut writer = ogg::PacketWriter::new(Vec::new());
    let last = packets.len() - 1;
    for (i, packet) in packets.into_iter().enumerate() {
        // The first header on a page of its own, the other two on the next.
        let end = if i == last {
            ogg::PacketWriteEndInfo::EndStream
        } else if i == 0 || i == 2 || breaks[i % breaks.len()] {
            ogg::PacketWriteEndInfo::EndPage
        } else {
            ogg::PacketWriteEndInfo::NormalPacket
        };
        let granule = granules[i % granules.len()];
        writer
            .write_packet(packet.data.into_boxed_slice(), 7, end, granule)
            .unwrap();
    }
    writer.into_inner()
}

/// A frame a page might claim to end on: early, late, or nowhere.
fn any_granule() -> impl Strategy<Value = u64> {
    prop_oneof![Just(0), Just(u64::MAX), 0_u64..100_000, any::<u64>(),]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Wherever a damaged Ogg is moved to, it's refused or read: never a
    /// crash, and never further on than asked.
    #[test]
    fn seeking_in_damaged_ogg_files_never_crashes(
        flips in prop::collection::vec((0_usize..8000, any::<u8>()), 0..8),
        seeks in prop::collection::vec(0_u64..100_000, 1..4),
    ) {
        let mut bytes = std::fs::read(data("tone_many_pages.ogg")).unwrap();
        for (at, value) in flips {
            let at = at % bytes.len();
            bytes[at] = value;
        }
        let Ok(mut decoder) = Decoder::new(Cursor::new(&bytes)) else {
            return Ok(());
        };
        let mut chunk = Vec::new();
        for index in seeks {
            if let Ok(at) = decoder.seek(index) {
                prop_assert!(at <= index);
            }
            chunk.clear();
            let _ = decoder.next_chunk(&mut chunk);
        }
    }

    /// Pages that end on any frames at all, in any order, with right
    /// checksums: moving in the file never crashes, never lands further
    /// on than asked, and is only ever refused for what the file holds.
    #[test]
    fn pages_that_lie_about_their_frames_never_mislead_a_seek(
        breaks in prop::collection::vec(any::<bool>(), 1..12),
        granules in prop::collection::vec(any_granule(), 1..12),
        seeks in prop::collection::vec(prop_oneof![0_u64..100_000, any::<u64>()], 1..6),
    ) {
        let bytes = paged_again(&breaks, &granules);
        let mut decoder = Decoder::new(Cursor::new(&bytes)).unwrap();
        let mut chunk = Vec::new();
        for index in seeks {
            match decoder.seek(index) {
                Ok(at) => prop_assert!(at <= index),
                Err(err) => prop_assert!(!err.to_string().contains("changed"), "{}", err),
            }
            chunk.clear();
            let _ = decoder.next_chunk(&mut chunk);
        }
    }
}

#[test]
fn an_ogg_paged_again_as_it_was_reads_and_seeks_the_same() {
    // The sweep above, with nothing changed: the file it starts from.
    let original = std::fs::read(data("tone_many_pages.ogg")).unwrap();
    let mut decoder = Decoder::new(Cursor::new(&original)).unwrap();
    let whole = rest(&mut decoder);
    // A page after every eighth packet, each ending on the frame it does.
    let mut breaks = [false; 8];
    breaks[7] = true;
    let mut frame = 0;
    let mut stream = VorbisStream::open(Cursor::new(&original), 0).unwrap().0;
    let mut granules = vec![0, 0, 0];
    while let Some(packet) = stream.next_packet().unwrap() {
        frame += packet.len() as u64;
        granules.push(frame);
    }
    let bytes = paged_again(&breaks, &granules);
    let mut decoder = Decoder::new(Cursor::new(&bytes)).unwrap();
    assert!(rest(&mut decoder) == whole);
    for index in [1, 5_000, 40_000, whole.len() - 1] {
        assert_eq!(decoder.seek(index as u64).unwrap(), index as u64);
        assert!(rest(&mut decoder) == whole[index..], "from frame {index}");
    }
}
