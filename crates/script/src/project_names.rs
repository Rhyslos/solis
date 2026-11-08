//! The project's own names as Lust states ([decision 0065](../../../docs/decisions/0065-engine-api-guidelines.md)):
//! `solis/project.lst`, written by the editor whenever the project's files
//! change, so game code names an assembly as `Assembly.Ships.Fighter`, not
//! as text. A typo is then a mistake before the game runs, and a renamed
//! file points at every line still using the old name.
//!
//! Folders become options inside options: `ships/fighter.solassembly` is
//! `Assembly.Ships.Fighter`. Each part's first letter is made a capital
//! (Lust's options start with one); the rest is kept as it is. Natives get
//! the full name back (`"Ships.Fighter"`) and find the file from it,
//! ignoring capitals as the Library does.

use std::collections::BTreeMap;

/// The buses a new project's Mixer has (`sound::Mixer::default`; the app
/// checks they agree).
pub const STANDARD_BUSES: &[&str] = &["Master", "Music", "Effects", "Voice", "Ambience", "UI"];

/// The option a project has when it has none of its own (Lust's states
/// need one): `Assembly.None`, which makes nothing.
pub const NO_OPTION: &str = "None";

/// One level of a state: options, each with the options inside it (none
/// for a file).
#[derive(Debug, Default)]
struct Level {
    options: BTreeMap<String, (String, Level)>,
    /// Whether this option is a file itself (and not only a folder).
    file: bool,
}

/// The project's names, as Solis finds them, for [`project_file`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Names {
    /// Its assemblies: their paths in the assets, with `/` between
    /// folders and without `.solassembly` (`ships/fighter`).
    pub assemblies: Vec<String>,
    /// Its input actions' names (`Jump`), as Edit > Input actions… has
    /// them.
    pub actions: Vec<String>,
    /// Its sound events: their paths in the assets, without `.solsound`
    /// (`sfx/footstep`).
    pub sounds: Vec<String>,
    /// Its Mixer's buses' names (`Music`).
    pub buses: Vec<String>,
    /// Its sprites' clips, as paths: the sheet's path in the assets
    /// without its extension, then the clip's name
    /// (`sprites/farmer/walk`).
    pub clips: Vec<String>,
    /// The names of its clips' events (cues: `footstep`).
    pub cues: Vec<String>,
    /// Its game screens' names (`PauseMenu`, from `ui/pauseMenu.lst`).
    pub screens: Vec<String>,
    /// Its pictures cut by Atlas Crop, as paths: the picture's path in the
    /// assets without its extension, then the crop's name
    /// (`sprites/tools/Axe`).
    pub pictures: Vec<String>,
}

impl Names {
    /// A project's names before it has any of its own: no assemblies, and
    /// the ready-made input actions.
    pub fn standard() -> Self {
        Self {
            assemblies: Vec::new(),
            actions: input::Bindings::default()
                .actions
                .into_iter()
                .map(|a| a.name)
                .collect(),
            sounds: Vec::new(),
            buses: STANDARD_BUSES.iter().map(|b| (*b).to_string()).collect(),
            clips: Vec::new(),
            cues: Vec::new(),
            screens: Vec::new(),
            pictures: Vec::new(),
        }
    }
}

impl Names {
    /// [`Names::standard`] with these assemblies (paths without the
    /// extension), for tests and samples.
    pub fn with_assemblies(paths: &[&str]) -> Self {
        let mut names = Self::standard();
        names.assemblies = paths.iter().map(|p| (*p).to_string()).collect();
        names
    }

    /// These names with these sound events (paths without the extension).
    #[must_use]
    pub fn and_sounds(mut self, sounds: &[&str]) -> Self {
        self.sounds.extend(sounds.iter().map(|a| (*a).to_string()));
        self
    }

    /// These names with these clips (`sheet/clip` paths) and cues.
    #[must_use]
    pub fn and_clips(mut self, clips: &[&str], cues: &[&str]) -> Self {
        self.clips.extend(clips.iter().map(|a| (*a).to_string()));
        self.cues.extend(cues.iter().map(|a| (*a).to_string()));
        self
    }

