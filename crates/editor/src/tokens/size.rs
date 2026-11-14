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
/// The command search: the least width it is drawn at. Narrower, it
/// goes (its key still opens the search). Solis's own.
pub const SEARCH_MIN_W: f32 = 120.0;
/// The command search: its icon.
pub const SEARCH_ICON: f32 = 12.0;
/// The room between the logo tile's edge and the logo. Solis's own.
pub const LOGO_INSET: f32 = 2.0;

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
/// The room either side of a large button's word when the word is
/// wider than the button. Solis's own.
pub const RIBBON_BIG_WORD_PAD: f32 = 6.0;
/// A large ribbon button's icon.
pub const RIBBON_BIG_ICON: f32 = 18.0;
/// The gap between a large ribbon button's icon and its label.
pub const RIBBON_BIG_GAP: f32 = 4.0;
/// The gap before a large button's key, small beside its word (Shape B,
/// the approved mock-up `docs/design/tools-1/01-brush-list.png`).
pub const RIBBON_KEY_GAP: f32 = 3.0;
/// A small ribbon row's height.
pub const RIBBON_ROW_H: f32 = 20.0;
/// A small ribbon row's padding at each side.
pub const RIBBON_ROW_PAD: f32 = 8.0;
/// A small ribbon row's height where three are stacked: three of 20
/// and a group's label don't fit the ribbon, so such a group has no
/// label and rows of 19 (the approved additions, round 1).
pub const RIBBON_ROW_H_TIGHT: f32 = 19.0;
/// The gap between stacked ribbon rows.
pub const RIBBON_ROW_GAP: f32 = 1.0;
/// The height of the line a ribbon group's label is on.
pub const RIBBON_GROUP_LABEL_H: f32 = 12.0;
/// The icon on the ribbon's one light button (Play, Stop).
pub const RIBBON_LIGHT_ICON: f32 = 16.0;
/// The small arrow that opens a ribbon control's list: its width.
pub const LIST_ARROW: f32 = 5.0;
/// The room round that arrow that takes its click. Solis's own.
pub const LIST_ARROW_REACH: f32 = 14.0;

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
/// Space above the toolbox's first tool and below its last (from the HTML).
pub const TOOLBOX_PAD: f32 = 6.0;
/// Space between the toolbox's tools (from the HTML).
pub const TOOL_GAP: f32 = 2.0;
/// How long the line between kinds of tool is (from the HTML).
pub const TOOL_DIVIDER: f32 = 22.0;
/// Space above and below that line (from the HTML).
pub const TOOL_DIVIDER_GAP: f32 = 4.0;

