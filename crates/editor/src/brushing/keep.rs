//! Keeping a shaped brush quickly (Tools 1a, B5): its whole check
//! (`mesh::check_whole`) and what keeping it writes and draws ([`Kept`])
//! are worked out side by side on two cores ([`both`]), and while an
//! extrude or inset follows the mouse, ahead of the click, on a worker,
//! for the brush as it is now ([`Ahead`]): the frame that keeps it then
//! only makes the undo step.
//!
//! Only the newest piece of work ahead for a pull is done: one asked for
//! before a newer one starts is skipped, so a fast mouse never piles
//! work up. Each pull counts its own ([`Newest`]).

use crate::brushes::BrushModel;
use crate::brushing::parts::PartCache;
use level::builtin::Brush;
use mesh::{Broken, Mesh, VertexId};
use reflect::Component;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

/// What keeping a shaped brush writes and draws, worked out beside its
/// whole check: its values, the mesh any later read of them gives (the
/// same one, unless a point or face left a gap), its model, and the
/// picking geometry editing goes on with (with the mesh's topology,
/// which drawing it needs too).
pub(crate) struct Kept {
    pub(crate) data: reflect::ComponentData,
    pub(crate) read: Arc<Mesh>,
    pub(crate) model: Arc<BrushModel>,
    pub(crate) parts: Arc<PartCache>,
}

impl Kept {
    /// For `written`, drawn by `made` (a pull's last frame) if there is
    /// one and no ids change. Known whole (the check beside it says so),
    /// so neither the read nor the model looks at it whole again.
    fn of(written: &Arc<Mesh>, made: Option<Arc<BrushModel>>) -> Option<Self> {
        let data = Brush {
            mesh: Arc::clone(written),
            ..Brush::default()
        }
        .to_data();
        let read = if has_gaps(written) {
            Arc::new(level::builtin::compacted(written)?)
        } else {
            Arc::clone(written)
        };
        let model = match made.filter(|_| Arc::ptr_eq(written, &read)) {
            Some(model) => model.kept(false),
            None => BrushModel::passing(&read).kept(false),
        };
        let _ = read.topology();
        let parts = Arc::new(PartCache::of(Arc::clone(&read), model.key));
        Some(Self {
            data,
            read,
            model: Arc::new(model),
            parts,
        })
    }
}

/// A shaped brush checked and written: the mesh, whether it is whole,
/// and what keeping it writes and draws (`None` if it can't be).
pub(crate) struct Checked {
    pub(crate) written: Arc<Mesh>,
    pub(crate) whole: Result<(), Broken>,
    pub(crate) kept: Option<Kept>,
}

/// `written` checked whole around `around` and written, side by side.
pub(crate) fn both(
    written: Arc<Mesh>,
    around: &[VertexId],
    made: Option<Arc<BrushModel>>,
) -> Checked {
    let (whole, kept) = rayon::join(
        || mesh::check_whole(&written, around),
        || Kept::of(&written, made),
    );
    Checked {
        written,
        whole,
        kept,
    }
}

/// Whether `mesh` has gaps left by deleted points or faces, which
/// writing its lists closes up.
pub(crate) fn has_gaps(mesh: &Mesh) -> bool {
    mesh.point_slot_count() != mesh.vertex_count()
        || mesh.face_layout().0.len().saturating_sub(1) != mesh.face_count()
}

/// How many pieces of work a pull has asked for ahead: a piece that finds
/// a newer one asked for skips itself.
pub(crate) type Newest = Arc<AtomicU64>;

/// A keep worked out ahead on a worker, for a pull at `metres`.
#[derive(Debug)]
pub(crate) struct Ahead {
    /// How far the pull was when it was asked for.
    pub(crate) metres: f32,
    /// Waiting, under way, or called off ([`WAITING`] and the rest).
    state: Arc<AtomicU8>,
    answer: Mutex<mpsc::Receiver<Checked>>,
}

/// Not started by a worker yet.
const WAITING: u8 = 0;
/// A worker is doing it.
const UNDER_WAY: u8 = 1;
/// Called off before a worker started it.
const CALLED_OFF: u8 = 2;

impl std::fmt::Debug for Checked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checked")
            .field("whole", &self.whole)
            .field("kept", &self.kept.is_some())
            .finish()
    }
}

impl Ahead {
    /// Starts checking and writing `written` (a pull at `metres`, moving
    /// `around`, drawn by `made`) on a worker, as the newest of `newest`'s.
    pub(crate) fn start(
        newest: &Newest,
        (written, around): (Arc<Mesh>, Vec<VertexId>),
        made: Arc<BrushModel>,
        metres: f32,
    ) -> Self {
        let mine = newest.fetch_add(1, Ordering::Relaxed) + 1;
        let newest = Arc::clone(newest);
        let state = Arc::new(AtomicU8::new(WAITING));
        let worker_state = Arc::clone(&state);
        let (send, answer) = mpsc::channel();
        rayon::spawn(move || {
            let begun = worker_state
                .compare_exchange(WAITING, UNDER_WAY, Ordering::AcqRel, Ordering::Acquire)
                .is_ok();
            if !begun || newest.load(Ordering::Relaxed) != mine {
                return;
            }
            let _ = send.send(both(written, &around, Some(made)));
        });
        Self {
            metres,
            state,
            answer: Mutex::new(answer),
        }
    }

    /// Its answer: at once if it is done, or the rest of the work waited
    /// for if a worker is doing it (less than doing it all here would
    /// take). `None` if no worker has started it (it is called off, and
    /// the caller does the work itself rather than wait behind older
    /// work) or it was skipped for a newer one.
    pub(crate) fn answer(&self) -> Option<Checked> {
        let answer = self.answer.lock().ok()?;
        if let Ok(done) = answer.try_recv() {
            return Some(done);
        }
        let called_off = self
            .state
            .compare_exchange(WAITING, CALLED_OFF, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if called_off {
            return None;
        }
        answer.recv().ok()
    }
}

#[cfg(test)]
#[path = "keep_tests.rs"]
mod tests;