    /// These names with these screens (`PauseMenu`).
    #[must_use]
    pub fn and_screens(mut self, screens: &[&str]) -> Self {
        self.screens
            .extend(screens.iter().map(|a| (*a).to_string()));
        self
    }

    /// These names with more buses.
    #[must_use]
    pub fn and_buses(mut self, buses: &[&str]) -> Self {
        self.buses.extend(buses.iter().map(|a| (*a).to_string()));
        self
    }

    /// These names with more input actions.
    #[must_use]
    pub fn and_actions(mut self, actions: &[&str]) -> Self {
        self.actions
            .extend(actions.iter().map(|a| (*a).to_string()));
        self
    }
}

/// The actions the kit's ready-made components read (`solis/kit/`). They
/// are always named, so the kit compiles in a project that hasn't made
/// them; one the project hasn't made is never held and its axis is 0.
pub const KIT_ACTIONS: [&str; 8] = [
    "Move", "Look", "Jump", "Sprint", "Zoom", "Pan", "Turn", "Drag",
];

/// `solis/project.lst`'s text, from the project's names, and notes on any
/// that can't be named in Lust. `made_by` names the program and version,
/// for the first line.
pub fn project_file(made_by: &str, names: &Names) -> (String, Vec<String>) {
    let mut notes = Vec::new();
    let assembly = state_text("Assembly", &names.assemblies, "assembly", &mut notes);
    let mut actions = names.actions.clone();
    for kit in KIT_ACTIONS {
        if !actions.iter().any(|a| a.eq_ignore_ascii_case(kit)) {
            actions.push(kit.to_string());
        }
    }
    let action = names_text("Action", &actions, "input action", true, &mut notes);
    let sound = state_text("Sound", &names.sounds, "sound event", &mut notes);
    let bus = names_text("Bus", &names.buses, "bus", false, &mut notes);
    let clip = state_text("Clip", &names.clips, "sprite clip", &mut notes);
    let cue = names_text("Cue", &names.cues, "clip event", false, &mut notes);
    let screen = names_text("Screen", &names.screens, "screen", false, &mut notes);
    let picture = state_text("Picture", &names.pictures, "cropped picture", &mut notes);
    let text = format!(
        "-- Made by {made_by}: the project's names, for game code. It's written again\n\
         -- whenever the project's files change, so changes made here don't last.\n\n\
         -- The project's assemblies, folders as options inside options: World:spawn(Assembly.Ships.Fighter, place)\n\
         -- for ships/fighter.\n\
         {assembly}\n\n\
         -- The project's input actions (Edit > Input actions…): Input:pressed(Action.Jump).\n\
         {action}\n\n\
         -- The project's sound events, folders as options inside options: Audio:play(Sound.Sfx.Footstep)\n\
         -- for sfx/footstep.\n\
         {sound}\n\n\
         -- The Mixer's buses: Audio:setVolume(Bus.Music, 0.5).\n\
         {bus}\n\n\
         -- The sprites' clips, by their sheets: animate(Clip.Sprites.Farmer.Walk).\n\
         {clip}\n\n\
         -- The clips' events: on CueReached(Cue cue) {{ is cue == Cue.Footstep? … }}.\n\
         {cue}\n\n\
         -- The game's screens (files in ui/): showScreen(Screen.PauseMenu) for ui/pauseMenu.lst.\n\
         {screen}\n\n\
         -- The pictures cut out with Atlas Crop, by their pictures: Picture.Sprites.Tools.Axe for\n\
         -- sprites/tools.png's Axe.\n\
         {picture}\n\n\
         pass {};\n\
         end\n",
        crate::describe::PROJECT_STATES.join(", ")
    );
    (text, notes)
}

