//! The project's buses (Phase 7, S4; decision 0056, question 4): groups of
//! sounds that share a volume, in a tree under **Master**, and ducking
//! rules ("while Voice plays, Music goes down 8 dB").
//!
//! The buses are one file per project, `mixer.solmix`, in the same Lust
//! style as sound events, one line per bus so Git merges it cleanly:
//!
//! ```text
//! -- The project's buses. Safe to edit by hand: one line per bus.
//! -- Each bus: [name, the bus it's inside, volume in dB, ducking rules].
//! -- Each rule: [while this bus plays, down by dB, fade down s, fade back s].
//! mixer version 1;
//!
//! bus1 = ["Master", "", 0.0, []];
//! bus2 = ["Music", "Master", 0.0, [["Voice", -8.0, 0.2, 1.0]]];
//! bus3 = ["Effects", "Master", 0.0, []];
//! voices = 64;
//! ```
//!
//! `voices` is how many sounds are heard at once (S5,
//! [`crate::voices`]): 16 to 256.
//!
//! Read as a hostile file: within limits, a value out of range is brought
//! into range (with a note), and anything that can't make a sensible tree
//! (no Master, a name twice, a bus inside one that comes later) is put
//! right with a note where it can be, or refused with its line.
//!
//! Mute and solo aren't saved: they're for listening while working.
//! Everything here is plain data and sums; [`crate::SoundDevice`] carries
//! it out on Kira's tracks.

use area::settings;
use reflect::Value;
use std::fmt;

/// The mixer file's name, in the project's folder.
pub const FILE_NAME: &str = "mixer.solmix";
/// The layout version this build writes and reads.
pub const VERSION: u32 = 1;
/// The word the file starts with.
const KIND: &str = "mixer";
/// This file's version and upgraders (Phase 8, C1): each step turns
/// fields read at one version into the next, before they're applied.
/// Every version ever released keeps an example file in `tests/fixtures/`.
pub static FORMAT: reflect::upgrade::Format<settings::Settings> = reflect::upgrade::Format {
    name: "mixer",
    current: VERSION,
    version_of: |text| reflect::upgrade::header_version(text, &[KIND]),
    steps: &[],
};
/// The biggest mixer file read.
pub const MAX_BYTES: usize = 256 << 10;
/// The most buses in a project.
pub const MAX_BUSES: usize = 64;
/// The most ducking rules on one bus.
pub const MAX_RULES: usize = 8;
/// The deepest buses nest, Master being 1.
pub const MAX_DEPTH: usize = 8;
/// The longest bus name.
pub const MAX_NAME: usize = 64;
/// The quietest and loudest a bus's fader goes, in decibels.
pub const VOLUME_RANGE_DB: (f32, f32) = (-60.0, 12.0);
/// The most a rule ducks by, in decibels (as a negative number).
pub const MOST_DUCKING_DB: f32 = -60.0;
/// The longest a ducking fade takes, in seconds.
pub const MAX_FADE_S: f32 = 10.0;
/// The bus at the top, which every other is inside.
pub const MASTER: &str = "Master";
/// Quieter than this counts as silent, in decibels.
pub const SILENT_DB: f32 = -60.0;

/// A ducking rule: while `while_bus` (or a bus inside it) plays, the bus
/// it's on goes down by `by_db`, over `down_s`, and back over `back_s`
/// after the last of those sounds ends.
#[derive(Debug, Clone, PartialEq)]
pub struct Ducking {
    /// The bus whose sounds make this one quieter.
    pub while_bus: String,
    /// How much quieter, in decibels (0 or less).
    pub by_db: f32,
    /// How long going down takes, in seconds.
    pub down_s: f32,
    /// How long coming back takes, in seconds.
    pub back_s: f32,
}

impl Default for Ducking {
    fn default() -> Self {
        Self {
            while_bus: "Voice".to_owned(),
            by_db: -8.0,
            down_s: 0.2,
            back_s: 1.0,
        }
    }
}

