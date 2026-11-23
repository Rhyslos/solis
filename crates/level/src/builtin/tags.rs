//! An entity's tags (stage 1, tags): the project's tags it carries, such
//! as Enemy or Collectible, so game code can ask for "every enemy within
//! 3 m" or "the nearest collectible". Each tag is kept by its ID in the
//! project's list (`crate::tags`), so renaming a tag there changes no
//! level or module. Shown as chips under the entity's name in Properties,
//! never as a card of its own.
//!
//! Saved as one line of IDs (`tags = "1BWHNR3RV8PYY 0X3C…";`), in the
//! order they were given. A module's copy that adds or takes away a tag
//! changes this one field, so R5's rule holds: the copy's own change wins.

use reflect::{Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, TagId, Value};

/// The most tags one entity may carry. More in a file are left out.
pub const MAX_TAGS_PER_ENTITY: usize = 32;

/// The tags an entity carries, by their IDs in the project's list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tags {
    /// Each tag once, in the order given.
    pub ids: Vec<TagId>,
}

impl Tags {
    /// These tags (each once, at most [`MAX_TAGS_PER_ENTITY`]).
    pub fn of(ids: impl IntoIterator<Item = TagId>) -> Self {
        let mut tags = Self::default();
        for id in ids {
            tags.add(id);
        }
        tags
    }

    /// Whether it carries `id`.
    pub fn has(&self, id: TagId) -> bool {
        self.ids.contains(&id)
    }

    /// Adds `id` at the end, unless it's there already or there's no room;
    /// whether it was added.
    pub fn add(&mut self, id: TagId) -> bool {
        if self.has(id) || self.ids.len() >= MAX_TAGS_PER_ENTITY {
            return false;
        }
        self.ids.push(id);
        true
    }

    /// Takes `id` away; whether it was there.
    pub fn remove(&mut self, id: TagId) -> bool {
        let before = self.ids.len();
        self.ids.retain(|&t| t != id);
        self.ids.len() != before
    }

    /// The IDs as the level keeps them: written out, a space between.
    pub fn text(&self) -> String {
        let words: Vec<String> = self.ids.iter().map(ToString::to_string).collect();
        words.join(" ")
    }

    /// Tags read from their saved line: anything that isn't an ID is left
    /// out, as is a tag named twice, and the line is read only as far as
    /// [`MAX_TAGS_PER_ENTITY`] tags.
    pub fn read(text: &str) -> Self {
        let mut tags = Self::default();
        for word in text.split_whitespace() {
            if tags.ids.len() >= MAX_TAGS_PER_ENTITY {
                break;
            }
            if let Ok(id) = word.parse::<TagId>() {
                tags.add(id);
            }
        }
        tags
    }
}

impl Component for Tags {
    const NAME: &'static str = "Tags";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "The project's tags it carries (Project settings › Tags), such as Enemy: game \
                   code asks for them with hasTag, World:within and World:nearest."
                .into(),
            fields: vec![FieldInfo::new(
                "tags",
                "Tags",
                FieldKind::Text,
                Value::Text(String::new()),
            )],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("tags", Value::Text(self.text()));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        match data.get("tags") {
            Some(Value::Text(text)) => Self::read(text),
            _ => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_kept_once_each_in_order_and_round_trip() {
        let (a, b) = (TagId::random(), TagId::random());
        let mut tags = Tags::default();
        assert!(tags.add(a));
        assert!(tags.add(b));
        assert!(!tags.add(a), "never twice");
        assert_eq!(tags.ids, [a, b]);
        assert_eq!(Tags::from_data(&tags.to_data()), tags);
        assert!(tags.remove(a));
        assert!(!tags.remove(a));
        assert_eq!(tags.ids, [b]);
    }

    #[test]
    fn a_damaged_line_keeps_what_it_can() {
        let a = TagId::random();
        let text = format!("{a} nonsense {a}  ☃ 0000000000000");
        assert_eq!(Tags::read(&text).ids, [a]);
        let many: Vec<String> = (0..100).map(|_| TagId::random().to_string()).collect();
        assert_eq!(Tags::read(&many.join(" ")).ids.len(), MAX_TAGS_PER_ENTITY);
        let mut data = ComponentData::default();
        data.set("tags", Value::Int(3));
        assert_eq!(Tags::from_data(&data), Tags::default());
    }

    proptest::proptest! {
        /// Whatever a file says, reading never fails and never keeps more
        /// than the most, each once.
        #[test]
        fn any_line_reads_within_the_limits(text in "\\PC{0,400}") {
            let tags = Tags::read(&text);
            proptest::prop_assert!(tags.ids.len() <= MAX_TAGS_PER_ENTITY);
            let mut seen = std::collections::HashSet::new();
            proptest::prop_assert!(tags.ids.iter().all(|id| seen.insert(*id)));
            proptest::prop_assert_eq!(Tags::read(&tags.text()), tags);
        }
    }
}
