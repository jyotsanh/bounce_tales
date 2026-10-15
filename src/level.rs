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
//!   `Tile::from_shape_name`); `spike`/`spike_down`/`water`/`crumble`/
//!   `cracked` bool properties set flags.
//! - The tile layer named **`ground`** is the collision layer. Every tile
//!   layer is drawn; layers with a bool property `front = true` are drawn in
//!   front of the ball, others behind, in file order.
//! - An object layer holds one point object of type **`spawn`**, placed
//!   where the bottom of the ball should rest.
//! - Other objects with a type become [`ObjectSpec`]s, turned into game
//!   entities by `sim::entities` (e.g. `exit`: a rectangle the ball must
//!   reach). Unknown types are load errors, so typos are caught.
//! - Tile objects without a type (picked from a tileset such as `deco`) are
//!   [`Decoration`]s: pictures placed anywhere, no collision. They are drawn
//!   in front of the ball if the object or its layer has `front = true`.
//!
//! - Map property `background` (string) picks the parallax backdrop; one of
//!   [`BACKGROUNDS`] (default `forest`).
//! - Map property `music` (string) picks the tune; one of
//!   [`crate::audio::music::TRACKS`] (default `forest`; ASCII maps with a
//!   boss play `boss`).
//! - Map property `title` (string) is the name on the world map.
//! - Map property `par` (int, seconds) is the target time for the time
//!   bonus on the results screen (default [`DEFAULT_PAR_SECONDS`]).
//!
//! ASCII maps can mark objects too: `E` = exit, `C` = checkpoint (both
//! 32x32, standing on the floor of that cell, centred on it), `*` = gem,
//! `e` = egg, `+` = extra life (16x16, filling the cell), `H` / `V` =
//! moving platform 3 tiles wide starting at that cell, travelling 8 tiles
//! right / 4 tiles up and back, `S` = switch, `D` = door cell, `B` = bridge
//! cell (ASCII ones are all linked by the name `gate`), `j` = spring, `>` /
//! `<` = catapult firing right / left (32x16), `F` = fan blowing up (air
//! stream 32 wide, 128 tall, standing on the cell's floor), `X` = big
//! crate (32x32), `W` = seesaw (6-tile plank on a 1-tile post), `O` =
//! wheel (4 cars, radius 48, centred on the cell), `Z` = spinner (4-tile
//! plank turning about the cell's centre), `K` = cannon firing up-right by
//! itself, `Q` = swinging cannon that fires on jump, `1` / `2` / `0` =
//! transformer to stone / balloon / normal, `w` = walker (heading left),
//! `y` / `Y` = flyer gliding 4 tiles right / down and back, `J` = jumper,
//! `G` / `g` = shooter facing left / right, `M` = the Hypno Cube (centred
//! on the cell where it hovers).

use std::io::Cursor;
use std::path::Path;

use macroquad::math::{Vec2, vec2};

use crate::sim::ball::BALL_RADIUS;
use crate::sim::map::{self, TILE, Tile, TileMap};
use crate::tileset;

/// Embedded Tiled levels: (name, tmx text).
pub const TMX_LEVELS: &[(&str, &str)] = &[("big", include_str!("../assets/levels/big.tmx"))];

/// Embedded tilesets, looked up by file name from `.tmx` references.
const TILESETS: &[(&str, &str)] = &[
    ("forest.tsx", include_str!("../assets/gen/forest.tsx")),
    ("deco.tsx", include_str!("../assets/gen/deco.tsx")),
];

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

/// A custom property value from Tiled.
#[derive(Debug, Clone, PartialEq)]
pub enum Prop {
    Bool(bool),
    Int(i32),
    Float(f32),
    Str(String),
}

/// A typed object from a level, before the sim turns it into an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSpec {
    /// Tiled object type (`exit`, …).
    pub kind: String,
    pub name: String,
    /// Top-left corner (rectangles) or position (points), level pixels.
    pub pos: Vec2,
    /// Width/height for rectangles, zero otherwise.
    pub size: Vec2,
    /// Polyline/polygon points in level pixels (empty otherwise).
    pub points: Vec<Vec2>,
    /// The points form a closed polygon (not an open polyline).
    pub closed: bool,
    pub props: Vec<(String, Prop)>,
}

