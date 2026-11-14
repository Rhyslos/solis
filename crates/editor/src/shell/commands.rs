//! What the editor can do by name: one table for the ribbon's buttons,
//! the File page's items and the search, so each thing has one name, one
//! shortcut and one answer to "can it be done now, and if not, why not?".
//!
//! [`ShellCommand::ALL`] is every command, in the order the search lists
//! them with nothing typed. A command that can't be done now is left out
//! of the search, and drawn faint on the ribbon with its reason as the
//! hover text.

use super::RibbonTab;
use crate::library::Filter;
use crate::menu_items::MenuCommand;
use crate::play::PlayState;
use crate::tile_paint::{PaintTool, Turn};
use crate::viewport::Tool;
use crate::workbench::{Workbench, WorkbenchAction};

/// Why something that replaces or saves the area can't be done while the
/// game runs.
const STOP_FIRST: &str = "Stop the game first";

/// One thing the editor can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ShellCommand {
    NewProject,
    OpenProject,
    RecentProjects,
    NewArea,
    OpenArea,
    SaveArea,
    SaveAreaAs,
    ImportModel,
    ExportPlaytest,
    ProjectSettings,
    InputActions,
    Preferences,
    Undo,
    Redo,
    Duplicate,
    Delete,
    Rename,
    SelectAll,
    SelectNone,
    AddEntity,
    AddSprite,
    AddPointLight,
    AddSpotlight,
    AddCamera,
    AddReverbZone,
    /// Opens the brush list (Tools 1a, B2): Home › Create › Shape, and B
    /// over the 3D or 2D view.
    ChooseShapeBrush,
    /// Starts or ends editing the picked brush's insides (Tools 1a, B3):
    /// Tab, and HOME TOOLS · Brush › Done.
    EditBrushInsides,
    /// Picks points, edges or faces while a brush's insides are edited
    /// (1, 2, 3).
    BrushMode(crate::brushing::edit::Mode),
    /// Makes the brush being edited solid, or not.
    BrushSolid,
    NewTileMap,
    CreateAssembly,
    SnapToGrid,
    SnapAngles,
    SnapScale,
    ViewportTools,
    ViewportSnapping,
    ViewportStats,
    Play,
    Pause,
    Step,
    Stop,
    LoadSave,
    PlayAndRecord,
    ReplayRecording,
    ProfilerCpu,
    ProfilerGpu,
    ResetLayout,
    FoldShelf,
    HideUnfinished,
    CollapseRibbon,
    /// Picks one of the toolbox's tools (the build form's Transform
    /// group, where there is no toolbox).
    UseTool(crate::viewport::Tool),
    /// Makes something in the Library's open folder (its Create list and
    /// Folder).
    NewInLibrary(NewFile),
    /// The Library shows its files as a list (or, off, as tiles).
    LibraryList,
    /// The Library's folder tree is out.
    LibraryFolders,
    /// The Library shows only one group of kinds.
    LibraryFilter(crate::library::Filter),
    /// Places the file picked in the Library where the camera looks, on
    /// Home (Use › Place).
    PlacePicked,
    /// Picks one of a tile map's painting tools (HOME TOOLS · Tile map).
    PaintWith(crate::tile_paint::PaintTool),
    /// Paints on the selected map's layer of this index.
    PaintOnLayer(usize),
    /// Places exact tiles, or autotiled ones again.
    PaintExact,
    /// Shows the selected map's meaning (`true`) or its art.
    MapMeaning(bool),
    /// Lays the selected map's value of this index over it, or stops.
    ShowValue(usize),
    /// Outlines the selected map's chunks, or stops.
    MapChunks,
    /// Shows what's solid on the selected map, or stops.
    MapWalls,
    /// Flips or turns the copy held.
    TurnCopy(crate::tile_paint::Turn),
    /// Puts the copy held away.
    PutCopyAway,
    /// Adds a terrain to the open tileset (LIBRARY TOOLS · Tileset).
    TerrainAdd,
    /// Removes the open tileset's chosen terrain.
    TerrainRemove,
    /// Makes the chosen terrain this kind.
    TerrainKind(tiles::TileKind),
    /// Saves the chosen terrain's kind's template beside the tileset.
    SaveTemplate,
    /// Shows the tileset's picture this many times its size.
    TilesetZoom(u8),
    /// A crop for every grid cell with something drawn in it
    /// (LIBRARY TOOLS · Picture).
    CropEveryTile,
    /// Deletes the crop picked.
    CropDelete,
    /// Shows the picture this many times its size.
    CropsZoom(u8),
    /// A new clip in the open clips file (Animate › Clip).
    ClipNew,
    /// A copy of the chosen clip.
    ClipCopy,
    /// Deletes the chosen clip (or forgets what was added to a tag).
    ClipDelete,
    /// Takes the frames picked out of the chosen clip.
    FramesTakeOut,
    /// Plays the chosen clip in the direction of this index of
    /// `ribbons::animate::DIRECTIONS`.
    ClipDirection(u8),
    /// Adds a state to the open animator (Animate › State).
    StateAdd,
    /// Makes the state picked the one the animator starts in.
    StateStartHere,
    /// Deletes the state or transition picked.
    StateDelete,
    /// Starts the animator's preview again.
    PreviewAgain,
    /// Shows the screen on the canvas at the size of this preset of
    /// `ui::presets::PRESETS` (UI › Screen).
    ScreenSize(u8),
    /// Shows the screen's text at this size, in percent.
    ScreenText(u16),
    /// Shows the safe area on the canvas, or stops.
    SafeArea,
    /// Snaps sizes and places on the canvas, or stops.
    ScreenSnap,
    /// Opens the screen's file in the code editor.
    OpenScreenCode,
    /// A new bus inside the one the Bus group shows (Audio › Mixer).
    AddBus,
    /// A new ducking rule on the bus shown (Audio › Bus).
    AddDuckingRule,
    /// Removes the bus shown (never Master).
    RemoveBus,
    /// Unsolos every bus (Audio › Monitor).
    ClearSolo,
    /// Mutes the game's sound, or hears it again.
    MuteGame,
    /// How many sounds play at once: this many.
    HeardAtOnce(u16),
    /// Shows the last export's folder (Build › Playtest).
    OpenExportFolder,
    Tab(RibbonTab),
    OpenGuide,
    Learn,
    AddSampleArea,
    MakeSampleGame,
    MakeUiSample,
    MakeBenchmark,
    MakePlayground,
    Updates,
    CheckForUpdates,
    AutoUpdate,
    About,
    Exit,
}

