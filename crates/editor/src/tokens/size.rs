//! The design's sizes, in points (one point is one pixel at 100%
//! interface scale): heights, widths, paddings, gaps and corner radii
//! (specification, lines 98 to 113, with the mock-up's exact paddings),
//! the density table and the interface scale's steps.
//!
//! Nothing that draws writes one of these as a number of its own. The
//! frame's parts (tab bar, ribbon, shelf, status bar) and the kit's
//! controls take these as they are built; egui's own controls take them
//! through `theme.rs`.

use serde::{Deserialize, Serialize};

/// The tab bar's height.
pub const TAB_BAR_H: f32 = 34.0;
/// The tab bar's padding at each side.
pub const TAB_BAR_PAD: f32 = 10.0;
/// The gap between the tab bar's parts.
pub const TAB_BAR_GAP: f32 = 4.0;
/// A ribbon tab's height.
pub const RIBBON_TAB_H: f32 = 30.0;
/// A ribbon tab's padding at each side.
pub const RIBBON_TAB_PAD: f32 = 12.0;
/// The thickness of the accent line under the ribbon tab in front.
pub const TAB_UNDERLINE: f32 = 2.0;
/// The logo's light tile: its side.
pub const LOGO_TILE: f32 = 20.0;
/// The room after the logo tile.
pub const LOGO_GAP: f32 = 8.0;
/// The command search: its height.
pub const SEARCH_H: f32 = 24.0;
/// The command search: its width, when there is room.
pub const SEARCH_W: f32 = 380.0;
/// The command search: the most of the window's width it takes.
pub const SEARCH_MAX_SHARE: f32 = 0.36;

/// The ribbon's height.
pub const RIBBON_H: f32 = 66.0;
/// The ribbon's height without its buttons' labels.
pub const RIBBON_H_NO_LABELS: f32 = 46.0;
/// A ribbon group's padding: top, sides, bottom.
pub const RIBBON_GROUP_PAD: [f32; 3] = [4.0, 10.0, 2.0];
/// The gap between a ribbon group's buttons.
pub const RIBBON_GAP: f32 = 2.0;
/// A large ribbon button's size.
pub const RIBBON_BIG: [f32; 2] = [50.0, 44.0];
/// A large ribbon button's icon.
pub const RIBBON_BIG_ICON: f32 = 18.0;
/// The gap between a large ribbon button's icon and its label.
pub const RIBBON_BIG_GAP: f32 = 4.0;
/// A small ribbon row's height.
pub const RIBBON_ROW_H: f32 = 20.0;
/// A small ribbon row's padding at each side.
pub const RIBBON_ROW_PAD: f32 = 8.0;

/// A document tab strip's height.
pub const DOC_TAB_H: f32 = 30.0;
/// A document tab's padding at each side.
pub const DOC_TAB_PAD: f32 = 12.0;
/// The gap inside a document tab.
pub const DOC_TAB_GAP: f32 = 7.0;
/// The toolbox column's width.
pub const TOOLBOX_W: f32 = 44.0;
/// A toolbox button's side.
pub const TOOL_BUTTON: f32 = 34.0;
/// A toolbox button's icon.
pub const TOOL_ICON: f32 = 15.0;

/// The shelf bar's height.
pub const SHELF_BAR_H: f32 = 34.0;
/// The shelf's drag grip: its size.
pub const SHELF_GRIP: [f32; 2] = [36.0, 4.0];
/// The shelf's standard height, bar and all. Solis's own: the design
/// gives a range.
pub const SHELF_H: f32 = 300.0;
/// The shelf's standard height on the Animate tab.
pub const SHELF_H_ANIMATE: f32 = 360.0;
/// A shelf column header's height.
pub const COLUMN_HEADER_H: f32 = 30.0;
/// A row's, header's or shelf tab's padding at each side.
pub const ROW_PAD: f32 = 12.0;
/// How far a tree's children are set in.
pub const TREE_INDENT: f32 = 28.0;
/// A track's or lane's height.
pub const TRACK_H: f32 = 26.0;

/// A field's padding at each side.
pub const FIELD_PAD: f32 = 7.0;
/// The gap between a field's parts, and between small controls.
pub const GAP: f32 = 6.0;
/// The gap between rows of controls.
pub const ROW_GAP: f32 = 4.0;
/// An icon button's side.
pub const ICON_BUTTON: f32 = 26.0;
/// A small icon: in an icon button, before a button's word.
pub const ICON_SMALL: f32 = 14.0;
/// A floating pill's height.
pub const PILL_H: f32 = 24.0;
/// A floating pill's padding at each side.
pub const PILL_PAD: f32 = 8.0;
/// The status bar's height.
pub const STATUS_BAR_H: f32 = 24.0;
/// The gap between the status bar's parts.
pub const STATUS_GAP: f32 = 14.0;
/// The status bar's dot.
pub const STATUS_DOT: f32 = 6.0;

