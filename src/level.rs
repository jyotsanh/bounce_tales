//! Levels: a collision [`TileMap`] plus the tile layers to draw.
//!
//! Levels come from two places:
//! - built-in ASCII maps (`sim::map::builtin_text`) for tests, auto-tiled
//!   with forest pictures;
//! - Tiled `.tmx` files in `assets/levels`, embedded at compile time and
//!   listed in [`TMX_LEVELS`].
//!
//! Tiled conventions:
//! - Tiles are 16x16; maps are finite; tiles must not be flipped/rotated.
//! - Tilesets come from `assets/gen/*.tsx` (written by artgen). Each tile
//!   has a `name` and, if it collides, a `shape` property (see
//!   `Tile::from_shape_name`); `spike`/`water` bool properties set flags.
//! - The tile layer named **`ground`** is the collision layer. Every tile
//!   layer is drawn; layers with a bool property `front = true` are drawn in
//!   front of the ball, others behind, in file order.
//! - An object layer holds one point object of type **`spawn`**, placed
//!   where the bottom of the ball should rest.

use std::io::Cursor;
use std::path::Path;

use macroquad::math::{Vec2, vec2};

use crate::sim::ball::BALL_RADIUS;
use crate::sim::map::{self, TILE, Tile, TileMap};
use crate::tileset;

/// Embedded Tiled levels: (name, tmx text).
pub const TMX_LEVELS: &[(&str, &str)] = &[("big", include_str!("../assets/levels/big.tmx"))];

/// Embedded tilesets, looked up by file name from `.tmx` references.
const TILESETS: &[(&str, &str)] = &[("forest.tsx", include_str!("../assets/gen/forest.tsx"))];

/// One drawable tile: which tileset (index into `Level::sheets`) and which
/// sprite in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRef {
    pub sheet: u8,
    pub id: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileLayer {
    pub name: String,
    /// Drawn in front of the ball.
    pub front: bool,
    pub width: usize,
    pub height: usize,
    /// Row-major.
    pub cells: Vec<Option<TileRef>>,
}

impl TileLayer {
    pub fn get(&self, x: i32, y: i32) -> Option<TileRef> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        self.cells[y as usize * self.width + x as usize]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub map: TileMap,
    pub layers: Vec<TileLayer>,
    /// Tileset (= atlas) names that `TileRef::sheet` indexes.
    pub sheets: Vec<String>,
}

/// Every level name: built-in ASCII maps, then Tiled levels.
pub fn level_names() -> Vec<&'static str> {
    let mut v: Vec<&str> = map::BUILTIN_NAMES.to_vec();
    v.extend(TMX_LEVELS.iter().map(|(n, _)| *n));
    v
}

impl Level {
    /// Loads a level by name (Tiled levels win over ASCII maps of the same
    /// name).
    pub fn load(name: &str) -> Result<Level, String> {
        if let Some((_, tmx)) = TMX_LEVELS.iter().find(|(n, _)| *n == name) {
            return Level::from_tmx(name, tmx).map_err(|e| format!("level '{name}': {e}"));
        }
        match map::builtin_text(name) {
            Some(text) => Level::from_ascii(text),
            None => Err(format!(
                "unknown level '{name}' (known: {})",
                level_names().join(", ")
            )),
        }
    }

    /// An ASCII map with one auto-tiled `ground` layer of forest tiles.
    pub fn from_ascii(text: &str) -> Result<Level, String> {
        let map = TileMap::from_ascii(text)?;
        let names = tileset::forest_names();
        let mut cells = Vec::with_capacity(map.width * map.height);
        for y in 0..map.height as i32 {
            for x in 0..map.width as i32 {
                cells.push(tileset::autotile(&map, x, y).map(|n| TileRef {
                    sheet: 0,
                    id: names.iter().position(|m| *m == n).expect("autotile name") as u16,
                }));
            }
        }
        let layer = TileLayer {
            name: "ground".into(),
            front: false,
            width: map.width,
            height: map.height,
            cells,
        };
        Ok(Level {
            map,
            layers: vec![layer],
            sheets: vec!["forest".into()],
        })
    }

