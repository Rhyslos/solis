//! The kind table: what a 2D, a 3D or a both project shows (decision
//! 0118, R6; board 08 of `docs/design/ui-rethink/`).
//!
//! Every tab, ribbon control, Create item, component, setting, view and
//! command the editor offers belongs to 2D, to 3D or to both
//! ([`PartKind`]). The functions here are that table, one `match` for
//! each sort of part, so the compiler asks for a row whenever a part is
//! added. Each list the editor draws (the tab bar, a ribbon, a menu,
//! Add component, the settings, the search) asks its part's row once,
//! as it is built, through the same filters (`ribbon::filtered`,
//! `unbuilt::filter`'s neighbour here, [`shown`]); nothing walks the
//! table every frame, and no part checks the project's kind on its own.
//!
//! A project of one kind hides the other kind's parts and nothing else:
//! nothing in the project is deleted or taken out of a level. Something
//! of the hidden kind already in a level is still listed, with a quiet
//! note ([`PartKind::note`]). Both shows everything, as before R6.
//!
//! The editor draws on one thread, so the kind in use is kept there
//! ([`in_use`], set at the start of each frame from the open project),
//! as the interface's look is: parts drawn without the workbench at hand
//! (a menu, a component's card) read it there.

use crate::camera::ViewKind;
use crate::file_pages::{Setting, Tab};
use crate::launcher::{ProjectKind, Starter};
use crate::menu_items::MenuCommand;
use crate::panel::Panel;
use crate::shell::commands::NewFile;
use crate::shell::{RibbonTab, ShellCommand};
use crate::unbuilt::{self, UnbuiltId};
use std::cell::Cell;

/// Which kind of game a part of the editor is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PartKind {
    /// Only 2D games use it: tile maps, their painter, the 2D view.
    TwoD,
    /// Only 3D games use it: models, lights, shape brushes, shading.
    ThreeD,
    /// Both kinds use it. Most of the editor is shared (decision 0097:
    /// anything not purely 2D or 3D is built once, for both).
    Shared,
}

impl PartKind {
    /// "2D only", "3D only", or nothing for a shared part.
    pub(crate) fn only(self) -> Option<&'static str> {
        match self {
            Self::TwoD => Some("2D only"),
            Self::ThreeD => Some("3D only"),
            Self::Shared => None,
        }
    }

    /// The quiet note on something of this kind already in a project of
    /// the other: "3D only: switch the project to Both to edit it".
    pub(crate) fn note(self) -> Option<String> {
        self.only()
            .map(|only| format!("{only}: switch the project to Both to edit it"))
    }

    /// Why a command of this kind can't be done in a project of the
    /// other: "3D only: switch the project to Both to use it".
    pub(crate) fn use_note(self) -> Option<String> {
        self.only()
            .map(|only| format!("{only}: switch the project to Both to use it"))
    }
}

impl ProjectKind {
    /// Whether a project of this kind shows a part of `part`'s kind.
    pub(crate) fn shows(self, part: PartKind) -> bool {
        match (self, part) {
            (Self::Both, _) | (_, PartKind::Shared) => true,
            (Self::TwoD, PartKind::TwoD) | (Self::ThreeD, PartKind::ThreeD) => true,
            (Self::TwoD, PartKind::ThreeD) | (Self::ThreeD, PartKind::TwoD) => false,
        }
    }

