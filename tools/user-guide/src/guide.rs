//! The whole guide: the welcome, then each page of the manual as a
//! numbered chapter (its sections inside it), the contents down the side
//! and the page around them (`assets/page.html`, with the style and the
//! script written into it, so it is one file that works offline).

use crate::blocks::{self, Block};
use crate::contents::Contents;
use crate::html::{self, Writer, anchor_link, plain_words};
use crate::inline::escape;
use crate::links::{PageLinks, Reach};
use crate::{Font, Manual, pictures};
use std::collections::{BTreeMap, BTreeSet};

const PAGE: &str = include_str!("../assets/page.html");
const STYLE: &str = include_str!("../assets/guide.css");
const SCRIPT: &str = include_str!("../assets/guide.js");

/// One chapter, written.
struct Chapter {
    page: String,
    title: String,
    group: usize,
    /// Its sections: id and words.
    sections: Vec<(String, String)>,
    body: String,
}

/// The guide's HTML, or what stops it being built.
pub(crate) fn write(manual: &Manual, contents: &Contents) -> Result<String, Vec<String>> {
    let parsed: BTreeMap<&str, Vec<Block>> = contents
        .pages()
        .filter_map(|page| Some((page, blocks::parse(manual.pages.get(page)?))))
        .collect();
    let anchors: BTreeMap<String, Vec<String>> = parsed
        .iter()
        .map(|(page, blocks)| (page.to_string(), html::anchors(blocks)))
        .collect();
    let reach = Reach {
        anchors: &anchors,
        root: &manual.root,
        repo: &manual.repo,
        pictures: &manual.pictures,
    };
    let mut problems: Vec<String> = manual
        .pictures
        .iter()
        .filter_map(|(name, data)| pictures::problem(name, data))
        .collect();
    let mut shown = BTreeSet::new();
    let mut chapters = Vec::new();
    for (group, g) in contents.groups.iter().enumerate() {
        for page in &g.pages {
            let Some(blocks) = parsed.get(page.as_str()) else {
                problems.push(format!(
                    "SUMMARY.md lists {page}.md, which isn't in docs/manual"
                ));
                continue;
            };
            match chapter(page, group, blocks, &reach) {
                Ok((chapter, its_pictures)) => {
                    chapters.push(chapter);
                    shown.extend(its_pictures);
                }
                Err(found) => problems.extend(found),
            }
        }
    }
    problems.extend(
        manual
            .pictures
            .keys()
            .filter(|name| !shown.contains(*name))
            .map(|name| {
                format!(
                    "{}/{name} isn't shown on any page: show it, or take it out of the \
                     picture script's list and delete it",
                    pictures::FOLDER
                )
            }),
    );
    let welcome = welcome(manual, contents, &reach, &mut problems);
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(fill(manual, contents, &welcome, &chapters))
}

/// The welcome: the name and logo, the contents page's own words, and
/// how to find things in the guide.
fn welcome(
    manual: &Manual,
    contents: &Contents,
    reach: &Reach<'_>,
    problems: &mut Vec<String>,
) -> String {
    let mut links = PageLinks::new(reach, "SUMMARY");
    let mut writer = Writer::new(&mut links);
    writer.push(&format!(
        "<h1><span class=\"logo\" aria-hidden=\"true\"></span>{} user guide</h1>\n",
        escape(&manual.name)
    ));
    writer.blocks(&contents.welcome);
    writer.push(concat!(
        "<p>Search with the box at the top (press <span class=\"keys\"><kbd>/</kbd></span> or ",
        "<span class=\"keys\"><kbd>Ctrl</kbd><span class=\"plus\">+</span><kbd>K</kbd></span>), ",
        "or pick a chapter on the left. Every heading has a link of its own: point at it and ",
        "click the <strong>#</strong> beside it to copy where it is.</p>\n",
    ));
    let body = writer.finish();
    problems.extend(links.problems);
    body
}

/// A page as a chapter, with the pictures it shows.
fn chapter(
    page: &str,
    group: usize,
    blocks: &[Block],
    reach: &Reach<'_>,
) -> Result<(Chapter, BTreeSet<String>), Vec<String>> {
    let Some((Block::Heading(1, title), rest)) = blocks.split_first() else {
        return Err(vec![format!(
            "{page}.md must start with its title, as `# Title`"
        )]);
    };
    let mut links = PageLinks::new(reach, page);
    let mut writer = Writer::new(&mut links);
    // The title's own slug is taken, as on GitHub, so headings after it
    // are numbered the same way.
    writer.heading_id(title);
    let words = writer.words(title);
    writer.push(&format!("<h2>{words}{}</h2>\n", anchor_link(page)));
    let mut rest = rest;
    if let Some((Block::Quote(lead), after)) = rest.split_first() {
        for block in lead {
            if let Block::Paragraph(words) = block {
                let html = writer.words(words);
                writer.push(&format!("<p class=\"lead\">{html}</p>\n"));
            }
        }
        rest = after;
    }
    let mut sections = Vec::new();
    let mut open = false;
    // Each section's blocks are written together, so an example's three
    // blocks are seen as one.
    let mut parts: Vec<(Option<&str>, &[Block])> = Vec::new();
    let mut start = 0;
    let mut heading = None;
    for (at, block) in rest.iter().enumerate() {
        if let Block::Heading(2, words) = block {
            parts.push((heading, &rest[start..at]));
            heading = Some(words.as_str());
            start = at + 1;
        }
    }
    parts.push((heading, &rest[start..]));
    for (heading, blocks) in parts {
        if let Some(words) = heading {
            if open {
                writer.push("</section>\n");
            }
            let id = writer.heading_id(words);
            let html = writer.words(words);
            writer.push(&format!(
                "<section id=\"{id}\">\n<h3>{html}{}</h3>\n",
                anchor_link(&id)
            ));
            sections.push((id, plain_words(words)));
            open = true;
        }
        writer.blocks(blocks);
    }
    if open {
        writer.push("</section>\n");
    }
    let body = writer.finish();
    if !links.problems.is_empty() {
        return Err(links.problems);
    }
    let chapter = Chapter {
        page: page.to_string(),
        title: plain_words(title),
        group,
        sections,
        body,
    };
    Ok((chapter, links.pictures))
}

