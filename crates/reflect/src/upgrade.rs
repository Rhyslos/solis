//! The one upgrade path every file format shares (Phase 8, C1;
//! [decision 0053](../../../docs/decisions/0053-compatibility-before-2d.md)).
//!
//! Each format has a version and a chain of upgraders: step 1 turns a
//! version 1 file into version 2, step 2 turns version 2 into 3, and so
//! on. A reader reads a file as written, then runs the steps from its
//! version up to the current one, in memory: the file on disk isn't
//! touched until it's saved, and then a copy is kept first
//! (`asset_db::backup`).
//!
//! A step works on the format's own read-but-not-yet-checked form (an
//! area's entities, a settings file's fields, a TOML table), so it can
//! rename a field or change a value without caring how it's written.
//! A newer file than the build understands is refused, never changed.

use std::fmt;

/// One upgrader: turns a file of one version into the next. An error says
/// why, in plain words, and the file is refused (never half-upgraded).
pub type Step<T> = fn(&mut T) -> Result<(), String>;

/// A file format's version and its upgraders.
pub struct Format<T: 'static> {
    /// What the file is, in plain words ("area", "input actions").
    pub name: &'static str,
    /// The version this build writes.
    pub current: u32,
    /// Finds the version a file's text says it is, cheaply, without
    /// reading the rest: `None` when it doesn't look like this format.
    pub version_of: fn(&str) -> Option<u32>,
    /// The upgraders, in order: `steps[0]` turns version 1 into 2.
    pub steps: &'static [Step<T>],
}

/// What upgrading did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upgraded {
    /// The version the file was written in.
    pub from: u32,
    /// The version it is now (the format's current one).
    pub to: u32,
}

impl Upgraded {
    /// Whether it was older, so it changed as it was read.
    pub fn changed(self) -> bool {
        self.from != self.to
    }
}

/// Why a file couldn't be brought up to date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeError {
    /// Written by a newer build than this one.
    TooNew {
        /// What it is.
        name: &'static str,
        /// Its version.
        found: u32,
        /// The newest this build reads.
        current: u32,
    },
    /// Version 0, which was never written.
    NoVersion0(&'static str),
    /// An older version this build has no upgrader for: a bug, which the
    /// format's tests catch before it ships.
    Missing {
        /// What it is.
        name: &'static str,
        /// The version with no step after it.
        from: u32,
    },
    /// An upgrader refused the file.
    Failed {
        /// What it is.
        name: &'static str,
        /// The version it was being upgraded from.
        from: u32,
        /// Why.
        why: String,
    },
}

impl fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooNew {
                name,
                found,
                current,
            } => write!(
                f,
                "this {name} file was saved by a newer version (format {found}; this one reads \
                 up to {current}). Update to open it."
            ),
            Self::NoVersion0(name) => write!(f, "there is no {name} format 0"),
            Self::Missing { name, from } => write!(
                f,
                "this version can't upgrade a {name} file from format {from} (a bug: please \
                 report it)"
            ),
            Self::Failed { name, from, why } => write!(
                f,
                "this {name} file couldn't be upgraded from format {from}: {why}"
            ),
        }
    }
}

impl std::error::Error for UpgradeError {}

impl<T> Format<T> {
    /// Brings `data`, read from a file of `version`, up to the current
    /// version, one step at a time. On an error `data` may be part-way:
    /// the caller drops it and refuses the file.
    pub fn upgrade(&self, version: u32, data: &mut T) -> Result<Upgraded, UpgradeError> {
        if version == 0 {
            return Err(UpgradeError::NoVersion0(self.name));
        }
        if version > self.current {
            return Err(UpgradeError::TooNew {
                name: self.name,
                found: version,
                current: self.current,
            });
        }
        for from in version..self.current {
            let step = self
                .steps
                .get(from as usize - 1)
                .ok_or(UpgradeError::Missing {
                    name: self.name,
                    from,
                })?;
            step(data).map_err(|why| UpgradeError::Failed {
                name: self.name,
                from,
                why,
            })?;
        }
        Ok(Upgraded {
            from: version,
            to: self.current,
        })
    }

    /// Whether the chain is whole: a step for every version before the
    /// current one, and none past it. Each format's tests check it.
    pub fn is_complete(&self) -> bool {
        self.current >= 1 && self.steps.len() == self.current as usize - 1
    }
}

