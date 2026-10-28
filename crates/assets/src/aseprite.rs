//! Reading Aseprite files (`.aseprite`, `.ase`; Phase 9, D2), from
//! [Aseprite's own description of the format](https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md).
//!
//! What's kept:
//! - every frame, flattened: the visible layers drawn over each other in
//!   order (normal blending; other blend modes are drawn as normal, with a
//!   note), each with its duration;
//! - **tags** (a run of frames, its direction and how many times it
//!   repeats), which become clips;
//! - **slices** (named rectangles, with a 9-slice centre and a pivot),
//!   which become pivots, hitboxes and attach points;
//! - **user data** text: on a cel it's an event on that frame (a
//!   footstep), on a tag, slice or layer it's kept with it;
//! - the layers' names and the palette.
//!
//! Tilemap layers and tilesets (D5), and properties maps in user data, are
//! skipped. The file is hostile: every size is checked against
//! [`AseLimits`] before anything is allocated, compressed pictures are
//! never inflated past the size they claim, and a damaged file is refused
//! with a plain-language [`AseError`], never a crash.

use std::io::Read;

/// The most a file may hold, so a hostile one can't use up the memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AseLimits {
    /// The biggest file read, in bytes.
    pub max_file_bytes: usize,
    /// The widest and tallest canvas, in pixels.
    pub max_side: u32,
    /// The most frames.
    pub max_frames: u32,
    /// The most layers.
    pub max_layers: usize,
    /// The most bytes of flattened frames (width × height × 4 × frames).
    pub max_pixel_bytes: u64,
}

impl Default for AseLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 << 20,
            max_side: 4096,
            max_frames: 2048,
            max_layers: 256,
            max_pixel_bytes: 256 << 20,
        }
    }
}

/// Why an Aseprite file couldn't be read, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AseError(pub String);

impl std::fmt::Display for AseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AseError {}

fn fail<T>(why: impl Into<String>) -> Result<T, AseError> {
    Err(AseError(why.into()))
}

/// The order a tag plays its frames in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// First to last.
    #[default]
    Forward,
    /// Last to first.
    Reverse,
    /// First to last and back.
    PingPong,
    /// Last to first and back.
    PingPongReverse,
}

impl Direction {
    /// Its name, as Properties shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Reverse => "reverse",
            Self::PingPong => "ping-pong",
            Self::PingPongReverse => "ping-pong, reversed",
        }
    }
}

/// A tag: a run of frames with a name, which becomes a clip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AseTag {
    /// Its name.
    pub name: String,
    /// Its first and last frame (from 0).
    pub from: u32,
    /// Its last frame.
    pub to: u32,
    /// Which way it plays.
    pub direction: Direction,
    /// How many times it plays: 0 for over and over.
    pub repeats: u32,
    /// Its user data text, if any.
    pub text: Option<String>,
}

/// One frame's place of a slice: from that frame on, until the next key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceKey {
    /// The first frame it holds for (from 0).
    pub frame: u32,
    /// Its left, top, width and height, in the canvas's pixels.
    pub rect: [i32; 4],
    /// A 9-slice's centre, from the slice's corner: left, top, width and
    /// height.
    pub center: Option<[i32; 4]>,
    /// Its pivot, from the slice's corner.
    pub pivot: Option<[i32; 2]>,
}

/// A slice: a named rectangle (a pivot at the feet, where a held tool
/// goes, a hitbox).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AseSlice {
    /// Its name.
    pub name: String,
    /// Where it is, from frame to frame.
    pub keys: Vec<SliceKey>,
    /// Its user data text, if any.
    pub text: Option<String>,
}

impl AseSlice {
    /// Its key for `frame`: the last one starting at or before it.
    pub fn key_at(&self, frame: u32) -> Option<&SliceKey> {
        self.keys
            .iter()
            .filter(|k| k.frame <= frame)
            .max_by_key(|k| k.frame)
            .or_else(|| self.keys.first())
    }
}

/// A layer, as the file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AseLayer {
    /// Its name.
    pub name: String,
    /// Whether it's drawn (it and every group it's in are visible, and it
    /// isn't a reference layer).
    pub shown: bool,
    /// Whether it's a group of other layers.
    pub group: bool,
    /// Its user data text, if any.
    pub text: Option<String>,
}

