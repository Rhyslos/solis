//! Undo and redo: steps of edits, with the selection each step had
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//!
//! The selection type is the caller's (`S`): the area crate doesn't know
//! what the editor selects, only that undo brings it back.

use crate::document::Area;
use crate::edit::{Edit, EditError};
use reflect::EntityId;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// How much memory history may use by default: 64 MiB.
pub const DEFAULT_LIMIT_BYTES: usize = 64 << 20;

/// Which steps may merge into one: the same key while one gesture goes on
/// (a whole drag of one field is one step).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MergeKey {
    /// What is being changed (a field of an entity, say), hashed.
    pub target: u64,
    /// Which gesture (a press of the mouse, say) is doing it.
    pub gesture: u64,
}

/// How much memory a selection kept with each step uses, so the history's
/// limit counts it: a step keeps the selection before and after it.
pub trait Weigh {
    /// About how many bytes it uses.
    fn weight(&self) -> usize;
}

impl Weigh for () {
    fn weight(&self) -> usize {
        0
    }
}

impl Weigh for u32 {
    fn weight(&self) -> usize {
        4
    }
}

/// One undo step.
#[derive(Debug, Clone, PartialEq)]
pub struct Step<S> {
    /// What it did, for Edit > Undo ("Change Light.strength").
    pub label: String,
    /// The edits that undo it, in the order to apply them.
    undo: Vec<Edit>,
    /// The edits that redo it, in the order to apply them.
    redo: Vec<Edit>,
    merge: Option<MergeKey>,
    /// The selection before it.
    before: S,
    /// The selection after it.
    after: S,
    bytes: usize,
    /// Tells this step's result apart from every other state, for
    /// [`History::state`].
    serial: u64,
}

impl<S: Weigh> Step<S> {
    fn measure(&mut self) {
        self.bytes = self.label.len()
            + self.before.weight()
            + self.after.weight()
            + self
                .undo
                .iter()
                .chain(&self.redo)
                .map(Edit::bytes)
                .sum::<usize>();
    }

    /// Whether undoing it would change nothing: every edit in it put back
    /// the value it found (a drag that ended where it started). Looked up
    /// by what each edit changes, so a drag of thousands stays quick.
    fn changes_nothing(&self) -> bool {
        if self.undo.len() != self.redo.len() {
            return false;
        }
        let mut undone: HashMap<Target<'_>, Vec<&Edit>> = HashMap::with_capacity(self.undo.len());
        for edit in &self.undo {
            if let Some(t) = target(edit) {
                undone.entry(t).or_default().push(edit);
            }
        }
        self.redo.iter().all(|edit| {
            target(edit)
                .and_then(|t| undone.get(&t))
                .is_some_and(|found| found.contains(&edit))
        })
    }
}

/// What a field or name edit changes: the entity, and the component and
/// field (none for its name).
type Target<'a> = (EntityId, Option<(&'a str, &'a str)>);

/// What `edit` changes, for a field or name edit (others have none).
fn target(edit: &Edit) -> Option<Target<'_>> {
    match edit {
        Edit::SetField {
            entity,
            component,
            field,
            ..
        } => Some((*entity, Some((component.as_str(), field.as_str())))),
        Edit::Rename { entity, .. } => Some((*entity, None)),
        _ => None,
    }
}

/// What undo or redo did.
#[derive(Debug, Clone, PartialEq)]
pub struct Undone<S> {
    /// The step's label.
    pub label: String,
    /// The selection to show now.
    pub selection: S,
}

/// The undo and redo stacks.
#[derive(Debug, Clone)]
pub struct History<S> {
    // Steps are shared, so a copy of the history (Play keeps one) costs
    // next to nothing; one is copied only if it changes afterwards.
    undo: VecDeque<Arc<Step<S>>>,
    redo: Vec<Arc<Step<S>>>,
    bytes: usize,
    limit: usize,
    /// Whether the newest step may still take more edits (false after
    /// undo or redo, so a returned-to step never grows).
    open: bool,
    /// The next step's serial.
    next_serial: u64,
    /// The serial of the newest step dropped to keep within the limit (0
    /// if none was): with every step left undone, the area is still past
    /// it, so it mustn't look like the start.
    dropped: u64,
    /// The state [`Self::hold`] was called at, if it was: steps made after
    /// it are kept whatever the limit.
    held: Option<u64>,
}

/// What [`History::follow`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Followed {
    /// The labels of the steps made again, oldest first.
    pub made: Vec<String>,
    /// The labels of the steps left out, oldest first: nothing of them
    /// could be made here.
    pub left_out: Vec<String>,
}

impl<S: Clone + Weigh> Default for History<S> {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT_BYTES)
    }
}

