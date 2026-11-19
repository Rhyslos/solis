//! The text layout study (stage 1, docs/reports/text-layout-study.md):
//! parley against Solis's own layout (unicode-linebreak and harfrust), on
//! the same paragraphs, the same font and the same widths. Not part of
//! Solis's build: run it from this folder with `cargo run --release`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

const INTER: &[u8] = include_bytes!("../../../crates/editor/assets/fonts/Inter-Regular.ttf");
const SIZE: f32 = 20.0;

fn sets(frame: usize) -> Vec<(&'static str, Vec<String>, f32)> {
    let lorem = "The lantern swung as the cart rattled over the bridge, and the river below caught every spark of it. Nobody spoke; the horses knew the road better than the driver, and the driver knew better than to say so. At the far bank a figure waited with a lamp of its own, patient as a milestone. ";
    let greek = "Η γρήγορη καφέ αλεπού πηδάει πάνω από τον τεμπέλη σκύλο, και ο σκύλος δεν ξυπνά. Съешь же ещё этих мягких французских булок, да выпей чаю. ";
    vec![
        (
            "2,000 changing lines (a list)",
            (0..2000)
                .map(|i| format!("Item {}: {} gold, {} left", i + frame * 7, (i * 37 + frame) % 1000, (i + frame) % 13))
                .collect(),
            400.0,
        ),
        (
            "200 paragraphs, 4 sentences each",
            (0..200).map(|i| format!("{i}. {lorem}{lorem}")).collect(),
            320.0,
        ),
        (
            "200 Greek and Cyrillic paragraphs",
            (0..200).map(|i| format!("{i}. {greek}{greek}")).collect(),
            320.0,
        ),
    ]
}

// ---------- parley ----------

struct Parley {
    fcx: parley::FontContext,
    lcx: parley::LayoutContext<()>,
}

impl Parley {
    fn new() -> Self {
        let mut fcx = parley::FontContext::default();
        fcx.collection
            .register_fonts(parley::fontique::Blob::new(Arc::new(INTER.to_vec())), None);
        Self { fcx, lcx: parley::LayoutContext::new() }
    }

    /// Lines and glyphs.
    fn lay_out(&mut self, text: &str, width: f32) -> (usize, usize, f32) {
        let mut b = self.lcx.ranged_builder(&mut self.fcx, text, 1.0, true);
        b.push_default(parley::StyleProperty::FontFamily(parley::FontFamily::Source("Inter".into())));
        b.push_default(parley::StyleProperty::FontSize(SIZE));
        let mut layout: parley::Layout<()> = b.build(text);
        layout.break_all_lines(Some(width));
        layout.align(parley::Alignment::Start, parley::AlignmentOptions::default());
        let mut glyphs = 0;
        let mut x = 0.0;
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(run) = item {
                    for g in run.positioned_glyphs() {
                        glyphs += 1;
                        x += g.x;
                    }
                }
            }
        }
        (layout.len(), glyphs, x)
    }
}

// ---------- own ----------

struct Own {
    font: harfrust::FontRef<'static>,
    data: harfrust::ShaperData,
    plans: HashMap<(harfrust::Script, harfrust::Direction), harfrust::ShapePlan>,
    /// Shaped pieces between break chances: glyphs (id, advance in px).
    words: HashMap<String, Arc<Vec<(u32, f32)>>>,
    upem: f32,
    cache_words: bool,
}

impl Own {
    fn new(cache_words: bool) -> Self {
        let font = harfrust::FontRef::new(INTER).unwrap();
        let data = harfrust::ShaperData::new(&font);
        let upem = {
            let shaper = data.shaper(&font).build();
            shaper.units_per_em() as f32
        };
        Self { font, data, plans: HashMap::new(), words: HashMap::new(), upem, cache_words }
    }

