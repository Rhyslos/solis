//! Pictures for the editor's tests: real frames of the editor, drawn on a
//! GPU or a software driver, and the design's own screens, to hold one
//! against the other.
//!
//! There are three kinds of check, each honest about what it proves:
//! measured layout against the specification (no pictures at all: the
//! whole-window tests read rectangles), sampled colours against the
//! design's pictures in `docs/design/g-layout/`, and Solis's own approved
//! pictures in `crates/editor/tests/pictures/`, which catch accidents.
//!
//! Drawing a frame needs a graphics driver. Without one these tests skip
//! with a note, unless `REQUIRE_GPU` is set, in which case a missing
//! driver is a failure: the same rule as the renderer's tests. Mesa's
//! software driver does (`WGPU_BACKEND=gl` under `xvfb-run`).

use crate::library::{LibraryItem, LibraryKind, ProjectInfo};
use crate::theme::{self, InterfaceLook};
use crate::viewport::NoRenderer;
use crate::workbench::Workbench;
use egui::Color32;
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A picture: sRGB, four bytes a pixel, rows from the top.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Picture {
    pub(crate) size: [u32; 2],
    pub(crate) rgba: Vec<u8>,
}

impl Picture {
    /// A picture of one colour.
    pub(crate) fn filled(size: [u32; 2], colour: Color32) -> Self {
        let pixel = [colour.r(), colour.g(), colour.b(), 255];
        Self {
            size,
            rgba: pixel.repeat((size[0] * size[1]) as usize),
        }
    }

    /// The colour of one pixel (the nearest edge's, outside the picture).
    pub(crate) fn pixel(&self, x: u32, y: u32) -> Color32 {
        let x = x.min(self.size[0].saturating_sub(1));
        let y = y.min(self.size[1].saturating_sub(1));
        let at = ((y * self.size[0] + x) * 4) as usize;
        Color32::from_rgb(self.rgba[at], self.rgba[at + 1], self.rgba[at + 2])
    }

    fn put(&mut self, x: u32, y: u32, colour: Color32) {
        if x < self.size[0] && y < self.size[1] {
            let at = ((y * self.size[0] + x) * 4) as usize;
            self.rgba[at..at + 4].copy_from_slice(&[colour.r(), colour.g(), colour.b(), 255]);
        }
    }

    /// The colour at a point of a screen drawn `scale` pixels to the point.
    pub(crate) fn at(&self, point: [f32; 2], scale: f32) -> Color32 {
        self.pixel((point[0] * scale) as u32, (point[1] * scale) as u32)
    }

    /// Whether every pixel within `reach` pixels of that point is one
    /// colour: a sample is only taken from a flat surface, away from text
    /// and edges, where a browser and egui can't differ.
    pub(crate) fn is_flat_at(&self, point: [f32; 2], scale: f32, reach: u32) -> bool {
        let (x, y) = ((point[0] * scale) as u32, (point[1] * scale) as u32);
        let colour = self.pixel(x, y);
        (y.saturating_sub(reach)..=y + reach)
            .all(|py| (x.saturating_sub(reach)..=x + reach).all(|px| self.pixel(px, py) == colour))
    }

