//! A screen open in the UI editor (decision 0089, the canvas): its Lust
//! file's text, the screen's pieces read from it ([`ui::source`]), what's
//! picked, hidden and locked, undo, and the screen's view as the canvas
//! shows it. Every change edits the fewest letters it can and is saved at
//! once by the workbench (`workbench_screens.rs`).

use std::collections::HashSet;
use ui::source::{self, Block, ElementSource, Piece, ScreenSource};
use ui::view::{Node, Style};

/// The most undo steps kept.
const MAX_UNDO: usize = 200;

/// A screen open for editing.
#[derive(Debug, Clone)]
pub(crate) struct ScreenDoc {
    /// Its file, inside the assets (`ui/pause.lst`).
    pub(crate) path: String,
    /// The screen's name (its assembly's).
    pub(crate) screen: String,
    /// The file's text as it is now.
    pub(crate) text: String,
    /// The screen read from it, or why it couldn't be.
    pub(crate) source: Result<ScreenSource, String>,
    /// The pieces picked, by their place in the file's view (see
    /// [`Block::at`]); the first is the one Properties shows.
    pub(crate) picked: Vec<Vec<usize>>,
    /// Pieces hidden while editing (never saved).
    pub(crate) hidden: HashSet<Vec<usize>>,
    /// Pieces that can't be picked on the canvas.
    pub(crate) locked: HashSet<Vec<usize>>,
    /// The screen's view as the canvas draws it: the program's, with the
    /// canvas's changes made to it straight away.
    pub(crate) view: Option<Node>,
    /// Why there's no view, if there isn't.
    pub(crate) view_problem: Option<String>,
    /// Whether the view is behind the text (a piece was added, moved or
    /// taken out), until the code is checked again.
    pub(crate) behind: bool,
    /// The preview size's number in [`ui::presets::PRESETS`].
    pub(crate) preset: usize,
    /// Whether the safe area's guide shows.
    pub(crate) safe_area: bool,
    /// Whether sizes snap to others' edges.
    pub(crate) snap: bool,
    /// How big the screen is drawn (1 = its pixels as the screen's), or
    /// `None` to fit the canvas.
    pub(crate) zoom: Option<f32>,
    /// How far it's moved from the middle, in points.
    pub(crate) pan: egui::Vec2,
    /// The tool in use on the canvas (M5).
    pub(crate) tool: crate::canvas_tools::CanvasTool,
    /// The pen's corners so far, in design pixels on the screen.
    pub(crate) pen: Vec<[f32; 2]>,
    /// The picture the Image tool puts (a path inside the assets).
    pub(crate) picture: Option<String>,
    /// Values tried on the canvas in place of components' starting ones
    /// (component, field), never saved.
    pub(crate) preview: std::collections::BTreeMap<(String, String), f64>,
    /// The view is to be drawn again (a preview value changed).
    pub(crate) redraw: bool,
    undo: Vec<String>,
    redo: Vec<String>,
    /// The text Solis last saved, to tell its own save from a change on
    /// disk.
    pub(crate) saved: String,
}

/// Reads `name`'s screen from `text`.
fn read(text: &str, name: &str) -> Result<ScreenSource, String> {
    source::parse(text)?
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("there's no screen called {name} in it any more"))
}

/// A style's value as written (`12`, `fill`, `[background red5]`, or
/// nothing for a word alone) as the view has it.
pub(crate) fn style_of(value: Option<&str>) -> Style {
    match value.map(str::trim) {
        None | Some("") => Style::Flag(true),
        Some(v) => match v.parse::<f64>() {
            Ok(n) if n.is_finite() => Style::Number(n),
            _ => Style::Text(v.trim_matches('"').to_string()),
        },
    }
}

/// The piece in `view` starting where `piece` starts (or, after a change
/// before it, the nearest of the same kind).
fn find_same(view: &Block, piece: &ElementSource) -> Option<ElementSource> {
    fn walk(block: &Block, piece: &ElementSource, best: &mut Option<ElementSource>) {
        for p in &block.pieces {
            if let Piece::Element(e) = p {
                if e.kind == piece.kind {
                    let near = |x: &ElementSource| x.span.start.abs_diff(piece.span.start);
                    if best.as_ref().is_none_or(|b| near(e) < near(b)) {
                        *best = Some(e.clone());
                    }
                }
                if let Some(c) = &e.children {
                    walk(c, piece, best);
                }
            }
        }
    }
    let mut best = None;
    walk(view, piece, &mut best);
    best
}

