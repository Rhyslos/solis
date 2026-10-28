//! Editing an animator file (Phase 9, D3b): the animator tab's model. The
//! file as it's being edited, what's picked in its graph, undo and redo,
//! and the preview: the animator run on the parameters set beside the
//! graph, playing its sprite's clips. Drawn by `animator_view.rs`.
//!
//! Every change goes through a method here that keeps the file whole
//! (names unique, transitions only between states that exist, conditions
//! on parameters that exist) and makes one undo step.

use animation::animator_file::{self, MAX_PARAMETERS, MAX_STATES, MAX_TRANSITIONS};
use animation::{
    Animator, AnimatorFile, ClipSet, Condition, Facing, ParamValue, Parameter, ParameterKind,
    Plays, Run, State, Transition,
};
use reflect::Id;
use std::collections::BTreeSet;

/// The most undo steps kept.
const MAX_UNDO: usize = 200;
/// A state's box, in graph units.
pub(crate) const BOX_SIZE: [f32; 2] = [150.0, 74.0];

/// What's picked in the graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Picked {
    /// Nothing.
    #[default]
    Nothing,
    /// These states (and Any state, as `None`).
    States(BTreeSet<Option<Id>>),
    /// A transition.
    Transition(Id),
}

/// What a parameter of a given kind is called when it's made.
fn standard_name(kind: ParameterKind) -> &'static str {
    match kind {
        ParameterKind::Number(_) => "speed",
        ParameterKind::Switch(_) => "moving",
        ParameterKind::Trigger => "use",
        ParameterKind::Direction(_) => "facing",
    }
}

/// The animator file open in the animator tab.
#[derive(Debug, Clone)]
pub(crate) struct AnimatorEditor {
    /// Its path inside the assets.
    pub(crate) path: String,
    /// The file as it is now.
    pub(crate) file: AnimatorFile,
    /// What's picked.
    pub(crate) picked: Picked,
    /// The graph canvas's place and drag.
    pub(crate) canvas: crate::graph_canvas::GraphCanvas,
    undo: Vec<AnimatorFile>,
    redo: Vec<AnimatorFile>,
    /// The file ready to run, for the preview.
    pub(crate) running: Animator,
    /// The preview's run: its state and the parameters set beside the
    /// graph.
    pub(crate) preview: Run,
    /// The clip the preview plays, and how far in (seconds).
    pub(crate) preview_clip: String,
    pub(crate) preview_time: f64,
    /// Time not yet stepped (the preview steps in the game's ticks).
    pending: f64,
    /// The versions saved lately: the app reading one of these back is
    /// Solis's own save (perhaps an older one, arriving late), not a change
    /// made on disk.
    written: std::collections::VecDeque<AnimatorFile>,
    /// Whether the app has read the file from disk since it was opened (a
    /// file made just now isn't there until it has).
    pub(crate) on_disk: bool,
}

impl AnimatorEditor {
    /// Opens `file` (at `path`).
    pub(crate) fn new(path: String, file: AnimatorFile) -> Self {
        let running = Animator::new(&file);
        let preview = running.start("");
        Self {
            path,
            file,
            picked: Picked::Nothing,
            canvas: crate::graph_canvas::GraphCanvas::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            running,
            preview,
            preview_clip: String::new(),
            preview_time: 0.0,
            pending: 0.0,
            written: std::collections::VecDeque::new(),
            on_disk: false,
        }
    }

    /// Notes the file as it's being saved now.
    pub(crate) fn saving(&mut self) {
        self.written.push_back(self.file.clone());
        if self.written.len() > 16 {
            self.written.pop_front();
        }
    }

    /// Whether `file`, read from disk, is one Solis saved (or the file as
    /// it is).
    pub(crate) fn is_own(&self, file: &AnimatorFile) -> bool {
        *file == self.file || self.written.contains(file)
    }

