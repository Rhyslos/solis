//! The icon table: every icon's name and its drawing, as path text on a
//! 16 × 16 grid ([`super::reader`] says how the text is read).
//!
//! The first part is the project lead's own drawings, copied from the
//! design's screens as they are written there. The second is what Solis
//! needs that the design doesn't draw, made on the same grid and approved
//! on the icon sheets (`docs/design/g-layout/additions/round-1`). Nothing
//! is taken from an icon library or another program.
//!
//! An entry is the icon's name in code, its name in the design's files,
//! how heavy its lines are ([`Weight`]), and its parts: `lines` are
//! stroked, `fills` filled, `dashed` stroked in dashes (so long on, so
//! long off, in grid units), `faint` stroked at half strength.

/// How heavy an icon's lines are, as the design draws that kind of icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    /// Nearly every icon.
    Line,
    /// Small marks that must carry: an error's cross, a warning, close,
    /// back.
    Mark,
    /// The smallest chevrons.
    Chevron,
}

/// One icon's drawing: its parts, each a list of paths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawing {
    /// How heavy its lines are.
    pub weight: Weight,
    /// Paths stroked with the icon's colour.
    pub lines: &'static [&'static str],
    /// Paths filled with it.
    pub fills: &'static [&'static str],
    /// Paths stroked in dashes.
    pub dashed: &'static [&'static str],
    /// A dash's length and the gap after it, in grid units.
    pub dash: [f32; 2],
    /// Paths stroked at half strength.
    pub faint: &'static [&'static str],
}

macro_rules! icons {
    (@dash) => {
        [0.0, 0.0]
    };
    (@dash $on:literal, $off:literal) => {
        [$on, $off]
    };
    ($(
        $(#[$doc:meta])*
        $name:ident = $label:literal, $weight:ident
        $(, lines: [$($line:literal),* $(,)?])?
        $(, fills: [$($fill:literal),* $(,)?])?
        $(, dashed($on:literal, $off:literal): [$($dash:literal),* $(,)?])?
        $(, faint: [$($faint:literal),* $(,)?])?
        ;
    )*) => {
        /// An icon: a small line drawing on a tab, a button or a row.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Icon {
            $($(#[$doc])* $name,)*
        }

        impl Icon {
            /// Every icon, in the table's order.
            pub const ALL: &'static [Self] = &[$(Self::$name),*];

            /// The icon's name in the design's files and on the sheets.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$name => $label,)*
                }
            }

            /// The icon's drawing.
            pub fn drawing(self) -> Drawing {
                match self {
                    $(Self::$name => Drawing {
                        weight: Weight::$weight,
                        lines: &[$($($line),*)?],
                        fills: &[$($($fill),*)?],
                        dashed: &[$($($dash),*)?],
                        dash: icons!(@dash $($on, $off)?),
                        faint: &[$($($faint),*)?],
                    },)*
                }
            }
        }
    };
}

