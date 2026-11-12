//! The kinds of file the Library knows, what each can do, and the
//! groups of kinds its chips show. Split from the Library's `mod.rs` so
//! neither is long (review finding DEBT-08).

use super::MenuAction;
use crate::icons::Icon;
use crate::theme;
use egui::Color32;

/// What kind of file a Library item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LibraryKind {
    /// A model.
    #[default]
    Model,
    /// An assembly.
    Assembly,
    /// An area.
    Area,
    /// A picture.
    Texture,
    /// Game code (Lust, Lua or Rust).
    Code,
    /// A sound file (WAV or Ogg Vorbis).
    Sound,
    /// A sound event (`.solsound`): takes and how to vary them.
    SoundEvent,
    /// An Aseprite file: a sprite sheet (Phase 9, D2).
    SpriteSheet,
    /// A sprite's clips file (`.solframes`, Phase 9, D3).
    SpriteClips,
    /// An animator (`.solanim`, Phase 9, D3b).
    Animator,
    /// A tileset (`.tileset.lst`, Phase 9, D4): the terrains a cell can be.
    Tileset,
    /// A tile map (`.tilemap.lst`, Phase 9, D4).
    TileMap,
    /// A picture's crops (`.atlas.lst`, Atlas Crop).
    Atlas,
    /// Plain text: notes (`.md`, `.txt`) and data (`.json`, `.csv`,
    /// `.toml`, `.ron`), shown in Properties.
    Text,
    /// Anything else.
    Other,
}

impl LibraryKind {
    /// What it's called, for tooltips.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Model => "Model",
            Self::Assembly => "Assembly",
            Self::Area => "Area",
            Self::Texture => "Picture",
            Self::Code => "Code",
            Self::Sound => "Sound",
            Self::SoundEvent => "Sound event",
            Self::SpriteSheet => "Sprite sheet",
            Self::SpriteClips => "Sprite clips",
            Self::Animator => "Animator",
            Self::Tileset => "Tileset",
            Self::TileMap => "Tile map",
            Self::Atlas => "Atlas crops",
            Self::Text => "Text",
            Self::Other => "File",
        }
    }

    /// The stripe under its thumbnail, a colour per kind (decision 0004),
    /// the same as its icon in the Area panel (decision 0060).
    pub(crate) fn color(self) -> Color32 {
        use theme::kind as k;
        match self {
            Self::Model => k::MODEL,
            Self::Assembly => k::ASSEMBLY,
            Self::Area => k::AREA,
            Self::Texture | Self::SpriteSheet | Self::Atlas => k::PICTURE,
            Self::SpriteClips | Self::Animator => k::CLIPS,
            Self::Tileset | Self::TileMap => k::TILES,
            Self::Code => k::CODE,
            Self::Sound => k::SOUND,
            Self::SoundEvent => k::SOUND_EVENT,
            Self::Text => k::TEXT,
            Self::Other => k::OTHER,
        }
    }

    /// Every kind, in the order the colour sheet shows them.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 15] = [
        Self::Model,
        Self::Assembly,
        Self::Area,
        Self::Texture,
        Self::SpriteSheet,
        Self::Atlas,
        Self::SpriteClips,
        Self::Animator,
        Self::Tileset,
        Self::TileMap,
        Self::Sound,
        Self::SoundEvent,
        Self::Code,
        Self::Text,
        Self::Other,
    ];

    /// Its icon when it has no thumbnail.
    pub(super) fn icon(self) -> Icon {
        match self {
            Self::Model => Icon::Model,
            Self::Assembly => Icon::Assembly,
            Self::Area => Icon::AreaFile,
            Self::Texture => Icon::Picture,
            Self::Code => Icon::CodeFile,
            Self::Sound => Icon::Sound,
            Self::SoundEvent => Icon::SoundEvent,
            Self::SpriteSheet => Icon::SpriteSheet,
            Self::SpriteClips => Icon::Clips,
            Self::Animator => Icon::Animator,
            Self::Tileset => Icon::Tileset,
            Self::TileMap => Icon::TileMap,
            Self::Atlas => Icon::Crops,
            Self::Text | Self::Other => Icon::TextFile,
        }
    }

    /// Whether it can be placed in an area (dragged in, double-clicked; a
    /// picture, as a sprite, is only dragged in: a double-click opens it).
    pub fn placeable(self) -> bool {
        matches!(
            self,
            Self::Model
                | Self::Assembly
                | Self::SpriteSheet
                | Self::SpriteClips
                | Self::TileMap
                | Self::Texture
        )
    }

    /// Whether a double-click opens it (in the editor) rather than placing
    /// it: a sprite's clips file opens its Clips and Frames tabs, an
    /// animator its Animator tab.
    pub fn opens_on_double_click(self) -> bool {
        matches!(
            self,
            Self::SpriteClips | Self::Animator | Self::Tileset | Self::Atlas
        )
    }

    /// What an item's right-click menu offers to do with a file of this
    /// kind, in the menu's order: Open, Place or both, each once. (One
    /// that opens on a double-click and can't be placed was offered Open
    /// twice: the review, BUG-30.)
    pub(super) fn menu_actions(self) -> &'static [MenuAction] {
        match (
            self.opens_on_double_click() || self == Self::Texture,
            self.placeable(),
        ) {
            (true, true) => &[MenuAction::Open, MenuAction::Place],
            (_, false) => &[MenuAction::Open],
            (false, true) => &[MenuAction::Place],
        }
    }

    /// Whether a file of this kind at `path` may be opened in the program
    /// the computer has for it. A project can come from anyone, so only
    /// kinds whose program just shows or edits them do (a picture viewer,
    /// a text editor for notes, data and Lust). Anything else, such as a
    /// file called "brick.png.exe" or a `.lua` file a program on the
    /// computer might run outside the sandbox, is shown in its folder
    /// instead, never opened.
    ///
    /// This is the only copy of the rule. The editor asks it to say what
    /// will happen; the app asks it again, about the kind the file's own
    /// name gives, at the one place anything is handed to another program,
    /// so no way of asking can skip it.
    pub fn opens_in_its_program(self, path: &str) -> bool {
        match self {
            // Text files only ever show text: a .md opens in the
            // computer's Markdown program (Ledger), never runs.
            Self::Texture | Self::Text => true,
            Self::Code => path.to_lowercase().ends_with(".lst"),
            _ => false,
        }
    }
}

