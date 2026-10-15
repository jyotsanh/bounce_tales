//! Pixel text with the 5x7 artgen font (`assets/art/font.txt`): white
//! glyphs tinted when drawn, 6 px advance (times `scale`), optional 1 px
//! drop shadow. Lowercase is drawn as capitals; unknown characters as `?`.

use macroquad::prelude::*;

use super::atlas::Atlas;

/// Horizontal advance per character at scale 1, px.
pub const ADVANCE: f32 = 6.0;
/// Glyph height at scale 1, px.
pub const HEIGHT: f32 = 7.0;

/// Sprite name for a character, or `None` for a space.
pub fn glyph_name(c: char) -> Option<&'static str> {
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    const LETTERS: [&str; 26] = [
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    let c = c.to_ascii_uppercase();
    Some(match c {
        ' ' => return None,
        '0'..='9' => DIGITS[c as usize - '0' as usize],
        'A'..='Z' => LETTERS[c as usize - 'A' as usize],
        ':' => "colon",
        '/' => "slash",
        '*' => "times",
        '-' => "dash",
        '.' => "dot",
        '!' => "bang",
        '+' => "plus",
        ',' => "comma",
        '\'' => "apos",
        _ => "quest",
    })
}

/// Width of `text` in pixels at `scale`.
pub fn width(text: &str, scale: f32) -> f32 {
    let n = text.chars().count() as f32;
    if n == 0.0 {
        0.0
    } else {
        (n * ADVANCE - 1.0) * scale
    }
}

/// Draws `text` with its top-left at `pos`.
pub fn draw(font: &Atlas, text: &str, pos: Vec2, scale: f32, color: Color, shadow: bool) {
    let pos = pos.round();
    if shadow {
        draw_plain(font, text, pos + Vec2::splat(scale), scale, BLACK);
    }
    draw_plain(font, text, pos, scale, color);
}

fn draw_plain(font: &Atlas, text: &str, pos: Vec2, scale: f32, color: Color) {
    let size = vec2(ADVANCE, 8.0) * scale;
    for (i, c) in text.chars().enumerate() {
        if let Some(name) = glyph_name(c) {
            font.draw_tinted(
                name,
                pos + vec2(i as f32 * ADVANCE * scale, 0.0),
                size,
                color,
            );
        }
    }
}

/// Draws `text` horizontally centred on `centre_x`.
pub fn draw_centred(font: &Atlas, text: &str, centre_x: f32, y: f32, scale: f32, color: Color) {
    let x = centre_x - width(text, scale) / 2.0;
    draw(font, text, vec2(x, y), scale, color, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_name_is_in_the_font() {
        let idx =
            super::super::atlas::parse_index(include_str!("../../assets/gen/font.idx")).unwrap();
        let has = |n: &str| idx.iter().any(|(m, _)| m == n);
        for c in (' '..='~').chain(['é']) {
            if let Some(n) = glyph_name(c) {
                assert!(has(n), "{c:?} -> {n}");
            }
        }
    }

    #[test]
    fn widths() {
        assert_eq!(width("", 1.0), 0.0);
        assert_eq!(width("A", 1.0), 5.0);
        assert_eq!(width("AB", 2.0), 22.0);
    }
}
