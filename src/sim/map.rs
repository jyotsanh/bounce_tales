//! Tile map used for collision. Built from Tiled files from step 8; for now
//! from ASCII art via [`TileMap::from_ascii`].

use macroquad::math::{Vec2, vec2};

/// Side of one square tile in pixels. The ball (radius 8) is exactly one
/// tile wide, like the original where the ball fits one-tile tunnels.
pub const TILE: f32 = 16.0;

/// One side of a tile square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// Grid offset to the neighbour across this side.
    pub fn offset(self) -> (i32, i32) {
        match self {
            Side::Top => (0, -1),
            Side::Bottom => (0, 1),
            Side::Left => (-1, 0),
            Side::Right => (1, 0),
        }
    }
}

/// Tile shapes. Slopes are named by which way they rise when read left to
/// right. The 1:2 slopes (about 26.6°) take two tiles: a low half then a
/// high half.
///
/// ASCII glyphs: `#` solid, `/` 45° rising right, `\` 45° rising left,
/// `r` `R` 1:2 rising right (low, high), `L` `l` 1:2 rising left (high, low).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tile {
    #[default]
    Empty,
    Solid,
    Slope45Right,
    Slope45Left,
    Slope2RightLow,
    Slope2RightHigh,
    Slope2LeftHigh,
    Slope2LeftLow,
}

const fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

const T: f32 = TILE;
const H: f32 = TILE / 2.0;

impl Tile {
    pub const ALL: [Tile; 8] = [
        Tile::Empty,
        Tile::Solid,
        Tile::Slope45Right,
        Tile::Slope45Left,
        Tile::Slope2RightLow,
        Tile::Slope2RightHigh,
        Tile::Slope2LeftHigh,
        Tile::Slope2LeftLow,
    ];

    /// True for anything the ball collides with.
    pub fn is_solid(self) -> bool {
        !matches!(self, Tile::Empty)
    }

    pub fn from_glyph(c: char) -> Option<Tile> {
        Some(match c {
            '.' => Tile::Empty,
            '#' => Tile::Solid,
            '/' => Tile::Slope45Right,
            '\\' => Tile::Slope45Left,
            'r' => Tile::Slope2RightLow,
            'R' => Tile::Slope2RightHigh,
            'L' => Tile::Slope2LeftHigh,
            'l' => Tile::Slope2LeftLow,
            _ => return None,
        })
    }

    /// Outline in tile-local pixels (0..16, y down), clockwise on screen.
    /// Empty tiles have no outline.
    pub fn polygon(self) -> &'static [Vec2] {
        const SOLID: [Vec2; 4] = [v(0.0, 0.0), v(T, 0.0), v(T, T), v(0.0, T)];
        const S45R: [Vec2; 3] = [v(T, 0.0), v(T, T), v(0.0, T)];
        const S45L: [Vec2; 3] = [v(0.0, 0.0), v(T, T), v(0.0, T)];
        const S2RL: [Vec2; 3] = [v(T, H), v(T, T), v(0.0, T)];
        const S2RH: [Vec2; 4] = [v(0.0, H), v(T, 0.0), v(T, T), v(0.0, T)];
        const S2LH: [Vec2; 4] = [v(0.0, 0.0), v(T, H), v(T, T), v(0.0, T)];
        const S2LL: [Vec2; 3] = [v(0.0, H), v(T, T), v(0.0, T)];
        match self {
            Tile::Empty => &[],
            Tile::Solid => &SOLID,
            Tile::Slope45Right => &S45R,
            Tile::Slope45Left => &S45L,
            Tile::Slope2RightLow => &S2RL,
            Tile::Slope2RightHigh => &S2RH,
            Tile::Slope2LeftHigh => &S2LH,
            Tile::Slope2LeftLow => &S2LL,
        }
    }

    /// Edges of the outline as (start, end) pairs.
    pub fn edges(self) -> impl Iterator<Item = (Vec2, Vec2)> {
        let poly = self.polygon();
        (0..poly.len()).map(move |i| (poly[i], poly[(i + 1) % poly.len()]))
    }

    /// The part of `side` this tile's outline covers, as a span along that
    /// side (x for top/bottom, y for left/right), or `None`.
    pub fn side_span(self, side: Side) -> Option<(f32, f32)> {
        self.edges()
            .filter_map(|(a, b)| edge_side(a, b))
            .find(|(s, _, _)| *s == side)
            .map(|(_, lo, hi)| (lo, hi))
    }
}

