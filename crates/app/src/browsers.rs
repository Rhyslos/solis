//! The web browsers Windows lists as installed, for File › Preferences ›
//! Open the guide in (decision 0112).
//!
//! Windows keeps the list in the registry, under
//! `SOFTWARE\Clients\StartMenuInternet` of the machine and of the user:
//! one key a browser, holding its name and, under `shell\open\command`,
//! how to start it. It is read by asking Windows' own `reg.exe` (no
//! library and no unsafe code), once, on a thread as the editor starts.
//! What it says is untrusted (rule 9): its size, the number of browsers
//! and every name and path are limited, and a path that isn't a program
//! on this computer's own drives is dropped. Reading the answer is plain
//! code, tested on every system; only asking is Windows' alone.
#![cfg_attr(
    not(windows),
    allow(
        dead_code,
        reason = "the answer is read on Windows only, and tested everywhere"
    )
)]

use editor::FoundBrowser;
use std::path::PathBuf;

/// More of `reg.exe`'s answer than this isn't read.
const LONGEST_ANSWER: usize = 1 << 20;
/// More browsers than this aren't listed.
const MOST_BROWSERS: usize = 32;
/// A longer name isn't a browser's.
const LONGEST_NAME: usize = 100;
/// Where the list is, under each of the two roots.
const LIST: &str = "SOFTWARE\\Clients\\StartMenuInternet";

/// Starts looking, and gives back where the list will arrive. Elsewhere
/// than Windows the list is empty at once.
pub(crate) fn start_looking() -> std::sync::mpsc::Receiver<Vec<FoundBrowser>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    #[cfg(windows)]
    std::thread::spawn(move || {
        let mut found = Vec::new();
        for root in ["HKLM", "HKCU"] {
            if let Some(answer) = registry::ask(root) {
                found.extend(read_answer(&answer, |name| std::env::var(name).ok()));
            }
        }
        let _ = sender.send(tidy(found));
    });
    #[cfg(not(windows))]
    let _ = sender.send(Vec::new());
    receiver
}

/// The browsers in `reg.exe`'s answer to `reg query <root>\…\StartMenuInternet /s`.
/// `variable` gives an environment variable's value, for paths written
/// with `%ProgramFiles%`.
pub(crate) fn read_answer(
    answer: &str,
    variable: impl Fn(&str) -> Option<String>,
) -> Vec<FoundBrowser> {
    let answer = cut_to(answer, LONGEST_ANSWER);
    // Each browser's key: its name (the key's own value) and its program.
    let mut browsers: Vec<(String, Option<String>, Option<PathBuf>)> = Vec::new();
    let mut key: Option<(String, String)> = None;
    for line in answer.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with("HKEY_") {
            key = browser_key(line);
            continue;
        }
        let (Some((browser, rest)), Some((name, data, expand))) = (&key, value(line)) else {
            continue;
        };
        if !(name.starts_with('(') && name.ends_with(')')) {
            continue; // Only a key's own value (Windows calls it "(Default)").
        }
        let at = match browsers.iter().position(|(b, ..)| b == browser) {
            Some(at) => at,
            None if browsers.len() < MOST_BROWSERS => {
                browsers.push((browser.clone(), None, None));
                browsers.len() - 1
            }
            None => continue,
        };
        if rest.is_empty() {
            browsers[at].1 = Some(data.to_string());
        } else if rest.eq_ignore_ascii_case("shell\\open\\command") {
            let data = if expand {
                expanded(data, &variable)
            } else {
                data.to_string()
            };
            browsers[at].2 = program_of(&data);
        }
    }
    browsers
        .into_iter()
        .filter_map(|(key, name, program)| {
            let program = program?;
            let name = name.filter(|n| good_name(n)).unwrap_or_else(|| key.clone());
            good_name(&name).then_some(FoundBrowser { name, program })
        })
        .collect()
}

/// The first `limit` bytes of `text`, cut at a character's edge.
fn cut_to(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// For a key line under the list: the browser's key name and the rest of
/// the path after it (empty for the browser's own key).
fn browser_key(line: &str) -> Option<(String, String)> {
    let lower = line.to_ascii_lowercase();
    let at = lower.find(&LIST.to_ascii_lowercase())? + LIST.len();
    let rest = line[at..].strip_prefix('\\')?;
    let (browser, after) = rest.split_once('\\').unwrap_or((rest, ""));
    (!browser.is_empty()).then(|| (browser.to_string(), after.to_string()))
}

/// A value line: its name, its data, and whether variables in it are to
/// be filled in (`REG_EXPAND_SZ`). Other kinds of value are left alone.
fn value(line: &str) -> Option<(&str, &str, bool)> {
    let line = line.trim_start();
    for (kind, expand) in [("    REG_SZ    ", false), ("    REG_EXPAND_SZ    ", true)] {
        if let Some((name, data)) = line.split_once(kind) {
            return Some((name.trim(), data.trim(), expand));
        }
    }
    // An empty value ends with its kind.
    None
}

/// `%NAME%` replaced by its value; an unknown one stays as it is.
fn expanded(text: &str, variable: &impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match variable(name).filter(|_| !name.is_empty()) {
                    Some(value) => out.push_str(&value),
                    None => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The program a command line starts: the part in quotes, or up to and
/// including `.exe`. `None` unless it is a program on this computer's
/// own drives.
fn program_of(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let path = if let Some(quoted) = command.strip_prefix('"') {
        quoted.split('"').next()?
    } else {
        let end = command.to_ascii_lowercase().find(".exe")? + 4;
        &command[..end]
    };
    let program = PathBuf::from(path);
    let drive = path.as_bytes();
    let on_a_drive = drive.len() > 3 && drive[0].is_ascii_alphabetic() && &drive[1..3] == b":\\";
    let is_exe = path.to_ascii_lowercase().ends_with(".exe");
    let plain = path.len() <= 1024 && !path.chars().any(char::is_control) && !path.contains("..");
    (on_a_drive && is_exe && plain).then_some(program)
}

fn good_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= LONGEST_NAME
        && !name.starts_with('@')
        && !name.chars().any(char::is_control)
}

/// One of each program, in the order found (the machine's first).
fn tidy(found: Vec<FoundBrowser>) -> Vec<FoundBrowser> {
    let mut out: Vec<FoundBrowser> = Vec::new();
    for browser in found {
        let seen = out.iter().any(|b| {
            b.program
                .to_string_lossy()
                .eq_ignore_ascii_case(&browser.program.to_string_lossy())
        });
        if !seen && out.len() < MOST_BROWSERS {
            out.push(browser);
        }
    }
    out
}

#[cfg(windows)]
mod registry {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    /// Starts a console program without a console window flashing up.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// `reg.exe`'s answer for the list under `root`, read up to the
    /// limit; `None` if it couldn't be asked or said nothing.
    pub(super) fn ask(root: &str) -> Option<String> {
        // Windows' own reg.exe, by its full path: never one a folder on
        // the search path happens to hold.
        let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let reg = std::path::Path::new(&system)
            .join("System32")
            .join("reg.exe");
        let mut child = Command::new(reg)
            .args(["query", &format!("{root}\\{}", super::LIST), "/s"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .ok()?;
        let mut bytes = Vec::new();
        let read = child
            .stdout
            .take()?
            .take(super::LONGEST_ANSWER as u64)
            .read_to_end(&mut bytes);
        let _ = child.kill();
        let _ = child.wait();
        read.ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }
}

#[cfg(test)]
#[path = "browsers_tests.rs"]
mod tests;
