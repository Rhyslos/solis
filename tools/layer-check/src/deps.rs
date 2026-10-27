//! The dependency register (`docs/dependencies.md`, decision 0052).
//!
//! Every outside library a crate in `crates/` uses must be listed in the
//! register, with the crates allowed to use it. These functions read the
//! crates' manifests and the register, and say where they disagree:
//! a library used but not listed, used in a crate it isn't allowed in,
//! listed but no longer used, or listed without an exit plan.

use std::fmt;

/// How a crate uses a library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// In the program (`[dependencies]`).
    Normal,
    /// In the build script only (`[build-dependencies]`).
    Build,
    /// In tests only (`[dev-dependencies]`).
    Tests,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Normal => "the program",
            Self::Build => "its build script",
            Self::Tests => "its tests",
        })
    }
}

/// One crate's use of one library.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Use {
    /// The crate, such as `render`.
    pub krate: String,
    /// The library, such as `wgpu`.
    pub library: String,
    /// Where in the crate it's used.
    pub kind: Kind,
}

/// One row of the register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The library.
    pub library: String,
    /// The crates allowed to use it, and how.
    pub allowed: Vec<(String, Kind)>,
    /// Whether the row has an exit plan.
    pub has_exit_plan: bool,
}

/// One disagreement between the crates and the register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// A crate uses a library the register doesn't list.
    Unlisted {
        /// The library.
        library: String,
        /// The crate using it.
        krate: String,
    },
    /// A crate uses a listed library where the register doesn't allow it.
    NotAllowedHere {
        /// The library.
        library: String,
        /// The crate using it.
        krate: String,
        /// How it's used.
        kind: Kind,
    },
    /// The register lists a library no crate uses any more.
    Unused {
        /// The library.
        library: String,
    },
    /// The register allows a use that no longer exists.
    StaleAllowance {
        /// The library.
        library: String,
        /// The crate the register allows it in.
        krate: String,
        /// How it would be used.
        kind: Kind,
    },
    /// A row has no exit plan.
    NoExitPlan {
        /// The library.
        library: String,
    },
    /// A library is listed twice.
    Duplicate {
        /// The library.
        library: String,
    },
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlisted { library, krate } => write!(
                f,
                "{krate} uses `{library}`, which isn't in docs/dependencies.md: add a row \
                 saying what it does, its licence, where it's allowed and its exit plan"
            ),
            Self::NotAllowedHere {
                library,
                krate,
                kind,
            } => write!(
                f,
                "{krate} uses `{library}` in {kind}, but docs/dependencies.md doesn't allow it \
                 there: keep it in the crates it's walled into, or widen its row (and say why)"
            ),
            Self::Unused { library } => write!(
                f,
                "docs/dependencies.md lists `{library}`, but no crate uses it: remove its row"
            ),
            Self::StaleAllowance {
                library,
                krate,
                kind,
            } => write!(
                f,
                "docs/dependencies.md allows `{library}` in {krate} ({kind}), but {krate} \
                 doesn't use it there: remove it from the row's \"Allowed in\""
            ),
            Self::NoExitPlan { library } => write!(
                f,
                "`{library}` has no exit plan in docs/dependencies.md: say what Solis does \
                 if it goes bad"
            ),
            Self::Duplicate { library } => {
                write!(f, "`{library}` is listed twice in docs/dependencies.md")
            }
        }
    }
}

/// The dependency tables of a manifest, with their kinds: the top-level
/// ones and those under `[target.'cfg(...)'.*]`.
fn dependency_tables(doc: &toml::Table) -> Vec<(&toml::Table, Kind)> {
    let sections = [
        ("dependencies", Kind::Normal),
        ("build-dependencies", Kind::Build),
        ("dev-dependencies", Kind::Tests),
    ];
    let mut places = vec![doc];
    if let Some(targets) = doc.get("target").and_then(toml::Value::as_table) {
        places.extend(targets.values().filter_map(toml::Value::as_table));
    }
    let mut tables = Vec::new();
    for place in places {
        for (section, kind) in sections {
            if let Some(deps) = place.get(section).and_then(toml::Value::as_table) {
                tables.push((deps, kind));
            }
        }
    }
    tables
}

