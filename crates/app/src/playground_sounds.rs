//! The playground's sounds (Phase 7, S7), made here in code so they're
//! Solis's own (CC0, listed in `licenses/`) and nothing is downloaded: four
//! footsteps on gravel, a short music loop, a radio's chatter, and a drip
//! for the cave. Each is a mono WAV at 44,100 Hz, made the same every time
//! from fixed seeds, so the project's files never change between builds.

/// The sample rate the sounds are made at.
const RATE: u32 = 44_100;

/// A tiny random stream (xorshift), so the noise is the same every time.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

/// Samples from -1 to 1 as a 16-bit mono WAV file.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32_000.0) as i16).to_le_bytes())
        .collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend(b"RIFF");
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16_u32.to_le_bytes());
    out.extend(1_u16.to_le_bytes()); // whole numbers
    out.extend(1_u16.to_le_bytes()); // mono
    out.extend(RATE.to_le_bytes());
    out.extend((RATE * 2).to_le_bytes());
    out.extend(2_u16.to_le_bytes());
    out.extend(16_u16.to_le_bytes());
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

/// A footstep on gravel: a crunch of noise, low-passed so it's soft, with
/// a quick start and a short tail. `take` makes each a little different.
pub(crate) fn footstep(take: u64) -> Vec<u8> {
    let mut noise = Noise(0x5EED_F00D_0000_0001 + take * 7919);
    let length = 0.16 + 0.02 * take as f32;
    let n = (RATE as f32 * length) as usize;
    let mut low = 0.0_f32;
    let mut lower = 0.0_f32;
    let smooth = 0.18 + 0.03 * take as f32;
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            // Two filters in a row: a soft crunch, not a hiss.
            low += (noise.next() - low) * smooth;
            lower += (low - lower) * 0.5;
            let attack = (t / 0.004).min(1.0);
            let tail = (-t / (0.035 + 0.006 * take as f32)).exp();
            // A second, smaller crunch as the heel rolls onto the toe.
            let roll = if t > 0.05 {
                0.5 * (-(t - 0.05) / 0.03).exp()
            } else {
                0.0
            };
            lower * 2.2 * attack * (tail + roll)
        })
        .collect();
    wav(&samples)
}

/// The music: eight seconds that loop cleanly, four chords of soft
/// plucked notes over a low hum, quiet enough to sit under everything.
pub(crate) fn music() -> Vec<u8> {
    let n = RATE as usize * 8;
    // Am, F, C, G, as note frequencies.
    let chords: [[f32; 3]; 4] = [
        [220.0, 261.63, 329.63],
        [174.61, 220.0, 261.63],
        [261.63, 329.63, 392.0],
        [196.0, 246.94, 293.66],
    ];
    let tau = std::f32::consts::TAU;
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let bar = ((t / 2.0) as usize).min(3);
            let chord = chords[bar];
            // Eight plucks a bar, climbing the chord.
            let beat = t % 0.25;
            let note = chord[((t / 0.25) as usize) % 3];
            let pluck = (tau * note * t).sin() * (-beat / 0.12).exp() * 0.25;
            // The chord's root, an octave down, held under it all.
            let bar_t = t % 2.0;
            let swell = (bar_t / 0.3).min(1.0) * ((2.0 - bar_t) / 0.3).min(1.0);
            let hum = (tau * chord[0] * 0.5 * t).sin() * 0.15 * swell;
            pluck + hum
        })
        .collect();
    wav(&samples)
}

/// A radio's chatter: four seconds of a voice-like buzz in syllables, as
/// heard through a small speaker (no words, so it's no one's voice), with
/// pauses, looping cleanly.
pub(crate) fn radio() -> Vec<u8> {
    let n = RATE as usize * 4;
    let mut noise = Noise(0x5EED_0000_0000_0ADD);
    let tau = std::f32::consts::TAU;
    // Each syllable: (start s, length s, pitch Hz, vowel's first and
    // second resonances).
    let syllables = [
        (0.05, 0.18, 140.0, 700.0, 1200.0),
        (0.26, 0.14, 150.0, 400.0, 2000.0),
        (0.43, 0.22, 135.0, 600.0, 1000.0),
        (0.9, 0.16, 145.0, 300.0, 2300.0),
        (1.1, 0.2, 160.0, 650.0, 1100.0),
        (1.34, 0.12, 150.0, 500.0, 1700.0),
        (2.1, 0.2, 140.0, 700.0, 1200.0),
        (2.34, 0.16, 130.0, 350.0, 2100.0),
        (2.55, 0.24, 145.0, 600.0, 900.0),
        (3.2, 0.18, 155.0, 450.0, 1800.0),
    ];
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let mut s = 0.0;
            for (start, len, pitch, f1, f2) in syllables {
                let local = t - start;
                if !(0.0..len).contains(&local) {
                    continue;
                }
                let shape = (local / len * std::f32::consts::PI).sin();
                // A buzz at the pitch, shaped by two resonances: enough
                // like a vowel to read as speech.
                let buzz = (tau * pitch * t).sin().signum() * 0.5;
                let vowel = (tau * f1 * t).sin() * 0.6 + (tau * f2 * t).sin() * 0.3;
                s += buzz * vowel * shape;
            }
            // A little static, as a radio has.
            s * 0.5 + noise.next() * 0.02
        })
        .collect();
    wav(&samples)
}

/// A water drip in the cave: a short falling "plink", then quiet, so
/// looped it drips every two and a half seconds.
pub(crate) fn drip() -> Vec<u8> {
    let n = (RATE as f32 * 2.5) as usize;
    let tau = std::f32::consts::TAU;
    let samples: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let pitch = 1400.0 - 2000.0 * t;
            (tau * pitch * t).sin() * (-t / 0.05).exp() * 0.6
        })
        .collect();
    wav(&samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_reads_as_a_sound_and_is_the_same_each_time() {
        for (name, bytes) in [
            ("footstep", footstep(0)),
            ("music", music()),
            ("radio", radio()),
            ("drip", drip()),
        ] {
            let clip = sound::read_bytes(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(clip.info.channels, 1, "{name}");
            assert!(clip.info.seconds() > 0.1, "{name}");
            assert_eq!(
                bytes,
                match name {
                    "footstep" => footstep(0),
                    "music" => music(),
                    "radio" => radio(),
                    _ => drip(),
                }
            );
        }
        assert!((sound::read_bytes(&music()).unwrap().info.seconds() - 8.0).abs() < 0.01);
    }

    #[test]
    fn the_footsteps_differ() {
        let takes: Vec<Vec<u8>> = (0..4).map(footstep).collect();
        for a in 0..4 {
            for b in a + 1..4 {
                assert_ne!(takes[a], takes[b]);
            }
        }
    }
}