/// One frame: its picture (the visible layers drawn together, straight
/// RGBA, rows top to bottom) and how long it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AseFrame {
    /// How long it shows, in milliseconds.
    pub duration_ms: u32,
    /// Width × height × 4 bytes.
    pub rgba: Vec<u8>,
    /// User data text on its cels: events that happen on this frame.
    pub events: Vec<String>,
}

/// An Aseprite file, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aseprite {
    /// The canvas's width and height, in pixels.
    pub size: [u32; 2],
    /// Its frames.
    pub frames: Vec<AseFrame>,
    /// Its layers, bottom first.
    pub layers: Vec<AseLayer>,
    /// Its tags.
    pub tags: Vec<AseTag>,
    /// Its slices.
    pub slices: Vec<AseSlice>,
    /// Its palette, RGBA.
    pub palette: Vec<[u8; 4]>,
    /// What was drawn differently from Aseprite or skipped, in plain words.
    pub notes: Vec<String>,
}

/// Whether `path` names an Aseprite file (by its extension).
pub fn is_aseprite_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("aseprite") || e.eq_ignore_ascii_case("ase"))
}

/// Little-endian reading from a slice, failing (never panicking) past its
/// end.
struct Bytes<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], AseError> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len());
        match end {
            Some(end) => {
                let s = &self.data[self.at..end];
                self.at = end;
                Ok(s)
            }
            None => fail("the file ends part-way through"),
        }
    }

    fn skip(&mut self, n: usize) -> Result<(), AseError> {
        self.take(n).map(drop)
    }

    fn u8(&mut self) -> Result<u8, AseError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, AseError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn i16(&mut self) -> Result<i16, AseError> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, AseError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32, AseError> {
        Ok(self.u32()? as i32)
    }

    fn string(&mut self) -> Result<String, AseError> {
        let n = usize::from(self.u16()?);
        let b = self.take(n)?;
        Ok(String::from_utf8_lossy(b).into_owned())
    }

    fn left(&self) -> usize {
        self.data.len() - self.at
    }
}

/// How a cel's pixels are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CelPixels {
    /// Its own picture: width, height and the file's pixels.
    Image { w: u32, h: u32, pixels: Vec<u8> },
    /// The same as this layer's cel in another frame.
    Linked(u32),
}

/// A cel: one layer's picture in one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cel {
    layer: usize,
    x: i32,
    y: i32,
    opacity: u8,
    z: i16,
    pixels: CelPixels,
}

/// A layer as the file has it, before visibility is worked out.
#[derive(Debug, Clone)]
struct RawLayer {
    name: String,
    visible: bool,
    reference: bool,
    background: bool,
    group: bool,
    tilemap: bool,
    level: u16,
    opacity: u8,
    text: Option<String>,
}

/// What a user data chunk belongs to: the last thing read.
enum Owner {
    Nothing,
    Layer(usize),
    Cel(usize),
    Slice(usize),
    /// The tags just read, the next of which gets the next user data.
    Tags(usize),
}

/// Reads an Aseprite file within the standard [`AseLimits`].
pub fn read_aseprite(bytes: &[u8]) -> Result<Aseprite, AseError> {
    read_aseprite_within(bytes, AseLimits::default())
}

