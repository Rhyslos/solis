use super::*;

/// Places that leave links and anchors as they are.
struct Same;

impl Places for Same {
    fn href(&mut self, url: &str) -> String {
        url.to_string()
    }
    fn anchor(&mut self, id: &str) -> String {
        format!("page--{id}")
    }
}

fn html(text: &str) -> String {
    render(text, &mut Same)
}

#[test]
fn bold_italics_and_code() {
    assert_eq!(
        html("**Bold** and *em* and _em_ and `x < y`"),
        "<strong>Bold</strong> and <em>em</em> and <em>em</em> and <code>x &lt; y</code>"
    );
}

#[test]
fn underscores_inside_words_stay() {
    assert_eq!(html("snake_case_name"), "snake_case_name");
}

#[test]
fn keys_in_code_are_keys() {
    assert_eq!(
        html("`Ctrl + K`"),
        "<span class=\"keys\"><kbd>Ctrl</kbd><span class=\"plus\">+</span><kbd>K</kbd></span>"
    );
    assert_eq!(html("`Esc`"), "<span class=\"keys\"><kbd>Esc</kbd></span>");
    assert_eq!(html("`F2`"), "<span class=\"keys\"><kbd>F2</kbd></span>");
    assert_eq!(
        html("`Shift + click`"),
        "<span class=\"keys\"><kbd>Shift</kbd><span class=\"plus\">+</span>click</span>"
    );
    assert_eq!(html("<kbd>Q</kbd>"), "<kbd>Q</kbd>");
}

#[test]
fn code_that_isnt_keys_stays_code() {
    for code in [
        "`.`",
        "`#`",
        "`x`",
        "`Start`",
        "`click`",
        "`Ctrl + nothing`",
        "`dt`",
    ] {
        assert!(html(code).starts_with("<code>"), "{code} is code");
    }
}

#[test]
fn links_and_web_links() {
    assert_eq!(
        html("[Physics](physics.md#top)"),
        "<a href=\"physics.md#top\">Physics</a>"
    );
    assert_eq!(
        html("[site](https://example.org)"),
        "<a href=\"https://example.org\" target=\"_blank\" rel=\"noopener\">site<span class=\"out\" aria-hidden=\"true\">↗</span></a>"
    );
    assert_eq!(
        html("[a `[b]` c](x.md)"),
        "<a href=\"x.md\">a <code>[b]</code> c</a>"
    );
}

#[test]
fn a_named_anchor_is_kept_and_other_html_is_words() {
    assert_eq!(
        html("<a id=\"pill\"></a>Pill"),
        "<a id=\"page--pill\"></a>Pill"
    );
    assert_eq!(html("<script>x</script>"), "&lt;script&gt;x&lt;/script&gt;");
    assert_eq!(html("a <b> c"), "a &lt;b&gt; c");
}

#[test]
fn escapes_and_unclosed_marks_are_words() {
    assert_eq!(html("\\*not em\\*"), "*not em*");
    assert_eq!(html("a ` b"), "a ` b");
    assert_eq!(html("**open"), "**open");
    assert_eq!(html("[no link]"), "[no link]");
}

#[test]
fn deep_marks_stop_without_running_out_of_stack() {
    let deep = "*".repeat(5_000) + "x" + &"*".repeat(5_000);
    let _ = html(&deep);
    let brackets = "[".repeat(5_000) + "x" + &"](y)".repeat(5_000);
    let _ = html(&brackets);
}