impl ObjectSpec {
    pub fn prop(&self, key: &str) -> Option<&Prop> {
        self.props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// A picture placed freely in the level (no collision).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decoration {
    pub tile: TileRef,
    /// Top-left corner, level pixels.
    pub pos: Vec2,
    pub front: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub map: TileMap,
    pub layers: Vec<TileLayer>,
    /// Tileset (= atlas) names that `TileRef::sheet` indexes.
    pub sheets: Vec<String>,
    /// Typed objects, in file order.
    pub objects: Vec<ObjectSpec>,
    pub decorations: Vec<Decoration>,
    /// Parallax backdrop name (one of [`BACKGROUNDS`]).
    pub background: String,
    /// Target time for the time bonus, seconds.
    pub par_seconds: u32,
    /// Music track name (one of [`crate::audio::music::TRACKS`]).
    pub music: String,
    /// Name shown on the world map (map property `title`; default: the
    /// level name in capitals).
    pub title: String,
}

/// Music when a level sets none.
pub const DEFAULT_MUSIC: &str = "forest";

/// Target time when a level sets no `par` property, seconds.
pub const DEFAULT_PAR_SECONDS: u32 = 120;
/// Target time for built-in ASCII maps (all small), seconds.
const ASCII_PAR_SECONDS: u32 = 60;

/// Parallax backdrops the renderer knows.
pub const BACKGROUNDS: &[&str] = &["forest"];

/// Size of the exit gate, px.
pub const EXIT_SIZE: Vec2 = Vec2::new(32.0, 32.0);
/// Size of a collectible (gem, egg, extra life), px.
pub const ITEM_SIZE: Vec2 = Vec2::new(16.0, 16.0);
/// Size of an ASCII crate (`X`: a big 2x2-tile crate standing on the
/// cell's floor, centred on it), px. Tiled crates can be any size.
pub const CRATE_SIZE: Vec2 = Vec2::new(32.0, 32.0);
/// Size of an ASCII seesaw (`W`: a 6-tile plank on a 1-tile post standing
/// on the cell's floor, centred on it), px.
pub const SEESAW_SIZE: Vec2 = Vec2::new(96.0, 16.0);
/// Size of an ASCII transformer (`1` stone, `2` balloon, `0` back to
/// normal; standing on the cell's floor), px.
pub const TRANSFORMER_SIZE: Vec2 = Vec2::new(32.0, 32.0);
/// Size of the boss cube (`M`: centred on the cell, where it hovers), px.
/// Must match `sim::boss::BOSS_SIZE`.
pub const BOSS_BOX: Vec2 = Vec2::new(40.0, 40.0);
/// Size of an ASCII wheel (`O`: radius 48 around the cell's centre), px.
pub const WHEEL_SIZE: Vec2 = Vec2::new(96.0, 96.0);
/// Size of an ASCII spinner (`Z`: a 4-tile plank turning about the cell's
/// centre), px.
pub const SPINNER_SIZE: Vec2 = Vec2::new(64.0, 16.0);
/// Size of an ASCII cannon (`K` fires by itself up-right; `Q` swings and
/// fires on jump), px. `K` shoots steeply (-60°, 500 px/s).
pub const CANNON_SIZE: Vec2 = Vec2::new(32.0, 32.0);
/// Size of an ASCII catapult (`>` / `<`), px.
pub const CATAPULT_SIZE: Vec2 = Vec2::new(32.0, 16.0);
/// Size of an ASCII fan's air stream (`F`: 2 tiles wide, 8 tall, rising from
/// the marker's cell), px.
pub const FAN_SIZE: Vec2 = Vec2::new(32.0, 128.0);
/// Size of a checkpoint post, px.
pub const CHECKPOINT_SIZE: Vec2 = Vec2::new(32.0, 32.0);

/// Path offset for ASCII platform markers: `H` travels 8 tiles right and
/// back, `V` rises 4 tiles and back.
fn ascii_platform(c: char) -> Option<Vec2> {
    match c {
        'H' => Some(vec2(8.0 * TILE, 0.0)),
        'V' => Some(vec2(0.0, -4.0 * TILE)),
        _ => None,
    }
}

/// An object an ASCII marker letter stands for.
struct Marker {
    kind: &'static str,
    name: &'static str,
    size: Vec2,
    props: Vec<(String, Prop)>,
    /// Centred on the cell's centre instead of standing on its floor.
    centred: bool,
}

/// Object type, name, size and properties for an ASCII marker letter.
/// Switches, doors and bridges are all linked by the name "gate".
fn ascii_marker(c: char) -> Option<Marker> {
    let m = |kind, name, size| Marker {
        kind,
        name,
        size,
        props: Vec::new(),
        centred: false,
    };
    let centred = |kind, size| Marker {
        centred: true,
        ..m(kind, kind, size)
    };
    let dir = |d: &str| vec![("dir".to_owned(), Prop::Str(d.to_owned()))];
    Some(match c {
        'E' => m("exit", "exit", EXIT_SIZE),
        'C' => m("checkpoint", "checkpoint", CHECKPOINT_SIZE),
        '*' => m("gem", "gem", ITEM_SIZE),
        'e' => m("egg", "egg", ITEM_SIZE),
        '+' => m("life", "life", ITEM_SIZE),
        'S' => Marker {
            props: vec![("target".into(), Prop::Str("gate".into()))],
            ..m("switch", "switch", ITEM_SIZE)
        },
        'D' => m("door", "gate", ITEM_SIZE),
        'B' => m("bridge", "gate", ITEM_SIZE),
        'j' => m("spring", "spring", ITEM_SIZE),
        '>' => Marker {
            props: dir("right"),
            ..m("catapult", "catapult", CATAPULT_SIZE)
        },
        '<' => Marker {
            props: dir("left"),
            ..m("catapult", "catapult", CATAPULT_SIZE)
        },
        'F' => m("fan", "fan", FAN_SIZE),
        'X' => m("crate", "crate", CRATE_SIZE),
        'W' => m("seesaw", "seesaw", SEESAW_SIZE),
        'O' => centred("wheel", WHEEL_SIZE),
        // Counter-clockwise, so crossing left to right its far end rises.
        'Z' => Marker {
            props: vec![("speed".into(), Prop::Int(-45))],
            ..centred("spinner", SPINNER_SIZE)
        },
        '1' => Marker {
            props: vec![("form".into(), Prop::Str("stone".into()))],
            ..m("transformer", "transformer", TRANSFORMER_SIZE)
        },
        '2' => Marker {
            props: vec![("form".into(), Prop::Str("balloon".into()))],
            ..m("transformer", "transformer", TRANSFORMER_SIZE)
        },
        '0' => Marker {
            props: vec![("form".into(), Prop::Str("normal".into()))],
            ..m("transformer", "transformer", TRANSFORMER_SIZE)
        },
        'w' => m("walker", "walker", ITEM_SIZE),
        'y' => m("flyer", "flyer", ITEM_SIZE),
        'Y' => Marker {
            props: vec![("axis".into(), Prop::Str("y".into()))],
            ..m("flyer", "flyer", ITEM_SIZE)
        },
        'J' => m("jumper", "jumper", ITEM_SIZE),
        'M' => centred("boss", BOSS_BOX),
        'G' => m("shooter", "shooter", ITEM_SIZE),
        'g' => Marker {
            props: vec![("dir".into(), Prop::Str("right".into()))],
            ..m("shooter", "shooter", ITEM_SIZE)
        },
        'K' => Marker {
            props: vec![
                ("angle".into(), Prop::Int(-60)),
                ("power".into(), Prop::Int(500)),
            ],
            ..m("cannon", "cannon", CANNON_SIZE)
        },
        'Q' => Marker {
            props: vec![
                ("fire".into(), Prop::Str("jump".into())),
                ("swing".into(), Prop::Int(35)),
                ("angle".into(), Prop::Int(-60)),
            ],
            ..m("cannon", "cannon", CANNON_SIZE)
        },
        _ => return None,
    })
}

/// ASCII object markers, replaced by empty space before the map is parsed.
fn ascii_objects(text: &str) -> (String, Vec<ObjectSpec>) {
    let mut objects = Vec::new();
    let rows: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let mut out = String::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            if let Some(Marker {
                kind,
                name,
                size,
                props,
                centred,
            }) = ascii_marker(c)
            {
                let floor = (y as f32 + 1.0) * TILE;
                let centre = vec2(x as f32 + 0.5, y as f32 + 0.5) * TILE;
                let top = if centred {
                    centre.y - size.y / 2.0
                } else {
                    floor - size.y
                };
                objects.push(ObjectSpec {
                    kind: kind.into(),
                    name: name.into(),
                    pos: vec2(centre.x - size.x / 2.0, top),
                    size,
                    points: Vec::new(),
                    closed: false,
                    props,
                });
                out.push('.');
            } else if let Some(offset) = ascii_platform(c) {
                // Platform 3 tiles wide starting at this cell, top flush
                // with the cell's top.
                let start = vec2(x as f32 * TILE + 1.5 * TILE, y as f32 * TILE);
                objects.push(ObjectSpec {
                    kind: "platform".into(),
                    name: "platform".into(),
                    pos: start,
                    size: Vec2::ZERO,
                    points: vec![start, start + offset],
                    closed: false,
                    props: Vec::new(),
                });
                out.push('.');
            } else {
                out.push(c);
            }
        }
        out.push('\n');
    }
    (out, objects)
}