/// One bus.
#[derive(Debug, Clone, PartialEq)]
pub struct Bus {
    /// Its name, which sound events name.
    pub name: String,
    /// The bus it's inside, by name; empty for Master.
    pub inside: String,
    /// Its fader, in decibels: 0 as the sounds are, -6 about half as loud.
    pub volume_db: f32,
    /// Its ducking rules.
    pub ducking: Vec<Ducking>,
}

impl Bus {
    /// A bus inside `inside` at 0 dB, with no rules.
    pub fn new(name: &str, inside: &str) -> Self {
        Self {
            name: name.to_owned(),
            inside: inside.to_owned(),
            volume_db: 0.0,
            ducking: Vec::new(),
        }
    }
}

/// The project's buses, Master first, each after the bus it's inside,
/// and how many sounds are heard at once.
#[derive(Debug, Clone, PartialEq)]
pub struct Mixer {
    /// The buses.
    pub buses: Vec<Bus>,
    /// How many sounds are heard at once (the most important are).
    pub voices: u32,
}

impl Default for Mixer {
    /// The buses a new project has: Master, and under it Music, Effects,
    /// Voice, Ambience and UI, with Voice ducking Music.
    fn default() -> Self {
        let mut music = Bus::new("Music", MASTER);
        music.ducking.push(Ducking::default());
        Self {
            buses: vec![
                Bus::new(MASTER, ""),
                music,
                Bus::new("Effects", MASTER),
                Bus::new("Voice", MASTER),
                Bus::new("Ambience", MASTER),
                Bus::new("UI", MASTER),
            ],
            voices: crate::voices::STANDARD_VOICES,
        }
    }
}

impl Mixer {
    /// The bus called `name`, by its place in the list.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.buses.iter().position(|b| b.name == name)
    }

    /// The bus a sound event's `bus` plays through: the one by that name,
    /// or Master when there's none (a bus renamed or removed).
    pub fn bus_for(&self, name: &str) -> usize {
        self.index_of(name).unwrap_or(0)
    }

    /// The bus each bus is inside, by place (Master's is `None`).
    pub fn parents(&self) -> Vec<Option<usize>> {
        self.buses
            .iter()
            .map(|b| {
                if b.inside.is_empty() {
                    None
                } else {
                    self.index_of(&b.inside)
                }
            })
            .collect()
    }

    /// Whether bus `inner` is `outer` or inside it, however deep.
    pub fn is_within(&self, inner: usize, outer: usize) -> bool {
        let parents = self.parents();
        let mut at = Some(inner);
        for _ in 0..=MAX_DEPTH {
            match at {
                Some(i) if i == outer => return true,
                Some(i) => at = parents.get(i).copied().flatten(),
                None => return false,
            }
        }
        false
    }

    /// A name not yet used, from `stem` ("Bus", "Bus 2", ...).
    pub fn free_name(&self, stem: &str) -> String {
        if self.index_of(stem).is_none() {
            return stem.to_owned();
        }
        (2..)
            .map(|n| format!("{stem} {n}"))
            .find(|n| self.index_of(n).is_none())
            .unwrap_or_else(|| stem.to_owned())
    }

    /// Adds a bus inside `inside` (or Master), named from "Bus"; `None`
    /// when there's no room for another, or it would nest too deep.
    pub fn add_bus(&mut self, inside: &str) -> Option<usize> {
        if self.buses.len() >= MAX_BUSES {
            return None;
        }
        let parent = self.index_of(inside).unwrap_or(0);
        if self.depth(parent) >= MAX_DEPTH {
            return None;
        }
        let name = self.free_name("Bus");
        let inside = self.buses[parent].name.clone();
        // Placed after its parent's last descendant, so the list stays a
        // tree read top to bottom.
        let at = (parent + 1..self.buses.len())
            .take_while(|i| self.is_within(*i, parent))
            .last()
            .map_or(parent + 1, |i| i + 1);
        self.buses.insert(at, Bus::new(&name, &inside));
        Some(at)
    }

    /// Removes a bus (never Master): the buses inside it move up to its
    /// parent, and rules listening to it go.
    pub fn remove_bus(&mut self, at: usize) {
        if at == 0 || at >= self.buses.len() {
            return;
        }
        let gone = self.buses.remove(at);
        for bus in &mut self.buses {
            if bus.inside == gone.name {
                bus.inside.clone_from(&gone.inside);
            }
            bus.ducking.retain(|d| d.while_bus != gone.name);
        }
    }

    /// Renames a bus, and everything that names it (buses inside it and
    /// rules). Refused (`false`) for an empty, too long or taken name.
    pub fn rename_bus(&mut self, at: usize, to: &str) -> bool {
        let to = to.trim();
        if !good_name(to) || self.index_of(to).is_some_and(|i| i != at) || at >= self.buses.len() {
            return false;
        }
        let from = std::mem::replace(&mut self.buses[at].name, to.to_owned());
        for bus in &mut self.buses {
            if bus.inside == from {
                bus.inside = to.to_owned();
            }
            for rule in &mut bus.ducking {
                if rule.while_bus == from {
                    rule.while_bus = to.to_owned();
                }
            }
        }
        true
    }

    /// How deep a bus is: Master is 1.
    pub fn depth(&self, at: usize) -> usize {
        let parents = self.parents();
        let mut depth = 1;
        let mut here = parents.get(at).copied().flatten();
        while let Some(p) = here {
            depth += 1;
            if depth > MAX_DEPTH + 1 {
                break;
            }
            here = parents.get(p).copied().flatten();
        }
        depth
    }
}

