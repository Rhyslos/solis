//! What Preferences and Project settings hold (Screens 8 and 12; round
//! 2's `11-prefs-*` and `12-project-settings-*`): their sub-tabs, every
//! setting with its name and the line that explains it, and the sections
//! each sub-tab lays them out in, unfinished rows in their places. The
//! search looks in this table, across every sub-tab of the page.
//!
//! Preferences are the editor's own, kept on this computer; Project
//! settings belong to the open project, kept in its folder.

use crate::unbuilt::{Entry, UnbuiltId};

/// A sub-tab of Preferences or Project settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Tab {
    /// Preferences › General: updates, hints, unfinished parts.
    #[default]
    General,
    /// Preferences › Appearance: the accent, scale, density, the layout
    /// and its sizes.
    Appearance,
    /// Preferences › Viewport: what it shows, snapping, the camera, looks.
    Viewport,
    /// Preferences › Shortcuts: every key, to read.
    Shortcuts,
    /// Project settings › General: the name, where the game starts, the
    /// engine.
    Project,
    /// Project settings › 2D: the four pixel settings (answer 24).
    TwoD,
    /// Project settings › Input: the game's input actions.
    Input,
    /// Project settings › Build: Export for playtesting.
    Build,
}

impl Tab {
    /// Preferences' sub-tabs, with the unfinished ones in their places.
    pub(crate) fn preferences() -> Vec<Entry<Self>> {
        vec![
            Entry::Built(Self::General),
            Entry::Built(Self::Appearance),
            Entry::Built(Self::Viewport),
            Entry::Unbuilt(UnbuiltId::PrefsTabInput),
            Entry::Built(Self::Shortcuts),
            Entry::Unbuilt(UnbuiltId::PrefsTabSourceControl),
        ]
    }

    /// Project settings' sub-tabs, with the unfinished ones in their
    /// places.
    pub(crate) fn project() -> Vec<Entry<Self>> {
        vec![
            Entry::Built(Self::Project),
            Entry::Built(Self::TwoD),
            Entry::Unbuilt(UnbuiltId::ProjTabPhysics),
            Entry::Built(Self::Input),
            Entry::Unbuilt(UnbuiltId::ProjTabAudio),
            Entry::Unbuilt(UnbuiltId::ProjTabPlatforms),
            Entry::Built(Self::Build),
            Entry::Unbuilt(UnbuiltId::ProjTabPlugins),
        ]
    }

    /// Its word on the sub-tab.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::General | Self::Project => "General",
            Self::Appearance => "Appearance",
            Self::Viewport => "Viewport",
            Self::Shortcuts => "Shortcuts",
            Self::TwoD => "2D",
            Self::Input => "Input",
            Self::Build => "Build",
        }
    }

    /// Whether it is one of Project settings' (the project's own).
    pub(crate) fn is_project(self) -> bool {
        matches!(self, Self::Project | Self::TwoD | Self::Input | Self::Build)
    }

    /// Its sections: small capitals over the settings in them.
    pub(crate) fn sections(self) -> Vec<(&'static str, Vec<Entry<Setting>>)> {
        use Entry::{Built, Unbuilt};
        use Setting as S;
        match self {
            Self::General => vec![
                ("Starting", vec![Built(S::AutoUpdate)]),
                ("Help", vec![Built(S::Hints), Built(S::GuideBrowser)]),
                ("Unfinished parts", vec![Built(S::HideUnfinished)]),
            ],
            Self::Appearance => vec![
                (
                    "Theme",
                    vec![Built(S::Accent), Built(S::Scale), Built(S::Density)],
                ),
                (
                    "Layout",
                    vec![
                        Built(S::ShelfOpensOn),
                        Built(S::Layout),
                        Built(S::ResetLayout),
                        Unbuilt(UnbuiltId::PrefsCodeSplit),
                    ],
                ),
            ],
            Self::Viewport => vec![
                (
                    "Show in the viewport",
                    vec![
                        Built(S::Stats),
                        Built(S::Pills),
                        Built(S::Toolbox),
                        Built(S::PreviewAnimation),
                    ],
                ),
                (
                    "Snapping",
                    vec![
                        Built(S::GridStep),
                        Built(S::AngleStep),
                        Built(S::ScaleStep),
                        Built(S::PixelSnap),
                    ],
                ),
                (
                    "Camera and look",
                    vec![
                        Built(S::CameraSpeed),
                        Built(S::Views),
                        Built(S::Look),
                        Built(S::Bands),
                        Built(S::Outlines),
                    ],
                ),
            ],
            Self::Project => vec![
                (
                    "Identity",
                    vec![
                        Built(S::ProjectName),
                        Unbuilt(UnbuiltId::ProjCompany),
                        Unbuilt(UnbuiltId::ProjVersion),
                        Unbuilt(UnbuiltId::ProjIcon),
                    ],
                ),
                (
                    "Startup",
                    vec![
                        Built(S::StartsIn),
                        Unbuilt(UnbuiltId::ProjEditorStart),
                        Unbuilt(UnbuiltId::ProjFrameRate),
                    ],
                ),
                ("Engine", vec![Built(S::EngineVersion), Built(S::Scripting)]),
            ],
            Self::TwoD => vec![(
                "Pixels",
                vec![
                    Built(S::BaseResolution),
                    Built(S::PixelsPerUnit),
                    Built(S::TileSize),
                    Built(S::Scaling),
                ],
            )],
            Self::Build => vec![(
                "Export for playtesting",
                vec![
                    Built(S::GameName),
                    Built(S::StartsIn),
                    Built(S::Window),
                    Built(S::ExportFolder),
                ],
            )],
            // Laid out as tables of their own.
            Self::Shortcuts | Self::Input => Vec::new(),
        }
    }
}

