//! Drawing. Everything is drawn into a fixed 240x320 render target (the
//! original Nokia screen), which is then scaled by a whole number onto the
//! window so pixels stay crisp.

use macroquad::prelude::*;

use bounce_tales::sim::Sim;
use bounce_tales::sim::map::{TILE, TileMap};

pub const SCREEN_W: u32 = 240;
pub const SCREEN_H: u32 = 320;

const SKY: Color = Color::new(0.55, 0.80, 0.95, 1.0);
const GRASS: Color = Color::new(0.30, 0.65, 0.20, 1.0);
const DIRT: Color = Color::new(0.50, 0.33, 0.18, 1.0);
const DIRT_EDGE: Color = Color::new(0.40, 0.25, 0.13, 1.0);
const BALL_RED: Color = Color::new(0.85, 0.10, 0.10, 1.0);
const BALL_DARK: Color = Color::new(0.55, 0.05, 0.05, 1.0);
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
    camera: Camera2D,
}

impl Renderer {
    pub fn new() -> Renderer {
        let target = render_target(SCREEN_W, SCREEN_H);
        target.texture.set_filter(FilterMode::Nearest);
        let mut camera =
            Camera2D::from_display_rect(Rect::new(0.0, 0.0, SCREEN_W as f32, SCREEN_H as f32));
        camera.render_target = Some(target.clone());
        Renderer { target, camera }
    }

    /// Draws the game into the 240x320 target. `alpha` blends moving things
    /// between the last two sim steps so motion stays smooth at any FPS.
    pub fn draw_game(&self, sim: &Sim, alpha: f32) {
        set_camera(&self.camera);
        clear_background(SKY);

        draw_tiles(&sim.map);

        // Placeholder ball; real sprite in step 7. The dark stripe turns with
        // the ball so rolling is visible; the highlight stays put (light from
        // the top-left).
        let b = &sim.ball;
        let p = b.render_pos(alpha);
        let r = b.radius;
        draw_circle(p.x, p.y, r, BALL_RED);
        let d = vec2(b.angle.cos(), b.angle.sin()) * (r - 1.5);
        draw_line(p.x - d.x, p.y - d.y, p.x + d.x, p.y + d.y, 2.0, BALL_DARK);
        draw_circle(p.x - 3.0, p.y - 3.0, 2.0, WHITE);
    }

    /// F1 overlay: frame rate, tick, ball state and held input.
    pub fn draw_debug(&self, sim: &Sim, fps: i32) {
        set_camera(&self.camera);
        let b = &sim.ball;
        let lines = [
            format!("fps {fps:3}  tick {}", sim.tick),
            format!("pos {:7.2} {:7.2}", b.pos.x, b.pos.y),
            format!("vel {:7.2} {:7.2}", b.vel.x, b.vel.y),
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

/// Placeholder tile art (real tileset in step 7): dirt shapes, a darker
/// outline on open faces, and grass on any open face that points upward
/// (tops and slopes).
fn draw_tiles(map: &TileMap) {
    for ty in 0..map.height as i32 {
        for tx in 0..map.width as i32 {
            let tile = map.get(tx, ty);
            if !tile.is_solid() {
                continue;
            }
            let origin = vec2(tx as f32, ty as f32) * TILE;
            let poly = tile.polygon();
            for i in 1..poly.len() - 1 {
                draw_triangle(
                    origin + poly[0],
                    origin + poly[i],
                    origin + poly[i + 1],
                    DIRT,
                );
            }
            for (a, b) in tile.edges() {
                if map.edge_hidden(tx, ty, a, b) {
                    continue;
                }
                let e = b - a;
                let outward = vec2(e.y, -e.x).normalize();
                let (pa, pb) = (origin + a, origin + b);
                if outward.y < -0.5 {
                    // Grass strip just inside the surface.
                    let inset = -outward * 1.5;
                    draw_line(
                        pa.x + inset.x,
                        pa.y + inset.y,
                        pb.x + inset.x,
                        pb.y + inset.y,
                        3.0,
                        GRASS,
                    );
                } else {
                    let inset = -outward * 0.5;
                    draw_line(
                        pa.x + inset.x,
                        pa.y + inset.y,
                        pb.x + inset.x,
                        pb.y + inset.y,
                        1.0,
                        DIRT_EDGE,
                    );
                }
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
    fn letterbox_never_below_one() {
        assert_eq!(letterbox(100.0, 100.0).2, 1.0);
    }
}
