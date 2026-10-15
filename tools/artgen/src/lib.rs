//! ASCII sprite sheets → PNG atlas + index.
//!
//! Source format (one sheet per `.txt` file). Outside sprite blocks, lines
//! starting with `;` are comments and blank lines are ignored:
//!
//! ```text
//! sheet ball cell 16x16 columns 8
//! palette
//!   r #e3241b          ; char, then #RRGGBB or #RRGGBBAA
//!   h #ffffffdc
//! end
//! sprite body          ; exactly cell-height rows of cell-width chars
//! ....rrrr....
//! end
//! spin roll from body frames 16 fill r over shade
//! ```
//!
//! `.` is always transparent. Sprites are packed left to right, top to
//! bottom, `columns` per row, in the order they appear. `spin` appends
//! `frames` copies of a sprite rotated clockwise in equal steps
//! (`roll_0` … `roll_15`): each pixel inside the original's shape takes the
//! colour found by rotating back (`fill` where that lands outside), then the
//! optional `over` sprite is blended on top unrotated, inside the shape only
//! (for lighting that stays put while the ball turns).
//!
//! Output: `<name>.png` and `<name>.idx` (one `name x y w h` line per
//! sprite, in sheet order).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

pub type Rgba = [u8; 4];

#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub name: String,
    pub pixels: Vec<Rgba>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub cell_w: usize,
    pub cell_h: usize,
    pub columns: usize,
    pub sprites: Vec<Sprite>,
}

/// A rendered sheet ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct Atlas {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    pub index: String,
}

fn parse_color(s: &str) -> Option<Rgba> {
    let hex = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        6 => Some([byte(0)?, byte(2)?, byte(4)?, 255]),
        8 => Some([byte(0)?, byte(2)?, byte(4)?, byte(6)?]),
        _ => None,
    }
}