/// The outside libraries one crate's `Cargo.toml` uses. `internal` lists
/// Solis's own crates, which aren't outside libraries. A dependency
/// renamed with `package = "..."` counts under its real name.
pub fn library_uses(manifest: &str, internal: &[String]) -> Result<Vec<Use>, String> {
    let doc: toml::Table = manifest.parse().map_err(|e| format!("{e}"))?;
    let krate = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| "no package name".to_string())?
        .to_string();
    let mut uses = Vec::new();
    for (deps, kind) in dependency_tables(&doc) {
        for (name, spec) in deps {
            let by_path = spec
                .get("path")
                .and_then(toml::Value::as_str)
                .is_some_and(|p| p.contains("crates/"));
            if internal.contains(name) || by_path {
                continue;
            }
            let library = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(name)
                .to_string();
            let u = Use {
                krate: krate.clone(),
                library,
                kind,
            };
            if !uses.contains(&u) {
                uses.push(u);
            }
        }
    }
    uses.sort();
    Ok(uses)
}

/// Reads the register's table from `docs/dependencies.md`: the rows under
/// `## The register` whose first cell is a library name in backticks.
pub fn parse_register(markdown: &str) -> Result<Vec<Entry>, String> {
    let start = markdown
        .find("## The register")
        .ok_or_else(|| "no `## The register` section".to_string())?;
    let section = &markdown[start..];
    let section = section[1..]
        .find("\n## ")
        .map_or(section, |end| &section[..=end]);
    let mut entries = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() != 6 {
            return Err(format!(
                "a register row needs 6 cells (library, what it does, licence, allowed in, \
                 reach, exit plan), this one has {}: {line}",
                cells.len()
            ));
        }
        let library = cells[0].trim_matches('`').to_string();
        if library.is_empty() || library.contains('`') {
            return Err(format!("a register row has no library name: {line}"));
        }
        let mut allowed = Vec::new();
        for item in cells[3].split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (krate, kind) = if let Some(k) = item.strip_suffix("(tests)") {
                (k.trim(), Kind::Tests)
            } else if let Some(k) = item.strip_suffix("(build)") {
                (k.trim(), Kind::Build)
            } else {
                (item, Kind::Normal)
            };
            if krate.is_empty() || krate.contains(' ') {
                return Err(format!("`{library}`: \"{item}\" isn't a crate name"));
            }
            allowed.push((krate.to_string(), kind));
        }
        let plan = cells[5];
        entries.push(Entry {
            library,
            allowed,
            has_exit_plan: !plan.is_empty() && plan != "—" && plan != "-",
        });
    }
    if entries.is_empty() {
        return Err("the register has no rows".to_string());
    }
    Ok(entries)
}

/// Whether `entry` allows `u`. A crate allowed a library in the program
/// may also use it in its tests.
fn allows(entry: &Entry, u: &Use) -> bool {
    entry.allowed.iter().any(|(krate, kind)| {
        krate == &u.krate && (*kind == u.kind || (u.kind == Kind::Tests && *kind == Kind::Normal))
    })
}

