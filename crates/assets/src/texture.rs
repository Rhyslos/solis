//! Decoding texture images (PNG and JPEG) with limits, and working out
//! their mip levels.

use crate::model::{Limits, Sampler, Texture};
use image::imageops::FilterType;
use image::{ImageReader, RgbaImage};
use std::io::Cursor;

/// The most memory one image may take while decoding, in bytes.
const MAX_DECODE_BYTES: u64 = 1 << 30;

/// Decodes `bytes` into a texture with all its mip levels, scaling it down
/// first if it is wider or taller than `limits.max_texture_side`. Images
/// bigger than `limits.max_image_side`, or in formats other than PNG and
/// JPEG, are refused with a reason.
pub fn decode(
    bytes: &[u8],
    sampler: Sampler,
    srgb: bool,
    limits: &Limits,
    budget: u64,
) -> Result<Texture, String> {
    // The size it will take, worked out from the image's header before
    // any pixels are decoded: refused if it's past what's left of the
    // model's texture budget.
    let (width, height) = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("couldn't read the image: {e}"))?
        .into_dimensions()
        .map_err(|e| format!("the image is damaged: {e}"))?;
    let side = limits.max_texture_side.max(1);
    let scale = f64::from(side) / f64::from(width.max(height).max(side));
    let kept = (f64::from(width) * scale) * (f64::from(height) * scale);
    let needed = (kept * 4.0 * 4.0 / 3.0) as u64;
    if needed > budget {
        return Err(format!(
            "the model's textures would need more than {} MB",
            limits.max_texture_bytes / 1_000_000
        ));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("couldn't read the image: {e}"))?;
    let mut image_limits = image::Limits::default();
    image_limits.max_image_width = Some(limits.max_image_side);
    image_limits.max_image_height = Some(limits.max_image_side);
    image_limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(image_limits);
    let decoded = reader.decode().map_err(|e| match e {
        image::ImageError::Limits(_) => format!(
            "the image is bigger than {} pixels across",
            limits.max_image_side
        ),
        image::ImageError::Unsupported(_) => {
            "the image isn't PNG or JPEG, the formats Solis reads".to_string()
        }
        other => format!("the image is damaged: {other}"),
    })?;
    let mut rgba = decoded.into_rgba8();
    if rgba.width() > side || rgba.height() > side {
        let scale = f64::from(side) / f64::from(rgba.width().max(rgba.height()));
        let width = ((f64::from(rgba.width()) * scale).round() as u32).clamp(1, side);
        let height = ((f64::from(rgba.height()) * scale).round() as u32).clamp(1, side);
        rgba = image::imageops::resize(&rgba, width, height, FilterType::Triangle);
    }
    Ok(with_mips(rgba, sampler, srgb))
}

/// A texture of one colour, one pixel in size (used where a material has
/// no texture, so every material is drawn the same way).
pub fn solid(rgba: [u8; 4], srgb: bool) -> Texture {
    Texture {
        width: 1,
        height: 1,
        levels: vec![rgba.to_vec()],
        sampler: Sampler::default(),
        srgb,
    }
}

/// `image` with every smaller level added, each half the size of the last.
pub fn with_mips(image: RgbaImage, sampler: Sampler, srgb: bool) -> Texture {
    let (width, height) = image.dimensions();
    let mut levels = Vec::new();
    let mut level = image;
    loop {
        let (w, h) = level.dimensions();
        let next = (w > 1 || h > 1).then(|| {
            image::imageops::resize(&level, (w / 2).max(1), (h / 2).max(1), FilterType::Triangle)
        });
        levels.push(level.into_raw());
        match next {
            Some(next) => level = next,
            None => break,
        }
    }
    Texture {
        width,
        height,
        levels,
        sampler,
        srgb,
    }
}