/// A format's version, whatever it holds: what saving needs to know
/// whether the file on disk is older and must be backed up first.
pub trait Versioned: Sync {
    /// What the file is, in plain words.
    fn name(&self) -> &'static str;
    /// The version this build writes.
    fn current(&self) -> u32;
    /// The version a file's text says it is.
    fn version_of(&self, text: &str) -> Option<u32>;
}

impl<T> Versioned for Format<T> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn current(&self) -> u32 {
        self.current
    }

    fn version_of(&self, text: &str) -> Option<u32> {
        (self.version_of)(text)
    }
}

/// The version in a header line `<kind> version N` (areas, assemblies,
/// sound events, the mixer), after any `--` comments and blank lines:
/// for a format's `version_of`.
pub fn header_version(text: &str, kinds: &[&str]) -> Option<u32> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("--"))?;
    let mut words = line.split(|c: char| c.is_whitespace() || c == ';');
    let kind = words.next()?;
    if !kinds.contains(&kind) || words.next()? != "version" {
        return None;
    }
    words.next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version_of(text: &str) -> Option<u32> {
        header_version(text, &["thing"])
    }

    /// A made-up format at version 3: version 1 had `size`, 2 renamed it
    /// `width`, and 3 doubled it.
    static THING: Format<Vec<(String, i64)>> = Format {
        name: "thing",
        current: 3,
        version_of,
        steps: &[
            |fields| {
                for (key, _) in fields.iter_mut() {
                    if key == "size" {
                        *key = "width".into();
                    }
                }
                Ok(())
            },
            |fields| {
                for (_, value) in fields.iter_mut() {
                    *value = value.checked_mul(2).ok_or("the width is too big")?;
                }
                Ok(())
            },
        ],
    };

    #[test]
    fn an_old_file_climbs_one_version_at_a_time() {
        assert!(THING.is_complete());
        let mut fields = vec![("size".to_owned(), 4)];
        let done = THING.upgrade(1, &mut fields).unwrap();
        assert_eq!(fields, [("width".to_owned(), 8)]);
        assert_eq!((done.from, done.to, done.changed()), (1, 3, true));
        // From the middle: only the steps after it.
        let mut fields = vec![("width".to_owned(), 4)];
        THING.upgrade(2, &mut fields).unwrap();
        assert_eq!(fields, [("width".to_owned(), 8)]);
        // Current: nothing changes.
        let mut fields = vec![("width".to_owned(), 4)];
        assert!(!THING.upgrade(3, &mut fields).unwrap().changed());
        assert_eq!(fields, [("width".to_owned(), 4)]);
    }

    #[test]
    fn newer_missing_and_failing_files_are_refused() {
        let mut fields = vec![("size".to_owned(), i64::MAX)];
        let failed = THING.upgrade(1, &mut fields).unwrap_err();
        assert!(matches!(failed, UpgradeError::Failed { from: 2, .. }));
        assert!(failed.to_string().contains("too big"), "{failed}");
        let newer = THING.upgrade(4, &mut Vec::new()).unwrap_err();
        assert!(newer.to_string().contains("newer version"), "{newer}");
        assert_eq!(
            THING.upgrade(0, &mut Vec::new()),
            Err(UpgradeError::NoVersion0("thing"))
        );
        static GAP: Format<()> = Format {
            name: "gap",
            current: 3,
            version_of: |_| None,
            steps: &[|_| Ok(())],
        };
        assert!(!GAP.is_complete());
        assert_eq!(
            GAP.upgrade(1, &mut ()),
            Err(UpgradeError::Missing {
                name: "gap",
                from: 2
            })
        );
    }

    #[test]
    fn a_header_s_version_is_found_past_comments() {
        assert_eq!(version_of("thing version 2;\nx = 1;"), Some(2));
        assert_eq!(
            version_of("\u{feff}-- mine\r\n\r\n  thing version 7 root X;\r\n"),
            Some(7)
        );
        assert_eq!(version_of("other version 2;"), None);
        assert_eq!(version_of("thing versions 2;"), None);
        assert_eq!(version_of("thing version two;"), None);
        assert_eq!(version_of(""), None);
        let dynamic: &dyn Versioned = &THING;
        assert_eq!((dynamic.name(), dynamic.current()), ("thing", 3));
        assert_eq!(dynamic.version_of("thing version 1;"), Some(1));
    }
}
