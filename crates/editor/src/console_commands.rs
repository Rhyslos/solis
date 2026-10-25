//! The console's commands: typed in the command line under the messages,
//! run with the same editor commands as the menus (so every change is an
//! undo step), answered in the console. Names are found without caring
//! about capitals; a name with spaces goes in quotes:
//! `select "Point light 1"`.

use crate::console::LogLevel;
use crate::inspector::{degrees_of, rotation_of};
use crate::look::LookKind;
use crate::properties::PropertyRequest;
use crate::viewport::Tool;
use crate::workbench::{Workbench, WorkbenchAction};
use reflect::{EntityId, FieldInfo, FieldKind, Value};

/// One command: its name, how it's typed, and what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandInfo {
    pub(crate) name: &'static str,
    pub(crate) usage: &'static str,
    pub(crate) help: &'static str,
}

/// Every command, in the order `help` lists them.
pub(crate) const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "help",
        usage: "help [command]",
        help: "Lists the commands, or says how one is used.",
    },
    CommandInfo {
        name: "clear",
        usage: "clear",
        help: "Empties the console.",
    },
    CommandInfo {
        name: "select",
        usage: "select <name or ID>… | all | none",
        help: "Selects entities by name (every one with that name) or ID.",
    },
    CommandInfo {
        name: "find",
        usage: "find <text>",
        help: "Lists the entities whose names contain the text.",
    },
    CommandInfo {
        name: "frame",
        usage: "frame",
        help: "Points the camera at the selection (F).",
    },
    CommandInfo {
        name: "new",
        usage: "new [name]",
        help: "Adds an empty entity where the camera looks, and selects it.",
    },
    CommandInfo {
        name: "delete",
        usage: "delete",
        help: "Deletes the selection and everything under it.",
    },
    CommandInfo {
        name: "duplicate",
        usage: "duplicate",
        help: "Duplicates the selection (Ctrl+D).",
    },
    CommandInfo {
        name: "get",
        usage: "get <Component>[.field]",
        help: "Shows a component's settings on the selected entity.",
    },
    CommandInfo {
        name: "set",
        usage: "set <Component.field> <value>",
        help: "Changes a setting on every selected entity: set Transform.position 0 0 2, \
               set Light.color #ffaa33.",
    },
    CommandInfo {
        name: "undo",
        usage: "undo",
        help: "Undoes the last change (Ctrl+Z).",
    },
    CommandInfo {
        name: "redo",
        usage: "redo",
        help: "Redoes the last change undone (Ctrl+Y).",
    },
    CommandInfo {
        name: "save",
        usage: "save",
        help: "Saves the area (Ctrl+S).",
    },
    CommandInfo {
        name: "look",
        usage: "look standard | stylized | unlit",
        help: "Chooses how the viewport shades models.",
    },
    CommandInfo {
        name: "tool",
        usage: "tool select | move | rotate | scale | transform",
        help: "Chooses a tool (Q, W, R, S, T).",
    },
    CommandInfo {
        name: "play",
        usage: "play",
        help: "Runs the area as a game (the Play button), or carries on after pause.",
    },
    CommandInfo {
        name: "pause",
        usage: "pause",
        help: "Stops the game's clock, or starts it again.",
    },
    CommandInfo {
        name: "seed",
        usage: "seed [number | new]",
        help: "Says the running game's seed (what its random numbers come from); with a number, every Play starts from that seed, so a play can be repeated; `seed new` gives each Play a new one again.",
    },
    CommandInfo {
        name: "checksum",
        usage: "checksum",
        help: "While playing, the game's checksums at this tick: one number for the world and one for game code's values. Two plays that agree on them at the same tick are in exactly the same state.",
    },
    CommandInfo {
        name: "step",
        usage: "step",
        help: "While paused, runs exactly one tick (the Step button).",
    },
    CommandInfo {
        name: "stop",
        usage: "stop",
        help: "Stops the game: the area goes back to how it was before Play.",
    },
    CommandInfo {
        name: "stats",
        usage: "stats",
        help: "Counts what's in the area, and how fast the editor draws.",
    },
];