    fn shape(&mut self, piece: &str) -> Arc<Vec<(u32, f32)>> {
        if self.cache_words {
            if let Some(found) = self.words.get(piece) {
                return found.clone();
            }
        }
        let shaper = self.data.shaper(&self.font).build();
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str(piece);
        buffer.guess_segment_properties();
        let key = (buffer.script(), buffer.direction());
        let plan = self.plans.entry(key).or_insert_with(|| {
            harfrust::ShapePlan::new(&shaper, key.1, Some(key.0), None, &[])
        });
        let out = shaper.shape(buffer, harfrust::ShapeOptions::new().plan(Some(plan)));
        let k = SIZE / self.upem;
        let glyphs: Vec<(u32, f32)> = out
            .glyph_infos()
            .iter()
            .zip(out.glyph_positions())
            .map(|(i, p)| (i.glyph_id, p.x_advance as f32 * k))
            .collect();
        let glyphs = Arc::new(glyphs);
        if self.cache_words {
            self.words.insert(piece.to_string(), glyphs.clone());
        }
        glyphs
    }

    fn lay_out(&mut self, text: &str, width: f32) -> (usize, usize, f32) {
        let mut lines = 1;
        let mut glyph_count = 0;
        let mut x = 0.0_f32;
        let mut sum = 0.0;
        let mut start = 0;
        for (end, chance) in unicode_linebreak::linebreaks(text) {
            let piece = &text[start..end];
            start = end;
            let glyphs = self.shape(piece);
            let w: f32 = glyphs.iter().map(|g| g.1).sum();
            let trailing = piece.len() - piece.trim_end().len();
            let ink = if trailing > 0 { w - glyphs.last().map_or(0.0, |g| g.1) } else { w };
            if x > 0.0 && x + ink > width {
                lines += 1;
                x = 0.0;
            }
            for g in glyphs.iter() {
                sum += x;
                x += g.1;
                glyph_count += 1;
            }
            if chance == unicode_linebreak::BreakOpportunity::Mandatory && end < text.len() {
                lines += 1;
                x = 0.0;
            }
        }
        (lines, glyph_count, sum)
    }
}

fn main() {
    let mut parley = Parley::new();
    let mut own = Own::new(false);
    let mut own_cached = Own::new(true);
    // Warm up: fonts read, plans made, a frame of words cached.
    for (_, texts, w) in sets(0) {
        for t in &texts {
            parley.lay_out(t, w);
            own.lay_out(t, w);
            own_cached.lay_out(t, w);
        }
    }
    println!("| Paragraphs | parley | own, shaped whole pieces | own, words remembered |");
    println!("| --- | --- | --- | --- |");
    for set in 0..3 {
        let mut best = [f64::MAX; 3];
        let mut check = [(0, 0); 3];
        for frame in 1..=10 {
            let (name, texts, w) = sets(frame).swap_remove(set);
            let _ = name;
            let t0 = Instant::now();
            let mut a = (0, 0);
            for t in &texts {
                let r = parley.lay_out(t, w);
                a = (a.0 + r.0, a.1 + r.1);
            }
            let t1 = Instant::now();
            let mut b = (0, 0);
            for t in &texts {
                let r = own.lay_out(t, w);
                b = (b.0 + r.0, b.1 + r.1);
            }
            let t2 = Instant::now();
            let mut c = (0, 0);
            for t in &texts {
                let r = own_cached.lay_out(t, w);
                c = (c.0 + r.0, c.1 + r.1);
            }
            let t3 = Instant::now();
            let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.0;
            best[0] = best[0].min(ms(t0, t1));
            best[1] = best[1].min(ms(t1, t2));
            best[2] = best[2].min(ms(t2, t3));
            check = [a, b, c];
        }
        let name = sets(0)[set].0;
        println!(
            "| {name} | {:.2} ms | {:.2} ms | {:.2} ms |",
            best[0], best[1], best[2]
        );
        eprintln!("  lines/glyphs: parley {:?}, own {:?}, own cached {:?}", check[0], check[1], check[2]);
    }
}