/// Parses a sheet source. Errors carry the 1-based line number.
pub fn parse(text: &str) -> Result<Sheet, String> {
    let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
    let mut sheet: Option<Sheet> = None;
    let mut palette: BTreeMap<char, Rgba> = BTreeMap::new();
    palette.insert('.', [0, 0, 0, 0]);

    while let Some((n, raw)) = lines.next() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let err = |m: &str| format!("line {n}: {m}: '{line}'");
        let tok: Vec<&str> = line.split_whitespace().collect();
        match tok[0] {
            "sheet" => {
                let ["sheet", name, "cell", size, "columns", cols] = tok[..] else {
                    return Err(err("usage: sheet <name> cell <W>x<H> columns <N>"));
                };
                let (w, h) = size.split_once('x').ok_or_else(|| err("bad cell size"))?;
                sheet = Some(Sheet {
                    name: name.to_owned(),
                    cell_w: w.parse().map_err(|_| err("bad cell width"))?,
                    cell_h: h.parse().map_err(|_| err("bad cell height"))?,
                    columns: cols.parse().map_err(|_| err("bad column count"))?,
                    sprites: Vec::new(),
                });
            }
            "palette" => loop {
                let Some((n, raw)) = lines.next() else {
                    return Err(format!("line {n}: palette not closed with 'end'"));
                };
                let l = raw.split(';').next().unwrap().trim();
                if l.is_empty() {
                    continue;
                }
                if l == "end" {
                    break;
                }
                let mut parts = l.split_whitespace();
                let (Some(key), Some(color), None) = (parts.next(), parts.next(), parts.next())
                else {
                    return Err(format!("line {n}: expected '<char> <#color>'"));
                };
                let mut chars = key.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    return Err(format!("line {n}: palette key must be one character"));
                };
                if c == '.' {
                    return Err(format!("line {n}: '.' is reserved for transparent"));
                }
                let rgba =
                    parse_color(color).ok_or_else(|| format!("line {n}: bad colour '{color}'"))?;
                palette.insert(c, rgba);
            },
            "sprite" => {
                let s = sheet
                    .as_mut()
                    .ok_or_else(|| err("'sheet' line must come first"))?;
                let [_, name] = tok[..] else {
                    return Err(err("usage: sprite <name>"));
                };
                let mut pixels = Vec::with_capacity(s.cell_w * s.cell_h);
                let mut rows = 0;
                loop {
                    let Some((rn, row)) = lines.next() else {
                        return Err(format!("line {n}: sprite '{name}' not closed with 'end'"));
                    };
                    let row = row.trim();
                    if row == "end" {
                        break;
                    }
                    if row.chars().count() != s.cell_w {
                        return Err(format!(
                            "line {rn}: sprite '{name}' row is {} wide, cell is {}",
                            row.chars().count(),
                            s.cell_w
                        ));
                    }
                    for c in row.chars() {
                        pixels.push(
                            *palette
                                .get(&c)
                                .ok_or_else(|| format!("line {rn}: '{c}' is not in the palette"))?,
                        );
                    }
                    rows += 1;
                }
                if rows != s.cell_h {
                    return Err(format!(
                        "line {n}: sprite '{name}' has {rows} rows, cell is {}",
                        s.cell_h
                    ));
                }
                push_unique(
                    s,
                    Sprite {
                        name: name.to_owned(),
                        pixels,
                    },
                )
                .map_err(|m| err(&m))?;
            }
            "spin" => {
                let s = sheet
                    .as_mut()
                    .ok_or_else(|| err("'sheet' line must come first"))?;
                let (prefix, from, frames, fill, over) = match tok[..] {
                    ["spin", p, "from", f, "frames", n, "fill", c] => (p, f, n, c, None),
                    ["spin", p, "from", f, "frames", n, "fill", c, "over", o] => {
                        (p, f, n, c, Some(o))
                    }
                    _ => {
                        return Err(err(
                            "usage: spin <prefix> from <sprite> frames <N> fill <char> [over <sprite>]",
                        ));
                    }
                };
                let find = |name: &str| {
                    s.sprites
                        .iter()
                        .find(|sp| sp.name == name)
                        .map(|sp| sp.pixels.clone())
                        .ok_or_else(|| err(&format!("no sprite named '{name}' (yet)")))
                };
                let base = find(from)?;
                let over = over.map(find).transpose()?;
                let frames: usize = frames.parse().map_err(|_| err("bad frame count"))?;
                let fill_char = fill.chars().next().unwrap_or('.');
                let fill = *palette
                    .get(&fill_char)
                    .ok_or_else(|| err("fill colour not in palette"))?;
                for k in 0..frames {
                    let angle = k as f32 * std::f32::consts::TAU / frames as f32;
                    let mut px = rotate(&base, s.cell_w, s.cell_h, angle, fill);
                    if let Some(o) = &over {
                        // Lighting only applies inside the shape.
                        let masked: Vec<Rgba> = o
                            .iter()
                            .zip(&px)
                            .map(|(t, b)| if b[3] == 0 { [0; 4] } else { *t })
                            .collect();
                        blend_over(&mut px, &masked);
                    }
                    push_unique(
                        s,
                        Sprite {
                            name: format!("{prefix}_{k}"),
                            pixels: px,
                        },
                    )
                    .map_err(|m| err(&m))?;
                }
            }
            _ => return Err(err("unknown directive")),
        }
    }
    let sheet = sheet.ok_or("no 'sheet' line")?;
    if sheet.sprites.is_empty() {
        return Err("sheet has no sprites".into());
    }
    Ok(sheet)
}

fn push_unique(s: &mut Sheet, sprite: Sprite) -> Result<(), String> {
    if s.sprites.iter().any(|x| x.name == sprite.name) {
        return Err(format!("duplicate sprite name '{}'", sprite.name));
    }
    s.sprites.push(sprite);
    Ok(())
}

