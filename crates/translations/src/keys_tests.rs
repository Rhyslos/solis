use super::*;

#[test]
fn keys_are_a_group_and_names_lust_can_use() {
    for good in [
        "Shop.Welcome",
        "Menu.Options.Volume",
        "Items.IronSword.description",
        "a.b",
    ] {
        assert_eq!(key_problem(good), None, "{good}");
    }
    for bad in [
        "Welcome",
        "Shop.",
        ".Welcome",
        "Shop.2nd",
        "Shop.Wel-come",
        "Shop.Big Word",
        "Shop.UI",
        "Maths.Pi",
    ] {
        let why = key_problem(bad).unwrap_or_else(|| panic!("{bad} taken"));
        assert!(why.contains("such as Shop.Welcome"), "{why}");
    }
}

#[test]
fn a_key_and_its_fluent_name_turn_into_each_other() {
    assert_eq!(fluent_id("Shop.Welcome"), "Shop-Welcome");
    assert_eq!(key_of("Shop-Welcome").unwrap(), "Shop.Welcome");
    assert!(key_of("shop-welcome-").is_err());
    assert_eq!(file_for("Items.IronSword.name"), "items.ftl");
    assert_eq!(as_path("Shop.Welcome"), "Shop/Welcome");
}

#[test]
fn lines_with_values_are_methods_on_their_groups() {
    assert_eq!(group_object("Shop"), "ShopText");
    assert_eq!(method_name("Shop.Welcome"), "welcome");
    assert_eq!(method_name("Menu.Options.Volume"), "optionsVolume");
    assert_eq!(value_name("player-name").as_deref(), Some("playerName"));
    assert_eq!(value_name("count").as_deref(), Some("count"));
    assert_eq!(value_name("2x"), None);
    assert_eq!(value_name("a b"), None);
}

#[test]
fn the_nearest_key_is_suggested() {
    let keys = ["Shop.Welcome", "Shop.Bye"];
    assert_eq!(
        nearest("Shop.Welcom", keys.into_iter()),
        Some("Shop.Welcome")
    );
    assert_eq!(nearest("Menu.Quit", keys.into_iter()), None);
}