/// One setting: its name and its line, which the search looks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Setting {
    AutoUpdate,
    Hints,
    GuideBrowser,
    HideUnfinished,
    Accent,
    Scale,
    Density,
    ShelfOpensOn,
    Layout,
    ResetLayout,
    Stats,
    Pills,
    Toolbox,
    PreviewAnimation,
    GridStep,
    AngleStep,
    ScaleStep,
    PixelSnap,
    CameraSpeed,
    Views,
    Look,
    Bands,
    Outlines,
    ProjectName,
    StartsIn,
    EngineVersion,
    Scripting,
    BaseResolution,
    PixelsPerUnit,
    TileSize,
    Scaling,
    GameName,
    Window,
    ExportFolder,
}

impl Setting {
    /// Every setting, in the pages' order.
    pub(crate) const ALL: [Self; 34] = [
        Self::AutoUpdate,
        Self::Hints,
        Self::GuideBrowser,
        Self::HideUnfinished,
        Self::Accent,
        Self::Scale,
        Self::Density,
        Self::ShelfOpensOn,
        Self::Layout,
        Self::ResetLayout,
        Self::Stats,
        Self::Pills,
        Self::Toolbox,
        Self::PreviewAnimation,
        Self::GridStep,
        Self::AngleStep,
        Self::ScaleStep,
        Self::PixelSnap,
        Self::CameraSpeed,
        Self::Views,
        Self::Look,
        Self::Bands,
        Self::Outlines,
        Self::ProjectName,
        Self::StartsIn,
        Self::EngineVersion,
        Self::Scripting,
        Self::BaseResolution,
        Self::PixelsPerUnit,
        Self::TileSize,
        Self::Scaling,
        Self::GameName,
        Self::Window,
        Self::ExportFolder,
    ];

    /// The sub-tab it is on (the area the game starts in is on two: its
    /// home is Build, where the export reads it).
    pub(crate) fn tab(self) -> Tab {
        use Setting as S;
        match self {
            S::AutoUpdate | S::Hints | S::GuideBrowser | S::HideUnfinished => Tab::General,
            S::Accent | S::Scale | S::Density | S::ShelfOpensOn | S::Layout | S::ResetLayout => {
                Tab::Appearance
            }
            S::Stats
            | S::Pills
            | S::Toolbox
            | S::PreviewAnimation
            | S::GridStep
            | S::AngleStep
            | S::ScaleStep
            | S::PixelSnap
            | S::CameraSpeed
            | S::Views
            | S::Look
            | S::Bands
            | S::Outlines => Tab::Viewport,
            S::ProjectName | S::EngineVersion | S::Scripting => Tab::Project,
            S::BaseResolution | S::PixelsPerUnit | S::TileSize | S::Scaling => Tab::TwoD,
            S::StartsIn | S::GameName | S::Window | S::ExportFolder => Tab::Build,
        }
    }

