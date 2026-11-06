//! A screen written in Lust ([decision 0087](../../../docs/decisions/0087-screens-through-the-engine.md)):
//! the tree Lust's `view` gives (the same tree Lust's own windows draw),
//! laid out with Lust's rules into elements in design pixels.
//!
//! The rules are Lust's (its `native_ui.rs`: `measure`, `place`, `apply`,
//! `base_look`), for the pieces a game screen uses: `column`, `row`,
//! `stack`, `space`, `text`, `progress`, and `canvas` with `box`,
//! `circle` and `line`. Other pieces are laid out as columns and draw
//! nothing (yet). Nothing here trusts the tree: sizes, depth and counts
//! are kept within limits.

use crate::screen::{Colour, Edge, Element, ElementKind, Fill, MAX_DEPTH, MAX_ELEMENTS};

/// The biggest size or place taken from a screen, in design pixels.
const MAX_DESIGN: f32 = 1.0e5;
/// The longest text kept from one piece, in characters.
const MAX_TEXT: usize = 4096;

/// One piece of a view, as Lust's `__view()` gives it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    /// Which piece: `column`, `text`, `box`…
    pub kind: String,
    /// What a text piece says.
    pub text: String,
    /// A progress bar's value.
    pub value: Option<f64>,
    /// Its styles in order (`[pad 8, bold]`), as Lust gives them.
    pub styles: Vec<(String, Style)>,
    /// What's inside it, in order.
    pub children: Vec<Node>,
    /// For a piece with a `{ }` (a button, or a box or circle in a
    /// canvas): its number among the view's clickable pieces, in order.
    pub action: Option<usize>,
}

/// What's happening to a view's clickable pieces, by their numbers: the
/// one under the pointer, and the one with the keys' focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Looks {
    /// Under the pointer.
    pub hover: Option<usize>,
    /// Has the focus (decision 0088).
    pub focus: Option<usize>,
}

/// A clickable piece where it was laid out: its box in design pixels
/// (left, top, right, bottom) and its number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clickable {
    /// Left, top, right, bottom, in design pixels.
    pub rect: [f32; 4],
    /// Its number ([`Node::action`]).
    pub action: usize,
}

/// One style's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Style {
    /// A word on its own (`bold`), or one that's on only when something
    /// is true (`bold done`: false leaves it off).
    Flag(bool),
    /// A number (`pad 8`).
    Number(f64),
    /// A word or text (`width fill`, `background "red5"`).
    Text(String),
    /// Styles inside (`hover:[background red5]`).
    Nested(Vec<(String, Style)>),
}

/// What laying a view out needs to know about text.
pub trait Measure {
    /// How wide and tall `text` is in `font` at `size` design pixels.
    fn measure(&mut self, font: usize, text: &str, size: f32) -> [f32; 2];
}

impl Measure for crate::text::Fonts {
    fn measure(&mut self, font: usize, text: &str, size: f32) -> [f32; 2] {
        crate::text::Fonts::measure(self, font, text, size)
    }
}

/// Text measured as nothing, when there are no fonts.
pub struct NoFonts;

impl Measure for NoFonts {
    fn measure(&mut self, _: usize, _: &str, _: f32) -> [f32; 2] {
        [0.0, 0.0]
    }
}

/// How a piece's width or height is chosen.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Size {
    /// As big as what it holds.
    Auto,
    /// As big as its parent lets it be.
    Fill,
    /// This many design pixels.
    Fixed(f32),
}

/// Where a piece puts what it holds, along one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Start,
    Center,
    End,
}

/// A piece's look and layout, from its kind and styles (Lust's `Look`).
#[derive(Debug, Clone)]
struct Look {
    /// Top, right, bottom, left.
    pad: [f32; 4],
    gap: f32,
    width: Size,
    height: Size,
    grow: Option<f32>,
    min_w: f32,
    max_w: f32,
    min_h: f32,
    max_h: f32,
    size: f32,
    bold: bool,
    color: u32,
    background: Option<u32>,
    radius: f32,
    border: Option<u32>,
    border_w: f32,
    opacity: f32,
    halign: Align,
    valign: Align,
    spread: bool,
    wrap: bool,
    from: f32,
    to: f32,
    accent: u32,
    at_x: f32,
    at_y: f32,
    to_x: f32,
    to_y: f32,
    rad: f32,
    thick: f32,
    disabled: bool,
}

/// Lust's text colour on a light window, and its muted one.
const INK: u32 = 0x1f2328;
const MUTED: u32 = 0x6b7280;

