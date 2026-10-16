//! Parallax backdrop: sky bands, drifting clouds, far hills and near hills
//! with trees. The hill pictures are generated once at start-up from sine
//! waves whose periods divide the texture width, so they tile seamlessly.
//!
//! Layers move slower than the level (`factor` < 1) horizontally and sink a
//! little as the camera climbs, which reads as depth.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use bounce_tales::sim::camera::{VIEW_H, VIEW_W};

/// Width of every tiling layer texture, px.
const LAYER_W: u16 = 256;
const LAYER_H: u16 = 112;

fn rgb(hex: u32) -> Color {
    Color::from_rgba((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255)
}

/// One sine term of a hill profile: (waves per texture width, amplitude px,
/// phase).
type Wave = (f32, f32, f32);

/// Hill surface height (px from the texture top) at column `x`.
pub fn profile(x: f32, base: f32, waves: &[Wave]) -> f32 {
    base + waves
        .iter()
        .map(|&(n, amp, phase)| amp * (TAU * n * x / LAYER_W as f32 + phase).sin())
        .sum::<f32>()
}

struct Hills {
    base: f32,
    waves: &'static [Wave],
    body: u32,
    rim: u32,
    /// (x, canopy radius) of round trees standing on the hill line.
    trees: &'static [(f32, f32)],
    tree: u32,
}

const FAR: Hills = Hills {
    base: 46.0,
    waves: &[(1.0, 14.0, 0.0), (3.0, 7.0, 1.0)],
    body: 0xa7d7b0,
    rim: 0xc4e8c8,
    trees: &[],
    tree: 0,
};

const NEAR: Hills = Hills {
    base: 60.0,
    waves: &[(2.0, 10.0, 0.5), (5.0, 4.0, 2.0)],
    body: 0x6cb865,
    rim: 0x8fd07f,
    trees: &[(22.0, 9.0), (74.0, 12.0), (150.0, 10.0), (206.0, 8.0)],
    tree: 0x4e9a4e,
};

fn hills_image(h: &Hills) -> Image {
    let mut img = Image::gen_image_color(LAYER_W, LAYER_H, Color::new(0.0, 0.0, 0.0, 0.0));
    let (body, rim, tree) = (rgb(h.body), rgb(h.rim), rgb(h.tree));
    for x in 0..LAYER_W as u32 {
        let top = profile(x as f32 + 0.5, h.base, h.waves).round() as u32;
        for y in top..LAYER_H as u32 {
            let c = if y < top + 2 { rim } else { body };
            img.set_pixel(x, y, c);
        }
    }
    for &(tx, r) in h.trees {
        let ground = profile(tx, h.base, h.waves);
        let (cx, cy) = (tx, ground - r - 3.0);
        for y in 0..LAYER_H as u32 {
            for x in 0..LAYER_W as u32 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let canopy = (px - cx).hypot(py - cy) <= r;
                let trunk = (px - cx).abs() < 1.5 && py > cy && py < ground + 1.0;
                if canopy || trunk {
                    img.set_pixel(x, y, tree);
                }
            }
        }
    }
    img
}

fn clouds_image() -> Image {
    const W: u16 = 128;
    const H: u16 = 32;
    let mut img = Image::gen_image_color(W, H, Color::new(0.0, 0.0, 0.0, 0.0));
    let (white, shade) = (rgb(0xffffff), rgb(0xe2f0fa));
    // Each cloud is a few overlapping puffs (x, y, r).
    let puffs: [(f32, f32, f32); 7] = [
        (16.0, 20.0, 7.0),
        (26.0, 15.0, 9.0),
        (37.0, 20.0, 7.0),
        (78.0, 13.0, 5.0),
        (86.0, 10.0, 7.0),
        (95.0, 13.0, 5.0),
        (86.0, 15.0, 6.0),
    ];
    for y in 0..H as u32 {
        for x in 0..W as u32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            if puffs
                .iter()
                .any(|&(cx, cy, r)| (px - cx).hypot(py - cy) <= r)
            {
                let lower = puffs
                    .iter()
                    .any(|&(cx, cy, r)| (px - cx).hypot(py - cy) <= r && py > cy + r * 0.45);
                img.set_pixel(x, y, if lower { shade } else { white });
            }
        }
    }
    img
}

