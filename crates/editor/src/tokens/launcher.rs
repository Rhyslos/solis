//! The launcher's sizes, in points (decision 0118): its window, its
//! column, the project cards and New project, measured from the approved
//! boards `01-launcher.png` and `02-new-project.png` in
//! `docs/design/ui-rethink/`. Everything else it draws (buttons, fields,
//! menus, radii) is the kit's, with the sizes in [`super::size`].

/// The launcher window's size the first time it opens (board 01).
pub const WINDOW: [f32; 2] = [1100.0, 700.0];
/// The smallest the launcher window may get while staying usable.
pub const MIN_WINDOW: [f32; 2] = [760.0, 520.0];

/// The column down the left: its width.
pub const SIDE_W: f32 = 210.0;
/// Its padding: top and bottom, then left and right.
pub const SIDE_PAD: [f32; 2] = [16.0, 10.0];
/// The logo's tile at its top.
pub const LOGO: f32 = 30.0;
/// From the logo's tile to the name beside it.
pub const LOGO_GAP: f32 = 10.0;
/// From the name and logo to the first page.
pub const BRAND_GAP: f32 = 22.0;
/// A page's row in the column.
pub const SIDE_ITEM_H: f32 = 36.0;

/// The page's padding: top and bottom, then left and right.
pub const PAGE_PAD: [f32; 2] = [24.0, 28.0];
/// From the heading to the search and filter.
pub const TOOLS_GAP: f32 = 18.0;
/// The search's width.
pub const SEARCH_W: f32 = 240.0;
/// The Sort by list's width.
pub const SORT_W: f32 = 132.0;

/// A project card's least width: as many fit across as can.
pub const CARD_MIN_W: f32 = 240.0;
/// The gap between cards.
pub const CARD_GAP: f32 = 16.0;
/// The picture at a card's top.
pub const CARD_PICTURE_H: f32 = 128.0;
/// The padding under the picture: top and bottom, then left and right.
pub const CARD_PAD: [f32; 2] = [12.0, 14.0];
/// Between the lines under the picture.
pub const CARD_LINE_GAP: f32 = 4.0;
/// The kind's mark in the picture's corner: its inset from the corner.
pub const BADGE_INSET: f32 = 10.0;
/// The kind's mark: its height.
pub const BADGE_H: f32 = 20.0;
/// The kind's mark: the room either side of its word.
pub const BADGE_PAD: f32 = 7.0;
/// The ⋯ button at the right of a card's name.
pub const CARD_MENU: f32 = 20.0;

/// New project's width (board 02).
pub const DIALOG_W: f32 = 640.0;
/// New project's padding.
pub const DIALOG_PAD: f32 = 20.0;
/// Between New project's sections.
pub const DIALOG_SECTION_GAP: f32 = 16.0;
/// A kind's card in New project: its picture's height.
pub const KIND_PICTURE_H: f32 = 62.0;
/// A kind's card: its padding.
pub const KIND_PAD: f32 = 8.0;
/// A kind's card: its height.
pub const KIND_H: f32 = 138.0;
/// A starter's tile in New project: its width.
pub const STARTER_W: f32 = 104.0;
/// A starter's tile: its height.
pub const STARTER_H: f32 = 64.0;
/// Between starters' tiles.
pub const STARTER_GAP: f32 = 10.0;
/// The tick in a chosen kind's corner.
pub const TICK: f32 = 16.0;