/// Reads an Aseprite file within `limits`.
pub fn read_aseprite_within(bytes: &[u8], limits: AseLimits) -> Result<Aseprite, AseError> {
    if bytes.len() > limits.max_file_bytes {
        return fail(format!(
            "it's {} MB; Aseprite files up to {} MB are read",
            bytes.len() >> 20,
            limits.max_file_bytes >> 20
        ));
    }
    let mut b = Bytes::new(bytes);
    if bytes.len() < 128 {
        return fail("it's too short to be an Aseprite file");
    }
    let _file_size = b.u32()?;
    if b.u16()? != 0xA5E0 {
        return fail("it isn't an Aseprite file (its first bytes are wrong)");
    }
    let frame_count = u32::from(b.u16()?);
    let width = u32::from(b.u16()?);
    let height = u32::from(b.u16()?);
    let depth = b.u16()?;
    let flags = b.u32()?;
    let _speed = b.u16()?;
    b.skip(8)?;
    let transparent_index = b.u8()?;
    b.skip(3)?;
    let _colours = b.u16()?;
    b.skip(84 + 2 + 4 + 4)?;
    if width == 0 || height == 0 {
        return fail("its canvas has no size");
    }
    if width > limits.max_side || height > limits.max_side {
        return fail(format!(
            "its canvas is {width} × {height}; up to {} pixels each way are read",
            limits.max_side
        ));
    }
    if frame_count == 0 {
        return fail("it has no frames");
    }
    if frame_count > limits.max_frames {
        return fail(format!(
            "it has {frame_count} frames; up to {} are read",
            limits.max_frames
        ));
    }
    let bpp = match depth {
        32 => 4,
        16 => 2,
        8 => 1,
        other => {
            return fail(format!(
                "its colour depth ({other} bits) isn't one Aseprite writes"
            ));
        }
    };
    let pixel_bytes = u64::from(width) * u64::from(height) * 4 * u64::from(frame_count);
    if pixel_bytes > limits.max_pixel_bytes {
        return fail(format!(
            "its frames would take {} MB; up to {} MB are read",
            pixel_bytes >> 20,
            limits.max_pixel_bytes >> 20
        ));
    }
    let layer_opacity = flags & 1 != 0;
    let mut layers: Vec<RawLayer> = Vec::new();
    let mut cels: Vec<Vec<Cel>> = Vec::with_capacity(frame_count as usize);
    let mut durations = Vec::with_capacity(frame_count as usize);
    let mut cel_texts: Vec<Vec<Option<String>>> = Vec::with_capacity(frame_count as usize);
    let mut tags: Vec<AseTag> = Vec::new();
    let mut slices: Vec<AseSlice> = Vec::new();
    let mut palette: Vec<[u8; 4]> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for frame in 0..frame_count {
        let start = b.at;
        let frame_bytes = b.u32()? as usize;
        if b.u16()? != 0xF1FA {
            return fail(format!(
                "frame {} is damaged (its mark is wrong)",
                frame + 1
            ));
        }
        let old_chunks = b.u16()?;
        let duration = b.u16()?;
        b.skip(2)?;
        let new_chunks = b.u32()?;
        let chunks = if new_chunks == 0 {
            u32::from(old_chunks)
        } else {
            new_chunks
        };
        if frame_bytes < 16
            || start
                .checked_add(frame_bytes)
                .is_none_or(|e| e > bytes.len())
        {
            return fail(format!(
                "frame {} says it's longer than the file",
                frame + 1
            ));
        }
        let end = start + frame_bytes;
        let mut frame_cels: Vec<Cel> = Vec::new();
        let mut texts: Vec<Option<String>> = Vec::new();
        let mut owner = Owner::Nothing;
        let mut chunk = 0;
        while chunk < chunks && b.at < end {
            chunk += 1;
            let chunk_start = b.at;
            let size = b.u32()? as usize;
            let kind = b.u16()?;
            if size < 6 || chunk_start.checked_add(size).is_none_or(|e| e > end) {
                return fail(format!(
                    "frame {} has a damaged part (it says it's longer than the frame)",
                    frame + 1
                ));
            }
            let mut c = Bytes::new(&bytes[chunk_start + 6..chunk_start + size]);
            b.at = chunk_start + size;
            match kind {
                0x2004 => {
                    if layers.len() == limits.max_layers {
                        return fail(format!("it has more than {} layers", limits.max_layers));
                    }
                    let flags = c.u16()?;
                    let kind = c.u16()?;
                    let level = c.u16()?;
                    c.skip(4)?;
                    let blend = c.u16()?;
                    let opacity = c.u8()?;
                    c.skip(3)?;
                    let name = c.string()?;
                    if blend != 0 && !notes.iter().any(|n| n.contains("blend")) {
                        notes.push(format!(
                            "layer \"{name}\" uses a blend mode other than Normal, which is drawn as Normal"
                        ));
                    }
                    layers.push(RawLayer {
                        name,
                        visible: flags & 1 != 0,
                        background: flags & 8 != 0,
                        reference: flags & 64 != 0,
                        group: kind == 1,
                        tilemap: kind == 2,
                        level,
                        opacity: if layer_opacity { opacity } else { 255 },
                        text: None,
                    });
                    owner = Owner::Layer(layers.len() - 1);
                }
                0x2005 => {
                    let layer = usize::from(c.u16()?);
                    let x = i32::from(c.i16()?);
                    let y = i32::from(c.i16()?);
                    let opacity = c.u8()?;
                    let cel_type = c.u16()?;
                    let z = c.i16()?;
                    c.skip(5)?;
                    let pixels = match cel_type {
                        0 | 2 => {
                            let w = u32::from(c.u16()?);
                            let h = u32::from(c.u16()?);
                            if w > limits.max_side * 2 || h > limits.max_side * 2 {
                                return fail(format!(
                                    "frame {} has a picture of {w} × {h}, bigger than allowed",
                                    frame + 1
                                ));
                            }
                            let want = (w as usize) * (h as usize) * bpp;
                            let rest = &c.data[c.at..];
                            let data = if cel_type == 0 {
                                if rest.len() < want {
                                    return fail(format!(
                                        "frame {} has a picture cut short",
                                        frame + 1
                                    ));
                                }
                                rest[..want].to_vec()
                            } else {
                                inflate(rest, want).map_err(|why| {
                                    AseError(format!("frame {}'s picture {why}", frame + 1))
                                })?
                            };
                            CelPixels::Image { w, h, pixels: data }
                        }
                        1 => CelPixels::Linked(u32::from(c.u16()?)),
                        3 => {
                            // Tilemaps arrive with tilesets (D5).
                            if !notes.iter().any(|n| n.contains("tilemap")) {
                                notes.push(
                                    "its tilemap layers are skipped (tilesets arrive later)".into(),
                                );
                            }
                            owner = Owner::Nothing;
                            continue;
                        }
                        other => {
                            return fail(format!(
                                "frame {} has a picture of a kind Aseprite doesn't write ({other})",
                                frame + 1
                            ));
                        }
                    };
                    frame_cels.push(Cel {
                        layer,
                        x,
                        y,
                        opacity,
                        z,
                        pixels,
                    });
                    texts.push(None);
                    owner = Owner::Cel(frame_cels.len() - 1);
                }
                0x2018 => {
                    let n = usize::from(c.u16()?);
                    c.skip(8)?;
                    let first = tags.len();
                    for _ in 0..n {
                        let from = u32::from(c.u16()?);
                        let to = u32::from(c.u16()?);
                        let direction = match c.u8()? {
                            1 => Direction::Reverse,
                            2 => Direction::PingPong,
                            3 => Direction::PingPongReverse,
                            _ => Direction::Forward,
                        };
                        let repeats = u32::from(c.u16()?);
                        c.skip(6 + 3 + 1)?;
                        let name = c.string()?;
                        tags.push(AseTag {
                            name,
                            from,
                            to,
                            direction,
                            repeats,
                            text: None,
                        });
                    }
                    owner = Owner::Tags(first);
                }
                0x2019 => {
                    let size = c.u32()? as usize;
                    let first = c.u32()? as usize;
                    let last = c.u32()? as usize;
                    c.skip(8)?;
                    if size > 65_536 || last < first || last >= size.max(1) {
                        return fail("its palette is damaged");
                    }
                    palette.resize(size, [0, 0, 0, 255]);
                    for entry in palette.iter_mut().take(last + 1).skip(first) {
                        let entry_flags = c.u16()?;
                        let rgba = c.take(4)?;
                        *entry = [rgba[0], rgba[1], rgba[2], rgba[3]];
                        if entry_flags & 1 != 0 {
                            let _name = c.string()?;
                        }
                    }
                }
                0x0004 | 0x0011 if palette.is_empty() => {
                    // The old palette, from files made before 1.2.
                    let packets = c.u16()?;
                    let scale = |v: u8| {
                        if kind == 0x0011 {
                            v.saturating_mul(4)
                        } else {
                            v
                        }
                    };
                    let mut at = 0usize;
                    for _ in 0..packets {
                        at += usize::from(c.u8()?);
                        let count = match c.u8()? {
                            0 => 256,
                            n => usize::from(n),
                        };
                        for _ in 0..count {
                            let rgb = c.take(3)?;
                            if at < 256 {
                                if palette.len() <= at {
                                    palette.resize(at + 1, [0, 0, 0, 255]);
                                }
                                palette[at] = [scale(rgb[0]), scale(rgb[1]), scale(rgb[2]), 255];
                            }
                            at += 1;
                        }
                    }
                }
                0x2020 => {
                    let flags = c.u32()?;
                    let text = if flags & 1 != 0 {
                        Some(c.string()?)
                    } else {
                        None
                    };
                    let text = text.filter(|t| !t.is_empty());
                    match owner {
                        Owner::Layer(i) => layers[i].text = text,
                        Owner::Cel(i) => texts[i] = text,
                        Owner::Slice(i) => slices[i].text = text,
                        Owner::Tags(next) => {
                            if let Some(tag) = tags.get_mut(next) {
                                tag.text = text;
                                owner = Owner::Tags(next + 1);
                            }
                            continue;
                        }
                        Owner::Nothing => {}
                    }
                    owner = Owner::Nothing;
                }
                0x2022 => {
                    let n = c.u32()?;
                    let slice_flags = c.u32()?;
                    c.skip(4)?;
                    let name = c.string()?;
                    // Each key is at least 20 bytes.
                    if n as usize > c.left() / 20 {
                        return fail(format!("slice \"{name}\" is damaged"));
                    }
                    let mut keys = Vec::with_capacity(n as usize);
                    for _ in 0..n {
                        let frame = c.u32()?;
                        let rect = [c.i32()?, c.i32()?, c.u32()? as i32, c.u32()? as i32];
                        let center = if slice_flags & 1 != 0 {
                            Some([c.i32()?, c.i32()?, c.u32()? as i32, c.u32()? as i32])
                        } else {
                            None
                        };
                        let pivot = if slice_flags & 2 != 0 {
                            Some([c.i32()?, c.i32()?])
                        } else {
                            None
                        };
                        keys.push(SliceKey {
                            frame,
                            rect,
                            center,
                            pivot,
                        });
                    }
                    if slices.len() < 1024 {
                        slices.push(AseSlice {
                            name,
                            keys,
                            text: None,
                        });
                        owner = Owner::Slice(slices.len() - 1);
                    }
                }
                _ => {
                    // Colour profile, cel extras, external files, masks,
                    // paths and tilesets: nothing Solis uses yet.
                    owner = Owner::Nothing;
                }
            }
        }
        b.at = end;
        durations.push(u32::from(duration));
        cels.push(frame_cels);
        cel_texts.push(texts);
    }
    for tag in &mut tags {
        if tag.from > tag.to || tag.to >= frame_count {
            notes.push(format!(
                "tag \"{}\" runs past the last frame, so it ends there",
                tag.name
            ));
            tag.to = tag.to.min(frame_count - 1);
            tag.from = tag.from.min(tag.to);
        }
    }
    let shown = shown_layers(&layers);
    let frames = flatten(
        [width, height],
        &layers,
        &shown,
        &cels,
        &cel_texts,
        &durations,
        bpp,
        &palette,
        transparent_index,
    )?;
    Ok(Aseprite {
        size: [width, height],
        frames,
        layers: layers
            .iter()
            .zip(&shown)
            .map(|(l, &shown)| AseLayer {
                name: l.name.clone(),
                shown,
                group: l.group,
                text: l.text.clone(),
            })
            .collect(),
        tags,
        slices,
        palette,
        notes,
    })
}