impl<S: Clone + Weigh> History<S> {
    /// Empty history that keeps about `limit` bytes of steps (always at
    /// least the newest).
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            bytes: 0,
            limit,
            open: false,
            next_serial: 1,
            dropped: 0,
            held: None,
        }
    }

    /// Applies `edits` to `area` as one step called `label`. With a
    /// `merge` key equal to the newest step's, they join that step
    /// instead. `before` and `after` are the selection either side of it.
    /// On an error nothing changes.
    pub fn apply(
        &mut self,
        area: &mut Area,
        label: &str,
        edits: Vec<Edit>,
        merge: Option<MergeKey>,
        before: S,
        after: S,
    ) -> Result<(), EditError> {
        if edits.is_empty() {
            return Ok(());
        }
        let redo = edits.clone();
        let undo = area.apply_all(edits)?;
        self.redo.clear();
        let merging =
            self.open && merge.is_some() && self.undo.back().is_some_and(|s| s.merge == merge);
        let newest = if merging {
            self.undo.pop_back().map(Arc::unwrap_or_clone)
        } else {
            None
        };
        if let Some(mut step) = newest {
            self.bytes -= step.bytes;
            step.redo.extend(redo);
            let mut undo = undo;
            undo.append(&mut step.undo);
            step.undo = undo;
            step.after = after;
            compact(&mut step);
            // A step that grew is a new state.
            step.serial = self.serial();
            if step.changes_nothing() {
                // Back where it started: no step at all. The next edit of
                // the same gesture starts a new one.
                self.open = false;
                return Ok(());
            }
            step.measure();
            self.push(step);
        } else {
            let mut step = Step {
                label: label.to_string(),
                undo,
                redo,
                merge,
                before,
                after,
                bytes: 0,
                serial: self.serial(),
            };
            step.measure();
            self.push(step);
        }
        self.open = true;
        Ok(())
    }

    fn serial(&mut self) -> u64 {
        self.next_serial += 1;
        self.next_serial - 1
    }

    /// A number standing for the area as it is now: it comes back exactly
    /// when undo or redo return to the same point, so comparing it with
    /// the one at the last save says whether there are unsaved changes.
    /// 0 is the start (nothing done).
    pub fn state(&self) -> u64 {
        self.undo.back().map_or(self.dropped, |s| s.serial)
    }

    fn push(&mut self, step: Step<S>) {
        self.bytes += step.bytes;
        self.undo.push_back(Arc::new(step));
        while self.bytes > self.limit && self.undo.len() > 1 {
            // A held step stays, and so does every step after it.
            let held = self
                .held
                .is_some_and(|held| self.undo.front().is_some_and(|oldest| oldest.serial > held));
            if held {
                break;
            }
            if let Some(old) = self.undo.pop_front() {
                self.bytes -= old.bytes;
                self.dropped = old.serial;
            }
        }
    }

    /// From now on, keeps every step made (or redone) after this moment,
    /// whatever the limit: older steps are still dropped to keep within
    /// it. For a copy that [`Self::follow`] will be asked about (the
    /// editor's, while the game plays): a step dropped couldn't be made
    /// again.
    pub fn hold(&mut self) {
        self.held = Some(self.state());
    }

    /// Makes here what was done in `played` since it was a copy of this
    /// history (and `area` a copy of its area) that was then
    /// [held](Self::hold): the steps undone there are undone here, and the
    /// steps made there are made here, as steps of their own that can be
    /// undone. What changed `played`'s area without being a step (a
    /// running game's entities) isn't made, so a step may no longer fit:
    /// for each entity, what the step did to it is made whole or left
    /// out, and a step with nothing left isn't made at all. The steps
    /// undone and not made again there can't be redone here.
    pub fn follow(&mut self, area: &mut Area, played: &Self) -> Followed {
        let there: HashSet<u64> = played.undo.iter().map(|s| s.serial).collect();
        // Steps there keep their order, so the ones undone are the newest
        // here. One dropped there to keep within the limit is still done.
        while self
            .undo
            .back()
            .is_some_and(|s| s.serial > played.dropped && !there.contains(&s.serial))
        {
            // Undoing a step on the area it was made on can't fail.
            let _ = self.undo(area);
        }
        let here: HashSet<u64> = self.undo.iter().map(|s| s.serial).collect();
        let mut followed = Followed::default();
        for step in played.undo.iter().filter(|s| !here.contains(&s.serial)) {
            let (undo, redo) = make_what_fits(area, &step.redo);
            if redo.is_empty() {
                followed.left_out.push(step.label.clone());
                continue;
            }
            let mut made = Step {
                label: step.label.clone(),
                undo,
                redo,
                merge: None,
                before: step.before.clone(),
                after: step.after.clone(),
                bytes: 0,
                serial: self.serial(),
            };
            made.measure();
            self.redo.clear();
            self.push(made);
            followed.made.push(step.label.clone());
        }
        self.open = false;
        followed
    }

    /// Undoes the newest step, returning its label and the selection it
    /// started with; `None` if there's nothing to undo.
    pub fn undo(&mut self, area: &mut Area) -> Option<Result<Undone<S>, EditError>> {
        let mut step = Arc::unwrap_or_clone(self.undo.pop_back()?);
        self.bytes -= step.bytes;
        self.open = false;
        match area.apply_all(step.undo.clone()) {
            Ok(redo) => {
                step.redo = redo;
                let done = Undone {
                    label: step.label.clone(),
                    selection: step.before.clone(),
                };
                self.redo.push(Arc::new(step));
                Some(Ok(done))
            }
            Err(why) => Some(Err(why)),
        }
    }

    /// Redoes the step undone last, returning its label and the selection
    /// it ended with; `None` if there's nothing to redo.
    pub fn redo(&mut self, area: &mut Area) -> Option<Result<Undone<S>, EditError>> {
        let mut step = Arc::unwrap_or_clone(self.redo.pop()?);
        self.open = false;
        match area.apply_all(step.redo.clone()) {
            Ok(undo) => {
                step.undo = undo;
                let done = Undone {
                    label: step.label.clone(),
                    selection: step.after.clone(),
                };
                step.measure();
                self.push(step);
                Some(Ok(done))
            }
            Err(why) => Some(Err(why)),
        }
    }

    /// The label of the step undo would undo.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|s| s.label.as_str())
    }

    /// The label of the step redo would redo.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|s| s.label.as_str())
    }

    /// How many steps can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// About how many bytes the undo steps use.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Ends the newest step: the next edit starts a new one even with the
    /// same merge key.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Forgets everything (a new area was opened). Serials keep counting,
    /// so no old state is mistaken for a new one.
    pub fn clear(&mut self) {
        let next = self.next_serial;
        *self = Self::with_limit(self.limit);
        self.next_serial = next;
    }
}

