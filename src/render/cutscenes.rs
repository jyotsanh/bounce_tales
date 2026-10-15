//! Cutscene panels: a framed picture drawn from shapes and game sprites,
//! the caption on paper under it, and a wipe between pages.

use macroquad::prelude::*;

use bounce_tales::cutscene::{Picture, Player, TURN};
use bounce_tales::sim::camera::{VIEW_H, VIEW_W};

use super::{Renderer, ball_frame, font};

/// Picture frame, screen px.
const BOX: Rect = Rect {
    x: 16.0,
    y: 28.0,
    w: 208.0,
    h: 156.0,
};
const PAPER: Color = Color::new(0.98, 0.95, 0.85, 1.0);
const INK: Color = Color::new(0.15, 0.1, 0.1, 1.0);
const PURPLE: Color = Color::new(0.65, 0.25, 0.85, 1.0);

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

/// Cheap deterministic hash for scattering stars and sparkles.
fn scatter(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x % 10_000) as f32 / 10_000.0
}

impl Renderer {
    pub fn draw_cutscene(&self, p: &Player, tick: u64) {
        draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, BLACK);
        self.draw_picture(p.panel().picture, p.ticks as f32, tick);
        // Crop whatever spilled out of the picture, then the comic frame:
        // a white border round it.
        let (x0, y0) = (BOX.x - 3.0, BOX.y - 3.0);
        let (x1, y1) = (BOX.x + BOX.w + 3.0, BOX.y + BOX.h + 3.0);
        draw_rectangle(0.0, 0.0, VIEW_W, y0, BLACK);
        draw_rectangle(0.0, y1, VIEW_W, VIEW_H - y1, BLACK);
        draw_rectangle(0.0, y0, x0, y1 - y0, BLACK);
        draw_rectangle(x1, y0, VIEW_W - x1, y1 - y0, BLACK);
        draw_rectangle(x0, y0, x1 - x0, 3.0, WHITE);
        draw_rectangle(x0, y1 - 3.0, x1 - x0, 3.0, WHITE);
        draw_rectangle(x0, y0, 3.0, y1 - y0, WHITE);
        draw_rectangle(x1 - 3.0, y0, 3.0, y1 - y0, WHITE);