pub(super) use ShellCommand as C;

/// What the Library's Create list and its Folder button make.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NewFile {
    CodeFile,
    Folder,
    Area,
    TileMap,
    Tileset,
    Animator,
    SoundEvent,
}

impl NewFile {
    /// Every one, in the Create list's order.
    pub(crate) const ALL: [Self; 7] = [
        Self::CodeFile,
        Self::Folder,
        Self::Area,
        Self::TileMap,
        Self::Tileset,
        Self::Animator,
        Self::SoundEvent,
    ];

    /// Its word in the Create list (the Library's right-click New has
    /// the same).
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::CodeFile => "Lust code file",
            Self::Folder => "Folder",
            Self::Area => "Area",
            Self::TileMap => "Tile map",
            Self::Tileset => "Tileset",
            Self::Animator => "Animator",
            Self::SoundEvent => "Sound event",
        }
    }

    /// The Library's request that makes it in `folder`.
    pub(crate) fn request(self, folder: String) -> crate::library::LibraryRequest {
        use crate::library::LibraryRequest as R;
        match self {
            Self::CodeFile => R::NewCodeFile(folder),
            Self::Folder => R::NewFolder(folder),
            Self::Area => R::NewArea(folder),
            Self::TileMap => R::NewTileMap(folder),
            Self::Tileset => R::NewTileset(folder),
            Self::Animator => R::NewAnimator(folder),
            Self::SoundEvent => R::NewSoundEvent(folder),
        }
    }
}