    /// What choosing it hides, as Project settings says under Kind
    /// (board 08). The 3D sentence mirrors the board's 2D one.
    pub(crate) fn hides(self) -> &'static str {
        match self {
            Self::TwoD => {
                "2D hides what only 3D uses: the Shading tab, lighting and baking, models, and \
                 the 3D view's camera. Nothing in the project is deleted; switch to Both to see \
                 them again."
            }
            Self::ThreeD => {
                "3D hides what only 2D uses: tile maps and their painter, pixel snapping, and the \
                 2D view. Nothing in the project is deleted; switch to Both to see them again."
            }
            Self::Both => "Both shows everything: 2D and 3D.",
        }
    }

    /// The note Home's ribbon shows the first few times a project of
    /// this kind opens (board 08); none for Both, which hides nothing.
    pub(crate) fn note(self) -> Option<&'static str> {
        match self {
            Self::TwoD => Some("No Shading tab and no Lighting group: they are 3D only"),
            Self::ThreeD => Some("No tile maps and no tile painter: they are 2D only"),
            Self::Both => None,
        }
    }

    /// The view a project of this kind opens its views in, when the one
    /// they show is hidden: the 2D view, or the perspective one.
    pub(crate) fn first_view(self) -> ViewKind {
        match self {
            Self::TwoD => ViewKind::TwoD,
            Self::ThreeD | Self::Both => ViewKind::Perspective,
        }
    }
}

thread_local! {
    static IN_USE: Cell<ProjectKind> = const { Cell::new(ProjectKind::Both) };
}

/// The open project's kind, as the frame being drawn has it.
pub(crate) fn in_use() -> ProjectKind {
    IN_USE.with(Cell::get)
}

/// Makes `kind` the one in use (each frame, from the open project).
pub(crate) fn use_kind(kind: ProjectKind) {
    IN_USE.with(|cell| cell.set(kind));
}

/// Whether a part of `part`'s kind is shown in the project in use.
pub(crate) fn shown(part: PartKind) -> bool {
    in_use().shows(part)
}

/// A tab of the tab bar.
pub(crate) fn tab(tab: RibbonTab) -> PartKind {
    use RibbonTab as T;
    match tab {
        // Materials are for models.
        T::Shading => PartKind::ThreeD,
        // The model viewer: models are 3D.
        T::Model => PartKind::ThreeD,
        // Effects, game screens, sound and animation are built once, for
        // both (decision 0097). A tileset or a picture's crops come with a
        // file the project holds, which opens whatever the kind.
        T::File
        | T::Home
        | T::Library
        | T::Animate
        | T::Ui
        | T::Vfx
        | T::Audio
        | T::Debug
        | T::Build
        | T::Tileset
        | T::Picture => PartKind::Shared,
    }
}

/// One of the editor's commands: a ribbon's control, a list's item, the
/// search's line and a key all run one, so its row decides them all.
pub(crate) fn command(command: ShellCommand) -> PartKind {
    use ShellCommand as C;
    match command {
        C::ImportModel
        | C::ChooseShapeBrush
        | C::EditBrushInsides
        | C::BrushMode(_)
        | C::BrushSolid
        | C::BrushExtrude
        | C::BrushInset
        | C::BrushSplit
        | C::BrushDelete
        // The 3D view's grid, in metres: the 2D view snaps to pixels or
        // tiles instead (Snap to tiles, Pixel perfect).
        | C::SnapToGrid => PartKind::ThreeD,
        C::NewTileMap
        | C::NewInLibrary(NewFile::TileMap | NewFile::Tileset)
        | C::PaintWith(_)
        | C::PaintOnLayer(_)
        | C::PaintExact
        | C::MapMeaning(_)
        | C::ShowValue(_)
        | C::MapChunks
        | C::MapWalls
        | C::TurnCopy(_)
        | C::PutCopyAway
        | C::SnapToTiles
        | C::PixelPerfect => PartKind::TwoD,
        C::Tab(t) => tab(t),
        // Lights are lamps on sprites in 2D too (decision 0092).
        // Turning and scaling snap in both views. A tileset's tools come
        // with its file (its tab is shared). The Library's kind filters
        // mix kinds: Models holds prefabs, Maps holds levels.
        C::CloseProject
        | C::NewLevel
        | C::OpenLevel
        | C::SaveLevel
        | C::SaveLevelAs
        | C::ExportPlaytest
        | C::ProjectSettings
        | C::InputActions
        | C::Preferences
        | C::Undo
        | C::Redo
        | C::Duplicate
        | C::Delete
        | C::Rename
        | C::SelectAll
        | C::SelectNone
        | C::AddEntity
        | C::AddSprite
        | C::AddPointLight
        | C::AddSpotlight
        | C::AddCamera
        | C::AddReverbZone
        | C::AddTriggerZone
        | C::MakePrefab
        | C::SnapAngles
        | C::SnapScale
        | C::ViewportTools
        | C::ViewportSnapping
        | C::ViewportStats
        | C::Play
        | C::Pause
        | C::Step
        | C::Stop
        | C::LoadSave
        | C::PlayAndRecord
        | C::ReplayRecording
        | C::ProfilerCpu
        | C::ProfilerGpu
        | C::ResetLayout
        | C::FoldBottom
        | C::Placing
        | C::ShowHiddenPanels
        | C::HideUnfinished
        | C::CollapseRibbon
        | C::UseTool(_)
        | C::NewInLibrary(_)
        | C::LibraryList
        | C::LibraryFolders
        | C::LibraryFilter(_)
        | C::PlacePicked
        | C::TerrainAdd
        | C::TerrainRemove
        | C::TerrainKind(_)
        | C::SaveTemplate
        | C::TilesetZoom(_)
        | C::CropEveryTile
        | C::CropDelete
        | C::CropsZoom(_)
        | C::ClipNew
        | C::ClipCopy
        | C::ClipDelete
        | C::FramesTakeOut
        | C::ClipDirection(_)
        | C::StateAdd
        | C::StateStartHere
        | C::StateDelete
        | C::PreviewAgain
        | C::ScreenSize(_)
        | C::ScreenText(_)
        | C::SafeArea
        | C::ScreenSnap
        | C::OpenScreenCode
        | C::AddBus
        | C::AddDuckingRule
        | C::RemoveBus
        | C::ClearSolo
        | C::MuteGame
        | C::HeardAtOnce(_)
        | C::OpenExportFolder
        | C::OpenGuide
        | C::AddSampleLevel
        | C::CheckForUpdates
        | C::AutoUpdate
        | C::About
        | C::Exit => PartKind::Shared,
    }
}