/// Inflates zlib `data` into exactly `want` bytes, never more.
fn inflate(data: &[u8], want: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(want);
    flate2::read::ZlibDecoder::new(data)
        .take(want as u64)
        .read_to_end(&mut out)
        .map_err(|e| format!("can't be unpacked ({e})"))?;
    if out.len() < want {
        return Err("is cut short".into());
    }
    Ok(out)
}

/// Whether each layer is drawn: it's visible, every group it's in is,
/// it isn't a reference layer, and it holds pictures (not a group or a
/// tilemap).
fn shown_layers(layers: &[RawLayer]) -> Vec<bool> {
    let mut shown = vec![false; layers.len()];
    // The visibility of the group at each depth, as the list is walked.
    let mut groups: Vec<bool> = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let level = usize::from(layer.level);
        groups.truncate(level);
        let parents = groups.iter().all(|&v| v) && groups.len() == level;
        let visible = layer.visible && !layer.reference && parents;
        if layer.group {
            groups.push(visible);
        }
        shown[i] = visible && !layer.group && !layer.tilemap;
    }
    shown
}

/// Every frame's picture: its shown layers' cels drawn over each other.
#[allow(clippy::too_many_arguments)]
fn flatten(
    [width, height]: [u32; 2],
    layers: &[RawLayer],
    shown: &[bool],
    cels: &[Vec<Cel>],
    cel_texts: &[Vec<Option<String>>],
    durations: &[u32],
    bpp: usize,
    palette: &[[u8; 4]],
    transparent: u8,
) -> Result<Vec<AseFrame>, AseError> {
    let mut frames = Vec::with_capacity(cels.len());
    for (f, frame_cels) in cels.iter().enumerate() {
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        // Drawing order: the layer's place plus the cel's z-index; when
        // equal, the bigger z-index on top (Aseprite's own rule).
        let mut order: Vec<&Cel> = frame_cels
            .iter()
            .filter(|c| shown.get(c.layer).copied().unwrap_or(false))
            .collect();
        order.sort_by_key(|c| (c.layer as i64 + i64::from(c.z), c.z));
        for cel in order {
            let Some(layer) = layers.get(cel.layer) else {
                continue;
            };
            // A linked cel draws another frame's picture of the same layer.
            let source = match &cel.pixels {
                CelPixels::Image { .. } => Some(cel),
                CelPixels::Linked(other) => cels
                    .get(*other as usize)
                    .and_then(|c| c.iter().find(|o| o.layer == cel.layer))
                    .filter(|o| matches!(o.pixels, CelPixels::Image { .. })),
            };
            let Some(CelPixels::Image { w, h, pixels }) = source.map(|s| &s.pixels) else {
                continue;
            };
            let opacity = u32::from(cel.opacity) * u32::from(layer.opacity) / 255;
            draw_cel(
                &mut rgba,
                [width, height],
                (cel.x, cel.y),
                (*w, *h, pixels),
                opacity,
                bpp,
                palette,
                (transparent, layer.background),
            );
        }
        let events = cel_texts
            .get(f)
            .map(|texts| texts.iter().flatten().cloned().collect())
            .unwrap_or_default();
        frames.push(AseFrame {
            duration_ms: durations.get(f).copied().unwrap_or(100),
            rgba,
            events,
        });
    }
    Ok(frames)
}