icons! {
    // The design's own drawings, as its screens have them.
    /// A box's outline: the selected entity, in the Properties header.
    CubeOutline = "cube-outline", Line,
        lines: ["M8 1.5l5.5 3v7L8 14.5 2.5 11.5v-7z"];
    /// A magnifying glass: search.
    Search = "search", Line,
        lines: ["M2.5 7a4.5 4.5 0 1 0 9 0a4.5 4.5 0 1 0 -9 0z", "M10.5 10.5L14 14"];
    /// A chevron pointing up: collapse the ribbon.
    ChevronUp = "chevron-up", Chevron,
        lines: ["M4 10l4-4 4 4"];
    /// A box seen from a corner: a model.
    Model = "cube", Line,
        lines: ["M8 1.5l5.5 3v7L8 14.5 2.5 11.5v-7z", "M2.5 4.5L8 7.5l5.5-3M8 7.5v7"];
    /// A lamp's bulb: a light.
    Light = "light", Line,
        lines: ["M4.5 7a3.5 3.5 0 1 0 7 0a3.5 3.5 0 1 0 -7 0z", "M6.5 12.5h3M7 14.5h2"];
    /// A film camera: a Camera entity.
    Camera = "camera", Line,
        lines: [
            "M2.5 4.5h7a1 1 0 0 1 1 1v5a1 1 0 0 1 -1 1h-7a1 1 0 0 1 -1 -1v-5a1 1 0 0 1 1 -1z",
            "M10.5 7l4-2v6l-4-2",
        ];
    /// A dashed square: a volume.
    Volume = "volume", Line,
        dashed(2.0, 1.5): ["M3 2h10a1 1 0 0 1 1 1v10a1 1 0 0 1 -1 1h-10a1 1 0 0 1 -1 -1v-10a1 1 0 0 1 1 -1z"];
    /// A sun with eight rays: lighting, the sun.
    Sun = "sun", Line,
        lines: [
            "M5 8a3 3 0 1 0 6 0a3 3 0 1 0 -6 0z",
            "M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.4 1.4M11.6 11.6L13 13M3 13l1.4-1.4M11.6 4.4L13 3",
        ];
    /// A disk: Save.
    Save = "save", Line,
        lines: ["M3 2.5h8l2.5 2.5v8.5h-10.5z", "M5.5 2.5v3.5h5V2.5"];
    /// A tag with a tail: Build.
    Build = "build", Line,
        lines: ["M2 13l3-3M6 2h8v8l-5 5-7-7z"];
    /// A filled triangle: Play.
    Play = "play", Line,
        fills: ["M4 2.5l9 5.5-9 5.5z"];
    /// A triangle in a ring: Simulate.
    Simulate = "simulate", Line,
        lines: ["M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z", "M6.5 5.5l4 2.5-4 2.5z"];
    /// A mouse pointer: the Select tool.
    Select = "select", Line,
        lines: ["M3 2l9 5-4 1.2L6.2 12z"];
    /// A cross of four arrows: the Move tool.
    Move = "move", Line,
        lines: ["M8 1.5v13M1.5 8h13M8 1.5L6 3.5M8 1.5l2 2M8 14.5l-2-2M8 14.5l2-2M1.5 8l2-2M1.5 8l2 2M14.5 8l-2-2M14.5 8l-2 2"];
    /// Most of a ring, ending in an arrow: the Rotate tool.
    Rotate = "rotate", Line,
        lines: ["M13 8a5 5 0 11-1.5-3.6M13 2.5v3h-3"];
    /// A square with an arrow leaving its corner: the Scale tool.
    Scale = "scale", Line,
        lines: [
            "M3 6h6a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-6a1 1 0 0 1 -1 -1v-6a1 1 0 0 1 1 -1z",
            "M9 2h5v5M14 2L8.5 7.5",
        ];
    /// Two joints and the bone between them: Pose bones.
    PoseBones = "pose-bones", Line,
        lines: [
            "M2.5 12a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M10.5 4a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M5 11l6-6",
        ];
    /// A dashed arc: Motion path.
    MotionPath = "motion-path", Line,
        dashed(2.0, 2.0): ["M2 12c3-8 9-8 12 0"];
    /// A diamond: a key on a timeline.
    Key = "key", Line,
        lines: ["M8 3.05L12.95 8 8 12.95 3.05 8z"];
    /// A ruler: Measure.
    Measure = "measure", Line,
        lines: ["M2 11L11 2l3 3-9 9zM5 8l1.5 1.5M7.5 5.5L9 7"];
    /// A triangle pointing back at a bar: the previous key.
    PreviousKey = "previous-key", Line,
        fills: ["M12 3L5 8l7 5z", "M3 3h2v10h-2z"];
    /// A triangle pointing on at a bar: the next key.
    NextKey = "next-key", Line,
        fills: ["M4 3l7 5-7 5z", "M11 3h2v10h-2z"];
    /// A wave: an animation.
    Animation = "animation", Line,
        lines: ["M2 12c2-6 4 2 6-4s4 2 6-4"];
    /// A speaker with a sound wave: a sound.
    Sound = "speaker", Line,
        lines: ["M3 6v4h3l4 3V3L6 6z", "M12 6a3 3 0 010 4"];
    /// A plus: add.
    Plus = "plus", Line,
        lines: ["M8 3v10M3 8h10"];
    /// A cross in a ring: an error.
    Error = "error", Mark,
        lines: ["M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z", "M5.5 5.5l5 5M10.5 5.5l-5 5"];
    /// A triangle with a stroke in it: a warning, quietly.
    WarningOutline = "warning-outline", Mark,
        lines: ["M8 2l6.5 11.5h-13z", "M8 6.5v3"];
    /// A brush's tip: Brush, in placing and in tile painting.
    Brush = "brush", Line,
        lines: ["M10 2l4 4-6 6H4V8z"];
    /// Four tiles: the grid view.
    FourSquares = "four-squares", Line,
        lines: [
            "M3 2h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
            "M10 2h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
            "M3 9h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
            "M10 9h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
        ];
    /// Three blocks: an assembly; Scatter.
    Assembly = "assembly", Line,
        lines: [
            "M3 2h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
            "M10 2h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
            "M6.5 9h3a1 1 0 0 1 1 1v3a1 1 0 0 1 -1 1h-3a1 1 0 0 1 -1 -1v-3a1 1 0 0 1 1 -1z",
        ];
    /// An arrow into a tray: Import.
    Import = "import", Line,
        lines: ["M8 2v8M4.5 6.5L8 10l3.5-3.5M2.5 12.5v1h11v-1"];
    /// A folder: the Library, a folder.
    Library = "folder", Line,
        lines: ["M1.5 4.5a1 1 0 011-1h3.5l1.5 1.5h6a1 1 0 011 1v6.5a1 1 0 01-1 1h-11a1 1 0 01-1-1z"];
    /// A branching line: source control.
    Branch = "branch", Line,
        lines: [
            "M2.5 3.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M2.5 12.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M10.5 6a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M4 5v6M12 7.5c0 3-8 2-8 3.5",
        ];
    /// A clock: Recent projects, History.
    Clock = "clock", Line,
        lines: ["M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z", "M8 4.5V8l2.5 1.5"];
    /// A chevron pointing left: back.
    ChevronLeft = "chevron-left", Mark,
        lines: ["M10 3L5 8l5 5"];
    /// An arrow pointing up: up one folder.
    ArrowUp = "arrow-up", Mark,
        lines: ["M4 7l4-4 4 4M8 3v10"];
    /// A lit ball: a material.
    Material = "material", Line,
        lines: ["M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z", "M5 5a4 4 0 014-1"];
    /// A tick.
    Check = "check", Line,
        lines: ["M3 8.5l3 3 7-7"];
    /// A framed landscape: a picture.
    Picture = "picture", Line,
        lines: [
            "M3.5 2h9a1.5 1.5 0 0 1 1.5 1.5v9a1.5 1.5 0 0 1 -1.5 1.5h-9a1.5 1.5 0 0 1 -1.5 -1.5v-9a1.5 1.5 0 0 1 1.5 -1.5z",
            "M2 11l4-4 3 3 2-2 3 3",
        ];
    /// Plus, times, minus and equals: a Math node.
    Math = "math", Line,
        lines: ["M3 5h4M5 3v4M9.5 4.5l3 3M12.5 4.5l-3 3M3 11.5h4M9 10h4M9 13h4"];
    /// Two sliders: Preferences, parameters.
    Sliders = "sliders", Line,
        lines: [
            "M2 4h7M12 4h2M2 12h2M7 12h7",
            "M9 4a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
            "M4 12a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z",
        ];
    /// A dot in a ring: Record.
    Record = "record", Line,
        lines: ["M3 8a5 5 0 1 0 10 0a5 5 0 1 0 -10 0z"],
        fills: ["M6 8a2 2 0 1 0 4 0a2 2 0 1 0 -4 0z"];
    /// A strip in three rows: a track.
    Track = "track", Line,
        lines: [
            "M3 3h10a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1 -1.5 1.5h-10a1.5 1.5 0 0 1 -1.5 -1.5v-7a1.5 1.5 0 0 1 1.5 -1.5z",
            "M1.5 6.5h13M1.5 9.5h13",
        ];
    /// An easing curve: Curves.
    Curve = "curve", Line,
        lines: ["M2 13C6 13 6 3 14 3"];
    /// Two bars: Pause.
    Pause = "pause", Line,
        lines: [
            "M4.5 3h1.5a0.5 0.5 0 0 1 0.5 0.5v9a0.5 0.5 0 0 1 -0.5 0.5h-1.5a0.5 0.5 0 0 1 -0.5 -0.5v-9a0.5 0.5 0 0 1 0.5 -0.5z",
            "M10 3h1.5a0.5 0.5 0 0 1 0.5 0.5v9a0.5 0.5 0 0 1 -0.5 0.5h-1.5a0.5 0.5 0 0 1 -0.5 -0.5v-9a0.5 0.5 0 0 1 0.5 -0.5z",
        ];
    /// A bug: the Debugger.
    Debugger = "debugger", Line,
        lines: [
            "M8 5h0a3.5 3.5 0 0 1 3.5 3.5v2a3.5 3.5 0 0 1 -3.5 3.5h0a3.5 3.5 0 0 1 -3.5 -3.5v-2a3.5 3.5 0 0 1 3.5 -3.5z",
            "M8 5V3M6 3.5l-1-1.5M10 3.5l1-1.5M4.5 8H2M11.5 8H14M4.5 11.5H2.5M11.5 11.5h2",
        ];
    /// A memory module: Memory.
    Memory = "memory", Line,
        lines: ["M2.5 5h11a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-11a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1z"],
        faint: ["M4 5v6M7 5v6M10 5v6M13 5v6"];
    /// An open folder: Open….
    FolderOpen = "folder-open", Line,
        lines: ["M1.5 4.5a1 1 0 011-1h3.5l1.5 1.5h6a1 1 0 011 1v1", "M1.5 12.5L3.5 7h11l-2 5.5z"];
    /// A cog: Project settings.
    Gear = "gear", Line,
        lines: [
            "M5.5 8a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0z",
            "M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4",
        ];
    /// A jigsaw piece: Plugins.
    Plugin = "plugin", Line,
        lines: ["M6 2h4v3h3v4h-3v5H6V9H3V5h3z"];
    /// An open book: Learn.
    Book = "book", Line,
        lines: ["M2 3.5h4.5A1.5 1.5 0 018 5v8.5a1.5 1.5 0 00-1.5-1.5H2zM14 3.5H9.5A1.5 1.5 0 008 5v8.5a1.5 1.5 0 011.5-1.5H14z"];
    /// An arrow leaving a door: Exit.
    Exit = "exit", Line,
        lines: ["M6 2.5H3v11h3M10 5l3 3-3 3M13 8H6"];
    /// Lines of text, each shorter: a list.
    Lines = "lines", Line,
        lines: ["M2 4h12M2 8h8M2 12h5"];
    /// An eye: shown.
    Eye = "eye", Line,
        lines: [
            "M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z",
            "M6 8a2 2 0 1 0 4 0a2 2 0 1 0 -4 0z",
        ];
    /// A cross: close.
    Close = "close", Mark,
        lines: ["M4 4l8 8M12 4l-8 8"];
    /// A sun with four rays.
    SunSmall = "sun-small", Line,
        lines: ["M5 8a3 3 0 1 0 6 0a3 3 0 1 0 -6 0z", "M8 1v2M8 13v2M1 8h2M13 8h2"];
    /// Bars of a graph: the Profiler.
    Profiler = "bars", Line,
        lines: ["M1.5 13.5h13M3.5 11V7M7 11V3.5M10.5 11V6M14 11V8.5"];
    /// "!" in a triangle: a warning, Problems.
    Warning = "warning", Mark,
        lines: ["M8 2l6.5 11.5h-13z", "M8 6.5v3M8 11.5v.5"];

    // Drawn for what the design lacks, on its grid and in its style
    // (approved on the sheets of docs/design/g-layout/additions/round-1).
    /// An arrow turning back: Home › Edit › Undo.
    Undo = "undo", Line,
        lines: ["M5.5 3.5l-3 3 3 3", "M2.5 6.5H10a3.5 3.5 0 010 7H6.5"];
    /// An arrow turning forward: Home › Edit › Redo.
    Redo = "redo", Line,
        lines: ["M10.5 3.5l3 3-3 3", "M13.5 6.5H6a3.5 3.5 0 000 7h3.5"];
    /// One sheet over another: Console › Copy; copy a value.
    Copy = "copy", Line,
        lines: [
            "M6.5 5.5h6a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-6a1 1 0 0 1 -1 -1v-6a1 1 0 0 1 1 -1z",
            "M3.5 10.5h-1v-8h8v1",
        ];
    /// A bin with its lid: Delete; Remove from list.
    Trash = "trash", Line,
        lines: ["M2.5 4.5h11", "M6 4.5v-2h4v2", "M4 4.5l.5 9h7l.5-9", "M6.75 7v4M9.25 7v4"];
    /// An arrow going round, backwards: Back to the starting value (↺ on a
    /// field).
    Reset = "reset", Line,
        lines: ["M3 8a5 5 0 101.5-3.6", "M3 2.5v3h3"];
    /// Lines of text, crossed off: Console › Clear.
    Clear = "clear", Line,
        lines: ["M2 4h12M2 8h6M2 12h4", "M10 9.5l4 4M14 9.5l-4 4"];
    /// A filled square: Play group › Stop (on the light button).
    Stop = "stop", Line,
        fills: ["M4.5 3.5h7a1 1 0 0 1 1 1v7a1 1 0 0 1 -1 1h-7a1 1 0 0 1 -1 -1v-7a1 1 0 0 1 1 -1z"];
    /// One play triangle, up against a bar: Play group › Step (one tick).
    Step = "step", Line,
        lines: ["M3.5 3.5L10 8l-6.5 4.5z", "M12.5 3.5v9"];
    /// Play, inside an arrow going round again: Debug › Session › Replay a
    /// recording…
    Replay = "replay", Line,
        lines: ["M3 8a5 5 0 101.5-3.6", "M3 2.5v3h3", "M6.9 6.1L10 8l-3.1 1.9z"];
    /// An arrow lifting out of a tray: Debug › Session › Load a save…
    Load = "load", Line,
        lines: ["M8 10V2.5", "M4.5 6L8 2.5 11.5 6", "M2.5 12.5v1h11v-1"];
    /// A game pad: Debug › Session › Play and record inputs.
    Gamepad = "gamepad", Line,
        lines: [
            "M4.5 4.5h7a3 3 0 013 3V10a1.8 1.8 0 01-3.2 1.1L10 9.5H6l-1.3 1.6A1.8 1.8 0 011.5 10V7.5a3 3 0 013-3z",
            "M4.2 7h2.2M5.3 5.9v2.2",
        ],
        fills: ["M9.8 7a0.9 0.9 0 1 0 1.8 0a0.9 0.9 0 1 0 -1.8 0z"];
    /// A speaker, crossed: Game tab › Mute the game.
    Mute = "mute", Line,
        lines: ["M3 6v4h3l4 3V3L6 6z", "M12 6.5l2.5 3M14.5 6.5l-2.5 3"];
    /// Four corners pushed outwards: Maximise a view; All views.
    Fullscreen = "fullscreen", Line,
        lines: ["M2.5 6V2.5H6", "M10 2.5h3.5V6", "M13.5 10v3.5H10", "M6 13.5H2.5V10"];
    /// Play, on a screen: The Game document tab.
    Game = "game", Line,
        lines: [
            "M3.5 3h9a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1 -1.5 1.5h-9a1.5 1.5 0 0 1 -1.5 -1.5v-7a1.5 1.5 0 0 1 1.5 -1.5z",
            "M6.8 5.9L10.2 8l-3.4 2.1z",
        ];
    /// A mouse: Game tab: the game has the mouse and keys.
    Mouse = "mouse", Line,
        lines: ["M8 2.5A3.5 3.5 0 0111.5 6v4a3.5 3.5 0 01-7 0V6A3.5 3.5 0 018 2.5z", "M8 5v2.5"];
    /// Two people: Play with others (when it is built).
    PlayTogether = "play-together", Line,
        lines: [
            "M3.5 5a2 2 0 1 0 4 0a2 2 0 1 0 -4 0z",
            "M1.5 13c0-2.3 1.8-4 4-4s4 1.7 4 4",
            "M10.4 3.2a2 2 0 010 3.6",
            "M11.5 9.3c1.8.4 3 1.8 3 3.7",
        ];
    /// A bare point with its three directions: Home › Create › Empty.
    Empty = "empty", Line,
        lines: ["M8 9V2.5", "M8 9l-5.5 3.5", "M8 9l5.5 3.5"],
        fills: ["M6.7 9a1.3 1.3 0 1 0 2.6 0a1.3 1.3 0 1 0 -2.6 0z"];
    /// A small flat creature (a sprite, taken at its word): Home › Create ›
    /// Sprite; the Sprite card.
    Sprite = "sprite", Line,
        lines: ["M4 13.5V7a4 4 0 018 0v6.5l-2-1.6-2 1.6-2-1.6z"],
        fills: [
            "M5.7 7a0.8 0.8 0 1 0 1.6 0a0.8 0.8 0 1 0 -1.6 0z",
            "M8.7 7a0.8 0.8 0 1 0 1.6 0a0.8 0.8 0 1 0 -1.6 0z",
        ];
    /// A sheet ruled into cells: Home › Create › Tile map; a tile map file.
    TileMap = "tile-map", Line,
        lines: [
            "M3.5 2.5h9a1 1 0 0 1 1 1v9a1 1 0 0 1 -1 1h-9a1 1 0 0 1 -1 -1v-9a1 1 0 0 1 1 -1z",
            "M2.5 6.2h11M2.5 9.8h11",
            "M6.2 2.5v11M9.8 2.5v11",
        ];
    /// A lamp head throwing a cone: Create › More › Spotlight.
    Spotlight = "spotlight", Line,
        lines: ["M6 2.5h4l1.5 3.5h-7z", "M5 8.5l-2 5", "M11 8.5l2 5", "M8 8.5v5"];
    /// The design's dashed volume, with sound in it: Create › More › Reverb
    /// zone.
    ReverbZone = "reverb-zone", Line,
        lines: ["M6.5 6.2a2.6 2.6 0 010 3.6", "M9 4.8a5 5 0 010 6.4"],
        dashed(2.0, 1.5): ["M3 2h10a1 1 0 0 1 1 1v10a1 1 0 0 1 -1 1h-10a1 1 0 0 1 -1 -1v-10a1 1 0 0 1 1 -1z"];
    /// Sheets lying on each other: Tile map › Layers.
    Layers = "layers", Line,
        lines: ["M8 2.5l6 3-6 3-6-3z", "M2 8.25l6 3 6-3", "M2 11l6 3 6-3"];
    /// Hills under a sun: An area file; its document tab.
    AreaFile = "area", Line,
        lines: ["M1.5 13L6 6.5l3 4 2-2.5 3.5 5z", "M10 4.5a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0 -3 0z"];
    /// Angle brackets round a slash: Game code; the Code filter.
    Code = "code", Line,
        lines: ["M5.5 4.5L2 8l3.5 3.5", "M10.5 4.5L14 8l-3.5 3.5", "M9 3L7 13"];
    /// A page with angle brackets: A Lust, Lua or Rust file.
    CodeFile = "code-file", Line,
        lines: [
            "M3.5 2.5H9l3.5 3.5v7.5h-9z",
            "M9 2.5V6h3.5",
            "M7 8.5L5.5 10 7 11.5",
            "M9 8.5l1.5 1.5L9 11.5",
        ];
    /// A page with lines: A text file.
    TextFile = "text-file", Line,
        lines: ["M3.5 2.5H9l3.5 3.5v7.5h-9z", "M9 2.5V6h3.5", "M5.5 8.5h5M5.5 11h3.5"];
    /// A page with a plus: New area; + New in the Library.
    NewFile = "new-file", Line,
        lines: ["M3.5 2.5H9l3.5 3.5v7.5h-9z", "M9 2.5V6h3.5", "M8 8v4M6 10h4"];
    /// A sheet of frames, one still empty: A sprite sheet.
    SpriteSheet = "sprite-sheet", Line,
        lines: [
            "M3.5 2.5h9a1 1 0 0 1 1 1v9a1 1 0 0 1 -1 1h-9a1 1 0 0 1 -1 -1v-9a1 1 0 0 1 1 -1z",
            "M8 2.5v11M2.5 8h11",
        ],
        fills: [
            "M4.25 5.25a1 1 0 1 0 2 0a1 1 0 1 0 -2 0z",
            "M9.75 5.25a1 1 0 1 0 2 0a1 1 0 1 0 -2 0z",
            "M4.25 10.75a1 1 0 1 0 2 0a1 1 0 1 0 -2 0z",
        ];
    /// A strip of film: A clips file; the Frames shelf tab.
    Clips = "clips", Line,
        lines: [
            "M3 3.5h10a1 1 0 0 1 1 1v7a1 1 0 0 1 -1 1h-10a1 1 0 0 1 -1 -1v-7a1 1 0 0 1 1 -1z",
            "M5 3.5v9M11 3.5v9",
            "M2 6.5h3M2 9.5h3M11 6.5h3M11 9.5h3",
        ];
    /// Two states and the way between them: An animator file.
    Animator = "animator", Line,
        lines: [
            "M2.5 2.5h3.5a1 1 0 0 1 1 1v2a1 1 0 0 1 -1 1h-3.5a1 1 0 0 1 -1 -1v-2a1 1 0 0 1 1 -1z",
            "M10 9.5h3.5a1 1 0 0 1 1 1v2a1 1 0 0 1 -1 1h-3.5a1 1 0 0 1 -1 -1v-2a1 1 0 0 1 1 -1z",
            "M7 4.5h3a2 2 0 012 2v3",
            "M10.5 8L12 9.5 13.5 8",
        ];
    /// A tile cut in four, on a pile: A tileset file.
    Tileset = "tileset", Line,
        lines: [
            "M6.5 5.5h6a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-6a1 1 0 0 1 -1 -1v-6a1 1 0 0 1 1 -1z",
            "M9.5 5.5v8M5.5 9.5h8",
            "M3.5 10.5h-1v-8h8v1",
        ];
    /// Two crossed corner rules: A picture's crops (Atlas Crop).
    Crops = "crops", Line,
        lines: ["M4.5 1.5v10h10", "M1.5 4.5h10v10"];
    /// A window with a button in it: A game screen file; the UI tab.
    Screen = "screen", Line,
        lines: [
            "M3.5 3h9a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1 -1.5 1.5h-9a1.5 1.5 0 0 1 -1.5 -1.5v-7a1.5 1.5 0 0 1 1.5 -1.5z",
            "M2 6h12",
            "M5.8 8h4.4a0.8 0.8 0 0 1 0.8 0.8v0.9a0.8 0.8 0 0 1 -0.8 0.8h-4.4a0.8 0.8 0 0 1 -0.8 -0.8v-0.9a0.8 0.8 0 0 1 0.8 -0.8z",
        ];
    /// A sound's wave: A sound event.
    SoundEvent = "sound-event", Line,
        lines: ["M2.5 7v2M5.25 5v6M8 2.5v11M10.75 5v6M13.5 7v2"];
    /// A screen on its stand: The Viewport document tab.
    Viewport = "viewport", Line,
        lines: [
            "M3 3h10a1 1 0 0 1 1 1v6.5a1 1 0 0 1 -1 1h-10a1 1 0 0 1 -1 -1v-6.5a1 1 0 0 1 1 -1z",
            "M8 11.5V14M5.5 14h5",
        ];
    /// A trunk with two branches: The Area column.
    Area = "area-tree", Line,
        lines: ["M2.5 3h5", "M4 3v9.5h2.5", "M4 7.75h2.5", "M8.5 7.75h5", "M8.5 12.5h5"];
    /// Names with their values: The Properties column.
    Properties = "properties", Line,
        lines: ["M2.5 4H5M7 4h6.5", "M2.5 8H5M7 8h6.5", "M2.5 12H5M7 12h6.5"];
    /// A prompt on a screen: The Console shelf tab.
    Console = "console", Line,
        lines: [
            "M3.5 3h9a1.5 1.5 0 0 1 1.5 1.5v7a1.5 1.5 0 0 1 -1.5 1.5h-9a1.5 1.5 0 0 1 -1.5 -1.5v-7a1.5 1.5 0 0 1 1.5 -1.5z",
            "M4.5 6.5L7 8.5l-2.5 2",
            "M8.5 10.5h3",
        ];
    /// An i in a ring: Information; About.
    Info = "info", Line,
        lines: ["M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z", "M8 7.5V11"],
        fills: ["M7.15 5.2a0.85 0.85 0 1 0 1.7 0a0.85 0.85 0 1 0 -1.7 0z"];
    /// A question mark in a ring: A card's ? (opens the manual page).
    Help = "help", Line,
        lines: [
            "M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z",
            "M6.3 6.4a1.8 1.8 0 113 1.4c-.8.5-1.3 1-1.3 1.9",
        ],
        fills: ["M7.15 11.5a0.85 0.85 0 1 0 1.7 0a0.85 0.85 0 1 0 -1.7 0z"];
    /// A speech bubble with a few words: The Hints card.
    Hint = "hint", Line,
        lines: ["M2.5 3h11v8h-5l-3 2.5V11h-3z", "M5.5 6h5M5.5 8.25h3"];
    /// The design's eye, struck through: Hidden in the editor.
    Hidden = "eye-off", Line,
        lines: [
            "M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z",
            "M6 8a2 2 0 1 0 4 0a2 2 0 1 0 -4 0z",
            "M3 13L13 3",
        ];
    /// A closed padlock: Can't be picked in the viewport.
    Lock = "lock", Line,
        lines: [
            "M4.5 7.5h7a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-7a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1z",
            "M5.5 7.5v-2a2.5 2.5 0 015 0v2",
        ];
    /// An open padlock: Unlock; unlock everything.
    Unlock = "unlock", Line,
        lines: [
            "M4.5 7.5h7a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-7a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1z",
            "M5.5 7.5v-2a2.5 2.5 0 014.8-1",
        ];
    /// A drawing pin: Pin a project.
    Pin = "pin", Line,
        lines: ["M5.5 2.5h5", "M6.5 2.5V7l-2 2.5h7L9.5 7V2.5", "M8 9.5V14"];
    /// Three dots: A card's or row's ⋯ menu.
    More = "more", Line,
        fills: [
            "M2.3 8a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0z",
            "M6.8 8a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0z",
            "M11.3 8a1.2 1.2 0 1 0 2.4 0a1.2 1.2 0 1 0 -2.4 0z",
        ];
    /// A funnel: Filter a list by kind.
    Filter = "filter", Line,
        lines: ["M2.5 3.5h11l-4 5v4.5l-3-1.5v-3z"];
    /// Two arrowheads closing: Area column › fold all.
    FoldAll = "fold-all", Line,
        lines: ["M4.5 2.5L8 6l3.5-3.5", "M4.5 13.5L8 10l3.5 3.5"];
    /// Two arrowheads opening: Area column › unfold all.
    UnfoldAll = "unfold-all", Line,
        lines: ["M4.5 6L8 2.5 11.5 6", "M4.5 10L8 13.5 11.5 10"];
    /// The design's chevron, turned down: Expand the ribbon; open a list.
    ChevronDown = "chevron-down", Chevron,
        lines: ["M4 6l4 4 4-4"];
    /// The design's chevron, turned right: A sub-menu; a folded row.
    ChevronRight = "chevron-right", Chevron,
        lines: ["M6 4l4 4-4 4"];
    /// An arrow leaving a box: Opens outside the editor (the manual, a code
    /// editor).
    External = "external", Line,
        lines: ["M7 3.5H3.5v9h9V9", "M9.5 2.5h4v4", "M13.5 2.5L8 8"];
    /// The design's folder with the thing inside it: Show in the Library.
    Reveal = "reveal", Line,
        lines: ["M1.5 4.5a1 1 0 011-1h3.5l1.5 1.5h6a1 1 0 011 1v6.5a1 1 0 01-1 1h-11a1 1 0 01-1-1z"],
        fills: ["M6.6 9.2a1.4 1.4 0 1 0 2.8 0a1.4 1.4 0 1 0 -2.8 0z"];
    /// Two arrows chasing round: Check for updates; Restart to update.
    Refresh = "refresh", Line,
        lines: [
            "M13 6.5A5.2 5.2 0 003.6 5",
            "M3 2.5v3h3",
            "M3 9.5a5.2 5.2 0 009.4 1.5",
            "M13 13.5v-3h-3",
        ];
    /// Move's four arrows round Rotate's ring: Toolbox › Transform (T).
    Transform = "transform", Line,
        lines: [
            "M8 1.5v3M6 3.5l2-2 2 2",
            "M8 14.5v-3M6 12.5l2 2 2-2",
            "M1.5 8h3M3.5 6l-2 2 2 2",
            "M14.5 8h-3M12.5 6l2 2-2 2",
            "M5.5 8a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0 -5 0z",
        ];
    /// A rectangle: Tile painting › Rectangle; canvas › Box.
    Rectangle = "rectangle", Line,
        lines: ["M3.5 4h9a1 1 0 0 1 1 1v6a1 1 0 0 1 -1 1h-9a1 1 0 0 1 -1 -1v-6a1 1 0 0 1 1 -1z"];
    /// An ellipse: Canvas › Ellipse.
    Ellipse = "ellipse", Line,
        lines: ["M8 3.5c3.3 0 6 2 6 4.5s-2.7 4.5-6 4.5-6-2-6-4.5 2.7-4.5 6-4.5z"];
    /// A line between two ends: Tile painting › Line; canvas › Line.
    Line = "line", Line,
        lines: [
            "M1.8 12.8a1.4 1.4 0 1 0 2.8 0a1.4 1.4 0 1 0 -2.8 0z",
            "M11.4 3.2a1.4 1.4 0 1 0 2.8 0a1.4 1.4 0 1 0 -2.8 0z",
            "M4.2 11.8l7.6-7.6",
        ];
    /// A tipped bucket and a drop: Tile painting › Fill.
    Fill = "fill", Line,
        lines: ["M7.5 2.5l5 5-4.3 4.3a1 1 0 01-1.4 0L2.5 7.5z"],
        fills: ["M13.5 9.6c.8 1.2 1.2 1.9 1.2 2.5a1.2 1.2 0 01-2.4 0c0-.6.4-1.3 1.2-2.5z"];
    /// A pipette, bulb at the top: Tile painting › Eyedropper.
    Eyedropper = "eyedropper", Line,
        lines: ["M9.5 5l-6 6v1.5H5l6-6", "M8.5 4L12 7.5"],
        fills: ["M10.4 3.8a1.8 1.8 0 1 0 3.6 0a1.8 1.8 0 1 0 -3.6 0z"];
    /// A pen's nib: Canvas › Pen.
    Pen = "pen", Line,
        lines: ["M8 2l3.5 6-1.5 4H6L4.5 8z", "M8 2v5.5", "M5 14h6"],
        fills: ["M7 8.5a1 1 0 1 0 2 0a1 1 0 1 0 -2 0z"];
    /// A capital T: Canvas › Text.
    Text = "text", Line,
        lines: ["M3.5 5V3h9v2", "M8 3v10", "M6 13h4"];
    /// A rounded button with its word: Canvas › Button.
    Button = "button", Line,
        lines: [
            "M3.5 4.5h9a2 2 0 0 1 2 2v3a2 2 0 0 1 -2 2h-9a2 2 0 0 1 -2 -2v-3a2 2 0 0 1 2 -2z",
            "M5 8h6",
        ];
    /// A horseshoe magnet: Snapping on or off.
    Magnet = "magnet", Line,
        lines: ["M4 2.5v6a4 4 0 008 0v-6H9.5v6a1.5 1.5 0 01-3 0v-6z", "M4 5.5h2.5M9.5 5.5H12"];
    /// Crossing rules: Show the grid; Grid step.
    Grid = "grid", Line,
        lines: ["M5.5 2v12M10.5 2v12", "M2 5.5h12M2 10.5h12"];
    /// A globe: World axes (against Object axes).
    World = "world", Line,
        lines: [
            "M2 8a6 6 0 1 0 12 0a6 6 0 1 0 -12 0z",
            "M2 8h12",
            "M8 2c2 1.7 3 3.7 3 6s-1 4.3-3 6c-2-1.7-3-3.7-3-6s1-4.3 3-6z",
        ];
    /// A shape and its mirror, side by side: Tile copy bar › Flip X.
    FlipHorizontal = "flip-horizontal", Line,
        lines: ["M8 2v12", "M5.5 4.5L2 8l3.5 3.5z", "M10.5 4.5L14 8l-3.5 3.5z"];
    /// A shape and its mirror, one above the other: Tile copy bar › Flip Y.
    FlipVertical = "flip-vertical", Line,
        lines: ["M2 8h12", "M4.5 5.5L8 2l3.5 3.5z", "M4.5 10.5L8 14l3.5-3.5z"];
    /// A tile and a quarter turn: Tile copy bar › Turn.
    Turn = "turn", Line,
        lines: [
            "M3.5 7.5h4a1 1 0 0 1 1 1v4a1 1 0 0 1 -1 1h-4a1 1 0 0 1 -1 -1v-4a1 1 0 0 1 1 -1z",
            "M5.5 4A6.5 6.5 0 0112 10.5",
            "M10 8.5l2 2 2-2",
        ];
}
