//! Tests for reading sound files, their overviews and the device.

use crate::read::{MAX_FILE_BYTES, MAX_IN_MEMORY_SECONDS, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE};
use crate::{
    Clip, Format, Overview, ReadError, Samples, SoundDevice, measure_file, read_bytes, read_file,
};
use kira::Frame;
use proptest::prelude::*;
use std::io::Cursor;
use std::path::PathBuf;

/// A WAV file made in memory with hound: `channels` channels at `rate`,
/// holding `samples` (interleaved), as `bits`-bit whole numbers.
fn wav_int(rate: u32, channels: u16, bits: u16, samples: &[i32]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = Cursor::new(Vec::new());
    let mut w = hound::WavWriter::new(&mut out, spec).unwrap();
    for &s in samples {
        match bits {
            8 => w.write_sample(s as i8).unwrap(),
            16 => w.write_sample(s as i16).unwrap(),
            _ => w.write_sample(s).unwrap(),
        }
    }
    w.finalize().unwrap();
    out.into_inner()
}

/// A 32-bit floating-point WAV file.
fn wav_float(rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut out = Cursor::new(Vec::new());
    let mut w = hound::WavWriter::new(&mut out, spec).unwrap();
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
    out.into_inner()
}

fn memory(clip: &Clip) -> &[Frame] {
    match &clip.samples {
        Samples::Memory(frames) => frames,
        Samples::Streamed(_) => panic!("expected the clip in memory"),
    }
}

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// --- WAV ------------------------------------------------------------

#[test]
fn a_mono_16_bit_wav_plays_on_both_sides() {
    let clip = read_bytes(&wav_int(44_100, 1, 16, &[0, 16_384, -32_768])).unwrap();
    assert_eq!(clip.info.format, Format::Wav);
    assert_eq!(clip.info.sample_rate, 44_100);
    assert_eq!(clip.info.channels, 1);
    assert_eq!(clip.info.frames, 3);
    let f = memory(&clip);
    assert!(close(f[1].left, 0.5) && close(f[1].right, 0.5));
    assert!(close(f[2].left, -1.0));
}

#[test]
fn a_stereo_wav_keeps_its_sides() {
    let clip = read_bytes(&wav_int(48_000, 2, 16, &[16_384, -16_384, 0, 8_192])).unwrap();
    let f = memory(&clip);
    assert_eq!(f.len(), 2);
    assert!(close(f[0].left, 0.5) && close(f[0].right, -0.5));
    assert!(close(f[1].left, 0.0) && close(f[1].right, 0.25));
}

#[test]
fn every_whole_number_size_reads_to_the_same_level() {
    for (bits, half) in [(8, 64), (16, 16_384), (24, 4_194_304), (32, 1_073_741_824)] {
        let clip = read_bytes(&wav_int(22_050, 1, bits, &[half])).unwrap();
        assert!(close(memory(&clip)[0].left, 0.5), "{bits}-bit");
    }
}

#[test]
fn floating_point_wav_is_read_and_cleaned() {
    let clip = read_bytes(&wav_float(
        48_000,
        1,
        &[0.25, f32::NAN, f32::INFINITY, 3.0, -7.0],
    ))
    .unwrap();
    let left: Vec<f32> = memory(&clip).iter().map(|f| f.left).collect();
    assert_eq!(left, [0.25, 0.0, 0.0, 1.0, -1.0]);
}

#[test]
fn the_edges_of_what_is_read() {
    // One sample, the lowest and highest rates.
    assert_eq!(
        read_bytes(&wav_int(MIN_SAMPLE_RATE, 1, 16, &[1]))
            .unwrap()
            .info
            .frames,
        1
    );
    assert_eq!(
        read_bytes(&wav_int(MAX_SAMPLE_RATE, 2, 16, &[1, 2]))
            .unwrap()
            .info
            .frames,
        1
    );
    // Silence is still a sound.
    let silence = read_bytes(&wav_int(8_000, 1, 16, &[0; 100])).unwrap();
    assert!(memory(&silence).iter().all(|f| f.left == 0.0));
}

#[test]
fn rates_outside_the_range_are_refused() {
    for rate in [MIN_SAMPLE_RATE - 1, MAX_SAMPLE_RATE + 1, 1] {
        assert_eq!(
            read_bytes(&wav_int(rate, 1, 16, &[1])).unwrap_err(),
            ReadError::SampleRate(rate)
        );
    }
}