/// Rotates clockwise (on screen, y down) by `angle` around the cell centre,
/// nearest-neighbour. Pixels inside the source's shape keep a colour.
pub fn rotate(src: &[Rgba], w: usize, h: usize, angle: f32, fill: Rgba) -> Vec<Rgba> {
    let (sin, cos) = angle.sin_cos();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let mut out = vec![[0; 4]; w * h];
    for y in 0..h {
        for x in 0..w {
            if src[y * w + x][3] == 0 {
                continue; // outside the shape stays transparent
            }
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            // Inverse rotation finds where this pixel came from.
            let sx = (cos * dx + sin * dy + cx).floor();
            let sy = (-sin * dx + cos * dy + cy).floor();
            let sample = if sx >= 0.0 && sy >= 0.0 && (sx as usize) < w && (sy as usize) < h {
                src[sy as usize * w + sx as usize]
            } else {
                [0; 4]
            };
            out[y * w + x] = if sample[3] == 0 { fill } else { sample };
        }
    }
    out
}

/// Standard "over" alpha blending of `top` onto `base`.
pub fn blend_over(base: &mut [Rgba], top: &[Rgba]) {
    for (b, t) in base.iter_mut().zip(top) {
        let ta = t[3] as f32 / 255.0;
        let ba = b[3] as f32 / 255.0;
        let oa = ta + ba * (1.0 - ta);
        if oa <= 0.0 {
            *b = [0; 4];
            continue;
        }
        for i in 0..3 {
            let c = (t[i] as f32 * ta + b[i] as f32 * ba * (1.0 - ta)) / oa;
            b[i] = c.round() as u8;
        }
        b[3] = (oa * 255.0).round() as u8;
    }
}

/// Packs a sheet into one image plus its index text.
pub fn render(sheet: &Sheet) -> Atlas {
    let (cw, ch) = (sheet.cell_w, sheet.cell_h);
    let cols = sheet.columns.max(1);
    let rows = sheet.sprites.len().div_ceil(cols);
    let (width, height) = (cols * cw, rows * ch);
    let mut rgba = vec![0u8; width * height * 4];
    let mut index = String::new();
    for (i, sp) in sheet.sprites.iter().enumerate() {
        let (ox, oy) = ((i % cols) * cw, (i / cols) * ch);
        let _ = writeln!(index, "{} {ox} {oy} {cw} {ch}", sp.name);
        for y in 0..ch {
            for x in 0..cw {
                let dst = ((oy + y) * width + ox + x) * 4;
                rgba[dst..dst + 4].copy_from_slice(&sp.pixels[y * cw + x]);
            }
        }
    }
    Atlas {
        width,
        height,
        rgba,
        index,
    }
}

pub fn encode_png(atlas: &Atlas) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, atlas.width as u32, atlas.height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(&atlas.rgba).expect("png data");
    }
    out
}

/// Decodes an RGBA8 PNG into (width, height, pixels).
pub fn decode_png(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let dec = png::Decoder::new(bytes);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("expected 8-bit RGBA".into());
    }
    buf.truncate(info.buffer_size());
    Ok((info.width as usize, info.height as usize, buf))
}

