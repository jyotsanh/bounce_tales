//! Sprite atlases built by `tools/artgen`: one PNG plus an index of named
//! rectangles. Embedded in the binary so the game is a single file.

use std::collections::HashMap;

use macroquad::prelude::*;

/// Parses an artgen `.idx` file (`name x y w h` per line), keeping sheet
/// order (the position is the tile id).
pub fn parse_index(text: &str) -> Result<Vec<(String, Rect)>, String> {
    let mut list = Vec::new();
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
        list.push((
            name.to_owned(),
            Rect::new(num(x)?, num(y)?, num(w)?, num(h)?),
        ));
    }
    Ok(list)
}

pub struct Atlas {
    pub texture: Texture2D,
    /// In sheet order (index = tile id).
    rects: Vec<Rect>,
    by_name: HashMap<String, usize>,
}

impl Atlas {
    /// Loads an embedded atlas. Panics on bad data: assets are compiled in
    /// and checked by tests, so this cannot fail for a user.
    pub fn from_bytes(name: &str, png: &[u8], index: &str) -> Atlas {
        let texture = Texture2D::from_file_with_format(png, Some(ImageFormat::Png));
        texture.set_filter(FilterMode::Nearest);
        let list = parse_index(index).unwrap_or_else(|e| panic!("atlas {name}: {e}"));
        let by_name = list
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.clone(), i))
            .collect();
        let rects = list.into_iter().map(|(_, r)| r).collect();
        Atlas {
            texture,
            rects,
            by_name,
        }
    }

    /// Source rectangle of a sprite. Panics if the name is unknown (a typo
    /// in code, caught the first time that sprite is drawn).
    pub fn rect(&self, sprite: &str) -> Rect {
        let i = *self
            .by_name
            .get(sprite)
            .unwrap_or_else(|| panic!("no sprite named '{sprite}'"));
        self.rects[i]
    }

    /// Draws a sprite with its top-left corner at `pos` (rounded to whole
    /// pixels so pixel art stays crisp).
    pub fn draw(&self, sprite: &str, pos: Vec2) {
        self.draw_rect(self.rect(sprite), pos);
    }

    /// Draws the sprite at a sheet position (Tiled tile id). Out-of-range
    /// ids are skipped (the level loader and tests guard against them).
    pub fn draw_index(&self, id: usize, pos: Vec2) {
        if let Some(&r) = self.rects.get(id) {
            self.draw_rect(r, pos);
        }
    }

    /// Draws a sprite stretched to `size`, tinted (multiplied) by `color`.
    pub fn draw_tinted(&self, sprite: &str, pos: Vec2, size: Vec2, color: Color) {
        let pos = pos.round();
        draw_texture_ex(
            &self.texture,
            pos.x,
            pos.y,
            color,
            DrawTextureParams {
                source: Some(self.rect(sprite)),
                dest_size: Some(size.round()),
                ..Default::default()
            },
        );
    }

    /// Draws a sprite rotated by `angle` (radians, clockwise) about `pivot`
    /// (relative to its top-left).
    pub fn draw_rotated_about(&self, sprite: &str, pos: Vec2, size: Vec2, angle: f32, pivot: Vec2) {
        draw_texture_ex(
            &self.texture,
            pos.x.round(),
            pos.y.round(),
            WHITE,
            DrawTextureParams {
                source: Some(self.rect(sprite)),
                dest_size: Some(size),
                rotation: angle,
                pivot: Some(pos.round() + pivot),
                ..Default::default()
            },
        );
    }

    /// Draws a sprite stretched to `size`, rotated by `angle` (radians,
    /// clockwise) about its centre.
    pub fn draw_rotated(&self, sprite: &str, pos: Vec2, size: Vec2, angle: f32) {
        draw_texture_ex(
            &self.texture,
            pos.x.round(),
            pos.y.round(),
            WHITE,
            DrawTextureParams {
                source: Some(self.rect(sprite)),
                dest_size: Some(size),
                rotation: angle,
                ..Default::default()
            },
        );
    }

    /// Draws a sprite stretched to `size` (whole pixels).
    pub fn draw_scaled(&self, sprite: &str, pos: Vec2, size: Vec2) {
        let pos = pos.round();
        draw_texture_ex(
            &self.texture,
            pos.x,
            pos.y,
            WHITE,
            DrawTextureParams {
                source: Some(self.rect(sprite)),
                dest_size: Some(size.round()),
                ..Default::default()
            },
        );
    }

    fn draw_rect(&self, source: Rect, pos: Vec2) {
        let pos = pos.round();
        draw_texture_ex(
            &self.texture,
            pos.x,
            pos.y,
            WHITE,
            DrawTextureParams {
                source: Some(source),
                ..Default::default()
            },
        );
    }
}

/// All sprite atlases, embedded at compile time.
pub struct Sprites {
    pub ball: Atlas,
    pub forest: Atlas,
    pub deco: Atlas,
    pub objects: Atlas,
    pub items: Atlas,
    pub icons: Atlas,
    pub font: Atlas,
    pub seesaw: Atlas,
    pub cannon: Atlas,
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
            deco: embedded_atlas!("deco"),
            objects: embedded_atlas!("objects"),
            items: embedded_atlas!("items"),
            icons: embedded_atlas!("icons"),
            font: embedded_atlas!("font"),
            seesaw: embedded_atlas!("seesaw"),
            cannon: embedded_atlas!("cannon"),
        }
    }

    /// Atlas for a Tiled tileset name. Panics on an unknown tileset (levels
    /// are checked by `level` tests to use only known tilesets).
    pub fn by_name(&self, name: &str) -> &Atlas {
        match name {
            "ball" => &self.ball,
            "forest" => &self.forest,
            "deco" => &self.deco,
            "objects" => &self.objects,
            "items" => &self.items,
            "icons" => &self.icons,
            "font" => &self.font,
            "seesaw" => &self.seesaw,
            "cannon" => &self.cannon,
            _ => panic!("no atlas named '{name}'"),
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
        assert_eq!(m[1], ("b".to_owned(), Rect::new(16.0, 0.0, 16.0, 16.0)));
        assert!(parse_index("a 0 0 16").is_err());
        assert!(parse_index("a 0 0 16 x").is_err());
    }

    #[test]
    fn embedded_objects_index_has_entity_sprites() {
        let mut names = Vec::new();
        for idx in [
            include_str!("../../assets/gen/objects.idx"),
            include_str!("../../assets/gen/items.idx"),
            include_str!("../../assets/gen/icons.idx"),
        ] {
            names.extend(parse_index(idx).unwrap().into_iter().map(|(n, _)| n));
        }
        for name in super::super::ENTITY_SPRITES {
            assert!(names.iter().any(|n| n == name), "{name}");
        }
    }

    #[test]
    fn embedded_ball_index_has_every_roll_frame() {
        let ball = parse_index(include_str!("../../assets/gen/ball.idx")).unwrap();
        for k in 0..super::super::BALL_FRAMES {
            assert!(
                ball.iter().any(|(n, _)| *n == format!("roll_{k}")),
                "roll_{k}"
            );
        }
    }
}
