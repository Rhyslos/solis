//! Timing and the report's tables.

use std::time::Instant;

/// Milliseconds `work` takes, and what it gave.
pub(crate) fn timed<T>(work: impl FnOnce() -> T) -> (f64, T) {
    let clock = Instant::now();
    let out = work();
    (clock.elapsed().as_secs_f64() * 1000.0, out)
}

/// A spread of times, in milliseconds.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Spread {
    pub(crate) mean: f64,
    pub(crate) median: f64,
    pub(crate) p95: f64,
    pub(crate) worst: f64,
}

impl Spread {
    pub(crate) fn of(times: &[f64]) -> Self {
        if times.is_empty() {
            return Self::default();
        }
        let mut sorted = times.to_vec();
        sorted.sort_by(f64::total_cmp);
        let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q).round() as usize];
        let total: f64 = sorted.iter().sum();
        Self {
            mean: total / sorted.len() as f64,
            median: at(0.5),
            p95: at(0.95),
            worst: at(1.0),
        }
    }
}

/// A Markdown table, printed as it's made.
pub(crate) struct Table;

impl Table {
    pub(crate) fn start(title: &str, heads: &[&str]) -> Self {
        println!("\n#### {title}\n");
        println!("| {} |", heads.join(" | "));
        println!("|{}", " --- |".repeat(heads.len()));
        Self
    }

    pub(crate) fn row(&self, cells: &[String]) {
        println!("| {} |", cells.join(" | "));
    }
}

/// Milliseconds, to two places (three below a tenth).
pub(crate) fn ms(v: f64) -> String {
    if v < 0.1 {
        format!("{v:.3}")
    } else {
        format!("{v:.2}")
    }
}

/// Bytes as megabytes, to one place.
pub(crate) fn mb(bytes: f64) -> String {
    format!("{:.1} MB", bytes / 1_048_576.0)
}

/// A count with thousands marked.
pub(crate) fn count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
