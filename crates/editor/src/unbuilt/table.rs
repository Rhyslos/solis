//! The table of unfinished parts: one line for each thing the design
//! shows that Solis doesn't do yet.
//!
//! A line is the part's name in code, its key (the dotted id of the
//! plan's inventory, `docs/plans/shared-1-ui-remake-inventory.md`, part
//! b, so the plan, the manual and the code name the same thing), the
//! words shown on it, the roadmap phase that builds it, and one plain
//! sentence saying what it will do.
//!
//! **When a part is built, delete its line.** The compiler then points
//! at every place that still draws it as unfinished. A test holds this
//! table against the inventory, so neither can drift from the other.

use super::Phase::{
    NotPlanned, Shared4, Shared6, Shared7, Shared8, Shared10, Shared11, Shared13, ThreeD1, ThreeD5,
    TwoD2,
};
use super::Unbuilt;

macro_rules! unbuilt {
    ($($id:ident: $key:literal => ($name:literal, $phase:expr, $what:literal),)*) => {
        /// An unfinished part, by name. Only the functions of
        /// `crate::unbuilt` may draw one.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum UnbuiltId {
            $(#[doc = concat!("`", $key, "`: ", $name, ".")] $id,)*
        }

        /// Every unfinished part, in the table's order.
        pub const ALL: &[Unbuilt] = &[
            $(Unbuilt {
                id: UnbuiltId::$id,
                key: $key,
                name: $name,
                phase: $phase,
                what: $what,
            },)*
        ];
    };
}