    /// Parses Tiled map XML. `name` is only used for the virtual file path.
    pub fn from_tmx(name: &str, tmx: &str) -> Result<Level, String> {
        Level::from_tmx_with(name, tmx, &[])
    }

    /// Like `from_tmx`, with extra tilesets available besides the embedded
    /// ones (for tests).
    fn from_tmx_with(
        name: &str,
        tmx: &str,
        extra_tilesets: &[(&str, &str)],
    ) -> Result<Level, String> {
        let file = format!("{name}.tmx");
        let reader = EmbeddedReader {
            map_file: file.clone(),
            map_text: tmx.to_owned(),
            extra: extra_tilesets
                .iter()
                .map(|(n, t)| (n.to_string(), t.to_string()))
                .collect(),
        };
        let tm = tiled::Loader::with_reader(reader)
            .load_tmx_map(&file)
            .map_err(|e| e.to_string())?;

        if tm.infinite() {
            return Err("infinite maps are not supported".into());
        }
        if tm.tile_width != TILE as u32 || tm.tile_height != TILE as u32 {
            return Err(format!("tiles must be {TILE}x{TILE}"));
        }
        let (w, h) = (tm.width as usize, tm.height as usize);
        let sheets: Vec<String> = tm.tilesets().iter().map(|t| t.name.clone()).collect();
        if sheets.len() > u8::MAX as usize {
            return Err("too many tilesets".into());
        }

        let mut shapes = vec![Tile::Empty; w * h];
        let mut cell_flags = vec![0u8; w * h];
        let mut layers = Vec::new();
        let mut has_ground = false;
        let mut spawn: Option<Vec2> = None;

        for layer in tm.layers() {
            match layer.layer_type() {
                tiled::LayerType::Tiles(tiles) => {
                    let ground = layer.name == "ground";
                    if ground {
                        if has_ground {
                            return Err("more than one 'ground' layer".into());
                        }
                        has_ground = true;
                    }
                    let front = matches!(
                        layer.properties.get("front"),
                        Some(tiled::PropertyValue::BoolValue(true))
                    );
                    let mut cells = Vec::with_capacity(w * h);
                    for y in 0..h as i32 {
                        for x in 0..w as i32 {
                            let Some(t) = tiles.get_tile(x, y) else {
                                cells.push(None);
                                continue;
                            };
                            if t.flip_h || t.flip_v || t.flip_d {
                                return Err(format!(
                                    "layer '{}': flipped tile at {x},{y} (not supported)",
                                    layer.name
                                ));
                            }
                            cells.push(Some(TileRef {
                                sheet: t.tileset_index() as u8,
                                id: t.id() as u16,
                            }));
                            if ground {
                                let i = y as usize * w + x as usize;
                                let data = t.get_tile();
                                let props = data.as_ref().map(|d| &d.properties);
                                let prop = |k: &str| props.and_then(|p| p.get(k));
                                if let Some(v) = prop("shape") {
                                    let tiled::PropertyValue::StringValue(s) = v else {
                                        return Err("'shape' must be a string".into());
                                    };
                                    shapes[i] = Tile::from_shape_name(s)
                                        .ok_or_else(|| format!("unknown shape '{s}'"))?;
                                }
                                let is_true = |k: &str| {
                                    matches!(prop(k), Some(tiled::PropertyValue::BoolValue(true)))
                                };
                                if is_true("spike") {
                                    cell_flags[i] |= map::flags::SPIKE;
                                }
                                if is_true("water") {
                                    cell_flags[i] |= map::flags::WATER;
                                }
                            }
                        }
                    }
                    layers.push(TileLayer {
                        name: layer.name.clone(),
                        front,
                        width: w,
                        height: h,
                        cells,
                    });
                }
                tiled::LayerType::Objects(objects) => {
                    for obj in objects.objects() {
                        if obj.user_type == "spawn" {
                            if spawn.is_some() {
                                return Err("more than one spawn".into());
                            }
                            // Point marks where the ball's bottom rests.
                            spawn = Some(vec2(obj.x, obj.y - BALL_RADIUS));
                        }
                    }
                }
                tiled::LayerType::Image(_) => {} // backgrounds come later
                tiled::LayerType::Group(_) => {
                    return Err("group layers are not supported".into());
                }
            }
        }
        if !has_ground {
            return Err("no tile layer named 'ground'".into());
        }
        let spawn = spawn.ok_or("no object of type 'spawn'")?;
        Ok(Level {
            map: TileMap::from_parts(w, h, shapes, cell_flags, spawn)?,
            layers,
            sheets,
        })
    }
}