/// Which tile side an edge lies on (if any) and the span it covers.
pub fn edge_side(a: Vec2, b: Vec2) -> Option<(Side, f32, f32)> {
    let span = |p: f32, q: f32| (p.min(q), p.max(q));
    let (side, (lo, hi)) = if a.y == 0.0 && b.y == 0.0 {
        (Side::Top, span(a.x, b.x))
    } else if a.y == T && b.y == T {
        (Side::Bottom, span(a.x, b.x))
    } else if a.x == 0.0 && b.x == 0.0 {
        (Side::Left, span(a.y, b.y))
    } else if a.x == T && b.x == T {
        (Side::Right, span(a.y, b.y))
    } else {
        return None;
    };
    Some((side, lo, hi))
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
    /// Parses a map drawn in text, one row per line. Tile glyphs are listed
    /// on [`Tile`]; `o` marks the ball start (an empty tile, ball resting on
    /// the tile's bottom edge). All rows must be the same length.
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
                    other => Tile::from_glyph(other)
                        .ok_or_else(|| format!("unknown tile '{other}' at {x},{y}"))?,
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

    /// True if the edge `a`-`b` of tile (x, y) is hidden by the neighbour
    /// across it (the neighbour's matching side covers the whole edge).
    /// Hidden edges are not real surfaces and never collide; this is what
    /// stops the ball catching on the seams between tiles.
    pub fn edge_hidden(&self, x: i32, y: i32, a: Vec2, b: Vec2) -> bool {
        let Some((side, lo, hi)) = edge_side(a, b) else {
            return false;
        };
        let (dx, dy) = side.offset();
        self.get(x + dx, y + dy)
            .side_span(side.opposite())
            .is_some_and(|(nlo, nhi)| nlo <= lo + 1e-4 && nhi >= hi - 1e-4)
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
        "slopes" => SLOPES_MAP,
        _ => return None,
    };
    Some(TileMap::from_ascii(text).expect("built-in maps are valid"))
}

pub const BUILTIN_NAMES: &[&str] = &["test", "flat", "slopes"];

/// Default map: one screen with a step block, a low ceiling, a tunnel, a
/// staircase of platforms, a 45° ramp and a gentle 1:2 hill.
const TEST_MAP: &str = r"
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
    #...o.##.rRLl/#
    ###############
    ###############
";

/// Empty room with a flat floor, for physics replays.
const FLAT_MAP: &str = r"
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

/// Valley of slopes for physics replays: a 45° slope down from the left
/// plateau (x 64..144), a flat bottom, a one-tile 1:2 hill (x 192..256),
/// flat, then a 45° slope (x 336..416) up to the right plateau. 30 tiles wide.
const SLOPES_MAP: &str = r"
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #o...........................#
    ####\..................../####
    #####\................../#####
    ######\................/######
    #######\............../#######
    ########\...rRLl...../########
    ##############################
    ##############################
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        for name in BUILTIN_NAMES {
            let m = builtin(name).unwrap();
            assert_eq!(m.height, 20, "{name}");
        }
        assert!(builtin("nope").is_none());
    }

    #[test]
    fn glyphs_round_trip() {
        for c in ['.', '#', '/', '\\', 'r', 'R', 'L', 'l'] {
            assert!(Tile::from_glyph(c).is_some(), "{c}");
        }
        assert!(Tile::from_glyph('x').is_none());
    }

    #[test]
    fn polygons_are_convex_and_clockwise() {
        for t in Tile::ALL {
            let poly = t.polygon();
            for i in 0..poly.len() {
                let a = poly[i];
                let b = poly[(i + 1) % poly.len()];
                let c = poly[(i + 2) % poly.len()];
                let cross = (b - a).perp_dot(c - b);
                assert!(cross > 0.0, "{t:?} not clockwise/convex at {i}");
            }
        }
    }

    #[test]
    fn side_spans() {
        assert_eq!(Tile::Solid.side_span(Side::Left), Some((0.0, T)));
        assert_eq!(Tile::Slope45Right.side_span(Side::Right), Some((0.0, T)));
        assert_eq!(Tile::Slope45Right.side_span(Side::Left), None);
        assert_eq!(Tile::Slope45Right.side_span(Side::Top), None);
        assert_eq!(Tile::Slope2RightLow.side_span(Side::Right), Some((H, T)));
        assert_eq!(Tile::Slope2RightHigh.side_span(Side::Left), Some((H, T)));
    }

    #[test]
    fn hidden_edges() {
        let m = TileMap::from_ascii("o....\n.rR#.\n#####").unwrap();
        let (top, bottom) = (v(0.0, 0.0), v(0.0, T));
        // Low half's right edge is fully covered by the high half's left.
        assert!(m.edge_hidden(1, 1, v(T, T), v(T, H)));
        // High half's left edge likewise covered by the low half.
        assert!(m.edge_hidden(2, 1, v(0.0, T), v(0.0, H)));
        // The high half's right side is full height, so the solid's left
        // face is hidden...
        assert!(m.edge_hidden(3, 1, bottom, top));
        // ...but next to a low half (right side only half covered) it is not.
        let m2 = TileMap::from_ascii("o..\n.r#\n###").unwrap();
        assert!(!m2.edge_hidden(2, 1, bottom, top));
        // Floor tops under slopes are hidden; slope surfaces never are.
        assert!(m.edge_hidden(1, 2, v(0.0, 0.0), v(T, 0.0)));
        assert!(!m.edge_hidden(1, 1, v(0.0, T), v(T, H)));
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