/// The Library's filter chips (decision 0085): every file, or one group
/// of kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum Filter {
    /// Every file.
    #[default]
    All,
    /// Models and assemblies.
    Models,
    /// Sprite sheets, their clips, animators and pictures.
    Sprites,
    /// Areas, tile maps and tilesets.
    Maps,
    /// Sound files and sound events.
    Sounds,
    /// Lust, Lua and Rust files.
    Code,
}

impl Filter {
    /// Every chip, in order.
    pub(crate) const ALL: [Self; 6] = [
        Self::All,
        Self::Models,
        Self::Sprites,
        Self::Maps,
        Self::Sounds,
        Self::Code,
    ];

    /// The chip's words.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Models => "Models",
            Self::Sprites => "Sprites",
            Self::Maps => "Maps",
            Self::Sounds => "Sounds",
            Self::Code => "Code",
        }
    }

    /// Whether a file of `kind` passes.
    pub(crate) fn passes(self, kind: LibraryKind) -> bool {
        use LibraryKind as K;
        match self {
            Self::All => true,
            Self::Models => matches!(kind, K::Model | K::Assembly),
            Self::Sprites => matches!(
                kind,
                K::SpriteSheet | K::SpriteClips | K::Animator | K::Texture
            ),
            Self::Maps => matches!(kind, K::Area | K::TileMap | K::Tileset),
            Self::Sounds => matches!(kind, K::Sound | K::SoundEvent),
            Self::Code => matches!(kind, K::Code | K::Text),
        }
    }

    /// The dot before the chip's word: its first kind's colour (none for
    /// All).
    pub(crate) fn dot(self) -> Option<Color32> {
        use LibraryKind as K;
        let kind = match self {
            Self::All => return None,
            Self::Models => K::Model,
            Self::Sprites => K::SpriteSheet,
            Self::Maps => K::TileMap,
            Self::Sounds => K::Sound,
            Self::Code => K::Code,
        };
        Some(kind.color())
    }
}