    /// The file as text, to save.
    pub(crate) fn text(&self) -> String {
        animator_file::write(&self.file)
    }

    /// Makes one undo step from the file as it was before `change`, if
    /// `change` changed it.
    fn change(&mut self, change: impl FnOnce(&mut AnimatorFile) -> bool) -> bool {
        let before = self.file.clone();
        if !change(&mut self.file) || self.file == before {
            self.file = before;
            return false;
        }
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.file_changed();
        true
    }

    /// After any change to the file: run it again, keeping the preview's
    /// place by name, and forget picks of what's gone.
    fn file_changed(&mut self) {
        let running = Animator::new(&self.file);
        self.preview = running.carry(&self.running, &self.preview);
        self.running = running;
        let file = &self.file;
        self.picked = match std::mem::take(&mut self.picked) {
            Picked::States(ids) => {
                let kept: BTreeSet<Option<Id>> = ids
                    .into_iter()
                    .filter(|id| id.is_none_or(|id| file.state(id).is_some()))
                    .collect();
                if kept.is_empty() {
                    Picked::Nothing
                } else {
                    Picked::States(kept)
                }
            }
            Picked::Transition(id) if file.transitions.iter().any(|t| t.id == id) => {
                Picked::Transition(id)
            }
            _ => Picked::Nothing,
        };
    }

