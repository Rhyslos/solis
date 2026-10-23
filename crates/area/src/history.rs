//! Undo and redo: steps of edits, with the selection each step had
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//!
//! The selection type is the caller's (`S`): the area crate doesn't know
//! what the editor selects, only that undo brings it back.

use crate::document::Area;
use crate::edit::{Edit, EditError};
use std::collections::VecDeque;

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

impl<S> Step<S> {
    fn measure(&mut self) {
        self.bytes = self.label.len()
            + self
                .undo
                .iter()
                .chain(&self.redo)
                .map(Edit::bytes)
                .sum::<usize>();
    }

    /// Whether undoing it would change nothing: every edit in it put back
    /// the value it found (a drag that ended where it started).
    fn changes_nothing(&self) -> bool {
        self.undo.len() == self.redo.len()
            && self.redo.iter().all(|edit| match edit {
                Edit::SetField { .. } | Edit::Rename { .. } => {
                    self.undo.iter().any(|undo| undo == edit)
                }
                _ => false,
            })
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
    undo: VecDeque<Step<S>>,
    redo: Vec<Step<S>>,
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
}

impl<S: Clone> Default for History<S> {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT_BYTES)
    }
}

impl<S: Clone> History<S> {
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
        let newest = if merging { self.undo.pop_back() } else { None };
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
        self.undo.push_back(step);
        while self.bytes > self.limit && self.undo.len() > 1 {
            if let Some(old) = self.undo.pop_front() {
                self.bytes -= old.bytes;
                self.dropped = old.serial;
            }
        }
    }

    /// Undoes the newest step, returning its label and the selection it
    /// started with; `None` if there's nothing to undo.
    pub fn undo(&mut self, area: &mut Area) -> Option<Result<Undone<S>, EditError>> {
        let mut step = self.undo.pop_back()?;
        self.bytes -= step.bytes;
        self.open = false;
        match area.apply_all(step.undo.clone()) {
            Ok(redo) => {
                step.redo = redo;
                let done = Undone {
                    label: step.label.clone(),
                    selection: step.before.clone(),
                };
                self.redo.push(step);
                Some(Ok(done))
            }
            Err(why) => Some(Err(why)),
        }
    }

    /// Redoes the step undone last, returning its label and the selection
    /// it ended with; `None` if there's nothing to redo.
    pub fn redo(&mut self, area: &mut Area) -> Option<Result<Undone<S>, EditError>> {
        let mut step = self.redo.pop()?;
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

/// Merged steps of field changes keep only the first value each field had
/// and the last it was given, so a long drag stays small.
fn compact<S>(step: &mut Step<S>) {
    let target = |edit: &Edit| match edit {
        Edit::SetField {
            entity,
            component,
            field,
            ..
        } => Some((*entity, Some((component.clone(), field.clone())))),
        Edit::Rename { entity, .. } => Some((*entity, None)),
        _ => None,
    };
    if step
        .redo
        .iter()
        .chain(&step.undo)
        .any(|e| target(e).is_none())
    {
        return;
    }
    // Redo: the last edit of each target wins. Undo: the last edit in the
    // undo list restores the first value, so it wins there too.
    let keep_last = |edits: &mut Vec<Edit>| {
        let mut seen = Vec::new();
        let mut kept: Vec<Edit> = Vec::new();
        for edit in edits.drain(..).rev() {
            let t = target(&edit);
            if !seen.contains(&t) {
                seen.push(t);
                kept.push(edit);
            }
        }
        kept.reverse();
        *edits = kept;
    };
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
}