fn convert_props(props: &tiled::Properties) -> Vec<(String, Prop)> {
    let mut v: Vec<(String, Prop)> = props
        .iter()
        .filter_map(|(k, p)| {
            let prop = match p {
                tiled::PropertyValue::BoolValue(b) => Prop::Bool(*b),
                tiled::PropertyValue::IntValue(i) => Prop::Int(*i),
                tiled::PropertyValue::FloatValue(f) => Prop::Float(*f),
                tiled::PropertyValue::StringValue(s) => Prop::Str(s.clone()),
                _ => return None,
            };
            Some((k.clone(), prop))
        })
        .collect();
    // Tiled stores properties in a HashMap: sort for a stable order.
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
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
        let mut level = Level::load_untitled(name)?;
        if level.title.is_empty() {
            level.title = name.to_uppercase();
        }
        Ok(level)
    }

    fn load_untitled(name: &str) -> Result<Level, String> {
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
        let (text, objects) = ascii_objects(text);
        let map = TileMap::from_ascii(&text)?;
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
        let music = if objects.iter().any(|o| o.kind == "boss") {
            "boss".into()
        } else {
            DEFAULT_MUSIC.into()
        };
        Ok(Level {
            map,
            layers: vec![layer],
            sheets: vec!["forest".into()],
            objects,
            decorations: Vec::new(),
            background: BACKGROUNDS[0].into(),
            par_seconds: ASCII_PAR_SECONDS,
            music,
            title: String::new(),
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
        let mut objects = Vec::new();
        let mut decorations = Vec::new();

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
                                if is_true("spike_down") {
                                    cell_flags[i] |= map::flags::SPIKE_DOWN;
                                }
                                if is_true("crumble") {
                                    cell_flags[i] |= map::flags::CRUMBLE;
                                }
                                if is_true("cracked") {
                                    cell_flags[i] |= map::flags::CRACKED;
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
                tiled::LayerType::Objects(group) => {
                    let layer_front = matches!(
                        layer.properties.get("front"),
                        Some(tiled::PropertyValue::BoolValue(true))
                    );
                    for obj in group.objects() {
                        let pos = vec2(obj.x, obj.y);
                        if obj.user_type == "spawn" {
                            if spawn.is_some() {
                                return Err("more than one spawn".into());
                            }
                            // Point marks where the ball's bottom rests.
                            spawn = Some(pos - vec2(0.0, BALL_RADIUS));
                            continue;
                        }
                        if let (Some(t), "") = (obj.tile_data(), obj.user_type.as_str()) {
                            if t.flip_h || t.flip_v || t.flip_d || obj.rotation != 0.0 {
                                return Err(format!(
                                    "decoration '{}' is flipped or rotated (not supported)",
                                    obj.name
                                ));
                            }
                            let tiled::TilesetLocation::Map(sheet) = *t.tileset_location() else {
                                return Err("decorations from templates are not supported".into());
                            };
                            let front = layer_front
                                || matches!(
                                    obj.properties.get("front"),
                                    Some(tiled::PropertyValue::BoolValue(true))
                                );
                            // Tile objects are anchored at their bottom-left.
                            let h = match obj.shape {
                                tiled::ObjectShape::Rect { height, .. } => height,
                                _ => TILE,
                            };
                            decorations.push(Decoration {
                                tile: TileRef {
                                    sheet: sheet as u8,
                                    id: t.id() as u16,
                                },
                                pos: pos - vec2(0.0, h),
                                front,
                            });
                            continue;
                        }
                        if obj.user_type.is_empty() {
                            return Err(format!(
                                "object '{}' at {},{} has no type",
                                obj.name, obj.x, obj.y
                            ));
                        }
                        let abs = |points: &Vec<(f32, f32)>| -> Vec<Vec2> {
                            points.iter().map(|&(x, y)| pos + vec2(x, y)).collect()
                        };
                        let (size, points, closed) = match &obj.shape {
                            tiled::ObjectShape::Rect { width, height } => {
                                (vec2(*width, *height), Vec::new(), false)
                            }
                            tiled::ObjectShape::Polyline { points } => {
                                (Vec2::ZERO, abs(points), false)
                            }
                            tiled::ObjectShape::Polygon { points } => {
                                (Vec2::ZERO, abs(points), true)
                            }
                            _ => (Vec2::ZERO, Vec::new(), false),
                        };
                        objects.push(ObjectSpec {
                            kind: obj.user_type.clone(),
                            name: obj.name.clone(),
                            pos,
                            size,
                            points,
                            closed,
                            props: convert_props(&obj.properties),
                        });
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
        let background = match tm.properties.get("background") {
            None => BACKGROUNDS[0].to_owned(),
            Some(tiled::PropertyValue::StringValue(b)) if BACKGROUNDS.contains(&b.as_str()) => {
                b.clone()
            }
            Some(other) => {
                return Err(format!(
                    "map property 'background' = {other:?}; known: {}",
                    BACKGROUNDS.join(", ")
                ));
            }
        };
        let tracks = crate::audio::music::TRACKS;
        let music = match tm.properties.get("music") {
            None => DEFAULT_MUSIC.to_owned(),
            Some(tiled::PropertyValue::StringValue(m)) if tracks.contains(&m.as_str()) => m.clone(),
            Some(other) => {
                return Err(format!(
                    "map property 'music' = {other:?}; known: {}",
                    tracks.join(", ")
                ));
            }
        };
        let title = match tm.properties.get("title") {
            None => String::new(),
            Some(tiled::PropertyValue::StringValue(t)) => t.to_uppercase(),
            Some(other) => {
                return Err(format!(
                    "map property 'title' must be a string, got {other:?}"
                ));
            }
        };
        let par_seconds = match tm.properties.get("par") {
            None => DEFAULT_PAR_SECONDS,
            Some(tiled::PropertyValue::IntValue(p)) if *p > 0 => *p as u32,
            Some(other) => {
                return Err(format!(
                    "map property 'par' must be a positive int (seconds), got {other:?}"
                ));
            }
        };
        // Catch unknown object types and bad properties at load time.
        crate::sim::entities::build_all(&objects)?;
        Ok(Level {
            map: TileMap::from_parts(w, h, shapes, cell_flags, spawn)?,
            layers,
            sheets,
            objects,
            decorations,
            background,
            par_seconds,
            music,
            title,
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
    let mut extra = String::new();
    for (i, o) in level.objects.iter().enumerate() {
        if o.points.is_empty() {
            let mut props = String::new();
            for (k, v) in &o.props {
                if let Prop::Str(v) = v {
                    props.push_str(&format!("    <property name=\"{k}\" value=\"{v}\"/>\n"));
                }
            }
            let body = if props.is_empty() {
                "/>".to_owned()
            } else {
                format!(">\n   <properties>\n{props}   </properties>\n  </object>")
            };
            extra.push_str(&format!(
                "  <object id=\"{}\" name=\"{}\" type=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{body}\n",
                i + 2,
                o.name,
                o.kind,
                o.pos.x,
                o.pos.y,
                o.size.x,
                o.size.y
            ));
        } else {
            // Paths are stored relative to the object's position.
            let rel: Vec<String> = o
                .points
                .iter()
                .map(|p| format!("{},{}", p.x - o.pos.x, p.y - o.pos.y))
                .collect();
            let tag = if o.closed { "polygon" } else { "polyline" };
            extra.push_str(&format!(
                "  <object id=\"{}\" name=\"{}\" type=\"{}\" x=\"{}\" y=\"{}\">\n   <{tag} points=\"{}\"/>\n  </object>\n",
                i + 2,
                o.name,
                o.kind,
                o.pos.x,
                o.pos.y,
                rel.join(" ")
            ));
        }
    }
    let next_id = level.objects.len() + 2;
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" orientation="orthogonal" renderorder="right-down" width="{w}" height="{h}" tilewidth="16" tileheight="16" infinite="0" nextlayerid="3" nextobjectid="{next_id}">
 <tileset firstgid="1" source="../gen/forest.tsx"/>
 <layer id="1" name="ground" width="{w}" height="{h}">
  <data encoding="csv">
{csv}</data>
 </layer>
 <objectgroup id="2" name="objects">
  <object id="1" name="spawn" type="spawn" x="{spawn_x}" y="{spawn_y}">
   <point/>
  </object>
{extra} </objectgroup>
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
                assert!(
                    ["forest", "deco"].contains(&sheet.as_str()),
                    "{name}: tileset '{sheet}' has no atlas in the renderer"
                );
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
    fn ascii_exit_marker() {
        let level = Level::from_ascii("o.E\n###").unwrap();
        assert_eq!(level.objects.len(), 1);
        let exit = &level.objects[0];
        assert_eq!(exit.kind, "exit");
        assert_eq!(exit.pos, vec2(40.0 - 16.0, 16.0 - 32.0));
        assert_eq!(exit.size, EXIT_SIZE);
        assert_eq!(level.map.get(2, 0), Tile::Empty);
    }

    #[test]
    fn every_object_marker_has_a_kind() {
        for &c in map::OBJECT_MARKERS {
            assert!(
                ascii_marker(c).is_some() || ascii_platform(c).is_some(),
                "{c}"
            );
        }
        let level = Level::from_ascii("oC.\n###").unwrap();
        assert_eq!(level.objects[0].kind, "checkpoint");
    }

    #[test]
    fn decorations_and_typed_objects_from_tmx() {
        let tmx = ascii_to_tmx("o...\n####")
            .unwrap()
            .replace(
                r#"<tileset firstgid="1" source="../gen/forest.tsx"/>"#,
                r#"<tileset firstgid="1" source="../gen/forest.tsx"/>
 <tileset firstgid="15" source="../gen/deco.tsx"/>"#,
            )
            .replace(
                " </objectgroup>",
                r#"  <object id="5" gid="17" x="32" y="16" width="16" height="16"/>
  <object id="6" gid="18" x="48" y="16" width="16" height="16">
   <properties><property name="front" type="bool" value="true"/></properties>
  </object>
  <object id="7" name="goal" type="exit" x="40" y="-16" width="32" height="32">
   <properties><property name="next" value="level2"/></properties>
  </object>
 </objectgroup>"#,
            );
        let level = Level::from_tmx("t", &tmx).unwrap();
        assert_eq!(level.sheets, ["forest", "deco"]);
        assert_eq!(level.decorations.len(), 2);
        let d = level.decorations[0];
        assert_eq!(d.tile, TileRef { sheet: 1, id: 2 });
        assert_eq!(d.pos, vec2(32.0, 0.0), "anchored at the bottom-left");
        assert!(!d.front);
        assert!(level.decorations[1].front);
        assert_eq!(level.objects.len(), 1);
        let exit = &level.objects[0];
        assert_eq!((exit.kind.as_str(), exit.name.as_str()), ("exit", "goal"));
        assert_eq!(exit.pos, vec2(40.0, -16.0));
        assert_eq!(exit.prop("next"), Some(&Prop::Str("level2".into())));
    }

    #[test]
    fn unknown_object_types_fail_to_load() {
        let tmx = ascii_to_tmx("o.\n##").unwrap().replace(
            " </objectgroup>",
            r#"  <object id="5" name="oops" type="exitt" x="0" y="0" width="32" height="32"/>
 </objectgroup>"#,
        );
        let e = Level::from_tmx("t", &tmx).unwrap_err();
        assert!(e.contains("unknown object type 'exitt'"), "{e}");
        let untyped = ascii_to_tmx("o.\n##").unwrap().replace(
            " </objectgroup>",
            r#"  <object id="5" name="what" x="0" y="0" width="32" height="32"/>
 </objectgroup>"#,
        );
        assert!(
            Level::from_tmx("t", &untyped)
                .unwrap_err()
                .contains("no type")
        );
    }

    #[test]
    fn par_property() {
        let tmx = ascii_to_tmx("o.\n##").unwrap();
        assert_eq!(
            Level::from_tmx("t", &tmx).unwrap().par_seconds,
            DEFAULT_PAR_SECONDS
        );
        let with = |v: &str| {
            tmx.replace(
                "<tileset firstgid",
                &format!(
                    "<properties><property name=\"par\" type=\"int\" value=\"{v}\"/></properties>\n <tileset firstgid"
                ),
            )
        };
        assert_eq!(Level::from_tmx("t", &with("45")).unwrap().par_seconds, 45);
        assert!(
            Level::from_tmx("t", &with("0"))
                .unwrap_err()
                .contains("par")
        );
        assert_eq!(Level::from_ascii("o.\n##").unwrap().par_seconds, 60);
    }

    #[test]
    fn background_property() {
        let tmx = ascii_to_tmx("o.\n##").unwrap();
        assert_eq!(Level::from_tmx("t", &tmx).unwrap().background, "forest");
        let with = |v: &str| {
            tmx.replace(
                "<tileset firstgid",
                &format!(
                    "<properties><property name=\"background\" value=\"{v}\"/></properties>\n <tileset firstgid"
                ),
            )
        };
        assert_eq!(
            Level::from_tmx("t", &with("forest")).unwrap().background,
            "forest"
        );
        assert!(
            Level::from_tmx("t", &with("moon"))
                .unwrap_err()
                .contains("background")
        );
    }

    #[test]
    fn music_property() {
        let tmx = ascii_to_tmx("o.\n##").unwrap();
        assert_eq!(Level::from_tmx("t", &tmx).unwrap().music, "forest");
        let with = |v: &str| {
            tmx.replace(
                "<tileset firstgid",
                &format!(
                    "<properties><property name=\"music\" value=\"{v}\"/></properties>\n <tileset firstgid"
                ),
            )
        };
        assert_eq!(Level::from_tmx("t", &with("cave")).unwrap().music, "cave");
        assert!(
            Level::from_tmx("t", &with("disco"))
                .unwrap_err()
                .contains("music")
        );
        assert_eq!(Level::load("hypno").unwrap().music, "boss");
        assert_eq!(Level::load("flat").unwrap().music, "forest");
        assert_eq!(Level::load("flat").unwrap().title, "FLAT");
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