/// Cave rock silhouettes (no trees).
const CAVE_FAR: Hills = Hills {
    base: 40.0,
    waves: &[(2.0, 16.0, 0.3), (5.0, 6.0, 1.7)],
    body: 0x2a2432,
    rim: 0x3a3244,
    trees: &[],
    tree: 0,
};

const CAVE_NEAR: Hills = Hills {
    base: 62.0,
    waves: &[(3.0, 12.0, 1.1), (7.0, 5.0, 0.2)],
    body: 0x1e1a22,
    rim: 0x302838,
    trees: &[],
    tree: 0,
};

/// Stalactites hanging from a rock ceiling (the hills turned upside down,
/// with spikes).
fn ceiling_image() -> Image {
    let mut img = Image::gen_image_color(LAYER_W, 48, Color::new(0.0, 0.0, 0.0, 0.0));
    let rock = rgb(0x241e2a);
    for x in 0..LAYER_W as u32 {
        let fx = x as f32 + 0.5;
        let mut depth = 8.0 + 4.0 * (TAU * 3.0 * fx / LAYER_W as f32).sin();
        // Spikes every ~21 px of varying length.
        let k = (fx / 21.0).fract();
        let len = 10.0 + 16.0 * ((fx / 21.0).floor() * 1.7).sin().abs();
        depth += (1.0 - (k - 0.5).abs() * 2.0).max(0.0).powi(2) * len;
        for y in 0..(depth as u32).min(48) {
            img.set_pixel(x, y, rock);
        }
    }
    img
}

/// Cave colour bands, top to bottom.
const CAVE: [u32; 6] = [0x14101a, 0x18131f, 0x1c1624, 0x211a2a, 0x261e30, 0x2b2236];

/// Backdrop names [`Background::draw`] knows (see `level::BACKGROUNDS`).
/// Bands top to bottom for the factory and the circus.
const FACTORY: [u32; 6] = [0x1a2028, 0x1e252e, 0x222a34, 0x262f3a, 0x2a3440, 0x2e3946];
const CIRCUS: [u32; 6] = [0x140c28, 0x1c1032, 0x26143c, 0x301846, 0x3a1c4e, 0x441e54];

fn bands(colours: &[u32; 6]) {
    let band = VIEW_H / colours.len() as f32;
    for (i, &c) in colours.iter().enumerate() {
        draw_rectangle(0.0, i as f32 * band, VIEW_W, band + 1.0, rgb(c));
    }
}

/// A cog silhouette: a disc with `teeth` square teeth, turned by `angle`.
fn gear(centre: Vec2, r: f32, teeth: u32, angle: f32, colour: Color) {
    draw_circle(centre.x, centre.y, r, colour);
    for k in 0..teeth {
        let a = angle + k as f32 * TAU / teeth as f32;
        let d = vec2(a.cos(), a.sin());
        let p = centre + d * (r + 2.0);
        let side = vec2(-d.y, d.x) * (r * 0.18).max(2.0);
        draw_triangle(
            p - side - d * 4.0,
            p + side - d * 4.0,
            p + side + d * 3.0,
            colour,
        );
        draw_triangle(
            p - side - d * 4.0,
            p + side + d * 3.0,
            p - side + d * 3.0,
            colour,
        );
    }
    draw_circle(centre.x, centre.y, r * 0.35, rgb(FACTORY[1]));
}