#[test]
fn too_many_channels_are_refused_and_up_to_eight_mix_down() {
    let eight = read_bytes(&wav_int(8_000, 8, 16, &[16_384; 8])).unwrap();
    let f = memory(&eight)[0];
    assert!(close(f.left, 0.5) && close(f.right, 0.5));
    let nine = read_bytes(&wav_int(8_000, 9, 16, &[0; 9]));
    assert_eq!(nine.unwrap_err(), ReadError::Channels(9));
}

#[test]
fn a_wav_with_no_samples_is_empty() {
    assert_eq!(
        read_bytes(&wav_int(8_000, 1, 16, &[])).unwrap_err(),
        ReadError::Empty
    );
}

#[test]
fn a_cut_off_wav_keeps_its_whole_frames() {
    let mut bytes = wav_int(8_000, 2, 16, &[100, 200, 300, 400, 500, 600]);
    bytes.truncate(bytes.len() - 3); // the last frame is now incomplete
    let clip = read_bytes(&bytes).unwrap();
    assert_eq!(clip.info.frames, 2);
}

#[test]
fn files_that_arent_sound_are_refused() {
    assert_eq!(read_bytes(b"").unwrap_err(), ReadError::NotSound);
    assert_eq!(
        read_bytes(b"hello, this is text").unwrap_err(),
        ReadError::NotSound
    );
    assert_eq!(
        read_bytes(b"\x89PNG\r\n\x1a\n........").unwrap_err(),
        ReadError::NotSound
    );
    // Looks like WAV, isn't.
    let fake = read_bytes(b"RIFF\x04\x00\x00\x00WAVEjunkjunkjunk");
    assert!(
        matches!(fake.unwrap_err(), ReadError::Damaged(_)),
        "damaged header"
    );
    // Looks like Ogg, isn't.
    let fake = read_bytes(b"OggS\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00junk");
    assert!(matches!(fake.unwrap_err(), ReadError::Damaged(_)));
}

#[test]
fn errors_say_what_happened_in_plain_words() {
    for e in [
        ReadError::NotSound,
        ReadError::TooBig(300 << 20),
        ReadError::SampleRate(4),
        ReadError::Channels(0),
        ReadError::Channels(12),
        ReadError::Encoding("12-bit whole numbers".into()),
        ReadError::Damaged("why".into()),
        ReadError::Empty,
        ReadError::TooLong,
        ReadError::Io("gone".into()),
    ] {
        let text = e.to_string();
        assert!(!text.is_empty() && !text.contains("Error"), "{text}");
    }
}

// --- Long sounds ----------------------------------------------------

/// A WAV a little longer than what's held in memory, at the lowest rate.
fn too_long_wav() -> Vec<u8> {
    let frames = (MAX_IN_MEMORY_SECONDS * u64::from(MIN_SAMPLE_RATE) + 10) as usize;
    let samples: Vec<i32> = (0..frames).map(|i| (i % 100) as i32 * 300).collect();
    wav_int(MIN_SAMPLE_RATE, 1, 16, &samples)
}

#[test]
fn a_sound_too_long_for_memory_is_refused_from_bytes_and_streamed_from_a_file() {
    let bytes = too_long_wav();
    assert_eq!(read_bytes(&bytes).unwrap_err(), ReadError::TooLong);

    let dir = test_dir("long");
    let path = dir.join("long.wav");
    std::fs::write(&path, &bytes).unwrap();
    let clip = read_file(&path).unwrap();
    assert!(clip.is_streamed());
    assert_eq!(
        clip.info.frames,
        MAX_IN_MEMORY_SECONDS * u64::from(MIN_SAMPLE_RATE) + 10
    );
    assert_eq!(clip.overview.frames(), clip.info.frames);
    assert!(clip.info.seconds() > MAX_IN_MEMORY_SECONDS as f64);
}

#[test]
fn a_file_bigger_than_the_limit_is_refused_before_reading() {
    let dir = test_dir("big");
    let path = dir.join("big.wav");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap(); // sparse: nothing is written
    assert_eq!(
        read_file(&path).unwrap_err(),
        ReadError::TooBig(MAX_FILE_BYTES + 1)
    );
}

