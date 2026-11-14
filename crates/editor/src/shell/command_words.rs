//! The words of the editor's commands: each one's name, where it lives,
//! its shortcut and the sentence its button says on hover. The one table
//! of shortcuts is [`ShellCommand::keys`].

use super::commands::{C, ShellCommand};
use crate::viewport::Tool;
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
            C::ViewportTools => switch(w.viewport.overlays.tools, "the viewport's tools"),
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
            C::UseTool(tool) => format!("{} tool", tool_word(tool)),
            C::NewInLibrary(new) => format!("New in the Library: {}", new.word()),
            C::LibraryList => on_off(
                "Show the Library as a list",
                "Show the Library as tiles",
                w.open.library_state.list,
            ),
            C::LibraryFolders => switch(
                w.open.library_state.folders_shown,
                "the Library's folder tree",
            ),
            C::LibraryFilter(filter) => match filter {
                crate::library::Filter::All => "Show every kind of file".to_owned(),
                other => format!("Show only {}", other.name().to_lowercase()),
            },
            C::About => format!("About {}", w.info.name),
            C::PaintWith(_)
            | C::PaintOnLayer(_)
            | C::PaintExact
            | C::MapMeaning(_)
            | C::ShowValue(_)
            | C::MapChunks
            | C::MapWalls => tile_map_name(self, w),
            C::TerrainKind(kind) => format!("Make the terrain {}", kind.word()),
            C::TilesetZoom(zoom) => format!("Show the tileset at {}%", u32::from(zoom) * 100),
            C::CropsZoom(zoom) => format!("Show the picture at {}%", u32::from(zoom) * 100),
            C::ScreenSize(i) => match ui::presets::PRESETS.get(usize::from(i)) {
                Some(p) => format!("Show the screen at {} × {}", p.size[0], p.size[1]),
                None => "Show the screen at another size".to_owned(),
            },
            C::ScreenText(percent) => format!("Show the screen's text at {percent}%"),
            C::SafeArea => switch(
                w.open.screen_doc.as_ref().is_some_and(|d| d.safe_area),
                "the screen's safe area",
            ),
            C::ScreenSnap => on_off(
                "Snap on the canvas",
                "Stop snapping on the canvas",
                w.open.screen_doc.as_ref().is_some_and(|d| d.snap),
            ),
            C::ClipDirection(i) => {
                let d = crate::shell::ribbons::animate::DIRECTIONS
                    .get(usize::from(i))
                    .copied()
                    .unwrap_or(animation::Direction::Forward);
                format!("Play the clip {}", d.name())
            }
            C::MuteGame => on_off(
                "Mute the game",
                "Hear the game again",
                w.open.mixer.listening.all_muted,
            ),
            C::HeardAtOnce(n) => format!("Hear {n} sounds at once"),
            C::EditBrushInsides if w.viewport.brushing.edit.is_some() => {
                "Stop editing the brush's insides".to_owned()
            }
            C::BrushMode(mode) => format!("Pick the brush's {}", mode.name().to_lowercase()),
            C::BrushSolid => on_off(
                "Make the brush solid",
                "Make the brush not solid",
                w.edited_brush_solid().unwrap_or(false),
            ),
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
            C::ChooseShapeBrush => "Choose a shape brush",
            C::EditBrushInsides => "Edit the brush's insides",
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
            C::OpenGuide => "Documentation ↗",
            C::Learn => "Learn",
            C::AddSampleArea => "Add the sample area",
            C::MakeSampleGame => "Make the sample game…",
            C::MakeUiSample => "Make the UI sample…",
            C::MakeBenchmark => "Make the benchmark…",
            C::MakePlayground => "Make the playground…",
            C::Updates => "Updates",
            C::CheckForUpdates => "Check for updates",
            C::Exit => "Exit",
            C::PlacePicked => "Place the picked file",
            C::TurnCopy(crate::tile_paint::Turn::FlipX) => "Flip the copy left and right",
            C::TurnCopy(crate::tile_paint::Turn::FlipY) => "Flip the copy top and bottom",
            C::TurnCopy(crate::tile_paint::Turn::Rotate) => "Turn the copy a quarter",
            C::PutCopyAway => "Put the copy away",
            C::TerrainAdd => "Add a terrain",
            C::TerrainRemove => "Remove the chosen terrain",
            C::SaveTemplate => "Save the terrain's template",
            C::CropEveryTile => "Crop every tile",
            C::CropDelete => "Delete the picked crop",
            C::ClipNew => "New clip",
            C::ClipCopy => "Copy the clip",
            C::ClipDelete => "Delete the clip",
            C::FramesTakeOut => "Take the frames picked out",
            C::StateAdd => "Add a state",
            C::StateStartHere => "Start in the state picked",
            C::StateDelete => "Delete what is picked in the graph",
            C::PreviewAgain => "Start the preview again",
            C::OpenScreenCode => "Open the screen as code",
            C::AddBus => "Add a bus",
            C::AddDuckingRule => "Add a ducking rule",
            C::RemoveBus => "Remove the bus",
            C::ClearSolo => "Clear solo",
            C::OpenExportFolder => "Open the export's folder",
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
            | C::UseTool(_)
            | C::NewInLibrary(_)
            | C::LibraryList
            | C::LibraryFolders
            | C::LibraryFilter(_)
            | C::PaintWith(_)
            | C::PaintOnLayer(_)
            | C::PaintExact
            | C::MapMeaning(_)
            | C::ShowValue(_)
            | C::MapChunks
            | C::MapWalls
            | C::TerrainKind(_)
            | C::TilesetZoom(_)
            | C::CropsZoom(_)
            | C::ClipDirection(_)
            | C::ScreenSize(_)
            | C::ScreenText(_)
            | C::SafeArea
            | C::ScreenSnap
            | C::MuteGame
            | C::HeardAtOnce(_)
            | C::BrushMode(_)
            | C::BrushSolid
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
            | C::OpenGuide
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
            C::ImportModel
            | C::NewInLibrary(_)
            | C::LibraryList
            | C::LibraryFolders
            | C::LibraryFilter(_)
            | C::PlacePicked => "Library",
            C::ExportPlaytest | C::OpenExportFolder => "Build",
            C::LoadSave
            | C::PlayAndRecord
            | C::ReplayRecording
            | C::ProfilerCpu
            | C::ProfilerGpu => "Debug",
            C::Play | C::Pause | C::Step | C::Stop => "Play",
            C::ViewportTools | C::ViewportSnapping | C::ViewportStats => "Viewport",
            C::ResetLayout | C::HideUnfinished | C::CollapseRibbon | C::FoldShelf => "Preferences",
            C::Tab(_) => "Tabs",
            C::PaintWith(_)
            | C::PaintOnLayer(_)
            | C::PaintExact
            | C::MapMeaning(_)
            | C::ShowValue(_)
            | C::MapChunks
            | C::MapWalls
            | C::TurnCopy(_)
            | C::PutCopyAway => "Tile map",
            C::TerrainAdd
            | C::TerrainRemove
            | C::TerrainKind(_)
            | C::SaveTemplate
            | C::TilesetZoom(_) => "Tileset",
            C::CropEveryTile | C::CropDelete | C::CropsZoom(_) => "Picture",
            C::ClipNew
            | C::ClipCopy
            | C::ClipDelete
            | C::FramesTakeOut
            | C::ClipDirection(_)
            | C::StateAdd
            | C::StateStartHere
            | C::StateDelete
            | C::PreviewAgain => "Animate",
            C::ScreenSize(_)
            | C::ScreenText(_)
            | C::SafeArea
            | C::ScreenSnap
            | C::OpenScreenCode => "UI",
            C::AddBus
            | C::AddDuckingRule
            | C::RemoveBus
            | C::ClearSolo
            | C::MuteGame
            | C::HeardAtOnce(_) => "Audio",
            C::EditBrushInsides | C::BrushMode(_) | C::BrushSolid => "Brush",
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
            C::ChooseShapeBrush => "B",
            C::EditBrushInsides => "Tab",
            C::BrushMode(mode) => mode.key().name(),
            C::UseTool(tool) => tool.key().name(),
            // The 2D editors' keys, which act where their tab is under
            // the pointer (`key_owner.rs`).
            C::PaintWith(tool) => tool.key().name(),
            C::TurnCopy(turn) => turn.key().name(),
            C::MapMeaning(_) => "Tab",
            C::CropDelete | C::FramesTakeOut | C::StateDelete => "Delete",
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
            C::ChooseShapeBrush => {
                "Opens the list of shape brushes beside the pointer: B moves down it and Space \
                 takes one, to draw out on the ground. B over the 3D or 2D view opens it too"
            }
            C::NewTileMap => {
                "A new tile map file in the Library's folder, to place in the area and paint"
            }
            C::EditBrushInsides => {
                "Edits the picked brush's points, edges and faces: pick them, then move, turn or \
                 stretch them with the gizmo. Tab again (or Done) ends it"
            }
            C::BrushMode(crate::brushing::edit::Mode::Points) => {
                "Clicks pick the brush's points (its corners)"
            }
            C::BrushMode(crate::brushing::edit::Mode::Edges) => {
                "Clicks pick the brush's edges (where two faces meet)"
            }
            C::BrushMode(crate::brushing::edit::Mode::Faces) => {
                "Clicks pick the brush's faces (its flat sides)"
            }
            C::BrushSolid => "Whether characters and bodies in the game collide with it",
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
            C::OpenGuide => {
                "The user guide: every part of the editor explained, in your web browser (File › \
                 Preferences chooses which)"
            }
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
            C::PlacePicked => {
                "Places the picked file where the camera looks on Home. A double-click in the \
                 Library only picks a file"
            }
            C::PaintWith(tool) => tool.what(),
            C::PaintExact => {
                "Places the tile chosen under the Map tab's terrains, not the one autotiling \
                 picks (hold Ctrl while painting for the same)"
            }
            C::MapMeaning(_) => {
                "Art: as the game draws it. Meaning: each terrain in its colour (Tab)"
            }
            C::MapChunks => "Outlines the map's chunks of 32 × 32 cells, which save and draw apart",
            C::MapWalls => {
                "Shows the walls 2D physics makes of the map's solid cells. Right-click a terrain \
                 in the Map tab to make it solid"
            }
            C::PutCopyAway => "Paints the terrain again (Esc)",
            C::TerrainAdd => "Something a cell can be: grass, water, a wall",
            C::SaveTemplate => {
                "A picture of the chosen kind's slots at the tileset's tile size, beside it: draw \
                 each tile over its slot, then Mark up from the template"
            }
            C::CropEveryTile => {
                "A crop for every grid cell with something drawn in it, named by its column and row"
            }
            C::CropDelete => "Delete",
            C::ClipNew => "A new clip of your own, made from the sheet's frames",
            C::ClipCopy => {
                "A copy of the clip to change here (an Aseprite tag's frames stay Aseprite's)"
            }
            C::ClipDelete => {
                "Deletes a clip of your own; for an Aseprite tag, forgets what was added here"
            }
            C::FramesTakeOut => "Takes the frames picked on the Frames tab out of the clip",
            C::StateAdd => "A state that plays a clip, to join to the others",
            C::ScreenSize(_) => "The screen size the canvas shows: check the screen on each",
            C::ScreenText(_) => {
                "The player's text size: check the screen still fits when players make text bigger"
            }
            C::SafeArea => "A guide 5% in from each edge: keep text and buttons inside it",
            C::ScreenSnap => "Sizes snap to whole pixels and to other pieces' edges",
            C::OpenScreenCode => "Opens the screen's file in your code editor",
            C::AddBus => "A new bus inside the one the Bus group shows (or Master)",
            C::AddDuckingRule => {
                "Makes the bus shown quieter while another plays: music under voices"
            }
            C::RemoveBus => "Buses inside it move up; sound events on it play through Master",
            C::ClearSolo => "Hears every bus again",
            C::OpenExportFolder => "Shows the folder the last export made, to zip and send",
            C::MuteGame => "Silences the game while you work; the game's sound isn't changed",
            C::HeardAtOnce(_) => {
                "How many sounds play at once: when more want to, the most important are heard \
                 (by their event's priority, then the loudest)"
            }
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