/// Makes as much of `edits` in `area` as fits, an entity at a time: what
/// they do to one entity is made whole or not at all, so nothing is left
/// half done (an entity moved under a new parent without the place that
/// goes with it). Returns the edits that undo what was made, and the
/// edits made, each in the order to apply them.
fn make_what_fits(area: &mut Area, edits: &[Edit]) -> (Vec<Edit>, Vec<Edit>) {
    let mut left_out: HashSet<Option<EntityId>> = HashSet::new();
    loop {
        let mut undo = Vec::new();
        let mut made = Vec::new();
        let mut all_fit = true;
        for edit in edits {
            let about = subject(edit);
            if left_out.contains(&about) {
                continue;
            }
            let done = if has_its_parent(area, edit) {
                area.apply(edit.clone()).ok()
            } else {
                None
            };
            match done {
                Some(inverse) => {
                    undo.push(inverse);
                    made.push(edit.clone());
                }
                None => {
                    left_out.insert(about);
                    all_fit = false;
                }
            }
        }
        undo.reverse();
        if all_fit {
            return (undo, made);
        }
        // One didn't fit: everything is taken back, and the rest made
        // again without that entity's share (each round leaves out at
        // least one more entity, so this ends).
        for inverse in undo {
            // Undoing what was just done can't fail.
            let _ = area.apply(inverse);
        }
    }
}

/// The entity `edit` is about: the one it changes, or the first of the
/// tree it adds or deletes.
fn subject(edit: &Edit) -> Option<EntityId> {
    match edit {
        Edit::SetField { entity, .. }
        | Edit::AddComponent { entity, .. }
        | Edit::RemoveComponent { entity, .. }
        | Edit::Rename { entity, .. }
        | Edit::SetOrigin { entity, .. }
        | Edit::Reparent { entity, .. } => Some(*entity),
        Edit::Spawn { entities } => entities.first().map(|(id, _)| *id),
        Edit::Despawn { root } => Some(*root),
    }
}

/// Whether the tree `edit` adds has the parent it goes under in `area`
/// (true for every other edit, and for a tree at the top level): applying
/// the edit doesn't ask, and would leave the tree under nothing.
fn has_its_parent(area: &Area, edit: &Edit) -> bool {
    match edit {
        Edit::Spawn { entities } => entities
            .first()
            .and_then(|(_, data)| data.parent)
            .is_none_or(|parent| area.contains(parent)),
        _ => true,
    }
}