impl ShellCommand {
    /// Every command, in the order the search lists them.
    pub(crate) const ALL: [Self; 143] = [
        C::NewProject,
        C::OpenProject,
        C::RecentProjects,
        C::NewArea,
        C::OpenArea,
        C::SaveArea,
        C::SaveAreaAs,
        C::ImportModel,
        C::ExportPlaytest,
        C::ProjectSettings,
        C::InputActions,
        C::Preferences,
        C::Undo,
        C::Redo,
        C::Duplicate,
        C::Delete,
        C::Rename,
        C::SelectAll,
        C::SelectNone,
        C::AddEntity,
        C::AddSprite,
        C::AddPointLight,
        C::AddSpotlight,
        C::AddCamera,
        C::AddReverbZone,
        C::ChooseShapeBrush,
        C::EditBrushInsides,
        C::BrushMode(crate::brushing::edit::Mode::Points),
        C::BrushMode(crate::brushing::edit::Mode::Edges),
        C::BrushMode(crate::brushing::edit::Mode::Faces),
        C::BrushSolid,
        C::NewTileMap,
        C::CreateAssembly,
        C::SnapToGrid,
        C::SnapAngles,
        C::SnapScale,
        C::ViewportTools,
        C::ViewportSnapping,
        C::ViewportStats,
        C::Play,
        C::Pause,
        C::Step,
        C::Stop,
        C::LoadSave,
        C::PlayAndRecord,
        C::ReplayRecording,
        C::ProfilerCpu,
        C::ProfilerGpu,
        C::ResetLayout,
        C::FoldShelf,
        C::HideUnfinished,
        C::CollapseRibbon,
        C::UseTool(Tool::Select),
        C::UseTool(Tool::Move),
        C::UseTool(Tool::Rotate),
        C::UseTool(Tool::Scale),
        C::UseTool(Tool::Transform),
        C::NewInLibrary(NewFile::CodeFile),
        C::NewInLibrary(NewFile::Folder),
        C::NewInLibrary(NewFile::Area),
        C::NewInLibrary(NewFile::TileMap),
        C::NewInLibrary(NewFile::Tileset),
        C::NewInLibrary(NewFile::Animator),
        C::NewInLibrary(NewFile::SoundEvent),
        C::LibraryList,
        C::LibraryFolders,
        C::LibraryFilter(Filter::All),
        C::LibraryFilter(Filter::Models),
        C::LibraryFilter(Filter::Sprites),
        C::LibraryFilter(Filter::Maps),
        C::LibraryFilter(Filter::Sounds),
        C::LibraryFilter(Filter::Code),
        C::PlacePicked,
        C::PaintWith(PaintTool::Brush),
        C::PaintWith(PaintTool::Rectangle),
        C::PaintWith(PaintTool::Line),
        C::PaintWith(PaintTool::Fill),
        C::PaintWith(PaintTool::Eyedropper),
        C::PaintExact,
        C::MapMeaning(false),
        C::MapMeaning(true),
        C::MapChunks,
        C::MapWalls,
        C::TurnCopy(Turn::FlipX),
        C::TurnCopy(Turn::FlipY),
        C::TurnCopy(Turn::Rotate),
        C::PutCopyAway,
        C::TerrainAdd,
        C::TerrainRemove,
        C::TerrainKind(tiles::TileKind::Plain),
        C::TerrainKind(tiles::TileKind::Dual16),
        C::TerrainKind(tiles::TileKind::Edges),
        C::TerrainKind(tiles::TileKind::Blob47),
        C::TerrainKind(tiles::TileKind::Rules),
        C::SaveTemplate,
        C::CropEveryTile,
        C::CropDelete,
        C::ClipNew,
        C::ClipCopy,
        C::ClipDelete,
        C::FramesTakeOut,
        C::ClipDirection(0),
        C::ClipDirection(1),
        C::ClipDirection(2),
        C::ClipDirection(3),
        C::StateAdd,
        C::StateStartHere,
        C::StateDelete,
        C::PreviewAgain,
        C::ScreenText(100),
        C::ScreenText(150),
        C::ScreenText(200),
        C::SafeArea,
        C::ScreenSnap,
        C::OpenScreenCode,
        C::AddBus,
        C::AddDuckingRule,
        C::RemoveBus,
        C::ClearSolo,
        C::MuteGame,
        C::OpenExportFolder,
        C::Tab(RibbonTab::File),
        C::Tab(RibbonTab::Home),
        C::Tab(RibbonTab::Library),
        C::Tab(RibbonTab::Shading),
        C::Tab(RibbonTab::Animate),
        C::Tab(RibbonTab::Ui),
        C::Tab(RibbonTab::Vfx),
        C::Tab(RibbonTab::Audio),
        C::Tab(RibbonTab::Debug),
        C::Tab(RibbonTab::Build),
        C::OpenGuide,
        C::Learn,
        C::AddSampleArea,
        C::MakeSampleGame,
        C::MakeUiSample,
        C::MakeBenchmark,
        C::MakePlayground,
        C::Updates,
        C::CheckForUpdates,
        C::AutoUpdate,
        C::About,
        C::Exit,
    ];

