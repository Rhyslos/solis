use super::*;

#[test]
fn languages_are_named_and_called_by_game_code() {
    let nb = Language::new("nb").unwrap();
    assert_eq!(
        (nb.code(), nb.name().as_str(), nb.option().as_str()),
        ("nb", "Norsk", "Nb")
    );
    let br = Language::new("pt-BR").unwrap();
    assert_eq!((br.base(), br.option().as_str()), ("pt", "PtBr"));
    assert_eq!(br.name(), "Português (Brasil)");
    assert_eq!(Language::new("zh-Hans").unwrap().option(), "ZhHans");
    assert_eq!(
        Language::new("tlh").unwrap().name(),
        "tlh",
        "unknown: its code"
    );
    assert!(Language::new("English").is_err());
    assert_eq!(of_option("ptbr", &[nb, br]), Some(br));
}

#[test]
fn the_best_language_for_a_player() {
    let all: Vec<Language> = ["en", "nb", "pt-BR", "fr"]
        .iter()
        .map(|c| Language::new(c).unwrap())
        .collect();
    let l = |c: &str| Language::new(c).unwrap();
    assert_eq!(best(Some(l("fr")), &all), l("fr"));
    assert_eq!(best(Some(l("nb-NO")), &all), l("nb"));
    assert_eq!(best(Some(l("pt")), &all), l("pt-BR"));
    assert_eq!(best(Some(l("ja")), &all), l("en"), "not had: the main one");
    assert_eq!(best(None, &all), l("en"));
    assert_eq!(best(None, &[]), Language::english());
}

#[test]
fn the_systems_language_never_fails() {
    let _ = system();
    assert!(known().len() > 40);
    assert!(known().iter().all(|l| !l.name().is_empty()));
}