/// A tool's word, as the build form's Transform group names it.
fn tool_word(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "Select",
        Tool::Move => "Move",
        Tool::Rotate => "Rotate",
        Tool::Scale => "Scale",
        Tool::Transform => "All three",
    }
}

/// The name of a command of HOME TOOLS · Tile map, from what the
/// painter and the selected map hold.
fn tile_map_name(command: ShellCommand, w: &Workbench) -> String {
    let switch = |on: bool, what: &str| format!("{} {what}", if on { "Hide" } else { "Show" });
    let paint = &w.viewport.tile_overlay.paint;
    let edits = &w.open.map_edits;
    match command {
        C::PaintWith(tool) => format!("{} tool", tool.name()),
        C::PaintOnLayer(i) => match paint.layers.get(i) {
            Some(layer) => format!("Paint on {layer}"),
            None => "Paint on another layer".to_owned(),
        },
        C::PaintExact => on_off("Place exact tiles", "Place autotiled tiles", paint.exact),
        C::MapMeaning(true) => "Show the map's meaning".to_owned(),
        C::MapMeaning(false) => "Show the map's art".to_owned(),
        C::ShowValue(i) => match w.map_value_names().get(i) {
            Some(name) => switch(
                edits.shown_values.contains(name),
                &format!("{name} on the map"),
            ),
            None => "Show a value on the map".to_owned(),
        },
        C::MapChunks => switch(edits.chunks, "the map's chunks"),
        C::MapWalls => switch(edits.walls, "what's solid on the map"),
        other => other.fixed_name().to_owned(),
    }
}

/// A switch's name: what choosing it does now.
fn on_off(turn_on: &str, turn_off: &str, on: bool) -> String {
    if on { turn_off } else { turn_on }.to_owned()
}