/// A piece's look before its styles (Lust's `base_look`).
fn base_look(kind: &str) -> Look {
    let mut l = Look {
        pad: [0.0; 4],
        gap: 0.0,
        width: Size::Auto,
        height: Size::Auto,
        grow: None,
        min_w: 0.0,
        max_w: f32::INFINITY,
        min_h: 0.0,
        max_h: f32::INFINITY,
        size: 15.0,
        bold: false,
        color: INK,
        background: None,
        radius: 0.0,
        border: None,
        border_w: 1.0,
        opacity: 1.0,
        halign: Align::Start,
        valign: Align::Start,
        spread: false,
        wrap: true,
        from: 0.0,
        to: 100.0,
        accent: 0x1c7ed6,
        at_x: 0.0,
        at_y: 0.0,
        to_x: 0.0,
        to_y: 0.0,
        rad: 10.0,
        thick: 2.0,
        disabled: false,
    };
    match kind {
        "row" => l.valign = Align::Center,
        "button" => {
            l.pad = [7.0, 14.0, 7.0, 14.0];
            l.background = Some(0xe9ecef);
            l.radius = 6.0;
            l.halign = Align::Center;
        }
        "canvas" => {
            l.width = Size::Fixed(300.0);
            l.height = Size::Fixed(150.0);
        }
        "box" => {
            l.width = Size::Fixed(20.0);
            l.height = Size::Fixed(20.0);
            l.background = Some(0x868e96);
        }
        "circle" => {
            l.radius = f32::INFINITY;
            l.background = Some(0x868e96);
        }
        "line" => l.color = 0x495057,
        "progress" => {
            l.width = Size::Fixed(200.0);
            l.height = Size::Fixed(8.0);
            l.radius = f32::INFINITY;
            l.background = Some(0xe9ecef);
        }
        _ => {}
    }
    l
}

/// A number style's value, kept within design sizes.
fn num(v: &Style) -> Option<f32> {
    match v {
        Style::Number(n) if n.is_finite() => Some((*n as f32).clamp(-MAX_DESIGN, MAX_DESIGN)),
        _ => None,
    }
}

/// A word style's value.
fn word(v: &Style) -> Option<&str> {
    match v {
        Style::Text(t) => Some(t),
        _ => None,
    }
}

/// Applies `styles` to `l`, as Lust's `apply` does.
fn apply(l: &mut Look, kind: &str, styles: &[(String, Style)]) {
    for (k, v) in styles {
        if let Style::Flag(false) = v {
            continue;
        }
        let n = num(v);
        match k.as_str() {
            "pad" => l.pad = [n.unwrap_or(8.0); 4],
            "padX" => {
                let p = n.unwrap_or(8.0);
                l.pad[1] = p;
                l.pad[3] = p;
            }
            "padY" => {
                let p = n.unwrap_or(8.0);
                l.pad[0] = p;
                l.pad[2] = p;
            }
            "padTop" => l.pad[0] = n.unwrap_or(8.0),
            "padRight" => l.pad[1] = n.unwrap_or(8.0),
            "padBottom" => l.pad[2] = n.unwrap_or(8.0),
            "padLeft" => l.pad[3] = n.unwrap_or(8.0),
            "gap" => l.gap = n.unwrap_or(8.0),
            "width" | "height" => {
                let s = match v {
                    Style::Number(_) => Size::Fixed(n.unwrap_or(0.0).max(0.0)),
                    Style::Text(t) if t == "fill" => Size::Fill,
                    Style::Flag(_) => Size::Fill,
                    _ => Size::Auto,
                };
                if k == "width" {
                    l.width = s;
                } else {
                    l.height = s;
                }
            }
            "grow" => l.grow = Some(n.unwrap_or(1.0).max(0.0)),
            "minWidth" => l.min_w = n.unwrap_or(0.0).max(0.0),
            "maxWidth" => l.max_w = n.unwrap_or(MAX_DESIGN).max(0.0),
            "minHeight" => l.min_h = n.unwrap_or(0.0).max(0.0),
            "maxHeight" => l.max_h = n.unwrap_or(MAX_DESIGN).max(0.0),
            "size" => l.size = n.unwrap_or(15.0).clamp(0.0, 2000.0),
            "tiny" => l.size = 11.0,
            "small" => l.size = 13.0,
            "large" => l.size = 18.0,
            "huge" => l.size = 24.0,
            "weight" => l.bold = n.unwrap_or(400.0) >= 600.0,
            "bold" => l.bold = true,
            "muted" => l.color = MUTED,
            "nowrap" => l.wrap = false,
            "color" => {
                if let Some(c) = word(v).and_then(colour) {
                    l.color = c;
                }
            }
            "background" => l.background = word(v).and_then(colour),
            "round" => l.radius = n.unwrap_or(8.0).max(0.0),
            "pill" => l.radius = f32::INFINITY,
            "border" => match v {
                Style::Number(_) => {
                    l.border_w = n.unwrap_or(1.0).max(0.0);
                    l.border = Some(l.border.unwrap_or(0xdee2e6));
                }
                _ => l.border = Some(word(v).and_then(colour).unwrap_or(0xdee2e6)),
            },
            "borderWidth" => {
                l.border_w = n.unwrap_or(1.0).max(0.0);
                l.border = Some(l.border.unwrap_or(0xdee2e6));
            }
            "opacity" => l.opacity = n.unwrap_or(1.0).clamp(0.0, 1.0),
            "left" => l.halign = Align::Start,
            "center" => l.halign = Align::Center,
            "right" => l.halign = Align::End,
            "top" => l.valign = Align::Start,
            "middle" => l.valign = Align::Center,
            "bottom" => l.valign = Align::End,
            "spread" => l.spread = true,
            "subtle" => l.background = None,
            "disabled" => l.disabled = true,
            "from" => l.from = n.unwrap_or(0.0),
            "to" => l.to = n.unwrap_or(100.0),
            "x" => l.at_x = n.unwrap_or(0.0),
            "y" => l.at_y = n.unwrap_or(0.0),
            "toX" => l.to_x = n.unwrap_or(0.0),
            "toY" => l.to_y = n.unwrap_or(0.0),
            "radius" => l.rad = n.unwrap_or(10.0).max(0.0),
            "thick" => l.thick = n.unwrap_or(2.0).max(0.0),
            other => {
                // A colour on its own: a shape's fill, a bar's, or text's.
                if let Some(c) = colour(other) {
                    if kind == "button" {
                        // A button takes the colour, with text that reads on it.
                        l.background = Some(c);
                        l.color = if light(c) { INK } else { 0xffffff };
                    } else if matches!(kind, "box" | "circle") {
                        l.background = Some(c);
                    } else if kind == "progress" {
                        l.accent = c;
                    } else {
                        l.color = c;
                    }
                }
            }
        }
    }
}

