//! The words of the editor's commands: each one's name, where it lives,
//! its shortcut and the sentence its button says on hover. The one table
//! of shortcuts is [`ShellCommand::keys`].

use super::commands::{C, ShellCommand};
use crate::workbench::Workbench;

impl ShellCommand {
    /// The command's name, as the search lists it and a hover text says
    /// it. A switch is named for what choosing it does now ("Hide
    /// unfinished parts", then "Show unfinished parts"), and Undo and
    /// Redo for the step they act on ("Undo Move").
    pub(crate) fn name(self, w: &Workbench) -> String {
        let switch = |on: bool, what: &str| format!("{} {what}", if on { "Hide" } else { "Show" });
        let history = w.open.document.history();
        match self {
            C::Undo => history
                .undo_label()
                .map_or_else(|| "Undo".to_owned(), |step| format!("Undo {step}")),
            C::Redo => history
                .redo_label()
                .map_or_else(|| "Redo".to_owned(), |step| format!("Redo {step}")),
            C::SnapToGrid => on_off(
                "Snap to the grid",
                "Stop snapping to the grid",
                w.viewport.snapping.grid,
            ),
            C::SnapAngles => on_off(
                "Snap angles",
                "Stop snapping angles",
                w.viewport.snapping.angle,
            ),
            C::SnapScale => on_off(
                "Snap scale",
                "Stop snapping scale",
                w.viewport.snapping.scale,
            ),
            C::ViewportTools => switch(w.viewport.overlays.tools, "the toolbox"),
            C::ViewportSnapping => switch(
                w.viewport.overlays.snapping,
                "the viewport's axis locks and camera speed",
            ),
            C::ViewportStats => switch(
                w.viewport.overlays.stats,
                "the viewport's frame time and frame rate",
            ),
            C::ProfilerCpu => switch(!w.profiler.hide_cpu, "the CPU's times in the Profiler"),
            C::ProfilerGpu => switch(!w.profiler.hide_gpu, "the GPU's times in the Profiler"),
            C::HideUnfinished => switch(!w.interface_look().hide_unfinished, "unfinished parts"),
            C::CollapseRibbon if w.shell.collapsed => "Show the ribbon".to_owned(),
            C::FoldShelf if w.shelf_folded() => "Unfold the shelf".to_owned(),
            C::AutoUpdate => on_off(
                "Check for updates when starting",
                "Don't check for updates when starting",
                w.auto_update,
            ),
            C::Tab(tab) => format!("{} tab", tab.name()),
            C::About => format!("About {}", w.info.name),
            other => other.fixed_name().to_owned(),
        }
    }

