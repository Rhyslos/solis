//! A tileset's picture (Phase 9, D5): a PNG, a JPEG or an Aseprite file,
//! read whole (never scaled) so it can be cut into tiles. An Aseprite
//! file's frames come too, each with its time, for tiles that animate.
//! Read as a hostile file, within limits, with a plain-language reason
//! when refused.

use image::ImageReader;
use std::io::Cursor;

/// The widest and tallest picture read, in pixels.
pub const MAX_SIDE: u32 = 4096;
/// The most frames read from an Aseprite picture.
pub const MAX_FRAMES: u32 = 64;
/// The most bytes of pixels a picture may take (every frame together).
pub const MAX_PIXEL_BYTES: u64 = 256 << 20;
/// The biggest file read.
pub const MAX_FILE_BYTES: usize = 64 << 20;

/// A tileset's picture, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileArt {
    /// Its width and height, in pixels.
    pub size: [u32; 2],
    /// Its frames (one for a PNG): straight sRGB RGBA, rows from the top,
    /// width × height × 4 bytes each.
    pub frames: Vec<Vec<u8>>,
    /// How long each frame shows, in milliseconds.
    pub durations: Vec<u32>,
}

/// Reads a tileset's picture from `bytes`: an Aseprite file if
/// `aseprite`, otherwise a PNG or JPEG.
pub fn read_tile_art(bytes: &[u8], aseprite: bool) -> Result<TileArt, String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(format!(
            "the file is {} MB; pictures up to {} MB are read",
            bytes.len() >> 20,
            MAX_FILE_BYTES >> 20
        ));
    }
    if aseprite {
        let limits = crate::aseprite::AseLimits {
            max_file_bytes: MAX_FILE_BYTES,
            max_side: MAX_SIDE,
            max_frames: MAX_FRAMES,
            max_layers: 256,
            max_pixel_bytes: MAX_PIXEL_BYTES,
        };
        let ase = crate::aseprite::read_aseprite_within(bytes, limits).map_err(|e| e.0)?;
        if ase.frames.is_empty() {
            return Err("the Aseprite file has no frames".to_owned());
        }
        return Ok(TileArt {
            size: ase.size,
            durations: ase.frames.iter().map(|f| f.duration_ms.max(1)).collect(),
            frames: ase.frames.into_iter().map(|f| f.rgba).collect(),
        });
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("couldn't read the picture: {e}"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_PIXEL_BYTES);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| match e {
        image::ImageError::Limits(_) => {
            format!("the picture is bigger than {MAX_SIDE} pixels across")
        }
        image::ImageError::Unsupported(_) => {
            "the picture isn't PNG or JPEG, the formats Solis reads".to_owned()
        }
        other => format!("the picture is damaged: {other}"),
    })?;
    let rgba = decoded.into_rgba8();
    let size = [rgba.width(), rgba.height()];
    if size[0] == 0 || size[1] == 0 {
        return Err("the picture is empty".to_owned());
    }
    Ok(TileArt {
        size,
        frames: vec![rgba.into_raw()],
        durations: vec![100],
    })
}

/// Writes `rgba` (`size` pixels, straight RGBA) as a PNG: a template to
/// draw tiles over.
pub fn png_of(size: [u32; 2], rgba: &[u8]) -> Result<Vec<u8>, String> {
    let image = image::RgbaImage::from_raw(size[0], size[1], rgba.to_vec())
        .ok_or("the picture's size doesn't match its pixels")?;
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("couldn't write the picture: {e}"))?;
    Ok(out.into_inner())
}

#[cfg(test)]
#[path = "tile_art_tests.rs"]
mod tests;