/// Whether a colour is light (dark text reads on it), as Lust decides.
fn light(c: u32) -> bool {
    let [_, r, g, b] = c.to_be_bytes();
    0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b) > 160.0
}

/// A colour word (`red5`, `white`) or `#rgb` / `#rrggbb`, as Lust has them.
pub fn colour(v: &str) -> Option<u32> {
    if let Some(hex) = v.strip_prefix('#') {
        let full = match hex.len() {
            3 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
            6 => hex.to_string(),
            _ => return None,
        };
        return u32::from_str_radix(&full, 16).ok();
    }
    PALETTE.iter().find(|(n, _)| *n == v).map(|(_, c)| *c)
}

/// Lust's colour words (its `native_ui.rs`, `named`).
const PALETTE: &[(&str, u32)] = &[
    ("white", 0xffffff),
    ("black", 0x000000),
    ("gray1", 0xf8f9fa),
    ("gray2", 0xf1f3f5),
    ("gray3", 0xe9ecef),
    ("gray4", 0xdee2e6),
    ("gray5", 0xced4da),
    ("gray6", 0xadb5bd),
    ("gray7", 0x868e96),
    ("gray8", 0x495057),
    ("gray9", 0x212529),
    ("red", 0xe03131),
    ("orange", 0xf76707),
    ("yellow", 0xf59f00),
    ("green", 0x2f9e44),
    ("teal", 0x0c8599),
    ("blue", 0x1c7ed6),
    ("indigo", 0x4263eb),
    ("purple", 0x7048e8),
    ("pink", 0xd6336c),
    ("red1", 0xffe3e3),
    ("red2", 0xffc9c9),
    ("red3", 0xffa8a8),
    ("red4", 0xff8787),
    ("red5", 0xff6b6b),
    ("red6", 0xfa5252),
    ("red7", 0xf03e3e),
    ("red8", 0xe03131),
    ("red9", 0xc92a2a),
    ("orange1", 0xffe8cc),
    ("orange2", 0xffd8a8),
    ("orange3", 0xffc078),
    ("orange4", 0xffa94d),
    ("orange5", 0xff922b),
    ("orange6", 0xfd7e14),
    ("orange7", 0xf76707),
    ("orange8", 0xe8590c),
    ("orange9", 0xd9480f),
    ("yellow1", 0xfff3bf),
    ("yellow2", 0xffec99),
    ("yellow3", 0xffe066),
    ("yellow4", 0xffd43b),
    ("yellow5", 0xfcc419),
    ("yellow6", 0xfab005),
    ("yellow7", 0xf59f00),
    ("yellow8", 0xf08c00),
    ("yellow9", 0xe67700),
    ("green1", 0xd3f9d8),
    ("green2", 0xb2f2bb),
    ("green3", 0x8ce99a),
    ("green4", 0x69db7c),
    ("green5", 0x51cf66),
    ("green6", 0x40c057),
    ("green7", 0x37b24d),
    ("green8", 0x2f9e44),
    ("green9", 0x2b8a3e),
    ("teal1", 0xc5f6fa),
    ("teal2", 0x99e9f2),
    ("teal3", 0x66d9e8),
    ("teal4", 0x3bc9db),
    ("teal5", 0x22b8cf),
    ("teal6", 0x15aabf),
    ("teal7", 0x1098ad),
    ("teal8", 0x0c8599),
    ("teal9", 0x0b7285),
    ("blue1", 0xd0ebff),
    ("blue2", 0xa5d8ff),
    ("blue3", 0x74c0fc),
    ("blue4", 0x4dabf7),
    ("blue5", 0x339af0),
    ("blue6", 0x228be6),
    ("blue7", 0x1c7ed6),
    ("blue8", 0x1971c2),
    ("blue9", 0x1864ab),
    ("indigo1", 0xdbe4ff),
    ("indigo2", 0xbac8ff),
    ("indigo3", 0x91a7ff),
    ("indigo4", 0x748ffc),
    ("indigo5", 0x5c7cfa),
    ("indigo6", 0x4c6ef5),
    ("indigo7", 0x4263eb),
    ("indigo8", 0x3b5bdb),
    ("indigo9", 0x364fc7),
    ("purple1", 0xe5dbff),
    ("purple2", 0xd0bfff),
    ("purple3", 0xb197fc),
    ("purple4", 0x9775fa),
    ("purple5", 0x845ef7),
    ("purple6", 0x7950f2),
    ("purple7", 0x7048e8),
    ("purple8", 0x6741d9),
    ("purple9", 0x5f3dc4),
    ("pink1", 0xffdeeb),
    ("pink2", 0xfcc2d7),
    ("pink3", 0xfaa2c1),
    ("pink4", 0xf783ac),
    ("pink5", 0xf06595),
    ("pink6", 0xe64980),
    ("pink7", 0xd6336c),
    ("pink8", 0xc2255c),
    ("pink9", 0xa61e4d),
];

