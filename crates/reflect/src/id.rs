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
    /// A new random ID. The standard library's hasher keys come from the
    /// operating system's random numbers; mixed with the clock and a
    /// counter, every call gives a fresh, unpredictable number.
    pub fn random() -> Self {
        Self::from_raw(random_u64()).unwrap_or_else(Self::random)
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

/// 64 random bits without a dependency: a hasher seeded by the operating
/// system (`RandomState`), fed the clock and a counter so no two calls
/// match.
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(nanos);
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.finish()
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

        // Written in files as its 13 characters, as everywhere else.
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let text = <std::borrow::Cow<'de, str>>::deserialize(d)?;
                text.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

typed_id!(
    /// An entity in an area, stable forever: in files, undo, selection and
    /// assembly overrides.
    EntityId
);
typed_id!(
    /// An asset (a model, a texture, an assembly), kept in its `.solmeta`
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
        assert_ne!(random_u64(), random_u64());
    }

    #[test]
    fn ids_are_written_in_files_as_their_text() {
        let id = AssetId::random();
        let text = toml::to_string(&std::collections::BTreeMap::from([("id", id)])).unwrap();
        assert_eq!(text.trim(), format!("id = \"{id}\""));
        let back: std::collections::BTreeMap<String, AssetId> = toml::from_str(&text).unwrap();
        assert_eq!(back["id"], id);
        assert!(
            toml::from_str::<std::collections::BTreeMap<String, AssetId>>("id = \"0\"").is_err()
        );
    }
}

/// A hasher for maps keyed by IDs: IDs are random numbers already, so
/// one multiplication mixes them enough, many times quicker than the
/// standard hasher (which guards against keys chosen to collide; a
/// crafted file could at most slow a map, never break it).
#[derive(Debug, Default, Clone, Copy)]
pub struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// Builds [`IdHasher`]s, for `HashMap<EntityId, _, IdHashing>`.
pub type IdHashing = std::hash::BuildHasherDefault<IdHasher>;

/// A map keyed by entity IDs, with [`IdHasher`].
pub type IdMap<V> = std::collections::HashMap<EntityId, V, IdHashing>;

#[cfg(test)]
mod hasher_tests {
    use super::*;

    #[test]
    fn id_maps_find_what_they_hold_and_spread_the_hashes() {
        let mut map: IdMap<u32> = IdMap::default();
        let ids: Vec<EntityId> = (1..=10_000u64)
            .map(|n| EntityId(Id::from_raw(n).unwrap()))
            .collect();
        for (i, id) in ids.iter().enumerate() {
            map.insert(*id, i as u32);
        }
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(map.get(id), Some(&(i as u32)));
        }
        // Even counting numbers (the worst case for a plain identity hash)
        // land in many different top bits, which the map uses.
        use std::hash::BuildHasher;
        let tops: std::collections::HashSet<u64> = ids
            .iter()
            .map(|id| IdHashing::default().hash_one(id) >> 57)
            .collect();
        assert!(tops.len() > 100, "{}", tops.len());
    }
}