    /// The name of a command whose name never changes.
    fn fixed_name(self) -> &'static str {
        match self {
            C::NewProject => "New project…",
            C::OpenProject => "Open project…",
            C::RecentProjects => "Recent projects",
            C::NewArea => "New area",
            C::OpenArea => "Open area…",
            C::SaveArea => "Save area",
            C::SaveAreaAs => "Save area as…",
            C::ImportModel => "Import model…",
            C::ExportPlaytest => "Export for playtesting…",
            C::ProjectSettings => "Project settings",
            C::InputActions => "Input actions…",
            C::Preferences => "Preferences",
            C::Duplicate => "Duplicate",
            C::Delete => "Delete",
            C::Rename => "Rename",
            C::SelectAll => "Select all",
            C::SelectNone => "Select nothing",
            C::AddEntity => "Add entity",
            C::AddSprite => "Add sprite",
            C::AddPointLight => "Add point light",
            C::AddSpotlight => "Add spotlight",
            C::AddCamera => "Add camera",
            C::AddReverbZone => "Add reverb zone",
            C::NewTileMap => "New tile map",
            C::CreateAssembly => "Create assembly from selection",
            C::Play => "Play",
            C::Pause => "Pause",
            C::Step => "Step one tick",
            C::Stop => "Stop",
            C::LoadSave => "Load a save…",
            C::PlayAndRecord => "Play and record inputs",
            C::ReplayRecording => "Replay a recording…",
            C::ResetLayout => "Reset layout",
            C::FoldShelf => "Fold the shelf",
            C::CollapseRibbon => "Collapse the ribbon",
            C::OpenManual => "Open the manual ↗",
            C::Learn => "Learn",
            C::AddSampleArea => "Add the sample area",
            C::MakeSampleGame => "Make the sample game…",
            C::MakeUiSample => "Make the UI sample…",
            C::MakeBenchmark => "Make the benchmark…",
            C::MakePlayground => "Make the playground…",
            C::Updates => "Updates",
            C::CheckForUpdates => "Check for updates",
            C::Exit => "Exit",
            // Named by `name`, from what the editor holds.
            C::Undo
            | C::Redo
            | C::SnapToGrid
            | C::SnapAngles
            | C::SnapScale
            | C::ViewportTools
            | C::ViewportSnapping
            | C::ViewportStats
            | C::ProfilerCpu
            | C::ProfilerGpu
            | C::HideUnfinished
            | C::AutoUpdate
            | C::Tab(_)
            | C::About => "",
        }
    }

    /// Where the command lives in the frame: the tab whose ribbon has it,
    /// or File. The search shows it beside the name.
    pub(crate) fn place(self) -> &'static str {
        match self {
            C::NewProject
            | C::OpenProject
            | C::RecentProjects
            | C::ProjectSettings
            | C::InputActions
            | C::Preferences
            | C::OpenManual
            | C::Learn
            | C::AddSampleArea
            | C::MakeSampleGame
            | C::MakeUiSample
            | C::MakeBenchmark
            | C::MakePlayground
            | C::Updates
            | C::CheckForUpdates
            | C::AutoUpdate
            | C::About
            | C::Exit => "File",
            C::ImportModel => "Library",
            C::ExportPlaytest => "Build",
            C::LoadSave
            | C::PlayAndRecord
            | C::ReplayRecording
            | C::ProfilerCpu
            | C::ProfilerGpu => "Debug",
            C::Play | C::Pause | C::Step | C::Stop => "Play",
            C::ViewportTools | C::ViewportSnapping | C::ViewportStats => "Viewport",
            C::ResetLayout | C::HideUnfinished | C::CollapseRibbon | C::FoldShelf => "Preferences",
            C::Tab(_) => "Tabs",
            _ => "Home",
        }
    }

    /// The command's shortcut, as it is shown. The one table of
    /// shortcuts: the search, the ribbon's hover texts and the manual's
    /// page all say these.
    pub(crate) fn keys(self) -> &'static str {
        match self {
            C::NewArea => "Ctrl+N",
            C::OpenArea => "Ctrl+O",
            C::SaveArea => "Ctrl+S",
            C::SaveAreaAs => "Ctrl+Shift+S",
            C::Undo => "Ctrl+Z",
            C::Redo => "Ctrl+Y",
            C::Duplicate => "Ctrl+D",
            C::Delete => "Delete",
            C::Rename => "F2",
            C::SelectAll => "Ctrl+A",
            C::SelectNone => "Esc",
            C::FoldShelf => "Ctrl+Space",
            _ => "",
        }
    }

    /// What the command does, in a sentence, for the hover text of its
    /// button. Empty where the name says it all.
    pub(crate) fn hint(self) -> &'static str {
        match self {
            C::NewProject => "Makes a folder into a project: its assets, settings and areas",
            C::OpenProject => "Opens a project's folder",
            C::RecentProjects => "Every project, the examples, and where to begin",
            C::SaveArea => "Saves the open area. Other files save as they change",
            C::ImportModel => {
                "A glTF model (.gltf or .glb). You can also drop one onto the window."
            }
            C::ExportPlaytest => {
                "A folder with the game in it, to zip and send to a friend: no editor needed"
            }
            C::ProjectSettings => "This project's settings: its 2D sizes",
            C::InputActions => "The game's actions (Move, Jump…) and the keys and buttons for each",
            C::Preferences => "The editor's settings: its colour, its size and its hints",
            C::AddEntity => "An empty entity, at the point the camera looks at",
            C::AddSprite => "An entity that draws a sprite sheet, at the point the camera looks at",
            C::AddPointLight => "A bulb shining every way, placed above where the camera looks",
            C::AddSpotlight => "A cone of light at the camera, shining where it looks",
            C::AddCamera => {
                "A camera where you're looking from: the game looks through the first one in the area"
            }
            C::AddReverbZone => "A place where sounds echo, at the point the camera looks at",
            C::NewTileMap => {
                "A new tile map file in the Library's folder, to place in the area and paint"
            }
            C::CreateAssembly => {
                "Saves the selected entity, and everything under it, as an assembly in the project, \
                 and turns it into a copy of it"
            }
            C::SnapToGrid => "Moves land on the grid. A click on its step chooses another",
            C::SnapAngles => "Turns go in steps. A click on the step chooses another",
            C::SnapScale => "Scaling goes in steps of 10%",
            C::Step => "Runs the paused game for one tick",
            C::LoadSave => {
                "Plays from a save game in the project's saves folder, exactly as it was saved"
            }
            C::PlayAndRecord => {
                "Plays, keeping every tick's input; Stop saves it in the project's recordings \
                 folder, to play back exactly"
            }
            C::ReplayRecording => {
                "Plays a recording's input back from Play, and says whether the game came out \
                 exactly the same"
            }
            C::ResetLayout => "Puts this tab's panels back where they started",
            C::CollapseRibbon => "Leaves only the tabs, giving the ribbon's room to the editor",
            C::FoldShelf => {
                "Folds the shelf to its bar, giving its room to the editor, or opens it again"
            }
            C::OpenManual => "The user manual, in your web browser",
            C::Learn => "The examples, the first steps and the manual",
            C::AddSampleArea => {
                "A small area for trying the looks: a checker cube, rows of plastic and metal \
                 spheres, and a grooved wall with a glowing stripe"
            }
            C::MakeSampleGame => {
                "A small first game, as a new project in a folder you choose: a turret shooting at \
                 a drone, written in Lust, Lua and Rust that talk through events"
            }
            C::MakeUiSample => {
                "A game's screens, as a new project in a folder you choose: a HUD, a health bar \
                 and a pause menu, each drawn on the UI editor's canvas"
            }
            C::MakeBenchmark => {
                "Two fleets of ships firing lasers, as a new project in a folder you choose: play \
                 its areas to see how many this computer keeps at 60 frames a second"
            }
            C::MakePlayground => {
                "A level to walk around in, as a new project in a folder you choose: a character \
                 with the third-person camera, steps, ramps, crates to push and a moving platform"
            }
            C::Updates => "This build, the newest one and what it brings",
            C::CheckForUpdates => {
                "Asks GitHub for the newest build now, and asks you whether to update if there is \
                 one (even one you skipped)"
            }
            C::Exit => "Closes the editor, asking about unsaved work first",
            _ => "",
        }
    }

    /// The command whose name is `name` now.
    #[cfg(test)]
    pub(crate) fn named(name: &str, w: &Workbench) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name(w) == name)
    }

    /// What pointing at the command's button says: its name, its
    /// shortcut, and what it does or why it can't be done now.
    pub(crate) fn hover_text(self, w: &Workbench) -> String {
        let mut text = self.name(w);
        if !self.keys().is_empty() {
            text.push_str(&format!(" ({})", self.keys()));
        }
        match self.why_not(w) {
            Some(why) if !why.is_empty() => text.push_str(&format!("\n{why}")),
            _ if !self.hint().is_empty() => text.push_str(&format!("\n{}", self.hint())),
            _ => {}
        }
        text
    }
}

/// A switch's name: what choosing it does now.
fn on_off(turn_on: &str, turn_off: &str, on: bool) -> String {
    if on { turn_off } else { turn_on }.to_owned()
}
