//! Minimal .dmi reader: a PNG whose zTXt "Description" chunk lists icon states.
use std::collections::HashMap;
use std::path::Path;

pub struct Sheet {
    pub w: u32,
    pub h: u32,
    pub tw: u32,
    pub th: u32,
    pub rgba: Vec<u8>,
    /// state name -> (index of first icon, dirs, frames)
    pub states: HashMap<String, (u32, u32, u32)>,
}

pub fn load(path: &Path) -> Option<Sheet> {
    let mut dec = png::Decoder::new(std::fs::File::open(path).ok()?);
    dec.set_transformations(png::Transformations::EXPAND);
    let mut reader = dec.read_info().ok()?;
    let mut desc = String::new();
    for c in &reader.info().compressed_latin1_text {
        if c.keyword == "Description" {
            desc = c.get_text().ok()?;
        }
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let fi = reader.next_frame(&mut buf).ok()?;
    let rgba = to_rgba(&buf[..fi.buffer_size()], fi.color_type)?;
    let (mut tw, mut th) = (32, 32);
    let mut states = HashMap::new();
    let mut next = 0u32;
    let mut cur: Option<(String, u32, u32)> = None;
    let mut flush = |cur: &mut Option<(String, u32, u32)>, next: &mut u32, st: &mut HashMap<String, (u32, u32, u32)>| {
        if let Some((n, d, f)) = cur.take() {
            st.entry(n).or_insert((*next, d, f));
            *next += d * f;
        }
    };
    for line in desc.lines() {
        let l = line.trim();
        if let Some(v) = l.strip_prefix("width = ") {
            tw = v.parse().unwrap_or(32);
        } else if let Some(v) = l.strip_prefix("height = ") {
            th = v.parse().unwrap_or(32);
        } else if let Some(v) = l.strip_prefix("state = ") {
            flush(&mut cur, &mut next, &mut states);
            cur = Some((v.trim_matches('"').to_string(), 1, 1));
        } else if let Some(v) = l.strip_prefix("dirs = ") {
            if let Some(c) = cur.as_mut() { c.1 = v.parse().unwrap_or(1); }
        } else if let Some(v) = l.strip_prefix("frames = ") {
            if let Some(c) = cur.as_mut() { c.2 = v.parse().unwrap_or(1); }
        }
    }
    flush(&mut cur, &mut next, &mut states);
    Some(Sheet { w: fi.width, h: fi.height, tw, th, rgba, states })
}

fn to_rgba(b: &[u8], ct: png::ColorType) -> Option<Vec<u8>> {
    Some(match ct {
        png::ColorType::Rgba => b.to_vec(),
        png::ColorType::Rgb => b.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => b.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => b.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        _ => return None,
    })
}

impl Sheet {
    /// Pixel rect (x, y) of the first frame, south-facing icon of `state`.
    /// An empty/unknown state falls back to the unnamed state "".
    pub fn locate(&self, state: &str) -> Option<(u32, u32)> {
        let (idx, _, _) = *self.states.get(state).or_else(|| self.states.get(""))?;
        let per_row = (self.w / self.tw).max(1);
        Some(((idx % per_row) * self.tw, (idx / per_row) * self.th))
    }
}