/// The shelf bar's height.
pub const SHELF_BAR_H: f32 = 34.0;
/// The shelf's drag grip: its size.
pub const SHELF_GRIP: [f32; 2] = [36.0, 4.0];
/// The shelf's standard height under its bar. Solis's own: the design
/// gives a range (290 to 360), and this one leaves the center 442 tall
/// in a 1440 × 900 window, inside the design's 400 to 470.
pub const SHELF_H: f32 = 300.0;
/// The shelf's standard height on the Animate tab (spec 418).
pub const SHELF_H_ANIMATE: f32 = 360.0;
/// The shelf's standard height on Home in a layout with the Area,
/// Properties and Assets lists beside the groups (decision 0109): it
/// holds only the Console, Problems and the like there. Solis's own.
pub const SHELF_H_SIDE: f32 = 160.0;
/// The shelf's least height under its bar, when the grip is dragged.
/// Solis's own.
pub const SHELF_MIN_H: f32 = 80.0;
/// The least height the center keeps when the grip is dragged up.
/// Solis's own.
pub const CENTER_MIN_H: f32 = 120.0;
/// The Scene (Area) column's width (spec 205: 220 to 240).
pub const SCENE_COLUMN_W: f32 = 240.0;
/// A tileset's Terrains column on its shelf tab (round 3,
/// `02-tileset-editor.png`: the list and the chosen terrain).
pub const TERRAINS_COLUMN_W: f32 = 630.0;
/// The Assets (Library) column's width (spec 207: 280 to 320).
pub const ASSETS_COLUMN_W: f32 = 300.0;
/// Properties as a column of its own beside the groups (decision 0109):
/// its cards one under another, each with room for three numbers in a
/// row; dragged wider, two go side by side. Solis's own.
pub const PROPERTIES_COLUMN_W: f32 = 340.0;
/// The Assets strip under the groups (decision 0109), header and all:
/// two rows of tiles. Solis's own.
pub const ASSETS_STRIP_H: f32 = 190.0;
/// The least height of the Assets strip, or of one of two lists stacked
/// in a column, when a border is dragged. Solis's own.
pub const PART_MIN_H: f32 = 80.0;
/// The least width the editor groups keep between the side columns:
/// narrower, the columns give way. Solis's own.
pub const CENTER_MIN_W: f32 = 240.0;
/// The upper list's standard share of a column holding two (decision
/// 0109). Solis's own.
pub const SIDE_SHARE: f32 = 0.5;
/// The least width of a tile in the Assets list: as many go across as
/// fit, four in its standard column. Solis's own.
pub const ASSETS_TILE_MIN_W: f32 = 64.0;
/// The Placement column on the Place shelf tab (spec 270: 250 to 280).
pub const PLACEMENT_COLUMN_W: f32 = 260.0;
/// How far right of the pointer the placing label starts. Solis's own.
pub const PLACE_LABEL_OFFSET: f32 = 18.0;
/// The brush list B opens (Tools 1a, B2; the approved mock-up
/// `docs/design/tools-1/01-brush-list.png`): its width.
pub const BRUSH_LIST_W: f32 = 236.0;
/// The brush list: a row's height, and its foot's.
pub const BRUSH_LIST_ROW_H: f32 = 26.0;
/// The brush list: a row's padding at each side.
pub const BRUSH_LIST_PAD: f32 = 9.0;
/// The brush list: a shape's icon, and the gap after it.
pub const BRUSH_LIST_ICON: [f32; 2] = [15.0, 9.0];
/// Drawing a brush out (the approved mock-up `02-brush-draw.png`): the
/// squares at the footprint's corners.
pub const BRUSH_CORNER: f32 = 6.0;
/// Drawing a brush out: how far a size's label stands off its edge.
pub const BRUSH_LABEL_OFFSET: f32 = 18.0;
/// Drawing a brush out: the height arrow's head, and the dot at its foot.
pub const BRUSH_ARROW: [f32; 2] = [7.0, 3.0];
/// Drawing a brush out: how far below the view's pills its steps are
/// (and, while a brush's insides are edited, the text saying so).
pub const BRUSH_STEPS_DROP: f32 = 22.0;
/// The Brush card's boxes counting points, edges and faces: their height
/// (the approved mock-up `03-brush-edit-faces.png`).
pub const BRUSH_COUNT_H: f32 = 44.0;
/// The placing hint strip's height (spec 254).
pub const PLACE_HINT_H: f32 = 30.0;
/// The placing hint strip's widest. Solis's own: room for its five keys.
pub const PLACE_HINT_W: f32 = 600.0;
/// The Place shelf tab's search field (spec 272). Solis's own width.
pub const PLACE_SEARCH_W: f32 = 180.0;
/// The least width of a tile on the Place shelf tab (spec 272).
pub const PLACE_TILE_MIN_W: f32 = 92.0;
/// The least width of a shelf column: narrower than this, columns wrap
/// under each other (spec 1143). Solis's own, from the cards' 180.
pub const COLUMN_MIN_W: f32 = 180.0;
/// The standard split between two editor groups: the first one's share.
pub const SPLIT: f32 = 0.5;
/// The Library's split when a tileset or a picture's crops open beside
/// it.
pub const SPLIT_LIBRARY: f32 = 0.4;
/// The mixer's share of the Audio tab's center, beside the Bus group
/// (spec 760: about 60%).
pub const SPLIT_AUDIO: f32 = 0.6;
/// The least width of an editor group side by side: narrower than this,
/// the groups stack (spec 1145). Solis's own.
pub const GROUP_MIN_W: f32 = 360.0;
/// How far either side of a border (the 1 px line between groups or
/// columns, the shelf's top edge) a drag takes hold. Solis's own.
pub const DIVIDER_REACH: f32 = 3.0;
/// A shelf column header's height.
pub const COLUMN_HEADER_H: f32 = 30.0;
/// A row's, header's or shelf tab's padding at each side.
pub const ROW_PAD: f32 = 12.0;
/// The column of a Console line's level word ("ERROR").
pub const LOG_LEVEL_W: f32 = 40.0;
/// How far a tree's children are set in.
pub const TREE_INDENT: f32 = 28.0;
/// How much further in each level of the Area column's tree is (from the
/// approved additions, round 2).
pub const TREE_LEVEL: f32 = 18.0;
/// The room before a tree row for its fold arrow.
pub const TREE_FOLD_W: f32 = 14.0;
/// The fold arrow itself.
pub const FOLD_ICON: f32 = 9.0;
/// An icon on a list row (an entity's kind, its hide and lock switches).
pub const ROW_ICON: f32 = 12.0;
/// The room for each of a row's hide and lock switches.
pub const ROW_SWITCH: f32 = 16.0;
/// A row of the Problems shelf tab (from the HTML: 26 tall, 14 in from
/// the sides, a 13 px icon).
pub const PROBLEM_ROW_H: f32 = 26.0;
/// How far a Problems row's icon is in from the column's edge.
pub const PROBLEM_ROW_PAD: f32 = 14.0;
/// A Problems row's icon.
pub const PROBLEM_ICON: f32 = 13.0;
/// A track's or lane's height.
pub const TRACK_H: f32 = 26.0;