/// The contents down the side: the welcome, then each group's name and
/// its numbered chapters, each with its sections (shown for the chapter
/// being read).
fn contents_list(contents: &Contents, chapters: &[Chapter]) -> String {
    let mut out = String::from(
        "<ol>\n<li><a href=\"#welcome\"><span class=\"num\"></span>Welcome</a></li>\n",
    );
    let mut group = None;
    for (n, chapter) in chapters.iter().enumerate() {
        if group != Some(chapter.group) {
            group = Some(chapter.group);
            if let Some(g) = contents.groups.get(chapter.group) {
                out.push_str(&format!("<li class=\"group\">{}</li>\n", escape(&g.name)));
            }
        }
        out.push_str(&format!(
            "<li><a href=\"#{}\"><span class=\"num\">{}</span>{}</a>",
            chapter.page,
            n + 1,
            escape(&chapter.title)
        ));
        if !chapter.sections.is_empty() {
            out.push_str("<ol>");
            for (id, words) in &chapter.sections {
                out.push_str(&format!("<li><a href=\"#{id}\">{}</a></li>", escape(words)));
            }
            out.push_str("</ol>");
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
    out
}

/// The way on from a chapter: the one before and the one after.
fn pager(before: Option<(usize, &Chapter)>, after: Option<(usize, &Chapter)>) -> String {
    let card = |(n, chapter): (usize, &Chapter), class: &str, dir: &str| {
        let number = if n == 0 {
            String::new()
        } else {
            format!(" · Chapter {n}")
        };
        format!(
            "<a class=\"{class}\" href=\"#{}\"><span class=\"dir\">{dir}{number}</span><span class=\"name\">{}</span></a>",
            chapter.page,
            escape(&chapter.title)
        )
    };
    let mut out = String::from("<nav class=\"pager\" aria-label=\"Chapters before and after\">");
    if let Some(before) = before {
        out.push_str(&card(before, "prev", "← Previous"));
    }
    if let Some(after) = after {
        out.push_str(&card(after, "next", "Next →"));
    }
    out.push_str("</nav>\n");
    out
}

fn fill(manual: &Manual, contents: &Contents, welcome: &str, chapters: &[Chapter]) -> String {
    let welcome_card = Chapter {
        page: "welcome".into(),
        title: "Welcome".into(),
        group: 0,
        sections: Vec::new(),
        body: String::new(),
    };
    let mut body = format!("<section class=\"chapter\" id=\"welcome\">\n{welcome}");
    body.push_str(&pager(None, chapters.first().map(|c| (1, c))));
    body.push_str("</section>\n");
    for (n, chapter) in chapters.iter().enumerate() {
        let before = if n == 0 {
            (0, &welcome_card)
        } else {
            (n, &chapters[n - 1])
        };
        body.push_str(&format!(
            "<section class=\"chapter\" id=\"{}\" data-num=\"{}\">\n{}",
            chapter.page,
            n + 1,
            chapter.body
        ));
        body.push_str(&pager(
            Some(before),
            chapters.get(n + 1).map(|c| (n + 2, c)),
        ));
        body.push_str("</section>\n");
    }
    crate::fill_in(
        PAGE,
        &[
            ("style", &style(manual)),
            ("script", SCRIPT),
            ("contents", &contents_list(contents, chapters)),
            ("chapters", &body),
            ("name", &escape(&manual.name)),
        ],
    )
}

/// The style, with the logo and the fonts written into it.
fn style(manual: &Manual) -> String {
    let logo = format!("data:image/svg+xml;base64,{}", crate::base64(&manual.logo));
    let fonts: String = manual.fonts.iter().map(font_face).collect();
    format!("{fonts}:root {{ --logo: url(\"{logo}\"); }}\n{STYLE}")
}

fn font_face(font: &Font) -> String {
    format!(
        "@font-face {{ font-family: \"{}\"; font-weight: {}; font-style: normal; font-display: swap; \
         src: url(\"data:font/ttf;base64,{}\") format(\"truetype\"); }}\n",
        font.family,
        font.weight,
        crate::base64(&font.data)
    )
}