/// The most words a command line is split into (more are ignored).
const MAX_WORDS: usize = 64;
/// The most entities `find` lists.
const MAX_FOUND: usize = 20;

/// Splits a command line into words; words in "quotes" stay together.
pub(crate) fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut current));
                    started = false;
                    if out.len() >= MAX_WORDS {
                        return out;
                    }
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started && out.len() < MAX_WORDS {
        out.push(current);
    }
    out
}

/// What Tab makes of a command line: the line completed as far as it's
/// certain, and the choices when there are several.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Completion {
    pub(crate) line: String,
    pub(crate) choices: Vec<String>,
}

/// The longest start every one of `words` shares.
fn shared_start(words: &[String]) -> String {
    let Some(first) = words.first() else {
        return String::new();
    };
    let mut length = first.len();
    for w in &words[1..] {
        length = first
            .char_indices()
            .zip(w.chars())
            .take_while(|((_, a), b)| a.eq_ignore_ascii_case(b))
            .last()
            .map_or(0, |((i, a), _)| (i + a.len_utf8()).min(length));
    }
    first[..length].to_string()
}

/// A word as it's typed back into the command line: quoted if it has a
/// space.
fn typed(word: &str) -> String {
    if word.contains(char::is_whitespace) {
        format!("\"{word}\"")
    } else {
        word.to_string()
    }
}

impl Workbench {
    /// Runs a command line typed in the console, saying what it did (or
    /// why it couldn't) there.
    pub(crate) fn run_command(
        &mut self,
        line: &str,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.log(LogLevel::Info, format!("› {line}"));
        let words = words(line);
        let (name, args) = match words.split_first() {
            Some((name, args)) => (name.to_lowercase(), args),
            None => return,
        };
        let result = match name.as_str() {
            "help" => self.help(args),
            "clear" => {
                self.console.clear();
                Ok(String::new())
            }
            "select" => self.select_named(args),
            "find" => self.find_named(args),
            "frame" => {
                self.frame_selection();
                Ok(String::new())
            }
            "new" => {
                let id = self.add_entity();
                let name = args.join(" ");
                if !name.trim().is_empty() {
                    self.apply_area_request(crate::area_panel::AreaRequest::Rename {
                        entity: id,
                        name: name.trim().to_string(),
                    });
                }
                Ok(String::new())
            }
            "delete" => self.with_selection(Self::delete_selection),
            "duplicate" => self.with_selection(Self::duplicate_selection),
            "get" => self.get_field(args),
            "set" => self.set_field(args, ctx),
            "undo" => {
                self.undo();
                Ok(String::new())
            }
            "redo" => {
                self.redo();
                Ok(String::new())
            }
            "save" => {
                actions.push(WorkbenchAction::SaveArea);
                Ok(String::new())
            }
            "look" => self.choose_look(args),
            "tool" => self.choose_tool(args),
            "play" => {
                self.play();
                Ok(String::new())
            }
            "pause" => {
                self.pause();
                Ok(String::new())
            }
            "step" => {
                self.step();
                Ok(String::new())
            }
            "stop" => {
                self.stop();
                Ok(String::new())
            }
            "stats" => Ok(self.stats()),
            "seed" => self.seed_command(args),
            "checksum" => self.checksum_command(),
            _ => Err(format!(
                "there's no command \"{name}\"; type help to see them all"
            )),
        };
        match result {
            Ok(text) => {
                // A long answer, one line each, so none is cut short.
                for line in text.lines() {
                    self.log(LogLevel::Info, line.to_string());
                }
            }
            Err(why) => self.log(LogLevel::Error, why),
        }
    }

    fn with_selection(&mut self, run: fn(&mut Self)) -> Result<String, String> {
        if self.document.selection.primary().is_none() {
            return Err("nothing is selected".into());
        }
        run(self);
        Ok(String::new())
    }