/// Whether `name` can name a bus.
fn good_name(name: &str) -> bool {
    !name.is_empty() && name.chars().count() <= MAX_NAME && !name.contains(char::is_control)
}

/// Why a mixer file couldn't be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerError {
    /// The line it went wrong on (from 1), or 0 for the whole file.
    pub line: usize,
    /// What went wrong, in plain words.
    pub message: String,
}

impl fmt::Display for MixerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for MixerError {}

/// A mixer as read, with notes about what was put right.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadMixer {
    /// The buses.
    pub mixer: Mixer,
    /// What was skipped or put right, in plain words with lines.
    pub notes: Vec<String>,
}

/// Reads a `mixer.solmix` file.
pub fn read(text: &str) -> Result<ReadMixer, MixerError> {
    if text.len() > MAX_BYTES {
        return Err(MixerError {
            line: 0,
            message: format!(
                "the file is {} KB; mixers up to {} KB are read",
                text.len() >> 10,
                MAX_BYTES >> 10
            ),
        });
    }
    let mut file = settings::read_settings(text, KIND, VERSION).map_err(|e| MixerError {
        line: e.line,
        message: e.message,
    })?;
    let version = file.version;
    FORMAT.upgrade(version, &mut file).map_err(|e| MixerError {
        line: 1,
        message: e.to_string(),
    })?;
    let mut notes = Vec::new();
    let mut buses: Vec<(usize, Bus)> = Vec::new();
    let mut voices = crate::voices::STANDARD_VOICES;
    for field in &file.fields {
        let line = field.line;
        if field.key == "voices" {
            let Value::Int(n) = field.value else {
                return Err(MixerError {
                    line,
                    message: "voices should be a whole number".to_owned(),
                });
            };
            let (lo, hi) = crate::voices::VOICES_RANGE;
            let fitted = n.clamp(i64::from(lo), i64::from(hi));
            if fitted != n {
                notes.push(format!(
                    "line {line}: voices = {n} is outside {lo} to {hi}, so it's {fitted}"
                ));
            }
            voices = u32::try_from(fitted).unwrap_or(crate::voices::STANDARD_VOICES);
            continue;
        }
        if !field.key.starts_with("bus") {
            notes.push(format!(
                "line {line}: \"{}\" isn't a mixer setting, so it's skipped",
                field.key
            ));
            continue;
        }
        if buses.len() == MAX_BUSES {
            return Err(MixerError {
                line,
                message: format!("more than {MAX_BUSES} buses"),
            });
        }
        let bus = read_bus(&field.value, line, &mut notes)?;
        if buses.iter().any(|(_, b)| b.name == bus.name) {
            notes.push(format!(
                "line {line}: a second bus called \"{}\" is skipped",
                bus.name
            ));
            continue;
        }
        buses.push((line, bus));
    }
    let mut mixer = tidy(buses, &mut notes)?;
    mixer.voices = voices;
    Ok(ReadMixer { mixer, notes })
}