    /// Its name.
    pub(crate) fn name(self) -> &'static str {
        use Setting as S;
        match self {
            S::AutoUpdate => "Look for updates when the editor starts",
            S::Hints => "Show hints",
            S::GuideBrowser => "Open the guide in",
            S::HideUnfinished => "Hide unfinished parts",
            S::Accent => "Accent colour",
            S::Scale => "Interface scale",
            S::Density => "Density",
            S::ShelfOpensOn => "Shelf opens on",
            S::Layout => "Panel layout",
            S::ResetLayout => "Sizes",
            S::Stats => "Frame time and frame rate",
            S::Pills => "Axis locks, axes and camera speed",
            S::Toolbox => "Toolbox",
            S::PreviewAnimation => "Preview animation",
            S::GridStep => "Grid step",
            S::AngleStep => "Angle step",
            S::ScaleStep => "Scale step",
            S::PixelSnap => "In the 2D view, moves land on",
            S::CameraSpeed => "Camera speed",
            S::Views => "Views",
            S::Look => "Look",
            S::Bands => "Bands of light",
            S::Outlines => "Outlines",
            S::ProjectName => "Project name",
            S::StartsIn => "Game starts in",
            S::EngineVersion => "Engine version",
            S::Scripting => "Scripting",
            S::BaseResolution => "Base resolution",
            S::PixelsPerUnit => "Pixels per unit",
            S::TileSize => "Tile size",
            S::Scaling => "Scaling to the screen",
            S::GameName => "Game's name",
            S::Window => "Window",
            S::ExportFolder => "Export",
        }
    }

    /// The line under its name.
    pub(crate) fn hint(self) -> &'static str {
        use Setting as S;
        match self {
            S::AutoUpdate => {
                "Asks GitHub for a newer build at each start, in the background. Off, nothing is \
                 asked until you choose Check now on File › Updates."
            }
            S::Hints => {
                "The Hints card under Properties says what's selected and what you can do with it."
            }
            S::GuideBrowser => {
                "The program that shows the user guide when you click Documentation: your \
                 default browser, one installed on this computer, or one you choose."
            }
            S::HideUnfinished => {
                "Parts of the editor still to come are shown faint, with a small square, so the \
                 layout is the finished one. Turn this on to take them all away."
            }
            S::Accent => "Used for selection, the tool in use and focus.",
            S::Scale => "Scales text and controls. Useful on very sharp screens.",
            S::Density => "Compact fits more rows; Comfortable is easier to click.",
            S::ShelfOpensOn => "Which shelf tab comes to the front when you select something.",
            S::Layout => {
                "Where the Area, Properties and Assets lists sit on Home: columns beside the \
                 view, or on the shelf as designed. Changes at once."
            }
            S::ResetLayout => {
                "Puts this tab's shelf, groups and every border you dragged back as they started."
            }
            S::Stats => {
                "The pill at the top right. Hover it for the count of entities and triangles."
            }
            S::Pills => {
                "The pills at the bottom right: Lock X Y Z, World or Object, and the camera's speed."
            }
            S::Toolbox => {
                "The column of tools at the left. The keys Q, W, R, S and T work either way."
            }
            S::PreviewAnimation => {
                "Sprites play their clips while you edit. Off, each shows its clip's frame and \
                 the editor rests. The game always animates."
            }
            S::GridStep => {
                "Moves land on a grid of this size. The Home ribbon turns it on and off."
            }
            S::AngleStep => "Turning goes in steps of this many degrees.",
            S::ScaleStep => "Scaling goes in steps of ten percent.",
            S::PixelSnap => {
                "Whole pixels keep pixel art crisp; tiles follow the project's tile size."
            }
            S::CameraSpeed => {
                "How fast the camera flies. Also changed by holding the right mouse button and \
                 turning the wheel."
            }
            S::Views => "How many views the viewport is split into.",
            S::Look => {
                "How the viewport shades models while you edit. The game always uses the \
                 standard look."
            }
            S::Bands => {
                "In the stylized look: how many steps the light goes in, from shadow to full light."
            }
            S::Outlines => "In the stylized look: dark edges round models.",
            S::ProjectName => {
                "Shown in the window's title and on Recent projects. The folder keeps its own name."
            }
            S::StartsIn => "The area the exported game opens in.",
            S::EngineVersion => "The build of the editor that last opened this project.",
            S::Scripting => {
                "Game code is written in Lust and opens in your code editor. A code editor \
                 inside the editor comes later."
            }
            S::BaseResolution => {
                "The size the game is drawn at, in pixels, before it's scaled up to fill the \
                 screen."
            }
            S::PixelsPerUnit => {
                "How many pixels make one metre of the world. Usually the tile size, so a tile \
                 is one metre."
            }
            S::TileSize => "The grid of the 2D view and of new tile maps, in pixels.",
            S::Scaling => {
                "Whole steps keeps every pixel the same size, with bars at the edges if need be. \
                 Fill the screen has no bars and smooths only the edge of each pixel."
            }
            S::GameName => {
                "The program's name, the window's title and the player's saves folder. Keep it \
                 the same between builds, so saves carry over."
            }
            S::Window => {
                "How the game's window first opens. After that the player's own choice is kept."
            }
            S::ExportFolder => {
                "Makes a folder holding the game, ready to zip and send. Where it goes is \
                 remembered on this computer, never in the project."
            }
        }
    }

    /// Whether the search `words` (already lower case) find it.
    pub(crate) fn matches(self, words: &str) -> bool {
        let text = format!("{} {} {}", self.tab().name(), self.name(), self.hint()).to_lowercase();
        words.split_whitespace().all(|word| text.contains(word))
    }
}

/// The settings the search finds on one page (Preferences or Project
/// settings), in the page's order.
pub(crate) fn found(project: bool, words: &str) -> Vec<Setting> {
    Setting::ALL
        .into_iter()
        .filter(|s| s.tab().is_project() == project && s.matches(words))
        .collect()
}