/// One of the Level panel's menu items (its Create items among them):
/// all shared today, lights being lamps on sprites in 2D.
pub(crate) fn menu(command: MenuCommand) -> PartKind {
    use MenuCommand as M;
    match command {
        M::ExportPlaytest
        | M::Undo
        | M::Redo
        | M::Duplicate
        | M::Delete
        | M::Rename
        | M::SelectAll
        | M::SelectNone
        | M::AddEntity
        | M::AddPointLight
        | M::AddSpotlight
        | M::AddCamera
        | M::MakePrefab
        | M::ExpandAll
        | M::CollapseAll => PartKind::Shared,
    }
}

/// An unfinished part, by its key in the plan's inventory: shading,
/// the model viewer, baked light, posing and a model's own material and
/// shadows are 3D's; every other is shared.
pub(crate) fn unbuilt(id: UnbuiltId) -> PartKind {
    const THREE_D: [&str; 7] = [
        "shell.tab.shading",
        "shading.",
        "mesh.",
        "home.lighting.",
        "home.tool.pose",
        "home.object.model-",
        "cross.bake-now",
    ];
    let key = unbuilt::get(id).key;
    if THREE_D.iter().any(|start| key.starts_with(start)) {
        PartKind::ThreeD
    } else {
        PartKind::Shared
    }
}

/// A component, by its name: Add component offers it, and its card in
/// Properties is edited, only where its kind is shown. Game code's own
/// components are shared.
pub(crate) fn component(name: &str) -> PartKind {
    match name {
        // Models, shape brushes and 3D physics.
        "Model" | "Brush" | "Collider" | "RigidBody" | "CharacterBody" => PartKind::ThreeD,
        // Tile maps, 2D physics and parallax layers.
        "TileMap" | "Collider2D" | "Parallax" => PartKind::TwoD,
        // "Light" and "Environment" (lamps on sprites and day and night
        // light 2D games too, decision 0092), "Transform", "Camera" (a 2D camera in the 2D
        // view), "Sprite",
        // "Animator", "AudioSource", "Listener", "ReverbZone", "Screen",
        // "TriggerZone" (built once for 2D and 3D, decision 0097), "Tags"
        // (stage 1), and game code's.
        _ => PartKind::Shared,
    }
}

