use super::*;
use std::path::PathBuf;

fn small_manual(pages: &[(&str, &str)], contents: &str) -> Manual {
    Manual {
        contents: contents.to_string(),
        pages: pages
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect(),
        name: "Example".into(),
        repo: "owner/repo".into(),
        logo: b"<svg/>".to_vec(),
        fonts: Vec::new(),
        root: PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")),
    }
}

const CONTENTS: &str = "# Manual\n\nHello.\n\n## Start\n\n- [One](one.md)\n- [Two](two.md)\n";

#[test]
fn a_small_manual_builds_into_one_page() {
    let manual = small_manual(
        &[
            (
                "one",
                "# One\n\n> The first.\n\n## Part\n\nSee [two](two.md#more).\n",
            ),
            ("two", "# Two\n\n## More\n\n1. Press `Ctrl + K`.\n"),
        ],
        CONTENTS,
    );
    let html = build(&manual).unwrap();
    assert!(html.contains("<title>Example user guide</title>"));
    assert!(html.contains("<p class=\"lead\">The first.</p>"));
    assert!(html.contains("<section id=\"one--part\">"));
    assert!(html.contains("<a href=\"#two--more\">two</a>"));
    assert!(html.contains("<li class=\"group\">Start</li>"));
    assert!(html.contains("<span class=\"num\">2</span>Two"));
    assert!(!html.contains("{{"), "every slot is filled");
    assert!(
        !html.contains("http://") || !html.contains("<link"),
        "nothing from the network"
    );
}

#[test]
fn building_twice_gives_the_same_page() {
    let manual = small_manual(&[("one", "# One\n"), ("two", "# Two\n")], CONTENTS);
    assert_eq!(build(&manual), build(&manual));
}

#[test]
fn a_page_missing_from_the_contents_stops_the_build() {
    let manual = small_manual(
        &[
            ("one", "# One\n"),
            ("two", "# Two\n"),
            ("three", "# Three\n"),
        ],
        CONTENTS,
    );
    let problems = build(&manual).unwrap_err();
    assert_eq!(
        problems,
        ["three.md isn't in SUMMARY.md: add it to a group there"]
    );
    assert_eq!(unlisted(&manual).unwrap(), ["three"]);
}

#[test]
fn a_broken_link_or_a_missing_title_stops_the_build() {
    let manual = small_manual(
        &[("one", "# One\n[x](nowhere.md)"), ("two", "No title")],
        CONTENTS,
    );
    let problems = build(&manual).unwrap_err();
    assert_eq!(problems.len(), 2, "{problems:?}");
}

#[test]
fn the_name_comes_from_branding() {
    let source = "/// The name.\npub const APP_NAME: &str = \"Example\";\npub const UPDATE_REPO: &str = \"a/b\";\n";
    assert_eq!(constant(source, "APP_NAME").unwrap(), "Example");
    assert_eq!(constant(source, "UPDATE_REPO").unwrap(), "a/b");
    assert!(constant(source, "APP_ID").is_err());
    assert!(constant("pub const APP_NAME: &str = \"<b>\";", "APP_NAME").is_err());
}

#[test]
fn base64_matches_the_standards_examples() {
    for (plain, coded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(plain.as_bytes()), coded);
    }
}

#[test]
fn filling_in_never_searches_a_value() {
    assert_eq!(
        fill_in("{{a}} {{b}} {{c", &[("a", "{{b}}"), ("b", "B")]),
        "{{b}} B {{c"
    );
}

/// A tiny random number source, the same every run.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Thousands of damaged pages: none may crash the build (rule 9).
#[test]
fn damaged_pages_never_crash_the_build() {
    let seeds: Vec<String> = [
        "keyboard-shortcuts",
        "game-code",
        "concepts",
        "first-platformer",
    ]
    .iter()
    .map(|page| {
        std::fs::read_to_string(format!(
            "{}/../../docs/manual/{page}.md",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    })
    .collect();
    let marks = [
        "*", "_", "`", "[", "]", "(", ")", "<", ">", "|", "#", "\n", "-", "1.", "> ", "```", "\\",
        "é", "<a id=\"", "<kbd>", "   ",
    ];
    let mut dice = Dice(0x5eed_1234_abcd_ef01);
    for round in 0..3_000 {
        let seed: Vec<char> = seeds[round % seeds.len()].chars().collect();
        // A window of the page: the damage, not the length, is the test.
        let from = (dice.next() as usize) % seed.len();
        let mut chars: Vec<char> = seed[from..(from + 2_000).min(seed.len())].to_vec();
        for _ in 0..(1 + dice.next() % 40) {
            let at = (dice.next() as usize) % (chars.len() + 1);
            match dice.next() % 3 {
                0 => {
                    let mark = marks[(dice.next() as usize) % marks.len()];
                    for (k, c) in mark.chars().enumerate() {
                        chars.insert((at + k).min(chars.len()), c);
                    }
                }
                1 if at < chars.len() => {
                    chars.remove(at);
                }
                _ => {
                    let cut = (at + (dice.next() as usize) % 200).min(chars.len());
                    chars.drain(at..cut);
                }
            }
        }
        let page: String = chars.into_iter().collect();
        let manual = small_manual(&[("one", &page), ("two", "# Two")], CONTENTS);
        let _ = build(&manual);
        let _ = contents::read(&page);
    }
}

/// The dark theme's colours are the editor's tokens: each line of it that
/// names a token holds that token's value from `tokens.rs`, and the
/// theme chosen by hand is the same as the one the system chooses.
#[test]
fn the_dark_theme_is_the_editors_tokens() {
    let tokens = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/editor/src/tokens.rs"
    ))
    .unwrap();
    let style = include_str!("../assets/guide.css");
    let dark_start = style.find("@media (prefers-color-scheme: dark)").unwrap();
    let chosen_start = style.find(":root[data-theme=\"dark\"]").unwrap();
    let block = |from: usize| -> Vec<(String, String)> {
        style[from..]
            .lines()
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with("color-scheme"))
            .filter_map(|l| {
                let (name, rest) = l.trim().split_once(':')?;
                if !name.starts_with("--") {
                    return None;
                }
                let value = rest.split(';').next()?.trim().to_string();
                Some((name.trim().to_string(), value))
            })
            .collect()
    };
    let system = block(dark_start);
    assert_eq!(system, block(chosen_start), "the two dark blocks differ");
    let mut named = 0;
    for line in style[dark_start..chosen_start].lines() {
        let Some((_, comment)) = line.split_once("/* ") else {
            continue;
        };
        let token = comment.split([',', ' ']).next().unwrap();
        let value = line
            .split(':')
            .nth(1)
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .trim();
        let expected = if token == "Blue" {
            let at = tokens.find("Self::Blue => hex(0x").unwrap() + 20;
            format!("#{}", &tokens[at..at + 6])
        } else if token.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
            let at = tokens.find(&format!("    {token} = 0x")).unwrap() + token.len() + 9;
            format!("#{}", &tokens[at..at + 6])
        } else {
            let at = tokens.find(&format!("    {token}: grey(0x")).unwrap() + token.len() + 13;
            let grey = &tokens[at..at + 2];
            format!("#{grey}{grey}{grey}")
        };
        assert_eq!(value, expected, "{token}");
        named += 1;
    }
    assert!(named >= 14, "only {named} colours name their token");
}
