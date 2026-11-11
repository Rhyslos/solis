//! What the editor can do by name: one table for the ribbon's buttons,
//! the File page's items and the search, so each thing has one name, one
//! shortcut and one answer to "can it be done now, and if not, why not?".
//!
//! [`ShellCommand::ALL`] is every command, in the order the search lists
//! them with nothing typed. A command that can't be done now is left out
//! of the search, and drawn faint on the ribbon with its reason as the
//! hover text.

use super::RibbonTab;
use crate::menu_items::MenuCommand;
use crate::play::PlayState;
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
    PinLibrary,
    UnpinLibrary,
    HideUnfinished,
    CollapseRibbon,
    Tab(RibbonTab),
    OpenManual,
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

impl ShellCommand {
    /// Every command, in the order the search lists them.
    pub(crate) const ALL: [Self; 69] = [
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
        C::PinLibrary,
        C::UnpinLibrary,
        C::HideUnfinished,
        C::CollapseRibbon,
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
        C::OpenManual,
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
            C::PinLibrary => when(!w.library_pinned, "The Library is pinned"),
            C::UnpinLibrary => when(w.library_pinned, "The Library isn't pinned"),
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
            C::OpenManual => when(
                !w.manual_url.is_empty(),
                "This copy doesn't know where its manual is",
            ),
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
            C::SnapScale => w.viewport.snapping.scale,
            C::ViewportTools => w.viewport.overlays.tools,
            C::ViewportSnapping => w.viewport.overlays.snapping,
            C::ViewportStats => w.viewport.overlays.stats,
            C::ProfilerCpu => !w.profiler.hide_cpu,
            C::ProfilerGpu => !w.profiler.hide_gpu,
            C::HideUnfinished => w.interface_look().hide_unfinished,
            C::AutoUpdate => w.auto_update,
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
            C::RecentProjects => w.show_file_page(super::FileItem::RecentProjects),
            C::Learn => w.show_file_page(super::FileItem::Learn),
            C::Updates => w.show_file_page(super::FileItem::Updates),
            C::ProjectSettings => w.open_project_settings(),
            C::InputActions => w.open_input_window(),
            C::Preferences => w.open_settings(),
            C::AddSprite => {
                w.add_sprite();
            }
            C::AddReverbZone => {
                w.add_reverb_zone();
            }
            C::NewTileMap => {
                let folder = w.open.library_state.folder.clone();
                w.new_tile_map(&folder);
            }
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
            C::PinLibrary => w.set_library_pinned(true),
            C::UnpinLibrary => w.set_library_pinned(false),
            C::HideUnfinished => {
                let hide = !w.interface_look().hide_unfinished;
                w.hide_unfinished(ctx, hide);
            }
            C::CollapseRibbon => w.set_ribbon_collapsed(!w.shell.collapsed),
            C::Tab(tab) => w.show_tab(tab),
            C::OpenManual => ctx.open_url(egui::OpenUrl::new_tab(w.manual_url.clone())),
            C::AutoUpdate => {
                w.auto_update ^= true;
                actions.push(WorkbenchAction::SetAutoUpdate(w.auto_update));
            }
            C::About => w.about_open = true,
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
