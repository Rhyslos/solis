//! The workbench's side of the Library: turning its requests into actions
//! for the app (which changes files), and placing what's dropped on the
//! viewport or the Area panel where it was dropped.

use crate::library::{LibraryKind, LibraryRequest};
use crate::workbench::{Workbench, WorkbenchAction};
use glam::Vec3;
use reflect::{AssetId, EntityId};
use std::path::PathBuf;

impl Workbench {
    /// Carries out something asked for in the Library.
    pub(crate) fn library_request(
        &mut self,
        request: LibraryRequest,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let Some(assets) = self.project.as_ref().map(|p| p.assets.clone()) else {
            if request == LibraryRequest::Import {
                actions.push(WorkbenchAction::ImportModel);
            }
            return;
        };
        let full = |path: &str| -> PathBuf {
            if path.is_empty() {
                assets.clone()
            } else {
                assets.join(path)
            }
        };
        match request {
            LibraryRequest::Place(id) => {
                // A double-click places it where the camera looks, even if
                // an earlier drop of the same asset never arrived.
                self.place_targets.remove(&id);
                actions.push(WorkbenchAction::PlaceAsset(id));
            }
            LibraryRequest::Open(LibraryKind::Area, path) => {
                self.request(WorkbenchAction::OpenAreaFile(full(&path)), actions);
            }
            LibraryRequest::Open(_, path) | LibraryRequest::Show(path) => {
                actions.push(WorkbenchAction::OpenExternally(full(&path)));
            }
            LibraryRequest::Move(from, to) => actions.push(WorkbenchAction::MoveFile {
                from: from.into(),
                to: to.into(),
            }),
            LibraryRequest::NewFolder(parent) => {
                actions.push(WorkbenchAction::NewFolder(parent.into()));
            }
            LibraryRequest::Delete(path) => actions.push(WorkbenchAction::DeleteFile(path.into())),
            LibraryRequest::Import => actions.push(WorkbenchAction::ImportModel),
        }
    }

    /// Places `asset`, dropped at `at` in the world (under `parent`, if it
    /// was dropped onto an entity): it goes there once the app has loaded
    /// it.
    pub(crate) fn place_dropped(&mut self, asset: AssetId, at: Vec3, parent: Option<EntityId>) {
        self.place_targets.insert(asset, (at, parent));
        self.outbox.push(WorkbenchAction::PlaceAsset(asset));
    }

    /// Forgets where `asset` was to go: its import failed, so a later
    /// placing of it doesn't land at an old drop point.
    pub fn placing_failed(&mut self, asset: AssetId) {
        self.place_targets.remove(&asset);
    }

    /// Where a model or assembly being placed goes: the point it was
    /// dropped at (made relative to the entity it goes under) and that
    /// entity, or `None` to place it where the camera looks.
    pub(crate) fn take_place_target(&mut self, asset: AssetId) -> Option<(Vec3, Option<EntityId>)> {
        let (at, parent) = self.place_targets.remove(&asset)?;
        let area = self.document.area();
        let parent = parent.filter(|&p| area.contains(p));
        let local = match parent {
            Some(p) => area.world_matrix(p).inverse().transform_point3(at),
            None => at,
        };
        local.is_finite().then_some((local, parent))
    }
}

#[cfg(test)]
#[path = "workbench_library_tests.rs"]
mod tests;
