//! A checksum of a game's state (Phase 6, determinism): one number that
//! changes if anything in what's fed to it does, so two runs of a game can
//! be compared tick by tick, and a difference found the tick it starts.
//!
//! Not for security: quick, and the same on every computer (only whole
//! numbers, little-endian, and floats by their exact bits).

use crate::{ComponentData, EntityId, Value};

/// A checksum being made: feed it values in a fixed order, then
/// [`Checksum::finish`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checksum(u64);

impl Default for Checksum {
    fn default() -> Self {
        Self::new()
    }
}

impl Checksum {
    /// An empty one.
    pub fn new() -> Self {
        Self(0x9e37_79b9_7f4a_7c15)
    }

    /// Feeds one whole number.
    pub fn u64(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    /// Feeds a decimal, by its exact bits.
    pub fn f64(&mut self, value: f64) {
        self.u64(value.to_bits());
    }

    /// Feeds a small decimal, by its exact bits.
    pub fn f32(&mut self, value: f32) {
        self.u64(u64::from(value.to_bits()));
    }

    /// Feeds bytes, with their length so neighbours can't run together.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.u64(bytes.len() as u64);
        for chunk in bytes.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.u64(u64::from_le_bytes(word));
        }
    }

    /// Feeds an entity.
    pub fn entity(&mut self, id: EntityId) {
        self.u64(id.0.raw());
    }

    /// Feeds a value, with what kind it is.
    pub fn value(&mut self, value: &Value) {
        match value {
            Value::Bool(b) => {
                self.u64(1);
                self.u64(u64::from(*b));
            }
            Value::Int(i) => {
                self.u64(2);
                self.u64(*i as u64);
            }
            Value::Float(f) => {
                self.u64(3);
                self.f64(*f);
            }
            Value::Text(t) => {
                self.u64(4);
                self.bytes(t.as_bytes());
            }
            Value::List(items) => {
                self.u64(5);
                self.u64(items.len() as u64);
                for item in items {
                    self.value(item);
                }
            }
            Value::Asset(asset) => {
                self.u64(6);
                match asset {
                    Some(a) => {
                        self.u64(a.asset.0.raw());
                        self.u64(a.part.map_or(u64::MAX, u64::from));
                    }
                    None => self.u64(0),
                }
            }
            Value::Entity(entity) => {
                self.u64(7);
                self.u64(entity.map_or(0, |e| e.0.raw()));
            }
        }
    }

    /// Feeds a component's values, by field name (always in that order).
    pub fn data(&mut self, data: &ComponentData) {
        self.u64(data.fields.len() as u64);
        for (name, value) in &data.fields {
            self.bytes(name.as_bytes());
            self.value(value);
        }
    }

    /// The checksum (mixed once more, so every bit fed counts).
    pub fn finish(&self) -> u64 {
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn of(values: &[Value]) -> u64 {
        let mut c = Checksum::new();
        for v in values {
            c.value(v);
        }
        c.finish()
    }

    #[test]
    fn any_difference_changes_it_and_the_same_never_does() {
        let a = [Value::Int(1), Value::Text("ab".into()), Value::Float(0.5)];
        assert_eq!(of(&a), of(&a.clone()));
        assert_ne!(
            of(&a),
            of(&[Value::Int(1), Value::Text("ab".into()), Value::Float(0.25)])
        );
        // Order matters, and kinds are told apart.
        assert_ne!(
            of(&[Value::Int(1), Value::Int(2)]),
            of(&[Value::Int(2), Value::Int(1)])
        );
        assert_ne!(of(&[Value::Int(1)]), of(&[Value::Float(1.0)]));
        assert_ne!(of(&[Value::Bool(false)]), of(&[Value::Int(0)]));
        // Neighbouring texts don't run together.
        assert_ne!(
            of(&[Value::Text("ab".into()), Value::Text("c".into())]),
            of(&[Value::Text("a".into()), Value::Text("bc".into())])
        );
        // The smallest change to a decimal shows.
        assert_ne!(
            of(&[Value::Float(1.0)]),
            of(&[Value::Float(f64::from_bits(1.0_f64.to_bits() + 1))])
        );
        assert_ne!(of(&[]), of(&[Value::List(Vec::new())]));
    }

    #[test]
    fn it_is_the_same_on_every_computer() {
        // A fixed answer: if this changes, saved checksums (recordings)
        // from before can't be compared.
        let mut c = Checksum::new();
        c.u64(42);
        c.f64(1.5);
        c.bytes(b"Solis");
        assert_eq!(c.finish(), FIXED);
    }

    const FIXED: u64 = 6_965_721_918_133_649_403;
}