/// A card's padding.
pub const CARD_PAD: f32 = 10.0;
/// The gap inside a card.
pub const CARD_INNER_GAP: f32 = 8.0;
/// The gap between cards.
pub const CARD_GAP: f32 = 10.0;
/// The least width of a card.
pub const CARD_MIN_W: f32 = 180.0;
/// The gap in a grid of tiles.
pub const TILE_GAP: f32 = 4.0;
/// A tile's padding round its thumbnail and words.
pub const TILE_PAD: f32 = 6.0;
/// The accent outline inside a chosen tile's thumbnail.
pub const TILE_OUTLINE: f32 = 2.0;
/// The small square dot in a kind's colour.
pub const KIND_DOT: f32 = 7.0;
/// The least width of a tile in the Library.
pub const TILE_MIN_W: f32 = 108.0;
/// The width of the names in a list of names and values.
pub const KEY_COLUMN_W: f32 = 96.0;
/// The gap between a name and its value.
pub const KEY_VALUE_GAP: f32 = 10.0;
/// The gap between the lines of a list of names and values.
pub const KEY_VALUE_ROW_GAP: f32 = 7.0;
/// A panel's padding. The design gives 10 to 12.
pub const PANEL_PAD: f32 = 12.0;

/// A toggle's size.
pub const TOGGLE: [f32; 2] = [32.0, 18.0];
/// A toggle's knob.
pub const TOGGLE_KNOB: f32 = 14.0;
/// A checkbox's side. The design allows 12 to 14.
pub const CHECKBOX: f32 = 12.0;
/// A segmented control's segment: its height.
pub const SEGMENT_H: f32 = 20.0;
/// A segment's padding at each side.
pub const SEGMENT_PAD: f32 = 9.0;
/// The padding of the well the segments sit in.
pub const SEGMENT_WELL_PAD: f32 = 2.0;
/// A chip's height.
pub const CHIP_H: f32 = 24.0;
/// A colour swatch's side.
pub const SWATCH: f32 = 22.0;
/// The gap between swatches.
pub const SWATCH_GAP: f32 = 8.0;
/// The width of each of the two rings round the chosen swatch.
pub const SWATCH_RING: f32 = 2.0;
/// A slider's track: its thickness.
pub const SLIDER_TRACK: f32 = 4.0;
/// A slider's knob.
pub const SLIDER_KNOB: f32 = 12.0;
/// A keycap's height and least width.
pub const KEYCAP: f32 = 22.0;
/// How many times thicker a keycap's border is along its foot.
pub const KEYCAP_FOOT: f32 = 2.0;
/// A dashed "add" button's height.
pub const DASHED_BUTTON_H: f32 = 22.0;
/// A dashed "add" button's padding at each side.
pub const DASHED_BUTTON_PAD: f32 = 10.0;
/// A plain or main button's height on a File page.
pub const PAGE_BUTTON_H: f32 = 30.0;
/// A plain or main button's padding at each side on a File page.
pub const PAGE_BUTTON_PAD: f32 = 14.0;
/// A plain button's padding at each side. Solis's own.
pub const BUTTON_PAD: f32 = 8.0;

/// The File page's menu column: its width.
pub const FILE_MENU_W: f32 = 220.0;
/// An item of the File page's menu: its height.
pub const FILE_MENU_ITEM_H: f32 = 36.0;
/// A File page's padding: top and bottom, sides.
pub const FILE_PAGE_PAD: [f32; 2] = [28.0, 36.0];
/// The most a File page's settings column is wide.
pub const SETTINGS_MAX_W: f32 = 760.0;
/// The width of a setting row's control.
pub const SETTING_CONTROL_W: f32 = 260.0;
/// A setting row's padding above and below.
pub const SETTING_ROW_PAD: f32 = 12.0;
/// The gap between a setting's words and its control.
pub const SETTING_COLUMN_GAP: f32 = 16.0;
/// The gap between a setting's name and the line under it.
pub const SETTING_HINT_GAP: f32 = 3.0;
/// The least height of a setting row's inside: a field's.
pub const SETTING_ROW_MIN_H: f32 = 24.0;
/// The least room a setting's words keep beside its control.
pub const SETTING_WORDS_MIN_W: f32 = 120.0;
/// The gap above a section's capitals, after the rows before it.
pub const SECTION_GAP: f32 = 18.0;
/// The gap between the blocks of a File page.
pub const PAGE_BLOCK_GAP: f32 = 22.0;
/// The width of a drop-down on a settings page.
pub const SETTING_FIELD_W: f32 = 200.0;
/// A project card's least width.
pub const PROJECT_CARD_MIN_W: f32 = 240.0;
/// A project card's thumbnail: its height.
pub const PROJECT_THUMB_H: f32 = 140.0;

