//! Checks an Ogg Vorbis file's headers before `lewton` reads them.
//!
//! `lewton` sets aside memory for as many things as a header claims
//! (comments, codebook entries, lookup values) before reading any of
//! them. A damaged or hostile file can claim billions and ask for far more
//! memory than there is, which ends the program rather than returning an
//! error. Fuzzing found both cases (crates/sound/tests/data). So Solis
//! reads the two headers that hold such counts itself first and refuses
//! a file whose counts don't fit in the bytes it has, or are beyond what
//! the Vorbis reference decoder accepts.
//!
//! Only the counts are checked here; `lewton` still does the real reading.

use crate::read::ReadError;
use std::io::{Read, Seek};

/// The most lookup values all of a file's codebooks may unpack into, in
/// all (16 million numbers, 64 MB). Real files use well under a million.
const MAX_LOOKUP_TOTAL: u64 = 1 << 24;

/// The most codebook entries a file may have, in all.
const MAX_ENTRIES_TOTAL: u64 = 1 << 24;

fn damaged(why: &str) -> ReadError {
    ReadError::Damaged(why.to_owned())
}

/// Checks the comment and setup headers of the Vorbis stream in `reader`.
/// Leaves the reader anywhere: rewind it before reading the file.
pub(crate) fn check<R: Read + Seek>(reader: &mut R) -> Result<(), ReadError> {
    one_stream(reader)?;
    reader.rewind()?;
    let mut packets = ogg::PacketReader::new(reader);
    let mut next = || {
        packets
            .read_packet_expected()
            .map_err(|e| ReadError::Damaged(e.to_string()))
    };
    let serial = next()?.stream_serial();
    // The next two packets of the same stream, as `lewton` finds them.
    let mut ours = || loop {
        let packet = next()?;
        if packet.stream_serial() == serial {
            return Ok::<_, ReadError>(packet);
        }
    };
    comments(&ours()?.data)?;
    setup(&ours()?.data)
}

/// The most pages a file is scanned for a second stream (a 256 MB file of
/// the smallest pages Vorbis writes has far fewer).
const MAX_PAGES: usize = 4 << 20;

/// Refuses a file holding more than one stream (a chained Ogg, several
/// sounds one after another). `lewton` reads a later stream's headers when
/// it meets it, unchecked, and a hostile one asks for more memory than
/// there is (the Phase 7 review). Only the pages' headers are read: their
/// contents are skipped.
fn one_stream<R: Read + Seek>(reader: &mut R) -> Result<(), ReadError> {
    let mut first_serial = None;
    for _ in 0..MAX_PAGES {
        let mut header = [0_u8; 27];
        let mut got = 0;
        while got < header.len() {
            match reader.read(&mut header[got..])? {
                0 => break,
                n => got += n,
            }
        }
        if got == 0 {
            return Ok(());
        }
        // A short or broken page: lewton reports it as it reads.
        if got < header.len() || &header[..4] != b"OggS" {
            return Ok(());
        }
        let first_of_stream = header[5] & 0x02 != 0;
        let serial = u32::from_le_bytes([header[14], header[15], header[16], header[17]]);
        match first_serial {
            None => first_serial = Some(serial),
            Some(first) if serial != first || first_of_stream => {
                return Err(damaged(
                    "it holds more than one sound one after another (a chained Ogg): save it as \
                     one",
                ));
            }
            Some(_) => {}
        }
        let segments = usize::from(header[26]);
        let mut lacing = [0_u8; 255];
        // A cut-off last page: lewton reads what's whole before it.
        if reader.read_exact(&mut lacing[..segments]).is_err() {
            return Ok(());
        }
        let body: i64 = lacing[..segments].iter().map(|&l| i64::from(l)).sum();
        reader.seek(std::io::SeekFrom::Current(body))?;
    }
    Err(damaged("it has more pages than any sound Solis reads"))
}

