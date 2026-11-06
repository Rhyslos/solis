//! What a screen is made of ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)):
//! elements in a tree, each pinned to its parent by an [`Anchor`], and a
//! [`ScaleRule`] saying how big everything gets on a bigger screen.

/// A colour, sRGB from 0 to 1, with how opaque it is (alpha).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colour(pub [f32; 4]);

impl Colour {
    /// Fully see-through.
    pub const NONE: Self = Self([0.0, 0.0, 0.0, 0.0]);
    /// White.
    pub const WHITE: Self = Self([1.0, 1.0, 1.0, 1.0]);

    /// An opaque colour from 0 to 255 values.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        let [r, g, b] = [r, g, b].map(|v| f32::from(v) / 255.0);
        Self([r, g, b, 1.0])
    }

    /// This colour at `alpha` (0 to 1).
    #[must_use]
    pub fn with_alpha(self, alpha: f32) -> Self {
        let [r, g, b, _] = self.0;
        Self([r, g, b, alpha])
    }

    /// Every number finite and from 0 to 1 (anything else made safe).
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self(self.0.map(|v| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        }))
    }
}

/// How a shape is filled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    /// One colour.
    Solid(Colour),
    /// From the first colour at the left to the second at the right.
    Across(Colour, Colour),
    /// From the first colour at the top to the second at the bottom.
    Down(Colour, Colour),
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Colour::NONE)
    }
}

/// Where along one side of its parent an element is pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Edge {
    /// The left (across) or the top (down): the offset is how far in.
    #[default]
    Start,
    /// The middle: the offset moves it from the middle.
    Middle,
    /// The right or the bottom: the offset is how far in from it.
    End,
    /// Both edges: the element stretches with its parent. The offset is
    /// the gap at the start, and the size the gap at the end.
    Stretch,
}

/// Where an element is pinned in its parent: one [`Edge`] across and one
/// down (the anchor picker's 4 × 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Anchor {
    /// Across: left, middle, right or stretch.
    pub across: Edge,
    /// Down: top, middle, bottom or stretch.
    pub down: Edge,
}

impl Anchor {
    /// The top left corner.
    pub const TOP_LEFT: Self = Self::new(Edge::Start, Edge::Start);
    /// The top right corner.
    pub const TOP_RIGHT: Self = Self::new(Edge::End, Edge::Start);
    /// The bottom left corner.
    pub const BOTTOM_LEFT: Self = Self::new(Edge::Start, Edge::End);
    /// The bottom right corner.
    pub const BOTTOM_RIGHT: Self = Self::new(Edge::End, Edge::End);
    /// The middle of the bottom edge.
    pub const BOTTOM: Self = Self::new(Edge::Middle, Edge::End);
    /// The middle.
    pub const MIDDLE: Self = Self::new(Edge::Middle, Edge::Middle);

    /// Pinned `across` and `down`.
    pub const fn new(across: Edge, down: Edge) -> Self {
        Self { across, down }
    }

    /// In plain words, as the anchor picker says it: "bottom left".
    pub fn words(self) -> String {
        let down = match self.down {
            Edge::Start => "top",
            Edge::Middle => "middle",
            Edge::End => "bottom",
            Edge::Stretch => "stretched down",
        };
        let across = match self.across {
            Edge::Start => "left",
            Edge::Middle => "middle",
            Edge::End => "right",
            Edge::Stretch => "stretched across",
        };
        match (self.across, self.down) {
            (Edge::Middle, Edge::Middle) => "middle".into(),
            (Edge::Stretch, Edge::Stretch) => "stretched to fill".into(),
            (_, Edge::Middle) => format!("middle {across}"),
            (Edge::Middle, _) => down.into(),
            _ => format!("{down} {across}"),
        }
    }
}

