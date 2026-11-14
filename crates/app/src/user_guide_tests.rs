use super::*;

fn guide() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:\\Programs\\Example\\user-guide\\index.html")
    } else {
        PathBuf::from("/opt/example/user-guide/index.html")
    }
}

fn browser() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:\\Browsers\\example.exe")
    } else {
        PathBuf::from("/usr/bin/example")
    }
}

fn everything_is_there(_: &Path) -> bool {
    true
}

#[test]
fn the_guide_is_beside_the_program() {
    let program = guide()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("example.exe");
    assert_eq!(beside(&program), Some(guide()));
}

#[test]
fn the_default_browser_gets_the_guide_s_address() {
    let plan = plan(
        &guide(),
        Some("lights"),
        &GuideBrowser::Default,
        everything_is_there,
    )
    .unwrap();
    let Opening::Default(address) = plan.opening else {
        panic!("the default browser");
    };
    assert!(address.starts_with("file:///"), "{address}");
    assert!(
        address.ends_with("/user-guide/index.html#lights"),
        "{address}"
    );
    assert_eq!(plan.note, None);
}

#[test]
fn a_chosen_program_is_started_with_the_address() {
    for browser_choice in [
        GuideBrowser::Chosen(browser()),
        GuideBrowser::Installed {
            name: "Example".into(),
            program: browser(),
        },
    ] {
        let plan = plan(&guide(), None, &browser_choice, everything_is_there).unwrap();
        assert!(
            matches!(&plan.opening, Opening::Program(p, a) if p == &browser() && a.ends_with("index.html")),
            "{plan:?}"
        );
    }
}

#[test]
fn a_browser_that_is_gone_falls_back_to_the_default_with_a_note() {
    let only_the_guide = |path: &Path| path == guide();
    let installed = GuideBrowser::Installed {
        name: "Example Browser".into(),
        program: browser(),
    };
    let plan = plan(&guide(), None, &installed, only_the_guide).unwrap();
    assert!(matches!(plan.opening, Opening::Default(_)));
    assert_eq!(
        plan.note.as_deref(),
        Some("Example Browser isn't there any more: the guide opened in the default browser")
    );
}

#[test]
fn a_missing_guide_says_where_it_was_looked_for() {
    let missing = plan(&guide(), None, &GuideBrowser::Default, |_| false).unwrap_err();
    assert_eq!(missing.looked, guide());
}

#[test]
fn addresses_are_written_safely() {
    let path = Path::new("C:\\My Games\\Ünïcode #1\\user-guide\\index.html");
    assert_eq!(
        address(path, Some("tile-maps")),
        "file:///C:/My%20Games/%C3%9Cn%C3%AFcode%20%231/user-guide/index.html#tile-maps"
    );
    // A chapter that isn't a page's name is left out.
    assert!(!address(path, Some("x\"><script>")).contains('#'));
    assert!(!address(path, Some("")).contains('#'));
}

#[test]
fn a_saved_choice_that_couldn_t_be_a_program_is_the_default() {
    assert_eq!(
        checked(GuideBrowser::Chosen(browser())),
        GuideBrowser::Chosen(browser())
    );
    for bad in [
        GuideBrowser::Chosen(PathBuf::from("relative.exe")),
        GuideBrowser::Chosen(PathBuf::from("\\\\server\\share\\b.exe")),
        GuideBrowser::Installed {
            name: String::new(),
            program: browser(),
        },
        GuideBrowser::Installed {
            name: "x".repeat(500),
            program: browser(),
        },
        GuideBrowser::Installed {
            name: "Bell\u{7}".into(),
            program: browser(),
        },
    ] {
        assert_eq!(checked(bad), GuideBrowser::Default);
    }
}
