//! Tile map used for collision. Built from Tiled files from step 8; for now
//! from ASCII art via [`TileMap::from_ascii`].

use macroquad::math::{Vec2, vec2};

/// Side of one square tile in pixels. The ball (radius 8) is exactly one
/// tile wide, like the original where the ball fits one-tile tunnels.
pub const TILE: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tile {
    #[default]
    Empty,
    Solid,
}

impl Tile {
    pub fn is_solid(self) -> bool {
        matches!(self, Tile::Solid)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileMap {
    /// Size in tiles.
    pub width: usize,
    pub height: usize,
    /// Row-major, `width * height` entries.
    tiles: Vec<Tile>,
    /// Where the ball starts (centre, pixels).
    pub spawn: Vec2,
}

impl TileMap {
    /// Parses a map drawn in text, one row per line:
    /// `#` solid, `.` empty, `o` ball start (empty tile, ball resting on the
    /// tile's bottom edge). All rows must be the same length.
    pub fn from_ascii(text: &str) -> Result<TileMap, String> {
        let rows: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let height = rows.len();
        let width = rows.first().map_or(0, |r| r.chars().count());
        if width == 0 {
            return Err("empty map".into());
        }
        let mut tiles = Vec::with_capacity(width * height);
        let mut spawn = None;
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return Err(format!("row {y} has a different length"));
            }
            for (x, c) in row.chars().enumerate() {
                tiles.push(match c {
                    '#' => Tile::Solid,
                    '.' => Tile::Empty,
                    'o' => {
                        if spawn.is_some() {
                            return Err("more than one 'o' spawn".into());
                        }
                        // Resting on the floor of this tile cell.
                        spawn = Some(vec2(
                            (x as f32 + 0.5) * TILE,
                            (y as f32 + 1.0) * TILE - super::ball::BALL_RADIUS,
                        ));
                        Tile::Empty
                    }
                    other => return Err(format!("unknown tile '{other}' at {x},{y}")),
                });
            }
        }
        Ok(TileMap {
            width,
            height,
            tiles,
            spawn: spawn.ok_or("map has no 'o' spawn")?,
        })
    }

    /// Map size in pixels.
    pub fn pixel_size(&self) -> Vec2 {
        vec2(self.width as f32 * TILE, self.height as f32 * TILE)
    }

    /// Tile at grid position. Outside the map: the left and right edges act
    /// as solid walls; above and below are open (falling out the bottom is a
    /// pit).
    pub fn get(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.width as i32 {
            return Tile::Solid;
        }
        if y < 0 || y >= self.height as i32 {
            return Tile::Empty;
        }
        self.tiles[y as usize * self.width + x as usize]
    }

    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_solid()
    }

    /// Grid cell containing a pixel position.
    pub fn cell_at(p: Vec2) -> (i32, i32) {
        ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32)
    }
}

/// Built-in maps, by name. Tiled levels join these in step 8.
pub fn builtin(name: &str) -> Option<TileMap> {
    let text = match name {
        "test" => TEST_MAP,
        "flat" => FLAT_MAP,
        _ => return None,
    };
    Some(TileMap::from_ascii(text).expect("built-in maps are valid"))
}

pub const BUILTIN_NAMES: &[&str] = &["test", "flat"];

/// Default map: one screen with a step block, a low ceiling, a tunnel and a
/// staircase of platforms to climb.
const TEST_MAP: &str = "
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #........####.#
    #.............#
    #..####.......#
    #.............#
    ###...........#
    #........######
    #..##.........#
    #.............#
    #...o.##......#
    ###############
    ###############
";

/// Empty room with a flat floor, for physics replays.
const FLAT_MAP: &str = "
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #..o..........#
    ###############
    ###############
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        for name in BUILTIN_NAMES {
            let m = builtin(name).unwrap();
            assert_eq!((m.width, m.height), (15, 20), "{name}");
        }
        assert!(builtin("nope").is_none());
    }

    #[test]
    fn parse_spawn_and_tiles() {
        let m = TileMap::from_ascii("#o.\n###").unwrap();
        assert_eq!(m.spawn, vec2(24.0, 8.0));
        assert!(m.is_solid(0, 0));
        assert!(!m.is_solid(1, 0));
        assert!(m.is_solid(2, 1));
    }

    #[test]
    fn outside_rules() {
        let m = TileMap::from_ascii("o.\n..").unwrap();
        assert!(m.is_solid(-1, 0), "left edge is a wall");
        assert!(m.is_solid(2, 1), "right edge is a wall");
        assert!(!m.is_solid(0, -1), "sky is open");
        assert!(!m.is_solid(0, 2), "bottom is a pit");
    }

    #[test]
    fn parse_errors() {
        assert!(TileMap::from_ascii("").is_err());
        assert!(TileMap::from_ascii("#.\n#").is_err());
        assert!(TileMap::from_ascii("##").is_err(), "no spawn");
        assert!(TileMap::from_ascii("oo").is_err());
        assert!(TileMap::from_ascii("o?").is_err());
    }
}
