//! Reading sound files: WAV through `hound` and Ogg Vorbis through
//! `lewton` (decision 0056), within limits, never crashing on a damaged
//! file. Short sounds are decoded into memory; long ones (past
//! [`MAX_IN_MEMORY_SECONDS`]) are measured once and then streamed from
//! their file as they play.

use crate::overview::Overview;
use kira::Frame;
use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The biggest sound file read (256 MB).
pub const MAX_FILE_BYTES: u64 = 256 << 20;
/// The lowest sample rate read, in Hz.
pub const MIN_SAMPLE_RATE: u32 = 8_000;
/// The highest sample rate read, in Hz.
pub const MAX_SAMPLE_RATE: u32 = 192_000;
/// The most channels read. Mono and stereo play as they are; more are
/// mixed down to one.
pub const MAX_CHANNELS: u16 = 8;
/// Sounds up to this long (10 minutes) are held in memory; longer ones
/// are streamed from their file.
pub const MAX_IN_MEMORY_SECONDS: u64 = 600;

/// How many frames are decoded at a time.
const CHUNK_FRAMES: usize = 4096;

/// A sound file's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// WAV (RIFF WAVE): uncompressed, for short effects.
    Wav,
    /// Ogg Vorbis: compressed, for music and anything long.
    Ogg,
}

impl Format {
    /// Its name, as people know it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Wav => "WAV",
            Self::Ogg => "Ogg Vorbis",
        }
    }

    /// The format of a file starting with `head`, if Solis reads it.
    pub fn sniff(head: &[u8]) -> Option<Self> {
        if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WAVE" {
            Some(Self::Wav)
        } else if head.starts_with(b"OggS") {
            Some(Self::Ogg)
        } else {
            None
        }
    }

    /// Whether a file's extension is one Solis reads as sound (`.wav`,
    /// `.ogg`). The contents decide in the end ([`Format::sniff`]).
    pub fn is_sound_file(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("wav") || e.eq_ignore_ascii_case("ogg"))
    }
}

/// What a sound file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipInfo {
    /// Its format.
    pub format: Format,
    /// Samples a second, per channel.
    pub sample_rate: u32,
    /// Channels in the file (1 mono, 2 stereo, more are mixed down).
    pub channels: u16,
    /// Frames (one sample per channel each) it plays.
    pub frames: u64,
}

impl ClipInfo {
    /// How long it plays, in seconds.
    pub fn seconds(&self) -> f64 {
        self.frames as f64 / f64::from(self.sample_rate.max(1))
    }
}

/// Where a clip's samples are.
#[derive(Debug, Clone)]
pub enum Samples {
    /// Decoded, in memory.
    Memory(Arc<[Frame]>),
    /// Too long to hold: read from this file as it plays.
    Streamed(PathBuf),
}

/// A sound file, read: what it is, an overview for drawing its waveform,
/// and its samples (or where to stream them from).
#[derive(Debug, Clone)]
pub struct Clip {
    /// What it holds.
    pub info: ClipInfo,
    /// Loudness over its length, for its waveform.
    pub overview: Overview,
    /// Its samples.
    pub samples: Samples,
}

impl Clip {
    /// Whether it's streamed from its file rather than held in memory.
    pub fn is_streamed(&self) -> bool {
        matches!(self.samples, Samples::Streamed(_))
    }
}

/// Why a sound file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The file couldn't be opened or read.
    Io(String),
    /// It isn't WAV or Ogg Vorbis.
    NotSound,
    /// It's bigger than [`MAX_FILE_BYTES`].
    TooBig(u64),
    /// Its sample rate is outside the range read.
    SampleRate(u32),
    /// It has no channels, or more than [`MAX_CHANNELS`].
    Channels(u16),
    /// Its samples are in a form the reader doesn't know.
    Encoding(String),
    /// It's damaged.
    Damaged(String),
    /// It holds no sound at all.
    Empty,
    /// It's too long to hold in memory (only from [`read_bytes`], which
    /// has no file to stream from).
    TooLong,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(why) => write!(f, "couldn't read it: {why}"),
            Self::NotSound => f.write_str("it isn't a WAV or Ogg Vorbis file"),
            Self::TooBig(bytes) => write!(
                f,
                "it's {} MB; sound files up to {} MB are read",
                bytes >> 20,
                MAX_FILE_BYTES >> 20
            ),
            Self::SampleRate(rate) => write!(
                f,
                "its sample rate is {rate} Hz; {MIN_SAMPLE_RATE} to {MAX_SAMPLE_RATE} Hz are read"
            ),
            Self::Channels(0) => f.write_str("it has no channels"),
            Self::Channels(n) => {
                write!(f, "it has {n} channels; up to {MAX_CHANNELS} are read")
            }
            Self::Encoding(what) => write!(f, "its samples are {what}, which isn't read"),
            Self::Damaged(why) => write!(f, "it's damaged ({why})"),
            Self::Empty => f.write_str("it holds no sound"),
            Self::TooLong => write!(
                f,
                "it's longer than {} minutes, too long to hold in memory",
                MAX_IN_MEMORY_SECONDS / 60
            ),
        }
    }
}