#[test]
fn a_missing_file_is_an_error_not_a_crash() {
    assert!(matches!(
        read_file(&data("not-here.ogg")).unwrap_err(),
        ReadError::Io(_)
    ));
}

// --- Ogg Vorbis -----------------------------------------------------

#[test]
fn ogg_files_read_with_their_rates_and_lengths() {
    for (name, rate, channels, seconds) in [
        ("tone_mono_44k.ogg", 44_100, 1, 0.5),
        ("tone_stereo_48k.ogg", 48_000, 2, 0.75),
        ("silence_22k.ogg", 22_050, 1, 0.2),
    ] {
        let clip = read_file(&data(name)).unwrap();
        assert_eq!(clip.info.format, Format::Ogg, "{name}");
        assert_eq!(clip.info.sample_rate, rate, "{name}");
        assert_eq!(clip.info.channels, channels, "{name}");
        assert!(
            (clip.info.seconds() - seconds).abs() < 0.03,
            "{name}: {}",
            clip.info.seconds()
        );
        assert!(!clip.is_streamed());
    }
}

#[test]
fn an_ogg_tone_has_sound_in_it_and_silence_has_none() {
    let tone = read_file(&data("tone_mono_44k.ogg")).unwrap();
    let loudest = memory(&tone)
        .iter()
        .map(|f| f.left.abs())
        .fold(0.0, f32::max);
    assert!(loudest > 0.1, "{loudest}");
    let silence = read_file(&data("silence_22k.ogg")).unwrap();
    let loudest = memory(&silence)
        .iter()
        .map(|f| f.left.abs())
        .fold(0.0, f32::max);
    assert!(loudest < 0.01, "{loudest}");
}

#[test]
fn six_channel_ogg_is_mixed_down_to_one() {
    let clip = read_file(&data("tone_six_channels.ogg")).unwrap();
    assert_eq!(clip.info.channels, 6);
    assert!(memory(&clip).iter().all(|f| f.left == f.right));
}

#[test]
fn a_cut_off_ogg_is_an_error_or_shorter_never_a_crash() {
    let bytes = std::fs::read(data("tone_stereo_48k.ogg")).unwrap();
    for cut in [10, 60, 200, bytes.len() / 2, bytes.len() - 1] {
        if let Ok(clip) = read_bytes(&bytes[..cut]) {
            assert!(clip.info.seconds() <= 0.76);
        }
    }
}

/// Found by fuzzing: an Ogg file whose comment header claims billions of
/// comments made `lewton` ask for hundreds of gigabytes before reading
/// one. It's refused as damaged now, before `lewton` sees it.
#[test]
fn an_ogg_claiming_billions_of_comments_is_refused() {
    let bytes = std::fs::read(data("huge_comment_count.ogg")).unwrap();
    let err = read_bytes(&bytes).unwrap_err();
    assert!(matches!(err, ReadError::Damaged(_)), "{err}");
}

/// Found by fuzzing too: a codebook claiming more lookup values than
/// there's memory for.
#[test]
fn an_ogg_with_a_huge_codebook_is_refused() {
    let bytes = std::fs::read(data("huge_codebook.ogg")).unwrap();
    let err = read_bytes(&bytes).unwrap_err();
    assert!(matches!(err, ReadError::Damaged(_)), "{err}");
}

