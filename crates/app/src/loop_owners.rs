//! What the game's looping sounds belong to (stage 1, states; decision
//! 0126): a loop a component starts in an option of its own state stops
//! when the component leaves that option, unless it kept it
//! (`keepStarted()`). Each (component, option) a sound was started in gets
//! a number the sound device knows the sound by; a state left gives back
//! the numbers of the options it left, and forgets them.

use reflect::EntityId;
use std::collections::HashMap;
use std::sync::Arc;

/// A component, by its entity and name.
type Component = (EntityId, Arc<str>);

/// The numbers of the options game code's sounds were started in, by
/// the component that started them.
#[derive(Debug, Default)]
pub(crate) struct LoopOwners {
    /// Each component's options with sounds, and their numbers.
    by_component: HashMap<Component, Vec<(Arc<str>, u64)>>,
    /// The last number given.
    last: u64,
}

impl LoopOwners {
    /// The number of the option `started` names, given one if it's new.
    pub(crate) fn number(&mut self, started: &script::Started) -> u64 {
        let options = self
            .by_component
            .entry((started.entity, Arc::clone(&started.component)))
            .or_default();
        if let Some((_, n)) = options.iter().find(|(o, _)| *o == started.option) {
            return *n;
        }
        self.last += 1;
        options.push((Arc::clone(&started.option), self.last));
        self.last
    }

    /// A component's state changed: the numbers of the options it left
    /// (their loops stop), forgotten, so starting one again gets a new
    /// number.
    pub(crate) fn left(&mut self, change: &script::StateChange) -> Vec<u64> {
        let key = (change.entity, Arc::clone(&change.component));
        let Some(options) = self.by_component.get_mut(&key) else {
            return Vec::new();
        };
        let mut ended = Vec::new();
        options.retain(|(option, n)| {
            let goes_on = script::still_in(&change.to, option);
            if !goes_on {
                ended.push(*n);
            }
            goes_on
        });
        if options.is_empty() {
            self.by_component.remove(&key);
        }
        ended
    }

    /// `keepStarted()`: the component's loops so far carry on past any
    /// state (their numbers are forgotten, so nothing stops them).
    pub(crate) fn keep(&mut self, entity: EntityId, component: &Arc<str>) {
        self.by_component.remove(&(entity, Arc::clone(component)));
    }

    /// A new play: nothing belongs to anything yet.
    pub(crate) fn clear(&mut self) {
        self.by_component.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> EntityId {
        EntityId(reflect::Id::from_raw(n).unwrap())
    }

    fn started(entity: u64, option: &str) -> script::Started {
        script::Started {
            entity: id(entity),
            component: "Hero".into(),
            option: option.into(),
        }
    }

    fn left(entity: u64, to: &str) -> script::StateChange {
        script::StateChange {
            entity: id(entity),
            component: "Hero".into(),
            from: "Anything".into(),
            to: to.into(),
        }
    }

    #[test]
    fn leaving_an_option_gives_back_its_loops_and_no_others() {
        let mut owners = LoopOwners::default();
        let combat = owners.number(&started(1, "Combat"));
        let melee = owners.number(&started(1, "Combat.Melee"));
        let other = owners.number(&started(2, "Combat.Melee"));
        assert_eq!(
            owners.number(&started(1, "Combat")),
            combat,
            "the same option, the same number"
        );
        assert_eq!(owners.left(&left(1, "Combat")), Vec::<u64>::new());
        assert_eq!(owners.left(&left(1, "Combat.Ranged")), vec![melee]);
        assert_eq!(owners.left(&left(1, "Calm")), vec![combat]);
        // Started again, a new number: a stop already sent never reaches
        // a loop started after it.
        assert_ne!(owners.number(&started(1, "Combat")), combat);
        assert_eq!(owners.left(&left(2, "Calm")), vec![other]);
    }

    #[test]
    fn kept_loops_are_never_given_back() {
        let mut owners = LoopOwners::default();
        owners.number(&started(1, "Stunned"));
        owners.keep(id(1), &"Hero".into());
        assert!(owners.left(&left(1, "Calm")).is_empty());
        owners.number(&started(1, "Stunned"));
        owners.clear();
        assert!(owners.left(&left(1, "Calm")).is_empty());
    }
}