    fn help(&self, args: &[String]) -> Result<String, String> {
        if let Some(name) = args.first() {
            let command = COMMANDS
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("there's no command \"{name}\""))?;
            return Ok(format!("{} — {}", command.usage, command.help));
        }
        let mut text = String::from("Commands:\n");
        for command in COMMANDS {
            text.push_str(&format!("  {}\n", command.usage));
        }
        text.push_str(
            "Type help and a command's name for more. Tab completes names; Up and Down \
             bring back earlier commands.",
        );
        Ok(text)
    }

    /// The entities `word` names: every one with that name (any capitals),
    /// or the one with that ID.
    fn named(&self, word: &str) -> Vec<EntityId> {
        let area = self.document.area();
        let by_name: Vec<EntityId> = area
            .iter()
            .filter(|(_, e)| e.name.eq_ignore_ascii_case(word))
            .map(|(id, _)| id)
            .collect();
        if !by_name.is_empty() {
            return by_name;
        }
        word.parse::<EntityId>()
            .ok()
            .filter(|id| area.contains(*id))
            .into_iter()
            .collect()
    }

    fn select_named(&mut self, args: &[String]) -> Result<String, String> {
        match args.first().map(|a| a.to_lowercase()).as_deref() {
            None => return Err("select what? Give a name, an ID, all or none".into()),
            Some("all") => {
                self.select_all();
                return Ok(String::new());
            }
            Some("none") => {
                self.document.selection.clear();
                return Ok(String::new());
            }
            _ => {}
        }
        let mut found = Vec::new();
        for word in args {
            let named = self.named(word);
            if named.is_empty() {
                return Err(format!("nothing is called \"{word}\""));
            }
            found.extend(named);
        }
        let first = found[0];
        self.document.selection.select_all(found.clone(), first);
        self.area_panel.reveal = Some(first);
        let s = if found.len() == 1 { "y" } else { "ies" };
        Ok(format!("Selected {} entit{s}.", found.len()))
    }

    fn find_named(&self, args: &[String]) -> Result<String, String> {
        let text = args.join(" ").to_lowercase();
        if text.is_empty() {
            return Err("find what? Give part of a name".into());
        }
        let found: Vec<String> = self
            .document
            .area()
            .iter()
            .filter(|(_, e)| e.name.to_lowercase().contains(&text))
            .map(|(id, e)| format!("{} ({id})", e.name))
            .collect();
        if found.is_empty() {
            return Ok(format!("Nothing's name contains \"{text}\"."));
        }
        let more = found.len().saturating_sub(MAX_FOUND);
        let mut text = found
            .into_iter()
            .take(MAX_FOUND)
            .collect::<Vec<_>>()
            .join("\n");
        if more > 0 {
            text.push_str(&format!("\n…and {more} more"));
        }
        Ok(text)
    }

    /// A component's schema and one of its fields, from "Component.field"
    /// (any capitals).
    fn field_named(&self, path: &str) -> Result<(String, Option<FieldInfo>), String> {
        let (component, field) = match path.split_once('.') {
            Some((c, f)) => (c, Some(f)),
            None => (path, None),
        };
        let schema = self
            .document
            .registry
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(component))
            .ok_or_else(|| format!("there's no component called \"{component}\""))?;
        let field = match field {
            None => None,
            Some(f) => Some(
                schema
                    .fields
                    .iter()
                    .find(|i| i.name.eq_ignore_ascii_case(f))
                    .cloned()
                    .ok_or_else(|| {
                        let names: Vec<&str> =
                            schema.fields.iter().map(|i| i.name.as_str()).collect();
                        format!(
                            "{} has no setting \"{f}\"; it has {}",
                            schema.name,
                            names.join(", ")
                        )
                    })?,
            ),
        };
        Ok((schema.name.clone(), field))
    }

    fn get_field(&self, args: &[String]) -> Result<String, String> {
        let path = args
            .first()
            .ok_or("get what? Give Component or Component.field")?;
        let (component, field) = self.field_named(path)?;
        let id = self
            .document
            .selection
            .primary()
            .ok_or("nothing is selected")?;
        let entity = self.document.area().get(id).ok_or("nothing is selected")?;
        let data = entity
            .components
            .get(&component)
            .ok_or_else(|| format!("{} has no {component}", entity.name))?;
        let schema = self
            .document
            .registry
            .get(&component)
            .ok_or("an unknown component")?;
        let fields: Vec<&FieldInfo> = match &field {
            Some(f) => vec![f],
            None => schema.fields.iter().collect(),
        };
        let shown: Vec<String> = fields
            .iter()
            .map(|f| {
                let value = data.get(&f.name).unwrap_or(&f.default);
                format!("{component}.{} = {}", f.name, shown_value(f, value))
            })
            .collect();
        Ok(shown.join("\n"))
    }

    fn set_field(&mut self, args: &[String], _ctx: &egui::Context) -> Result<String, String> {
        let (path, value) = args
            .split_first()
            .ok_or("set what? Give Component.field and a value")?;
        let (component, field) = self.field_named(path)?;
        let field = field.ok_or("set which setting? Give Component.field")?;
        if value.is_empty() {
            return Err(format!("set {component}.{} to what?", field.name));
        }
        let value = parse_value(&field, value)?;
        let selected: Vec<EntityId> = self.document.selection.iter().collect();
        if selected.is_empty() {
            return Err("nothing is selected".into());
        }
        let with: Vec<EntityId> = selected
            .into_iter()
            .filter(|&e| {
                self.document
                    .area()
                    .get(e)
                    .is_some_and(|d| d.has(&component))
            })
            .collect();
        if with.is_empty() {
            return Err(format!("nothing selected has a {component}"));
        }
        // A new gesture: typed changes never merge with each other.
        self.gesture += 1;
        let gesture = self.gesture;
        self.apply_property(
            PropertyRequest::SetFieldMany {
                entities: with,
                component,
                field: field.name.clone(),
                value,
                part: None,
            },
            gesture,
        );
        Ok(String::new())
    }

    fn choose_look(&mut self, args: &[String]) -> Result<String, String> {
        let wanted = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
        let kind = LookKind::ALL
            .into_iter()
            .find(|k| k.name().to_lowercase() == wanted)
            .ok_or("look standard, stylized or unlit?")?;
        self.viewport.look.kind = kind;
        Ok(String::new())
    }

    fn choose_tool(&mut self, args: &[String]) -> Result<String, String> {
        let wanted = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
        let tool = Tool::ALL
            .into_iter()
            .find(|t| t.name().to_lowercase() == wanted)
            .ok_or("tool select, move, rotate, scale or transform?")?;
        self.viewport.tool = tool;
        Ok(String::new())
    }

    /// `checksum`: the running game's checksums at this tick.
    fn checksum_command(&mut self) -> Result<String, String> {
        let Some(playing) = &mut self.playing else {
            return Err("nothing is running: press Play first".to_string());
        };
        let tick = playing.world.clock().ticks;
        let world = playing.world.checksum();
        let code = playing.game.as_mut().map_or(0, script::Game::checksum);
        Ok(format!(
            "At tick {tick}: world {world:016x}, game code {code:016x}."
        ))
    }

    /// `seed`, `seed 1234` or `seed new`.
    fn seed_command(&mut self, args: &[String]) -> Result<String, String> {
        match args {
            [] => {
                let running = self.playing.as_ref().map(|p| p.world.seed());
                Ok(match (running, self.fixed_seed) {
                    (Some(seed), Some(_)) => {
                        format!("The game's seed is {seed}, chosen for every Play.")
                    }
                    (Some(seed), None) => format!(
                        "The game's seed is {seed}. Type seed {seed} to start every Play from it."
                    ),
                    (None, Some(seed)) => format!("Every Play starts from seed {seed}."),
                    (None, None) => "Each Play has a new seed.".to_string(),
                })
            }
            [word] if word == "new" => {
                self.fixed_seed = None;
                Ok("Each Play has a new seed from now on.".to_string())
            }
            [number] => match number.parse::<u64>() {
                Ok(seed) if seed < 1_000_000_000_000_000 => {
                    self.fixed_seed = Some(seed);
                    Ok(format!("Every Play starts from seed {seed} from now on."))
                }
                _ => Err(format!(
                    "\"{number}\" isn't a seed: give a whole number from 0 to 999999999999999, or new"
                )),
            },
            _ => Err("seed takes one number, or new".to_string()),
        }
    }

    fn stats(&self) -> String {
        let area = self.document.area();
        let models = area.with::<area::builtin::Model>().count();
        let lights = area.with::<area::builtin::Light>().count();
        let lights = if lights == 1 {
            "1 light".to_string()
        } else {
            format!("{lights} lights")
        };
        format!(
            "{} entities ({models} with a model, {lights}), {} triangles, {} selected.\n\
             {:.0} fps ({:.1} ms a frame); {} undo steps.",
            crate::models::grouped(area.len()),
            crate::models::grouped(self.viewport.area.triangles),
            self.document.selection.iter().count(),
            self.viewport.frames_per_second(),
            1000.0 / self.viewport.frames_per_second().max(1e-3),
            self.document.history().undo_len(),
        )
    }

    /// What Tab makes of `line`: a command's name, an entity's name after
    /// select, a component's setting after get or set.
    pub(crate) fn complete(&self, line: &str) -> Completion {
        let words = words(line);
        let new_word = line.ends_with(char::is_whitespace) || line.is_empty();
        let (done, partial): (&[String], &str) = if new_word {
            (&words, "")
        } else {
            match words.split_last() {
                Some((last, rest)) => (rest, last.as_str()),
                None => (&[], ""),
            }
        };
        let partial_lower = partial.to_lowercase();
        let candidates: Vec<String> = match done.first().map(|w| w.to_lowercase()).as_deref() {
            None => COMMANDS.iter().map(|c| c.name.to_string()).collect(),
            Some("select") => {
                let mut names: Vec<String> = self
                    .document
                    .area()
                    .iter()
                    .map(|(_, e)| e.name.clone())
                    .chain(["all".to_string(), "none".to_string()])
                    .collect();
                names.sort_unstable();
                names.dedup();
                names
            }
            Some("get" | "set") if done.len() == 1 => self
                .document
                .registry
                .iter()
                .flat_map(|s| {
                    std::iter::once(s.name.clone()).chain(
                        s.fields
                            .iter()
                            .map(move |f| format!("{}.{}", s.name, f.name)),
                    )
                })
                .collect(),
            Some("look") => LookKind::ALL
                .iter()
                .map(|k| k.name().to_lowercase())
                .collect(),
            Some("tool") => Tool::ALL.iter().map(|t| t.name().to_lowercase()).collect(),
            Some("help") => COMMANDS.iter().map(|c| c.name.to_string()).collect(),
            _ => Vec::new(),
        };
        let matching: Vec<String> = candidates
            .into_iter()
            .filter(|c| c.to_lowercase().starts_with(&partial_lower))
            .collect();
        let before: Vec<String> = done.iter().map(|w| typed(w)).collect();
        let join = |last: &str, finished: bool| {
            let mut parts = before.clone();
            parts.push(typed(last));
            let mut line = parts.join(" ");
            if finished {
                line.push(' ');
            }
            line
        };
        match matching.as_slice() {
            [] => Completion {
                line: line.to_string(),
                choices: Vec::new(),
            },
            [one] => Completion {
                line: join(one, true),
                choices: Vec::new(),
            },
            many => {
                let start = shared_start(many);
                let line = if start.len() > partial.len() {
                    join(&start, false)
                } else {
                    line.to_string()
                };
                Completion {
                    line,
                    choices: many.to_vec(),
                }
            }
        }
    }
}