/// A field's padding at each side.
pub const FIELD_PAD: f32 = 7.0;
/// The column of names before a card's fields (Solis's own: the design's
/// names are a word or two).
pub const FIELD_LABEL_W: f32 = 84.0;
/// Room before a field's name for the changed-from-its-start dot.
pub const FIELD_DOT_W: f32 = 8.0;
/// That dot.
pub const FIELD_DOT: f32 = 5.0;
/// The narrowest a card's field gets.
pub const FIELD_MIN_W: f32 = 60.0;
/// Room after a field for its way back ("↺ 1000").
pub const FIELD_AFTER_W: f32 = 64.0;
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
/// Space between pills side by side (from the HTML).
pub const PILL_GAP: f32 = 6.0;
/// How far pills sit from the view's edges (from the HTML).
pub const PILL_MARGIN: f32 = 8.0;
/// Space between a pill's icon or word and what follows it.
pub const PILL_ICON_GAP: f32 = 6.0;
/// The narrowest a pill's list is.
pub const PILL_MENU_W: f32 = 180.0;
/// An axis lock's button inside its pill.
pub const LOCK_BUTTON: f32 = 18.0;
/// Space between axis locks.
pub const LOCK_GAP: f32 = 2.0;
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
/// The line under an Assets tile for its name (spec 216).
pub const ASSET_NAME_H: f32 = 14.0;
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
/// The File page's menu column: its padding, top and bottom, sides.
pub const FILE_MENU_PAD: [f32; 2] = [16.0, 10.0];
/// The gap between the items of the File page's menu.
pub const FILE_MENU_GAP: f32 = 2.0;
/// The room round a line between blocks of the File page's menu: above
/// and below, sides.
pub const FILE_MENU_LINE_PAD: [f32; 2] = [8.0, 6.0];
/// The room under Back to editor, at the top of the File page's menu.
pub const FILE_MENU_BACK_GAP: f32 = 8.0;
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
/// The gap between project cards, and between the cards on Learn.
pub const PROJECT_CARD_GAP: f32 = 16.0;
/// A project card's words: their padding, top and bottom, sides.
pub const PROJECT_CARD_PAD: [f32; 2] = [12.0, 14.0];
/// The gap between the lines of a project card's words.
pub const PROJECT_CARD_LINE_GAP: f32 = 5.0;
/// The letter that stands for a project's picture (the approved
/// additions, round 2), until projects have pictures.
pub const PROJECT_LETTER: f32 = 48.0;
/// The search in a File page's header: its width (round 2's pictures;
/// the design's own is 260, before Sort by joined it).
pub const PAGE_SEARCH_W: f32 = 220.0;
/// A template's tile: its width (round 2: eight fit one row at 1440 px).
pub const TEMPLATE_W: f32 = 126.0;
/// A template's tile: the box over its name.
pub const TEMPLATE_BOX_H: f32 = 84.0;
/// The gap between template tiles.
pub const TEMPLATE_GAP: f32 = 14.0;
/// The gap between a template's box and its name.
pub const TEMPLATE_NAME_GAP: f32 = 8.0;
/// A File page's column beside its settings (Updates' notes, a page's
/// "changed from defaults" card): its width.
pub const PAGE_SIDE_W: f32 = 320.0;
/// The gap between a File page's settings and the column beside them.
pub const PAGE_SIDE_GAP: f32 = 32.0;

