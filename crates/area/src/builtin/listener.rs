//! Where the game hears from (Phase 7, S8; decision 0058): a listener on
//! a camera or a character. A game with none hears through its camera.
//! Until netcode, one computer hears through one listener: the first with
//! Hears now on. Its groups are what sound events heard by Groups look
//! for.

use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

use super::field;

/// Where the game hears from.
#[derive(Debug, Clone, PartialEq)]
pub struct Listener {
    /// Whether it's the one hearing now.
    pub hears_now: bool,
    /// The groups it's in, such as "Team Red" and "Radio", written with
    /// commas between them.
    pub groups: String,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            hears_now: true,
            groups: String::new(),
        }
    }
}

impl Listener {
    /// The longest groups text kept.
    pub const MAX_GROUPS_TEXT: usize = 1024;

    /// Its groups, one by one, without spaces around them.
    pub fn group_names(&self) -> Vec<&str> {
        self.groups
            .split(',')
            .map(str::trim)
            .filter(|g| !g.is_empty())
            .collect()
    }

    /// The same with anything unusable put right.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let groups = if self.groups.len() > Self::MAX_GROUPS_TEXT
            || self.groups.contains(char::is_control)
        {
            String::new()
        } else {
            self.groups
        };
        Self { groups, ..self }
    }
}

impl Component for Listener {
    const NAME: &'static str = "Listener";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Where the game hears from: put it on a camera or a character. With none, the \
                   camera hears. Sound events heard by Groups reach it if it's in one of theirs."
                .into(),
            fields: vec![
                field::boolean(
                    "hears_now",
                    "Hears now",
                    d.hears_now,
                    "Whether it's the one hearing. One computer hears through the first \
                     listener with this on.",
                ),
                FieldInfo::new(
                    "groups",
                    "Groups",
                    FieldKind::Text,
                    Value::Text(String::new()),
                )
                .hint(Hint {
                    help: "The groups it's in, with commas between: \"Team Red, Radio\".",
                    ..Hint::default()
                }),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("hears_now", Value::Bool(self.hears_now));
        data.set("groups", Value::Text(self.groups.clone()));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            hears_now: f.bool("hears_now", d.hears_now),
            groups: f.text("groups").unwrap_or_default().to_owned(),
        }
        .sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_are_read_one_by_one() {
        let l = Listener {
            groups: " Team Red, Radio ,, ".into(),
            ..Listener::default()
        };
        assert_eq!(l.group_names(), ["Team Red", "Radio"]);
        assert!(Listener::default().group_names().is_empty());
        let odd = Listener {
            groups: "a\u{7}b".into(),
            ..Listener::default()
        }
        .sanitized();
        assert!(odd.groups.is_empty());
        assert_eq!(Listener::from_data(&l.to_data()), l);
    }
}
