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

    /// Shape names used by the `shape` tile property in Tiled tilesets.
    pub fn from_shape_name(name: &str) -> Option<Tile> {
        Some(match name {
            "solid" => Tile::Solid,
            "slope45_r" => Tile::Slope45Right,
            "slope45_l" => Tile::Slope45Left,
            "slope2_r_low" => Tile::Slope2RightLow,
            "slope2_r_high" => Tile::Slope2RightHigh,
            "slope2_l_high" => Tile::Slope2LeftHigh,
            "slope2_l_low" => Tile::Slope2LeftLow,
            _ => return None,
        })
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
    /// Row-major [`flags`] per cell (hazards, water…).
    flags: Vec<u8>,
    /// Where the ball starts (centre, pixels).
    pub spawn: Vec2,
}

/// ASCII letters that mark objects (exit, checkpoint, gem, egg, extra
/// life, moving platforms, switch, door, bridge, spring, catapults, fan,
/// crate, seesaw);
/// they are empty space
/// in the collision map. See `level::ascii_marker`.
pub const OBJECT_MARKERS: &[char] = &[
    'E', 'C', '*', 'e', '+', 'H', 'V', 'S', 'D', 'B', 'j', '>', '<', 'F', 'X', 'W',
];

/// Per-cell flag bits from tile properties (used from steps 11 and 15).
pub mod flags {
    /// `spike=true`: spikes pointing up from the bottom of the cell;
    /// touching them pops the ball.
    pub const SPIKE: u8 = 1;
    /// `water=true`: the ball floats in it.
    pub const WATER: u8 = 2;
    /// `spike_down=true`: spikes hanging from the top of the cell.
    pub const SPIKE_DOWN: u8 = 4;
    /// `crumble=true`: a solid block that falls away shortly after the
    /// ball stands on it, and comes back later.
    pub const CRUMBLE: u8 = 8;
    /// `cracked=true`: a solid block a heavy ball smashes for good.
    pub const CRACKED: u8 = 16;
}

impl TileMap {
    /// Parses a map drawn in text, one row per line. Tile glyphs are listed
    /// on [`Tile`]; `o` marks the ball start (an empty tile, ball resting on
    /// the tile's bottom edge). `^` / `v` are spikes pointing up / down,
    /// `~` is water (no collision shape), `x` a crumbling block and `k` a
    /// cracked block (both solid). All rows must be the same length. Object
    /// markers ([`OBJECT_MARKERS`]) are empty space here;
    /// `level::Level::from_ascii` turns them into objects.
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
        let mut cell_flags = vec![0u8; width * height];
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
                    c if OBJECT_MARKERS.contains(&c) => Tile::Empty,
                    '^' => {
                        cell_flags[y * width + x] = flags::SPIKE;
                        Tile::Empty
                    }
                    'v' => {
                        cell_flags[y * width + x] = flags::SPIKE_DOWN;
                        Tile::Empty
                    }
                    '~' => {
                        cell_flags[y * width + x] = flags::WATER;
                        Tile::Empty
                    }
                    'x' => {
                        cell_flags[y * width + x] = flags::CRUMBLE;
                        Tile::Solid
                    }
                    'k' => {
                        cell_flags[y * width + x] = flags::CRACKED;
                        Tile::Solid
                    }
                    other => Tile::from_glyph(other)
                        .ok_or_else(|| format!("unknown tile '{other}' at {x},{y}"))?,
                });
            }
        }
        TileMap::from_parts(
            width,
            height,
            tiles,
            cell_flags,
            spawn.ok_or("map has no 'o' spawn")?,
        )
    }

    /// Builds a map from row-major cell data (used by the Tiled loader).
    pub fn from_parts(
        width: usize,
        height: usize,
        tiles: Vec<Tile>,
        flags: Vec<u8>,
        spawn: Vec2,
    ) -> Result<TileMap, String> {
        if width == 0 || height == 0 {
            return Err("empty map".into());
        }
        if tiles.len() != width * height || flags.len() != width * height {
            return Err("cell data does not match map size".into());
        }
        Ok(TileMap {
            width,
            height,
            tiles,
            flags,
            spawn,
        })
    }

    /// Flag bits of a cell (0 outside the map). See [`flags`].
    pub fn flags(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return 0;
        }
        self.flags[y as usize * self.width + x as usize]
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

    /// Changes a cell's shape (no-op outside the map). Used by blocks that
    /// break, crumble, open or appear during play.
    pub fn set(&mut self, x: i32, y: i32, tile: Tile) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            self.tiles[y as usize * self.width + x as usize] = tile;
        }
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