/// Merged steps of field changes keep only the first value each field had
/// and the last it was given, so a long drag stays small.
fn compact<S>(step: &mut Step<S>) {
    if step
        .redo
        .iter()
        .chain(&step.undo)
        .any(|e| target(e).is_none())
    {
        return;
    }
    // Redo: the last edit of each target wins. Undo: the last edit in the
    // undo list restores the first value, so it wins there too. Found
    // from the end with a set, so each frame of a long drag of many
    // entities costs as much as the edits it has, not their square.
    fn keep_last(edits: &mut Vec<Edit>) {
        let mut seen = HashSet::with_capacity(edits.len());
        let mut keep: Vec<bool> = edits.iter().rev().map(|e| seen.insert(target(e))).collect();
        keep.reverse();
        let mut flags = keep.into_iter();
        edits.retain(|_| flags.next().unwrap_or(true));
    }
    keep_last(&mut step.redo);
    keep_last(&mut step.undo);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{Light, Transform};
    use crate::document::EntityData;
    use reflect::{ComponentData, EntityId, Value};

    fn set(entity: EntityId, strength: f64) -> Edit {
        Edit::SetField {
            entity,
            component: "Light".into(),
            field: "strength".into(),
            value: Some(Value::Float(strength)),
        }
    }

    fn strength(s: &Area, id: EntityId) -> f32 {
        s.get(id).unwrap().get::<Light>().unwrap().strength
    }

    fn lamp() -> (Area, EntityId) {
        let mut s = Area::new();
        let id = s.spawn(EntityData::new("Lamp").with(&Light::point()));
        (s, id)
    }

    const DRAG: Option<MergeKey> = Some(MergeKey {
        target: 1,
        gesture: 1,
    });

    /// A step called `label` that sets the lamp's strength.
    fn change(h: &mut History<u32>, s: &mut Area, label: &str, id: EntityId, to: f64) {
        h.apply(s, label, vec![set(id, to)], None, 0, 0).unwrap();
    }

    #[test]
    fn follow_makes_again_what_was_done_since_the_copy() {
        let (mut s, id) = lamp();
        let mut h = History::<u32>::default();
        change(&mut h, &mut s, "Change strength", id, 5.0);
        let (mut kept_area, mut kept) = (s.clone(), h.clone());
        h.hold();
        // Nothing done since: nothing to make.
        assert_eq!(kept.follow(&mut kept_area, &h), Followed::default());
        assert_eq!(kept.state(), h.state());
        change(&mut h, &mut s, "Brighter", id, 9.0);
        h.apply(&mut s, "Dimmer", vec![set(id, 2.0)], DRAG, 3, 4)
            .unwrap();
        let followed = kept.follow(&mut kept_area, &h);
        assert_eq!(followed.made, ["Brighter", "Dimmer"]);
        assert!(followed.left_out.is_empty());
        assert_eq!(kept_area, s);
        assert_eq!(kept.undo_len(), 3);
        // Each is a step of its own, with its selections, and closed: the
        // gesture that made the last one doesn't grow it here.
        kept.apply(&mut kept_area, "Dimmer", vec![set(id, 1.0)], DRAG, 4, 5)
            .unwrap();
        assert_eq!(kept.undo_len(), 4);
        kept.undo(&mut kept_area).unwrap().unwrap();
        let undone = kept.undo(&mut kept_area).unwrap().unwrap();
        assert_eq!((strength(&kept_area, id), undone.selection), (9.0, 3));
        kept.undo(&mut kept_area).unwrap().unwrap();
        assert_eq!(strength(&kept_area, id), 5.0);
        let redone = kept.redo(&mut kept_area).unwrap().unwrap();
        assert_eq!(
            (strength(&kept_area, id), redone.label.as_str()),
            (9.0, "Brighter")
        );
    }

    #[test]
    fn follow_undoes_what_was_undone_and_makes_what_was_redone() {
        let (mut s, id) = lamp();
        let mut h = History::<u32>::default();
        change(&mut h, &mut s, "One", id, 1.0);
        change(&mut h, &mut s, "Two", id, 2.0);
        change(&mut h, &mut s, "Three", id, 3.0);
        h.undo(&mut s).unwrap().unwrap();
        let copy = (s.clone(), h.clone());
        h.hold();
        // Undone since, and nothing else: it can be redone in the copy too.
        h.undo(&mut s).unwrap().unwrap();
        let (mut kept_area, mut kept) = copy.clone();
        assert_eq!(kept.follow(&mut kept_area, &h), Followed::default());
        assert_eq!((strength(&kept_area, id), kept.undo_len()), (1.0, 1));
        assert_eq!(kept.redo_label(), Some("Two"));
        // Undone and redone: as it was, so nothing to make.
        h.redo(&mut s).unwrap().unwrap();
        let (mut kept_area, mut kept) = copy.clone();
        assert_eq!(kept.follow(&mut kept_area, &h), Followed::default());
        assert_eq!(kept.state(), copy.1.state());
        // A step undone before the copy and redone since is made.
        h.redo(&mut s).unwrap().unwrap();
        let (mut kept_area, mut kept) = copy.clone();
        assert_eq!(kept.follow(&mut kept_area, &h).made, ["Three"]);
        assert_eq!((strength(&kept_area, id), kept.undo_len()), (3.0, 3));
        // Everything undone, then something new.
        while h.undo(&mut s).is_some() {}
        change(&mut h, &mut s, "Four", id, 4.0);
        let (mut kept_area, mut kept) = copy;
        assert_eq!(kept.follow(&mut kept_area, &h).made, ["Four"]);
        assert_eq!(kept_area, s);
        assert_eq!(kept.undo_len(), 1);
        assert!(kept.redo(&mut kept_area).is_none());
        kept.undo(&mut kept_area).unwrap().unwrap();
        assert_eq!(strength(&kept_area, id), 1000.0);
    }

    /// The game's entities: never in the copy.
    fn game_id(n: u64) -> EntityId {
        EntityId(reflect::Id::from_raw(1_000_000 + n).unwrap())
    }

    fn spawn(id: EntityId, name: &str, parent: Option<EntityId>) -> Edit {
        let mut data = EntityData::new(name).with(&Transform::default());
        data.parent = parent;
        Edit::Spawn {
            entities: vec![(id, data)],
        }
    }

    fn rename(entity: EntityId, name: &str) -> Edit {
        Edit::Rename {
            entity,
            name: name.into(),
        }
    }

    #[test]
    fn follow_leaves_out_what_was_done_to_entities_that_arent_steps() {
        let (mut s, lamp) = lamp();
        let mut h = History::<u32>::default();
        let crate_id = EntityId::random();
        h.apply(
            &mut s,
            "Add crate",
            vec![spawn(crate_id, "Crate", None)],
            None,
            0,
            0,
        )
        .unwrap();
        let (mut kept_area, mut kept) = (s.clone(), h.clone());
        let before = s.clone();
        h.hold();
        // The game adds a drone and takes the crate out: not steps.
        let drone = game_id(1);
        s.apply(spawn(drone, "Drone", None)).unwrap();
        // A step all about the drone; one about the drone and the lamp;
        // one that puts the lamp under the drone, with the place that goes
        // with it, and names the crate; and new entities under each.
        h.apply(
            &mut s,
            "Rename drone",
            vec![rename(drone, "Mine")],
            None,
            0,
            0,
        )
        .unwrap();
        h.apply(
            &mut s,
            "Rename 2 entities",
            vec![rename(drone, "Both"), rename(lamp, "Both")],
            None,
            0,
            0,
        )
        .unwrap();
        let place = Edit::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "range".into(),
            value: Some(Value::Float(7.0)),
        };
        let under = Edit::Reparent {
            entity: lamp,
            parent: Some(drone),
            order: "a".into(),
        };
        h.apply(
            &mut s,
            "Move under drone",
            vec![place, under, rename(crate_id, "Box")],
            None,
            0,
            0,
        )
        .unwrap();
        let (under_drone, under_crate) = (EntityId::random(), EntityId::random());
        h.apply(
            &mut s,
            "Add 2 entities",
            vec![
                spawn(under_drone, "Under drone", Some(drone)),
                spawn(under_crate, "Under crate", Some(crate_id)),
            ],
            None,
            0,
            0,
        )
        .unwrap();
        s.apply(Edit::Despawn { root: crate_id }).unwrap();
        h.apply(
            &mut s,
            "Delete drone",
            vec![Edit::Despawn { root: drone }],
            None,
            0,
            0,
        )
        .unwrap();
        let followed = kept.follow(&mut kept_area, &h);
        assert_eq!(
            followed.made,
            ["Rename 2 entities", "Move under drone", "Add 2 entities"]
        );
        assert_eq!(followed.left_out, ["Rename drone", "Delete drone"]);
        // The copy as it was, with only what fits it: the lamp renamed but
        // not moved (nor given the place that went with the move), the
        // crate (still there) renamed and given its new child.
        let mut expected = before.clone();
        for edit in [
            rename(lamp, "Both"),
            rename(crate_id, "Box"),
            spawn(under_crate, "Under crate", Some(crate_id)),
        ] {
            expected.apply(edit).unwrap();
        }
        assert_eq!(kept_area, expected);
        assert!(!kept_area.contains(drone) && !kept_area.contains(under_drone));
        // And each step made undoes exactly what it made.
        for _ in 0..3 {
            kept.undo(&mut kept_area).unwrap().unwrap();
        }
        assert_eq!(kept_area, before);
        assert_eq!(kept.undo_len(), 1);
    }

    #[test]
    fn a_held_history_keeps_every_step_made_since() {
        let (mut s, id) = lamp();
        let mut h = History::<u32>::with_limit(1_000);
        change(&mut h, &mut s, "One", id, 1.0);
        change(&mut h, &mut s, "Two", id, 2.0);
        change(&mut h, &mut s, "Three", id, 3.0);
        let copy = (s.clone(), h.clone());
        assert_eq!(h.undo_len(), 3, "within the limit so far");
        h.hold();
        h.undo(&mut s).unwrap().unwrap();
        for i in 0..100 {
            change(&mut h, &mut s, &format!("Since {i}"), id, f64::from(i));
        }
        // The steps from before go, as the limit says; those made since
        // stay, though they're over it.
        assert_eq!(h.undo_len(), 100);
        assert!(h.bytes() > 1_000);
        // So the copy can follow: One and Two were dropped, so they're
        // still done; Three was undone; and all hundred are made.
        let (mut kept_area, mut kept) = copy;
        let followed = kept.follow(&mut kept_area, &h);
        assert_eq!(followed.made.len(), 100);
        assert_eq!(kept_area, s);
        // The copy isn't held: it keeps to its limit again.
        assert!(kept.bytes() <= 1_000);
        while kept.undo(&mut kept_area).is_some() {}
        assert_ne!(
            strength(&kept_area, id),
            1000.0,
            "One and Two are still done"
        );
    }

    #[test]
    fn undo_and_redo_go_back_and_forth_with_the_selection() {
        let (mut s, id) = lamp();
        let mut h = History::<u32>::default();
        h.apply(&mut s, "Change strength", vec![set(id, 5.0)], None, 1, 2)
            .unwrap();
        h.apply(&mut s, "Change strength", vec![set(id, 9.0)], None, 2, 3)
            .unwrap();
        assert_eq!(h.undo_len(), 2);
        let undone = h.undo(&mut s).unwrap().unwrap();
        assert_eq!((strength(&s, id), undone.selection), (5.0, 2));
        h.undo(&mut s).unwrap().unwrap();
        assert_eq!(strength(&s, id), 1000.0);
        assert!(h.undo(&mut s).is_none());
        let redone = h.redo(&mut s).unwrap().unwrap();
        assert_eq!((strength(&s, id), redone.selection), (5.0, 2));
        assert_eq!(h.redo_label(), Some("Change strength"));
        // A new edit drops what could be redone.
        h.apply(
            &mut s,
            "Rename",
            vec![Edit::Rename {
                entity: id,
                name: "X".into(),
            }],
            None,
            2,
            2,
        )
        .unwrap();
        assert!(h.redo(&mut s).is_none());
        assert_eq!(h.undo_label(), Some("Rename"));
    }

    #[test]
    fn a_copy_shares_its_steps_but_never_sees_later_changes() {
        let (mut s, id) = lamp();
        let mut h = History::<()>::default();
        h.apply(&mut s, "Drag", vec![set(id, 1100.0)], DRAG, (), ())
            .unwrap();
        let (kept_area, kept) = (s.clone(), h.clone());
        assert!(Arc::ptr_eq(&h.undo[0], &kept.undo[0]), "shared, not copied");
        // The drag goes on in the original (as while playing): the copy's
        // step stays as it was, and undoing the copy goes back to the start.
        h.apply(&mut s, "Drag", vec![set(id, 1200.0)], DRAG, (), ())
            .unwrap();
        h.undo(&mut s).unwrap().unwrap();
        let (mut s2, mut kept) = (kept_area, kept);
        assert_eq!(strength(&s2, id), 1100.0);
        kept.undo(&mut s2).unwrap().unwrap();
        assert_eq!(strength(&s2, id), 1000.0);
        kept.redo(&mut s2).unwrap().unwrap();
        assert_eq!(
            strength(&s2, id),
            1100.0,
            "its own step, not the longer drag"
        );
    }

    #[test]
    fn a_drag_is_one_step_and_a_drag_back_is_none() {
        let (mut s, id) = lamp();
        let mut h = History::<()>::default();
        for v in [1100.0, 1200.0, 1300.0] {
            h.apply(&mut s, "Drag", vec![set(id, v)], DRAG, (), ())
                .unwrap();
        }
        assert_eq!(h.undo_len(), 1);
        let step = h.undo.back().unwrap();
        assert_eq!((step.undo.len(), step.redo.len()), (1, 1), "compacted");
        h.undo(&mut s).unwrap().unwrap();
        assert_eq!(strength(&s, id), 1000.0);
        h.redo(&mut s).unwrap().unwrap();
        assert_eq!(strength(&s, id), 1300.0);
        // A step returned to by undo or redo never grows.
        h.apply(&mut s, "Drag", vec![set(id, 1400.0)], DRAG, (), ())
            .unwrap();
        assert_eq!(h.undo_len(), 2);
        // A new drag that ends where it started leaves no step.
        let back = Some(MergeKey {
            target: 1,
            gesture: 2,
        });
        h.apply(&mut s, "Drag", vec![set(id, 1500.0)], back, (), ())
            .unwrap();
        h.apply(&mut s, "Drag", vec![set(id, 1400.0)], back, (), ())
            .unwrap();
        assert_eq!(h.undo_len(), 2);
        assert_eq!(strength(&s, id), 1400.0);
    }

    #[test]
    fn the_state_comes_back_with_undo_and_redo() {
        let (mut s, id) = lamp();
        let mut h = History::<()>::default();
        assert_eq!(h.state(), 0);
        h.apply(&mut s, "A", vec![set(id, 1.0)], None, (), ())
            .unwrap();
        let saved = h.state();
        h.apply(&mut s, "B", vec![set(id, 2.0)], DRAG, (), ())
            .unwrap();
        let after_b = h.state();
        assert_ne!(after_b, saved);
        // The drag grows: a new state.
        h.apply(&mut s, "B", vec![set(id, 3.0)], DRAG, (), ())
            .unwrap();
        assert_ne!(h.state(), after_b);
        let grown = h.state();
        h.undo(&mut s).unwrap().unwrap();
        assert_eq!(h.state(), saved, "back at the save");
        h.redo(&mut s).unwrap().unwrap();
        assert_eq!(h.state(), grown);
        h.clear();
        assert_eq!(h.state(), 0);
    }

    #[test]
    fn history_is_limited_by_memory() {
        let (mut s, id) = lamp();
        let mut h = History::<()>::with_limit(2_000);
        for i in 0..200 {
            h.apply(&mut s, "Change", vec![set(id, f64::from(i))], None, (), ())
                .unwrap();
        }
        assert!(h.bytes() <= 2_000, "{}", h.bytes());
        assert!(h.undo_len() > 1 && h.undo_len() < 200);
        // Found by review: with every step left undone, the area still has
        // the dropped steps' changes, so it isn't back at the start (which
        // would look "saved" for a file just opened).
        while h.undo(&mut s).is_some() {}
        assert_ne!(h.state(), 0);
        // The newest step is always kept, however big.
        let mut tiny = History::<()>::with_limit(1);
        tiny.apply(&mut s, "Big", vec![set(id, 1.0)], None, (), ())
            .unwrap();
        assert_eq!(tiny.undo_len(), 1);
        tiny.clear();
        assert_eq!(tiny.undo_len(), 0);
    }

    #[test]
    fn a_failed_step_changes_nothing() {
        let (mut s, id) = lamp();
        let before = s.clone();
        let mut h = History::<()>::default();
        let bad = Edit::Despawn {
            root: EntityId::random(),
        };
        assert!(
            h.apply(&mut s, "Bad", vec![set(id, 1.0), bad], None, (), ())
                .is_err()
        );
        assert_eq!(s, before);
        assert_eq!(h.undo_len(), 0);
    }

    /// A small random number generator, so the test needs no crate.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn pick<T: Copy>(&mut self, items: &[T]) -> T {
            items[(self.next() % items.len() as u64) as usize]
        }
    }

    /// A random edit, often a bad one, against the entities in `s`.
    fn random_edit(r: &mut Rng, s: &Area) -> Edit {
        let mut ids: Vec<EntityId> = s.iter().map(|(id, _)| id).collect();
        ids.push(EntityId::random());
        let entity = r.pick(&ids);
        let component = r.pick(&["Light", "Transform", "Health"]).to_string();
        match r.next() % 7 {
            0 => Edit::SetField {
                entity,
                component,
                field: r.pick(&["strength", "range", "position"]).into(),
                value: (!r.next().is_multiple_of(4)).then(|| Value::Float((r.next() % 100) as f64)),
            },
            1 => Edit::AddComponent {
                entity,
                component,
                data: ComponentData::default(),
            },
            2 => Edit::RemoveComponent { entity, component },
            3 => {
                let mut data = EntityData::new("New").with(&Transform::default());
                data.parent = None;
                Edit::Spawn {
                    entities: vec![(EntityId::random(), data)],
                }
            }
            4 => Edit::Despawn { root: entity },
            5 => Edit::Rename {
                entity,
                name: format!("N{}", r.next() % 10),
            },
            _ => Edit::Reparent {
                entity,
                parent: (!r.next().is_multiple_of(3)).then(|| r.pick(&ids)),
                order: format!("a{}", r.next() % 5),
            },
        }
    }

    #[test]
    fn random_edits_all_undo_back_to_the_start() {
        for seed in 1..=40u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let (mut s, _) = lamp();
            let mut h = History::<()>::default();
            let start = s.clone();
            let mut states = vec![s.clone()];
            for _ in 0..60 {
                let edit = random_edit(&mut r, &s);
                if h.apply(&mut s, "Random", vec![edit], None, (), ()).is_ok() {
                    states.push(s.clone());
                }
            }
            // Undo walks back through every state, redo forward again.
            for expected in states.iter().rev().skip(1) {
                h.undo(&mut s).unwrap().unwrap();
                assert_eq!(&s, expected, "seed {seed}");
            }
            assert_eq!(s, start);
            for expected in states.iter().skip(1) {
                h.redo(&mut s).unwrap().unwrap();
                assert_eq!(&s, expected, "seed {seed}");
            }
        }
    }

    /// Random steps, undos and redos on a held history while "the game"
    /// adds and removes entities of its own beside them, then the copy from
    /// before follows: it has none of the game's entities, is exactly the
    /// copy with the same steps made on it alone, and undoes back to where
    /// it was.
    #[test]
    fn following_random_steps_never_brings_what_wasnt_a_step() {
        for seed in 1..=60u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let (mut s, _) = lamp();
            let mut h = History::<()>::default();
            for _ in 0..8 {
                let edit = random_edit(&mut r, &s);
                let _ = h.apply(&mut s, "Before", vec![edit], None, (), ());
            }
            if seed % 3 == 0 {
                let _ = h.undo(&mut s);
            }
            let (mut kept_area, mut kept) = (s.clone(), h.clone());
            // The same steps, made on a copy the game never touches: what
            // following should come to. `done` and `undone` say which of
            // the played history's undo and redo steps it has.
            let (mut alone_area, mut alone) = (s.clone(), h.clone());
            let mut done = vec![true; h.undo_len()];
            let mut undone = vec![true; usize::from(h.redo_label().is_some())];
            let mut exact = true;
            h.hold();
            let mut made = 0;
            for _ in 0..80 {
                match r.next() % 8 {
                    // The game: a new entity of its own, or one taken out.
                    0 | 1 => {
                        made += 1;
                        let _ = s.apply(spawn(game_id(made), "Game's", None));
                    }
                    2 => {
                        let ids: Vec<EntityId> = s.iter().map(|(id, _)| id).collect();
                        if !ids.is_empty() {
                            let _ = s.apply(Edit::Despawn { root: r.pick(&ids) });
                        }
                    }
                    3 => match h.undo(&mut s) {
                        Some(Ok(_)) => {
                            let had = done.pop().unwrap();
                            if had {
                                alone.undo(&mut alone_area).unwrap().unwrap();
                            }
                            undone.push(had);
                        }
                        // The game took away what the step needs: it's
                        // gone from the history, which the copy alone
                        // can't mirror.
                        Some(Err(_)) => exact = false,
                        None => {}
                    },
                    4 => match h.redo(&mut s) {
                        Some(Ok(_)) => {
                            let had = undone.pop().unwrap();
                            if had {
                                alone.redo(&mut alone_area).unwrap().unwrap();
                            }
                            done.push(had);
                        }
                        Some(Err(_)) => exact = false,
                        None => {}
                    },
                    _ => {
                        let ids: Vec<EntityId> = s.iter().map(|(id, _)| id).collect();
                        let edit = if r.next().is_multiple_of(5) && !ids.is_empty() {
                            spawn(EntityId::random(), "Under", Some(r.pick(&ids)))
                        } else {
                            random_edit(&mut r, &s)
                        };
                        if h.apply(&mut s, "Since", vec![edit.clone()], None, (), ())
                            .is_ok()
                        {
                            undone.clear();
                            let fits = has_its_parent(&alone_area, &edit)
                                && alone
                                    .apply(&mut alone_area, "Since", vec![edit], None, (), ())
                                    .is_ok();
                            done.push(fits);
                        }
                    }
                }
            }
            let before = (kept_area.clone(), kept.undo_len());
            let followed = kept.follow(&mut kept_area, &h);
            for n in 1..=made {
                assert!(!kept_area.contains(game_id(n)), "seed {seed}");
            }
            for (_, data) in kept_area.iter() {
                assert!(
                    data.parent.is_none_or(|p| kept_area.contains(p)),
                    "seed {seed}"
                );
            }
            if exact {
                assert_eq!(kept_area, alone_area, "seed {seed}");
                assert_eq!(kept.undo_len(), alone.undo_len(), "seed {seed}");
            }
            // What was made undoes and redoes cleanly.
            let after = kept_area.clone();
            for _ in 0..followed.made.len() {
                kept.undo(&mut kept_area).unwrap().unwrap();
            }
            if kept.undo_len() == before.1 {
                assert_eq!(kept_area, before.0, "seed {seed}");
            }
            for _ in 0..followed.made.len() {
                kept.redo(&mut kept_area).unwrap().unwrap();
            }
            assert_eq!(kept_area, after, "seed {seed}");
        }
    }
}