/// Every `*.txt` source in `src_dir`, sorted.
pub fn sources(src_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut v: Vec<_> = std::fs::read_dir(src_dir)
        .map_err(|e| format!("{}: {e}", src_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    v.sort();
    Ok(v)
}

/// Builds one source file into (sheet name, atlas).
pub fn build(path: &Path) -> Result<(String, Atlas), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sheet = parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((sheet.name.clone(), render(&sheet)))
}

/// Compares a built atlas with the files on disk. `None` = up to date.
pub fn stale_reason(atlas: &Atlas, out_dir: &Path, name: &str) -> Option<String> {
    let png_path = out_dir.join(format!("{name}.png"));
    let idx_path = out_dir.join(format!("{name}.idx"));
    let Ok(bytes) = std::fs::read(&png_path) else {
        return Some(format!("{} missing", png_path.display()));
    };
    match decode_png(&bytes) {
        Ok((w, h, px)) if w == atlas.width && h == atlas.height && px == atlas.rgba => {}
        Ok(_) => return Some(format!("{} pixels differ", png_path.display())),
        Err(e) => return Some(format!("{}: {e}", png_path.display())),
    }
    match std::fs::read_to_string(&idx_path) {
        Ok(idx) if idx == atlas.index => None,
        Ok(_) => Some(format!("{} differs", idx_path.display())),
        Err(_) => Some(format!("{} missing", idx_path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "
        ; tiny test sheet
        sheet t cell 4x4 columns 2
        palette
          r #ff0000
          h #ffffff80   ; half-transparent white
        end
        sprite a
        .rr.
        rrrr
        rrrr
        .rr.
        end
        sprite light
        h...
        ....
        ....
        ....
        end
        spin s from a frames 4 fill r over light
    ";

    #[test]
    fn parses_and_packs() {
        let sheet = parse(SRC).unwrap();
        let names: Vec<_> = sheet.sprites.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["a", "light", "s_0", "s_1", "s_2", "s_3"]);
        let atlas = render(&sheet);
        assert_eq!((atlas.width, atlas.height), (8, 12));
        assert!(
            atlas
                .index
                .starts_with("a 0 0 4 4\nlight 4 0 4 4\ns_0 0 4 4 4\n")
        );
    }

    #[test]
    fn colours() {
        assert_eq!(parse_color("#102030"), Some([16, 32, 48, 255]));
        assert_eq!(parse_color("#10203040"), Some([16, 32, 48, 64]));
        assert_eq!(parse_color("102030"), None);
        assert_eq!(parse_color("#12"), None);
    }

    #[test]
    fn rotation_quarter_turn_moves_a_marker_clockwise() {
        // 4x4 opaque square with one marker pixel at top-left.
        let r = [255, 0, 0, 255];
        let m = [0, 0, 255, 255];
        let mut src = vec![r; 16];
        src[0] = m;
        let out = rotate(&src, 4, 4, std::f32::consts::FRAC_PI_2, r);
        // Clockwise on screen: top-left goes to top-right.
        assert_eq!(out[3], m);
        assert_eq!(out.iter().filter(|p| **p == m).count(), 1);
    }

    #[test]
    fn rotation_keeps_the_shape() {
        let sheet = parse(SRC).unwrap();
        let a = &sheet.sprites[0].pixels;
        for k in 2..6 {
            let s = &sheet.sprites[k].pixels;
            for i in 0..16 {
                assert_eq!(a[i][3] == 0, s[i][3] == 0, "frame {k} pixel {i}");
            }
        }
    }

    #[test]
    fn blending() {
        let mut base = vec![[255, 0, 0, 255]];
        blend_over(&mut base, &[[255, 255, 255, 128]]);
        assert_eq!(base[0][3], 255);
        assert!((127..=129).contains(&base[0][1]));
    }

    #[test]
    fn png_round_trip() {
        let atlas = render(&parse(SRC).unwrap());
        let (w, h, px) = decode_png(&encode_png(&atlas)).unwrap();
        assert_eq!((w, h), (atlas.width, atlas.height));
        assert_eq!(px, atlas.rgba);
    }

    #[test]
    fn errors_have_line_numbers() {
        let cases = [
            ("sprite a\n", "line 1"),
            ("sheet t cell 2x2 columns 1\nsprite a\n..\n.\nend", "line 4"),
            (
                "sheet t cell 2x2 columns 1\nsprite a\n.x\n..\nend",
                "line 3",
            ),
            ("sheet t cell 2x2 columns 1\nbogus", "line 2"),
            (
                "sheet t cell 2x2 columns 1\npalette\n. #000000\nend",
                "line 3",
            ),
        ];
        for (src, want) in cases {
            let e = parse(src).unwrap_err();
            assert!(e.contains(want), "{src:?}: {e}");
        }
    }
}