    /// Reads a PNG.
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let art = assets::tile_art::read_tile_art(&bytes, false)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let rgba = art.frames.into_iter().next().ok_or("an empty picture")?;
        Ok(Self {
            size: art.size,
            rgba,
        })
    }

    /// Writes a PNG, making its folder if need be.
    pub(crate) fn write(&self, path: &Path) {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).unwrap();
        }
        let png = assets::tile_art::png_of(self.size, &self.rgba).unwrap();
        std::fs::write(path, png).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }

    /// The picture at half its size, each pixel the mean of four: the
    /// design's screens are saved at twice the size they were drawn.
    pub(crate) fn halved(&self) -> Self {
        let size = [self.size[0] / 2, self.size[1] / 2];
        let mut half = Self::filled(size, Color32::BLACK);
        for y in 0..size[1] {
            for x in 0..size[0] {
                let four = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .map(|(dx, dy)| self.pixel(2 * x + dx, 2 * y + dy));
                let mean = |channel: fn(&Color32) -> u8| {
                    (four.iter().map(|c| u32::from(channel(c))).sum::<u32>() / 4) as u8
                };
                half.put(
                    x,
                    y,
                    Color32::from_rgb(mean(Color32::r), mean(Color32::g), mean(Color32::b)),
                );
            }
        }
        half
    }

    /// Two pictures side by side with a gap, for looking at.
    pub(crate) fn beside(&self, other: &Self, gap: u32, between: Color32) -> Self {
        let size = [
            self.size[0] + gap + other.size[0],
            self.size[1].max(other.size[1]),
        ];
        let mut both = Self::filled(size, between);
        for (picture, left) in [(self, 0), (other, self.size[0] + gap)] {
            for y in 0..picture.size[1] {
                for x in 0..picture.size[0] {
                    both.put(left + x, y, picture.pixel(x, y));
                }
            }
        }
        both
    }
}

/// The repository's top folder.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// One of the design's screens, by its file's name.
pub(crate) fn design(file: &str) -> Picture {
    let path = repository()
        .join("docs")
        .join("design")
        .join("g-layout")
        .join(file);
    Picture::read(&path).unwrap_or_else(|e| panic!("{e}"))
}

/// A text file, with its line ends made plain (a Windows checkout ends
/// lines with a carriage return too).
fn text_of(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// What kind of file the Library shows a file of the sample project as.
/// The files Solis keeps beside assets (their IDs) aren't shown.
fn kind_of(file: &Path) -> Option<LibraryKind> {
    match file.extension()?.to_str()? {
        "glb" => Some(LibraryKind::Model),
        "solarea" => Some(LibraryKind::Area),
        "png" => Some(LibraryKind::Texture),
        "wav" => Some(LibraryKind::Sound),
        "lst" => Some(LibraryKind::Code),
        "md" => Some(LibraryKind::Text),
        _ => None,
    }
}

/// The folders and files under `folder`, as the Library lists them: each
/// path from the assets folder with `/` between names, in name order.
fn listed(assets: &Path, folder: &Path, folders: &mut Vec<String>, files: &mut Vec<LibraryItem>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder)
        .unwrap_or_else(|e| panic!("{}: {e}", folder.display()))
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for entry in entries {
        let inside: Vec<String> = entry
            .strip_prefix(assets)
            .unwrap()
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        let path = inside.join("/");
        if entry.is_dir() {
            folders.push(path);
            listed(assets, &entry, folders, files);
        } else if let Some(kind) = kind_of(&entry) {
            files.push(LibraryItem {
                id: None,
                kind,
                name: crate::sprite_sheets::name_of(&path),
                path,
                thumbnail: None,
            });
        }
    }
}

/// The editor with the small sample project of `tests/fixtures/ui-sample/`
/// open, as the app would hand it over: its area, its files in the
/// Library, and its lamp selected, so that the Area panel, Properties and
/// the Library all have real content to show. Named after the design's
/// own sample (a harbour at night), so the two can be laid side by side.
pub(crate) fn sample_project() -> Workbench {
    let root = repository()
        .join("tests")
        .join("fixtures")
        .join("ui-sample");
    let assets = root.join("assets");
    let name = text_of(&root.join("project.solis"))
        .lines()
        .find_map(|line| line.strip_prefix("name = "))
        .map(|name| name.trim().trim_matches('"').to_string())
        .expect("the sample project has a name");
    let mut workbench = Workbench::new(crate::test_support::info(), None);
    workbench.set_project(Some(ProjectInfo {
        name,
        root: root.clone(),
        assets: assets.clone(),
    }));
    let (mut folders, mut files) = (Vec::new(), Vec::new());
    listed(&assets, &assets, &mut folders, &mut files);
    workbench.set_folders(folders);
    workbench.set_library(files);
    let file = assets.join("docks_night.solarea");
    let area = area::text::read(&text_of(&file), &workbench.document().registry)
        .unwrap_or_else(|e| panic!("the sample project's area doesn't read: {e:?}"));
    workbench.open_area(area, file);
    let lamp = workbench
        .document()
        .area()
        .iter()
        .find(|(_, entity)| entity.name == "Dock_Lamp")
        .map(|(id, _)| id)
        .expect("the sample area has its lamp");
    workbench.open.document.selection.select_only(lamp);
    workbench
}