/// A value as `get` shows it.
fn shown_value(field: &FieldInfo, value: &Value) -> String {
    let numbers = |v: &[f64], unit: &str| {
        v.iter()
            // No "-0.000".
            .map(|&n| format!("{:.3}", if n.abs() < 0.0005 { 0.0 } else { n }))
            .collect::<Vec<_>>()
            .join(" ")
            + unit
    };
    match (&field.kind, value) {
        (FieldKind::Vec3, v) => v.as_floats::<3>().map_or_else(
            || "?".into(),
            |f| numbers(&f.map(f64::from), field.hint.unit),
        ),
        (FieldKind::Rotation, v) => numbers(&degrees_of(v), "°"),
        (FieldKind::Color, v) => v.as_floats::<3>().map_or_else(
            || "?".into(),
            |c| {
                let [r, g, b] = c.map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8);
                format!("#{r:02x}{g:02x}{b:02x}")
            },
        ),
        (_, Value::Bool(b)) => b.to_string(),
        (_, Value::Int(n)) => format!("{n}{}", field.hint.unit),
        (_, Value::Float(n)) => format!("{n:.3}{}", field.hint.unit),
        (_, Value::Text(t)) => typed(t),
        (_, Value::Asset(Some(r))) => r.asset.to_string(),
        (_, Value::Entity(Some(e))) => e.to_string(),
        (_, Value::Asset(None) | Value::Entity(None)) => "none".into(),
        (_, Value::List(_)) => "a list".into(),
    }
}

