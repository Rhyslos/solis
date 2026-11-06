//! Tying a screen to the game (decision 0089, M5).

use super::*;

const HUD: &str = "fetch \"solis/screen\":Ui;\n\nassembly Hud {\n    view {\n        column [gap 8] {\n            text \"HP\" [size 20];\n            progress 5 [to 20, width 300, height 20];\n            box [width 300, height 12, red];\n            button \"Pause\" {\n            }\n        }\n    }\n}\n\npass Hud;\nend\n";

fn doc() -> ScreenDoc {
    ScreenDoc::new("ui/hud.lst".into(), "Hud".into(), HUD.into())
}

fn health() -> Readable {
    Readable {
        component: "Health".into(),
        reader: "health".into(),
        module: "main".into(),
        fields: vec![
            ("hp".into(), Kind::Number, 20.0),
            ("maxHp".into(), Kind::Number, 20.0),
            ("name".into(), Kind::Words, 0.0),
        ],
    }
}

fn reads_ok(text: &str) {
    let s = ui::source::parse(text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(s.iter().any(|s| s.name == "Hud"));
}

#[test]
fn a_component_is_read_into_its_name() {
    assert_eq!(var_of("Health"), "health");
    assert_eq!(var_of("AudioSource"), "audioSource");
    assert_eq!(var_of(""), "");
}

#[test]
fn tying_words_reads_the_component_once_and_fetches_it() {
    let mut d = doc();
    d.picked = vec![vec![0, 0]];
    assert!(d.tie(&health(), "hp", &TieTo::Words));
    reads_ok(&d.text);
    assert!(
        d.text.contains("Health health = Ui:entity():health();"),
        "{}",
        d.text
    );
    assert!(
        d.text.contains("text health.hp.text() [size 20];"),
        "{}",
        d.text
    );
    assert!(d.text.contains("fetch \"main\":Health;"), "{}", d.text);
    assert_eq!(d.picked, vec![vec![1, 0]], "the piece is one further down");
    assert_eq!(d.reads(), vec![("Health".into(), "health".into())]);
    // A second tie uses what's read, and adds nothing more.
    d.picked = vec![vec![1, 1]];
    assert!(d.tie(&health(), "hp", &TieTo::Value));
    assert!(d.text.contains("progress health.hp [to 20"), "{}", d.text);
    assert_eq!(d.text.matches("Ui:entity():health()").count(), 1);
    assert_eq!(d.text.matches("fetch \"main\"").count(), 1);
    assert_eq!(d.picked, vec![vec![1, 1]]);
    // One undo step each.
    assert!(d.undo());
    assert!(d.undo());
    assert_eq!(d.text, HUD);
}

#[test]
fn a_bar_fills_by_one_field_out_of_another() {
    let mut d = doc();
    d.picked = vec![vec![0, 2]];
    let to = TieTo::Bar {
        side: "width".into(),
        full: 300.0,
        max: "maxHp".into(),
    };
    assert!(d.tie(&health(), "hp", &to));
    reads_ok(&d.text);
    assert!(
        d.text
            .contains("box [width 300 * health.hp / health.maxHp, height 12, red];"),
        "{}",
        d.text
    );
    // Out of a number: its starting value.
    let to = TieTo::Bar {
        side: "height".into(),
        full: 12.0,
        max: "20".into(),
    };
    d.picked = vec![vec![1, 2]];
    assert!(d.tie(&health(), "hp", &to));
    assert!(d.text.contains("height 12 * health.hp / 20"), "{}", d.text);
    d.picked = vec![vec![1, 2]];
    assert!(d.tie(&health(), "hp", &TieTo::Style("opacity".into())));
    assert!(d.text.contains("opacity health.hp"), "{}", d.text);
}

#[test]
fn solis_s_own_components_come_from_solis_screen() {
    let mut d = doc();
    d.picked = vec![vec![0, 0]];
    let t = Readable {
        component: "Transform".into(),
        reader: "transform".into(),
        module: "solis/screen".into(),
        fields: vec![],
    };
    assert!(d.tie(&t, "position", &TieTo::Words));
    assert!(
        d.text.contains("fetch \"solis/screen\":[Ui, Transform];"),
        "{}",
        d.text
    );
    reads_ok(&d.text);
}

#[test]
fn fetching_adds_to_the_line_or_makes_one() {
    let t = "fetch \"main\":Clock;\nassembly A {}\n";
    assert_eq!(fetched(t, "main", "Clock"), t, "already");
    assert_eq!(
        fetched(t, "main", "Health"),
        "fetch \"main\":[Clock, Health];\nassembly A {}\n"
    );
    let t2 = "fetch \"main\":[Clock, Money];\n";
    assert_eq!(fetched(t2, "main", "Money"), t2);
    assert_eq!(
        fetched(t2, "main", "Hp"),
        "fetch \"main\":[Clock, Money, Hp];\n"
    );
    assert_eq!(
        fetched("assembly A {}\n", "main", "Hp"),
        "fetch \"main\":Hp;\nassembly A {}\n",
        "none yet: at the top"
    );
    assert_eq!(
        fetched("fetch \"solis/screen\":Ui;\r\nend\r\n", "main", "Hp"),
        "fetch \"solis/screen\":Ui;\r\nfetch \"main\":Hp;\r\nend\r\n",
        "after the others, with the file's line ends"
    );
}

#[test]
fn nothing_picked_or_nothing_to_tie_changes_nothing() {
    let mut d = doc();
    assert!(!d.tie(&health(), "hp", &TieTo::Words), "nothing picked");
    d.picked = vec![vec![0]];
    // A column's words: it takes none.
    assert!(!d.tie(&health(), "hp", &TieTo::Words));
    assert_eq!(d.text, HUD);
}

#[test]
fn a_button_sends_one_of_the_game_s_events_fetching_it() {
    let mut d = doc();
    d.picked = vec![vec![0, 3]];
    assert_eq!(d.sends(), Some(None), "nothing yet");
    assert!(d.set_sends("Pause", "main.lst"));
    reads_ok(&d.text);
    assert!(
        d.text.contains(
            "button \"Pause\" {\n                send Pause() to Ui:entity();\n            }"
        ),
        "{}",
        d.text
    );
    assert!(d.text.contains("fetch \"main\":Pause;"), "{}", d.text);
    assert_eq!(d.sends(), Some(Some("Pause".into())));
    assert!(d.set_sends("Quit", "main.lst"));
    assert_eq!(d.sends(), Some(Some("Quit".into())));
    assert_eq!(d.text.matches("send ").count(), 1, "replaced, not added");
    assert!(
        d.text.contains("fetch \"main\":[Pause, Quit];"),
        "{}",
        d.text
    );
    // Code of its own: not shown as an event.
    let t = d.text.replace(
        "send Quit() to Ui:entity();",
        "send Quit() to Ui:entity(); Ui:close();",
    );
    assert!(d.change(t));
    assert_eq!(d.sends(), None);
    // A piece with no { }.
    d.picked = vec![vec![0, 0]];
    assert_eq!(d.sends(), None);
    assert!(!d.set_sends("Pause", "main.lst"));
}

#[test]
fn review_findings_stay_fixed() {
    // A comment after a fetch line: the name goes in the list, not the
    // comment.
    assert_eq!(
        fetched("fetch \"main\":[Pause]; // events\n", "main", "Health"),
        "fetch \"main\":[Pause, Health]; // events\n"
    );
    assert_eq!(
        fetched("fetch \"main\":Pause; -- x\n", "main", "Health"),
        "fetch \"main\":[Pause, Health]; -- x\n"
    );
    // Odd spacing is still the same fetch line.
    assert_eq!(
        fetched("fetch  \"main\" : [A,B];\n", "main", "B"),
        "fetch  \"main\" : [A,B];\n"
    );
    assert_eq!(
        fetched("fetch \"main\" : [A,B];\n", "main", "C"),
        "fetch \"main\":[A, B, C];\n"
    );
    assert_eq!(
        fetched("fetch \"main\":[];\n", "main", "C"),
        "fetch \"main\":C;\n",
        "an empty list"
    );
    // A fetch with no line end after it.
    assert_eq!(
        fetched("fetch \"a\":X;", "main", "C"),
        "fetch \"a\":X;\nfetch \"main\":C;\n"
    );
    // Solis's own components: Ui fetched too.
    let mut d = ScreenDoc::new(
        "ui/hud.lst".into(),
        "Hud".into(),
        HUD.replace("fetch \"solis/screen\":Ui;", "fetch \"main\":Clock;"),
    );
    d.picked = vec![vec![0, 0]];
    let sparks = Readable {
        component: "Sparks".into(),
        reader: "sparks".into(),
        module: "solis/screen".into(),
        fields: vec![],
    };
    assert!(d.tie(&sparks, "shown", &TieTo::Words));
    assert!(
        d.text.contains("fetch \"solis/screen\":[Sparks, Ui];"),
        "{}",
        d.text
    );
    // Hidden and locked pieces stay so after a read is put in above them.
    let mut d = doc();
    d.hidden.insert(vec![0, 1]);
    d.locked.insert(vec![0, 2]);
    d.picked = vec![vec![0, 0]];
    assert!(d.tie(&health(), "hp", &TieTo::Words));
    assert!(d.hidden.contains(&vec![1, 1]), "{:?}", d.hidden);
    assert!(d.locked.contains(&vec![1, 2]), "{:?}", d.locked);
    // A component written in the screen's own file isn't fetched from it.
    let mut d = doc();
    d.picked = vec![vec![0, 0]];
    let own = Readable {
        module: "ui/hud".into(),
        ..health()
    };
    assert!(d.tie(&own, "hp", &TieTo::Words));
    assert!(!d.text.contains("fetch \"ui/hud\""), "{}", d.text);
}
