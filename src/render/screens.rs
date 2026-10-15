//! Title, menus, help, pause and game-over overlays, and the fade.

use macroquad::prelude::*;

use bounce_tales::app::{self, App, Screen};
use bounce_tales::sim::camera::{VIEW_H, VIEW_W};
use bounce_tales::sim::map::TILE;

use super::{Renderer, ball_frame, font};

const GOLD: Color = Color::new(1.0, 0.85, 0.25, 1.0);
const RED: Color = Color::new(0.95, 0.25, 0.2, 1.0);
const DIM: Color = Color::new(0.0, 0.0, 0.0, 0.6);
/// Top of the grass on the title backdrop.
const GROUND_Y: f32 = 288.0;

impl Renderer {
    /// Everything that isn't the game itself, drawn last. The game (if
    /// shown) is already in the target.
    pub fn draw_screens(&self, app: &App) {
        set_camera(&self.screen_camera);
        let t = app.ticks;
        match app.screen {
            Screen::Title => {
                self.draw_backdrop(t);
                self.draw_logo(48.0, 4.0, t);
                if (t / 30).is_multiple_of(2) {
                    font::draw_centred(self.font(), "PRESS SPACE", VIEW_W / 2.0, 214.0, 1.0, WHITE);
                }
                font::draw_centred(
                    self.font(),
                    "A FAN REMAKE",
                    VIEW_W / 2.0,
                    GROUND_Y + 14.0,
                    1.0,
                    Color::new(1.0, 1.0, 1.0, 0.8),
                );
            }
            Screen::Menu { sel } => {
                self.draw_backdrop(t);
                self.draw_logo(28.0, 3.0, t);
                self.draw_menu(app::MAIN_MENU, sel, 150.0, t);
            }
            Screen::Help => {
                self.draw_backdrop(t);
                self.draw_help();
            }
            Screen::Playing => {}
            Screen::Paused { sel } => {
                draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, DIM);
                font::draw_centred(self.font(), "PAUSED", VIEW_W / 2.0, 100.0, 3.0, GOLD);
                self.draw_menu(app::PAUSE_MENU, sel, 160.0, t);
            }
            Screen::GameOver { sel } => {
                // "GAME OVER" itself is the HUD's; the choices go under it.
                self.draw_menu(app::GAME_OVER_MENU, sel, 196.0, t);
            }
        }
        let fade = app.fade_alpha();
        if fade > 0.0 {
            draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.0, 0.0, 0.0, fade));
        }
    }

    fn font(&self) -> &super::atlas::Atlas {
        &self.sprites.font
    }

    /// Sky and hills scrolling by, a strip of grass, and the ball bouncing
    /// along it.
    fn draw_backdrop(&self, tick: u64) {
        let scroll = tick as f32 * 0.6;
        self.background.draw(vec2(scroll, 0.0), 0.0, tick);
        let tiles = (VIEW_W / TILE) as i32 + 2;
        let offset = scroll % TILE;
        for i in 0..tiles {
            let x = i as f32 * TILE - offset;
            self.sprites.forest.draw("grass_top", vec2(x, GROUND_Y));
            self.sprites.forest.draw("dirt", vec2(x, GROUND_Y + TILE));
        }
        // A ball bouncing in place while the world scrolls under it.
        let phase = (tick % 50) as f32 / 50.0;
        let height = 4.0 * phase * (1.0 - phase) * 60.0;
        let r = 8.0;
        let x = 60.0;
        let y = GROUND_Y - r - height;
        let angle = scroll / r;
        // Squash on the ground.
        let squash = if phase < 0.06 || phase > 0.94 {
            0.2
        } else {
            0.0
        };
        let size = vec2(2.0 * r * (1.0 + squash), 2.0 * r * (1.0 - squash));
        let top_left = vec2(x - size.x / 2.0, y + r - size.y);
        self.sprites
            .ball
            .draw_scaled(&format!("roll_{}", ball_frame(angle)), top_left, size);
    }

    /// "BOUNCE / TALES" with a little wobble.
    fn draw_logo(&self, y: f32, scale: f32, tick: u64) {
        let f = self.font();
        let bob = ((tick as f32) * 0.08).sin() * 2.0;
        font::draw_centred(f, "BOUNCE", VIEW_W / 2.0, (y + bob).round(), scale, RED);
        font::draw_centred(
            f,
            "TALES",
            VIEW_W / 2.0,
            (y + 10.0 * scale - bob).round(),
            scale,
            GOLD,
        );
    }

    /// A vertical menu centred on the screen; the chosen item is gold with a
    /// pointer bobbing beside it.
    fn draw_menu(&self, items: &[&str], sel: usize, y: f32, tick: u64) {
        let f = self.font();
        let line = 22.0;
        let widest = items
            .iter()
            .map(|s| font::width(s, 2.0))
            .fold(0.0, f32::max);
        let (bw, bh) = (widest + 44.0, items.len() as f32 * line + 10.0);
        draw_rectangle((VIEW_W - bw) / 2.0, y - 8.0, bw, bh, DIM);
        for (i, item) in items.iter().enumerate() {
            let iy = y + i as f32 * line;
            let chosen = i == sel;
            let color = if chosen { GOLD } else { WHITE };
            font::draw_centred(f, item, VIEW_W / 2.0, iy, 2.0, color);
            if chosen {
                let nudge = if (tick / 15).is_multiple_of(2) {
                    0.0
                } else {
                    2.0
                };
                let x = (VIEW_W - font::width(item, 2.0)) / 2.0 - 16.0 + nudge;
                font::draw(f, ">", vec2(x, iy), 2.0, GOLD, true);
            }
        }
    }

    fn draw_help(&self) {
        let f = self.font();
        let (x, y, w, h) = (14.0, 40.0, VIEW_W - 28.0, 236.0);
        draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.75));
        font::draw_centred(f, "HOW TO PLAY", VIEW_W / 2.0, y + 10.0, 2.0, GOLD);
        let rows = [
            ("LEFT RIGHT", "ROLL"),
            ("A D", "ROLL"),
            ("UP W SPACE", "JUMP"),
            ("ESC P", "PAUSE"),
            ("M", "SOUND ON/OFF"),
            ("- =", "MUSIC"),
            ("[ ]", "EFFECTS"),
        ];
        for (i, (keys, what)) in rows.iter().enumerate() {
            let ry = y + 40.0 + i as f32 * 14.0;
            font::draw(f, keys, vec2(x + 10.0, ry), 1.0, GOLD, true);
            font::draw(f, what, vec2(x + 110.0, ry), 1.0, WHITE, true);
        }
        let tips = [
            "COLLECT GEMS, FIND EGGS,",
            "REACH THE EXIT.",
            "STOMP CREATURES TO",
            "FREE THEM. MIND THE SPIKES!",
        ];
        for (i, tip) in tips.iter().enumerate() {
            font::draw_centred(
                f,
                tip,
                VIEW_W / 2.0,
                y + 150.0 + i as f32 * 12.0,
                1.0,
                WHITE,
            );
        }
        font::draw_centred(f, "PRESS SPACE", VIEW_W / 2.0, y + h - 16.0, 1.0, GOLD);
    }
}