/// How many pixels the design's screens have to a point.
pub(crate) const DESIGN_SCALE: f32 = 2.0;

/// Whether a frame can be drawn here. Found out once; without a driver
/// the first test to ask says so, or fails when `REQUIRE_GPU` is set.
pub(crate) fn can_draw() -> bool {
    static FOUND: OnceLock<bool> = OnceLock::new();
    let found = *FOUND.get_or_init(|| {
        // The renderer panics when no driver answers.
        std::panic::catch_unwind(|| drop(WgpuTestRenderer::new())).is_ok()
    });
    if !found {
        assert!(
            std::env::var_os("REQUIRE_GPU").is_none(),
            "REQUIRE_GPU is set but no graphics driver could draw the editor's pictures"
        );
        eprintln!("No graphics driver available; skipping a picture test.");
    }
    found
}

/// A `Ui` over the whole test window: the test window sets what it draws
/// in from its edges, and a picture is of the window edge to edge.
fn edge_to_edge(ui: &mut egui::Ui) -> egui::Ui {
    let whole = ui.ctx().content_rect();
    let mut root = ui.new_child(egui::UiBuilder::new().max_rect(whole));
    root.set_clip_rect(whole);
    root
}

/// A test window that draws `ui` for real, `scale` pixels to the point;
/// `None` (after a note) where nothing can draw.
pub(crate) fn window<'a>(
    size: [f32; 2],
    scale: f32,
    mut ui: impl FnMut(&mut egui::Ui) + 'a,
) -> Option<Harness<'a>> {
    can_draw().then(|| {
        Harness::builder()
            .with_size(egui::vec2(size[0], size[1]))
            .with_pixels_per_point(scale)
            .renderer(WgpuTestRenderer::new())
            .build_ui(move |inset| ui(&mut edge_to_edge(inset)))
    })
}

/// The whole editor in a test window, in `look`, with no 3D view (the
/// viewport says so on its backdrop); `None` where nothing can draw.
pub(crate) fn editor_window(
    workbench: Workbench,
    look: InterfaceLook,
    size: [f32; 2],
) -> Option<Harness<'static, Workbench>> {
    if !can_draw() {
        return None;
    }
    let mut renderer = NoRenderer {
        reason: "pictures are taken without it".into(),
    };
    let mut themed = false;
    // The editor holds the accent it draws with, as the app hands it over.
    let workbench = workbench.with_interface_look(look);
    let harness = Harness::builder()
        .with_size(egui::vec2(size[0], size[1]))
        .renderer(WgpuTestRenderer::new())
        .build_ui_state(
            move |inset, workbench: &mut Workbench| {
                if !themed {
                    // The fonts are in force from the next frame (the app
                    // loads them before its first): nothing is drawn in
                    // this one.
                    theme::apply(inset.ctx(), look);
                    themed = true;
                    return;
                }
                let _ = workbench.show(&mut edge_to_edge(inset), &mut renderer);
            },
            workbench,
        );
    Some(harness)
}

/// The frame the window last drew.
pub(crate) fn drawn<S>(harness: &mut Harness<'_, S>) -> Picture {
    let image = harness
        .render()
        .unwrap_or_else(|e| panic!("couldn't draw: {e}"));
    Picture {
        size: [image.width(), image.height()],
        rgba: image.into_raw(),
    }
}