    /// Undoes the last change. Whether there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.file, before));
        self.file_changed();
        true
    }

    /// Redoes the last change undone. Whether there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.file, after));
        self.file_changed();
        true
    }

    /// The file changed on disk (edited by hand, or by Git): taken as it
    /// is, as one undo step.
    pub(crate) fn reload(&mut self, file: AnimatorFile) {
        self.change(|f| {
            *f = file;
            true
        });
    }

    // Picking.

    /// Picks one state (`None`: Any state), or adds it to (or takes it
    /// from) what's picked.
    pub(crate) fn pick_state(&mut self, id: Option<Id>, add: bool) {
        let mut set = match (&self.picked, add) {
            (Picked::States(s), true) => s.clone(),
            _ => BTreeSet::new(),
        };
        if !set.remove(&id) || !add {
            set.insert(id);
        }
        self.picked = if set.is_empty() {
            Picked::Nothing
        } else {
            Picked::States(set)
        };
    }

    /// The one state picked, if exactly one (and not Any state).
    pub(crate) fn picked_state(&self) -> Option<&State> {
        match &self.picked {
            Picked::States(s) if s.len() == 1 => s
                .iter()
                .next()
                .copied()
                .flatten()
                .and_then(|id| self.file.state(id)),
            _ => None,
        }
    }

    /// The transition picked.
    pub(crate) fn picked_transition(&self) -> Option<&Transition> {
        match &self.picked {
            Picked::Transition(id) => self.file.transitions.iter().find(|t| t.id == *id),
            _ => None,
        }
    }

    // States.

    /// Adds a state at `place` (its middle), picked. The first state made
    /// is where the animator starts.
    pub(crate) fn add_state(&mut self, place: [f32; 2]) -> bool {
        let id = Id::random();
        let done = self.change(|f| {
            if f.states.len() >= MAX_STATES {
                return false;
            }
            let name = animator_file::free_state_name(f, "State");
            f.states.push(State {
                id,
                name,
                place: [place[0] - BOX_SIZE[0] / 2.0, place[1] - BOX_SIZE[1] / 2.0],
                plays: Plays::Clip(String::new()),
                speed: 1.0,
                speed_by: String::new(),
            });
            if f.entry.is_none() {
                f.entry = Some(id);
            }
            true
        });
        if done {
            self.picked = Picked::States([Some(id)].into());
        }
        done
    }

    /// Removes what's picked: states (with their transitions), or a
    /// transition. Any state stays.
    pub(crate) fn delete_picked(&mut self) -> bool {
        let picked = self.picked.clone();
        self.change(|f| match picked {
            Picked::States(ids) => {
                let gone: BTreeSet<Id> = ids.into_iter().flatten().collect();
                f.states.retain(|s| !gone.contains(&s.id));
                f.transitions.retain(|t| {
                    !gone.contains(&t.to) && t.from.is_none_or(|from| !gone.contains(&from))
                });
                if f.entry.is_some_and(|e| gone.contains(&e)) {
                    f.entry = f.states.first().map(|s| s.id);
                }
                true
            }
            Picked::Transition(id) => {
                f.transitions.retain(|t| t.id != id);
                true
            }
            Picked::Nothing => false,
        })
    }

    /// Moves states (`None`: Any state) by `by` graph units.
    pub(crate) fn move_states(&mut self, ids: &[Option<Id>], by: [f32; 2]) -> bool {
        let fit = |v: f32| v.clamp(-animator_file::MAX_PLACE, animator_file::MAX_PLACE);
        self.change(|f| {
            for id in ids {
                let place = match id {
                    None => &mut f.any_place,
                    Some(id) => match f.states.iter_mut().find(|s| s.id == *id) {
                        Some(s) => &mut s.place,
                        None => continue,
                    },
                };
                *place = [fit(place[0] + by[0]), fit(place[1] + by[1])];
            }
            true
        })
    }

    /// Renames a state (to a free name). Whether it changed.
    pub(crate) fn rename_state(&mut self, id: Id, name: &str) -> bool {
        let name = animation::frames_file::clean_name(name);
        if name.is_empty() {
            return false;
        }
        self.change(|f| {
            if f.state(id).is_none_or(|s| s.name == name) {
                return false;
            }
            let free = animator_file::free_state_name(f, &name);
            if let Some(s) = f.states.iter_mut().find(|s| s.id == id) {
                s.name = free;
            }
            true
        })
    }

    /// Makes `id` the state the animator starts in.
    pub(crate) fn set_entry(&mut self, id: Id) -> bool {
        self.change(|f| {
            let ok = f.state(id).is_some();
            if ok {
                f.entry = Some(id);
            }
            ok
        })
    }

    fn with_state(&mut self, id: Id, edit: impl FnOnce(&mut State, &[Parameter]) -> bool) -> bool {
        self.change(|f| {
            let params = f.parameters.clone();
            f.states
                .iter_mut()
                .find(|s| s.id == id)
                .is_some_and(|s| edit(s, &params))
        })
    }

    /// Plays one clip (`by_direction` false) or a clip by direction. A
    /// clip by direction uses the first direction parameter, made if
    /// there's none ("facing"); its clips start as the one clip, facing
    /// down.
    pub(crate) fn set_by_direction(&mut self, id: Id, by_direction: bool) -> bool {
        self.change(|f| {
            let by = match f
                .parameters
                .iter()
                .find(|p| matches!(p.kind, ParameterKind::Direction(_)))
            {
                Some(p) => p.name.clone(),
                None if by_direction => {
                    if f.parameters.len() >= MAX_PARAMETERS {
                        return false;
                    }
                    let name = free_parameter_name(f, "facing");
                    f.parameters.push(Parameter {
                        id: Id::random(),
                        name: name.clone(),
                        kind: ParameterKind::Direction(Facing::Down),
                    });
                    name
                }
                None => String::new(),
            };
            let Some(s) = f.states.iter_mut().find(|s| s.id == id) else {
                return false;
            };
            s.plays = match (&s.plays, by_direction) {
                (Plays::Clip(c), true) => Plays::ByDirection {
                    by,
                    clips: vec![(Facing::Down, c.clone())],
                },
                (Plays::ByDirection { clips, .. }, false) => Plays::Clip(
                    clips
                        .iter()
                        .find(|(d, _)| *d == Facing::Down)
                        .or(clips.first())
                        .map(|(_, c)| c.clone())
                        .unwrap_or_default(),
                ),
                _ => return false,
            };
            true
        })
    }

    /// Sets the one clip a state plays.
    pub(crate) fn set_clip(&mut self, id: Id, clip: &str) -> bool {
        let clip = animation::frames_file::clean_name(clip);
        self.with_state(id, |s, _| {
            s.plays = Plays::Clip(clip);
            true
        })
    }

    /// Sets the clip a state plays facing `facing` (adding the direction).
    pub(crate) fn set_direction_clip(&mut self, id: Id, facing: Facing, clip: &str) -> bool {
        let clip = animation::frames_file::clean_name(clip);
        self.with_state(id, |s, _| {
            let Plays::ByDirection { clips, .. } = &mut s.plays else {
                return false;
            };
            match clips.iter_mut().find(|(d, _)| *d == facing) {
                Some((_, c)) => *c = clip,
                None => clips.push((facing, clip)),
            }
            true
        })
    }

    /// Adds the next direction a state has no clip for (the four straight
    /// ones, then the diagonals), with the clip of the one nearest it.
    pub(crate) fn add_direction(&mut self, id: Id) -> bool {
        self.with_state(id, |s, _| {
            let Plays::ByDirection { clips, .. } = &mut s.plays else {
                return false;
            };
            let Some(next) = Facing::ALL
                .into_iter()
                .find(|f| !clips.iter().any(|(d, _)| d == f))
            else {
                return false;
            };
            let v = next.vector();
            let near = clips
                .iter()
                .max_by(|a, b| {
                    let dot = |f: Facing| f.vector()[0] * v[0] + f.vector()[1] * v[1];
                    dot(a.0).total_cmp(&dot(b.0))
                })
                .map(|(_, c)| c.clone())
                .unwrap_or_default();
            clips.push((next, near));
            true
        })
    }

    /// Takes a direction from a state's clips.
    pub(crate) fn remove_direction(&mut self, id: Id, facing: Facing) -> bool {
        self.with_state(id, |s, _| {
            let Plays::ByDirection { clips, .. } = &mut s.plays else {
                return false;
            };
            let before = clips.len();
            clips.retain(|(d, _)| *d != facing);
            clips.len() != before
        })
    }

    /// Sets which direction parameter picks a state's clip.
    pub(crate) fn set_direction_by(&mut self, id: Id, by: &str) -> bool {
        self.with_state(id, |s, params| {
            let Plays::ByDirection { by: b, .. } = &mut s.plays else {
                return false;
            };
            if !params
                .iter()
                .any(|p| p.name == by && matches!(p.kind, ParameterKind::Direction(_)))
            {
                return false;
            }
            *b = by.to_owned();
            true
        })
    }

    /// Sets a state's speed (within 0 to 16).
    pub(crate) fn set_state_speed(&mut self, id: Id, speed: f32) -> bool {
        if !speed.is_finite() {
            return false;
        }
        let (lo, hi) = animator_file::SPEED_RANGE;
        self.with_state(id, |s, _| {
            s.speed = speed.clamp(lo, hi);
            true
        })
    }

    /// Sets the number parameter a state's speed is multiplied by ("" for
    /// none).
    pub(crate) fn set_speed_by(&mut self, id: Id, by: &str) -> bool {
        self.with_state(id, |s, params| {
            if !by.is_empty()
                && !params
                    .iter()
                    .any(|p| p.name == by && matches!(p.kind, ParameterKind::Number(_)))
            {
                return false;
            }
            s.speed_by = by.to_owned();
            true
        })
    }

    // Transitions.

    /// Adds a transition (`from` `None`: from any state), picked, with no
    /// conditions yet (it's taken at once until it has some).
    pub(crate) fn join(&mut self, from: Option<Id>, to: Id) -> bool {
        if from == Some(to) {
            return false;
        }
        let id = Id::random();
        let done = self.change(|f| {
            if f.transitions.len() >= MAX_TRANSITIONS
                || f.state(to).is_none()
                || from.is_some_and(|from| f.state(from).is_none())
            {
                return false;
            }
            f.transitions.push(Transition {
                id,
                from,
                to,
                when: Vec::new(),
            });
            true
        });
        if done {
            self.picked = Picked::Transition(id);
        }
        done
    }

    fn with_transition(&mut self, id: Id, edit: impl FnOnce(&mut Transition) -> bool) -> bool {
        self.change(|f| {
            f.transitions
                .iter_mut()
                .find(|t| t.id == id)
                .is_some_and(edit)
        })
    }

    /// Adds a condition to a transition: on its first parameter, or
    /// "finished" when there are none.
    pub(crate) fn add_condition(&mut self, id: Id) -> bool {
        let first = self.file.parameters.first().map(condition_on);
        self.with_transition(id, |t| {
            if t.when.len() >= animator_file::MAX_CONDITIONS {
                return false;
            }
            t.when.push(first.unwrap_or(Condition::Finished));
            true
        })
    }

    /// Sets a transition's `n`th condition.
    pub(crate) fn set_condition(&mut self, id: Id, n: usize, condition: Condition) -> bool {
        self.with_transition(id, |t| match t.when.get_mut(n) {
            Some(c) => {
                *c = condition;
                true
            }
            None => false,
        })
    }

    /// Takes away a transition's `n`th condition.
    pub(crate) fn remove_condition(&mut self, id: Id, n: usize) -> bool {
        self.with_transition(id, |t| {
            if n < t.when.len() {
                t.when.remove(n);
                true
            } else {
                false
            }
        })
    }

    /// Moves a transition one place earlier (tried sooner) or later among
    /// those leaving the same state.
    pub(crate) fn move_transition(&mut self, id: Id, earlier: bool) -> bool {
        self.change(|f| {
            let Some(at) = f.transitions.iter().position(|t| t.id == id) else {
                return false;
            };
            let from = f.transitions[at].from;
            let swap = if earlier {
                f.transitions[..at].iter().rposition(|t| t.from == from)
            } else {
                f.transitions[at + 1..]
                    .iter()
                    .position(|t| t.from == from)
                    .map(|i| i + at + 1)
            };
            match swap {
                Some(other) => {
                    f.transitions.swap(at, other);
                    true
                }
                None => false,
            }
        })
    }

    // Parameters.

    /// Adds a parameter of `kind`, with a free name.
    pub(crate) fn add_parameter(&mut self, kind: ParameterKind) -> bool {
        self.change(|f| {
            if f.parameters.len() >= MAX_PARAMETERS {
                return false;
            }
            let name = free_parameter_name(f, standard_name(kind));
            f.parameters.push(Parameter {
                id: Id::random(),
                name,
                kind,
            });
            true
        })
    }

    /// Renames a parameter, and everything that uses it. Whether it was
    /// renamed (the name must be a parameter's name, and free).
    pub(crate) fn rename_parameter(&mut self, old: &str, new: &str) -> bool {
        let Ok(new) = animator_file::parameter_name(new) else {
            return false;
        };
        self.change(|f| {
            if f.parameter(&new).is_some() || f.parameter(old).is_none() {
                return false;
            }
            for p in &mut f.parameters {
                if p.name == old {
                    p.name.clone_from(&new);
                }
            }
            for s in &mut f.states {
                if s.speed_by == old {
                    s.speed_by.clone_from(&new);
                }
                if let Plays::ByDirection { by, .. } = &mut s.plays
                    && by == old
                {
                    by.clone_from(&new);
                }
            }
            for t in &mut f.transitions {
                for c in &mut t.when {
                    if let Condition::Is(n)
                    | Condition::Not(n)
                    | Condition::Above(n, _)
                    | Condition::Below(n, _) = c
                        && n == old
                    {
                        n.clone_from(&new);
                    }
                }
            }
            true
        })
    }

    /// Removes a parameter, and the conditions on it. States played by it
    /// or sped by it are left naming nothing (Problems says so).
    pub(crate) fn delete_parameter(&mut self, name: &str) -> bool {
        self.change(|f| {
            let before = f.parameters.len();
            f.parameters.retain(|p| p.name != name);
            for t in &mut f.transitions {
                t.when.retain(|c| c.parameter() != Some(name));
            }
            for s in &mut f.states {
                if s.speed_by == name {
                    s.speed_by.clear();
                }
            }
            f.parameters.len() != before
        })
    }

    /// Sets what a parameter starts at (its kind stays).
    pub(crate) fn set_parameter_start(&mut self, name: &str, kind: ParameterKind) -> bool {
        self.change(|f| match f.parameters.iter_mut().find(|p| p.name == name) {
            Some(p) if std::mem::discriminant(&p.kind) == std::mem::discriminant(&kind) => {
                p.kind = match kind {
                    ParameterKind::Number(v) if v.is_finite() => ParameterKind::Number(
                        v.clamp(-animator_file::MAX_NUMBER, animator_file::MAX_NUMBER),
                    ),
                    ParameterKind::Number(_) => return false,
                    other => other,
                };
                true
            }
            _ => false,
        })
    }

    /// Sets the sheet whose clips the file names.
    pub(crate) fn set_sheet(&mut self, sheet: Option<reflect::AssetId>) -> bool {
        self.change(|f| {
            f.sheet = sheet;
            true
        })
    }

    // The preview.

    /// Sets a parameter for the preview (not saved).
    pub(crate) fn set_preview(&mut self, name: &str, value: ParamValue) {
        let _ = self.running.set(&mut self.preview, name, value);
    }

    /// Puts the preview back at its start.
    pub(crate) fn restart_preview(&mut self) {
        self.preview = self.running.start("");
        self.preview_clip.clear();
        self.preview_time = 0.0;
        self.pending = 0.0;
    }

    /// Moves the preview on by `seconds`, in the game's ticks: the
    /// animator steps, then its clip plays, as in the running game.
    pub(crate) fn play_preview(&mut self, seconds: f64, clips: Option<&ClipSet>) {
        if !(seconds.is_finite() && seconds > 0.0) {
            return;
        }
        self.pending = (self.pending + seconds).min(0.25);
        while self.pending >= world::TICK {
            self.pending -= world::TICK;
            let clip = clips.and_then(|c| c.chosen(&self.preview_clip));
            let finished = clip
                .and_then(animation::Clip::length_seconds)
                .is_some_and(|l| self.preview_time >= l);
            let changed = self.running.step(&mut self.preview, finished);
            let chosen = self
                .running
                .clip(&self.preview, &self.preview_clip)
                .to_owned();
            if changed {
                self.preview_time = 0.0;
            }
            self.preview_clip = chosen;
            let Some(clip) = clips.and_then(|c| c.chosen(&self.preview_clip)) else {
                continue;
            };
            let step =
                world::TICK * f64::from(self.running.speed(&self.preview)) * f64::from(clip.speed);
            let to = self.preview_time + step;
            self.preview_time = match clip.length_seconds() {
                Some(l) => to.min(l),
                None => {
                    let pass = clip.pass_seconds();
                    if pass > 0.0 { to % pass } else { 0.0 }
                }
            };
        }
    }
}

/// A condition testing `p` as it's most often tested.
fn condition_on(p: &Parameter) -> Condition {
    match p.kind {
        ParameterKind::Number(_) => Condition::Above(p.name.clone(), 0.5),
        ParameterKind::Direction(_) => Condition::Finished,
        _ => Condition::Is(p.name.clone()),
    }
}

/// `name`, or with a number after it if a parameter has it already.
fn free_parameter_name(file: &AnimatorFile, name: &str) -> String {
    if file.parameter(name).is_none() {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name}{n}"))
        .find(|n| file.parameter(n).is_none())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "animator_editor_tests.rs"]
mod tests;