/// ASCII text of a built-in map (small test maps; real levels are Tiled
/// files, see `level`).
pub fn builtin_text(name: &str) -> Option<&'static str> {
    Some(match name {
        "test" => TEST_MAP,
        "flat" => FLAT_MAP,
        "slopes" => SLOPES_MAP,
        "hazards" => HAZARDS_MAP,
        "water" => WATER_MAP,
        "platforms" => PLATFORMS_MAP,
        "switches" => SWITCHES_MAP,
        "launchers" => LAUNCHERS_MAP,
        "crates" => CRATES_MAP,
        "seesaws" => SEESAWS_MAP,
        _ => return None,
    })
}

/// A built-in map by name.
pub fn builtin(name: &str) -> Option<TileMap> {
    builtin_text(name).map(|t| TileMap::from_ascii(t).expect("built-in maps are valid"))
}

pub const BUILTIN_NAMES: &[&str] = &[
    "test",
    "flat",
    "slopes",
    "hazards",
    "water",
    "platforms",
    "switches",
    "launchers",
    "crates",
    "seesaws",
];

/// Test map: one screen with a step block, a low ceiling, a tunnel, a
/// staircase of platforms up to an exit, a 45° ramp and a gentle 1:2 hill.
const TEST_MAP: &str = r"
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #.............#
    #..........E..#
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

/// Empty room with a flat floor and an exit by the right wall, for physics
/// replays.
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
    #..o........E.#
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

/// Hazard course (24 wide): floor spikes at x 112..144, a checkpoint
/// (x 152..184, respawn at x 168), a low ceiling with
/// hanging spikes at x 192..240 (bottom of the spikes at y 240), a 2-tile
/// bottomless pit at x 272..304 with an egg in the jump arc, then an exit.
/// Gems after the spikes.
const HAZARDS_MAP: &str = r"
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #...........###........#
    #...........vvv..e.....#
    #......................#
    #........***...........#
    #..o...^^.C...........E#
    #################..#####
    #################..#####
";

/// Pool level (24 wide): land at y 224 on both sides, a 4-deep pool at
/// x 96..288 (surface y 224, bottom y 288), gems over the water, exit on the
/// far bank.
const WATER_MAP: &str = r"
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #......................#
    #.......*.*.*.*........#
    #.o..................E.#
    ######~~~~~~~~~~~~######
    ######~~~~~~~~~~~~######
    ######~~~~~~~~~~~~######
    ######~~~~~~~~~~~~######
    ########################
    ########################
";

/// Moving platforms (30 wide): a ferry (x 96..144 at y 224, travelling 128
/// px right) across a bottomless gap to the right bank (x 272+), then a lift
/// (x 320..368, top y 208 rising to 144) up to a ledge (top y 160) with the
/// exit. Gems hang over the gap.
const PLATFORMS_MAP: &str = r"
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #............................#
    #..........................E.#
    #.......................######
    #........*.*.*...............#
    #............................#
    #.o.................V........#
    ######H..........#############
    ######...........#############
    ######...........#############
    ######...........#############
    ######...........#############
    ######...........#############
";

