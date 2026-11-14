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