unbuilt! {
    // The frame, on every screen.
    ShellTabShading: "shell.tab.shading" => ("Shading", Shared7, "Materials and shaders, drawn as a graph of nodes"),
    ShellTabVfx: "shell.tab.vfx" => ("VFX", Shared8, "Effects made of particles: smoke, sparks, rain"),
    ShellPlaySimulate: "shell.play.simulate" => ("Simulate", NotPlanned, "Runs physics and game code in the viewport without taking the camera"),
    ShellStatusBranch: "shell.status.branch" => ("Branch and changes", Shared11, "Shows the Git branch you are on and how many files you changed"),

    // Home: the level editor.
    HomeCreateShapes: "home.create.shapes" => ("Ready-made shapes", NotPlanned, "Adds a cube, a ball or another simple shape without a model file"),
    HomeCreateTrigger: "home.create.trigger" => ("Trigger zone", TwoD2, "Adds a zone that tells game code when something enters or leaves it"),
    HomeLightingBake: "home.lighting.bake" => ("Bake", ThreeD5, "Works out bounced light for the area"),
    HomeLightingQuality: "home.lighting.quality" => ("Quality", ThreeD5, "Chooses how carefully bounced light is worked out"),
    HomeLightingLightmapView: "home.lighting.lightmap-view" => ("Light map view", ThreeD5, "Shows the worked-out light on its own in the viewport"),
    BuildTargets: "build.targets" => ("Targets and the full export", Shared11, "A finished game for Windows: an installer, your game's own Rust code built in, and a list of targets to build"),
    BuildTargetWindows: "build.target.windows" => ("Windows", Shared11, "Builds the finished game for Windows"),
    BuildTargetPlatforms: "build.target.platforms" => ("Platforms", Shared11, "Chooses which platforms the finished game is built for"),
    BuildFull: "build.full" => ("Build", Shared11, "Builds the finished game, ready to hand out"),
    BuildInstaller: "build.installer" => ("Installer", Shared11, "Wraps the finished game in an installer"),
    BuildRustCode: "build.rust-code" => ("Your Rust code", Shared11, "Builds your game's own Rust code into the finished game"),
    BuildLog: "build.log" => ("Build log", Shared11, "Everything a build says, step by step"),
    HomeToolPose: "home.tool.pose" => ("Pose bones", ThreeD1, "Turns a model's bones by hand to pose it"),
    HomeToolMotionPath: "home.tool.motion-path" => ("Motion path", Shared4, "Draws a path in the viewport for things to move along"),
    HomeToolAddKey: "home.tool.add-key" => ("Add key", Shared10, "Records the selected entity's place at this moment of a cutscene"),
    HomeToolMeasure: "home.tool.measure" => ("Measure", NotPlanned, "Measures the distance between two points in the area"),
    HomeViewMotionCurve: "home.view.motion-curve" => ("Motion curve", Shared10, "Draws the selected entity's animated path over the viewport"),
    HomeShelfAnimation: "home.shelf.animation" => ("Animation", Shared10, "The selected entity's animation: its tracks, its keys and each key's details"),
    HomeShelfTransport: "home.shelf.transport" => ("Animation transport", Shared10, "Plays the animation and steps to the key before or after"),
    HomeShelfFrameCounter: "home.shelf.frame-counter" => ("Frame counter", Shared10, "Shows which frame of the animation is on screen"),
    HomeObjectModelMaterial: "home.object.model-material" => ("Material", Shared7, "Gives a model a material of your own in place of the one its file brings"),
    HomeObjectModelShadows: "home.object.model-shadows" => ("Shadows", NotPlanned, "Lets a model cast shadows of its own, not only from the sun"),
    HomeConsoleLineLink: "home.console.line-link" => ("Line link", Shared6, "Opens the game code line a Console message came from"),

    // Home: placing.
    PlaceSnapSurface: "place.snap.surface" => ("Snap to surface", NotPlanned, "Rests what you place on the surface under the pointer"),
    PlacePaintBrush: "place.paint.brush" => ("Brush", NotPlanned, "Places copies of a file along a stroke of the mouse"),
    PlacePaintScatter: "place.paint.scatter" => ("Scatter", NotPlanned, "Scatters copies of a file over the ground you drag across"),
    PlaceKeepPlacing: "place.keep-placing" => ("Keep placing", NotPlanned, "Keeps the same file ready so several copies can be placed in a row"),
    PlaceFree: "place.free" => ("Place freely", NotPlanned, "Places without snapping while a key is held"),
    PlaceRotateGhost: "place.rotate-ghost" => ("Turn before placing", NotPlanned, "Turns what you are placing before you let go"),
    PlaceSceneTarget: "place.scene.target" => ("New entities go into", NotPlanned, "Chooses the entity that newly placed things are put under"),
    PlaceAlignSurface: "place.align.surface" => ("Align to surface", NotPlanned, "Tilts what you place to match the surface under it"),
    PlaceRotation: "place.rotation" => ("Rotation", NotPlanned, "Chooses how placed things are turned"),
    PlaceRandomYaw: "place.random-yaw" => ("Random yaw", NotPlanned, "Turns each placed thing a random way"),
    PlaceScale: "place.scale" => ("Scale", NotPlanned, "Sets how large placed things are"),
    PlacePhysics: "place.physics" => ("Physics", NotPlanned, "Lets placed things settle under gravity as they land"),
    PlaceChipFavorites: "place.chip.favorites" => ("Favourites", NotPlanned, "Shows the files you starred"),
    PlaceChipRecent: "place.chip.recent" => ("Recent", NotPlanned, "Shows the files you placed most recently"),
    PlaceClickToPlace: "place.click-to-place" => ("Click to place", NotPlanned, "Places a file by clicking its tile and then a spot in the area"),

    // Library.
    LibImportAnyKind: "lib.import.any-kind" => ("Import any kind of file", NotPlanned, "Brings in pictures, sounds and every other kind of file through one window"),
    LibFilterTags: "lib.filter.tags" => ("Tags", NotPlanned, "Sorts files by tags you give them"),
    LibFilterMissing: "lib.filter.missing" => ("Missing references", NotPlanned, "Shows only the files that point at something that is gone"),
    LibSourceCheckIn: "lib.source.check-in" => ("Check in", Shared11, "Saves your changes into the project's Git history"),
    LibSourceHistory: "lib.source.history" => ("History", Shared11, "Lists the earlier versions of the selected file"),
    LibUseShowInLevel: "lib.use.show-in-level" => ("Show in area", NotPlanned, "Finds where the selected file is used in the open area"),
    LibPreviewModes: "lib.preview.modes" => ("Preview modes", NotPlanned, "Turns the preview around and shows it lit, as wires or by its picture layout"),
    LibUsedByAllKinds: "lib.used-by.all-kinds" => ("Used by", NotPlanned, "Lists every area and assembly that uses the selected file"),
    LibShelfImportQueue: "lib.shelf.import-queue" => ("Import queue", NotPlanned, "Lists the files being brought in, with how far each has got"),
    LibMultiSelect: "lib.multi-select" => ("Select several files", NotPlanned, "Selects several files at once with Ctrl or Shift"),
    LibDragToHome: "lib.drag-to-home" => ("Drag to Home", NotPlanned, "Carries a file to the Home tab by dragging it onto the tab"),

    // Shading: a whole screen.
    ShadingMaterialApply: "shading.material.apply" => ("Apply", Shared7, "Puts the material on what is selected in the area"),
    ShadingMaterialSave: "shading.material.save" => ("Save", Shared7, "Saves the material"),
    ShadingAddTexture: "shading.add.texture" => ("Picture", Shared7, "Adds a node that reads a picture"),
    ShadingAddMath: "shading.add.math" => ("Math", Shared7, "Adds a node that does arithmetic"),
    ShadingAddParam: "shading.add.param" => ("Param", Shared7, "Adds a value that each use of the material can set"),
    ShadingGraphAutoLayout: "shading.graph.auto-layout" => ("Auto-layout", Shared7, "Tidies the nodes into rows"),
    ShadingGraphComment: "shading.graph.comment" => ("Comment frame", Shared7, "Draws a named frame round a group of nodes"),
    ShadingGraphWires: "shading.graph.wires" => ("Wires", Shared7, "Chooses whether wires are drawn curved or straight"),
    ShadingPreviewShape: "shading.preview.shape" => ("Shape", Shared7, "Chooses the shape the material is previewed on"),
    ShadingPreviewLit: "shading.preview.lit" => ("Lit", Shared7, "Lights the preview"),
    ShadingPreviewLive: "shading.preview.live" => ("Live update", Shared7, "Redraws the preview as the graph changes"),
    ShadingGraph: "shading.graph" => ("Material graph", Shared7, "The material as boxes joined by wires, to build and change"),
    ShadingPreview: "shading.preview" => ("Material preview", Shared7, "The material on a simple shape, as the game will draw it"),
    ShadingShelfParameters: "shading.shelf.parameters" => ("Parameters", Shared7, "The values each use of the material can set"),
    ShadingShelfNode: "shading.shelf.node" => ("Node", Shared7, "The selected node's settings"),
    ShadingShelfPalette: "shading.shelf.palette" => ("Palette", Shared7, "Every kind of node, to drag into the graph"),
    ShadingShelfInstances: "shading.shelf.instances" => ("Instances", Shared7, "The materials made from this one"),

    // Animate: the sequencer.
    AnimKeysAdd: "anim.keys.add" => ("Add key", Shared10, "Records the selected values at this moment of a cutscene"),
    AnimKeysAuto: "anim.keys.auto" => ("Auto key", Shared10, "Records a key whenever you change something"),
    AnimInterpStep: "anim.interp.step" => ("Step", Shared10, "Holds each key's value until the next key"),
    AnimInterpLinear: "anim.interp.linear" => ("Linear", Shared10, "Moves evenly from each key to the next"),
    AnimInterpSmooth: "anim.interp.smooth" => ("Smooth", Shared10, "Eases in and out of each key"),
    AnimTracksTrack: "anim.tracks.track" => ("Track", Shared10, "Adds a track for an entity or one of its values"),
    AnimTracksCurves: "anim.tracks.curves" => ("Curves", Shared10, "Shows the selected track as a curve to shape"),
    AnimCamera: "anim.camera" => ("Shot camera", Shared10, "The cutscene through its camera, framed as the game will show it"),
    AnimCurves: "anim.curves" => ("Curve editor", Shared10, "A track's values over time, as curves to shape"),
    AnimShelfSequencer: "anim.shelf.sequencer" => ("Sequencer", Shared10, "A cutscene's tracks, clips and keys along a timeline"),
    AnimShelfKey: "anim.shelf.key" => ("Key", Shared10, "The selected key's time, value and easing"),

    // Debug.
    DebugCaptureRecord: "debug.capture.record" => ("Record", Shared6, "Starts keeping every frame's timings for a closer look"),
    DebugCapturePause: "debug.capture.pause" => ("Pause", Shared6, "Stops taking in new frames so the ones kept can be read"),
    DebugCaptureSave: "debug.capture.save" => ("Save", Shared6, "Saves the kept frames to a file"),
    DebugTargetSource: "debug.target.source" => ("Target", Shared11, "Chooses a built game to measure in place of the one playing in the editor"),
    DebugTargetDeep: "debug.target.deep" => ("Deep profile", NotPlanned, "Times every function of game code, at some cost in speed"),
    DebugShowMemory: "debug.show.memory" => ("Memory", NotPlanned, "Shows how much memory each frame uses"),
    DebugToolsDebugger: "debug.tools.debugger" => ("Debugger", NotPlanned, "Stops game code at a line and steps through it"),
    DebugToolsMemory: "debug.tools.memory" => ("Memory", NotPlanned, "Shows what the game's memory is spent on"),
    DebugTimelineThreads: "debug.timeline.threads" => ("Thread lanes", NotPlanned, "Shows each thread's work in a lane of its own, block by named block"),
    DebugStatsMemory: "debug.stats.memory" => ("Memory used", NotPlanned, "Shows how much memory the game is using"),
    DebugShelfHottest: "debug.shelf.hottest" => ("Hottest functions", NotPlanned, "Lists the functions of game code that take the most time"),
    DebugShelfMemory: "debug.shelf.memory" => ("Memory snapshot", NotPlanned, "Lists everything in the game's memory at one moment"),
    DebugStatus: "debug.status" => ("Recording", Shared6, "Says in the status bar that frames are being recorded, and how many"),

    // File: projects.
    FilePlugins: "file.plugins" => ("Plugins", Shared11, "Adds to the editor with plugins, and manages the ones you have"),
    RecentCardThumbnail: "recent.card.thumbnail" => ("Project picture", NotPlanned, "Shows a picture of the project on its card"),
    RecentCardBadge: "recent.card.badge" => ("2D or 3D badge", NotPlanned, "Marks a project as 2D or 3D on its card"),
    RecentTemplateFirstPerson: "recent.template.first-person" => ("First-person", Shared13, "Starts a project with a first-person player ready to walk around"),
    RecentTemplateTopDown: "recent.template.top-down" => ("Top-down", Shared13, "Starts a project seen from above, with a player to steer"),
    RecentTemplatePlatformer: "recent.template.platformer" => ("Platformer", Shared13, "Starts a 2D project with a player that runs and jumps"),
    RecentTemplateAll: "recent.template.all" => ("All templates", Shared13, "Shows every template a project can start from"),

    // File: Preferences.
    PrefsTabInput: "prefs.tab.input" => ("Input", NotPlanned, "The editor's own mouse and key settings"),
    PrefsShortcutsEdit: "prefs.shortcuts.edit" => ("Change a shortcut", NotPlanned, "Gives an action of the editor another key"),
    PrefsTabSourceControl: "prefs.tab.source-control" => ("Source control", Shared11, "How the editor works with Git"),
    PrefsCodeSplit: "prefs.code-split" => ("Code editor split", NotPlanned, "Shows game code next to the viewport when a code file is open"),

    // Library: the model viewer, a whole screen.
    MeshTab: "mesh.tab" => ("Model viewer", NotPlanned, "A model on its own: its levels of detail, its picture's layout, where it joins and how it was imported"),
    MeshViewLit: "mesh.view.lit" => ("Lit", NotPlanned, "Shows the model lit, as the game draws it"),
    MeshViewUnlit: "mesh.view.unlit" => ("Unlit", NotPlanned, "Shows the model's own colours with no light"),
    MeshViewWire: "mesh.view.wire" => ("Wire", NotPlanned, "Shows the model's triangles as lines"),
    MeshViewNormals: "mesh.view.normals" => ("Normals", NotPlanned, "Colours the model by which way each surface faces"),
    MeshViewUvCheck: "mesh.view.uv-check" => ("UV check", NotPlanned, "Covers the model in a checked pattern to show how pictures stretch over it"),
    MeshShowBounds: "mesh.show.bounds" => ("Bounds", NotPlanned, "Draws the box the model fits in, with its measurements"),
    MeshShowPivot: "mesh.show.pivot" => ("Pivot", NotPlanned, "Marks the point the model turns round"),
    MeshShowCollision: "mesh.show.collision" => ("Collision", NotPlanned, "Draws the shapes physics uses for the model"),
    MeshLightPreset: "mesh.light.preset" => ("Lighting preset", NotPlanned, "Chooses the light the model is looked at in"),
    MeshLightRotate: "mesh.light.rotate" => ("Rotate light", NotPlanned, "Turns the light round the model"),
    MeshLightBackground: "mesh.light.background" => ("Background", NotPlanned, "Chooses what is behind the model"),
    MeshLodAuto: "mesh.lod.auto" => ("Auto", NotPlanned, "Picks the level of detail by distance, as the game does"),
    MeshLodPick: "mesh.lod.pick" => ("Level of detail", NotPlanned, "Shows one level of detail whatever the distance"),
    MeshLodDensity: "mesh.lod.density" => ("Show triangle density", NotPlanned, "Colours the model by how small its triangles are on screen"),
    MeshToolsReimport: "mesh.tools.reimport" => ("Reimport", NotPlanned, "Reads the model's file again with other import settings"),
    MeshToolsPivot: "mesh.tools.pivot" => ("Pivot", NotPlanned, "Moves the point the model turns round"),
    MeshToolsShot: "mesh.tools.shot" => ("Shot", NotPlanned, "Takes the picture the Library shows for the model"),
    MeshUseShowInLevel: "mesh.use.show-in-level" => ("Show in area", NotPlanned, "Finds where the model is used in the open area"),
    MeshViewport: "mesh.viewport" => ("Model view", NotPlanned, "The model by itself in a studio light, to turn and measure"),
    MeshUv: "mesh.uv" => ("UV layout", NotPlanned, "How the model's surface is laid flat for its pictures"),
    MeshShelfOverview: "mesh.shelf.overview" => ("Overview", NotPlanned, "The model's levels of detail, material slots and import facts"),
    MeshShelfLods: "mesh.shelf.lods" => ("Levels of detail", NotPlanned, "Each level of detail, with its triangles and the distance it shows from"),
    MeshShelfMaterials: "mesh.shelf.materials" => ("Materials", Shared7, "The model's material slots and what fills each"),
    MeshShelfCollision: "mesh.shelf.collision" => ("Collision", NotPlanned, "The shapes physics uses for the model"),
    MeshShelfSockets: "mesh.shelf.sockets" => ("Sockets", NotPlanned, "Named points on the model that other things attach to"),
    MeshShelfImport: "mesh.shelf.import" => ("Import settings", NotPlanned, "How the model's file is read: its scale, its axes and what is kept"),

    // Audio.
    AudioMixerSend: "audio.mixer.send" => ("Send", NotPlanned, "Sends part of a bus's sound to another bus as well"),
    AudioMixerSnapshot: "audio.mixer.snapshot" => ("Snapshot", NotPlanned, "Keeps the mixer's settings under a name, to switch to while the game plays"),
    AudioMonitorMeters: "audio.monitor.meters" => ("Meters", NotPlanned, "Chooses what the meters measure"),
    AudioListenChoose: "audio.listen.choose" => ("Choose who listens", NotPlanned, "Chooses where the editor hears the area from"),
    AudioFxEq: "audio.fx.eq" => ("EQ", NotPlanned, "Makes a bus's low, middle or high sounds louder or quieter"),
    AudioFxCompress: "audio.fx.compress" => ("Compress", NotPlanned, "Evens out a bus's loud and quiet moments"),
    AudioFxReverb: "audio.fx.reverb" => ("Reverb", NotPlanned, "Gives a bus the echo of a room"),
    AudioFxLimiter: "audio.fx.limiter" => ("Limiter", NotPlanned, "Stops a bus from ever getting louder than a set level"),
    AudioStripInserts: "audio.strip.inserts" => ("Insert", NotPlanned, "A place on a bus's strip for an effect"),
    AudioStripPan: "audio.strip.pan" => ("Pan", NotPlanned, "Moves a bus's sound towards the left or the right"),
    AudioMetersStereo: "audio.meters.stereo" => ("Stereo meters", NotPlanned, "Measures the left and right of a bus apart, holding each one's peak"),
    AudioEffect: "audio.effect" => ("Bus effects", NotPlanned, "An equaliser, a compressor, reverb and a limiter on a bus, each with its settings"),
    AudioEventUsedBy: "audio.event.used-by" => ("Used by", NotPlanned, "Lists what plays the selected sound event"),

    // VFX: a whole screen.
    VfxEmitterAdd: "vfx.emitter.add" => ("Emitter", Shared8, "Adds a source of particles to the effect"),
    VfxEmitterDuplicate: "vfx.emitter.duplicate" => ("Duplicate", Shared8, "Copies the selected emitter"),
    VfxModuleSpawn: "vfx.module.spawn" => ("Spawn", Shared8, "Adds a rule for how many particles appear, and when"),
    VfxModuleVelocity: "vfx.module.velocity" => ("Velocity", Shared8, "Adds a rule for how particles move"),
    VfxModuleColor: "vfx.module.color" => ("Colour", Shared8, "Adds a rule for particles' colour over their life"),
    VfxModuleSize: "vfx.module.size" => ("Size", Shared8, "Adds a rule for particles' size over their life"),
    VfxModuleCollide: "vfx.module.collide" => ("Collide", Shared8, "Adds a rule for particles hitting the world"),
    VfxSimLoop: "vfx.sim.loop" => ("Loop", Shared8, "Plays the effect over and over"),
    VfxSimSpeed: "vfx.sim.speed" => ("Speed", Shared8, "Plays the effect faster or slower"),
    VfxSimBounds: "vfx.sim.bounds" => ("Show bounds", Shared8, "Draws the box the effect stays in"),
    VfxPreviewBackground: "vfx.preview.background" => ("Background", Shared8, "Chooses what is behind the effect"),
    VfxPreviewTime: "vfx.preview.time" => ("Time of day", Shared8, "Chooses the light the effect is looked at in"),
    VfxPreviewFollow: "vfx.preview.follow" => ("Follow emitter", Shared8, "Keeps the camera on the emitter as it moves"),
    VfxPreview: "vfx.preview" => ("Effect preview", Shared8, "The effect playing, as the game will draw it"),
    VfxStack: "vfx.stack" => ("Emitter stack", Shared8, "Each emitter's rules in order, to add to and change"),
    VfxShelfModule: "vfx.shelf.module" => ("Module", Shared8, "The selected rule's settings"),
    VfxShelfParameters: "vfx.shelf.parameters" => ("Parameters", Shared8, "The values game code can set on the effect"),
    VfxShelfPerformance: "vfx.shelf.performance" => ("Performance", Shared8, "How many particles the effect makes and what they cost"),

    // File: Project settings.
    ProjTabPhysics: "proj.tab.physics" => ("Physics", NotPlanned, "The project's physics settings"),
    ProjTabAudio: "proj.tab.audio" => ("Audio", NotPlanned, "The project's sound settings"),
    ProjTabPlatforms: "proj.tab.platforms" => ("Platforms", Shared11, "What the finished game is built for"),
    ProjTabPlugins: "proj.tab.plugins" => ("Plugins", Shared11, "The plugins this project uses"),
    ProjCompany: "proj.company" => ("Company", Shared11, "The maker's name, written into the finished game"),
    ProjVersion: "proj.version" => ("Version", Shared11, "The game's own version number"),
    ProjIcon: "proj.icon" => ("Project icon", Shared11, "The picture the finished game's file and window show"),
    ProjEditorStart: "proj.editor-start" => ("Editor start area", NotPlanned, "The area the editor opens the project in"),
    ProjFrameRate: "proj.frame-rate" => ("Target frame rate", NotPlanned, "How many frames a second the game aims to draw"),
    ProjVersionControl: "proj.version-control" => ("Version control", Shared11, "Checks the project's settings in to Git and shows what changed"),

    // File: Project settings, Input.
    InputContexts: "input.contexts" => ("Contexts", NotPlanned, "Sets of actions that take turns, such as walking, driving and menus"),
    InputTrigger: "input.trigger" => ("Fires on", NotPlanned, "Chooses whether an action fires when pressed, held or let go"),
    InputGroups: "input.groups" => ("Groups", NotPlanned, "Gathers actions under headings"),
    InputBuffer: "input.buffer" => ("Buffer", NotPlanned, "Remembers a press for a moment, so one made slightly early still counts"),
    InputDeadZone: "input.dead-zone" => ("Stick dead zone", NotPlanned, "How far a stick must move before it counts"),
    InputInvertY: "input.invert-y" => ("Invert look Y", NotPlanned, "Swaps up and down when looking around"),

    // What crosses screens.
    CrossBakeNow: "cross.bake-now" => ("Bake now", ThreeD5, "Works out bounced light straight from the problem that asks for it"),
}