/// Draws one cel's pixels over `canvas` at `(x, y)`, normal blending.
#[allow(clippy::too_many_arguments)]
fn draw_cel(
    canvas: &mut [u8],
    [width, height]: [u32; 2],
    (x, y): (i32, i32),
    (w, h, pixels): (u32, u32, &[u8]),
    opacity: u32,
    bpp: usize,
    palette: &[[u8; 4]],
    (transparent, background): (u8, bool),
) {
    if opacity == 0 {
        return;
    }
    for row in 0..h as i32 {
        let cy = y + row;
        if cy < 0 || cy >= height as i32 {
            continue;
        }
        for col in 0..w as i32 {
            let cx = x + col;
            if cx < 0 || cx >= width as i32 {
                continue;
            }
            let at = (row as usize * w as usize + col as usize) * bpp;
            let Some(p) = pixels.get(at..at + bpp) else {
                continue;
            };
            let src = match bpp {
                4 => [p[0], p[1], p[2], p[3]],
                2 => [p[0], p[0], p[0], p[1]],
                _ => {
                    if p[0] == transparent && !background {
                        [0, 0, 0, 0]
                    } else {
                        palette
                            .get(usize::from(p[0]))
                            .copied()
                            .unwrap_or([0, 0, 0, 255])
                    }
                }
            };
            let sa = u32::from(src[3]) * opacity / 255;
            if sa == 0 {
                continue;
            }
            let d = (cy as usize * width as usize + cx as usize) * 4;
            let dst = &mut canvas[d..d + 4];
            let da = u32::from(dst[3]);
            // Straight alpha, source over.
            let oa = sa + da * (255 - sa) / 255;
            if oa == 0 {
                continue;
            }
            for i in 0..3 {
                let c = (u32::from(src[i]) * sa + u32::from(dst[i]) * da * (255 - sa) / 255) / oa;
                dst[i] = c.min(255) as u8;
            }
            dst[3] = oa.min(255) as u8;
        }
    }
}

#[cfg(test)]
#[path = "aseprite_tests.rs"]
pub(crate) mod tests;