impl std::error::Error for ReadError {}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Reads a sound file: held in memory if it's up to
/// [`MAX_IN_MEMORY_SECONDS`] long, otherwise measured and streamed from
/// the file when played.
pub fn read_file(path: &Path) -> Result<Clip, ReadError> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    if size > MAX_FILE_BYTES {
        return Err(ReadError::TooBig(size));
    }
    let mut decoder = Decoder::new(BufReader::new(file))?;
    let (frames, overview) = decode_all(&mut decoder, true)?;
    let info = decoder.info_with(overview.frames());
    let samples = match frames {
        Some(frames) => Samples::Memory(frames.into()),
        None => Samples::Streamed(path.to_path_buf()),
    };
    Ok(Clip {
        info,
        overview,
        samples,
    })
}

/// Reads a whole sound file already in memory (tests and fuzzing). A
/// sound too long to hold is [`ReadError::TooLong`], since there's no
/// file to stream it from.
pub fn read_bytes(bytes: &[u8]) -> Result<Clip, ReadError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ReadError::TooBig(bytes.len() as u64));
    }
    let mut decoder = Decoder::new(Cursor::new(bytes))?;
    let (frames, overview) = decode_all(&mut decoder, false)?;
    let frames = frames.ok_or(ReadError::TooLong)?;
    Ok(Clip {
        info: decoder.info_with(overview.frames()),
        overview,
        samples: Samples::Memory(frames.into()),
    })
}

/// Decodes every frame. Keeps them while they fit in memory; past that,
/// keeps going only to measure, if `may_stream`, or stops.
fn decode_all<R: Read + Seek>(
    decoder: &mut Decoder<R>,
    may_stream: bool,
) -> Result<(Option<Vec<Frame>>, Overview), ReadError> {
    let limit = MAX_IN_MEMORY_SECONDS * u64::from(decoder.sample_rate());
    let mut kept = Some(Vec::new());
    let mut overview = Overview::new();
    let mut chunk = Vec::with_capacity(CHUNK_FRAMES);
    loop {
        chunk.clear();
        if !decoder.next_chunk(&mut chunk)? {
            break;
        }
        overview.add(&chunk);
        if let Some(frames) = kept.as_mut() {
            if frames.len() as u64 + chunk.len() as u64 > limit {
                kept = None;
                if !may_stream {
                    return Ok((None, overview));
                }
            } else {
                frames.extend_from_slice(&chunk);
            }
        }
    }
    if overview.frames() == 0 {
        return Err(ReadError::Empty);
    }
    Ok((kept, overview))
}

/// Which reader a file needs, and its state.
enum Inner<R: Read + Seek> {
    /// WAV, through hound, with how its samples are stored.
    Wav(hound::WavReader<R>, WavSamples),
    /// Ogg Vorbis, through lewton, with samples decoded but not yet handed out.
    Ogg(Box<lewton::inside_ogg::OggStreamReader<R>>, Vec<i16>),
}

/// How a WAV file's samples are stored.
#[derive(Debug, Clone, Copy)]
enum WavSamples {
    /// Whole numbers of this many bits (8, 16, 24 or 32).
    Int(u16),
    /// 32-bit floating point.
    Float,
}

/// Decodes a sound file a chunk at a time, into stereo frames. Shared by
/// reading into memory and by streaming while playing.
pub(crate) struct Decoder<R: Read + Seek> {
    inner: Inner<R>,
    format: Format,
    sample_rate: u32,
    channels: u16,
}

