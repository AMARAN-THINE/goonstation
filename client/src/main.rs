mod dmi;
mod dmm;
mod dmscan;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn layer_of(path: &str) -> u8 {
    if path.starts_with("/turf") { 0 } else if path.starts_with("/obj") { 1 } else if path.starts_with("/mob") { 2 } else { 3 }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: ssview <goonstation-root> <map.dmm> <out.png> [x0 y0 w h]");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[1]);
    let mut types = dmscan::Types::new();
    for d in ["_std", "code"] { dmscan::scan_tree(&root.join(d), &mut types); }
    eprintln!("scanned {} DM types", types.len());

    let map = dmm::parse(&std::fs::read_to_string(&args[2]).expect("read map"));
    eprintln!("map {}x{}, {} cell keys", map.width, map.height, map.keys.len());
    let n = |i: usize, d: usize| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (x0, y0) = (n(4, 0), n(5, 0));
    let (w, h) = (n(6, map.width.min(100)).min(map.width - x0), n(7, map.height.min(100)).min(map.height - y0));
    let (ts, mut out) = (32usize, vec![0u8; 0]);
    let (pw, ph) = (w * ts, h * ts);
    out.resize(pw * ph * 4, 0);
    for px in out.chunks_mut(4) { px[3] = 255; }

    let mut sheets: HashMap<String, Option<dmi::Sheet>> = HashMap::new();
    let (mut drawn, mut missing) = (0usize, 0usize);
    for cy in 0..h { for cx in 0..w {
        let key = &map.grid[(y0 + cy) * map.width + x0 + cx];
        let Some(insts) = map.keys.get(key) else { continue };
        let mut order: Vec<&dmm::Instance> = insts.iter().filter(|i| !i.path.starts_with("/area")).collect();
        order.sort_by_key(|i| layer_of(&i.path));
        for inst in order {
            let (ricon, rstate) = dmscan::resolve(&types, &inst.path);
            let (Some(icon), state) = (inst.icon.clone().or(ricon), inst.icon_state.clone().or(rstate).unwrap_or_default()) else { missing += 1; continue };
            let sheet = sheets.entry(icon.clone()).or_insert_with(|| dmi::load(&root.join(Path::new(&icon))));
            let Some(sheet) = sheet else { missing += 1; continue };
            let Some((sx, sy)) = sheet.locate(&state) else { missing += 1; continue };
            drawn += 1;
            for ty in 0..ts.min(sheet.th as usize) { for tx in 0..ts.min(sheet.tw as usize) {
                let s = ((sy as usize + ty) * sheet.w as usize + sx as usize + tx) * 4;
                let a = sheet.rgba[s + 3] as u32;
                if a == 0 { continue }
                let d = ((cy * ts + ty) * pw + cx * ts + tx) * 4;
                for c in 0..3 { out[d + c] = ((sheet.rgba[s + c] as u32 * a + out[d + c] as u32 * (255 - a)) / 255) as u8; }
            } }
        }
    } }
    eprintln!("drew {drawn} sprites, {missing} unresolved");
    let f = std::io::BufWriter::new(std::fs::File::create(&args[3]).unwrap());
    let mut enc = png::Encoder::new(f, pw as u32, ph as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&out).unwrap();
}