/// What an element draws.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementKind {
    /// Nothing: a group that holds others.
    Group,
    /// A box, its corners rounded by `round` design pixels.
    Box {
        /// How round its corners are, in design pixels.
        round: f32,
    },
    /// An ellipse filling the element's box.
    Ellipse,
    /// A line from the box's top left to its bottom right (or, rising,
    /// from its bottom left to its top right), `width` design pixels thick.
    Line {
        /// How thick, in design pixels.
        width: f32,
        /// From the bottom left to the top right.
        rising: bool,
    },
    /// Text in the element's box, in its fill's first colour, vertically
    /// in the middle (decision 0084).
    Text {
        /// What it says (`\n` starts a new line).
        text: String,
        /// How tall its letters are, in design pixels.
        size: f32,
        /// Which of the game's fonts (0: the standard one).
        font: usize,
        /// Where along the box: left, middle or right.
        align: Edge,
    },
    /// A bar: a box filled `value` of the way across (0 to 1), its
    /// background behind it (the energy bar).
    Bar {
        /// How round its corners are, in design pixels.
        round: f32,
        /// How full, from 0 to 1.
        value: f32,
        /// The empty part's colour.
        background: Colour,
    },
    /// A picture from the screen's [`crate::pictures::Pictures`] by its
    /// name (`sprites/tools.png#Axe`), fitted inside the box (or covering
    /// it), its fill's first colour multiplying it (white shows it as it
    /// is). One that isn't there is a grey box with its name.
    Picture {
        /// Its name.
        name: String,
        /// How round its corners are, in design pixels.
        round: f32,
        /// Whether it covers the box (cut off at its edges) rather than
        /// fitting inside it.
        cover: bool,
    },
}

/// One element of a screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Its name, unique in the screen (`energy`): code and the editor find
    /// it by this.
    pub name: String,
    /// What it draws.
    pub kind: ElementKind,
    /// Where it's pinned in its parent.
    pub anchor: Anchor,
    /// How far from where it's pinned, in design pixels (across, down):
    /// inwards from an edge, or away from the middle.
    pub offset: [f32; 2],
    /// Its size in design pixels; for a stretched side, the gap at the end.
    pub size: [f32; 2],
    /// How it's filled.
    pub fill: Fill,
    /// Its border: how thick in design pixels, and its colour.
    pub border: Option<(f32, Colour)>,
    /// How opaque, 0 to 1 (its children too).
    pub opacity: f32,
    /// Whether it's drawn (hidden ones and their children aren't).
    pub shown: bool,
    /// What's inside it, drawn after it (on top), in order.
    pub children: Vec<Element>,
    /// Whether what's inside it is cut off at its edges (a canvas).
    pub clips: bool,
    /// A glow round it: how far it reaches in design pixels, and its
    /// colour (decision 0089, M6).
    pub glow: Option<(f32, Colour)>,
    /// An outline just outside its edge: how thick in design pixels, and
    /// its colour.
    pub outline: Option<(f32, Colour)>,
}

impl Element {
    /// An element called `name`, drawing `kind`, pinned top left with no
    /// size and nothing to fill it (a picture is filled white: shown as it
    /// is).
    pub fn new(name: &str, kind: ElementKind) -> Self {
        let fill = match kind {
            ElementKind::Picture { .. } => Fill::Solid(Colour::WHITE),
            _ => Fill::default(),
        };
        Self {
            name: name.to_string(),
            kind,
            anchor: Anchor::TOP_LEFT,
            offset: [0.0, 0.0],
            size: [0.0, 0.0],
            fill,
            border: None,
            opacity: 1.0,
            shown: true,
            children: Vec::new(),
            clips: false,
            glow: None,
            outline: None,
        }
    }

    /// The same, pinned by `anchor`, `offset` from it.
    #[must_use]
    pub fn at(mut self, anchor: Anchor, offset: [f32; 2]) -> Self {
        self.anchor = anchor;
        self.offset = offset;
        self
    }

    /// The same, `size` big.
    #[must_use]
    pub fn sized(mut self, size: [f32; 2]) -> Self {
        self.size = size;
        self
    }

    /// The same, filled with `fill`.
    #[must_use]
    pub fn filled(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }

    /// The same, with a border `width` thick in `colour`.
    #[must_use]
    pub fn bordered(mut self, width: f32, colour: Colour) -> Self {
        self.border = Some((width, colour));
        self
    }

    /// The same, holding `children`.
    #[must_use]
    pub fn holding(mut self, children: Vec<Element>) -> Self {
        self.children = children;
        self
    }