impl Background {
    /// Machine hall: big cogs turning far back, pipes in front.
    fn draw_factory(&self, view: Vec2, tick: u64) {
        bands(&FACTORY);
        let far = rgb(0x323c48);
        let spin = tick as f32 * 0.01;
        // Cogs repeat every 192 px of parallax scroll.
        let off = (view.x * 0.15).rem_euclid(192.0);
        for k in -1..3 {
            let x = k as f32 * 192.0 - off;
            gear(vec2(x + 60.0, 90.0), 34.0, 10, spin, far);
            gear(vec2(x + 118.0, 150.0), 22.0, 8, -spin * 1.5, far);
            gear(vec2(x + 160.0, 60.0), 16.0, 6, spin * 2.0, far);
        }
        let pipe = rgb(0x3e4a58);
        let rim = rgb(0x4e5c6c);
        let off = (view.x * 0.35).rem_euclid(160.0);
        draw_rectangle(0.0, 214.0, VIEW_W, 10.0, pipe);
        draw_rectangle(0.0, 214.0, VIEW_W, 2.0, rim);
        draw_rectangle(0.0, 250.0, VIEW_W, 6.0, pipe);
        for k in -1..3 {
            let x = k as f32 * 160.0 - off;
            draw_rectangle(x + 40.0, 210.0, 8.0, 18.0, rim);
            draw_rectangle(x + 110.0, 224.0, 10.0, 96.0, pipe);
            draw_rectangle(x + 110.0, 224.0, 2.0, 96.0, rim);
        }
    }

    /// Night circus: stars, striped tents, a string of coloured lights.
    fn draw_circus(&self, view: Vec2, tick: u64) {
        bands(&CIRCUS);
        for k in 0..24u32 {
            let x = ((k * 97 + 13) % 240) as f32 - (view.x * 0.03).rem_euclid(240.0);
            let x = x.rem_euclid(240.0);
            let y = ((k * 53 + 7) % 130) as f32;
            let tw = if (tick / 20 + k as u64).is_multiple_of(7) {
                0.4
            } else {
                0.9
            };
            draw_rectangle(x, y, 1.0, 1.0, Color::new(1.0, 1.0, 0.9, tw));
        }
        let off = (view.x * 0.25).rem_euclid(200.0);
        for k in -1..3 {
            let x = k as f32 * 200.0 - off;
            let (cx, w, base) = (x + 100.0, 130.0, 236.0);
            // Stripes of the tent body, then its cone roof.
            for s in 0..8 {
                let colour = if s % 2 == 0 {
                    rgb(0x8a2a3a)
                } else {
                    rgb(0xc8b8c0)
                };
                draw_rectangle(
                    cx - w / 2.0 + s as f32 * w / 8.0,
                    base - 40.0,
                    w / 8.0,
                    80.0,
                    colour,
                );
            }
            draw_triangle(
                vec2(cx - w / 2.0 - 6.0, base - 40.0),
                vec2(cx + w / 2.0 + 6.0, base - 40.0),
                vec2(cx, base - 96.0),
                rgb(0x8a2a3a),
            );
            draw_rectangle(cx - 1.0, base - 110.0, 2.0, 16.0, rgb(0x6a4a3a));
            draw_triangle(
                vec2(cx + 1.0, base - 110.0),
                vec2(cx + 12.0, base - 106.0),
                vec2(cx + 1.0, base - 102.0),
                rgb(0xffd84a),
            );
        }
        // Lights strung across the top, blinking in turn.
        let off = (view.x * 0.5).rem_euclid(24.0);
        for k in 0..12 {
            let x = k as f32 * 24.0 - off;
            let y = 34.0 + ((x + view.x * 0.5) * 0.13).sin() * 4.0;
            let colours = [0xff5a5a, 0xffd84a, 0x7ad8ff, 0x9cf0a8];
            let lit = (tick / 15 + k as u64) % 4;
            let c = rgb(colours[lit as usize]);
            draw_circle(x, y, 2.0, c);
        }
    }
}

#[cfg(test)]
pub const NAMES: &[&str] = &["forest", "cave", "factory", "circus"];

/// Sky colour bands, top to bottom.
const SKY: [u32; 6] = [0x5aaeea, 0x6ab8ee, 0x7cc2f1, 0x8fccf4, 0xa3d7f7, 0xb8e2fa];

fn texture(img: &Image) -> Texture2D {
    let t = Texture2D::from_image(img);
    t.set_filter(FilterMode::Nearest);
    t
}

pub struct Background {
    far: Texture2D,
    near: Texture2D,
    clouds: Texture2D,
    cave_far: Texture2D,
    cave_near: Texture2D,
    ceiling: Texture2D,
}

/// Draws `tex` repeated across the screen width, shifted left by `offset`.
fn tile_across(tex: &Texture2D, offset: f32, y: f32) {
    let w = tex.width();
    let mut x = -(offset.rem_euclid(w)).round();
    while x < VIEW_W {
        draw_texture(tex, x, y.round(), WHITE);
        x += w;
    }
}