        // Caption on paper.
        let (cx, cy, cw, ch) = (BOX.x - 3.0, 196.0, BOX.w + 6.0, 70.0);
        draw_rectangle(cx, cy, cw, ch, PAPER);
        let f = &self.sprites.font;
        for (i, line) in p.lines().iter().enumerate() {
            font::draw(
                f,
                line,
                vec2(cx + 8.0, cy + 8.0 + i as f32 * 13.0),
                1.0,
                INK,
                false,
            );
        }
        let page = format!("{}/{}", p.page + 1, p.panels.len());
        font::draw(
            f,
            &page,
            vec2(BOX.x, 280.0),
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.5),
            false,
        );
        if p.fully_typed() && (tick / 20).is_multiple_of(2) {
            let last = p.page + 1 == p.panels.len();
            let hint = if last { "END >" } else { "NEXT >" };
            let x = BOX.x + BOX.w - font::width(hint, 1.0);
            font::draw(f, hint, vec2(x, 280.0), 1.0, WHITE, false);
        }
        font::draw_centred(
            f,
            "ESC SKIP",
            VIEW_W / 2.0,
            300.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 0.35),
        );

        // Page turn: black sweeps in from the left, then off to the right.
        if p.turning > 0 {
            let k = 1.0 - p.turning as f32 / TURN as f32; // 0..1
            let (x0, x1) = if k < 0.5 {
                (0.0, k * 2.0)
            } else {
                ((k - 0.5) * 2.0, 1.0)
            };
            draw_rectangle(x0 * VIEW_W, 0.0, (x1 - x0) * VIEW_W, 270.0, BLACK);
        }
    }

    /// `t` = steps on this page (animations restart per page), `tick` =
    /// global (idle loops).
    fn draw_picture(&self, picture: Picture, t: f32, tick: u64) {
        let (x, y, w, h) = (BOX.x, BOX.y, BOX.w, BOX.h);
        let ground = y + h - 34.0;
        let s = &self.sprites;
        let spin = |speed: u64| format!("spiral_{}", (tick / speed) % 4);
        let ball = |pos: Vec2, size: f32, angle: f32| {
            s.ball.draw_scaled(
                &format!("roll_{}", ball_frame(angle)),
                pos - vec2(size / 2.0, size),
                vec2(size, size),
            );
        };
        let hop = |phase: f32, height: f32| {
            let p = phase.rem_euclid(1.0);
            4.0 * p * (1.0 - p) * height
        };
        let grass = |top: f32, c: Color| {
            draw_rectangle(x, top, w, y + h - top, c);
            draw_rectangle(x, top, w, 3.0, rgb(120, 200, 80));
        };
        match picture {
            Picture::Meadow => {
                draw_rectangle(x, y, w, h, rgb(120, 190, 240));
                draw_circle(x + 176.0, y + 30.0, 16.0, rgb(255, 230, 90));
                grass(ground, rgb(70, 160, 60));
                for (i, name) in ["flower_red", "flower_yellow", "flower_red", "tuft"]
                    .iter()
                    .enumerate()
                {
                    s.deco
                        .draw(name, vec2(x + 14.0 + i as f32 * 52.0, ground - 14.0));
                }
                let b = hop(t / 40.0, 34.0);
                ball(vec2(x + w / 2.0, ground - b), 22.0, t * 0.05);
                for (i, dx) in [40.0, 160.0].iter().enumerate() {
                    let b = hop(t / 30.0 + i as f32 * 0.5, 14.0);
                    s.items
                        .draw("walker_freed", vec2(x + dx - 8.0, ground - 16.0 - b));
                }
            }
            Picture::CubeArrives => {
                draw_rectangle(x, y, w, h, rgb(20, 24, 60));
                for i in 0..30 {
                    let (sx, sy) = (scatter(i, 1) * w, scatter(i, 2) * (h - 50.0));
                    let twinkle = if (tick / 10 + i as u64).is_multiple_of(5) {
                        0.4
                    } else {
                        1.0
                    };
                    draw_rectangle(x + sx, y + sy, 1.0, 1.0, Color::new(1.0, 1.0, 1.0, twinkle));
                }
                draw_circle(x + 30.0, y + 26.0, 10.0, rgb(240, 240, 210));
                draw_circle(x + 35.0, y + 23.0, 9.0, rgb(20, 24, 60));
                grass(ground, rgb(25, 60, 35));
                // Sinks from above the frame, then hovers.
                let cy = (y - 40.0 + t * 0.8).min(y + 34.0) + (tick as f32 * 0.08).sin() * 2.0;
                let cx = x + w / 2.0 - 24.0;
                for r in 0..3 {
                    let k = ((tick as f32 * 0.03) + r as f32 / 3.0).fract();
                    let c = Color::new(0.75, 0.35, 1.0, 0.5 * (1.0 - k));
                    draw_circle_lines(cx + 24.0, cy + 24.0, 26.0 + k * 50.0, 1.0, c);
                }
                if cy + 48.0 > y {
                    s.boss.draw_scaled(
                        &spin(6),
                        vec2(cx, cy.max(y)),
                        vec2(48.0, 48.0 - (y - cy).max(0.0)),
                    );
                }
                s.items.draw("walker_0", vec2(x + 40.0, ground - 16.0));
                s.items.draw("walker_0", vec2(x + 150.0, ground - 16.0));
            }
            Picture::Hypnotised => {
                // Pulsing rings of the spell.
                draw_rectangle(x, y, w, h, rgb(60, 20, 80));
                let (cx, cy) = (x + w / 2.0, y + 60.0);
                for r in (0..9).rev() {
                    let k = r as f32 + (tick as f32 * 0.05).fract();
                    let c = if r % 2 == 0 {
                        rgb(110, 40, 150)
                    } else {
                        rgb(70, 25, 95)
                    };
                    draw_circle(cx, cy, k * 16.0, c);
                }
                grass(ground, rgb(40, 70, 40));
                let critters = ["walker_0", "flyer_0", "jumper_0", "shooter_0"];
                for (i, name) in critters.iter().enumerate() {
                    let px = x + 20.0 + i as f32 * 48.0;
                    let sway = (tick as f32 * 0.06 + i as f32).sin() * 2.0;
                    s.items.draw(name, vec2(px + sway, ground - 16.0));
                    // Dizzy spiral eyes above each one.
                    s.boss.draw_scaled(
                        &spin(5),
                        vec2(px + 2.0 + sway, ground - 30.0),
                        vec2(12.0, 12.0),
                    );
                }
            }
            Picture::SetsOff => {
                draw_rectangle(x, y, w, h, rgb(130, 200, 245));
                draw_circle(x + 50.0, y + h + 30.0, 110.0, rgb(90, 170, 80));
                draw_circle(x + 170.0, y + h + 40.0, 100.0, rgb(80, 155, 70));
                grass(ground, rgb(70, 160, 60));
                draw_rectangle(x, ground + 8.0, w, 8.0, rgb(180, 140, 90));
                s.deco.draw("sign_right", vec2(x + 150.0, ground - 16.0));
                let bx = x + 20.0 + (t * 0.9) % (w - 20.0);
                ball(vec2(bx, ground + 2.0 - hop(t / 25.0, 18.0)), 20.0, t * 0.12);
            }
            Picture::Cave => {
                draw_rectangle(x, y, w, h, rgb(40, 34, 46));
                for i in 0..9 {
                    let sx = x + 6.0 + i as f32 * 24.0;
                    let len = 14.0 + scatter(i, 3) * 26.0;
                    draw_triangle(
                        vec2(sx, y),
                        vec2(sx + 16.0, y),
                        vec2(sx + 8.0, y + len),
                        rgb(70, 60, 75),
                    );
                    // A drip falling from every other tip.
                    if i % 2 == 0 {
                        let k = ((tick as f32 * 0.01) + scatter(i, 4)).fract();
                        draw_rectangle(
                            sx + 7.0,
                            y + len + k * (ground - y - len),
                            2.0,
                            3.0,
                            rgb(120, 170, 255),
                        );
                    }
                }
                draw_rectangle(x, ground, w, y + h - ground, rgb(55, 48, 60));
                draw_rectangle(x + 60.0, ground, 90.0, 10.0, rgb(50, 100, 200));
                for i in 0..4 {
                    let gx = x + 30.0 + i as f32 * 48.0;
                    let glow = 0.25 + 0.15 * (tick as f32 * 0.1 + i as f32).sin();
                    draw_circle(
                        gx + 8.0,
                        ground - 30.0,
                        12.0,
                        Color::new(0.5, 0.8, 1.0, glow),
                    );
                    s.items.draw(
                        &format!("gem_{}", (tick / 8 + i) % 4),
                        vec2(gx, ground - 38.0),
                    );
                }
                ball(vec2(x + 24.0, ground), 18.0, 0.0);
            }
            Picture::Castle => {
                draw_rectangle(x, y, w, h, rgb(70, 40, 90));
                draw_rectangle(x, y + h * 0.5, w, h * 0.5, rgb(90, 50, 100));
                draw_circle(x + 170.0, y + 26.0, 12.0, rgb(240, 230, 200));
                let stone = rgb(45, 40, 55);
                let (kx, top) = (x + 40.0, y + 60.0);
                draw_rectangle(kx, top, 128.0, ground - top, stone);
                for i in 0..8 {
                    draw_rectangle(kx + i as f32 * 16.0, top - 8.0, 10.0, 8.0, stone);
                }
                // Central tower with the glowing spiral.
                draw_rectangle(kx + 44.0, y + 14.0, 40.0, top - y - 14.0, stone);
                draw_triangle(
                    vec2(kx + 40.0, y + 16.0),
                    vec2(kx + 88.0, y + 16.0),
                    vec2(kx + 64.0, y - 2.0),
                    rgb(90, 30, 60),
                );
                let glow = 0.35 + 0.25 * (tick as f32 * 0.08).sin();
                draw_circle(kx + 64.0, y + 34.0, 16.0, Color::new(0.8, 0.4, 1.0, glow));
                s.boss
                    .draw_scaled(&spin(6), vec2(kx + 56.0, y + 26.0), vec2(16.0, 16.0));
                draw_rectangle(kx + 52.0, ground - 30.0, 24.0, 30.0, rgb(25, 20, 30));
                grass(ground, rgb(40, 60, 40));
                ball(vec2(x + 18.0, ground), 16.0, 0.0);
            }
            Picture::Showdown => {
                draw_rectangle(x, y, w, h, rgb(90, 20, 50));
                let centre = vec2(x + 140.0, y + 60.0);
                for i in 0..16 {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU + tick as f32 * 0.01;
                    let end = centre + vec2(a.cos(), a.sin()) * 200.0;
                    let c = if i % 2 == 0 {
                        rgb(130, 30, 80)
                    } else {
                        rgb(100, 25, 60)
                    };
                    draw_triangle(
                        centre,
                        end,
                        centre + vec2((a + 0.2).cos(), (a + 0.2).sin()) * 200.0,
                        c,
                    );
                }
                grass(ground, rgb(50, 30, 50));
                let shake = if (tick / 4).is_multiple_of(2) {
                    1.0
                } else {
                    -1.0
                };
                s.boss.draw_scaled(
                    &spin(4),
                    vec2(x + 110.0 + shake, y + 30.0),
                    vec2(64.0, 64.0),
                );
                ball(vec2(x + 50.0, ground - hop(t / 30.0, 10.0)), 28.0, 0.0);
            }
            Picture::Shatter => {
                let flash = (1.0 - t / 30.0).max(0.0);
                draw_rectangle(x, y, w, h, rgb(250, 230, 160));
                let centre = vec2(x + w / 2.0, y + 60.0);
                for i in 0..40 {
                    let a = scatter(i, 5) * std::f32::consts::TAU;
                    let speed = 0.6 + scatter(i, 6) * 1.4;
                    let d = (t * speed).min(140.0);
                    let p = centre + vec2(a.cos(), a.sin()) * d;
                    let c = if i % 3 == 0 { PURPLE } else { WHITE };
                    if p.x > x && p.x < x + w - 3.0 && p.y > y && p.y < y + h - 3.0 {
                        draw_rectangle(p.x, p.y, 3.0, 3.0, c);
                    }
                }
                grass(ground, rgb(70, 160, 60));
                ball(
                    vec2(x + w / 2.0, ground - hop(t / 30.0, 24.0)),
                    22.0,
                    t * 0.1,
                );
                draw_rectangle(x, y, w, h, Color::new(1.0, 1.0, 1.0, flash));
            }
            Picture::Celebration => {
                draw_rectangle(x, y, w, h, rgb(120, 190, 240));
                draw_circle(x + 176.0, y + 30.0, 16.0, rgb(255, 230, 90));
                grass(ground, rgb(70, 160, 60));
                for i in 0..24 {
                    let cx = x + scatter(i, 7) * (w - 3.0);
                    let cy = y + ((scatter(i, 8) * h + tick as f32 * 0.6) % (ground - y));
                    let c = [RED, GOLD_C, PURPLE, WHITE][(i % 4) as usize];
                    draw_rectangle(cx, cy, 2.0, 3.0, c);
                }
                for i in 0..5 {
                    let px = x + 12.0 + i as f32 * 40.0;
                    if i == 2 {
                        ball(vec2(px + 8.0, ground - hop(t / 28.0, 30.0)), 22.0, 0.0);
                    } else {
                        let b = hop(t / 24.0 + i as f32 * 0.3, 16.0);
                        s.items.draw("walker_freed", vec2(px, ground - 16.0 - b));
                    }
                }
            }
            Picture::Credits => {
                draw_rectangle(x, y, w, h, rgb(20, 20, 30));
                let f = &s.font;
                font::draw_centred(f, "BOUNCE", x + w / 2.0, y + 28.0, 3.0, RED);
                font::draw_centred(f, "TALES", x + w / 2.0, y + 58.0, 3.0, GOLD_C);
                ball(
                    vec2(x + w / 2.0, y + h - 20.0 - hop(t / 36.0, 30.0)),
                    20.0,
                    0.0,
                );
            }
        }
    }
}

const GOLD_C: Color = Color::new(1.0, 0.85, 0.25, 1.0);