/// Breakables and gates (40 wide): a crumbling bridge over a bottomless pit
/// (x 128..224), a floor switch (x 256), a 5-tall door under a ceiling
/// (x 320) and a bridge over a second pit (x 416..496) — both worked by the
/// switch — then the exit, and a roofed cracked wall (x 560) hiding an egg.
const SWITCHES_MAP: &str = r"
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #..............###########.............#
    #...................D.............######
    #...................D..............k...#
    #...................D..............k...#
    #...................D..............k...#
    #.o.............S...D............E.k.e.#
    ########xxxxxx############BBBBB#########
    ########......############.....#########
    ########......############.....#########
    ########......############.....#########
    ########......############.....#########
    ########......############.....#########
";

/// Launchers (44 wide): a spring (x 96) up to a ledge (top y 208), a
/// catapult (x 248..280) over a bottomless pit (x 304..400), and a fan
/// updraft (x 536..568, up to y 160) to the exit ledge (top y 176).
const LAUNCHERS_MAP: &str = r"
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #..........................................#
    #.................................*........#
    #........................................E.#
    #...................................########
    #....................*.*...................#
    #.......######.............................#
    #..........................................#
    #..........................................#
    #..........................................#
    #.o...j.........>.................F........#
    ###################......###################
    ###################......###################
";

/// Crates (40 wide): a big crate (x 136..168) on the floor (top y 240)
/// and a 4-tile wall (ledge top y 176 from x 320) too high to jump without
/// standing on the crate; spikes on the ledge, then the exit.
const CRATES_MAP: &str = r"
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #......................................#
    #................................*.....#
    #......................................#
    #.............................^^...E...#
    #...................####################
    #...................####################
    #...................####################
    #.o......X..........####################
    ########################################
    ########################################
    ########################################
    ########################################
    ########################################
";

/// Seesaw (30 wide): banks at y 224, a bottomless pit (x 160..272) with a
/// pillar holding a 6-tile seesaw (x 168..264, pivot at y 224). Crossing
/// tips it one way then the other; the far bank needs a hop from the low
/// end. Exit on the far bank.
const SEESAWS_MAP: &str = r"
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
    #............*...............#
    #............................#
    #............................#
    #.o......................E...#
    ##########...W...#############
    ##########...#...#############
    ##########...#...#############
    ##########...#...#############
    ##########...#...#############
    ##########...#...#############
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        for name in BUILTIN_NAMES {
            let m = builtin(name).unwrap();
            assert!(
                m.width >= 15 && m.height >= 20,
                "{name} smaller than a screen"
            );
        }
        assert!(builtin("nope").is_none());
    }

    #[test]
    fn shape_names() {
        assert_eq!(Tile::from_shape_name("solid"), Some(Tile::Solid));
        assert_eq!(
            Tile::from_shape_name("slope2_l_low"),
            Some(Tile::Slope2LeftLow)
        );
        assert_eq!(Tile::from_shape_name("round"), None);
    }

    #[test]
    fn breakable_glyphs_and_set() {
        let mut m = TileMap::from_ascii("o.\nxk").unwrap();
        assert_eq!((m.get(0, 1), m.flags(0, 1)), (Tile::Solid, flags::CRUMBLE));
        assert_eq!((m.get(1, 1), m.flags(1, 1)), (Tile::Solid, flags::CRACKED));
        m.set(0, 1, Tile::Empty);
        assert_eq!(m.get(0, 1), Tile::Empty);
        m.set(-1, 0, Tile::Empty); // outside: ignored
    }

    #[test]
    fn flags_and_parts() {
        let mut f = vec![0; 4];
        f[3] = flags::SPIKE | flags::WATER;
        let m = TileMap::from_parts(2, 2, vec![Tile::Empty; 4], f, Vec2::ZERO).unwrap();
        assert_eq!(m.flags(1, 1), flags::SPIKE | flags::WATER);
        assert_eq!(m.flags(0, 0), 0);
        assert_eq!(m.flags(5, 5), 0);
        assert!(TileMap::from_parts(2, 2, vec![], vec![], Vec2::ZERO).is_err());
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