/// The built-in components the table names, so a test can check every
/// built-in has a row ([`component`] reads unknown names as shared).
#[cfg(test)]
pub(crate) const COMPONENTS: [&str; 20] = [
    "Animator",
    "AudioSource",
    "Brush",
    "Camera",
    "CharacterBody",
    "Collider",
    "Collider2D",
    "Environment",
    "Light",
    "Listener",
    "Model",
    "Parallax",
    "ReverbZone",
    "RigidBody",
    "Screen",
    "Sprite",
    "Tags",
    "TileMap",
    "Transform",
    "TriggerZone",
];

/// A setting of Preferences or Project settings.
pub(crate) fn setting(setting: Setting) -> PartKind {
    use Setting as S;
    match setting {
        // The 3D view's grid in metres, its flying camera and its looks.
        S::GridStep | S::CameraSpeed | S::Look | S::Bands | S::Outlines => PartKind::ThreeD,
        // The 2D view's snapping, and Project settings › 2D.
        S::PixelSnap | S::BaseResolution | S::PixelsPerUnit | S::TileSize | S::Scaling => {
            PartKind::TwoD
        }
        S::AutoUpdate
        | S::Hints
        | S::GuideBrowser
        | S::HideUnfinished
        | S::Accent
        | S::Scale
        | S::Density
        | S::ShelfOpensOn
        | S::Layout
        | S::ResetLayout
        | S::SizesPerTab
        | S::ResetSizes
        | S::Stats
        | S::Pills
        | S::Tools
        | S::PreviewAnimation
        | S::AngleStep
        | S::ScaleStep
        | S::Views
        | S::ProjectName
        | S::ProjectKind
        | S::StartsIn
        | S::EngineVersion
        | S::Scripting
        | S::GameName
        | S::Window
        | S::ExportFolder => PartKind::Shared,
    }
}

/// A sub-tab of Preferences or Project settings.
pub(crate) fn settings_tab(tab: Tab) -> PartKind {
    match tab {
        Tab::TwoD => PartKind::TwoD,
        Tab::General
        | Tab::Appearance
        | Tab::Layout
        | Tab::Viewport
        | Tab::Shortcuts
        | Tab::Project
        | Tab::Input
        | Tab::Tags
        | Tab::Languages
        | Tab::Build => PartKind::Shared,
    }
}

/// A view of the viewport: the 2D view is 2D's, the perspective and the
/// flat 3D views (Top, Front, Right) are 3D's camera choices.
pub(crate) fn view(view: ViewKind) -> PartKind {
    match view {
        ViewKind::TwoD => PartKind::TwoD,
        ViewKind::Perspective | ViewKind::Top | ViewKind::Front | ViewKind::Right => {
            PartKind::ThreeD
        }
    }
}

/// A panel: the Map tab paints a tile map; every other is shared (a
/// tileset's panels come with its file).
pub(crate) fn panel(panel: Panel) -> PartKind {
    match panel {
        Panel::Map => PartKind::TwoD,
        _ => PartKind::Shared,
    }
}

/// A starter in New project: the empty project suits every kind, and
/// Learn's examples are 3D.
pub(crate) fn starter(starter: Starter) -> PartKind {
    match starter {
        Starter::Empty => PartKind::Shared,
        Starter::Playground | Starter::TurretSample | Starter::UiSample | Starter::Benchmark => {
            PartKind::ThreeD
        }
    }
}

/// The kind of what an entity is made of: 3D or 2D when it has a
/// component of that kind (and none of the other), shared otherwise.
pub(crate) fn entity(entity: &level::EntityData) -> PartKind {
    let mut kinds = entity.components.keys().map(|name| component(name));
    let three = kinds.clone().any(|k| k == PartKind::ThreeD);
    let two = kinds.any(|k| k == PartKind::TwoD);
    match (two, three) {
        (true, false) => PartKind::TwoD,
        (false, true) => PartKind::ThreeD,
        _ => PartKind::Shared,
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
