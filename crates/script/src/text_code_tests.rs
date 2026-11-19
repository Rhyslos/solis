use super::*;
use level::languages::Languages;
use translations::Translations;

fn translations(text: &str) -> Translations {
    Translations::new(
        &Languages::default(),
        &[("locale/en/menu.ftl".into(), text.into())],
        Vec::new(),
    )
}

#[test]
fn no_line_with_values_writes_no_file() {
    let (names, notes) = TextNames::of(&translations("Menu-Start = Start\n"), &[]);
    assert!(notes.is_empty());
    assert_eq!(names.keys, ["Menu.Start"]);
    assert_eq!(texts_file("Test", &names), None);
}

#[test]
fn values_lust_cant_name_and_clashes_are_said() {
    let (names, notes) = TextNames::of(
        &translations(
            "Menu-Hi = Hi { $player-name }\nMenu-Bad = { $int }\nMenu-OptionsVolume = { $v }\nMenu-Options-Volume = { $v }\n",
        ),
        &["MenuText".into()],
    );
    assert!(names.methods.is_empty(), "MenuText is taken");
    assert_eq!(notes.len(), 4, "{notes:?}");
    let (names, notes) = TextNames::of(
        &translations(
            "Menu-Hi = Hi { $player-name }\nMenu-Bad = { $int }\nMenu-OptionsVolume = { $v }\nMenu-Options-Volume = { $v }\n",
        ),
        &[],
    );
    let made: Vec<String> = names
        .methods
        .iter()
        .map(|m| format!("{}:{}", m.object, m.method))
        .collect();
    assert_eq!(made, ["MenuText:hi", "MenuText:optionsVolume"]);
    assert_eq!(names.methods[0].values[0].1, "playerName");
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(notes.iter().any(|n| n.contains("$int")), "{notes:?}");
    assert!(
        notes
            .iter()
            .any(|n| n.contains("Menu.OptionsVolume is already")),
        "{notes:?}"
    );
}