/// Reads a typed value for `field`: numbers, yes or no, a choice, three
/// numbers ("0 0 2" or "0,0,2"), degrees for a rotation, a colour as
/// "#ff8800" or three numbers from 0 to 1.
pub(crate) fn parse_value(field: &FieldInfo, words: &[String]) -> Result<Value, String> {
    let all = words.join(" ");
    let number = |w: &str| -> Result<f64, String> {
        let trimmed = w
            .trim()
            .trim_end_matches(field.hint.unit.trim())
            .trim_end_matches(['m', '°']);
        trimmed
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .ok_or_else(|| format!("\"{w}\" isn't a number"))
    };
    let three = || -> Result<[f64; 3], String> {
        let parts: Vec<&str> = all
            .split([' ', ','])
            .filter(|p| !p.trim().is_empty())
            .collect();
        match parts.as_slice() {
            [x, y, z] => Ok([number(x)?, number(y)?, number(z)?]),
            _ => Err(format!(
                "{} takes three numbers, such as 0 0 2",
                field.label
            )),
        }
    };
    match &field.kind {
        FieldKind::Bool => match all.to_lowercase().as_str() {
            "true" | "yes" | "on" | "1" => Ok(Value::Bool(true)),
            "false" | "no" | "off" | "0" => Ok(Value::Bool(false)),
            _ => Err(format!("{} is on or off", field.label)),
        },
        FieldKind::Int => all
            .trim()
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| format!("\"{all}\" isn't a whole number")),
        FieldKind::Float => number(&all).map(Value::Float),
        FieldKind::Text => Ok(Value::Text(all)),
        FieldKind::Choice(options) => options
            .iter()
            .find(|o| o.eq_ignore_ascii_case(all.trim()))
            .map(|o| Value::Text(o.clone()))
            .ok_or_else(|| format!("{} is one of: {}", field.label, options.join(", "))),
        FieldKind::Vec3 => three().map(|v| Value::floats(&v.map(|c| c as f32))),
        FieldKind::Rotation => three().map(rotation_of),
        FieldKind::Color => {
            let hex = all.trim().trim_start_matches('#');
            if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
                return Ok(Value::floats(
                    &[0, 2, 4].map(|i| f32::from(byte(i)) / 255.0),
                ));
            }
            let c = three()?;
            if c.iter().all(|v| (0.0..=1.0).contains(v)) {
                Ok(Value::floats(&c.map(|v| v as f32)))
            } else {
                Err("a colour is #rrggbb, or three numbers from 0 to 1".into())
            }
        }
        FieldKind::Asset | FieldKind::Entity => Err(format!(
            "{} can't be typed yet; set it in Properties",
            field.label
        )),
    }
}

#[cfg(test)]
#[path = "console_commands_tests.rs"]
mod tests;
