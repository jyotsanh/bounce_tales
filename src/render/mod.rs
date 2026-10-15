//! Drawing. Everything is drawn into a fixed 240x320 render target (the
//! original Nokia screen), which is then scaled by a whole number onto the
//! window so pixels stay crisp.

mod atlas;
mod background;
mod font;
mod particles;

use macroquad::prelude::*;

use atlas::Sprites;
use background::Background;
use particles::Particles;

use bounce_tales::game::{self, Game, Mode, REVEAL_STEP};
use bounce_tales::level::{Decoration, TileLayer};
use bounce_tales::sim::Sim;
use bounce_tales::sim::camera::{VIEW_H, VIEW_W};
use bounce_tales::sim::entities::GATE_ANIM_TICKS;
use bounce_tales::sim::map::{TILE, TileMap, flags};
use bounce_tales::sim::seesaw;
use bounce_tales::sim::{Entity, EntityKind, Item, RESPAWN_ANIM_TICKS, cells_of};

pub const SCREEN_W: u32 = VIEW_W as u32;
pub const SCREEN_H: u32 = VIEW_H as u32;

/// Sprites in the `objects` atlas that entities use (checked in tests).
#[cfg(test)]
pub const ENTITY_SPRITES: &[&str] = &[
    "exit",
    "checkpoint_off",
    "checkpoint_on",
    "gem_0",
    "gem_1",
    "gem_2",
    "gem_3",
    "egg",
    "life",
    "icon_ball",
    "icon_gem",
    "icon_egg",
    "plat_l",
    "plat_m",
    "plat_r",
    "switch_off",
    "switch_on",
    "door",
    "bridge",
    "spring_0",
    "spring_1",
    "fan_0",
    "fan_1",
    "catapult_r",
    "catapult_l",
    "catapult_r_fired",
    "catapult_l_fired",
    "crate",
    "crate_big",
    "seesaw_post",
    "hub",
    "cannon_base",
];

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
        let map = &level.map;
        let shaking = |x: i32, y: i32| sim.crumbles.iter().any(|c| !c.gone && (c.x, c.y) == (x, y));
        for layer in level.layers.iter().filter(|l| !l.front) {
            self.draw_layer(layer, &level.sheets, view, |x, y| {
                !is_water(map, x, y) && !shaking(x, y)
            });
        }
        // Crumbling blocks about to fall shake in place.
        for c in sim.crumbles.iter().filter(|c| !c.gone) {
            let jitter = if (sim.tick / 2).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            let pos = vec2(c.x as f32, c.y as f32) * TILE + vec2(jitter, 0.0);
            self.sprites.forest.draw("crumble", pos);
        }
        self.draw_decorations(&level.decorations, &level.sheets, false);
        for e in &sim.entities {
            self.draw_entity(e, sim.tick, alpha);
        }

        self.particles.draw();

        // Squash/stretch keeps the ball's bottom on the ground. Hidden while
        // dying or game over (the burst shows instead); grows in after a
        // respawn.
        let b = &sim.ball;
        let p = b.render_pos(alpha);
        let grow = 1.0 - sim.respawn_anim as f32 / RESPAWN_ANIM_TICKS as f32;
        let d = b.radius * 2.0 * (0.2 + 0.8 * grow);
        let size = vec2(d * (1.0 + b.squash), d * (1.0 - b.squash)).round();
        let top_left = vec2(p.x - size.x / 2.0, p.y + b.radius - size.y);
        let frame = ball_frame(b.angle);
        if sim.ball_visible() {
            self.sprites
                .ball
                .draw_scaled(&format!("roll_{frame}"), top_left, size);
        }

        // Water goes over the ball so it looks submerged (the tiles are
        // see-through), then front layers.
        for layer in level.layers.iter().filter(|l| !l.front) {
            self.draw_layer(layer, &level.sheets, view, |x, y| is_water(map, x, y));
        }
        for layer in level.layers.iter().filter(|l| l.front) {
            self.draw_layer(layer, &level.sheets, view, |_, _| true);
        }
        self.draw_decorations(&level.decorations, &level.sheets, true);
        self.particles.draw_popups(&self.sprites.font);
    }

    fn draw_decorations(&self, decorations: &[Decoration], sheets: &[String], front: bool) {
        for d in decorations.iter().filter(|d| d.front == front) {
            let atlas = self.sprites.by_name(&sheets[d.tile.sheet as usize]);
            atlas.draw_index(d.tile.id as usize, d.pos);
        }
    }

    fn draw_entity(&self, e: &Entity, tick: u64, alpha: f32) {
        match &e.kind {
            EntityKind::Platform(p) => {
                let top_left =
                    p.prev_top_centre.lerp(p.top_centre, alpha) - vec2(p.width / 2.0, 0.0);
                let pieces = (p.width / TILE).round() as usize;
                for i in 0..pieces {
                    let name = match i {
                        0 => "plat_l",
                        _ if i == pieces - 1 => "plat_r",
                        _ => "plat_m",
                    };
                    self.sprites
                        .items
                        .draw(name, top_left + vec2(i as f32 * TILE, 0.0));
                }
            }
            EntityKind::Pickup { taken: true, .. } => {}
            EntityKind::Crate(c) if !c.gone => {
                let pos = c.prev_pos.lerp(e.pos, alpha);
                if e.size == vec2(32.0, 32.0) {
                    self.sprites.objects.draw("crate_big", pos);
                } else {
                    // Other sizes: a grid of small crates.
                    let (w, h) = ((e.size.x / TILE) as i32, (e.size.y / TILE) as i32);
                    for y in 0..h.max(1) {
                        for x in 0..w.max(1) {
                            let p = pos + vec2(x as f32, y as f32) * TILE;
                            self.sprites.items.draw("crate", p);
                        }
                    }
                }
            }
            EntityKind::Crate(_) => {}
            EntityKind::Seesaw(w) => {
                if w.motor.is_none() {
                    let post_top = vec2(w.pivot.x - 8.0, w.pivot.y);
                    let post_len = (w.base_y - w.pivot.y).max(16.0);
                    self.sprites
                        .items
                        .draw_scaled("seesaw_post", post_top, vec2(16.0, post_len));
                }
                let angle = w.prev_angle + (w.angle - w.prev_angle) * alpha;
                let size = vec2(w.half * 2.0, seesaw::THICKNESS);
                let top_left = w.pivot - size / 2.0;
                self.sprites
                    .seesaw
                    .draw_rotated("plank", top_left, size, angle);
                if w.motor.is_some() {
                    self.sprites.items.draw("hub", w.pivot - vec2(8.0, 8.0));
                }
            }
            EntityKind::Wheel {
                centre,
                radius,
                count,
                angle,
                ..
            } => {
                let spoke = Color::new(0.35, 0.26, 0.15, 1.0);
                for k in 0..*count {
                    let a = angle + k as f32 / *count as f32 * std::f32::consts::TAU;
                    let end = *centre + vec2(a.cos(), a.sin()) * *radius + vec2(0.0, 8.0);
                    draw_line(centre.x, centre.y, end.x, end.y, 2.0, spoke);
                }
                self.sprites.items.draw("hub", *centre - vec2(8.0, 8.0));
            }
            EntityKind::Cannon { aim, fired, .. } => {
                let pivot = e.pos + e.size / 2.0;
                // Recoil: the barrel kicks back a little when it fires.
                let back = if *fired > 0 { 3.0 } else { 0.0 };
                let dir = vec2(aim.cos(), aim.sin());
                let start = pivot - dir * back - vec2(0.0, 6.0);
                self.sprites.cannon.draw_rotated_about(
                    "barrel",
                    start,
                    vec2(24.0, 12.0),
                    *aim,
                    vec2(0.0, 6.0),
                );
                self.sprites.objects.draw("cannon_base", e.pos);
            }
            EntityKind::Spring { squish, .. } => {
                let name = if *squish > 0 { "spring_1" } else { "spring_0" };
                self.sprites.items.draw(name, e.pos);
            }
            EntityKind::Catapult { launch, fired, .. } => {
                let side = if launch.x < 0.0 { "l" } else { "r" };
                let pose = if *fired > 0 { "_fired" } else { "" };
                // 32x32 sprite whose base fills the bottom 16 px.
                let pos = vec2(e.pos.x + e.size.x / 2.0 - 16.0, e.pos.y + e.size.y - 32.0);
                self.sprites
                    .objects
                    .draw(&format!("catapult_{side}{pose}"), pos);
            }
            EntityKind::Fan { force } => {
                // The fan sits on the upwind edge of its air stream.
                let dir = force.normalize_or_zero();
                let centre = e.pos + e.size / 2.0;
                let base = centre - dir * (e.size * dir.abs()) / 2.0;
                let frame = (tick / 3) % 2;
                let pos = base - vec2(8.0, 8.0) + dir * -8.0;
                self.sprites.items.draw(&format!("fan_{frame}"), pos);
            }
            EntityKind::Switch { on, .. } => {
                let name = if *on { "switch_on" } else { "switch_off" };
                self.sprites.items.draw(name, e.pos);
            }
            EntityKind::Door {
                bridge,
                switched,
                anim,
                ..
            } => {
                // 1 at rest in the new state, counting up from 0 while
                // animating.
                let t = 1.0 - *anim as f32 / GATE_ANIM_TICKS as f32;
                let cells = cells_of(e);
                let (x0, y0) = cells[0];
                for (x, y) in cells {
                    let pos = vec2(x as f32, y as f32) * TILE;
                    if *bridge {
                        // Planks appear from left to right.
                        let col = (x - x0) as f32;
                        let cols = (e.size.x / TILE).max(1.0);
                        if *switched && col < (t * cols).ceil() {
                            self.sprites.items.draw("bridge", pos);
                        }
                    } else if !*switched || *anim > 0 {
                        // The door slides up into its frame.
                        let rows = (e.size.y / TILE).max(1.0);
                        let lift = if *switched { t * rows * TILE } else { 0.0 };
                        let p = pos - vec2(0.0, lift);
                        if p.y + TILE > y0 as f32 * TILE {
                            self.sprites.items.draw("door", p);
                        }
                    }
                }
            }
            &EntityKind::Pickup { item, .. } => {
                // Gentle bob, out of step between items so rows ripple.
                let phase = e.pos.x * 0.15;
                let bob = ((tick as f32 * 0.08 + phase).sin() * 1.5).round();
                let pos = e.pos + vec2(0.0, bob);
                match item {
                    Item::Gem => {
                        let frame = (tick / 8 + (e.pos.x / 16.0) as u64) % 4;
                        self.sprites.items.draw(&format!("gem_{frame}"), pos);
                    }
                    Item::Egg => self.sprites.items.draw("egg", pos),
                    Item::Life => self.sprites.items.draw("life", pos),
                }
            }
            EntityKind::Exit { .. } => self.sprites.objects.draw("exit", e.pos),
            &EntityKind::Checkpoint { active } => {
                let name = if active {
                    "checkpoint_on"
                } else {
                    "checkpoint_off"
                };
                self.sprites.objects.draw(name, e.pos);
            }
        }
    }

    /// Draws the part of a tile layer inside the view, only the cells
    /// `keep(x, y)` accepts.
    fn draw_layer(
        &self,
        layer: &TileLayer,
        sheets: &[String],
        view: Vec2,
        keep: impl Fn(i32, i32) -> bool,
    ) {
        let (x0, y0) = TileMap::cell_at(view);
        let (x1, y1) = TileMap::cell_at(view + vec2(VIEW_W, VIEW_H));
        for ty in y0.max(0)..=y1.min(layer.height as i32 - 1) {
            for tx in x0.max(0)..=x1.min(layer.width as i32 - 1) {
                if !keep(tx, ty) {
                    continue;
                }
                if let Some(t) = layer.get(tx, ty) {
                    let atlas = self.sprites.by_name(&sheets[t.sheet as usize]);
                    atlas.draw_index(t.id as usize, vec2(tx as f32, ty as f32) * TILE);
                }
            }
        }
    }

    /// Heads-up display in screen space: lives, gems, eggs (if the level
    /// has any) along the top left; score at the top right; the game-over
    /// banner.
    pub fn draw_hud(&self, sim: &Sim) {
        set_camera(&self.screen_camera);
        let font_atlas = &self.sprites.font;
        let icons = &self.sprites.icons;
        let text = |t: &str, x: f32| font::draw(font_atlas, t, vec2(x, 5.0), 1.0, WHITE, true);

        let mut x = 4.0;
        let mut item = |icon: &str, label: String| {
            icons.draw(icon, vec2(x, 4.0));
            text(&label, x + 10.0);
            x += 10.0 + font::width(&label, 1.0) + 8.0;
        };
        item("icon_ball", format!("*{}", sim.lives));
        item("icon_gem", sim.gems.to_string());
        if sim.eggs_total > 0 {
            item("icon_egg", format!("{}/{}", sim.eggs, sim.eggs_total));
        }

        let score = format!("{:06}", sim.score);
        text(&score, VIEW_W - 4.0 - font::width(&score, 1.0));

        if sim.game_over > 0 {
            let (w, h) = (170.0, 40.0);
            let (bx, by) = ((VIEW_W - w) / 2.0, (VIEW_H - h) / 2.0);
            draw_rectangle(bx, by, w, h, Color::new(0.0, 0.0, 0.0, 0.7));
            font::draw_centred(
                font_atlas,
                "GAME OVER",
                VIEW_W / 2.0,
                by + (h - font::HEIGHT * 2.0) / 2.0,
                2.0,
                Color::new(1.0, 0.85, 0.25, 1.0),
            );
        }
    }

    /// Level-complete results and the finished screen, over the game.
    pub fn draw_flow(&self, g: &Game) {
        set_camera(&self.screen_camera);
        let f = &self.sprites.font;
        let gold = Color::new(1.0, 0.85, 0.25, 1.0);
        let dim = Color::new(0.0, 0.0, 0.0, 0.72);
        match &g.mode {
            Mode::Playing => {}
            Mode::Results { result: r, ticks } => {
                let (x, y, w, h) = (20.0, 60.0, VIEW_W - 40.0, 190.0);
                draw_rectangle(x, y, w, h, dim);
                font::draw_centred(f, "LEVEL", VIEW_W / 2.0, y + 12.0, 2.0, gold);
                font::draw_centred(f, "COMPLETE!", VIEW_W / 2.0, y + 32.0, 2.0, gold);
                let lines = [
                    (
                        "TIME",
                        format!(
                            "{} / {}",
                            game::format_time(r.seconds),
                            game::format_time(r.par_seconds)
                        ),
                    ),
                    ("GEMS", format!("{}/{}", r.gems, r.gems_total)),
                    ("EGGS", format!("{}/{}", r.eggs, r.eggs_total)),
                    ("TIME BONUS", r.time_bonus.to_string()),
                    ("EGG BONUS", r.egg_bonus.to_string()),
                    ("SCORE", format!("{:06}", r.score)),
                ];
                // Lines appear one after another.
                let shown = (*ticks / REVEAL_STEP) as usize;
                for (i, (label, value)) in lines.iter().enumerate().take(shown) {
                    let ly = y + 62.0 + i as f32 * 16.0;
                    let color = if i == lines.len() - 1 { gold } else { WHITE };
                    font::draw(f, label, vec2(x + 14.0, ly), 1.0, color, true);
                    let vx = x + w - 14.0 - font::width(value, 1.0);
                    font::draw(f, value, vec2(vx, ly), 1.0, color, true);
                }
                if *ticks >= game::CONTINUE_AFTER && (*ticks / 20) % 2 == 0 {
                    font::draw_centred(f, "PRESS JUMP", VIEW_W / 2.0, y + h - 16.0, 1.0, WHITE);
                }
            }
            Mode::Finished { ticks } => {
                draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, dim);
                font::draw_centred(f, "WELL DONE!", VIEW_W / 2.0, 90.0, 2.0, gold);
                let (eggs, total) = g.eggs_found();
                font::draw_centred(
                    f,
                    &format!("SCORE {:06}", g.score()),
                    VIEW_W / 2.0,
                    140.0,
                    1.0,
                    WHITE,
                );
                font::draw_centred(
                    f,
                    &format!("EGGS {eggs}/{total}"),
                    VIEW_W / 2.0,
                    156.0,
                    1.0,
                    WHITE,
                );
                if *ticks >= game::CONTINUE_AFTER && (*ticks / 20) % 2 == 0 {
                    font::draw_centred(f, "PRESS JUMP", VIEW_W / 2.0, 220.0, 1.0, WHITE);
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

/// Water cells are drawn in front of the ball.
fn is_water(map: &TileMap, x: i32, y: i32) -> bool {
    map.flags(x, y) & flags::WATER != 0
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