/// One bus line's list: name, inside, volume, rules.
fn read_bus(value: &Value, line: usize, notes: &mut Vec<String>) -> Result<Bus, MixerError> {
    let wrong = |what: &str| MixerError {
        line,
        message: format!("a bus should be {what}"),
    };
    let shape = "[name, the bus it's inside, volume in dB, [rules]]";
    let Value::List(items) = value else {
        return Err(wrong(shape));
    };
    let [name, inside, volume, rules] = items.as_slice() else {
        return Err(wrong(shape));
    };
    let (Value::Text(name), Value::Text(inside)) = (name, inside) else {
        return Err(wrong("named in quotes, with the bus it's inside in quotes"));
    };
    let name = name.trim();
    if !good_name(name) {
        return Err(wrong(&format!("named with 1 to {MAX_NAME} letters")));
    }
    let volume = volume
        .as_f32()
        .ok_or_else(|| wrong("given a volume as a number"))?;
    let mut bus = Bus::new(name, inside.trim());
    bus.volume_db = fitted(volume, VOLUME_RANGE_DB, "volume", line, notes);
    let Value::List(rules) = rules else {
        return Err(wrong("given its ducking rules as a list, [] for none"));
    };
    for rule in rules {
        if bus.ducking.len() == MAX_RULES {
            notes.push(format!(
                "line {line}: {name} has more than {MAX_RULES} ducking rules; the rest are skipped"
            ));
            break;
        }
        bus.ducking.push(read_rule(rule, line, notes)?);
    }
    Ok(bus)
}

/// One ducking rule: while, by, down, back.
fn read_rule(value: &Value, line: usize, notes: &mut Vec<String>) -> Result<Ducking, MixerError> {
    let wrong = || MixerError {
        line,
        message: "a ducking rule should be [\"bus\", dB, fade down s, fade back s]".to_owned(),
    };
    let Value::List(items) = value else {
        return Err(wrong());
    };
    let [Value::Text(while_bus), by, down, back] = items.as_slice() else {
        return Err(wrong());
    };
    let number = |v: &Value| v.as_f32().ok_or_else(wrong);
    Ok(Ducking {
        while_bus: while_bus.trim().to_owned(),
        by_db: fitted(number(by)?, (MOST_DUCKING_DB, 0.0), "ducking", line, notes),
        down_s: fitted(number(down)?, (0.0, MAX_FADE_S), "fade down", line, notes),
        back_s: fitted(number(back)?, (0.0, MAX_FADE_S), "fade back", line, notes),
    })
}

/// `v` within `range`, with a note when it wasn't.
fn fitted(v: f32, range: (f32, f32), what: &str, line: usize, notes: &mut Vec<String>) -> f32 {
    let f = v.clamp(range.0, range.1);
    if f != v {
        notes.push(format!(
            "line {line}: {what} {v} is outside {} to {}, so it's {f}",
            range.0, range.1
        ));
    }
    f
}

