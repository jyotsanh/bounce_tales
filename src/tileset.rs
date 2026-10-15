//! Tile pictures known to the game logic: tileset sprite names and ids from
//! the generated atlas index, and the auto-tiler that picks forest pictures
//! for plain shape maps (ASCII maps and the `ascii2tmx` tool).

use crate::sim::map::{Side, Tile, TileMap, flags};

/// Forest tileset sprite names in id order (id = position in the sheet).
pub fn forest_names() -> Vec<&'static str> {
    names_from_index(include_str!("../assets/gen/forest.idx"))
}

/// Sprite names from an artgen `.idx` file, in sheet order.
pub fn names_from_index(idx: &str) -> Vec<&str> {
    idx.lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect()
}

/// Id of a forest tile by name. Panics on an unknown name (a code typo,
/// caught by tests).
pub fn forest_id(name: &str) -> u16 {
    forest_names()
        .iter()
        .position(|n| *n == name)
        .unwrap_or_else(|| panic!("no forest tile named '{name}'")) as u16
}

/// Is this side of tile (x, y) open to empty space (not fully covered by
/// the neighbour)?
fn side_open(map: &TileMap, x: i32, y: i32, side: Side) -> bool {
    let (dx, dy) = side.offset();
    match (
        map.get(x, y).side_span(side),
        map.get(x + dx, y + dy).side_span(side.opposite()),
    ) {
        (Some(_), None) => true,
        (Some((lo, hi)), Some((nlo, nhi))) => nlo > lo || nhi < hi,
        (None, _) => false,
    }
}

/// Picks a forest tile picture for a map cell from its shape and its
/// neighbours.
pub fn autotile(map: &TileMap, x: i32, y: i32) -> Option<&'static str> {
    let f = map.flags(x, y);
    if f & flags::SPIKE != 0 {
        return Some("spikes");
    }
    if f & flags::SPIKE_DOWN != 0 {
        return Some("spikes_down");
    }
    if f & flags::WATER != 0 {
        let water_above = map.flags(x, y - 1) & flags::WATER != 0;
        return Some(if water_above || map.get(x, y - 1).is_solid() {
            "water"
        } else {
            "water_top"
        });
    }
    Some(match map.get(x, y) {
        Tile::Empty => return None,
        Tile::Slope45Right => "slope45_r",
        Tile::Slope45Left => "slope45_l",
        Tile::Slope2RightLow => "slope2_r_low",
        Tile::Slope2RightHigh => "slope2_r_high",
        Tile::Slope2LeftHigh => "slope2_l_high",
        Tile::Slope2LeftLow => "slope2_l_low",
        Tile::Solid if f & flags::CRUMBLE != 0 => "crumble",
        Tile::Solid if f & flags::CRACKED != 0 => "cracked",
        Tile::Solid => {
            // The map's top row is the edge of the world, not open sky, and
            // no grass grows under water.
            let under_water = map.flags(x, y - 1) & flags::WATER != 0;
            let top = y > 0 && !under_water && side_open(map, x, y, Side::Top);
            let bottom = side_open(map, x, y, Side::Bottom);
            let left = side_open(map, x, y, Side::Left);
            let right = side_open(map, x, y, Side::Right);
            match (top, bottom) {
                (true, true) => "grass_platform",
                (true, false) if left && !right => "grass_left",
                (true, false) if right && !left => "grass_right",
                (true, false) => "grass_top",
                (false, true) => "dirt_bottom",
                (false, false) => {
                    // Fixed per-cell variety so the dirt doesn't look tiled.
                    let h =
                        (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
                    match h % 11 {
                        0 => "dirt_stones",
                        1 => "dirt_roots",
                        _ => "dirt",
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_autotile_name_exists() {
        let names = forest_names();
        for m in crate::sim::map::BUILTIN_NAMES {
            let map = crate::sim::map::builtin(m).unwrap();
            for y in 0..map.height as i32 {
                for x in 0..map.width as i32 {
                    if let Some(n) = autotile(&map, x, y) {
                        assert!(names.contains(&n), "{n}");
                    }
                }
            }
        }
        assert_eq!(forest_id("grass_top"), 0);
    }

    #[test]
    fn auto_tiles() {
        let m = TileMap::from_ascii(
            "o.......\n\
             ..###...\n\
             ........\n\
             .....r..\n\
             #####R##\n\
             ########",
        )
        .unwrap();
        assert_eq!(autotile(&m, 0, 0), None);
        assert_eq!(autotile(&m, 3, 1), Some("grass_platform"));
        assert_eq!(
            autotile(&m, 0, 4),
            Some("grass_top"),
            "map edge counts as wall"
        );
        assert_eq!(autotile(&m, 5, 3), Some("slope2_r_low"));
        assert_eq!(autotile(&m, 6, 4), Some("grass_top"));
        assert!(autotile(&m, 2, 5).unwrap().starts_with("dirt"));
    }

    #[test]
    fn auto_tile_ends() {
        let m = TileMap::from_ascii("o.....\n..##..\n..##..").unwrap();
        assert_eq!(autotile(&m, 2, 1), Some("grass_left"));
        assert_eq!(autotile(&m, 3, 1), Some("grass_right"));
        let pool = TileMap::from_ascii("o~.\n###").unwrap();
        assert!(
            autotile(&pool, 1, 1).unwrap().starts_with("dirt"),
            "no grass under water"
        );
        assert_eq!(autotile(&pool, 1, 0), Some("water_top"));
        let walls = TileMap::from_ascii("#o#\n#.#\n###").unwrap();
        assert!(
            autotile(&walls, 0, 0).unwrap().starts_with("dirt"),
            "no grass at map top"
        );
    }
}