    /// Why the command can't be done now, or `None` when it can.
    pub(crate) fn why_not(self, w: &Workbench) -> Option<String> {
        let state = w.play_state();
        let playing = state != PlayState::Editing;
        let selected = w.open.document.selection.primary().is_some();
        let project = w.open.project.is_some();
        let history = w.open.document.history();
        let when = |ok: bool, why: &str| (!ok).then(|| why.to_owned());
        match self {
            C::NewProject
            | C::OpenProject
            | C::NewArea
            | C::OpenArea
            | C::SaveArea
            | C::SaveAreaAs
            | C::AddSampleArea
            | C::MakeSampleGame
            | C::MakeUiSample
            | C::MakeBenchmark
            | C::MakePlayground => when(!playing, STOP_FIRST),
            C::ExportPlaytest => when(project, "Open a project first"),
            C::OpenExportFolder => when(
                w.open
                    .export
                    .as_ref()
                    .is_some_and(|form| matches!(form.done, Some(Ok(_)))),
                "Export once first: this opens the folder it made",
            ),
            C::NewTileMap => when(
                project,
                "Tile maps are kept in a project: open or make one first",
            ),
            C::Undo => when(history.undo_label().is_some(), "Nothing to undo"),
            C::Redo => when(history.redo_label().is_some(), "Nothing to redo"),
            C::Duplicate | C::Delete | C::Rename | C::SelectNone | C::CreateAssembly => {
                when(selected, "Select an entity first")
            }
            C::Play => match w.why_not_play() {
                Some(why) => Some(why.to_owned()),
                None => when(state != PlayState::Playing, "The game is playing"),
            },
            C::Pause | C::Stop => when(playing, "Press Play first"),
            C::Step => when(state == PlayState::Paused, "Pause the game first"),
            C::PlayAndRecord => {
                if playing {
                    Some(STOP_FIRST.to_owned())
                } else if !project {
                    Some("Recordings are kept in a project: open or make one first".to_owned())
                } else {
                    w.why_not_play().map(str::to_owned)
                }
            }
            C::FoldShelf => when(!w.shell.covers_center(), "This tab has no shelf"),
            C::EditBrushInsides => w.why_not_edit_brush(),
            C::BrushMode(_) | C::BrushSolid => when(
                w.viewport.brushing.edit.is_some(),
                "Pick one brush and press Tab first",
            ),
            C::NewInLibrary(_) => when(project, "Open or make a project first (File)"),
            C::PlacePicked => when(
                w.picked_placeable().is_some(),
                "Pick a model, an assembly, a sprite sheet or a tile map in the Library first",
            ),
            C::PaintWith(_)
            | C::PaintOnLayer(_)
            | C::PaintExact
            | C::MapMeaning(_)
            | C::ShowValue(_)
            | C::MapChunks
            | C::MapWalls => w.why_not_paint(),
            C::TurnCopy(_) | C::PutCopyAway => when(
                w.viewport.tile_overlay.paint.stamp.is_some(),
                "Copy some cells with the Eyedropper first",
            ),
            C::TerrainAdd
            | C::TerrainRemove
            | C::TerrainKind(_)
            | C::SaveTemplate
            | C::TilesetZoom(_)
            | C::CropEveryTile
            | C::CropDelete
            | C::CropsZoom(_) => w.why_not_library_tool(self),
            C::ClipNew
            | C::ClipCopy
            | C::ClipDelete
            | C::FramesTakeOut
            | C::ClipDirection(_)
            | C::StateAdd
            | C::StateStartHere
            | C::StateDelete
            | C::PreviewAgain => w.why_not_animate(self),
            C::AddBus
            | C::AddDuckingRule
            | C::RemoveBus
            | C::ClearSolo
            | C::MuteGame
            | C::HeardAtOnce(_) => w.why_not_audio(self),
            C::ScreenSize(_)
            | C::ScreenText(_)
            | C::SafeArea
            | C::ScreenSnap
            | C::OpenScreenCode => when(
                w.open.screen_doc.is_some(),
                "Open a screen from the Library first",
            ),
            C::Tab(tab) => {
                if tab == w.shell.tab {
                    Some(format!("{} is the tab in front", tab.name()))
                } else if !tab.is_built() {
                    // What isn't built is never offered as something to do.
                    Some(String::new())
                } else {
                    None
                }
            }
            C::CheckForUpdates => when(
                !matches!(
                    w.update,
                    crate::updates::UpdateNotice::Checking
                        | crate::updates::UpdateNotice::Downloading(_)
                ),
                "Already checking",
            ),
            _ => None,
        }
    }

