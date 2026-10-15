//! Drawing. Everything is drawn into a fixed 240x320 render target (the
//! original Nokia screen), which is then scaled by a whole number onto the
//! window so pixels stay crisp.

mod atlas;
mod background;
mod particles;

use macroquad::prelude::*;

use atlas::Sprites;
use background::Background;
use particles::Particles;

use bounce_tales::level::{Decoration, TileLayer};
use bounce_tales::sim::Sim;
use bounce_tales::sim::camera::{VIEW_H, VIEW_W};
use bounce_tales::sim::map::{TILE, TileMap};
use bounce_tales::sim::{Entity, EntityKind, RESPAWN_ANIM_TICKS};

pub const SCREEN_W: u32 = VIEW_W as u32;
pub const SCREEN_H: u32 = VIEW_H as u32;

/// Sprites in the `objects` atlas that entities use (checked in tests).
#[cfg(test)]
pub const ENTITY_SPRITES: &[&str] = &["exit"];

/// Roll frames in the ball atlas (`roll_0` … `roll_15`).
pub const BALL_FRAMES: usize = 16;

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
    background: Background,
    particles: Particles,
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
            background: Background::new(),
            particles: Particles::new(),
            screen_camera,
        }
    }

    /// Draws the game into the 240x320 target. `alpha` blends moving things
    /// between the last two sim steps so motion stays smooth at any FPS.
    /// Call after every sim step: spawns and advances visual effects.
    pub fn on_step(&mut self, sim: &Sim) {
        self.particles.on_step(sim);
    }

    pub fn draw_game(&self, sim: &Sim, alpha: f32) {
        let view = sim.camera.render_pos(alpha);
        let level_h = sim.level.map.pixel_size().y;
        set_camera(&self.screen_camera);
        self.background
            .draw(view, (level_h - VIEW_H).max(0.0), sim.tick);

        set_camera(&target_camera(&self.target, view));

        let level = &sim.level;
        for layer in level.layers.iter().filter(|l| !l.front) {
            self.draw_layer(layer, &level.sheets, view);
        }
        self.draw_decorations(&level.decorations, &level.sheets, false);
        for e in &sim.entities {
            self.draw_entity(e);
        }

        self.particles.draw();

        // Squash/stretch keeps the ball's bottom on the ground. Hidden while
        // dying (the burst shows instead); grows in after a respawn.
        let b = &sim.ball;
        let p = b.render_pos(alpha);
        let grow = 1.0 - sim.respawn_anim as f32 / RESPAWN_ANIM_TICKS as f32;
        let d = b.radius * 2.0 * (0.2 + 0.8 * grow);
        let size = vec2(d * (1.0 + b.squash), d * (1.0 - b.squash)).round();
        let top_left = vec2(p.x - size.x / 2.0, p.y + b.radius - size.y);
        let frame = ball_frame(b.angle);
        if sim.dying == 0 {
            self.sprites
                .ball
                .draw_scaled(&format!("roll_{frame}"), top_left, size);
        }

        for layer in level.layers.iter().filter(|l| l.front) {
            self.draw_layer(layer, &level.sheets, view);
        }
        self.draw_decorations(&level.decorations, &level.sheets, true);
    }

    fn draw_decorations(&self, decorations: &[Decoration], sheets: &[String], front: bool) {
        for d in decorations.iter().filter(|d| d.front == front) {
            let atlas = self.sprites.by_name(&sheets[d.tile.sheet as usize]);
            atlas.draw_index(d.tile.id as usize, d.pos);
        }
    }

    fn draw_entity(&self, e: &Entity) {
        match e.kind {
            EntityKind::Exit { .. } => self.sprites.objects.draw("exit", e.pos),
        }
    }

    /// Draws the part of a tile layer inside the view.
    fn draw_layer(&self, layer: &TileLayer, sheets: &[String], view: Vec2) {
        let (x0, y0) = TileMap::cell_at(view);
        let (x1, y1) = TileMap::cell_at(view + vec2(VIEW_W, VIEW_H));
        for ty in y0.max(0)..=y1.min(layer.height as i32 - 1) {
            for tx in x0.max(0)..=x1.min(layer.width as i32 - 1) {
                if let Some(t) = layer.get(tx, ty) {
                    let atlas = self.sprites.by_name(&sheets[t.sheet as usize]);
                    atlas.draw_index(t.id as usize, vec2(tx as f32, ty as f32) * TILE);
                }
            }
        }
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
                "deaths {}{}",
                sim.deaths,
                if sim.dying > 0 { " (dying)" } else { "" }
            ),
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
    fn letterbox_never_below_one() {
        assert_eq!(letterbox(100.0, 100.0).2, 1.0);
    }
}
