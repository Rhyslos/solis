//! Text typing itself out on the game's screens (stage 1, text effects):
//! for each screen shown, when each of its texts began typing, kept while
//! its words stay the same; the typing sound; `TypingFinished` once every
//! text on a screen has typed out; and `finishTyping`, which shows it all.
//!
//! None of this is the game's state (decision 0087): it's counted on the
//! game's ticks (a text begins on the tick it first shows, whatever the
//! frame rate), so the same letters show at the same moments at 30 or 144
//! frames a second, and what game code hears comes with a tick's input
//! (decision 0088), so a replay hears it on the same tick.

use reflect::EntityId;
use std::sync::Arc;
use ui::typing::{SoundEvery, Started, TypeOut};

/// The event game code hears when a screen's text has typed out.
pub(crate) const FINISHED: &str = "TypingFinished";

/// One text of a screen, as it types.
#[derive(Debug, Clone)]
struct Text {
    /// Its words as written (markup and all): a change starts it again.
    words: String,
    /// When it began, in UI seconds (−∞: shown whole).
    start: f64,
    /// When its letters show.
    out: TypeOut,
    /// How far into its typing the sound has been played up to.
    heard: f64,
}

/// What a shown screen types out at.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Typist {
    /// Letters a second (0: at once).
    pub(crate) speed: f32,
    /// The sound event played as it types (`Sfx.Blip`), if any.
    pub(crate) sound: Option<String>,
    /// Each letter or each word.
    pub(crate) every: SoundEvery,
}

/// One screen shown, and its texts.
#[derive(Debug, Clone)]
struct Shown {
    entity: EntityId,
    screen: String,
    typist: Typist,
    texts: Vec<Text>,
    /// Whether game code has been told its texts finished since one last
    /// began.
    told: bool,
    /// The starts, as its picture carries them.
    started: Arc<Started>,
}

impl Shown {
    fn elapsed(text: &Text, now: f64) -> f64 {
        if text.start.is_finite() {
            (now - text.start).max(0.0)
        } else {
            f64::NAN
        }
    }

    fn finished(&self, now: f64) -> bool {
        self.texts
            .iter()
            .all(|t| t.out.finished(Self::elapsed(t, now)))
    }

    fn share(&mut self) {
        self.started = Arc::new(Started {
            at: self.texts.iter().map(|t| t.start).collect(),
        });
    }
}

/// The screens' typing, kept between ticks.
#[derive(Debug, Default)]
pub(crate) struct ScreenTyping {
    shown: Vec<Shown>,
    /// Whether a screen shown types its text out: the screens are then
    /// read after every tick, so each text begins on the tick it shows.
    pub(crate) active: bool,
}

impl ScreenTyping {
    /// Takes in a screen's view read after a tick, at UI time `now`: each
    /// text keeps its start while its words stay, and starts typing from
    /// `now` when they're new. Gives the starts for its picture.
    pub(crate) fn update(
        &mut self,
        (entity, screen): (EntityId, &str),
        view: &ui::view::Node,
        typist: Typist,
        now: f64,
    ) -> Arc<Started> {
        let at = match self
            .shown
            .iter()
            .position(|s| s.entity == entity && s.screen == screen)
        {
            Some(at) => at,
            None => {
                self.shown.push(Shown {
                    entity,
                    screen: screen.to_string(),
                    typist: typist.clone(),
                    texts: Vec::new(),
                    told: true,
                    started: Arc::default(),
                });
                self.shown.len() - 1
            }
        };
        let shown = &mut self.shown[at];
        let speed_changed = shown.typist.speed != typist.speed;
        shown.typist = typist;
        let words = ui::view::texts_of(view);
        let mut changed = shown.texts.len() != words.len();
        shown.texts.truncate(words.len());
        for (i, words) in words.into_iter().enumerate() {
            let same = shown.texts.get(i).is_some_and(|t| t.words == words);
            // A load can take the clock back: what began later begins now.
            let ahead = shown.texts.get(i).is_some_and(|t| t.start > now);
            if same && !ahead && !speed_changed {
                continue;
            }
            let marked = ui::markup::parse(&words);
            let text = Text {
                words,
                start: now,
                out: TypeOut::of(&marked, shown.typist.speed),
                heard: f64::NEG_INFINITY,
            };
            if shown.typist.speed > 0.0 {
                shown.told = false;
            }
            match shown.texts.get_mut(i) {
                Some(t) => *t = text,
                None => shown.texts.push(text),
            }
            changed = true;
        }
        if changed {
            shown.share();
        }
        Arc::clone(&shown.started)
    }