    /// Whether a switch is on, for a command that is one: its box on the
    /// ribbon is ticked then.
    pub(crate) fn is_on(self, w: &Workbench) -> Option<bool> {
        Some(match self {
            C::SnapToGrid => w.viewport.snapping.grid,
            C::SnapAngles => w.viewport.snapping.angle,
            C::BrushSolid => w.edited_brush_solid()?,
            C::SnapScale => w.viewport.snapping.scale,
            C::ViewportTools => w.viewport.overlays.tools,
            C::ViewportSnapping => w.viewport.overlays.snapping,
            C::ViewportStats => w.viewport.overlays.stats,
            C::ProfilerCpu => !w.profiler.hide_cpu,
            C::ProfilerGpu => !w.profiler.hide_gpu,
            C::HideUnfinished => w.interface_look().hide_unfinished,
            C::AutoUpdate => w.auto_update,
            C::LibraryList => w.open.library_state.list,
            C::LibraryFolders => w.open.library_state.folders_shown,
            C::LibraryFilter(filter) => w.open.library_state.filter == filter,
            C::PaintExact => w.viewport.tile_overlay.paint.exact,
            C::MapChunks => w.open.map_edits.chunks,
            C::MapWalls => w.open.map_edits.walls,
            C::SafeArea => w.open.screen_doc.as_ref().is_some_and(|d| d.safe_area),
            C::ScreenSnap => w.open.screen_doc.as_ref().is_some_and(|d| d.snap),
            C::MuteGame => w.open.mixer.listening.all_muted,
            _ => return None,
        })
    }

