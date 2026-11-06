//! Pictures on the game's screens ([decision 0089](../../../docs/decisions/0089-atlas-crop-and-the-ui-editor.md)):
//! every `image "sprites/tools.png"` and `image "sprites/tools.png#Axe"`
//! the screens name, in their code or as they run, packed into one
//! [`ui::Pictures`] the screens draw from. The app reads each picture file
//! asked for ([`Workbench::take_wanted_pictures`]); a name written in a
//! screen's code that names no picture, or no crop of it, is in Problems
//! with its line, before the game runs.

use crate::console::LogLevel;
use crate::sprite_sheets::{ArtState, AtlasState, SpriteSheets, is_picture};
use crate::workbench::Workbench;
use script::CodeProblem;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// The most picture names seen in running screens that are kept.
pub(crate) const MAX_SEEN: usize = 1024;

/// The screens' pictures, and what's wrong with those named.
#[derive(Debug, Default)]
pub(crate) struct ScreenPictures {
    /// The pictures packed, for every screen to draw from.
    pub(crate) pictures: Option<Arc<ui::Pictures>>,
    /// Pictures named in screens' code that can't be shown, where.
    pub(crate) problems: Vec<CodeProblem>,
    /// Names the running screens' views gave (perhaps worked out as the
    /// game runs), in their order.
    pub(crate) seen: BTreeSet<String>,
    /// What the pictures were made from, to tell when to make them again.
    from: Option<(u64, usize, usize)>,
    /// The picture files last asked for, and whether they changed since
    /// the app last took them.
    wanted: BTreeSet<String>,
    wanted_changed: bool,
    /// Names worked out as the game ran that couldn't be shown, said once.
    told: HashSet<String>,
}

/// Every image name in `view` and what's inside it, into `out` (at most
/// [`MAX_SEEN`] in all).
pub(crate) fn names_in(view: &ui::view::Node, out: &mut BTreeSet<String>) {
    fn walk(node: &ui::view::Node, depth: usize, out: &mut BTreeSet<String>) {
        if out.len() >= MAX_SEEN || depth >= ui::screen::MAX_DEPTH {
            return;
        }
        if node.kind == "image" && !node.text.is_empty() {
            out.insert(node.text.chars().take(ui::pictures::MAX_NAME).collect());
        }
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }
    walk(view, 0, out);
}

/// A picture's pixels (straight RGBA) and its size.
type Pixels = (Vec<u8>, [u32; 2]);

/// Why the picture `name` can't be shown, or its pixels and size (`None`
/// for both while its file is still being read).
fn pixels_of(
    name: &str,
    sheets: &SpriteSheets,
    in_project: &dyn Fn(&str) -> bool,
) -> Result<Option<Pixels>, String> {
    let (file, crop) = ui::pictures::split(name);
    if !is_picture(file) {
        return Err(format!(
            "{file} isn't a picture: image names a PNG, JPEG or Aseprite file inside the assets, \
             like image \"sprites/tools.png\""
        ));
    }
    if !in_project(file) {
        return Err(format!(
            "there's no picture {file} in the project: image names a picture by its place \
             inside the assets, like image \"sprites/tools.png\""
        ));
    }
    let art = match sheets.tile_art.get(file) {
        Some(ArtState::Read(art)) => art,
        Some(ArtState::Failed(why)) => return Err(format!("{file} couldn't be read: {why}")),
        None => return Ok(None),
    };
    let Some(rgba) = art.frames.first() else {
        return Err(format!("{file} has no pictures in it"));
    };
    let Some(crop) = crop else {
        return Ok(Some((rgba.clone(), art.size)));
    };
    let atlas_path = animation::atlas::file_for(file);
    let atlas = match sheets.atlases.get(&atlas_path) {
        Some(AtlasState::Read(atlas, _)) => atlas,
        Some(AtlasState::Failed(why)) => {
            return Err(format!(
                "{atlas_path}, {file}'s crops, couldn't be read: {why}"
            ));
        }
        None => {
            return Err(format!(
                "{file} has no crops yet: right-click it in the Library and choose Atlas Crop… \
                 to cut out one called {crop}"
            ));
        }
    };
    let Some(found) = atlas.crop(crop) else {
        let names: Vec<&str> = atlas
            .crops
            .iter()
            .take(8)
            .map(|c| c.name.as_str())
            .collect();
        let more = if atlas.crops.len() > names.len() {
            ", …"
        } else {
            ""
        };
        return Err(if names.is_empty() {
            format!("{file} has no crops yet: cut one called {crop} in Atlas Crop")
        } else {
            format!(
                "{file} has no crop called {crop} (its crops: {}{more})",
                names.join(", ")
            )
        });
    };
    ui::pictures::cut(rgba, art.size, found.rect)
        .map(Some)
        .ok_or_else(|| format!("the crop {crop} is outside {file}"))
}