/// The pieces made by `block`'s source matched with the view's nodes made
/// from them, in order, as (file place, view place). Code that makes
/// pieces (a loop, a question) stops the matching in its block: what it
/// and those after it make is the code's.
pub(crate) fn matched(block: &Block, node: &Node) -> Vec<(Vec<usize>, Vec<usize>)> {
    fn walk(
        block: &Block,
        node: &Node,
        file: &mut Vec<usize>,
        seen: &mut Vec<usize>,
        out: &mut Vec<(Vec<usize>, Vec<usize>)>,
    ) {
        let mut child = 0;
        for (i, piece) in block.pieces.iter().enumerate() {
            match piece {
                Piece::Code(c) if c.makes_pieces => return,
                Piece::Code(_) => {}
                Piece::Element(e) => {
                    let Some(n) = node.children.get(child) else {
                        return;
                    };
                    // A game's own piece (`TodoRow`) is a "piece" in the view.
                    let game_piece = e.kind.starts_with(|c: char| c.is_ascii_uppercase());
                    if n.kind != e.kind && !(game_piece && n.kind == "piece") {
                        return;
                    }
                    file.push(i);
                    seen.push(child);
                    out.push((file.clone(), seen.clone()));
                    if let Some(inner) = &e.children {
                        walk(inner, n, file, seen, out);
                    }
                    file.pop();
                    seen.pop();
                    child += 1;
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(block, node, &mut Vec::new(), &mut Vec::new(), &mut out);
    out
}

/// The node at `path` (child indices) under `node`.
pub(crate) fn node_at_mut<'a>(node: &'a mut Node, path: &[usize]) -> Option<&'a mut Node> {
    match path.split_first() {
        None => Some(node),
        Some((i, rest)) => node_at_mut(node.children.get_mut(*i)?, rest),
    }
}

impl ScreenDoc {
    /// The screen `screen` in the file at `path`, as `text`.
    pub(crate) fn new(path: String, screen: String, text: String) -> Self {
        let source = read(&text, &screen);
        Self {
            path,
            screen,
            saved: text.clone(),
            text,
            source,
            picked: Vec::new(),
            hidden: HashSet::new(),
            locked: HashSet::new(),
            view: None,
            view_problem: None,
            behind: false,
            preset: 0,
            safe_area: true,
            snap: true,
            zoom: None,
            pan: egui::Vec2::ZERO,
            tool: Default::default(),
            pen: Vec::new(),
            picture: None,
            preview: Default::default(),
            redraw: false,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The view's pieces, if the file could be read.
    pub(crate) fn block(&self) -> Option<&Block> {
        self.source.as_ref().ok().map(|s| &s.view)
    }

    /// The piece at `path` in the file.
    pub(crate) fn piece(&self, path: &[usize]) -> Option<&ElementSource> {
        self.block()?.at(path)
    }

    /// The file place matched with the view place `seen`, or the other
    /// way, from the view as it is.
    pub(crate) fn matches(&self) -> Vec<(Vec<usize>, Vec<usize>)> {
        match (self.block(), &self.view) {
            (Some(b), Some(v)) if !self.behind => matched(b, v),
            _ => Vec::new(),
        }
    }

    /// Puts `text` in as one undo step (unless it's the same), reading it
    /// again. Gives whether it changed.
    pub(crate) fn change(&mut self, text: String) -> bool {
        if text == self.text {
            return false;
        }
        // A change that leaves the screen unreadable is refused: the
        // canvas never saves a broken file.
        if read(&text, &self.screen).is_err() && self.source.is_ok() {
            return false;
        }
        let before = std::mem::replace(&mut self.text, text);
        self.undo.push(before);
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.read_again();
        true
    }

    /// Reads the text again; what's picked and hidden is kept where it
    /// still names a piece.
    fn read_again(&mut self) {
        self.source = read(&self.text, &self.screen);
        let keep = |paths: &mut Vec<Vec<usize>>, block: Option<&Block>| {
            paths.retain(|p| block.and_then(|b| b.at(p)).is_some());
        };
        let block = self.source.as_ref().ok().map(|s| s.view.clone());
        keep(&mut self.picked, block.as_ref());
        let alive = |p: &Vec<usize>| block.as_ref().and_then(|b| b.at(p)).is_some();
        self.hidden.retain(alive);
        self.locked.retain(alive);
    }

    /// Forgets what's picked, hidden and locked: after pieces were added,
    /// taken out or moved, their places name other pieces.
    fn forget_places(&mut self) {
        self.picked.clear();
        self.hidden.clear();
        self.locked.clear();
    }

    /// Undoes the last change; whether there was one.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(before) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.text, before));
        self.read_again();
        self.forget_places();
        self.behind = true;
        true
    }

    /// Redoes the last change undone; whether there was one.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(after) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.text, after));
        self.read_again();
        self.forget_places();
        self.behind = true;
        true
    }

    /// The view's node made from the file's piece at `path`.
    fn seen_node(&mut self, path: &[usize]) -> Option<&mut Node> {
        let seen = self
            .matches()
            .into_iter()
            .find(|(f, _)| f == path)
            .map(|(_, s)| s)?;
        node_at_mut(self.view.as_mut()?, &seen)
    }

    /// Sets the `[ ]` item `name` of every piece picked to `value`
    /// (`None`: the word alone), as one undo step; the canvas shows it at
    /// once. Gives whether anything changed.
    pub(crate) fn set_style(&mut self, name: &str, value: Option<&str>) -> bool {
        self.edit_picked(
            |text, piece| source::set_style(text, piece, name, value),
            |node| {
                node.styles.retain(|(k, _)| k != name);
                node.styles.push((name.to_string(), style_of(value)));
            },
        )
    }

    /// Takes the `[ ]` items `remove` away from every piece picked and
    /// sets those in `set` (a name and its value, `None` for a word
    /// alone), as one undo step.
    pub(crate) fn restyle(&mut self, remove: &[&str], set: &[(&str, Option<&str>)]) -> bool {
        let screen = self.screen.clone();
        self.edit_picked(
            |text, piece| {
                let mut t = source::remove_styles(text, piece, remove);
                for (name, value) in set {
                    let Some(p) = read(&t, &screen)
                        .ok()
                        .and_then(|s| find_same(&s.view, piece))
                    else {
                        break;
                    };
                    t = source::set_style(&t, &p, name, *value);
                }
                t
            },
            |node| {
                node.styles.retain(|(k, _)| !remove.contains(&k.as_str()));
                for (name, value) in set {
                    node.styles.retain(|(k, _)| k != name);
                    node.styles.push((name.to_string(), style_of(*value)));
                }
            },
        )
    }

    /// Takes the `[ ]` items `names` away from every piece picked.
    pub(crate) fn remove_styles(&mut self, names: &[&str]) -> bool {
        self.edit_picked(
            |text, piece| source::remove_styles(text, piece, names),
            |node| {
                node.styles.retain(|(k, _)| !names.contains(&k.as_str()));
            },
        )
    }

    /// Sets the words of the first piece picked.
    pub(crate) fn set_words(&mut self, words: &str) -> bool {
        let Some(path) = self.picked.first().cloned() else {
            return false;
        };
        let Some(piece) = self.piece(&path).cloned() else {
            return false;
        };
        let Some(text) = source::set_words(&self.text, &piece, words) else {
            return false;
        };
        let changed = self.change(text);
        if changed && let Some(node) = self.seen_node(&path) {
            node.text = words.to_string();
        }
        changed
    }

    /// Makes one change to each piece picked, last in the file first so
    /// the places of those before stay right, as one undo step; `seen`
    /// makes the same change to the view so the canvas shows it at once.
    fn edit_picked(
        &mut self,
        edit: impl Fn(&str, &ElementSource) -> String,
        seen: impl Fn(&mut Node),
    ) -> bool {
        let mut picked = self.picked.clone();
        picked.sort();
        picked.dedup();
        // Last first, by where each starts in the file.
        let mut pieces: Vec<(Vec<usize>, ElementSource)> = picked
            .iter()
            .filter_map(|p| Some((p.clone(), self.piece(p)?.clone())))
            .collect();
        pieces.sort_by_key(|(_, e)| std::cmp::Reverse(e.span.start));
        // Pieces inside others change only their own letters, which are
        // inside the others' and before their ends: done last-first, each
        // edit leaves the earlier ones' places right, except a parent's
        // own [ ] (which comes before its children): read again each time.
        let mut text = self.text.clone();
        for (path, _) in &pieces {
            let Some(piece) = read(&text, &self.screen)
                .ok()
                .and_then(|s| s.view.at(path).cloned())
            else {
                continue;
            };
            text = edit(&text, &piece);
        }
        let changed = self.change(text);
        if changed {
            for (path, _) in &pieces {
                if let Some(node) = self.seen_node(path) {
                    seen(node);
                }
            }
        }
        changed
    }

    /// Takes the pieces picked out of the file.
    pub(crate) fn delete_picked(&mut self) -> bool {
        let mut picked = self.picked.clone();
        // Deepest and last first, so the others' places stay right; a
        // piece inside another picked one goes with it.
        picked.sort();
        picked.dedup();
        let all = picked.clone();
        picked.retain(|p| !all.iter().any(|q| q != p && p.starts_with(q)));
        picked.sort_by(|a, b| b.cmp(a));
        let mut text = self.text.clone();
        for path in &picked {
            let Some(piece) = read(&text, &self.screen)
                .ok()
                .and_then(|s| s.view.at(path).cloned())
            else {
                continue;
            };
            text = source::remove_piece(&text, &piece);
        }
        let changed = self.change(text);
        if changed {
            self.forget_places();
            self.behind = true;
        }
        changed
    }

    /// Copies the pieces picked, each just after itself.
    pub(crate) fn duplicate_picked(&mut self) -> bool {
        let mut picked = self.picked.clone();
        picked.sort();
        picked.dedup();
        // A piece inside another picked one is copied with it.
        let all = picked.clone();
        picked.retain(|p| !all.iter().any(|q| q != p && p.starts_with(q)));
        picked.sort_by(|a, b| b.cmp(a));
        let mut text = self.text.clone();
        let mut copies: Vec<Vec<usize>> = Vec::new();
        for path in &picked {
            let Some((parent, own)) = path.split_last().map(|(o, p)| (p.to_vec(), *o)) else {
                continue;
            };
            let Ok(s) = read(&text, &self.screen) else {
                continue;
            };
            let (Some(piece), Some(block)) = (s.view.at(path), s.view.block_at(&parent)) else {
                continue;
            };
            let own_indent = {
                let start = text[..piece.span.start].rfind('\n').map_or(0, |i| i + 1);
                text[start..piece.span.start].to_string()
            };
            let copy: String = text[piece.span.clone()]
                .lines()
                .enumerate()
                .map(|(i, l)| {
                    if i == 0 {
                        l.to_string()
                    } else {
                        l.strip_prefix(own_indent.as_str()).unwrap_or(l).to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            text = source::insert_piece(&text, block, own + 1, &copy);
            // Copies made already, after this one in the same block or
            // inside a piece after it, move down one.
            for c in &mut copies {
                if c.len() > parent.len() && c.starts_with(&parent) && c[parent.len()] > own {
                    c[parent.len()] += 1;
                }
            }
            let mut at = parent;
            at.push(own + 1);
            copies.push(at);
        }
        let changed = self.change(text);
        if changed {
            self.hidden.clear();
            self.locked.clear();
            self.picked = copies;
            self.behind = true;
        }
        changed
    }

    /// Moves the first piece picked into the block at `into` (a piece's
    /// place, `[]` for the view's own) at `index`.
    pub(crate) fn move_picked(&mut self, into: &[usize], index: usize) -> bool {
        let Some(path) = self.picked.first().cloned() else {
            return false;
        };
        let Some(text) = source::move_piece(&self.text, &self.screen, &path, into, index) else {
            return false;
        };
        let changed = self.change(text);
        if changed {
            // Where it is now: found by its new place.
            let (parent, own) = path.split_at(path.len() - 1);
            let mut index = index;
            let mut into = into.to_vec();
            if parent == into.as_slice() && own[0] < index {
                index -= 1;
            }
            if into.len() > parent.len() && into.starts_with(parent) && into[parent.len()] > own[0]
            {
                into[parent.len()] -= 1;
            }
            let len = self
                .block()
                .and_then(|b| b.block_at(&into))
                .map_or(0, |b| b.pieces.len());
            into.push(index.min(len.saturating_sub(1)));
            self.hidden.clear();
            self.locked.clear();
            self.picked = vec![into];
            self.behind = true;
        }
        changed
    }

    /// The file changed on disk to `text` (not Solis's own save): taken in
    /// as one undo step.
    pub(crate) fn taken_in(&mut self, text: String) {
        self.saved = text.clone();
        if self.change(text) {
            self.forget_places();
            self.behind = true;
        }
    }

    /// Whether `text` is what's open or what Solis last saved.
    pub(crate) fn is_own(&self, text: &str) -> bool {
        text == self.text || text == self.saved
    }
}

#[cfg(test)]
#[path = "screen_doc_tests.rs"]
pub(crate) mod tests;