/// The comment header: a vendor string, then a count of comments, each a
/// length and its text. Every length must fit in what's left.
pub(crate) fn comments(data: &[u8]) -> Result<(), ReadError> {
    // The packet's kind (3) and "vorbis"; lewton checks these itself.
    let mut rest = data
        .get(7..)
        .ok_or_else(|| damaged("its comments are cut off"))?;
    let take = |rest: &mut &[u8]| -> Result<usize, ReadError> {
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

/// The setup header's codebooks: how many entries each has, and how many
/// lookup values it unpacks into. (What follows the codebooks holds only
/// small counts.)
pub(crate) fn setup(data: &[u8]) -> Result<(), ReadError> {
    let cut = || damaged("its setup is cut off");
    // The packet's kind (5) and "vorbis"; lewton checks these itself.
    let mut bits = Bits::new(data.get(7..).ok_or_else(cut)?);
    let books = bits.read(8).ok_or_else(cut)? + 1;
    let mut lookup_total = 0_u64;
    let mut entries_total = 0_u64;
    for _ in 0..books {
        if bits.read(24).ok_or_else(cut)? != 0x56_43_42 {
            // Not a codebook: lewton refuses it with a clearer reason.
            return Ok(());
        }
        let dimensions = bits.read(16).ok_or_else(cut)?;
        let entries = bits.read(24).ok_or_else(cut)?;
        // The reference decoder's own limit.
        if ilog(dimensions) + ilog(entries) > 24 {
            return Err(damaged("a codebook in it is too big"));
        }
        entries_total += entries;
        if entries_total > MAX_ENTRIES_TOTAL {
            return Err(damaged("its codebooks are too big"));
        }
        let ordered = bits.read(1).ok_or_else(cut)? == 1;
        if ordered {
            bits.read(5).ok_or_else(cut)?;
            let mut entry = 0;
            while entry < entries {
                let number = bits.read(ilog(entries - entry)).ok_or_else(cut)?;
                entry += number;
                if number == 0 && entry < entries {
                    // lewton would loop on; nothing more fits.
                    if bits.left() == 0 {
                        return Err(cut());
                    }
                }
            }
            if entry > entries {
                return Err(damaged("a codebook in it is malformed"));
            }
        } else {
            let sparse = bits.read(1).ok_or_else(cut)? == 1;
            if entries > bits.left() {
                return Err(cut());
            }
            for _ in 0..entries {
                let used = !sparse || bits.read(1).ok_or_else(cut)? == 1;
                if used {
                    bits.read(5).ok_or_else(cut)?;
                }
            }
        }
        let lookup = bits.read(4).ok_or_else(cut)?;
        if lookup == 0 {
            continue;
        }
        if lookup > 2 {
            // lewton refuses it.
            return Ok(());
        }
        bits.read(32).ok_or_else(cut)?;
        bits.read(32).ok_or_else(cut)?;
        let value_bits = bits.read(4).ok_or_else(cut)? + 1;
        bits.read(1).ok_or_else(cut)?;
        let values = if lookup == 1 {
            lookup1_values(entries, dimensions)
        } else {
            entries * dimensions
        };
        if values.saturating_mul(value_bits) > bits.left() {
            return Err(cut());
        }
        bits.skip(values * value_bits);
        // lewton unpacks every entry's vector: entries × dimensions.
        lookup_total += entries * dimensions;
        if lookup_total > MAX_LOOKUP_TOTAL {
            return Err(damaged("its codebooks are too big"));
        }
    }
    Ok(())
}

/// How many bits `value` needs (Vorbis's `ilog`): 0 for 0.
fn ilog(value: u64) -> u64 {
    u64::from(64 - value.leading_zeros())
}

/// The biggest whole number whose `dimensions`th power is at most
/// `entries` (Vorbis's `lookup1_values`).
fn lookup1_values(entries: u64, dimensions: u64) -> u64 {
    if dimensions == 0 {
        // lewton gives the largest u32 here; it's then far too many.
        return u64::from(u32::MAX);
    }
    let fits = |r: u64| {
        u32::try_from(dimensions)
            .ok()
            .and_then(|d| r.checked_pow(d))
            .is_some_and(|p| p <= entries)
    };
    // Start near the answer and correct for rounding.
    let mut r = (entries as f64).powf(1.0 / dimensions as f64) as u64;
    while r > 0 && !fits(r) {
        r -= 1;
    }
    while fits(r + 1) {
        r += 1;
    }
    r
}

/// Reads bits least significant first, as Vorbis packs them.
struct Bits<'a> {
    data: &'a [u8],
    /// The next bit, counted from the start.
    at: u64,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    /// Bits not read yet.
    fn left(&self) -> u64 {
        (self.data.len() as u64 * 8).saturating_sub(self.at)
    }

    /// The next `n` bits (at most 32), or `None` past the end.
    fn read(&mut self, n: u64) -> Option<u64> {
        if n > self.left() {
            return None;
        }
        let mut value = 0;
        for i in 0..n {
            let at = self.at + i;
            let byte = self.data[(at / 8) as usize];
            value |= u64::from((byte >> (at % 8)) & 1) << i;
        }
        self.at += n;
        Some(value)
    }

    /// Passes over `n` bits (the caller checked they're there).
    fn skip(&mut self, n: u64) {
        self.at += n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup1_values_matches_the_specification() {
        // The Vorbis I specification's examples, as lewton tests them.
        assert_eq!(lookup1_values(1025, 10), 2);
        assert_eq!(lookup1_values(1024, 10), 2);
        assert_eq!(lookup1_values(1023, 10), 1);
        assert_eq!(lookup1_values(3126, 5), 5);
        assert_eq!(lookup1_values(3125, 5), 5);
        assert_eq!(lookup1_values(3124, 5), 4);
        assert_eq!(lookup1_values(1, 1), 1);
        assert_eq!(lookup1_values(0, 15), 0);
        assert_eq!(lookup1_values(81, 2), 9);
        assert_eq!(lookup1_values(1 << 20, 40), 1);
    }

    #[test]
    fn ilog_matches_the_specification() {
        for (v, want) in [(0, 0), (1, 1), (2, 2), (3, 2), (4, 3), (7, 3)] {
            assert_eq!(ilog(v), want);
        }
    }

    #[test]
    fn bits_are_read_least_significant_first() {
        let mut bits = Bits::new(&[0b1010_1100, 0xFF]);
        assert_eq!(bits.read(2), Some(0));
        assert_eq!(bits.read(3), Some(0b011));
        assert_eq!(bits.read(5), Some(0b11_101));
        assert_eq!(bits.left(), 6);
        assert_eq!(bits.read(7), None);
    }

    /// A setup header with one codebook, packed by hand.
    fn one_book(dimensions: u64, entries: u64, lookup: u64, values_in_file: u64) -> Vec<u8> {
        let mut w = Writer::default();
        w.put(0, 8); // one codebook
        w.put(0x56_43_42, 24);
        w.put(dimensions, 16);
        w.put(entries, 24);
        w.put(0, 1); // not ordered
        w.put(0, 1); // not sparse
        for _ in 0..entries {
            w.put(3, 5);
        }
        w.put(lookup, 4);
        if lookup != 0 {
            w.put(0, 32);
            w.put(0, 32);
            w.put(7, 4); // 8 bits a value
            w.put(0, 1);
            for _ in 0..values_in_file {
                w.put(1, 8);
            }
        }
        let mut packet = vec![5, b'v', b'o', b'r', b'b', b'i', b's'];
        packet.extend(w.bytes);
        packet
    }

    #[derive(Default)]
    struct Writer {
        bytes: Vec<u8>,
        at: u64,
    }

    impl Writer {
        fn put(&mut self, value: u64, n: u64) {
            for i in 0..n {
                if self.at.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                if (value >> i) & 1 == 1 {
                    *self.bytes.last_mut().unwrap() |= 1 << (self.at % 8);
                }
                self.at += 1;
            }
        }
    }

    #[test]
    fn a_sound_codebook_passes() {
        assert!(setup(&one_book(2, 16, 1, 4)).is_ok());
        assert!(setup(&one_book(2, 16, 2, 32)).is_ok());
        assert!(setup(&one_book(4, 81, 0, 0)).is_ok());
    }

    #[test]
    fn lookup_values_the_file_doesnt_hold_are_refused() {
        assert!(setup(&one_book(2, 16, 2, 10)).is_err());
        assert!(setup(&one_book(2, 16, 1, 3)).is_err());
    }

    #[test]
    fn a_codebook_bigger_than_the_reference_decoder_allows_is_refused() {
        // ilog(65535) + ilog(1024) = 16 + 11 > 24.
        assert!(setup(&one_book(65_535, 1024, 0, 0)).is_err());
    }

    #[test]
    fn a_cut_off_setup_is_refused() {
        let book = one_book(2, 16, 2, 32);
        for cut in 7..book.len() - 1 {
            assert!(setup(&book[..cut]).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn comments_that_dont_fit_are_refused() {
        let mut packet = vec![3, b'v', b'o', b'r', b'b', b'i', b's'];
        packet.extend(0_u32.to_le_bytes()); // no vendor
        packet.extend(u32::MAX.to_le_bytes()); // billions of comments
        assert!(comments(&packet).is_err());
        let mut packet = vec![3, b'v', b'o', b'r', b'b', b'i', b's'];
        packet.extend(0_u32.to_le_bytes());
        packet.extend(1_u32.to_le_bytes());
        packet.extend(1_000_u32.to_le_bytes()); // a comment longer than the rest
        assert!(comments(&packet).is_err());
        let mut packet = vec![3, b'v', b'o', b'r', b'b', b'i', b's'];
        packet.extend(2_u32.to_le_bytes());
        packet.extend(b"me");
        packet.extend(1_u32.to_le_bytes());
        packet.extend(3_u32.to_le_bytes());
        packet.extend(b"a=b");
        assert!(comments(&packet).is_ok());
    }
}