/// How many mip levels a texture of this size has.
pub fn level_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([(x * 7) as u8, (y * 5) as u8, 200, 255])
        });
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn a_png_decodes_with_every_mip_level() {
        let t = decode(
            &png(16, 4),
            Sampler::default(),
            true,
            &Limits::default(),
            u64::MAX,
        )
        .unwrap();
        assert_eq!((t.width, t.height), (16, 4));
        assert_eq!(t.levels.len() as u32, level_count(16, 4));
        let sizes: Vec<usize> = t.levels.iter().map(Vec::len).collect();
        assert_eq!(sizes, [16 * 4 * 4, 8 * 2 * 4, 4 * 4, 2 * 4, 4]);
    }

    #[test]
    fn level_counts_follow_the_longer_side() {
        assert_eq!(level_count(1, 1), 1);
        assert_eq!(level_count(256, 256), 9);
        assert_eq!(level_count(300, 2), 9);
        assert_eq!(level_count(0, 0), 1);
    }

    #[test]
    fn big_textures_are_scaled_down_to_the_limit() {
        let limits = Limits {
            max_texture_side: 8,
            ..Limits::default()
        };
        let t = decode(&png(32, 16), Sampler::default(), true, &limits, u64::MAX).unwrap();
        assert_eq!((t.width, t.height), (8, 4));
    }

    #[test]
    fn images_beyond_the_decode_limit_are_refused() {
        let limits = Limits {
            max_image_side: 8,
            ..Limits::default()
        };
        let err = decode(&png(32, 16), Sampler::default(), true, &limits, u64::MAX).unwrap_err();
        assert!(err.contains("bigger than 8"), "{err}");
    }

    #[test]
    fn other_formats_and_rubbish_are_refused_politely() {
        assert!(
            decode(
                b"GIF89a....",
                Sampler::default(),
                true,
                &Limits::default(),
                u64::MAX
            )
            .is_err()
        );
        assert!(decode(b"", Sampler::default(), true, &Limits::default(), u64::MAX).is_err());
        assert!(
            decode(
                &[0xff; 64],
                Sampler::default(),
                true,
                &Limits::default(),
                u64::MAX
            )
            .is_err()
        );
    }

    /// A PNG whose header claims an enormous size: refused before any
    /// pixels are allocated.
    #[test]
    fn a_png_lying_about_its_size_is_refused() {
        let mut bytes = png(4, 4);
        // Width and height sit at bytes 16..24 of a PNG (in the IHDR chunk).
        bytes[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(
            decode(
                &bytes,
                Sampler::default(),
                true,
                &Limits::default(),
                u64::MAX
            )
            .is_err()
        );
    }

    /// Thousands of randomly damaged images: always an answer, never a
    /// crash.
    #[test]
    fn damaged_images_never_crash_the_decoder() {
        let good = png(8, 8);
        let mut seed: u64 = 3;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n.max(1)
        };
        for _ in 0..2_000 {
            let mut bytes = good.clone();
            match next(3) {
                0 => bytes.truncate(next(bytes.len())),
                1 => {
                    for _ in 0..1 + next(8) {
                        let at = next(bytes.len());
                        bytes[at] = next(256) as u8;
                    }
                }
                _ => {
                    let at = next(bytes.len());
                    bytes.insert(at, next(256) as u8);
                }
            }
            let _ = decode(
                &bytes,
                Sampler::default(),
                true,
                &Limits::default(),
                u64::MAX,
            );
        }
    }

    #[test]
    fn textures_past_the_budget_are_refused_before_decoding() {
        let bytes = png(64, 64);
        assert!(decode(&bytes, Sampler::default(), true, &Limits::default(), 1_000).is_err());
        let t = decode(
            &bytes,
            Sampler::default(),
            true,
            &Limits::default(),
            100_000,
        )
        .unwrap();
        assert!(t.byte_size() <= 64 * 64 * 4 * 4 / 3 + 64);
    }

    #[test]
    fn a_solid_texture_is_one_pixel() {
        let t = solid([255, 0, 0, 255], true);
        assert_eq!(t.levels, vec![vec![255, 0, 0, 255]]);
    }
}