    /// Forgets the screens no longer shown (`shown`: entity and name).
    pub(crate) fn keep_only(&mut self, shown: &[(EntityId, String)]) {
        self.shown
            .retain(|s| shown.iter().any(|(e, n)| *e == s.entity && *n == s.screen));
        self.active = self.shown.iter().any(|s| s.typist.speed > 0.0);
    }

    /// `finishTyping`: every text the entity's screen named `screen` is
    /// typing shows whole, if any still types at `now`; `TypingFinished`
    /// follows on the next tick.
    pub(crate) fn finish(&mut self, entity: EntityId, screen: &str, now: f64) {
        for shown in self
            .shown
            .iter_mut()
            .filter(|s| s.entity == entity && s.screen.eq_ignore_ascii_case(screen))
        {
            if shown.finished(now) {
                continue;
            }
            for text in &mut shown.texts {
                text.start = f64::NEG_INFINITY;
            }
            shown.share();
        }
    }

    /// The starts of the entity's screen `screen`, as its picture carries
    /// them.
    #[cfg(test)]
    pub(crate) fn started(&self, entity: EntityId, screen: &str) -> Option<Arc<Started>> {
        self.shown
            .iter()
            .find(|s| s.entity == entity && s.screen == screen)
            .map(|s| Arc::clone(&s.started))
    }

    /// A tick at UI time `now`: the typing sounds due since the last (one
    /// a screen at most), and `TypingFinished` for each screen whose texts
    /// have all typed out since one began.
    pub(crate) fn tick(&mut self, now: f64) -> (Vec<input::Sent>, Vec<String>) {
        let mut sent = Vec::new();
        let mut sounds = Vec::new();
        for shown in &mut self.shown {
            // Text shown at once makes no typing sound.
            if let Some(sound) = shown
                .typist
                .sound
                .as_ref()
                .filter(|_| shown.typist.speed > 0.0)
            {
                let every = shown.typist.every;
                let mut due = false;
                for text in &mut shown.texts {
                    let elapsed = Shown::elapsed(text, now);
                    if elapsed.is_nan() {
                        continue;
                    }
                    due |= text.out.sounds_between(text.heard, elapsed, every);
                    text.heard = elapsed;
                }
                if due {
                    sounds.push(sound.clone());
                }
            }
            if !shown.told && shown.finished(now) {
                shown.told = true;
                sent.push(input::Sent {
                    to: shown.entity,
                    name: FINISHED.to_string(),
                    values: vec![reflect::Value::Text(shown.screen.clone())],
                });
            }
        }
        (sent, sounds)
    }
}

/// What's wrong with the effects in `view`'s texts as the game worked
/// them out (stage 1, text effects), said once each in the Console by the
/// screens: a text written out in code is in Problems already, but one
/// put together as the game runs is found only now. At most 8.
pub(crate) fn mistakes_in(screen: &str, view: &ui::view::Node) -> Vec<String> {
    fn walk(node: &ui::view::Node, depth: usize, out: &mut Vec<String>, screen: &str) {
        if out.len() >= 8 || depth >= ui::screen::MAX_DEPTH {
            return;
        }
        if matches!(node.kind.as_str(), "text" | "button") && node.text.contains('[') {
            let marked = ui::markup::parse(&node.text);
            if let Some(first) = marked.mistakes.first() {
                let shown: String = node.text.chars().take(60).collect();
                out.push(format!(
                    "The screen {screen} shows \"{shown}\", whose effects have a mistake: {}. \
                     It's shown as written.",
                    first.message
                ));
            }
        }
        for child in &node.children {
            walk(child, depth + 1, out, screen);
        }
    }
    let mut out = Vec::new();
    walk(view, 0, &mut out, screen);
    out
}

#[cfg(test)]
#[path = "screen_typing_tests.rs"]
mod tests;