impl Background {
    pub fn new() -> Background {
        Background {
            far: texture(&hills_image(&FAR)),
            near: texture(&hills_image(&NEAR)),
            clouds: texture(&clouds_image()),
            cave_far: texture(&hills_image(&CAVE_FAR)),
            cave_near: texture(&hills_image(&CAVE_NEAR)),
            ceiling: texture(&ceiling_image()),
        }
    }

    /// Draws in screen space (0..240, 0..320). `view` is the camera's
    /// top-left, `max_view_y` its lowest possible value (level height minus
    /// screen height), `tick` drives the cloud drift.
    pub fn draw(&self, name: &str, view: Vec2, max_view_y: f32, tick: u64) {
        match name {
            "cave" => return self.draw_cave(view, max_view_y),
            "factory" => return self.draw_factory(view, tick),
            "circus" => return self.draw_circus(view, tick),
            _ => {}
        }
        let band = VIEW_H / SKY.len() as f32;
        for (i, &c) in SKY.iter().enumerate() {
            draw_rectangle(0.0, i as f32 * band, VIEW_W, band + 1.0, rgb(c));
        }
        // How far the camera has climbed above the level's bottom.
        let climb = (max_view_y - view.y).max(0.0);

        let drift = tick as f32 * 0.12;
        tile_across(&self.clouds, view.x * 0.08 + drift, 24.0 + climb * 0.05);

        let far_y = 122.0 + climb * 0.1;
        tile_across(&self.far, view.x * 0.2, far_y);
        draw_rectangle(0.0, far_y + LAYER_H as f32, VIEW_W, VIEW_H, rgb(FAR.body));

        let near_y = 168.0 + climb * 0.25;
        tile_across(&self.near, view.x * 0.45, near_y);
        draw_rectangle(0.0, near_y + LAYER_H as f32, VIEW_W, VIEW_H, rgb(NEAR.body));
    }
}

impl Background {
    /// Dark rock: a ceiling with stalactites and two rows of rock
    /// silhouettes, all scrolling slowly.
    fn draw_cave(&self, view: Vec2, max_view_y: f32) {
        let band = VIEW_H / CAVE.len() as f32;
        for (i, &c) in CAVE.iter().enumerate() {
            draw_rectangle(0.0, i as f32 * band, VIEW_W, band + 1.0, rgb(c));
        }
        let climb = (max_view_y - view.y).max(0.0);
        tile_across(&self.ceiling, view.x * 0.15, -climb * 0.05);
        let far_y = 150.0 + climb * 0.1;
        tile_across(&self.cave_far, view.x * 0.2, far_y);
        draw_rectangle(
            0.0,
            far_y + LAYER_H as f32,
            VIEW_W,
            VIEW_H,
            rgb(CAVE_FAR.body),
        );
        let near_y = 196.0 + climb * 0.25;
        tile_across(&self.cave_near, view.x * 0.45, near_y);
        draw_rectangle(
            0.0,
            near_y + LAYER_H as f32,
            VIEW_W,
            VIEW_H,
            rgb(CAVE_NEAR.body),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_level_backdrop_is_drawable() {
        assert_eq!(NAMES, bounce_tales::level::BACKGROUNDS);
    }

    #[test]
    fn hill_profiles_tile_seamlessly() {
        for h in [&FAR, &NEAR] {
            let a = profile(0.0, h.base, h.waves);
            let b = profile(LAYER_W as f32, h.base, h.waves);
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }

    #[test]
    fn hills_and_trees_fit_in_the_texture() {
        for h in [&FAR, &NEAR] {
            for x in 0..LAYER_W {
                let top = profile(x as f32, h.base, h.waves);
                assert!(top > 0.0 && top < LAYER_H as f32 - 4.0, "x {x}: {top}");
            }
            for &(tx, r) in h.trees {
                let canopy_top = profile(tx, h.base, h.waves) - 2.0 * r - 3.0;
                assert!(canopy_top >= 0.0, "tree at {tx} clipped");
            }
        }
    }
}
