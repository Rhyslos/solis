//! An Ogg Vorbis file as it's decoded.
//!
//! Solis reads the file's pages and packets itself (the `ogg` crate) and
//! hands `lewton` one packet at a time: the three headers once, after
//! [`crate::vorbis_check`] has passed them, and then sound. `lewton`'s own
//! file reader isn't used, for two reasons found in a full code review.
//! On meeting a page that starts another stream it reads that stream's
//! headers, which nothing has checked. And its seek searches from the
//! middle of the file for whatever looks like a page, so bytes inside a
//! packet can be taken for the first page of such a stream. Here every
//! packet must be of the stream that was checked, and moving to a frame
//! starts on a page the check walked.

use crate::read::ReadError;
use crate::vorbis_check;
use lewton::audio::{PreviousWindowRight, read_audio_packet_generic};
use lewton::header::{IdentHeader, SetupHeader};
use lewton::samples::{InterleavedSamples, Samples};
use ogg::PacketReader;
use std::io::{Read, Seek, SeekFrom};

fn damaged(why: impl ToString) -> ReadError {
    ReadError::Damaged(why.to_string())
}

/// One Ogg Vorbis stream, read a packet at a time.
pub(crate) struct VorbisStream<R: Read + Seek> {
    packets: PacketReader<R>,
    /// The stream whose headers were checked: the only one read.
    serial: u32,
    ident: IdentHeader,
    setup: SetupHeader,
    /// What the last packet leaves for the next to be mixed with.
    window: PreviousWindowRight,
    /// The frame reached, once a page has ended since reading began here.
    /// The last packet of the file is cut to end where its page says.
    at: Option<u64>,
    /// Samples decoded on the way to a frame, from that frame on.
    held: Vec<i16>,
}

impl<R: Read + Seek> VorbisStream<R> {
    /// Opens the file at frame `index` (the end, if that's sooner),
    /// returning where it actually is. Its headers are checked first,
    /// every time: this is the only way in, so `lewton` never reads
    /// headers Solis hasn't checked.
    pub(crate) fn open(mut reader: R, index: u64) -> Result<(Self, u64), ReadError> {
        reader.rewind()?;
        // The last page that ends at or before the frame, and hands on a
        // packet when reading starts on it: that packet sets the decoder
        // up, and the page says which frame comes after it. Not the header
        // pages, which end on frame 0, nor the stream's last page: read on
        // its own, it's as long as its packets, not as its page says.
        let mut start = None;
        vorbis_check::check(&mut reader, |page| {
            let starts_sound = page.whole_packets > 0 && !page.last_of_stream;
            if starts_sound && (1..=index).contains(&page.granule) {
                start = Some(page.at);
            }
        })?;
        reader.rewind()?;
        let mut packets = PacketReader::new(reader);
        let ((ident, _, setup), serial) =
            lewton::inside_ogg::read_headers(&mut packets).map_err(damaged)?;
        let mut stream = Self {
            packets,
            serial,
            ident,
            setup,
            window: PreviousWindowRight::new(),
            at: None,
            held: Vec::new(),
        };
        // From the first frame, frames are counted. From a later page,
        // where it is isn't known until that page ends.
        let known = match start {
            None => Some(0),
            Some(page) => {
                stream.packets.seek_bytes(SeekFrom::Start(page))?;
                None
            }
        };
        let at = stream.pass_to(index, known)?;
        Ok((stream, at))
    }

    /// Decodes up to frame `index`, holding on to what's past it, and
    /// returns the frame reached. `at` is the frame it's on, if known yet.
    fn pass_to(&mut self, index: u64, mut at: Option<u64>) -> Result<u64, ReadError> {
        let channels = usize::from(self.channels()).max(1);
        loop {
            match at {
                Some(at) if at == index => return Ok(at),
                // Only a file that changed between the check and now.
                Some(at) if at > index => return Err(damaged("it changed as it was read")),
                _ => {}
            }
            let Some(samples) = self.decode_packet()? else {
                return Ok(at.unwrap_or(index));
            };
            at = match at {
                None => self.at,
                Some(at) => {
                    let frames = (samples.len() / channels) as u64;
                    let wanted = index - at;
                    if frames > wanted {
                        let passed = usize::try_from(wanted).unwrap_or(usize::MAX);
                        self.held = samples[passed * channels..].to_vec();
                    }
                    Some(at + frames.min(wanted))
                }
            };
        }
    }

    /// Samples a second.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.ident.audio_sample_rate
    }

    /// Channels in the file.
    pub(crate) fn channels(&self) -> u16 {
        u16::from(self.ident.audio_channels)
    }

    /// The file it reads, to open it again.
    pub(crate) fn into_reader(self) -> R {
        self.packets.into_inner()
    }

    /// The next packet's samples, channel by channel within each frame,
    /// or `None` at the end.
    pub(crate) fn next_packet(&mut self) -> Result<Option<Vec<i16>>, ReadError> {
        if self.held.is_empty() {
            self.decode_packet()
        } else {
            Ok(Some(std::mem::take(&mut self.held)))
        }
    }

    /// Reads and decodes one packet, as `lewton`'s own reader does, but
    /// for a packet of another stream, which is refused.
    fn decode_packet(&mut self) -> Result<Option<Vec<i16>>, ReadError> {
        let Some(packet) = self.packets.read_packet().map_err(damaged)? else {
            return Ok(None);
        };
        if packet.stream_serial() != self.serial {
            return Err(damaged(vorbis_check::MORE_THAN_ONE_SOUND));
        }
        let mut decoded: InterleavedSamples<i16> =
            read_audio_packet_generic(&self.ident, &self.setup, &packet.data, &mut self.window)
                .map_err(damaged)?;
        // The file's last packet is cut to end where its page says the
        // sound does (the Vorbis specification).
        if let (Some(at), true) = (self.at, packet.last_in_stream()) {
            let left = packet.absgp_page().saturating_sub(at);
            if let Some(left) = usize::try_from(left)
                .ok()
                .filter(|&left| left < decoded.num_samples())
            {
                decoded.truncate(left);
            }
        }
        if packet.last_in_page() {
            self.at = Some(packet.absgp_page());
        } else if let Some(at) = &mut self.at {
            *at = at.saturating_add(decoded.num_samples() as u64);
        }
        Ok(Some(decoded.samples))
    }
}