impl<R: Read + Seek> Decoder<R> {
    /// Reads the file's header, refusing what's outside the limits.
    pub(crate) fn new(mut reader: R) -> Result<Self, ReadError> {
        let mut head = [0_u8; 12];
        let read = read_up_to(&mut reader, &mut head)?;
        let format = Format::sniff(&head[..read]).ok_or(ReadError::NotSound)?;
        reader.rewind()?;
        let (inner, sample_rate, channels) = match format {
            Format::Wav => {
                let wav = hound::WavReader::new(reader).map_err(wav_error)?;
                let spec = wav.spec();
                let samples = match (spec.sample_format, spec.bits_per_sample) {
                    (hound::SampleFormat::Int, bits @ (8 | 16 | 24 | 32)) => WavSamples::Int(bits),
                    (hound::SampleFormat::Float, 32) => WavSamples::Float,
                    (hound::SampleFormat::Int, bits) => {
                        return Err(ReadError::Encoding(format!("{bits}-bit whole numbers")));
                    }
                    (hound::SampleFormat::Float, bits) => {
                        return Err(ReadError::Encoding(format!("{bits}-bit floating point")));
                    }
                };
                (Inner::Wav(wav, samples), spec.sample_rate, spec.channels)
            }
            Format::Ogg => {
                check_ogg_comments(&mut reader)?;
                reader.rewind()?;
                let ogg = lewton::inside_ogg::OggStreamReader::new(reader)
                    .map_err(|e| ReadError::Damaged(e.to_string()))?;
                let rate = ogg.ident_hdr.audio_sample_rate;
                let channels = u16::from(ogg.ident_hdr.audio_channels);
                (Inner::Ogg(Box::new(ogg), Vec::new()), rate, channels)
            }
        };
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(ReadError::SampleRate(sample_rate));
        }
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(ReadError::Channels(channels));
        }
        Ok(Self {
            inner,
            format,
            sample_rate,
            channels,
        })
    }

    /// Samples a second.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// What it holds, given how many frames it turned out to have.
    pub(crate) fn info_with(&self, frames: u64) -> ClipInfo {
        ClipInfo {
            format: self.format,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames,
        }
    }

    /// Decodes up to [`CHUNK_FRAMES`] more frames into `out`. Returns
    /// whether any came (false at the end). A WAV file that ends early
    /// ends there, keeping what it had.
    pub(crate) fn next_chunk(&mut self, out: &mut Vec<Frame>) -> Result<bool, ReadError> {
        let channels = usize::from(self.channels);
        match &mut self.inner {
            Inner::Wav(wav, kind) => {
                let mut frame = Vec::with_capacity(channels);
                while out.len() < CHUNK_FRAMES {
                    frame.clear();
                    for _ in 0..channels {
                        let sample = match *kind {
                            WavSamples::Int(bits) => wav
                                .samples::<i32>()
                                .next()
                                .map(|s| s.map(|v| int_sample(v, bits))),
                            WavSamples::Float => wav.samples::<f32>().next(),
                        };
                        match sample {
                            Some(Ok(v)) => frame.push(clean(v)),
                            // The data ran out part-way (a cut-off file):
                            // keep what was whole.
                            Some(Err(hound::Error::IoError(_))) => {
                                return Ok(!out.is_empty());
                            }
                            Some(Err(e)) => return Err(wav_error(e)),
                            None => return Ok(!out.is_empty()),
                        }
                    }
                    out.push(to_frame(&frame));
                }
                Ok(true)
            }
            Inner::Ogg(ogg, pending) => {
                while out.len() < CHUNK_FRAMES {
                    if pending.len() < channels {
                        match ogg.read_dec_packet_itl() {
                            Ok(Some(packet)) => {
                                pending.extend_from_slice(&packet);
                                continue;
                            }
                            Ok(None) => return Ok(!out.is_empty()),
                            Err(e) => return Err(ReadError::Damaged(e.to_string())),
                        }
                    }
                    let whole = (pending.len() / channels).min(CHUNK_FRAMES - out.len());
                    let mut frame = Vec::with_capacity(channels);
                    for f in pending.chunks_exact(channels).take(whole) {
                        frame.clear();
                        frame.extend(f.iter().map(|&s| f32::from(s) / 32_768.0));
                        out.push(to_frame(&frame));
                    }
                    pending.drain(..whole * channels);
                }
                Ok(true)
            }
        }
    }

    /// Moves to frame `index` (or a little before it, for Ogg), returning
    /// where it actually is.
    pub(crate) fn seek(&mut self, index: u64) -> Result<u64, ReadError> {
        match &mut self.inner {
            Inner::Wav(wav, _) => {
                let to = u32::try_from(index).unwrap_or(u32::MAX).min(wav.duration());
                wav.seek(to)?;
                Ok(u64::from(to))
            }
            Inner::Ogg(ogg, pending) => {
                pending.clear();
                ogg.seek_absgp_pg(index)
                    .map_err(|e| ReadError::Damaged(e.to_string()))?;
                Ok(index)
            }
        }
    }
}