/// Makes the buses read into a sensible tree: Master first with nothing
/// above it, every other inside a bus that comes before it, not too deep,
/// and rules only listening to buses that exist (and not to themselves).
fn tidy(buses: Vec<(usize, Bus)>, notes: &mut Vec<String>) -> Result<Mixer, MixerError> {
    let Some(master_at) = buses.iter().position(|(_, b)| b.name == MASTER) else {
        return Err(MixerError {
            line: 0,
            message: format!("there's no bus called {MASTER}, which every other is inside"),
        });
    };
    let mut ordered: Vec<(usize, Bus)> = Vec::with_capacity(buses.len());
    let mut rest = buses;
    let (line, mut master) = rest.remove(master_at);
    if !master.inside.is_empty() {
        notes.push(format!(
            "line {line}: {MASTER} is the top bus, so it isn't inside \"{}\"",
            master.inside
        ));
        master.inside.clear();
    }
    ordered.push((line, master));
    // Each pass places the buses whose parent is already placed, so a bus
    // inside one written later still lands, and a loop is caught.
    while !rest.is_empty() {
        let before = rest.len();
        let mut i = 0;
        while i < rest.len() {
            let parent = ordered.iter().position(|(_, b)| b.name == rest[i].1.inside);
            if let Some(p) = parent {
                let (line, bus) = rest.remove(i);
                let depth = depth_in(&ordered, p) + 1;
                let mut bus = bus;
                let at_parent = if depth > MAX_DEPTH {
                    notes.push(format!(
                        "line {line}: {} would nest deeper than {MAX_DEPTH} buses, so it's in {MASTER}",
                        bus.name
                    ));
                    bus.inside = MASTER.to_owned();
                    0
                } else {
                    p
                };
                let at = (at_parent + 1..ordered.len())
                    .take_while(|j| within(&ordered, *j, at_parent))
                    .last()
                    .map_or(at_parent + 1, |j| j + 1);
                ordered.insert(at, (line, bus));
            } else {
                i += 1;
            }
        }
        if rest.len() == before {
            // What's left is inside buses that don't exist, or in a loop:
            // put them in Master.
            let (line, mut bus) = rest.remove(0);
            notes.push(format!(
                "line {line}: {} was inside \"{}\", which isn't a bus here, so it's in {MASTER}",
                bus.name, bus.inside
            ));
            bus.inside = MASTER.to_owned();
            rest.insert(0, (line, bus));
        }
    }
    let names: Vec<String> = ordered.iter().map(|(_, b)| b.name.clone()).collect();
    for (line, bus) in &mut ordered {
        let own = bus.name.clone();
        bus.ducking.retain(|rule| {
            let keep = rule.while_bus != own && names.contains(&rule.while_bus);
            if !keep {
                notes.push(format!(
                    "line {line}: {own}'s ducking while \"{}\" plays is skipped (no such other bus)",
                    rule.while_bus
                ));
            }
            keep
        });
    }
    Ok(Mixer {
        buses: ordered.into_iter().map(|(_, b)| b).collect(),
        voices: crate::voices::STANDARD_VOICES,
    })
}

/// How deep the bus at `at` is in a list being ordered (Master 1).
fn depth_in(list: &[(usize, Bus)], at: usize) -> usize {
    let mut depth = 1;
    let mut here = at;
    while depth <= MAX_DEPTH + 1 {
        let inside = &list[here].1.inside;
        match list.iter().position(|(_, b)| &b.name == inside) {
            Some(p) if !inside.is_empty() => {
                depth += 1;
                here = p;
            }
            _ => break,
        }
    }
    depth
}

/// Whether the bus at `inner` is inside the one at `outer` in a list being
/// ordered.
fn within(list: &[(usize, Bus)], inner: usize, outer: usize) -> bool {
    let mut here = inner;
    for _ in 0..=MAX_DEPTH + 1 {
        if here == outer {
            return true;
        }
        let inside = &list[here].1.inside;
        match list.iter().position(|(_, b)| &b.name == inside) {
            Some(p) if !inside.is_empty() => here = p,
            _ => return false,
        }
    }
    false
}

/// Writes the mixer as a `mixer.solmix` file, one line per bus.
pub fn write(mixer: &Mixer) -> String {
    let f = |v: f32| Value::Float(f64::from(v));
    let mut fields: Vec<(String, Value)> = mixer
        .buses
        .iter()
        .enumerate()
        .map(|(i, bus)| {
            let rules = bus
                .ducking
                .iter()
                .map(|d| {
                    Value::List(vec![
                        Value::Text(d.while_bus.clone()),
                        f(d.by_db),
                        f(d.down_s),
                        f(d.back_s),
                    ])
                })
                .collect();
            (
                format!("bus{}", i + 1),
                Value::List(vec![
                    Value::Text(bus.name.clone()),
                    Value::Text(bus.inside.clone()),
                    f(bus.volume_db),
                    Value::List(rules),
                ]),
            )
        })
        .collect();
    fields.push(("voices".to_owned(), Value::Int(i64::from(mixer.voices))));
    let borrowed: Vec<(&str, Value)> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.clone()))
        .collect();
    settings::write_settings(
        KIND,
        VERSION,
        "The project's buses. Safe to edit by hand: one line per bus.\n\
         Each bus: [name, the bus it's inside, volume in dB, ducking rules].\n\
         Each rule: [while this bus plays, down by dB, fade down s, fade back s].",
        &borrowed,
    )
}