/// One state's line: `state Assembly = [Crate, Ships [Fighter]];`.
fn state_text(state: &str, paths: &[String], what: &str, notes: &mut Vec<String>) -> String {
    let mut top = Level::default();
    let mut paths: Vec<&String> = paths.iter().collect();
    paths.sort();
    paths.dedup();
    // Paths the same but for capitals name the same option.
    let mut seen: BTreeMap<String, &String> = BTreeMap::new();
    'paths: for path in paths {
        if let Some(first) = seen.get(&path.to_lowercase()) {
            notes.push(format!(
                "Two {what} files, {first} and {path}, have the same name but for capitals, so game code \
                 can't tell them apart. Rename one to use it from code."
            ));
            continue;
        }
        let mut parts = Vec::new();
        for part in path.split('/') {
            match option_name(part) {
                Some(name) => parts.push(name),
                None => {
                    notes.push(format!(
                        "The {what} {path} can't be named in game code: each part of its path must start \
                         with a letter, hold only letters, digits and _, and not be one \
                         of Lust's own objects (Maths, Files, Time, Json, Web, Random). Rename it to use it \
                         from code."
                    ));
                    continue 'paths;
                }
            }
        }
        seen.insert(path.to_lowercase(), path);
        let mut level = &mut top;
        for name in parts {
            level = &mut level
                .options
                .entry(name.to_lowercase())
                .or_insert_with(|| (name.clone(), Level::default()))
                .1;
        }
        level.file = true;
    }
    let mut options = Vec::new();
    write_level(&top, "", what, notes, &mut options);
    if options.is_empty() {
        options.push(NO_OPTION.to_string());
    }
    format!("state {state} = [{}];", options.join(", "))
}

/// A state of plain names. `exact`: each must already be an option as it
/// is (`Jump`; an input action, which game code asks for by its exact
/// name), else it's left out with a note. Otherwise it's made one as a
/// path's part is (the `UI` bus is `Bus.Ui`), and found again ignoring
/// capitals.
fn names_text(
    state: &str,
    names: &[String],
    what: &str,
    exact: bool,
    notes: &mut Vec<String>,
) -> String {
    let mut options: Vec<String> = Vec::new();
    for name in names {
        let option = option_name(name);
        if option.is_none() || exact && option.as_deref() != Some(name.as_str()) {
            notes.push(format!(
                "The {what} \"{name}\" can't be named in game code: it must start with a capital, hold \
                 only letters, digits and _, not be all capitals, and not be one of Lust's own objects. \
                 Rename it to use it from code."
            ));
        } else if options.iter().any(|o| o.eq_ignore_ascii_case(name)) {
            notes.push(format!(
                "Two {what}s are called \"{name}\" but for capitals, so game code can't tell them apart."
            ));
        } else if let Some(option) = option {
            options.push(option);
        }
    }
    if options.is_empty() {
        options.push(NO_OPTION.to_string());
    }
    format!("state {state} = [{}];", options.join(", "))
}

/// The options of one level, written: a file is its name, a folder its
/// name and the options inside. A name both a file and a folder keeps the
/// folder, with a note.
fn write_level(
    level: &Level,
    at: &str,
    what: &str,
    notes: &mut Vec<String>,
    out: &mut Vec<String>,
) {
    for (name, inner) in level.options.values() {
        let path = if at.is_empty() {
            name.clone()
        } else {
            format!("{at}.{name}")
        };
        if inner.options.is_empty() {
            out.push(name.clone());
            continue;
        }
        if inner.file {
            notes.push(format!(
                "The {what} {path} has the same name as a folder beside it, so game code can't name it. \
                 Rename one to use it from code."
            ));
        }
        let mut inside = Vec::new();
        write_level(inner, &path, what, notes, &mut inside);
        out.push(format!("{name} [{}]", inside.join(", ")));
    }
}

/// A path part as an option's name (its first letter a capital, and one in
/// all capitals, which Lust doesn't take, as `Ufo`), or none when it can't
/// be one, by Lust's rules: it must start with a letter, hold only ASCII
/// letters, digits and `_`, and not be one of Lust's own objects (`Time`).
fn option_name(part: &str) -> Option<String> {
    let mut chars = part.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || !part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let mut name = first.to_ascii_uppercase().to_string() + chars.as_str();
    let letters: Vec<char> = name.chars().filter(char::is_ascii_alphabetic).collect();
    if letters.len() >= 2 && letters.iter().all(char::is_ascii_uppercase) {
        name = name[..1].to_string() + &name[1..].to_ascii_lowercase();
    }
    if LUST_OBJECTS.contains(&name.as_str()) {
        return None;
    }
    Some(name)
}

