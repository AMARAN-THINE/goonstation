//! Very small DM source scanner. It only extracts `icon`/`icon_state` defaults
//! per type path, handling the indentation-nested and single-line forms. It is
//! NOT a DM parser: macros, conditionals and procs are skipped.
use std::collections::HashMap;
use std::path::Path;

#[derive(Default, Clone, Debug)]
pub struct TypeInfo {
    pub icon: Option<String>,
    pub icon_state: Option<String>,
}

pub type Types = HashMap<String, TypeInfo>;

pub fn scan_tree(root: &Path, types: &mut Types) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            scan_tree(&p, types);
        } else if p.extension().map_or(false, |x| x == "dm") {
            if let Ok(s) = std::fs::read_to_string(&p) {
                scan_file(&s, types);
            }
        }
    }
}

fn strip_comment(l: &str) -> &str {
    let mut in_str = false;
    let b = l.as_bytes();
    for i in 0..b.len() {
        match b[i] {
            b'"' => in_str = !in_str,
            b'/' if !in_str && i + 1 < b.len() && b[i + 1] == b'/' => return &l[..i],
            _ => {}
        }
    }
    l
}

fn is_path(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '/')
}

fn quoted(v: &str) -> Option<String> {
    let v = v.trim().trim_end_matches(';').trim();
    let q = v.chars().next()?;
    if (q == '\'' || q == '"') && v.len() >= 2 && v.ends_with(q) {
        Some(v[1..v.len() - 1].to_string())
    } else {
        None
    }
}

pub fn scan_file(src: &str, types: &mut Types) {
    // stack of (indent, path segments)
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut in_block_comment = false;
    for raw in src.lines() {
        let mut line = strip_comment(raw);
        if in_block_comment {
            if let Some(i) = line.find("*/") { line = &line[i + 2..]; in_block_comment = false; } else { continue; }
        }
        if let Some(i) = line.find("/*") {
            if !line[i..].contains("*/") { in_block_comment = true; }
            line = &line[..i];
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let indent = line.chars().take_while(|c| *c == '\t').count();
        while stack.last().map_or(false, |(i, _)| *i >= indent) { stack.pop(); }
        let ctx: String = stack.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("/");

        if let Some((lhs, rhs)) = trimmed.split_once('=') {
            if rhs.starts_with('=') || lhs.ends_with(['!', '<', '>']) { continue; }
            let lhs = lhs.trim().trim_start_matches("var/").trim();
            let (path, var) = match lhs.rsplit_once('/') {
                Some((p, v)) => (join(&ctx, p), v),
                None => (ctx.clone(), lhs),
            };
            if path.is_empty() { continue; }
            if var == "icon" || var == "icon_state" {
                if let Some(v) = quoted(rhs) {
                    let t = types.entry(path).or_default();
                    if var == "icon" { t.icon = Some(v) } else { t.icon_state = Some(v) }
                }
            }
        } else if is_path(trimmed) && !trimmed.contains("proc") && !trimmed.contains("verb") {
            let seg = trimmed.trim_end_matches('/');
            stack.push((indent, seg.to_string()));
            types.entry(join(&ctx, seg)).or_default();
        }
    }
}

fn join(ctx: &str, p: &str) -> String {
    let p = p.trim_matches('/');
    let full = if ctx.is_empty() { p.to_string() } else if p.is_empty() { ctx.to_string() } else { format!("{ctx}/{p}") };
    format!("/{}", full.trim_start_matches('/'))
}

/// Resolve icon + icon_state by walking up the path hierarchy.
pub fn resolve(types: &Types, path: &str) -> (Option<String>, Option<String>) {
    let (mut icon, mut state) = (None, None);
    let mut cur = path.to_string();
    loop {
        if let Some(t) = types.get(&cur) {
            icon = icon.or_else(|| t.icon.clone());
            state = state.or_else(|| t.icon_state.clone());
        }
        if (icon.is_some() && state.is_some()) || cur.len() <= 1 { break; }
        match cur.rfind('/') { Some(0) | None => break, Some(i) => cur.truncate(i) }
    }
    (icon, state)
}
