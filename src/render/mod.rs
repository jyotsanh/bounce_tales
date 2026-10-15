//! Drawing. Everything is drawn into a fixed 240x320 render target (the
//! original Nokia screen), which is then scaled by a whole number onto the
//! window so pixels stay crisp.

use macroquad::prelude::*;

use crate::sim::Sim;

pub const SCREEN_W: u32 = 240;
pub const SCREEN_H: u32 = 320;

const SKY: Color = Color::new(0.55, 0.80, 0.95, 1.0);
const GRASS: Color = Color::new(0.30, 0.65, 0.20, 1.0);
const DIRT: Color = Color::new(0.50, 0.33, 0.18, 1.0);
const BALL_RED: Color = Color::new(0.85, 0.10, 0.10, 1.0);
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

    /// Draws the game into the 240x320 target. `alpha` is the fixed-step blend
    /// factor (unused until things move smoothly in step 3).
    pub fn draw_game(&self, sim: &Sim, _alpha: f32) {
        set_camera(&self.camera);
        clear_background(SKY);

        let ground_y = 260.0;
        draw_rectangle(0.0, ground_y, SCREEN_W as f32, 6.0, GRASS);
        draw_rectangle(0.0, ground_y + 6.0, SCREEN_W as f32, SCREEN_H as f32, DIRT);

        // Placeholder ball sliding back and forth so a running sim is visible.
        let t = sim.tick as f32 / 60.0;
        let r = 8.0;
        let x = SCREEN_W as f32 / 2.0 + (t * 1.5).sin() * 80.0;
        draw_circle(x, ground_y - r, r, BALL_RED);
        draw_circle(x - 3.0, ground_y - r - 3.0, 2.0, WHITE);

        draw_text(format!("tick {}", sim.tick), 4.0, 14.0, 16.0, BLACK);
        draw_text("BOUNCE TALES", 60.0, 120.0, 24.0, BALL_RED);
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
