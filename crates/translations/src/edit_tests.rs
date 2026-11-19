use super::*;
use proptest::prelude::*;

const FILE: &str = "# Words.\nShop-Welcome = Welcome!\n    .title = Hi\n\nShop-Bye = Bye\n";

#[test]
fn a_line_is_changed_where_it_is_keeping_the_rest() {
    let out = set_line(FILE, "Shop.Welcome", "Velkommen!").unwrap();
    assert_eq!(
        out,
        "# Words.\nShop-Welcome = Velkommen!\n    .title = Hi\n\nShop-Bye = Bye\n"
    );
}

#[test]
fn a_new_line_goes_at_the_end_and_several_lines_are_indented() {
    let out = set_line(
        FILE,
        "Shop.Apples",
        "{ $count ->\n[one] One apple\n*[other] { $count } apples\n}",
    )
    .unwrap();
    assert!(out.ends_with("Shop-Apples = { $count ->\n    [one] One apple\n    *[other] { $count } apples\n    }\n"), "{out}");
    let read = crate::ftl::read("x.ftl", &out);
    assert!(read.problems.is_empty(), "{:?}", read.problems);
    assert_eq!(read.lines.len(), 3);
}

#[test]
fn empty_text_takes_the_line_out() {
    let out = set_line(FILE, "Shop.Welcome", "  ").unwrap();
    assert_eq!(out, "# Words.\n\nShop-Bye = Bye\n");
    assert_eq!(remove_line(&out, "Shop.Nothing"), out);
}

#[test]
fn text_that_isnt_fluent_is_refused_in_plain_words() {
    let why = set_line(FILE, "Shop.Bye", "Bye { $name").unwrap_err();
    assert!(why.contains("isn't Fluent"), "{why}");
    let why = set_line(FILE, "Shop.Bye", "Bye\n.title = x").unwrap_err();
    assert!(why.contains("more than one line"), "{why}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Whatever is typed, the file stays readable: the line is written and
    /// read back as typed, or refused and the file left as it was.
    #[test]
    fn typed_text_round_trips_or_is_refused(text in "[ -~\\n]{1,60}") {
        if let Ok(out) = set_line(FILE, "Shop.Typed", &text) {
            {
                let read = crate::ftl::read("x.ftl", &out);
                prop_assert!(read.problems.is_empty(), "{:?}", read.problems);
                if !text.trim().is_empty() {
                    let line = read.lines.iter().find(|l| l.key == "Shop.Typed").unwrap();
                    let typed: Vec<&str> = text.trim_end().lines().map(str::trim).filter(|l| !l.is_empty()).collect();
                    let back: Vec<&str> = line.text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
                    prop_assert_eq!(typed, back);
                }
            }
        }
    }
}