    /// Does it. A command that can't be done now does nothing.
    pub(crate) fn run(
        self,
        w: &mut Workbench,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if self.why_not(w).is_some() {
            return;
        }
        if let Some(action) = self.asks_the_app() {
            w.request(action, actions);
            return;
        }
        if let Some(command) = self.on_the_area() {
            w.run(command, ctx);
            return;
        }
        match self {
            C::RecentProjects => w.show_recent_projects(),
            C::Learn => w.show_file_page(crate::file_pages::Page::Learn),
            C::Updates => w.show_file_page(crate::file_pages::Page::Updates),
            C::ProjectSettings => w.show_settings(w.file_pages.project_tab),
            C::InputActions => w.open_input_page(),
            C::Preferences => w.show_settings(w.file_pages.prefs_tab),
            C::AddSprite => {
                w.add_sprite();
            }
            C::AddReverbZone => {
                w.add_reverb_zone();
            }
            C::ChooseShapeBrush => {
                // Brought into view first: the shape is drawn out there.
                w.show_panel(crate::panel::Panel::Viewport);
                let pointer = ctx.input(|i| i.pointer.latest_pos());
                w.open_brush_list(pointer);
            }
            C::NewTileMap => {
                let folder = w.open.library_state.folder.clone();
                w.new_tile_map(&folder);
            }
            C::EditBrushInsides => w.toggle_brush_editing(),
            C::BrushMode(mode) => w.brush_mode(mode),
            C::BrushSolid => w.switch_edited_brush_solid(),
            C::SnapToGrid => w.viewport.snapping.grid ^= true,
            C::SnapAngles => w.viewport.snapping.angle ^= true,
            C::SnapScale => w.viewport.snapping.scale ^= true,
            C::ViewportTools => w.viewport.overlays.tools ^= true,
            C::ViewportSnapping => w.viewport.overlays.snapping ^= true,
            C::ViewportStats => w.viewport.overlays.stats ^= true,
            C::Play => w.play(),
            C::Pause => w.pause(),
            C::Step => w.step(),
            C::Stop => w.stop(),
            C::PlayAndRecord => w.play_and_record(),
            C::ProfilerCpu => w.profiler.hide_cpu ^= true,
            C::ProfilerGpu => w.profiler.hide_gpu ^= true,
            C::ResetLayout => w.reset_layout(),
            C::FoldShelf => w.fold_shelf(!w.shelf_folded()),
            C::HideUnfinished => {
                let hide = !w.interface_look().hide_unfinished;
                w.hide_unfinished(ctx, hide);
            }
            C::CollapseRibbon => w.set_ribbon_collapsed(!w.shell.collapsed),
            C::NewInLibrary(new) => {
                let folder = w.open.library_state.folder.clone();
                w.library_request(new.request(folder), actions);
            }
            C::LibraryList => w.open.library_state.list ^= true,
            C::LibraryFolders => w.open.library_state.folders_shown ^= true,
            C::LibraryFilter(filter) => w.open.library_state.filter = filter,
            C::PlacePicked => {
                if let Some(id) = w.picked_placeable() {
                    w.show_tab(RibbonTab::Home);
                    w.library_request(crate::library::LibraryRequest::Place(id), actions);
                }
            }
            C::UseTool(tool) => {
                w.viewport.tool = tool;
                w.viewport.tile_overlay.paint.tool = None;
            }
            C::PaintWith(_)
            | C::PaintOnLayer(_)
            | C::PaintExact
            | C::MapMeaning(_)
            | C::ShowValue(_)
            | C::MapChunks
            | C::MapWalls
            | C::TurnCopy(_)
            | C::PutCopyAway => w.run_paint_command(self),
            C::TerrainAdd
            | C::TerrainRemove
            | C::TerrainKind(_)
            | C::SaveTemplate
            | C::TilesetZoom(_)
            | C::CropEveryTile
            | C::CropDelete
            | C::CropsZoom(_) => w.run_library_tool(self),
            C::ClipNew
            | C::ClipCopy
            | C::ClipDelete
            | C::FramesTakeOut
            | C::ClipDirection(_)
            | C::StateAdd
            | C::StateStartHere
            | C::StateDelete
            | C::PreviewAgain => w.run_animate(self),
            C::AddBus
            | C::AddDuckingRule
            | C::RemoveBus
            | C::ClearSolo
            | C::MuteGame
            | C::HeardAtOnce(_) => w.run_audio(self, actions),
            C::ScreenSize(_) | C::ScreenText(_) | C::SafeArea | C::ScreenSnap => {
                if let Some(doc) = &mut w.open.screen_doc {
                    match self {
                        C::ScreenSize(i) => doc.preset = usize::from(i),
                        C::ScreenText(percent) => doc.text_percent = percent,
                        C::SafeArea => doc.safe_area ^= true,
                        _ => doc.snap ^= true,
                    }
                }
            }
            C::OpenScreenCode => {
                if let Some(path) = w.open.screen_doc.as_ref().map(|d| d.path.clone()) {
                    let open = crate::library::LibraryRequest::Open(
                        crate::library::LibraryKind::Code,
                        path,
                    );
                    w.library_request(open, actions);
                }
            }
            C::Tab(tab) => w.show_tab(tab),
            C::AutoUpdate => {
                w.auto_update ^= true;
                actions.push(WorkbenchAction::SetAutoUpdate(w.auto_update));
            }
            C::About => w.show_file_page(crate::file_pages::Page::About),
            C::OpenExportFolder => {
                if let Some(Some(Ok(folder))) = w.open.export.as_ref().map(|f| f.done.clone()) {
                    actions.push(WorkbenchAction::ShowFolder(folder));
                }
            }
            _ => {}
        }
    }