#[test]
fn the_ogg_test_files_pass_the_header_check() {
    for name in [
        "tone_mono_44k.ogg",
        "tone_stereo_48k.ogg",
        "tone_six_channels.ogg",
        "silence_22k.ogg",
    ] {
        let mut file = std::io::Cursor::new(std::fs::read(data(name)).unwrap());
        crate::vorbis_check::check(&mut file).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

// --- Overviews ------------------------------------------------------

#[test]
fn an_overview_counts_every_frame_and_finds_the_peaks() {
    let mut o = Overview::new();
    let frames: Vec<Frame> = (0..10_000)
        .map(|i| Frame::from_mono(if i == 7_000 { 0.9 } else { 0.0 }))
        .collect();
    o.add(&frames);
    assert_eq!(o.frames(), 10_000);
    let cols = o.columns(10);
    assert_eq!(cols.len(), 10);
    let peak = cols.iter().position(|&(_, hi)| hi > 0.8).unwrap();
    assert_eq!(peak, 7);
}

#[test]
fn an_empty_overview_draws_nothing() {
    assert!(Overview::new().columns(32).is_empty());
    let mut o = Overview::new();
    o.add(&[Frame::from_mono(0.5)]);
    assert!(o.columns(0).is_empty());
    assert_eq!(o.columns(4).len(), 4);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// However long the sound, the overview stays small, counts every
    /// frame, and its loudest column holds the loudest sample.
    #[test]
    fn overviews_stay_small_and_true(len in 1_usize..400_000, spike in 0.0_f64..1.0, height in 0.1_f32..1.0) {
        let at = ((len as f64) * spike) as usize % len;
        let frames: Vec<Frame> = (0..len).map(|i| Frame::from_mono(if i == at { height } else { 0.0 })).collect();
        let mut o = Overview::new();
        for chunk in frames.chunks(4096) {
            o.add(chunk);
        }
        prop_assert_eq!(o.frames(), len as u64);
        prop_assert!(o.columns(100_000).len() <= 100_000);
        let top = o.columns(64).iter().map(|&(_, hi)| hi).fold(f32::MIN, f32::max);
        prop_assert!((top - height).abs() < 1e-6);
    }

    /// Damaged files never crash the reader.
    #[test]
    fn damaged_wav_files_never_crash(flips in prop::collection::vec((0_usize..200, any::<u8>()), 0..12), cut in 0_usize..200) {
        let mut bytes = wav_int(44_100, 2, 16, &(0..60).collect::<Vec<i32>>());
        for (at, value) in flips {
            if at < bytes.len() {
                bytes[at] = value;
            }
        }
        bytes.truncate(cut.min(bytes.len()));
        let _ = read_bytes(&bytes);
    }

    #[test]
    fn damaged_ogg_files_never_crash(flips in prop::collection::vec((0_usize..6000, any::<u8>()), 1..8)) {
        let mut bytes = std::fs::read(data("tone_stereo_48k.ogg")).unwrap();
        for (at, value) in flips {
            let at = at % bytes.len();
            bytes[at] = value;
        }
        let _ = read_bytes(&bytes);
    }
}

// --- The device -----------------------------------------------------

#[test]
fn the_test_device_previews_and_stops() {
    let mut device = SoundDevice::for_tests();
    assert!(!device.is_silent());
    let clip = read_bytes(&wav_int(48_000, 1, 16, &[1_000; 4_800])).unwrap();
    device.preview(&clip).unwrap();
    assert!(device.previewing());
    assert!(device.preview_position().is_some());
    // A second preview replaces the first.
    device.preview(&clip).unwrap();
    assert!(device.previewing());
    device.stop_preview();
    assert!(!device.previewing());
    assert!(device.preview_position().is_none());
}

#[test]
fn a_long_sound_previews_from_its_file() {
    let dir = test_dir("stream");
    let path = dir.join("long.wav");
    std::fs::write(&path, too_long_wav()).unwrap();
    let clip = read_file(&path).unwrap();
    let mut device = SoundDevice::for_tests();
    device.preview(&clip).unwrap();
    assert!(device.previewing());
}

#[test]
fn measuring_matches_reading_and_the_measure_plays_streamed() {
    let dir = test_dir("measure");
    let short = dir.join("short.wav");
    std::fs::write(&short, wav_int(44_100, 2, 16, &[500; 2 * 4_410])).unwrap();
    let (info, overview) = measure_file(&short).unwrap();
    let clip = read_file(&short).unwrap();
    assert_eq!(info, clip.info);
    assert_eq!(overview, clip.overview);
    // The Library plays what it measured straight from the file.
    let mut device = SoundDevice::for_tests();
    device.preview(&Clip::streamed(&short, info)).unwrap();
    assert!(device.previewing());

    let long = dir.join("long.wav");
    std::fs::write(&long, too_long_wav()).unwrap();
    let (info, overview) = measure_file(&long).unwrap();
    assert_eq!(info.frames, overview.frames());
    assert!(info.seconds() > MAX_IN_MEMORY_SECONDS as f64);

    std::fs::write(dir.join("bad.wav"), b"RIFF but not really").unwrap();
    assert!(measure_file(&dir.join("bad.wav")).is_err());
    assert!(matches!(
        measure_file(&dir.join("missing.wav")),
        Err(ReadError::Io(_))
    ));
}

#[test]
fn a_silent_device_does_nothing_without_failing() {
    let mut device = SoundDevice::silent();
    assert!(device.is_silent());
    let clip = read_bytes(&wav_int(48_000, 1, 16, &[1; 10])).unwrap();
    device.preview(&clip).unwrap();
    assert!(!device.previewing());
    device.stop_preview();
    assert!(format!("{device:?}").contains("silent"));
}

#[test]
fn the_extension_and_the_contents_name_the_format() {
    assert!(Format::is_sound_file(std::path::Path::new("a/b/Step.WAV")));
    assert!(Format::is_sound_file(std::path::Path::new("music.ogg")));
    assert!(!Format::is_sound_file(std::path::Path::new("music.mp3")));
    assert_eq!(Format::sniff(b"RIFF\0\0\0\0WAVE"), Some(Format::Wav));
    assert_eq!(Format::sniff(b"OggS"), Some(Format::Ogg));
    assert_eq!(Format::sniff(b"RIFF\0\0\0\0AVI "), None);
    assert_eq!(Format::Wav.name(), "WAV");
}

/// A fresh folder for one test.
fn test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("solis-sound-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// --- The game's sounds ----------------------------------------------

mod game_sounds {
    use super::*;
    use crate::event::{Fade, Pick};
    use crate::game::{MAX_PLAYING, Placement, Spot, Voice, within_reach};
    use reflect::{AssetId, Id};

    fn pick() -> Pick {
        Pick {
            take: AssetId(Id::from_raw(1).unwrap()),
            volume_db: -3.0,
            pitch_st: 1.0,
        }
    }

    fn spot(x: f32) -> Spot {
        Spot {
            place: [x, 0.0, 0.0],
            near_m: 1.0,
            far_m: 30.0,
            fade: Fade::Realistic,
        }
    }

    fn clip() -> Clip {
        read_bytes(&wav_int(48_000, 1, 16, &[1_000; 48_000])).unwrap()
    }

    #[test]
    fn game_sounds_play_flat_placed_and_on_entities() {
        let mut device = SoundDevice::for_tests();
        let clip = clip();
        device
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(2))
            .unwrap();
        device
            .play_game_sound(&clip, &pick(), Placement::At(spot(5.0)), Voice::on_bus(2))
            .unwrap();
        device
            .play_game_sound(
                &clip,
                &pick(),
                Placement::On(7, spot(2.0)),
                Voice::on_bus(2),
            )
            .unwrap();
        device
            .play_game_sound(
                &clip,
                &pick(),
                Placement::On(7, spot(3.0)),
                Voice::on_bus(2),
            )
            .unwrap();
        assert_eq!(device.game_sounds_playing(), 4);
        assert_eq!(device.game_sound_entities(), vec![7]);
        device.set_listener([0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0]);
        device.set_listener([f32::NAN, 0.0, 0.0], [0.0; 4]);
        device.move_game_sounds(|e| (e == 7).then_some([1.0, 2.0, 3.0]));
        device.stop_game_sounds_on(7);
        device.pause_game_sounds(true);
        // A Step's sounds play while paused.
        device
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(2))
            .unwrap();
        device.pause_game_sounds(false);
        device.stop_game_sounds();
        assert_eq!(device.game_sounds_playing(), 0, "Stop clears them");
    }

    #[test]
    fn streamed_game_sounds_play_too() {
        let dir = test_dir("game-stream");
        let path = dir.join("long.wav");
        std::fs::write(&path, wav_int(8_000, 1, 16, &[500; 8_000])).unwrap();
        let (info, _) = crate::measure_file(&path).unwrap();
        let mut device = SoundDevice::for_tests();
        device
            .play_game_sound(
                &Clip::streamed(&path, info),
                &pick(),
                Placement::At(spot(1.0)),
                Voice::on_bus(2),
            )
            .unwrap();
        assert_eq!(device.game_sounds_playing(), 1);
    }

    #[test]
    fn too_many_at_once_are_refused_and_silent_devices_do_nothing() {
        let mut device = SoundDevice::for_tests();
        let clip = clip();
        for _ in 0..MAX_PLAYING {
            device
                .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(2))
                .unwrap();
        }
        assert!(
            device
                .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(2))
                .is_err()
        );
        let mut silent = SoundDevice::silent();
        silent
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(2))
            .unwrap();
        silent.set_listener([0.0; 3], [0.0, 0.0, 0.0, 1.0]);
        silent.stop_game_sounds();
        assert_eq!(silent.game_sounds_playing(), 0);
    }

    #[test]
    fn sounds_play_through_their_bus_and_ducking_sees_them() {
        let mut device = SoundDevice::for_tests();
        let clip = clip();
        let mixer = crate::Mixer::default();
        device.set_mixer(&mixer);
        // Voice (3) and a bus that isn't there (99, so Master).
        device
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(3))
            .unwrap();
        device
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(99))
            .unwrap();
        let busy = device.busy_buses(mixer.buses.len());
        assert_eq!(busy, [true, false, false, true, false, false]);
        device.set_bus_levels(&[0.0, -8.0, 0.0, 0.0, f32::NEG_INFINITY, 0.0]);
        // The test device mixes nothing, so its meters read silence.
        assert_eq!(device.take_bus_meters(), vec![0.0; 6]);
        // The same buses again: nothing stops.
        device.set_mixer(&mixer.clone());
        assert_eq!(device.game_sounds_playing(), 2);
        // A bus added: the trees are made again, and what played stops.
        let mut more = mixer.clone();
        more.add_bus("Effects").unwrap();
        device.set_mixer(&more);
        assert_eq!(device.game_sounds_playing(), 0);
        assert_eq!(device.take_bus_meters().len(), 7);
        device
            .play_game_sound(&clip, &pick(), Placement::Flat, Voice::on_bus(3))
            .unwrap();
        assert!(device.busy_buses(7)[3]);
        let mut silent = SoundDevice::silent();
        silent.set_mixer(&more);
        silent.set_bus_levels(&[0.0; 7]);
        assert!(silent.take_bus_meters().is_empty());
        assert_eq!(silent.busy_buses(7), vec![false; 7]);
    }

    #[test]
    fn loops_play_on_until_stopped_and_are_counted_apart() {
        let mut device = SoundDevice::for_tests();
        let clip = clip();
        let looped = Voice {
            bus: 2,
            event: 9,
            looping: true,
            start_s: 0.5,
            extra_db: 3.0,
        };
        device
            .play_game_sound(&clip, &pick(), Placement::On(4, spot(2.0)), looped)
            .unwrap();
        device
            .play_game_sound(
                &clip,
                &pick(),
                Placement::On(4, spot(2.0)),
                Voice {
                    event: 9,
                    ..Voice::on_bus(2)
                },
            )
            .unwrap();
        device
            .play_game_sound(
                &clip,
                &pick(),
                Placement::Flat,
                Voice {
                    event: 1,
                    ..Voice::on_bus(2)
                },
            )
            .unwrap();
        assert_eq!(device.loops_playing(), vec![4]);
        assert_eq!(device.one_shots_playing(), 2);
        assert_eq!(device.game_sounds_of_event(9), 2);
        // Stopping the loop leaves the entity's one-shot alone.
        device.stop_loop_on(4);
        assert!(device.loops_playing().is_empty());
        assert_eq!(device.game_sounds_of_event(9), 1);
        // A start past the end, or not a number, is kept in the sound.
        for start_s in [1e9, f64::NAN, -3.0] {
            device
                .play_game_sound(
                    &clip,
                    &pick(),
                    Placement::Flat,
                    Voice {
                        start_s,
                        extra_db: f32::NAN,
                        ..looped
                    },
                )
                .unwrap();
        }
        let mut silent = SoundDevice::silent();
        assert_eq!(
            (silent.one_shots_playing(), silent.game_sounds_of_event(9)),
            (0, 0)
        );
        silent.stop_loop_on(4);
        assert!(silent.loops_playing().is_empty());
    }

    #[test]
    fn sounds_out_of_reach_are_not_started() {
        assert!(within_reach([10.0, 0.0, 0.0], [0.0; 3], 30.0));
        assert!(!within_reach([40.0, 0.0, 0.0], [0.0; 3], 30.0));
        assert!(
            within_reach([0.0, 30.0, 0.0], [0.0; 3], 30.0),
            "the edge counts"
        );
        assert!(!within_reach([f32::NAN, 0.0, 0.0], [0.0; 3], 30.0));
    }
}
