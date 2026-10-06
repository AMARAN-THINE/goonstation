//! TGM-format .dmm reader (what Goonstation's maps/*.dmm use).
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Instance {
    pub path: String,
    pub icon_state: Option<String>,
    pub icon: Option<String>,
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    /// cells[z][y*width+x] = key index list; z only kept for z==1
    pub grid: Vec<String>,
    pub keys: HashMap<String, Vec<Instance>>,
}

fn var_str(body: &str, name: &str) -> Option<String> {
    let i = body.find(&format!("{name} = "))?;
    let rest = &body[i + name.len() + 3..];
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' { return None; }
    let end = rest[1..].find(q)?;
    Some(rest[1..1 + end].to_string())
}

fn parse_instance(s: &str) -> Instance {
    let s = s.trim().trim_end_matches(',');
    match s.find('{') {
        Some(i) => {
            let body = &s[i..];
            Instance { path: s[..i].trim().into(), icon_state: var_str(body, "icon_state"), icon: var_str(body, "icon") }
        }
        None => Instance { path: s.into(), icon_state: None, icon: None },
    }
}

pub fn parse(src: &str) -> Map {
    let mut keys: HashMap<String, Vec<Instance>> = HashMap::new();
    let mut lines = src.lines().peekable();
    let mut blocks: Vec<(usize, Vec<String>)> = Vec::new(); // (x, rows) for z==1
    while let Some(l) = lines.next() {
        if l.starts_with('"') && l.contains("= (") {
            let key = l[1..l.find('"').unwrap_or(1).max(1) + 0].to_string();
            let key = l[1..].split('"').next().unwrap_or(&key).to_string();
            let mut insts = Vec::new();
            let mut cur = String::new();
            let mut depth = 0i32;
            while let Some(b) = lines.next() {
                let t = b.trim();
                let closes = t.ends_with(')') && depth == 0 && !t.ends_with("},");
                if t.contains('{') { depth += 1; }
                if t.contains('}') { depth -= 1; }
                cur.push_str(t);
                cur.push(' ');
                if depth <= 0 && (t.ends_with(',') || closes) {
                    let c = cur.trim().trim_end_matches(')').trim().to_string();
                    if !c.is_empty() { insts.push(parse_instance(&c)); }
                    cur.clear();
                    if closes { break; }
                }
            }
            keys.insert(key, insts);
        } else if l.starts_with('(') && l.contains("= {\"") {
            let nums: Vec<usize> = l[1..l.find(')').unwrap_or(1)].split(',').filter_map(|n| n.trim().parse().ok()).collect();
            let mut rows = Vec::new();
            while let Some(r) = lines.next() {
                if r.starts_with("\"}") { break; }
                rows.push(r.to_string());
            }
            if nums.len() == 3 && nums[2] == 1 { blocks.push((nums[0], rows)); }
        }
    }
    let kw = keys.keys().next().map_or(1, |k| k.len());
    let height = blocks.iter().map(|(_, r)| r.len()).max().unwrap_or(0);
    let width = blocks.iter().map(|(x, _)| *x).max().unwrap_or(0);
    let mut grid = vec![String::new(); width * height];
    for (x, rows) in &blocks {
        for (i, k) in rows.iter().enumerate() {
            let y = height - rows.len() + i; // rows are top-down
            grid[y * width + (x - 1)] = k.get(..kw).unwrap_or("").to_string();
        }
    }
    Map { width, height, grid, keys }
}