    /// The request to the app the command is, for one that is nothing
    /// else: the app opens the window, reads the file or closes.
    fn asks_the_app(self) -> Option<WorkbenchAction> {
        use WorkbenchAction as A;
        Some(match self {
            C::NewProject => A::NewProject,
            C::OpenProject => A::OpenProject,
            C::NewArea => A::NewArea,
            C::OpenArea => A::OpenArea,
            C::SaveArea => A::SaveArea,
            C::SaveAreaAs => A::SaveAreaAs,
            C::ImportModel => A::ImportModel,
            C::LoadSave => A::OpenSave,
            C::ReplayRecording => A::OpenRecording,
            C::AddSampleArea => A::AddSampleArea,
            C::MakeSampleGame => A::MakeSampleGame,
            C::MakeUiSample => A::MakeUiSample,
            C::MakeBenchmark => A::MakeBenchmark,
            C::MakePlayground => A::MakePlayground,
            C::CheckForUpdates => A::CheckForUpdates,
            C::OpenGuide => A::OpenGuide(None),
            C::Exit => A::Quit,
            _ => return None,
        })
    }

    /// The command as the Area panel's own menus know it, for the ones
    /// they share.
    fn on_the_area(self) -> Option<MenuCommand> {
        Some(match self {
            C::ExportPlaytest => MenuCommand::ExportPlaytest,
            C::Undo => MenuCommand::Undo,
            C::Redo => MenuCommand::Redo,
            C::Duplicate => MenuCommand::Duplicate,
            C::Delete => MenuCommand::Delete,
            C::Rename => MenuCommand::Rename,
            C::SelectAll => MenuCommand::SelectAll,
            C::SelectNone => MenuCommand::SelectNone,
            C::AddEntity => MenuCommand::AddEntity,
            C::AddPointLight => MenuCommand::AddPointLight,
            C::AddSpotlight => MenuCommand::AddSpotlight,
            C::AddCamera => MenuCommand::AddCamera,
            C::CreateAssembly => MenuCommand::CreateAssembly,
            _ => return None,
        })
    }
}
