//! Drawing. Everything is drawn into a fixed 240x320 render target (the
//! original Nokia screen), which is then scaled by a whole number onto the
//! window so pixels stay crisp.

mod atlas;

use macroquad::prelude::*;

use atlas::Sprites;

use bounce_tales::sim::Sim;
use bounce_tales::sim::camera::{VIEW_H, VIEW_W};
use bounce_tales::sim::map::{Side, TILE, Tile, TileMap};

pub const SCREEN_W: u32 = VIEW_W as u32;
pub const SCREEN_H: u32 = VIEW_H as u32;

const SKY: Color = Color::new(0.55, 0.80, 0.95, 1.0);

/// Roll frames in the ball atlas (`roll_0` … `roll_15`).
pub const BALL_FRAMES: usize = 16;

/// Every forest tile name the auto-tiler can pick (checked against the
/// atlas index in tests).
#[cfg(test)]
pub const FOREST_TILES: &[&str] = &[
    "grass_top",
    "grass_left",
    "grass_right",
    "grass_platform",
    "dirt",
    "dirt_stones",
    "dirt_bottom",
    "dirt_roots",
    "slope45_r",
    "slope45_l",
    "slope2_r_low",
    "slope2_r_high",
    "slope2_l_high",
    "slope2_l_low",
];
const LETTERBOX: Color = Color::new(0.05, 0.05, 0.07, 1.0);

/// Where the game image goes on the window: largest whole-number scale that
/// fits, centred. Returns (x, y, scale) in window pixels.
pub fn letterbox(window_w: f32, window_h: f32) -> (f32, f32, f32) {
    let scale = (window_w / SCREEN_W as f32)
        .min(window_h / SCREEN_H as f32)
        .floor()
        .max(1.0);
    let w = SCREEN_W as f32 * scale;
    let h = SCREEN_H as f32 * scale;
    (
        ((window_w - w) / 2.0).floor(),
        ((window_h - h) / 2.0).floor(),
        scale,
    )
}

pub struct Renderer {
    target: RenderTarget,
    sprites: Sprites,
    /// Fixed camera for screen-space drawing (HUD, debug overlay).
    screen_camera: Camera2D,
}

impl Renderer {
    pub fn new() -> Renderer {
        let target = render_target(SCREEN_W, SCREEN_H);
        target.texture.set_filter(FilterMode::Nearest);
        let screen_camera = target_camera(&target, Vec2::ZERO);
        Renderer {
            target,
            sprites: Sprites::load(),
            screen_camera,
        }
    }

    /// Draws the game into the 240x320 target. `alpha` blends moving things
    /// between the last two sim steps so motion stays smooth at any FPS.
    pub fn draw_game(&self, sim: &Sim, alpha: f32) {
        let view = sim.camera.render_pos(alpha);
        set_camera(&target_camera(&self.target, view));
        clear_background(SKY);

        draw_tiles(&self.sprites.forest, &sim.map, view);

        let b = &sim.ball;
        let p = b.render_pos(alpha);
        let frame = ball_frame(b.angle);
        self.sprites
            .ball
            .draw(&format!("roll_{frame}"), p - Vec2::splat(b.radius));
    }

    /// F1 overlay: frame rate, tick, ball state and held input.
    pub fn draw_debug(&self, sim: &Sim, fps: i32) {
        set_camera(&self.screen_camera);
        let b = &sim.ball;
        let lines = [
            format!("fps {fps:3}  tick {}", sim.tick),
            format!("pos {:7.2} {:7.2}", b.pos.x, b.pos.y),
            format!("vel {:7.2} {:7.2}", b.vel.x, b.vel.y),
            format!("cam {:7.2} {:7.2}", sim.camera.pos.x, sim.camera.pos.y),
            format!(
                "in  {:3} {}",
                sim.input.to_code(),
                if b.grounded { "ground" } else { "air" }
            ),
        ];
        let line_h = 10.0;
        draw_rectangle(
            0.0,
            0.0,
            130.0,
            lines.len() as f32 * line_h + 4.0,
            Color::new(0.0, 0.0, 0.0, 0.6),
        );
        for (i, line) in lines.iter().enumerate() {
            draw_text(line, 3.0, 10.0 + i as f32 * line_h, 14.0, WHITE);
        }
    }