/// Mute and solo, which aren't saved: for listening while working.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listening {
    /// Buses muted, by name.
    pub muted: Vec<String>,
    /// Buses soloed, by name: while any is, only they (and what they're
    /// inside, and what's inside them) are heard.
    pub soloed: Vec<String>,
    /// Everything muted (the Game panel's button).
    pub all_muted: bool,
}

/// What each bus's volume should be now, in decibels: its fader, times
/// game code's `setBusVolume` share while playing (`shares`, 1 as set, 0
/// silent; missing is 1), its ducking from `ducked`, and mute and solo.
/// Silent is [`f32::NEG_INFINITY`].
pub fn levels(mixer: &Mixer, shares: &[f32], ducked: &[f32], listening: &Listening) -> Vec<f32> {
    let soloed: Vec<usize> = listening
        .soloed
        .iter()
        .filter_map(|n| mixer.index_of(n))
        .collect();
    mixer
        .buses
        .iter()
        .enumerate()
        .map(|(i, bus)| {
            if listening.muted.contains(&bus.name) || (i == 0 && listening.all_muted) {
                return f32::NEG_INFINITY;
            }
            // While anything is soloed, a bus is heard only on the way to
            // or from a soloed one.
            if !soloed.is_empty()
                && !soloed
                    .iter()
                    .any(|s| mixer.is_within(i, *s) || mixer.is_within(*s, i))
            {
                return f32::NEG_INFINITY;
            }
            let share = shares.get(i).copied().unwrap_or(1.0);
            let share = if share.is_finite() {
                share.clamp(0.0, 1.0)
            } else {
                1.0
            };
            if share == 0.0 {
                return f32::NEG_INFINITY;
            }
            bus.volume_db + 20.0 * share.log10() + ducked.get(i).copied().unwrap_or(0.0)
        })
        .collect()
}

/// Ducking as it goes: each bus's ducking now, moved towards its target a
/// frame at a time at the rule's speed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ducker {
    /// Each bus's ducking now, in decibels (0 or less).
    pub now_db: Vec<f32>,
}

impl Ducker {
    /// Moves on by `dt` seconds, given which buses have sounds playing
    /// (`busy`, by place; a bus counts as playing when a bus inside it
    /// does). Returns each bus's ducking now.
    pub fn step(&mut self, mixer: &Mixer, busy: &[bool], dt: f32) -> &[f32] {
        self.now_db.resize(mixer.buses.len(), 0.0);
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let playing = |outer: usize| {
            busy.iter()
                .enumerate()
                .any(|(i, b)| *b && mixer.is_within(i, outer))
        };
        for (i, bus) in mixer.buses.iter().enumerate() {
            // The deepest ducking of the rules whose bus is playing; how
            // fast it moves comes from that rule (or, coming back, the
            // slowest rule's fade back).
            let mut target = 0.0_f32;
            let mut down_s = 0.0_f32;
            let mut back_s = 0.0_f32;
            for rule in &bus.ducking {
                back_s = back_s.max(rule.back_s);
                if let Some(w) = mixer.index_of(&rule.while_bus)
                    && playing(w)
                    && rule.by_db < target
                {
                    target = rule.by_db;
                    down_s = rule.down_s;
                }
            }
            let now = &mut self.now_db[i];
            if (*now - target).abs() < 1e-4 {
                *now = target;
                continue;
            }
            // Fades move at a steady rate over the rule's ducking, so a
            // fade of 0.2 s takes 0.2 s whatever the frame rate.
            let span = bus
                .ducking
                .iter()
                .map(|r| -r.by_db)
                .fold(0.0_f32, f32::max)
                .max(1e-3);
            let (time, going_down) = if target < *now {
                (down_s, true)
            } else {
                (back_s, false)
            };
            if time <= 0.0 {
                *now = target;
                continue;
            }
            let moved = span * dt / time;
            *now = if going_down {
                (*now - moved).max(target)
            } else {
                (*now + moved).min(target)
            };
        }
        &self.now_db
    }
}