/// A menu's or drop-down list's padding. Solis's own.
pub const MENU_PAD: f32 = 4.0;
/// The height of a row in a menu or a drop-down list. Solis's own.
pub const MENU_ROW_H: f32 = 24.0;
/// A scroll bar's width. Solis's own.
pub const SCROLL_BAR: f32 = 8.0;
/// The width of every border and separator, before the hairline rule.
pub const LINE: f32 = 1.0;

/// Corner radii (specification, line 112).
pub mod radius {
    /// Small rows and segments.
    pub const SMALL: u8 = 4;
    /// Fields, icon buttons, pills, keycaps: the controls.
    pub const CONTROL: u8 = 5;
    /// Tab tops, tiles, large ribbon buttons, the command search.
    pub const TAB: u8 = 6;
    /// Toolbox buttons, File menu items, menus and drop-down lists.
    pub const MENU: u8 = 7;
    /// Cards, and windows that ask something.
    pub const CARD: u8 = 8;
    /// Project cards.
    pub const PROJECT_CARD: u8 = 10;
    /// A checkbox.
    pub const CHECKBOX: u8 = 3;
    /// The shelf's grip and a slider's track.
    pub const GRIP: u8 = 2;
}

/// How tall rows, fields and shelf tabs are (Preferences, Density). The
/// tab bar and the ribbon don't change. The design names the three
/// choices; their numbers are Solis's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Density {
    /// More on screen.
    Compact,
    /// The design's own sizes.
    #[default]
    Default,
    /// Larger things to aim at.
    Comfortable,
}

impl Density {
    /// Every density, in the order the settings list them.
    pub const ALL: [Self; 3] = [Self::Compact, Self::Default, Self::Comfortable];

    /// The density's name, as the settings show it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Default => "Default",
            Self::Comfortable => "Comfortable",
        }
    }

    /// The height of a tree or list row.
    pub fn row_h(self) -> f32 {
        match self {
            Self::Compact => 22.0,
            Self::Default => 24.0,
            Self::Comfortable => 28.0,
        }
    }

    /// The height of a field, and of egui's own buttons and drop-downs.
    pub fn field_h(self) -> f32 {
        self.row_h()
    }

    /// The height of a shelf tab.
    pub fn shelf_tab_h(self) -> f32 {
        self.row_h() + 2.0
    }
}

/// The height of a tree or list row, at the density in use.
pub fn row_h() -> f32 {
    crate::interface_look::in_use().density.row_h()
}

/// The height of a field, at the density in use.
pub fn field_h() -> f32 {
    crate::interface_look::in_use().density.field_h()
}

/// The height of a shelf tab, at the density in use.
pub fn shelf_tab_h() -> f32 {
    crate::interface_look::in_use().density.shelf_tab_h()
}

/// How large the whole interface is drawn (Preferences, Interface scale),
/// as a percentage. Only the steps in [`Scale::STEPS`] exist: any other
/// number becomes the nearest of them, so a damaged or hand-edited
/// setting can't make the editor too small to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "u16", into = "u16")]
pub struct Scale(u16);

impl Scale {
    /// The steps offered, in percent. Solis's own: the design names the
    /// setting without its steps.
    pub const STEPS: [u16; 5] = [100, 125, 150, 175, 200];

    /// Every scale, smallest first.
    pub const ALL: [Self; 5] = [Self(100), Self(125), Self(150), Self(175), Self(200)];

    /// The scale as a percentage.
    pub fn percent(self) -> u16 {
        self.0
    }

    /// The scale as a factor: 1.25 for 125%.
    pub fn factor(self) -> f32 {
        f32::from(self.0) / 100.0
    }

    /// The scale's name, as the settings show it.
    pub fn name(self) -> String {
        format!("{}%", self.0)
    }
}

impl Default for Scale {
    fn default() -> Self {
        Self(100)
    }
}

impl From<u16> for Scale {
    fn from(percent: u16) -> Self {
        let nearest = Self::STEPS
            .into_iter()
            .min_by_key(|step| step.abs_diff(percent))
            .unwrap_or(100);
        Self(nearest)
    }
}

impl From<Scale> for u16 {
    fn from(scale: Scale) -> Self {
        scale.0
    }
}
