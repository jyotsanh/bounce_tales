//! Sprite atlases built by `tools/artgen`: one PNG plus an index of named
//! rectangles. Embedded in the binary so the game is a single file.

use std::collections::HashMap;

use macroquad::prelude::*;

/// Parses an artgen `.idx` file (`name x y w h` per line).
pub fn parse_index(text: &str) -> Result<HashMap<String, Rect>, String> {
    let mut map = HashMap::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [name, x, y, w, h] = parts[..] else {
            return Err(format!("index line {}: expected 5 fields", i + 1));
        };
        let num = |s: &str| {
            s.parse::<f32>()
                .map_err(|_| format!("index line {}: bad number '{s}'", i + 1))
        };
        map.insert(
            name.to_owned(),
            Rect::new(num(x)?, num(y)?, num(w)?, num(h)?),
        );
    }
    Ok(map)
}

pub struct Atlas {
    pub texture: Texture2D,
    rects: HashMap<String, Rect>,
}

impl Atlas {
    /// Loads an embedded atlas. Panics on bad data: assets are compiled in
    /// and checked by tests, so this cannot fail for a user.
    pub fn from_bytes(name: &str, png: &[u8], index: &str) -> Atlas {
        let texture = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
        texture.set_filter(FilterMode::Nearest);
        let rects = parse_index(index).unwrap_or_else(|e| panic!("atlas {name}: {e}"));
        Atlas { texture, rects }
    }

    /// Source rectangle of a sprite. Panics if the name is unknown (a typo
    /// in code, caught the first time that sprite is drawn).
    pub fn rect(&self, sprite: &str) -> Rect {
        *self
            .rects
            .get(sprite)
            .unwrap_or_else(|| panic!("no sprite named '{sprite}'"))
    }

    /// Draws a sprite with its top-left corner at `pos` (rounded to whole
    /// pixels so pixel art stays crisp).
    pub fn draw(&self, sprite: &str, pos: Vec2) {
        let pos = pos.round();
        draw_texture_ex(
            &self.texture,
            pos.x,
            pos.y,
            WHITE,
            DrawTextureParams {
                source: Some(self.rect(sprite)),
                ..Default::default()
            },
        );
    }
}

/// All sprite atlases, embedded at compile time.
pub struct Sprites {
    pub ball: Atlas,
    pub forest: Atlas,
}

macro_rules! embedded_atlas {
    ($name:literal) => {
        Atlas::from_bytes(
            $name,
            include_bytes!(concat!("../../assets/gen/", $name, ".png")),
            include_str!(concat!("../../assets/gen/", $name, ".idx")),
        )
    };
}

impl Sprites {
    pub fn load() -> Sprites {
        Sprites {
            ball: embedded_atlas!("ball"),
            forest: embedded_atlas!("forest"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_index() {
        let m = parse_index("a 0 0 16 16\nb 16 0 16 16\n\n").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m["b"], Rect::new(16.0, 0.0, 16.0, 16.0));
        assert!(parse_index("a 0 0 16").is_err());
        assert!(parse_index("a 0 0 16 x").is_err());
    }

    #[test]
    fn embedded_indexes_have_the_sprites_code_uses() {
        let ball = parse_index(include_str!("../../assets/gen/ball.idx")).unwrap();
        for k in 0..super::super::BALL_FRAMES {
            assert!(ball.contains_key(&format!("roll_{k}")), "roll_{k}");
        }
        let forest = parse_index(include_str!("../../assets/gen/forest.idx")).unwrap();
        for name in super::super::FOREST_TILES {
            assert!(forest.contains_key(*name), "{name}");
        }
    }
}