    /// Scales the game target onto the window.
    pub fn present(&self) {
        set_default_camera();
        clear_background(LETTERBOX);
        let (x, y, scale) = letterbox(screen_width(), screen_height());
        draw_texture_ex(
            &self.target.texture,
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(SCREEN_W as f32 * scale, SCREEN_H as f32 * scale)),
                // Render-target textures are stored bottom-up; flip so the
                // window matches the screenshot (which export_png flips itself).
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// Saves the current game target as a PNG (240x320).
    pub fn save_screenshot(&self, path: &str) {
        self.target.texture.get_texture_data().export_png(path);
    }
}

/// Camera drawing into the game target with `view` (level pixels) at the
/// top-left corner.
fn target_camera(target: &RenderTarget, view: Vec2) -> Camera2D {
    let mut cam = Camera2D::from_display_rect(Rect::new(view.x, view.y, VIEW_W, VIEW_H));
    cam.render_target = Some(target.clone());
    cam
}

/// Roll frame for a ball angle (radians, clockwise): nearest of
/// `BALL_FRAMES` evenly spaced frames.
pub fn ball_frame(angle: f32) -> usize {
    let step = std::f32::consts::TAU / BALL_FRAMES as f32;
    (angle.rem_euclid(std::f32::consts::TAU) / step).round() as usize % BALL_FRAMES
}

/// Is this side of tile (x, y) open to empty space (not covered by the
/// neighbour)?
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
/// neighbours. Built-in ASCII maps only store shapes; Tiled levels (step 8)
/// will name pictures directly.
pub fn forest_tile(map: &TileMap, x: i32, y: i32) -> Option<&'static str> {
    Some(match map.get(x, y) {
        Tile::Empty => return None,
        Tile::Slope45Right => "slope45_r",
        Tile::Slope45Left => "slope45_l",
        Tile::Slope2RightLow => "slope2_r_low",
        Tile::Slope2RightHigh => "slope2_r_high",
        Tile::Slope2LeftHigh => "slope2_l_high",
        Tile::Slope2LeftLow => "slope2_l_low",
        Tile::Solid => {
            // The map's top row is the edge of the world, not open sky.
            let top = y > 0 && side_open(map, x, y, Side::Top);
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

/// Draws the tiles inside the view.
fn draw_tiles(tiles: &atlas::Atlas, map: &TileMap, view: Vec2) {
    let (x0, y0) = TileMap::cell_at(view);
    let (x1, y1) = TileMap::cell_at(view + vec2(VIEW_W, VIEW_H));
    for ty in y0.max(0)..=y1.min(map.height as i32 - 1) {
        for tx in x0.max(0)..=x1.min(map.width as i32 - 1) {
            if let Some(name) = forest_tile(map, tx, ty) {
                tiles.draw(name, vec2(tx as f32, ty as f32) * TILE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_exact_fit() {
        assert_eq!(letterbox(480.0, 640.0), (0.0, 0.0, 2.0));
    }

    #[test]
    fn letterbox_centres_with_integer_scale() {
        // 1920x1080: height limits to 3x (960 tall), 720 wide.
        assert_eq!(letterbox(1920.0, 1080.0), (600.0, 60.0, 3.0));
    }

    #[test]
    fn ball_frames_follow_the_angle() {
        use std::f32::consts::TAU;
        assert_eq!(ball_frame(0.0), 0);
        assert_eq!(ball_frame(TAU / 16.0), 1);
        assert_eq!(ball_frame(TAU * 0.5), 8);
        assert_eq!(ball_frame(TAU - 0.01), 0, "wraps around");
        assert_eq!(ball_frame(-TAU / 16.0), 15);
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
        assert_eq!(forest_tile(&m, 0, 0), None);
        assert_eq!(forest_tile(&m, 3, 1), Some("grass_platform"));
        assert_eq!(
            forest_tile(&m, 0, 4),
            Some("grass_top"),
            "map edge counts as wall"
        );
        assert_eq!(forest_tile(&m, 5, 3), Some("slope2_r_low"));
        // Solid right of the high slope half: covered on the left by the
        // slope's full right side, open on top.
        assert_eq!(forest_tile(&m, 6, 4), Some("grass_top"));
        assert!(forest_tile(&m, 2, 5).unwrap().starts_with("dirt"));
    }

    #[test]
    fn auto_tile_ends() {
        let m = TileMap::from_ascii("o.....\n..##..\n..##..").unwrap();
        assert_eq!(forest_tile(&m, 2, 1), Some("grass_left"));
        let walls = TileMap::from_ascii("#o#\n#.#\n###").unwrap();
        assert!(
            forest_tile(&walls, 0, 0).unwrap().starts_with("dirt"),
            "no grass at map top"
        );
        assert_eq!(forest_tile(&m, 3, 1), Some("grass_right"));
        assert!(m.height == 3);
    }

    #[test]
    fn letterbox_never_below_one() {
        assert_eq!(letterbox(100.0, 100.0).2, 1.0);
    }
}