/// A piece placed: its look, and where it went.
struct Laid {
    kind: String,
    text: String,
    value: f32,
    look: Look,
    children: Vec<Laid>,
    /// Its number in the tree, for remembering sizes measured.
    id: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    lines: Vec<String>,
    /// Its number among the clickable pieces, if it's one.
    action: Option<usize>,
    /// Drawn with the focus ring.
    focused: bool,
}

/// Turns the tree into placed pieces, within the limits, with `looks`'
/// hover and focus styles on the pieces they name.
fn lay(node: &Node, depth: usize, count: &mut usize, looks: Looks) -> Laid {
    let id = *count;
    *count += 1;
    let mut look = base_look(&node.kind);
    apply(&mut look, &node.kind, &node.styles);
    let live = node.action.filter(|_| !look.disabled);
    if live.is_some() && live == looks.hover {
        // Lust's hover look, or a button a little darker.
        let hover: Vec<(String, Style)> = node
            .styles
            .iter()
            .filter_map(|(k, v)| match (k.as_str(), v) {
                ("hover", Style::Nested(inner)) => Some(inner.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        if hover.is_empty() {
            look.background = look.background.map(|c| shade(c, 0.9));
        } else {
            apply(&mut look, &node.kind, &hover);
        }
    }
    if look.disabled {
        look.opacity *= 0.45;
    }
    let focused = live.is_some() && live == looks.focus;
    let mut children = Vec::new();
    if depth + 1 < MAX_DEPTH {
        for c in &node.children {
            if *count >= MAX_ELEMENTS {
                break;
            }
            children.push(lay(c, depth + 1, count, looks));
        }
    }
    Laid {
        kind: node.kind.clone(),
        text: node.text.chars().take(MAX_TEXT).collect(),
        value: node
            .value
            .filter(|v| v.is_finite())
            .map_or(0.0, |v| v as f32),
        look,
        children,
        id,
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        lines: Vec::new(),
        action: live,
        focused,
    }
}

/// `c` with each channel times `k`.
fn shade(c: u32, k: f32) -> u32 {
    let [_, r, g, b] = c.to_be_bytes();
    let s = |v: u8| (f32::from(v) * k).clamp(0.0, 255.0) as u32;
    (s(r) << 16) | (s(g) << 8) | s(b)
}

/// Laying out: the text measurer, and sizes already measured.
struct Layout<'a> {
    measure: &'a mut dyn Measure,
    measured: std::collections::HashMap<(usize, u32), (f32, f32)>,
}

fn font_of(l: &Look) -> usize {
    usize::from(l.bold)
}

fn shows_text(kind: &str) -> bool {
    matches!(kind, "text" | "button")
}

fn flex(c: &Laid, row: bool) -> Option<f32> {
    if let Some(g) = c.look.grow {
        return (g > 0.0).then_some(g);
    }
    let s = if row { c.look.width } else { c.look.height };
    (s == Size::Fill || c.kind == "space").then_some(1.0)
}

fn wants_width(n: &Laid) -> bool {
    match n.kind.as_str() {
        "row" => n.look.halign != Align::Start || n.look.spread,
        "column" | "stack" => n.look.halign != Align::Start,
        _ => false,
    }
}

fn wants_height(n: &Laid) -> bool {
    match n.kind.as_str() {
        "column" => n.look.valign != Align::Start || n.look.spread,
        "stack" => n.look.valign != Align::Start,
        _ => false,
    }
}

fn clamp_w(c: &Laid, w: f32) -> f32 {
    w.min(c.look.max_w).max(c.look.min_w)
}

fn clamp_h(c: &Laid, h: f32) -> f32 {
    h.min(c.look.max_h).max(c.look.min_h)
}

fn offset(a: Align, room: f32) -> f32 {
    match a {
        Align::Start => 0.0,
        Align::Center => (room / 2.0).max(0.0),
        Align::End => room.max(0.0),
    }
}

fn spread_out(a: Align, spread: bool, room: f32, count: usize) -> (f32, f32) {
    if room <= 0.0 {
        return (0.0, 0.0);
    }
    if spread {
        return if count > 1 {
            (0.0, room / (count - 1) as f32)
        } else {
            (0.0, 0.0)
        };
    }
    (offset(a, room), 0.0)
}

impl Layout<'_> {
    fn text_width(&mut self, s: &str, l: &Look) -> f32 {
        self.measure.measure(font_of(l), s, l.size)[0]
    }

    fn line_height(&mut self, l: &Look) -> f32 {
        self.measure.measure(font_of(l), "", l.size)[1].ceil()
    }

    /// Breaks text into lines at `\n`, and between words where a line
    /// would be wider than `max_w` (a word too long alone stays whole).
    fn wrap_lines(&mut self, text: &str, l: &Look, max_w: f32) -> Vec<String> {
        let mut out = Vec::new();
        for para in text.split('\n') {
            if !l.wrap || !max_w.is_finite() || self.text_width(para, l) <= max_w + 0.5 {
                out.push(para.to_string());
                continue;
            }
            let mut line = String::new();
            for w in para.split(' ') {
                let joined = if line.is_empty() {
                    w.to_string()
                } else {
                    format!("{line} {w}")
                };
                if self.text_width(&joined, l) <= max_w + 0.5 || line.is_empty() {
                    line = joined;
                } else {
                    out.push(std::mem::take(&mut line));
                    line = w.to_string();
                }
            }
            out.push(line);
        }
        out
    }

    fn natural(&mut self, n: &Laid, max_w: f32) -> (f32, f32) {
        let key = (n.id, max_w.to_bits());
        if let Some(s) = self.measured.get(&key) {
            return *s;
        }
        let s = self.measure_node(n, max_w);
        self.measured.insert(key, s);
        s
    }

    fn measure_node(&mut self, n: &Laid, max_w: f32) -> (f32, f32) {
        let l = &n.look;
        let (padx, pady) = (l.pad[1] + l.pad[3], l.pad[0] + l.pad[2]);
        let outer = clamp_w(
            n,
            if let Size::Fixed(f) = l.width {
                f
            } else {
                max_w
            },
        );
        let inner = (outer - padx).max(0.0);
        let gaps = l.gap * (n.children.len().saturating_sub(1)) as f32;
        let (mut w, mut h) = match n.kind.as_str() {
            k if shows_text(k) => {
                let lines = self.wrap_lines(&n.text, l, inner);
                let mut w = 0.0_f32;
                for line in &lines {
                    w = w.max(self.text_width(line, l));
                }
                (w.ceil(), self.line_height(l) * lines.len().max(1) as f32)
            }
            "space" | "progress" | "canvas" | "box" | "line" => (0.0, 0.0),
            "circle" => (2.0 * l.rad, 2.0 * l.rad),
            "row" => {
                let sizes = self.row_sizes(n, inner);
                (
                    sizes.iter().map(|s| s.0).sum::<f32>() + gaps,
                    sizes.iter().map(|s| s.1).fold(0.0, f32::max),
                )
            }
            "stack" => {
                let mut size = (0.0_f32, 0.0_f32);
                for c in &n.children {
                    let s = self.natural(c, inner);
                    size = (size.0.max(s.0), size.1.max(s.1));
                }
                size
            }
            _ => {
                let mut size = (0.0_f32, gaps);
                for c in &n.children {
                    let s = self.natural(c, inner);
                    size = (size.0.max(s.0), size.1 + s.1);
                }
                size
            }
        };
        if wants_width(n) && inner.is_finite() {
            w = w.max(inner);
        }
        w += padx;
        h += pady;
        if let Size::Fixed(f) = l.width {
            w = f;
        }
        if let Size::Fixed(f) = l.height {
            h = f;
        }
        (clamp_w(n, w), clamp_h(n, h))
    }

    fn row_sizes(&mut self, n: &Laid, iw: f32) -> Vec<(f32, f32)> {
        let gaps = n.look.gap * (n.children.len().saturating_sub(1)) as f32;
        let widths: Vec<Option<f32>> = n
            .children
            .iter()
            .map(|c| {
                if flex(c, true).is_some() {
                    None
                } else {
                    Some(self.natural(c, iw).0)
                }
            })
            .collect();
        let fixed: f32 = widths.iter().flatten().sum();
        let total: f32 = n.children.iter().filter_map(|c| flex(c, true)).sum();
        let room = (iw - fixed - gaps).max(0.0);
        n.children
            .iter()
            .zip(widths)
            .map(|(c, w)| {
                let w = match w {
                    Some(w) => w,
                    None if iw.is_finite() && total > 0.0 => {
                        clamp_w(c, room * flex(c, true).unwrap_or(0.0) / total)
                    }
                    None => self.natural(c, f32::INFINITY).0,
                };
                let h = match c.look.height {
                    Size::Fixed(f) => clamp_h(c, f),
                    _ => self.natural(c, w).1,
                };
                (w, h)
            })
            .collect()
    }

    fn cross_width(&mut self, c: &Laid, iw: f32) -> f32 {
        let w = match c.look.width {
            Size::Fill => iw,
            Size::Fixed(f) => f,
            Size::Auto => self.natural(c, iw).0.min(iw),
        };
        clamp_w(c, w)
    }

    /// Places a piece in the room it's given, then what it holds.
    fn place(&mut self, n: &mut Laid, x: f32, y: f32, w: f32, h: f32) {
        n.x = x;
        n.y = y;
        n.w = w;
        n.h = h;
        let l = n.look.clone();
        let (ix, iy) = (x + l.pad[3], y + l.pad[0]);
        let iw = (w - l.pad[1] - l.pad[3]).max(0.0);
        let ih = (h - l.pad[0] - l.pad[2]).max(0.0);
        if shows_text(&n.kind) {
            n.lines = self.wrap_lines(&n.text.clone(), &l, iw);
            return;
        }
        if n.children.is_empty() {
            return;
        }
        let count = n.children.len();
        let mut children = std::mem::take(&mut n.children);
        match n.kind.as_str() {
            "canvas" => {
                for c in &mut children {
                    let cl = c.look.clone();
                    let (cx, cy, cw, ch) = match c.kind.as_str() {
                        "circle" => (
                            cl.at_x - cl.rad,
                            cl.at_y - cl.rad,
                            2.0 * cl.rad,
                            2.0 * cl.rad,
                        ),
                        "line" => (
                            cl.at_x.min(cl.to_x),
                            cl.at_y.min(cl.to_y),
                            (cl.to_x - cl.at_x).abs(),
                            (cl.to_y - cl.at_y).abs(),
                        ),
                        _ => {
                            let cw = match cl.width {
                                Size::Fixed(f) => clamp_w(c, f),
                                _ => self.natural(c, iw).0,
                            };
                            let ch = match cl.height {
                                Size::Fixed(f) => clamp_h(c, f),
                                _ => self.natural(c, cw).1,
                            };
                            (cl.at_x, cl.at_y, cw, ch)
                        }
                    };
                    self.place(c, ix + cx, iy + cy, cw, ch);
                }
            }
            "stack" => {
                for c in &mut children {
                    let cw = self.cross_width(c, iw);
                    let ch = match c.look.height {
                        Size::Fill => ih,
                        Size::Fixed(f) => f,
                        Size::Auto => self.natural(c, cw).1,
                    };
                    let ch = clamp_h(c, ch);
                    let cx = ix + offset(l.halign, iw - cw);
                    let cy = iy + offset(l.valign, ih - ch);
                    self.place(c, cx.round(), cy.round(), cw.round(), ch.round());
                }
            }
            "row" => {
                n.children = children;
                let sizes = self.row_sizes(n, iw);
                children = std::mem::take(&mut n.children);
                let used: f32 = sizes.iter().map(|s| s.0).sum::<f32>() + l.gap * (count - 1) as f32;
                let (mut at, extra) = spread_out(l.halign, l.spread, iw - used, count);
                for (c, (cw, nh)) in children.iter_mut().zip(sizes) {
                    let ch = match c.look.height {
                        Size::Fill => ih,
                        Size::Fixed(f) => f,
                        Size::Auto if wants_height(c) => ih,
                        Size::Auto => nh.min(ih),
                    };
                    let ch = clamp_h(c, ch);
                    let cy = iy + offset(l.valign, ih - ch);
                    self.place(c, (ix + at).round(), cy.round(), cw.round(), ch.round());
                    at += cw + l.gap + extra;
                }
            }
            _ => {
                // A column, and the view itself.
                let mut widths = Vec::with_capacity(count);
                let mut heights = Vec::with_capacity(count);
                for c in &children {
                    let cw = self.cross_width(c, iw);
                    widths.push(cw);
                    heights.push(match (flex(c, false), c.look.height) {
                        (Some(_), _) => None,
                        (None, Size::Fixed(f)) => Some(clamp_h(c, f)),
                        (None, _) => Some(self.natural(c, cw).1),
                    });
                }
                let gaps = l.gap * (count - 1) as f32;
                let fixed: f32 = heights.iter().flatten().sum();
                let total: f32 = children.iter().filter_map(|c| flex(c, false)).sum();
                let room = (ih - fixed - gaps).max(0.0);
                let heights: Vec<f32> = children
                    .iter()
                    .zip(heights)
                    .map(|(c, h)| {
                        h.unwrap_or_else(|| {
                            if total > 0.0 {
                                clamp_h(c, room * flex(c, false).unwrap_or(0.0) / total)
                            } else {
                                0.0
                            }
                        })
                    })
                    .collect();
                let used: f32 = heights.iter().sum::<f32>() + gaps;
                let (mut at, extra) = spread_out(l.valign, l.spread, ih - used, count);
                for ((c, cw), ch) in children.iter_mut().zip(widths).zip(heights) {
                    let cx = ix + offset(l.halign, iw - cw);
                    self.place(c, cx.round(), (iy + at).round(), cw.round(), ch.round());
                    at += ch + l.gap + extra;
                }
            }
        }
        n.children = children;
    }
}

/// `v` as one of the screen's colours, `opacity` worked in.
fn rgb(v: u32, opacity: f32) -> Colour {
    let [_, r, g, b] = v.to_be_bytes();
    Colour::rgb(r, g, b).with_alpha(opacity)
}

/// Lays `view` out in a screen `size` design pixels big (as wide as the
/// real screen's shape, and the view's design height), into elements in
/// drawing order: each placed from the top left, in design pixels.
pub fn lay_out(view: &Node, size: [f32; 2], measure: &mut dyn Measure) -> Vec<Element> {
    lay_out_with(view, size, measure, Looks::default()).0
}

/// [`lay_out`], with hover and focus looks, giving also where each
/// clickable piece went (frontmost last).
pub fn lay_out_with(
    view: &Node,
    size: [f32; 2],
    measure: &mut dyn Measure,
    looks: Looks,
) -> (Vec<Element>, Vec<Clickable>) {
    let mut count = 0;
    let mut top = lay(view, 0, &mut count, looks);
    // The view's top is a column as big as the screen; columns and rows
    // directly in it are as wide as it, as in Lust's windows.
    top.kind = "view".into();
    top.look = base_look("view");
    for c in &mut top.children {
        if matches!(c.kind.as_str(), "column" | "row") && c.look.width == Size::Auto {
            c.look.width = Size::Fill;
        }
    }
    let size = size.map(|v| {
        if v.is_finite() {
            v.clamp(0.0, MAX_DESIGN)
        } else {
            0.0
        }
    });
    let mut layout = Layout {
        measure,
        measured: std::collections::HashMap::new(),
    };
    layout.place(&mut top, 0.0, 0.0, size[0], size[1]);
    let mut out = Vec::new();
    emit(&top, 1.0, &mut out);
    let mut clickable = Vec::new();
    clickables(&top, &mut clickable);
    (out, clickable)
}

/// Every clickable piece's box, in drawing order.
fn clickables(n: &Laid, out: &mut Vec<Clickable>) {
    if let Some(action) = n.action {
        out.push(Clickable {
            rect: [n.x, n.y, n.x + n.w, n.y + n.h],
            action,
        });
    }
    for c in &n.children {
        clickables(c, out);
    }
}

/// The frontmost clickable piece at `at` (design pixels), if any.
pub fn hit(clickable: &[Clickable], at: [f32; 2]) -> Option<usize> {
    clickable
        .iter()
        .rev()
        .find(|c| {
            at[0] >= c.rect[0] && at[0] < c.rect[2] && at[1] >= c.rect[1] && at[1] < c.rect[3]
        })
        .map(|c| c.action)
}

/// Where the keys' focus goes from the clickable piece `from` (its
/// number), moving `towards` a way (x right, y down; any length): the
/// nearest piece that way, nearer counting more along the way than across
/// it, as most games' menus move (decision 0088). With no focus yet (or one
/// no longer there), the first piece; with nothing that way, it stays.
pub fn next_focus(
    clickable: &[Clickable],
    from: Option<usize>,
    towards: [f32; 2],
) -> Option<usize> {
    let centre = |c: &Clickable| [(c.rect[0] + c.rect[2]) / 2.0, (c.rect[1] + c.rect[3]) / 2.0];
    let Some(now) = from.and_then(|a| clickable.iter().find(|c| c.action == a)) else {
        return clickable.first().map(|c| c.action);
    };
    let length = (towards[0] * towards[0] + towards[1] * towards[1]).sqrt();
    if !length.is_finite() || length == 0.0 {
        return Some(now.action);
    }
    let way = [towards[0] / length, towards[1] / length];
    let here = centre(now);
    let mut best: Option<(f32, usize)> = None;
    for c in clickable.iter().filter(|c| c.action != now.action) {
        let there = centre(c);
        let d = [there[0] - here[0], there[1] - here[1]];
        let along = d[0] * way[0] + d[1] * way[1];
        // Only pieces past this one's edge that way, not beside it.
        if along.is_nan() || along <= 0.5 {
            continue;
        }
        let across = (d[0] * way[1] - d[1] * way[0]).abs();
        let score = along + 2.0 * across;
        if best.is_none_or(|(b, _)| score < b) {
            best = Some((score, c.action));
        }
    }
    Some(best.map_or(now.action, |(_, a)| a))
}

/// Whether a view has anything to click (a button): such a screen frees
/// the pointer while it's shown. Looked for no deeper than the layout goes.
pub fn has_actions(view: &Node) -> bool {
    fn look(node: &Node, depth: usize) -> bool {
        node.action.is_some()
            || depth < MAX_DEPTH && node.children.iter().any(|c| look(c, depth + 1))
    }
    look(view, 0)
}

/// The design size a view asks for (`view [width 1920, height 1080]`),
/// 1920 × 1080 without one.
pub fn design_of(view: &Node) -> [f32; 2] {
    let mut size = [1920.0, 1080.0];
    for (k, v) in &view.styles {
        if let Some(n) = num(v).filter(|n| *n >= 1.0) {
            match k.as_str() {
                "width" => size[0] = n,
                "height" => size[1] = n,
                _ => {}
            }
        }
    }
    size
}

/// The elements one placed piece draws, then those inside it.
fn emit(n: &Laid, opacity: f32, out: &mut Vec<Element>) {
    if out.len() >= MAX_ELEMENTS {
        return;
    }
    let l = &n.look;
    let opacity = opacity * l.opacity;
    let at = |x: f32, y: f32, w: f32, h: f32, kind: ElementKind| {
        Element::new("", kind)
            .sized([w.max(0.0), h.max(0.0)])
            .at(crate::screen::Anchor::TOP_LEFT, [x, y])
    };
    let round = |w: f32, h: f32| l.radius.min(w.min(h) * 0.5).max(0.0);
    match n.kind.as_str() {
        "line" => {
            // Its ends: from (x, y) to (toX, toY), so it may rise.
            let rising = (l.to_y < l.at_y) != (l.to_x < l.at_x);
            let mut e = at(
                n.x,
                n.y,
                n.w,
                n.h,
                ElementKind::Line {
                    width: l.thick,
                    rising,
                },
            );
            e.fill = Fill::Solid(rgb(l.color, opacity));
            out.push(e);
            return;
        }
        "circle" => {
            let mut e = at(n.x, n.y, n.w, n.h, ElementKind::Ellipse);
            e.fill = Fill::Solid(l.background.map_or(Colour::NONE, |c| rgb(c, opacity)));
            e.border = l.border.map(|c| (l.border_w, rgb(c, opacity)));
            out.push(e);
            return;
        }
        "progress" => {
            let span = l.to - l.from;
            let value = if span.abs() > f32::EPSILON {
                ((n.value - l.from) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut e = at(
                n.x,
                n.y,
                n.w,
                n.h,
                ElementKind::Bar {
                    round: round(n.w, n.h),
                    value,
                    background: l.background.map_or(Colour::NONE, |c| rgb(c, opacity)),
                },
            );
            e.fill = Fill::Solid(rgb(l.accent, opacity));
            out.push(e);
            return;
        }
        _ => {}
    }
    // Anything else draws its background and border, if it has one.
    if l.background.is_some() || l.border.is_some() {
        let mut e = at(
            n.x,
            n.y,
            n.w,
            n.h,
            ElementKind::Box {
                round: round(n.w, n.h),
            },
        );
        e.fill = Fill::Solid(l.background.map_or(Colour::NONE, |c| rgb(c, opacity)));
        e.border = l.border.map(|c| (l.border_w, rgb(c, opacity)));
        out.push(e);
    }
    if n.focused {
        // The focus ring: the accent's blue, just outside the piece.
        let mut ring = at(
            n.x - 3.0,
            n.y - 3.0,
            n.w + 6.0,
            n.h + 6.0,
            ElementKind::Box {
                round: round(n.w, n.h) + 3.0,
            },
        );
        ring.border = Some((2.5, rgb(0x4dabf7, opacity)));
        out.push(ring);
    }
    if shows_text(&n.kind) && !n.lines.is_empty() {
        let (ix, iy) = (n.x + l.pad[3], n.y + l.pad[0]);
        let iw = (n.w - l.pad[1] - l.pad[3]).max(0.0);
        let ih = (n.h - l.pad[0] - l.pad[2]).max(0.0);
        let align = match l.halign {
            Align::Start => Edge::Start,
            Align::Center => Edge::Middle,
            Align::End => Edge::End,
        };
        let mut e = at(
            ix,
            iy,
            iw,
            ih,
            ElementKind::Text {
                text: n.lines.join("\n"),
                size: l.size,
                font: font_of(l),
                align,
            },
        );
        e.fill = Fill::Solid(rgb(l.color, opacity));
        out.push(e);
    }
    for c in &n.children {
        emit(c, opacity, out);
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