/// How far a channel may be from the approved picture's and still count
/// as the same: drivers round the soft edges of letters differently.
const SAME_WITHIN: u8 = 12;
/// The share of pixels that may differ by more than that.
const MAY_DIFFER: f32 = 0.002;

/// How many pixels of `a` differ from `b`'s by more than a driver's
/// rounding, and a picture of where (the differences in red over `a`,
/// dimmed). `None` when the sizes differ.
pub(crate) fn difference(a: &Picture, b: &Picture) -> Option<(usize, Picture)> {
    if a.size != b.size {
        return None;
    }
    let mut marked = a.clone();
    let mut count = 0;
    for y in 0..a.size[1] {
        for x in 0..a.size[0] {
            let (p, q) = (a.pixel(x, y), b.pixel(x, y));
            let apart = [(p.r(), q.r()), (p.g(), q.g()), (p.b(), q.b())]
                .iter()
                .any(|(m, n)| m.abs_diff(*n) > SAME_WITHIN);
            if apart {
                count += 1;
                marked.put(x, y, Color32::RED);
            } else {
                marked.put(x, y, Color32::from_rgb(p.r() / 3, p.g() / 3, p.b() / 3));
            }
        }
    }
    Some((count, marked))
}

/// Where Solis's own approved pictures are kept.
fn approved_folder() -> PathBuf {
    repository()
        .join("crates")
        .join("editor")
        .join("tests")
        .join("pictures")
}

/// Holds `picture` against the approved one of that name. A picture that
/// differs fails, leaving what was drawn and where it differs in the
/// system's temporary folder. With `SOLIS_UPDATE_PICTURES=1` the picture
/// becomes the approved one instead: do that only after looking at it
/// beside the design.
pub(crate) fn matches_approved(name: &str, picture: &Picture) {
    // Letters' soft edges come out a little differently from each
    // system's drivers, and the pictures were approved on Linux's: other
    // systems check the sampled colours, which are exact everywhere, and
    // leave whole pictures to Linux.
    if !cfg!(target_os = "linux") {
        eprintln!("Approved pictures are compared on Linux only; {name} was not compared.");
        return;
    }
    let approved = approved_folder().join(format!("{name}.png"));
    if std::env::var_os("SOLIS_UPDATE_PICTURES").is_some() {
        picture.write(&approved);
        eprintln!("Approved picture written: {}", approved.display());
        return;
    }
    let kept = Picture::read(&approved).unwrap_or_else(|e| {
        panic!("{e}\nNo approved picture: look at the frame, then run with SOLIS_UPDATE_PICTURES=1")
    });
    let problem = match difference(picture, &kept) {
        None => Some((
            format!(
                "it is {:?} pixels, the approved one {:?}",
                picture.size, kept.size
            ),
            None,
        )),
        Some((count, marked)) => {
            let all = (picture.size[0] * picture.size[1]) as f32;
            (count as f32 > all * MAY_DIFFER)
                .then(|| (format!("{count} pixels differ"), Some(marked)))
        }
    };
    let Some((why, marked)) = problem else {
        return;
    };
    let out = std::env::temp_dir().join("solis-pictures");
    let drawn = out.join(format!("{name}.drawn.png"));
    picture.write(&drawn);
    let mut where_to_look = format!("drawn: {}", drawn.display());
    if let Some(marked) = marked {
        let differs = out.join(format!("{name}.differs.png"));
        marked.write(&differs);
        where_to_look += &format!("\nwhere it differs: {}", differs.display());
    }
    panic!(
        "{name} doesn't match its approved picture: {why}\n{where_to_look}\napproved: {}\n\
         If the change is meant, look at it beside the design and run with \
         SOLIS_UPDATE_PICTURES=1.",
        approved.display()
    );
}
