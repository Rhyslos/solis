use super::*;
use crate::blocks::parse;

struct Same;

impl Places for Same {
    fn href(&mut self, url: &str) -> String {
        url.to_string()
    }
    fn anchor(&mut self, id: &str) -> String {
        format!("p--{id}")
    }
}

fn html(text: &str) -> String {
    let mut places = Same;
    let mut writer = Writer::new(&mut places);
    writer.blocks(&parse(text));
    writer.finish()
}

#[test]
fn slugs_are_githubs() {
    assert_eq!(slug("How to use it"), "how-to-use-it");
    assert_eq!(
        slug("Tools (when the right mouse button isn't held)"),
        "tools-when-the-right-mouse-button-isnt-held"
    );
    assert_eq!(slug("Project settings › Input"), "project-settings--input");
    assert_eq!(slug("`Ctrl` and [a link](x.md)"), "ctrl-and-a-link");
}

#[test]
fn repeated_headings_are_numbered() {
    let blocks = parse("# T\n## Tips\n## Tips\n<a id=\"own\"></a>");
    assert_eq!(anchors(&blocks), ["t", "tips", "tips-1", "own"]);
}

#[test]
fn headings_go_one_level_down_with_a_link_to_themselves() {
    assert_eq!(
        html("### Fine print"),
        "<h4 id=\"p--fine-print\">Fine print<a class=\"anchor\" href=\"#p--fine-print\" aria-label=\"Link to this section\">#</a></h4>\n"
    );
}

#[test]
fn numbered_lists_are_steps() {
    assert!(html("1. a\n2. b").starts_with("<ol class=\"steps\">"));
    assert!(html("3. a").contains("counter-reset: step 2"));
    assert!(html("- a").starts_with("<ul>"));
}

#[test]
fn good_to_know_and_careful_are_boxes() {
    assert_eq!(
        html("> **Good to know:** keys work here."),
        "<div class=\"note\">\n<p>Keys work here.</p>\n</div>\n"
    );
    assert_eq!(
        html("> Note: one\n> two"),
        "<div class=\"note\">\n<p>One\ntwo</p>\n</div>\n"
    );
    assert!(html("> Careful. It deletes.").starts_with("<div class=\"warn\">"));
    assert!(html("> Noted elsewhere").starts_with("<blockquote>"));
}

#[test]
fn tables_of_keys_are_marked() {
    assert!(
        html("| Action | Shortcut |\n|---|---|\n| Undo | `Ctrl + Z` |")
            .contains("<table class=\"keys\">")
    );
    assert!(html("| A | B |\n|---|---|\n| 1 | 2 |").contains("<table>"));
}

#[test]
fn code_is_escaped() {
    assert_eq!(
        html("```\n<b>&\n```"),
        "<pre><code>&lt;b&gt;&amp;</code></pre>\n"
    );
}

/// Places that keep the problems said.
#[derive(Default)]
struct Telling(Vec<String>);

impl Places for Telling {
    fn href(&mut self, url: &str) -> String {
        url.to_string()
    }
    fn anchor(&mut self, id: &str) -> String {
        id.to_string()
    }
    fn picture(&mut self, url: &str) -> (String, Option<(u32, u32)>) {
        (format!("data:{url}"), Some((640, 400)))
    }
    fn problem(&mut self, why: String) {
        self.0.push(why);
    }
}

fn told(text: &str) -> (String, Vec<String>) {
    let mut places = Telling::default();
    let mut writer = Writer::new(&mut places);
    writer.blocks(&parse(text));
    let html = writer.finish();
    (html, places.0)
}

#[test]
fn three_languages_in_a_row_are_one_example_with_a_switch() {
    let (out, problems) = told(
        "```lust\nmoveTo(p);\n```\n\n```rust\nlet a = 1 < 2;\n```\n\n```lua\nsolis.entity.moveTo(self.me, p)\n```\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(out.matches("<div class=\"example\">").count(), 1);
    assert!(out.contains("<button type=\"button\" role=\"tab\" data-lang=\"lust\" aria-selected=\"true\">Lust</button>"));
    assert!(out.contains("data-lang=\"rust\" aria-selected=\"false\">Rust</button>"));
    assert!(out.contains(
        "<pre data-lang=\"rust\" data-name=\"Rust\"><code>let a = 1 &lt; 2;</code></pre>"
    ));
    assert!(
        out.find("data-lang=\"lust\" data-name").unwrap()
            < out.find("data-lang=\"lua\" data-name").unwrap()
    );
}

#[test]
fn a_game_code_block_on_its_own_is_a_problem_and_other_code_is_not() {
    let (_, problems) = told("```lust\nmoveTo(p);\n```\n\n```lua\nx()\n```\n");
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems[0].contains("all three languages"));
    let (_, problems) = told("```\ncomponent Mover {\n}\n```\n");
    assert!(problems[0].contains("reads as Lust"), "{problems:?}");
    let (out, problems) =
        told("```text\nsolis --check-project farm\n```\n\n```\narea version 3;\n```\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(out.contains("<pre data-lang=\"text\">"));
}

#[test]
fn a_picture_with_a_caption_is_a_figure() {
    let (out, problems) = told("![The editor](pictures/editor.png)\nThe editor, **open**.\n");
    assert!(problems.is_empty(), "{problems:?}");
    assert!(out.starts_with("<figure class=\"shot\">"), "{out}");
    assert!(out.contains("<img src=\"data:pictures/editor.png\" alt=\"The editor\" width=\"640\" height=\"400\" loading=\"lazy\">"));
    assert!(out.contains("<figcaption>The editor, <strong>open</strong>.</figcaption>"));
}

#[test]
fn a_picture_without_a_caption_is_a_problem_and_one_in_words_stays_in_them() {
    let (_, problems) = told("![The editor](pictures/editor.png)\n");
    assert!(problems[0].contains("no caption"), "{problems:?}");
    let (out, problems) = told("See ![a key](pictures/key.png) here.\n");
    assert!(problems.is_empty());
    assert!(out.starts_with("<p>See <img"), "{out}");
}