/// Compares the crates' uses with the register.
pub fn check(uses: &[Use], register: &[Entry]) -> Vec<Problem> {
    let mut found = Vec::new();
    for (i, entry) in register.iter().enumerate() {
        if register[..i].iter().any(|e| e.library == entry.library) {
            found.push(Problem::Duplicate {
                library: entry.library.clone(),
            });
        }
        if !entry.has_exit_plan {
            found.push(Problem::NoExitPlan {
                library: entry.library.clone(),
            });
        }
    }
    for u in uses {
        match register.iter().find(|e| e.library == u.library) {
            None => found.push(Problem::Unlisted {
                library: u.library.clone(),
                krate: u.krate.clone(),
            }),
            Some(entry) if !allows(entry, u) => found.push(Problem::NotAllowedHere {
                library: u.library.clone(),
                krate: u.krate.clone(),
                kind: u.kind,
            }),
            Some(_) => {}
        }
    }
    for entry in register {
        if !uses.iter().any(|u| u.library == entry.library) {
            found.push(Problem::Unused {
                library: entry.library.clone(),
            });
            continue;
        }
        for (krate, kind) in &entry.allowed {
            let used = uses.iter().any(|u| {
                u.library == entry.library
                    && &u.krate == krate
                    && (u.kind == *kind || (*kind == Kind::Tests && u.kind == Kind::Normal))
            });
            if !used {
                found.push(Problem::StaleAllowance {
                    library: entry.library.clone(),
                    krate: krate.clone(),
                    kind: *kind,
                });
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn internal() -> Vec<String> {
        vec!["assets".to_string(), "reflect".to_string()]
    }

    fn use_of(krate: &str, library: &str, kind: Kind) -> Use {
        Use {
            krate: krate.into(),
            library: library.into(),
            kind,
        }
    }

    fn entry(library: &str, allowed: &[(&str, Kind)]) -> Entry {
        Entry {
            library: library.into(),
            allowed: allowed.iter().map(|(k, w)| ((*k).into(), *w)).collect(),
            has_exit_plan: true,
        }
    }

    const MANIFEST: &str = r#"
        [package]
        name = "render"
        [dependencies]
        wgpu.workspace = true
        assets.workspace = true
        own = { path = "../crates/own" }
        fast = { package = "real-name", version = "1" }
        [dev-dependencies]
        pollster.workspace = true
        [build-dependencies]
        winresource = "0.1"
        [target.'cfg(windows)'.dependencies]
        gilrs.workspace = true
    "#;

    #[test]
    fn reads_every_kind_of_dependency_and_skips_solis_s_own() {
        let uses = library_uses(MANIFEST, &internal()).unwrap();
        assert_eq!(
            uses,
            vec![
                use_of("render", "gilrs", Kind::Normal),
                use_of("render", "pollster", Kind::Tests),
                use_of("render", "real-name", Kind::Normal),
                use_of("render", "wgpu", Kind::Normal),
                use_of("render", "winresource", Kind::Build),
            ]
        );
    }

    #[test]
    fn a_manifest_without_a_name_or_that_isn_t_toml_is_an_error() {
        assert!(library_uses("[dependencies]\nx = \"1\"", &[]).is_err());
        assert!(library_uses("this is [not toml", &[]).is_err());
    }

    const REGISTER: &str = "\
# Dependencies

| Library | What | Licence | Allowed in | Reach | Exit plan |
| --- | --- | --- | --- | --- | --- |
| `not-this` | outside the register section | MIT | app | walled | none |

## The register

| Library | What | Licence | Allowed in | Reach | Exit plan |
| --- | --- | --- | --- | --- | --- |
| `wgpu` | graphics | MIT | render | foundation | fork |
| `pollster` | waits | MIT | render (tests), app (tests) | tests | write it |
| `winresource` | icon | MIT | app (build) | build | — |

## Building from the archive

| `after` | also outside | MIT | app | walled | none |
";

    #[test]
    fn reads_the_register_rows_only() {
        let reg = parse_register(REGISTER).unwrap();
        let names: Vec<&str> = reg.iter().map(|e| e.library.as_str()).collect();
        assert_eq!(names, ["wgpu", "pollster", "winresource"]);
        assert_eq!(
            reg[1].allowed,
            vec![
                ("render".to_string(), Kind::Tests),
                ("app".to_string(), Kind::Tests)
            ]
        );
        assert_eq!(reg[2].allowed, vec![("app".to_string(), Kind::Build)]);
        assert!(reg[0].has_exit_plan);
        assert!(!reg[2].has_exit_plan);
    }

    #[test]
    fn a_broken_register_is_an_error_not_a_pass() {
        assert!(parse_register("# no section here").is_err());
        assert!(parse_register("## The register\n\nnothing\n").is_err());
        assert!(parse_register("## The register\n| `x` | too | few |\n").is_err());
        assert!(parse_register("## The register\n| `x` | a | b | two words | e | f |\n").is_err());
        assert!(parse_register("## The register\n| `` | a | b | app | e | f |\n").is_err());
    }

    #[test]
    fn a_matching_register_has_no_problems() {
        let uses = vec![
            use_of("render", "wgpu", Kind::Normal),
            use_of("render", "wgpu", Kind::Tests),
        ];
        assert!(check(&uses, &[entry("wgpu", &[("render", Kind::Normal)])]).is_empty());
    }

    #[test]
    fn an_unlisted_library_is_reported() {
        let uses = vec![use_of("render", "newlib", Kind::Normal)];
        assert_eq!(
            check(&uses, &[]),
            vec![Problem::Unlisted {
                library: "newlib".into(),
                krate: "render".into()
            }]
        );
    }

    #[test]
    fn a_library_outside_its_wall_is_reported() {
        let uses = vec![
            use_of("render", "wgpu", Kind::Normal),
            use_of("editor", "wgpu", Kind::Normal),
        ];
        let found = check(&uses, &[entry("wgpu", &[("render", Kind::Normal)])]);
        assert_eq!(
            found,
            vec![Problem::NotAllowedHere {
                library: "wgpu".into(),
                krate: "editor".into(),
                kind: Kind::Normal
            }]
        );
    }

    #[test]
    fn tests_only_doesn_t_allow_the_program_or_the_build() {
        let reg = [entry("pollster", &[("render", Kind::Tests)])];
        for kind in [Kind::Normal, Kind::Build] {
            let uses = vec![use_of("render", "pollster", kind)];
            assert!(
                check(&uses, &reg)
                    .iter()
                    .any(|p| matches!(p, Problem::NotAllowedHere { .. })),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn unused_rows_and_stale_allowances_are_reported() {
        let uses = vec![use_of("render", "wgpu", Kind::Normal)];
        let reg = [
            entry(
                "wgpu",
                &[("render", Kind::Normal), ("editor", Kind::Normal)],
            ),
            entry("gone", &[("app", Kind::Normal)]),
        ];
        assert_eq!(
            check(&uses, &reg),
            vec![
                Problem::StaleAllowance {
                    library: "wgpu".into(),
                    krate: "editor".into(),
                    kind: Kind::Normal
                },
                Problem::Unused {
                    library: "gone".into()
                },
            ]
        );
    }

    #[test]
    fn duplicates_and_missing_exit_plans_are_reported() {
        let uses = vec![use_of("app", "x", Kind::Normal)];
        let mut no_plan = entry("x", &[("app", Kind::Normal)]);
        no_plan.has_exit_plan = false;
        let found = check(&uses, &[entry("x", &[("app", Kind::Normal)]), no_plan]);
        assert!(found.contains(&Problem::Duplicate {
            library: "x".into()
        }));
        assert!(found.contains(&Problem::NoExitPlan {
            library: "x".into()
        }));
    }

    #[test]
    fn every_problem_says_what_to_do() {
        let problems = [
            Problem::Unlisted {
                library: "a".into(),
                krate: "b".into(),
            },
            Problem::NotAllowedHere {
                library: "a".into(),
                krate: "b".into(),
                kind: Kind::Build,
            },
            Problem::Unused {
                library: "a".into(),
            },
            Problem::StaleAllowance {
                library: "a".into(),
                krate: "b".into(),
                kind: Kind::Tests,
            },
            Problem::NoExitPlan {
                library: "a".into(),
            },
            Problem::Duplicate {
                library: "a".into(),
            },
        ];
        for p in problems {
            let text = p.to_string();
            assert!(text.contains("docs/dependencies.md"), "{text}");
            assert!(text.contains('a'), "{text}");
        }
    }
}