impl ScreenPictures {
    /// Makes the pictures again if anything they come from changed: the
    /// names in `uses` (written in screens' code) and those seen running,
    /// from `sheets`' pictures and crops. `in_project` says whether a file
    /// is among the assets. `fingerprint` changes with the code and the
    /// Library. Gives the names worked out as the game ran that can't be
    /// shown, with why, each once.
    pub(crate) fn refresh(
        &mut self,
        uses: &[script::screens::PictureUse],
        sheets: &Arc<SpriteSheets>,
        in_project: &dyn Fn(&str) -> bool,
        fingerprint: u64,
    ) -> Vec<String> {
        let from = (fingerprint, Arc::as_ptr(sheets) as usize, self.seen.len());
        if self.from == Some(from) {
            return Vec::new();
        }
        self.from = Some(from);
        let mut names: BTreeSet<String> = uses.iter().map(|u| u.name.clone()).collect();
        names.extend(self.seen.iter().cloned());
        // The files to read: those in the project.
        let wanted: BTreeSet<String> = names
            .iter()
            .map(|n| ui::pictures::split(n).0)
            .filter(|f| is_picture(f) && in_project(f))
            .map(str::to_string)
            .collect();
        if wanted != self.wanted {
            self.wanted = wanted;
            self.wanted_changed = true;
        }
        let mut pictures = ui::Pictures::new();
        let mut why_not = std::collections::HashMap::new();
        for name in &names {
            let added = pixels_of(name, sheets, in_project).and_then(|got| match got {
                Some((rgba, size)) => pictures
                    .add(name, size, &rgba)
                    .map_err(|why| format!("{name} can't be shown on a screen: {why}")),
                None => Ok(()),
            });
            if let Err(why) = added {
                why_not.insert(name.clone(), why);
            }
        }
        self.problems = uses
            .iter()
            .filter_map(|u| {
                why_not.get(&u.name).map(|why| CodeProblem {
                    file: u.file.clone(),
                    line: u.line,
                    column: u.column,
                    message: why.clone(),
                })
            })
            .collect();
        let written: HashSet<&str> = uses.iter().map(|u| u.name.as_str()).collect();
        let mut said = Vec::new();
        for name in &self.seen {
            if let Some(why) = why_not.get(name)
                && !written.contains(name.as_str())
                && self.told.insert(name.clone())
            {
                said.push(format!("A screen shows the picture \"{name}\", but {why}."));
            }
        }
        self.pictures = Some(Arc::new(pictures));
        said
    }
}

impl Workbench {
    /// Makes the screens' pictures again if what they come from changed,
    /// and puts them on the screens shown.
    pub(crate) fn refresh_screen_pictures(&mut self) {
        let uses = self
            .program
            .as_ref()
            .and_then(|p| p.screens.as_ref())
            .map_or(&[][..], |s| s.pictures.as_slice());
        if uses.is_empty() && self.screen_pictures.seen.is_empty() {
            if self.screen_pictures.pictures.is_some() || !self.screen_pictures.problems.is_empty()
            {
                self.screen_pictures = crate::screen_pictures::ScreenPictures {
                    wanted_changed: true,
                    ..Default::default()
                };
            }
            return;
        }
        let library: HashSet<&str> = self.library.iter().map(|i| i.path.as_str()).collect();
        let in_project = |f: &str| library.contains(f);
        let said =
            self.screen_pictures
                .refresh(uses, &self.viewport.sheets, &in_project, self.sources);
        for line in said {
            self.log(LogLevel::Warning, line);
        }
        let pictures = self.screen_pictures.pictures.clone();
        for screen in &mut self.game_screens.pictures {
            if screen.pictures != pictures {
                Arc::make_mut(screen).pictures = pictures.clone();
            }
        }
    }

    /// The picture files the game's screens show (paths inside the
    /// assets), when they changed since last taken: the app reads them.
    pub fn take_wanted_pictures(&mut self) -> Option<BTreeSet<String>> {
        let p = &mut self.screen_pictures;
        std::mem::take(&mut p.wanted_changed).then(|| p.wanted.clone())
    }

    /// Pictures named in the screens' code that can't be shown, where.
    pub fn picture_problems(&self) -> &[CodeProblem] {
        &self.screen_pictures.problems
    }
}

#[cfg(test)]
#[path = "screen_pictures_tests.rs"]
mod tests;