/// Fills `buf` as far as the reader goes, returning how much it read.
fn read_up_to(reader: &mut impl Read, buf: &mut [u8]) -> Result<usize, ReadError> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(filled)
}

/// A whole-number sample of `bits` bits, as -1 to 1.
fn int_sample(value: i32, bits: u16) -> f32 {
    // 8-bit WAV is unsigned; hound already centres it on 0.
    let scale = (1_i64 << (bits.clamp(1, 32) - 1)) as f32;
    value as f32 / scale
}

/// A sample kept in -1 to 1, with nothing that isn't a number.
fn clean(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// One frame from a file's channels: mono to both sides, stereo as it
/// is, more mixed down to one.
fn to_frame(channels: &[f32]) -> Frame {
    match channels {
        [mono] => Frame::from_mono(*mono),
        [left, right] => Frame {
            left: *left,
            right: *right,
        },
        many => {
            let mixed = many.iter().sum::<f32>() / many.len().max(1) as f32;
            Frame::from_mono(mixed)
        }
    }
}

/// hound's errors, in plain words.
fn wav_error(e: hound::Error) -> ReadError {
    match e {
        hound::Error::IoError(e) => ReadError::Damaged(e.to_string()),
        hound::Error::FormatError(why) => ReadError::Damaged(why.to_string()),
        hound::Error::Unsupported => ReadError::Encoding("in a form WAV readers rarely use".into()),
        other => ReadError::Damaged(other.to_string()),
    }
}

/// Checks an Ogg Vorbis file's comment header (its second packet: the
/// title, artist and the like) before `lewton` reads it. `lewton` sets
/// aside room for as many comments, and as long, as the header claims,
/// up to billions, before reading any; a damaged or hostile file could
/// ask for far more memory than there is. Every claimed length must fit
/// in what's left of the packet.
pub(crate) fn check_ogg_comments<R: Read + Seek>(reader: &mut R) -> Result<(), ReadError> {
    let damaged = |why: &str| ReadError::Damaged(why.to_owned());
    let mut packets = ogg::PacketReader::new(reader);
    let first = packets
        .read_packet_expected()
        .map_err(|e| ReadError::Damaged(e.to_string()))?;
    let serial = first.stream_serial();
    let comments = loop {
        let packet = packets
            .read_packet_expected()
            .map_err(|e| ReadError::Damaged(e.to_string()))?;
        if packet.stream_serial() == serial {
            break packet;
        }
    };
    let data = comments.data.as_slice();
    // The packet's kind (3) and "vorbis"; lewton checks these itself.
    let mut rest = data.get(7..).ok_or_else(|| damaged("its comments are cut off"))?;
    let mut take = |rest: &mut &[u8]| -> Result<usize, ReadError> {
        let (len, after) = rest
            .split_first_chunk::<4>()
            .ok_or_else(|| damaged("its comments are cut off"))?;
        *rest = after;
        Ok(u32::from_le_bytes(*len) as usize)
    };
    let vendor = take(&mut rest)?;
    rest = rest
        .get(vendor..)
        .ok_or_else(|| damaged("its comments are longer than the file"))?;
    let count = take(&mut rest)?;
    if count > rest.len() / 4 {
        return Err(damaged("it claims more comments than it holds"));
    }
    for _ in 0..count {
        let len = take(&mut rest)?;
        rest = rest
            .get(len..)
            .ok_or_else(|| damaged("its comments are longer than the file"))?;
    }
    Ok(())
}