    /// The element called `name`, here or inside.
    pub fn find(&self, name: &str) -> Option<&Element> {
        if self.name == name {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(name))
    }

    /// The element called `name`, here or inside, to change.
    pub fn find_mut(&mut self, name: &str) -> Option<&mut Element> {
        if self.name == name {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(name))
    }
}

/// How big a screen's elements get on a screen bigger or smaller than the
/// one it was designed at (decision 0083: chosen per screen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ScaleRule {
    /// Everything keeps its share of the screen's height: a third bigger
    /// on 1440p than on 1080p. An ultrawide screen of the same height
    /// shows it the same size, with more room at the sides.
    #[default]
    Grow,
    /// The same size in pixels everywhere: a bigger screen has more room
    /// around it.
    Keep,
    /// Pixel art: designed in the game's own pixels (the project's 2D
    /// base resolution), then scaled by the same whole number the game's
    /// picture is (×4 on 1080p), so every pixel stays square and crisp.
    WholeSteps,
}

impl ScaleRule {
    /// Its name as the editor and files say it.
    pub fn words(self) -> &'static str {
        match self {
            Self::Grow => "Grow with the screen",
            Self::Keep => "Keep its size",
            Self::WholeSteps => "Whole steps",
        }
    }
}

/// The most elements a screen holds, all levels together.
pub const MAX_ELEMENTS: usize = 10_000;
/// The deepest elements go inside each other.
pub const MAX_DEPTH: usize = 64;

/// A screen: the HUD, a menu.
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    /// Its name (`HUD`).
    pub name: String,
    /// The size it's designed at, in design pixels (1920 × 1080; for
    /// [`ScaleRule::WholeSteps`], the game's base resolution, 480 × 270).
    pub design: [f32; 2],
    /// How it grows on other screens.
    pub scale: ScaleRule,
    /// Its elements, drawn in order (later on top).
    pub elements: Vec<Element>,
    /// A view written in Lust (decision 0087), laid out with Lust's rules
    /// each time the screen is drawn, after the elements.
    pub view: Option<crate::view::Node>,
    /// Its view's hover and focus looks (decision 0088).
    pub looks: crate::view::Looks,
    /// The pictures it shows (decision 0089), shared by every screen of
    /// the game.
    pub pictures: Option<std::sync::Arc<crate::pictures::Pictures>>,
    /// The player's UI size (decision 0089, M6): 1 as designed, 1.5 half
    /// as big again (0.5 to 2).
    pub ui_size: f32,
    /// Its look, from the Screen component showing it: a glow and an
    /// outline round everything, and its pixel size.
    pub look: ScreenLook,
}

/// A whole screen's look (decision 0089, M6), set on the Screen
/// component that shows it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenLook {
    /// How far a glow reaches round everything, in design pixels, in each
    /// piece's own colour (0 for none).
    pub glow: f32,
    /// How thick an outline is just outside everything, in design pixels
    /// (0 for none).
    pub outline: f32,
    /// The outline's colour.
    pub outline_colour: Colour,
    /// The screen drawn in blocks this many design pixels big (1 or less:
    /// smooth).
    pub pixel: f32,
}

impl Default for ScreenLook {
    fn default() -> Self {
        Self {
            glow: 0.0,
            outline: 0.0,
            outline_colour: Colour::rgb(0, 0, 0),
            pixel: 1.0,
        }
    }
}

impl Screen {
    /// An empty screen called `name`, designed at `design`.
    pub fn new(name: &str, design: [f32; 2], scale: ScaleRule) -> Self {
        Self {
            name: name.to_string(),
            design,
            scale,
            elements: Vec::new(),
            view: None,
            looks: crate::view::Looks::default(),
            pictures: None,
            ui_size: 1.0,
            look: ScreenLook::default(),
        }
    }

    /// The element called `name`, anywhere in the screen.
    pub fn find(&self, name: &str) -> Option<&Element> {
        self.elements.iter().find_map(|e| e.find(name))
    }

    /// The element called `name`, anywhere in the screen, to change.
    pub fn find_mut(&mut self, name: &str) -> Option<&mut Element> {
        self.elements.iter_mut().find_map(|e| e.find_mut(name))
    }
}