/// A mixer strip's width (spec 765).
pub const STRIP_W: f32 = 104.0;
/// The gap between mixer strips, and the console's padding (spec 764:
/// 8 apart, 10 in).
pub const STRIP_GAP: f32 = 8.0;
/// The mixer console's padding round its strips.
pub const STRIP_PAD: f32 = 10.0;
/// A strip's header: its name over its routing (spec 766).
pub const STRIP_HEADER_H: f32 = 40.0;
/// A strip's slot row (an insert, the pan): its height (spec 767).
pub const STRIP_SLOT_H: f32 = 20.0;
/// A strip's slot row: its margin at each side.
pub const STRIP_SLOT_MARGIN: f32 = 6.0;
/// The least height of a strip's fader and meters (spec 770).
pub const FADER_MIN_H: f32 = 120.0;
/// A fader's cap (spec 770: 26 × 16).
pub const FADER_CAP: [f32; 2] = [26.0, 16.0];
/// A fader's track: its width.
pub const FADER_TRACK: f32 = 3.0;
/// Where a fader's track sits, from the strip's left edge.
pub const FADER_X: f32 = 38.0;
/// Where the dB scale's numbers end, from the strip's left edge.
pub const FADER_SCALE_X: f32 = 76.0;
/// A level meter's bar: its width (spec 771).
pub const METER_W: f32 = 5.0;
/// The gap between a strip's two meter bars.
pub const METER_GAP: f32 = 2.0;
/// A meter's peak line: its height.
pub const METER_PEAK: f32 = 2.0;
/// The share of a meter's range above which it tops out in amber
/// (spec 771).
pub const METER_HOT: f32 = 0.86;
/// A strip's M and S buttons: their height (spec 773).
pub const STRIP_BUTTON_H: f32 = 22.0;
/// A sound events table's row (spec 800).
pub const TABLE_ROW_H: f32 = 26.0;
/// A sound events table's header row.
pub const TABLE_HEADER_H: f32 = 28.0;
/// The Sound events tab's detail column: its share of the shelf.
pub const EVENT_DETAIL_SHARE: f32 = 0.5;

/// The Profiler's line over its graph: the frame shown.
pub const PROFILER_TOP_H: f32 = 24.0;
/// The frame-time graph's height (spec 467).
pub const FRAME_GRAPH_H: f32 = 100.0;
/// The room at the graph's left for "60 fps" and "30 fps".
pub const FRAME_GRAPH_LABEL_W: f32 = 52.0;
/// The timeline's column of lane names (spec 477).
pub const LANE_LABEL_W: f32 = 110.0;
/// A lane of the timeline, and its ruler (spec 479).
pub const LANE_H: f32 = 26.0;
/// A block on a lane: its inset from the lane's top and foot.
pub const LANE_BLOCK_INSET: f32 = 3.0;
/// The Statistics column beside the timeline (spec 491).
pub const STATS_W: f32 = 320.0;
/// A statistic's bar under its row (CPU, GPU).
pub const STATS_BAR_H: f32 = 3.0;

/// The shared dialog card's padding (mock-up round 1, `10-dialog-*`).
pub const DIALOG_PAD: f32 = 20.0;

/// The mark on a part that isn't built yet: a hollow square, this wide.
pub const UNBUILT_MARK: f32 = 5.0;
/// The gap between an unbuilt part's words and its mark.
pub const UNBUILT_MARK_GAP: f32 = 6.0;
/// How far below the middle of its words' line the mark's own middle
/// is: it sits just above the line the letters stand on.
pub const UNBUILT_MARK_DROP: f32 = 1.5;
/// How far the mark is from the top and the right of an unbuilt button.
pub const UNBUILT_CORNER: f32 = 4.0;
/// An unbuilt page's card: its width.
pub const UNBUILT_CARD_W: f32 = 400.0;
/// An unbuilt page's card: its padding at the top, the sides, the foot.
pub const UNBUILT_CARD_PAD: [f32; 3] = [28.0, 28.0, 24.0];
/// An unbuilt page's card: its icon.
pub const UNBUILT_PAGE_ICON: f32 = 28.0;
/// A File page's sub-tab: its height.
pub const SUB_TAB_H: f32 = 30.0;
/// A File page's sub-tab: its padding at each side.
pub const SUB_TAB_PAD: f32 = 2.0;
/// The gap between a File page's sub-tabs.
pub const SUB_TAB_GAP: f32 = 18.0;
/// An item of the File page's menu: its padding at each side.
pub const FILE_MENU_ITEM_PAD: f32 = 14.0;
/// An item of the File page's menu: the gap between its icon and name.
pub const FILE_MENU_ITEM_GAP: f32 = 10.0;

/// A menu's or drop-down list's padding. Solis's own.
pub const MENU_PAD: f32 = 4.0;
/// The height of a row in a menu or a drop-down list. Solis's own.
pub const MENU_ROW_H: f32 = 24.0;
/// The small capitals over a group of a menu's rows (from the approved
/// additions, round 2).
pub const MENU_CAPS_H: f32 = 20.0;
/// Room before a menu row's words for its check mark.
pub const MENU_CHECK_W: f32 = 21.0;
/// A menu row's check mark.
pub const MENU_CHECK_ICON: f32 = 11.0;
/// Room at a menu row's right for quiet words after it (a key, a count).
pub const MENU_AFTER_W: f32 = 24.0;
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
