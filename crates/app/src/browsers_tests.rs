use super::*;

/// What `reg.exe` says, in its own layout, for two made-up browsers
/// and one whose program can't be used.
const ANSWER: &str = "\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\Example.Browser\r
    (Default)    REG_SZ    Example Browser\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\Example.Browser\\Capabilities\r
    ApplicationName    REG_SZ    Example Browser\r
    (Default)    REG_SZ    not the name\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\Example.Browser\\shell\\open\\command\r
    (Default)    REG_SZ    \"C:\\Program Files\\Example\\browser.exe\" --flag\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\OTHER.EXE\r
    (Standard)    REG_SZ    @C:\\Other\\words.dll,-100\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\OTHER.EXE\\shell\\open\\command\r
    (Standard)    REG_EXPAND_SZ    %ProgramFiles%\\Other\\other.exe\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\Shared\\shell\\open\\command\r
    (Default)    REG_SZ    \\\\server\\share\\browser.exe\r
";

fn program_files(name: &str) -> Option<String> {
    (name == "ProgramFiles").then(|| "C:\\Program Files".to_string())
}

#[test]
fn browsers_are_read_with_their_names_and_programs() {
    let found = read_answer(ANSWER, program_files);
    assert_eq!(
        found,
        [
            FoundBrowser {
                name: "Example Browser".into(),
                program: PathBuf::from("C:\\Program Files\\Example\\browser.exe"),
            },
            // A name Windows looks up elsewhere (`@…`) isn't shown: the
            // key's name is, and its default value is found whatever
            // the language calls it.
            FoundBrowser {
                name: "OTHER.EXE".into(),
                program: PathBuf::from("C:\\Program Files\\Other\\other.exe"),
            },
        ]
    );
}

#[test]
fn a_program_on_a_network_share_or_without_exe_is_dropped() {
    assert_eq!(program_of("\\\\server\\share\\b.exe"), None);
    assert_eq!(program_of("C:\\Tools\\browser.bat"), None);
    assert_eq!(program_of("browser.exe"), None);
    assert_eq!(program_of("C:\\a\\..\\b.exe"), None);
    assert_eq!(
        program_of("C:\\Tools\\b.exe -new-window"),
        Some(PathBuf::from("C:\\Tools\\b.exe"))
    );
}

#[test]
fn an_unknown_variable_stays_and_the_program_is_dropped() {
    assert_eq!(
        expanded("%Nowhere%\\b.exe and %", &program_files),
        "%Nowhere%\\b.exe and %"
    );
    let found = read_answer(ANSWER, |_| None);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn nothing_said_is_no_browsers() {
    assert!(read_answer("", program_files).is_empty());
    assert!(
        read_answer(
            "ERROR: The system was unable to find the specified registry key or value.",
            program_files
        )
        .is_empty()
    );
}

#[test]
fn the_same_program_twice_is_listed_once() {
    let twice = format!(
        "{ANSWER}{}",
        ANSWER.replace("HKEY_LOCAL_MACHINE", "HKEY_CURRENT_USER")
    );
    let found = tidy(read_answer(&twice, program_files));
    assert_eq!(found.len(), 2);
}

#[test]
fn many_browsers_are_cut_to_the_limit() {
    let mut answer = String::new();
    for n in 0..100 {
        answer.push_str(&format!(
            "HKEY_LOCAL_MACHINE\\SOFTWARE\\Clients\\StartMenuInternet\\B{n}\\shell\\open\\command\n    (Default)    REG_SZ    C:\\B\\b{n}.exe\n"
        ));
    }
    assert_eq!(read_answer(&answer, program_files).len(), MOST_BROWSERS);
}

/// Thousands of damaged answers: none may crash the reader (rule 9).
#[test]
fn damaged_answers_never_crash() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let pieces = [
        "\\",
        "%",
        "\"",
        "    REG_SZ    ",
        "(",
        ")",
        "\n",
        "HKEY_",
        ".exe",
        "é",
        "\r",
        "StartMenuInternet\\",
        "C:\\",
    ];
    for _ in 0..5_000 {
        let mut text: Vec<char> = ANSWER.chars().collect();
        for _ in 0..(1 + next() % 30) {
            let at = (next() as usize) % (text.len() + 1);
            if next() % 2 == 0 {
                let piece = pieces[(next() as usize) % pieces.len()];
                for (k, c) in piece.chars().enumerate() {
                    text.insert((at + k).min(text.len()), c);
                }
            } else if at < text.len() {
                text.remove(at);
            }
        }
        let text: String = text.into_iter().collect();
        for browser in read_answer(&text, program_files) {
            assert!(good_name(&browser.name));
            assert!(browser.program.to_string_lossy().ends_with(".exe"));
        }
    }
}

#[test]
fn a_long_answer_is_cut_at_a_character() {
    let text = "é".repeat(10);
    assert_eq!(cut_to(&text, 5), "éé");
}
