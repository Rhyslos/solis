//! Stable IDs: random 64-bit numbers, written as 13 Crockford base-32
//! characters ([decision 0020](../../../docs/decisions/0020-scene-files-in-lust-style.md)).
//! Random rather than counted, so two people on two branches never make
//! the same one.

use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

/// Crockford's base-32 alphabet: no I, L, O or U, so IDs can't be misread.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters in a written ID: 13 × 5 bits covers 64.
pub const ID_TEXT_LEN: usize = 13;

/// A stable ID. Never zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(NonZeroU64);

/// Why some text isn't an ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdError(pub String);

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\" isn't an ID (13 letters and digits)", self.0)
    }
}

impl std::error::Error for IdError {}

impl Id {
    /// A new random ID from the operating system's random numbers (or, if
    /// those fail, from the clock and a counter: still unique in practice).
    pub fn random() -> Self {
        let raw = getrandom::u64().unwrap_or_else(|_| fallback_random());
        Self::from_raw(raw).unwrap_or_else(Self::random)
    }

    /// The ID with this number, or `None` for zero.
    pub fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    /// Its number.
    pub fn raw(self) -> u64 {
        self.0.get()
    }
}

/// Random enough when the system's source fails: the clock mixed with a
/// counter (splitmix64).
fn fallback_random() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut z = nanos ^ COUNTER.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = [0u8; ID_TEXT_LEN];
        let mut n = self.raw();
        for slot in out.iter_mut().rev() {
            *slot = ALPHABET[(n & 31) as usize];
            n >>= 5;
        }
        // Only ASCII from the alphabet.
        f.write_str(std::str::from_utf8(&out).unwrap_or("0000000000000"))
    }
}

impl FromStr for Id {
    type Err = IdError;

    fn from_str(text: &str) -> Result<Self, IdError> {
        let bad = || IdError(text.chars().take(40).collect());
        if text.len() != ID_TEXT_LEN {
            return Err(bad());
        }
        let mut n: u64 = 0;
        for (i, c) in text.bytes().enumerate() {
            let digit = match c.to_ascii_uppercase() {
                // Crockford's readings of easily confused letters.
                b'O' => 0,
                b'I' | b'L' => 1,
                c => ALPHABET.iter().position(|&a| a == c).ok_or_else(bad)? as u64,
            };
            // The first character holds only the top 4 bits.
            if i == 0 && digit > 15 {
                return Err(bad());
            }
            n = (n << 5) | digit;
        }
        Self::from_raw(n).ok_or_else(bad)
    }
}

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub Id);

        impl $name {
            /// A new random one.
            pub fn random() -> Self {
                Self(Id::random())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(text: &str) -> Result<Self, IdError> {
                text.parse().map(Self)
            }
        }
    };
}

typed_id!(
    /// An entity in a scene, stable forever: in files, undo, selection and
    /// prefab overrides.
    EntityId
);
typed_id!(
    /// An asset (a model, a texture, a prefab), kept in its `.solmeta`
    /// file beside it so renaming the file breaks nothing.
    AssetId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_thirteen_readable_characters_and_read_back() {
        for _ in 0..1000 {
            let id = Id::random();
            let text = id.to_string();
            assert_eq!(text.len(), ID_TEXT_LEN);
            assert!(text.bytes().all(|c| ALPHABET.contains(&c)));
            assert_eq!(text.parse::<Id>().unwrap(), id);
        }
        let max = Id::from_raw(u64::MAX).unwrap();
        assert_eq!(max.to_string(), "FZZZZZZZZZZZZ");
        assert_eq!("FZZZZZZZZZZZZ".parse::<Id>().unwrap(), max);
        assert_eq!("0000000000001".parse::<Id>().unwrap().raw(), 1);
    }

    #[test]
    fn misreadable_letters_and_lower_case_are_accepted() {
        let id: Id = "0000000000O1L".parse().unwrap();
        assert_eq!(id.raw(), (1 << 5) | 1);
        assert_eq!(
            "00000000000ab".parse::<Id>().unwrap().to_string(),
            "00000000000AB"
        );
    }

    #[test]
    fn rubbish_is_refused_politely() {
        for text in [
            "",
            "0000000000000",
            "G000000000000",
            "000000000000U",
            "00000000000000",
            "€€€€",
            "ZZZZZZZZZZZZZ",
        ] {
            let err = text.parse::<Id>().unwrap_err();
            assert!(err.to_string().contains("isn't an ID"), "{text}");
        }
    }

    #[test]
    fn random_ids_differ() {
        let ids: std::collections::HashSet<Id> = (0..10_000).map(|_| Id::random()).collect();
        assert_eq!(ids.len(), 10_000);
        assert_ne!(fallback_random(), fallback_random());
    }
}