/// Serves the map text and embedded tilesets to the `tiled` loader. Paths
/// are matched by file name, so `../gen/forest.tsx` (as the Tiled editor
/// writes it) finds the embedded copy.
struct EmbeddedReader {
    map_file: String,
    map_text: String,
    extra: Vec<(String, String)>,
}

impl tiled::ResourceReader for EmbeddedReader {
    type Resource = Cursor<Vec<u8>>;
    type Error = std::io::Error;

    fn read_from(&mut self, path: &Path) -> Result<Self::Resource, Self::Error> {
        let file = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if file == self.map_file {
            return Ok(Cursor::new(self.map_text.clone().into_bytes()));
        }
        let extra = self.extra.iter().map(|(n, t)| (n.as_str(), t.as_str()));
        TILESETS
            .iter()
            .copied()
            .chain(extra)
            .find(|(n, _)| *n == file)
            .map(|(_, text)| Cursor::new(text.as_bytes().to_vec()))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("'{}' is not an embedded tileset", path.display()),
                )
            })
    }
}

/// Converts an ASCII map to Tiled XML: one auto-tiled `ground` layer using
/// the forest tileset and a `spawn` point object. Used by the `ascii2tmx`
/// tool; levels are then edited in Tiled.
pub fn ascii_to_tmx(text: &str) -> Result<String, String> {
    let level = Level::from_ascii(text)?;
    let map = &level.map;
    let (w, h) = (map.width, map.height);
    let ground = &level.layers[0];
    let mut csv = String::new();
    for y in 0..h {
        let row: Vec<String> = (0..w)
            .map(|x| {
                ground.cells[y * w + x]
                    .map_or(0, |t| t.id as u32 + 1) // gid = firstgid (1) + id
                    .to_string()
            })
            .collect();
        csv.push_str(&row.join(","));
        if y + 1 < h {
            csv.push(',');
        }
        csv.push('\n');
    }
    let spawn_x = map.spawn.x;
    let spawn_y = map.spawn.y + BALL_RADIUS;
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="{w}" height="{h}" tilewidth="16" tileheight="16" infinite="0" nextlayerid="3" nextobjectid="2">
 <tileset firstgid="1" source="../gen/forest.tsx"/>
 <layer id="1" name="ground" width="{w}" height="{h}">
  <data encoding="csv">
{csv}</data>
 </layer>
 <objectgroup id="2" name="objects">
  <object id="1" name="spawn" type="spawn" x="{spawn_x}" y="{spawn_y}">
   <point/>
  </object>
 </objectgroup>
</map>
"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_loads() {
        for name in level_names() {
            let level = Level::load(name).unwrap_or_else(|e| panic!("{e}"));
            assert!(level.layers.iter().any(|l| l.name == "ground"), "{name}");
            for sheet in &level.sheets {
                assert_eq!(sheet, "forest", "{name}: unknown tileset");
            }
        }
        assert!(Level::load("nope").unwrap_err().contains("known:"));
    }

    #[test]
    fn every_tmx_file_is_registered() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "tmx") {
                let stem = path.file_stem().unwrap().to_str().unwrap();
                assert!(
                    TMX_LEVELS.iter().any(|(n, _)| *n == stem),
                    "{} is not listed in TMX_LEVELS",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn ascii_round_trips_through_tmx() {
        for name in map::BUILTIN_NAMES {
            let text = map::builtin_text(name).unwrap();
            let tmx = ascii_to_tmx(text).unwrap();
            let from_tmx = Level::from_tmx(name, &tmx).unwrap_or_else(|e| panic!("{name}: {e}"));
            let from_ascii = Level::from_ascii(text).unwrap();
            assert_eq!(from_tmx.map, from_ascii.map, "{name}: collision differs");
            assert_eq!(
                from_tmx.layers, from_ascii.layers,
                "{name}: pictures differ"
            );
        }
    }

    #[test]
    fn tile_properties_set_shapes_and_flags() {
        let tsx = r#"<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" name="forest" tilewidth="16" tileheight="16" tilecount="3" columns="3">
 <image source="forest.png" width="48" height="16"/>
 <tile id="0"><properties>
   <property name="shape" value="solid"/>
   <property name="spike" type="bool" value="true"/>
 </properties></tile>
 <tile id="1"><properties>
   <property name="water" type="bool" value="true"/>
 </properties></tile>
 <tile id="2"><properties>
   <property name="shape" value="slope45_l"/>
 </properties></tile>
</tileset>
"#;
        let tmx = ascii_to_tmx("o..\n###")
            .unwrap()
            .replace("../gen/forest.tsx", "test.tsx")
            .replace("0,0,0,\n4,4,4\n", "0,2,0,\n1,0,3\n");
        assert!(tmx.contains("0,2,0,"), "test setup: layer not replaced");
        let level = Level::from_tmx_with("t", &tmx, &[("test.tsx", tsx)]).unwrap();
        let m = &level.map;
        assert_eq!(m.get(0, 1), Tile::Solid);
        assert_eq!(m.flags(0, 1), map::flags::SPIKE);
        assert_eq!(m.get(1, 0), Tile::Empty, "water has no shape");
        assert_eq!(m.flags(1, 0), map::flags::WATER);
        assert_eq!(m.get(2, 1), Tile::Slope45Left);
        assert_eq!(m.get(1, 1), Tile::Empty);
        assert_eq!(level.layers[0].get(1, 0), Some(TileRef { sheet: 0, id: 1 }));
    }

    #[test]
    fn spawn_point_is_where_the_ball_rests() {
        let tmx = ascii_to_tmx("o.\n##").unwrap();
        let level = Level::from_tmx("t", &tmx).unwrap();
        assert_eq!(level.map.spawn, vec2(8.0, 8.0));
    }

    #[test]
    fn front_layers_are_marked() {
        let tmx = ascii_to_tmx("o.\n##").unwrap().replace(
            " </layer>\n",
            " </layer>\n <layer id=\"3\" name=\"leaves\" width=\"2\" height=\"2\">\n  <properties><property name=\"front\" type=\"bool\" value=\"true\"/></properties>\n  <data encoding=\"csv\">\n1,0,\n0,0\n</data>\n </layer>\n",
        );
        let level = Level::from_tmx("t", &tmx).unwrap();
        assert_eq!(level.layers.len(), 2);
        assert!(!level.layers[0].front);
        assert!(level.layers[1].front);
        assert_eq!(level.layers[1].name, "leaves");
        // Only the ground layer collides.
        assert_eq!(level.map.get(0, 0), Tile::Empty);
    }

    #[test]
    fn tmx_errors() {
        let good = ascii_to_tmx("o.\n##").unwrap();
        let no_ground = good.replace(r#"name="ground""#, r#"name="deco""#);
        assert!(
            Level::from_tmx("t", &no_ground)
                .unwrap_err()
                .contains("ground")
        );
        let no_spawn = good.replace(r#"type="spawn""#, r#"type="thing""#);
        assert!(
            Level::from_tmx("t", &no_spawn)
                .unwrap_err()
                .contains("spawn")
        );
        let flipped = good.replace("\n4,4\n", "\n2147483652,4\n"); // 0x80000000 = flip_h
        assert_ne!(flipped, good, "test setup: flip bit not inserted");
        assert!(
            Level::from_tmx("t", &flipped)
                .unwrap_err()
                .contains("flipped")
        );
        let bad_set = good.replace("../gen/forest.tsx", "../gen/nope.tsx");
        assert!(Level::from_tmx("t", &bad_set).is_err());
    }
}
