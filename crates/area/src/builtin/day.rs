//! Day and night (decision 0092): the colours of a day as time→colour
//! stops, written as text in an area (`6:00 #e99a6a, 9:00 #fff6e0`), the
//! tint at any hour, and whether it's night.

/// The most colour stops a day has.
pub const MAX_STOPS: usize = 8;

/// A day's colours: stops at hours, the tint running smoothly between
/// them and round midnight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayColours {
    stops: [(f32, [f32; 3]); MAX_STOPS],
    count: usize,
}

impl Default for DayColours {
    /// Dawn, day, dusk and night, the night held from 21:00 to 4:30.
    fn default() -> Self {
        let night = [
            0x2a as f32 / 255.0,
            0x2f as f32 / 255.0,
            0x5a as f32 / 255.0,
        ];
        Self::from_stops(&[
            (4.5, night),
            (
                6.0,
                [
                    0xe9 as f32 / 255.0,
                    0x9a as f32 / 255.0,
                    0x6a as f32 / 255.0,
                ],
            ),
            (
                9.0,
                [
                    0xff as f32 / 255.0,
                    0xf6 as f32 / 255.0,
                    0xe0 as f32 / 255.0,
                ],
            ),
            (
                18.0,
                [
                    0xf0 as f32 / 255.0,
                    0x8a as f32 / 255.0,
                    0x50 as f32 / 255.0,
                ],
            ),
            (
                21.0,
                [
                    0x2a as f32 / 255.0,
                    0x2f as f32 / 255.0,
                    0x5a as f32 / 255.0,
                ],
            ),
        ])
    }
}

/// An hour kept in 0 up to 24.
fn hour_of(h: f32) -> f32 {
    if h.is_finite() {
        h.rem_euclid(24.0)
    } else {
        0.0
    }
}

impl DayColours {
    /// From `stops` (hour, sRGB colour 0 to 1): sorted by hour, at most
    /// [`MAX_STOPS`], hours kept in 0 to 24 and colours in 0 to 1. None:
    /// the default.
    pub fn from_stops(stops: &[(f32, [f32; 3])]) -> Self {
        let mut list: Vec<(f32, [f32; 3])> = stops
            .iter()
            .take(MAX_STOPS)
            .map(|(h, c)| {
                (
                    hour_of(*h),
                    c.map(|v| {
                        if v.is_finite() {
                            v.clamp(0.0, 1.0)
                        } else {
                            0.0
                        }
                    }),
                )
            })
            .collect();
        if list.is_empty() {
            return Self::default();
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out = Self {
            stops: [(0.0, [1.0; 3]); MAX_STOPS],
            count: list.len(),
        };
        out.stops[..list.len()].copy_from_slice(&list);
        out
    }

    /// The stops, by hour.
    pub fn stops(&self) -> &[(f32, [f32; 3])] {
        &self.stops[..self.count]
    }

    /// Read from an area's text: `6:00 #e99a6a, 9:00 #fff6e0`. Parts that
    /// don't read are left out; nothing that reads gives the default.
    pub fn parse(text: &str) -> Self {
        let stops: Vec<(f32, [f32; 3])> = text
            .split(',')
            .filter_map(|part| {
                let mut words = part.split_whitespace();
                let (time, colour) = (words.next()?, words.next()?);
                let (h, m) = time.split_once(':').unwrap_or((time, "0"));
                let hour = h.parse::<f32>().ok()? + m.parse::<f32>().ok()? / 60.0;
                let hex = colour.strip_prefix('#')?;
                if hex.len() != 6 {
                    return None;
                }
                let v = u32::from_str_radix(hex, 16).ok()?;
                let [_, r, g, b] = v.to_be_bytes();
                Some((hour, [r, g, b].map(|c| f32::from(c) / 255.0)))
            })
            .collect();
        Self::from_stops(&stops)
    }

    /// As an area writes it.
    pub fn text(&self) -> String {
        self.stops()
            .iter()
            .map(|(h, c)| {
                let minutes = (h * 60.0).round() as u32;
                let [r, g, b] = c.map(|v| (v * 255.0).round() as u8);
                format!(
                    "{}:{:02} #{r:02x}{g:02x}{b:02x}",
                    minutes / 60 % 24,
                    minutes % 60
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The tint at `hour` (sRGB 0 to 1): between the stops either side,
    /// round midnight from the last to the first.
    pub fn at(&self, hour: f32) -> [f32; 3] {
        let h = hour_of(hour);
        let stops = self.stops();
        if stops.len() == 1 {
            return stops[0].1;
        }
        // The last stop at or before h (wrapping), and the next.
        let after = stops.iter().position(|(t, _)| *t > h).unwrap_or(0);
        let before = (after + stops.len() - 1) % stops.len();
        let (t0, c0) = stops[before];
        let (t1, c1) = stops[after];
        let span = (t1 - t0).rem_euclid(24.0);
        let into = (h - t0).rem_euclid(24.0);
        let k = if span > 0.0 { into / span } else { 0.0 };
        let k = k * k * (3.0 - 2.0 * k);
        [0, 1, 2].map(|i| c0[i] + (c1[i] - c0[i]) * k)
    }

    /// Whether it's night at `hour`: the tint darker than half the
    /// brightest stop's, so lamps that shine only at night come on at dusk
    /// and go off at dawn, whatever the colours.
    pub fn is_night(&self, hour: f32) -> bool {
        let brightness = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let brightest = self
            .stops()
            .iter()
            .map(|(_, c)| brightness(*c))
            .fold(0.0, f32::max);
        // A day all of black is all night.
        brightest <= 0.0 || brightness(self.at(hour)) < brightest * 0.5
    }
}

#[cfg(test)]
#[path = "day_tests.rs"]
mod tests;
