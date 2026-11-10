//! Which picture formats Solis reads: PNG and JPEG, told by a file's first
//! bytes. Every picture reader opens its bytes here, so the answer is the
//! same for a model's textures, sprites, tilesets and thumbnails.

use image::{ImageFormat, ImageReader};
use std::io::Cursor;

/// Opens `bytes` for decoding if they start as a PNG or a JPEG, whatever
/// the file was named; anything else is refused before a decoder runs.
/// `what` is the word for it in the reason ("image", "picture").
///
/// The `image` library decodes every format its build has switched on,
/// and any other library in the program can switch more on for the whole
/// build (the Windows build carries a BMP decoder that way). Checking
/// here keeps what Solis reads the same on every build, and keeps files
/// away from decoders that nothing tests or fuzzes.
pub(crate) fn open<'a>(
    bytes: &'a [u8],
    what: &str,
) -> Result<ImageReader<Cursor<&'a [u8]>>, String> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("couldn't read the {what}: {e}"))?;
    match reader.format() {
        Some(ImageFormat::Png | ImageFormat::Jpeg) => Ok(reader),
        _ => Err(format!(
            "the {what} isn't PNG or JPEG, the formats Solis reads"
        )),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A whole, valid BMP of one red pixel: a build that carries a BMP
    /// decoder would decode it.
    pub(crate) fn bmp() -> Vec<u8> {
        let mut bytes = b"BM".to_vec();
        bytes.extend_from_slice(&58u32.to_le_bytes()); // the file's length
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&54u32.to_le_bytes()); // where the pixels start
        bytes.extend_from_slice(&40u32.to_le_bytes()); // the header's length
        bytes.extend_from_slice(&1i32.to_le_bytes()); // width
        bytes.extend_from_slice(&1i32.to_le_bytes()); // height
        bytes.extend_from_slice(&1u16.to_le_bytes()); // planes
        bytes.extend_from_slice(&24u16.to_le_bytes()); // bits a pixel
        bytes.extend_from_slice(&0u32.to_le_bytes()); // not compressed
        bytes.extend_from_slice(&4u32.to_le_bytes()); // bytes of pixels
        bytes.extend_from_slice(&[0; 16]); // print size and palette: unused
        bytes.extend_from_slice(&[0, 0, 255, 0]); // blue, green, red, padding
        bytes
    }

    /// A JPEG of `width` × `height` grey pixels.
    pub(crate) fn jpeg(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([128, 128, 128]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
            .unwrap();
        bytes
    }

    /// How files in the other formats the `image` library knows begin.
    pub(crate) fn other_formats() -> Vec<(&'static str, Vec<u8>)> {
        let mut webp = b"RIFF\x1a\0\0\0WEBPVP8 ".to_vec();
        webp.resize(64, 0);
        let mut avif = b"\0\0\0\x20ftypavif\0\0\0\0avifmif1".to_vec();
        avif.resize(64, 0);
        vec![
            ("BMP", bmp()),
            ("GIF", b"GIF89a\x01\0\x01\0\0\0\0;".to_vec()),
            ("GIF, older", b"GIF87a\x01\0\x01\0\0\0\0;".to_vec()),
            ("WebP", webp),
            ("AVIF", avif),
            ("TIFF", b"II*\0\x08\0\0\0\0\0\0\0\0\0".to_vec()),
            (
                "TIFF, big end first",
                b"MM\0*\0\0\0\x08\0\0\0\0\0\0".to_vec(),
            ),
            ("icon", b"\0\0\x01\0\x01\0\x01\x01\0\0\x01\0\x18\0".to_vec()),
            ("DDS", b"DDS \x7c\0\0\0\0\0\0\0\0\0\0\0".to_vec()),
            (
                "HDR",
                b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 1\n".to_vec(),
            ),
            ("OpenEXR", b"\x76\x2f\x31\x01\x02\0\0\0\0\0\0\0".to_vec()),
            ("PNM", b"P6\n1 1\n255\n\xff\0\0".to_vec()),
            ("QOI", b"qoif\0\0\0\x01\0\0\0\x01\x04\0".to_vec()),
            ("farbfeld", b"farbfeld\0\0\0\x01\0\0\0\x01".to_vec()),
            ("PCX", b"\x0a\x05\x01\x08\0\0\0\0\0\0\0\0".to_vec()),
        ]
    }

    #[test]
    fn a_png_and_a_jpeg_are_opened() {
        let png = crate::sample::checker_png(4, 2);
        assert_eq!(
            open(&png, "image").unwrap().format(),
            Some(ImageFormat::Png)
        );
        let jpeg = jpeg(4, 4);
        assert_eq!(
            open(&jpeg, "image").unwrap().format(),
            Some(ImageFormat::Jpeg)
        );
    }

    /// The regression test for the Windows build decoding BMPs: every
    /// other format is refused by its first bytes, with the same reason,
    /// whether or not this build could decode it.
    #[test]
    fn every_other_format_is_refused_by_its_first_bytes() {
        for (name, bytes) in other_formats() {
            let why = open(&bytes, "picture").err();
            assert_eq!(
                why.as_deref(),
                Some("the picture isn't PNG or JPEG, the formats Solis reads"),
                "{name}"
            );
        }
        for rubbish in [&b""[..], b"not a picture", &[0xff; 64]] {
            assert!(open(rubbish, "image").is_err());
        }
    }

    /// Thousands of made-up beginnings: only one that starts as a PNG or a
    /// JPEG is ever opened.
    #[test]
    fn only_png_and_jpeg_beginnings_ever_open() {
        let mut starts: Vec<Vec<u8>> = other_formats().into_iter().map(|(_, b)| b).collect();
        starts.push(crate::sample::checker_png(4, 2));
        starts.push(jpeg(4, 4));
        let mut seed: u64 = 14;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n.max(1)
        };
        for _ in 0..4_000 {
            let mut bytes = starts[next(starts.len())].clone();
            bytes.truncate(1 + next(bytes.len()));
            for _ in 0..next(3) {
                let at = next(bytes.len().min(16));
                bytes[at] = next(256) as u8;
            }
            if open(&bytes, "picture").is_ok() {
                let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
                let jpeg = bytes.starts_with(&[0xff, 0xd8, 0xff]);
                assert!(png || jpeg, "{:?}", &bytes[..bytes.len().min(16)]);
            }
        }
    }
}
