use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn anchors() -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        (
            "physics".to_string(),
            vec!["physics".to_string(), "how-to-use-it".to_string()],
        ),
        (
            "concepts".to_string(),
            vec!["concepts".to_string(), "pill".to_string()],
        ),
    ])
}

fn href(url: &str) -> (String, Vec<String>) {
    let anchors = anchors();
    let root = root();
    let reach = Reach {
        anchors: &anchors,
        root: &root,
        repo: "owner/repo",
    };
    let mut links = PageLinks::new(&reach, "physics");
    let href = links.href(url);
    (href, links.problems)
}

#[test]
fn pages_and_their_headings_are_chapters_and_sections() {
    assert_eq!(href("concepts.md"), ("#concepts".into(), vec![]));
    assert_eq!(href("concepts.md#pill"), ("#concepts--pill".into(), vec![]));
    assert_eq!(
        href("#how-to-use-it"),
        ("#physics--how-to-use-it".into(), vec![])
    );
    assert_eq!(href("#physics"), ("#physics".into(), vec![]));
    assert_eq!(href("SUMMARY.md"), ("#welcome".into(), vec![]));
}

#[test]
fn files_outside_the_manual_are_on_github() {
    assert_eq!(
        href("../decisions/README.md"),
        (
            "https://github.com/owner/repo/blob/main/docs/decisions/README.md".into(),
            vec![]
        )
    );
    assert_eq!(
        href("../../tools"),
        (
            "https://github.com/owner/repo/tree/main/tools".into(),
            vec![]
        )
    );
    assert_eq!(href("https://example.org/a").1, Vec::<String>::new());
}

#[test]
fn broken_links_are_problems() {
    for url in [
        "missing.md",
        "concepts.md#nowhere",
        "#nowhere",
        "../decisions/9999-none.md",
        "../../../outside.md",
        "",
        "javascript:alert(1)",
        "/etc/passwd",
    ] {
        let (_, problems) = href(url);
        assert_eq!(problems.len(), 1, "{url}: {problems:?}");
        assert!(
            problems[0].starts_with("physics.md links to"),
            "{problems:?}"
        );
    }
}

#[test]
fn ids_join_the_page_and_the_anchor() {
    assert_eq!(id("tile-maps", "how-to-use-it"), "tile-maps--how-to-use-it");
}