/// A name as a Lust option or class name, folded as a path's part is
/// (`farmer` is `Farmer`), for the animators' files; or none.
pub fn option_name_of(part: &str) -> Option<String> {
    option_name(part)
}

/// Lust's own objects, which no option may be called.
const LUST_OBJECTS: &[&str] = &["Maths", "Files", "Time", "Json", "Web", "Random"];

/// A path (`sprites/farmer/walk`) or a name (`footstep`) as the option
/// the project file names it by (`Sprites.Farmer.Walk`, `Footstep`), for
/// the engine's events; `None` when it can't be one.
pub fn option_of(path: &str) -> Option<String> {
    let parts: Option<Vec<String>> = path.split('/').map(option_name).collect();
    parts.map(|p| p.join("."))
}

/// A state's full option name from a native (`"Ships.Fighter"`) as the
/// path it stands for, to find the file by: each part's first letter made
/// small again (`"ships/fighter"`), as files are mostly named; the Library
/// and the sounds find it ignoring capitals anyway. `None` for
/// [`NO_OPTION`].
pub fn path_of(option: &str) -> Option<String> {
    (option != NO_OPTION && !option.is_empty()).then(|| {
        option
            .split('.')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_ascii_lowercase().to_string() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join("/")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assemblies(list: &[&str]) -> Names {
        Names {
            assemblies: list.iter().map(|p| (*p).to_string()).collect(),
            ..Names::default()
        }
    }

    #[test]
    fn input_actions_are_a_state_of_their_own_names() {
        let names = Names {
            actions: [
                "Move",
                "Jump",
                "jump",
                "Quick save",
                "QuickSave",
                "UI",
                "QUICKSAVE",
                "Fire_2",
            ]
            .map(String::from)
            .to_vec(),
            ..Names::default()
        };
        let (text, notes) = project_file("Test 1.0", &names);
        // The project's own first, then the kit's it hasn't made, so the
        // kit compiles anywhere (the playtest games' feedback).
        assert!(
            text.contains(
                "state Action = [Move, Jump, QuickSave, Fire_2, Look, Sprint, Zoom, Pan, Turn, Drag];"
            ),
            "{text}"
        );
        assert_eq!(notes.len(), 4, "{notes:?}");
        let (text, _) = project_file("Test 1.0", &Names::default());
        assert!(
            text.contains("state Action = [Move, Look, Jump, Sprint, Zoom, Pan, Turn, Drag];"),
            "{text}"
        );
        // Buses are found again ignoring capitals, so the UI bus is Bus.Ui.
        let (text, notes) = project_file("Test 1.0", &Names::standard());
        assert!(notes.is_empty(), "{notes:?}");
        assert!(
            text.contains("state Bus = [Master, Music, Effects, Voice, Ambience, Ui];"),
            "{text}"
        );
        assert!(
            text.ends_with("pass Assembly, Action, Sound, Bus, Clip, Cue, Screen, Picture;\nend\n")
        );
        assert!(text.contains("state Picture = [None];"));
        assert!(text.contains("state Screen = [None];"));
        assert!(text.contains("state Clip = [None];"));
        assert!(text.contains("state Sound = [None];"));
    }

    #[test]
    fn cropped_pictures_are_options_under_their_pictures() {
        let mut names = Names::standard();
        names.pictures = vec![
            "sprites/tools/Axe".into(),
            "sprites/tools/Hoe".into(),
            "hero/Face".into(),
        ];
        let (text, notes) = project_file("Test 1.0", &names);
        assert!(
            text.contains("state Picture = [Hero [Face], Sprites [Tools [Axe, Hoe]]];"),
            "{text}"
        );
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn the_game_s_screens_are_a_state_of_their_names() {
        let names = Names::standard().and_screens(&["PauseMenu", "Hud", "HUD"]);
        let (text, notes) = project_file("Test 1.0", &names);
        assert!(text.contains("state Screen = [PauseMenu, Hud];"), "{text}");
        assert_eq!(notes.len(), 1, "HUD is all capitals: {notes:?}");
    }

    #[test]
    fn folders_become_options_inside_options() {
        let (text, notes) = project_file(
            "Test 1.0",
            &assemblies(&["ships/fighter", "ships/freighter", "crate", "crops/wheat"]),
        );
        assert!(notes.is_empty(), "{notes:?}");
        assert!(
            text.contains("state Assembly = [Crate, Crops [Wheat], Ships [Fighter, Freighter]];"),
            "{text}"
        );
        assert!(
            text.ends_with("pass Assembly, Action, Sound, Bus, Clip, Cue, Screen, Picture;\nend\n")
        );
        assert!(text.contains("state Screen = [None];"));
        assert!(text.contains("state Clip = [None];"));
        assert!(text.contains("state Sound = [None];"));
    }

    #[test]
    fn a_project_with_no_assemblies_has_none() {
        let (text, notes) = project_file("Test 1.0", &Names::default());
        assert!(notes.is_empty());
        assert!(text.contains("state Assembly = [None];"));
    }

    #[test]
    fn names_that_cant_be_options_are_left_out_with_a_note() {
        let (text, notes) = project_file(
            "Test 1.0",
            &assemblies(&[
                "2fast",
                "big-ship",
                "UFO",
                "time",
                "ok",
                "Ships/a",
                "ships/b",
                "dock",
                "dock/crane",
                "fighter",
                "Fighter",
            ]),
        );
        assert!(
            text.contains("state Assembly = [Dock [Crane], Fighter, Ok, Ships [A, B], Ufo];"),
            "{text}"
        );
        assert_eq!(notes.len(), 5, "{notes:?}");
        assert!(notes.iter().any(|n| n.contains("2fast")));
        assert!(notes.iter().any(|n| n.contains("big-ship")));
        assert!(
            notes
                .iter()
                .any(|n| n.contains("Fighter and fighter") && n.contains("capitals"))
        );
        assert!(
            notes
                .iter()
                .any(|n| n.contains("Dock") && n.contains("folder"))
        );
    }

    #[test]
    fn an_option_leads_back_to_its_file() {
        assert_eq!(path_of("Ships.Fighter").as_deref(), Some("ships/fighter"));
        assert_eq!(path_of("Crate").as_deref(), Some("crate"));
        assert_eq!(path_of("BigShip").as_deref(), Some("bigShip"));
        assert_eq!(path_of(NO_OPTION), None);
        assert_eq!(
            option_of("sprites/farmer/walk").as_deref(),
            Some("Sprites.Farmer.Walk")
        );
        assert_eq!(option_of("UI").as_deref(), Some("Ui"));
        assert_eq!(option_of("big-ship"), None);
    }

    #[test]
    fn the_file_checks_in_lust() {
        let (text, _) = project_file("Test 1.0", &assemblies(&["ships/fighter", "crate"]));
        let files = vec![
            ("solis/project.lst".to_string(), text),
            (
                "main.lst".to_string(),
                "fetch \"solis/project\":Assembly;\nAssembly a = Assembly.Ships.Fighter;\nend\n"
                    .to_string(),
            ),
        ];
        lust::check("main.lst", &files).unwrap();
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(64))]
        /// Any set of paths gives a file Lust takes, with every path either
        /// an option or a note.
        #[test]
        fn any_paths_give_a_file_lust_takes(
            paths in proptest::collection::vec("[a-zA-Z0-9_ -]{1,6}(/[a-zA-Z0-9_ -]{1,6}){0,2}", 0..12)
        ) {
            let (text, _notes) = project_file("Test 1.0", &Names { assemblies: paths.clone(), sounds: paths, ..Names::default() });
            let files = vec![
                ("solis/project.lst".to_string(), text.clone()),
                ("main.lst".to_string(), "fetch \"solis/project\":Assembly;\nend\n".to_string()),
            ];
            proptest::prop_assert!(lust::check("main.lst", &files).is_ok(), "{}", text);
        }
    }
}
