//! Pushable crates: boxes with gravity that rest on solid tiles and on each
//! other, float in water, and slide when the ball presses into their side.
//!
//! Crates collide as axis-aligned boxes (a slope tile counts as its
//! bounding box, so crates don't slide down slopes). They ignore moving
//! platforms. Moving one axis at a time keeps resting contact exact.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::approach;
use super::map::{TILE, TileMap, flags};
use super::physics::SolidBox;

/// Fastest the ball can push a crate, px/s (crates feel heavy).
pub const PUSH_SPEED: f32 = 70.0;
const GRAVITY: f32 = 900.0;
const MAX_FALL: f32 = 420.0;
/// Slow-down on the ground when nothing pushes, px/s².
const FRICTION: f32 = 900.0;
/// Upward push when fully under water, × gravity (wood floats).
const BUOYANCY: f32 = 1.5;
const WATER_DRAG: f32 = 4.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Crate {
    pub vel: Vec2,
    pub grounded: bool,
    /// Top-left before the last step, for drawing.
    pub prev_pos: Vec2,
    /// Push from the ball (its horizontal speed), applied next step.
    pub push: Option<f32>,
    /// Fell out of the level.
    pub gone: bool,
}

impl Crate {
    pub fn new(pos: Vec2) -> Crate {
        Crate {
            vel: Vec2::ZERO,
            grounded: false,
            prev_pos: pos,
            push: None,
            gone: false,
        }
    }

    /// One step of movement for the crate at `pos` of `size`, blocked by
    /// solid tiles and `others` (the other crates). Returns the new top-left.
    pub fn advance(&mut self, pos: Vec2, size: Vec2, map: &TileMap, others: &[SolidBox]) -> Vec2 {
        self.prev_pos = pos;
        if self.gone {
            return pos;
        }
        match self.push.take() {
            Some(vx) => self.vel.x = vx.clamp(-PUSH_SPEED, PUSH_SPEED),
            None if self.grounded => self.vel.x = approach(self.vel.x, 0.0, FRICTION * DT),
            None => {}
        }
        self.vel.y = (self.vel.y + GRAVITY * DT).min(MAX_FALL);
        let wet = submersion(pos, size, map);
        if wet > 0.0 {
            self.vel.y -= GRAVITY * BUOYANCY * wet * DT;
            self.vel *= 1.0 - WATER_DRAG * wet * DT;
        }

        let mut pos = pos;
        // Horizontal, then vertical.
        pos.x += self.vel.x * DT;
        for (min, max) in blockers(pos, size, map, others) {
            if self.vel.x > 0.0 {
                pos.x = pos.x.min(min.x - size.x);
            } else if self.vel.x < 0.0 {
                pos.x = pos.x.max(max.x);
            }
        }
        if !blockers(pos, size, map, others).is_empty() {
            // Couldn't resolve cleanly (pushed into a corner): undo.
            pos.x = self.prev_pos.x;
        }
        if pos.x != self.prev_pos.x + self.vel.x * DT {
            self.vel.x = 0.0;
        }

        pos.y += self.vel.y * DT;
        self.grounded = false;
        for (min, max) in blockers(pos, size, map, others) {
            if self.vel.y > 0.0 {
                pos.y = pos.y.min(min.y - size.y);
                self.grounded = true;
            } else if self.vel.y < 0.0 {
                pos.y = pos.y.max(max.y);
            }
        }
        if self.grounded || pos.y != self.prev_pos.y + self.vel.y * DT {
            self.vel.y = 0.0;
        }

        if pos.y > map.pixel_size().y + 2.0 * TILE {
            self.gone = true;
        }
        pos
    }
}

/// Solid things overlapping a box at `pos` of `size` (strictly, so boxes
/// resting flush against each other don't count).
fn blockers(pos: Vec2, size: Vec2, map: &TileMap, others: &[SolidBox]) -> Vec<SolidBox> {
    const EPS: f32 = 1e-3;
    let overlaps = |min: Vec2, max: Vec2| {
        pos.x + size.x > min.x + EPS
            && pos.x < max.x - EPS
            && pos.y + size.y > min.y + EPS
            && pos.y < max.y - EPS
    };
    let mut out = Vec::new();
    let (x0, y0) = TileMap::cell_at(pos);
    let (x1, y1) = TileMap::cell_at(pos + size);
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let tile = map.get(tx, ty);
            if !tile.is_solid() {
                continue;
            }
            let origin = vec2(tx as f32, ty as f32) * TILE;
            let poly = tile.polygon();
            // Outside the map the side walls count as full squares.
            let (mut min, mut max) = if poly.is_empty() {
                (Vec2::ZERO, Vec2::splat(TILE))
            } else {
                (Vec2::splat(TILE), Vec2::ZERO)
            };
            for &v in poly {
                min = min.min(v);
                max = max.max(v);
            }
            let (min, max) = (origin + min, origin + max);
            if overlaps(min, max) {
                out.push((min, max));
            }
        }
    }
    out.extend(
        others
            .iter()
            .copied()
            .filter(|&(min, max)| overlaps(min, max)),
    );
    out
}

/// Share (0..1) of the box's height inside water cells under its centre.
fn submersion(pos: Vec2, size: Vec2, map: &TileMap) -> f32 {
    let col = ((pos.x + size.x / 2.0) / TILE).floor() as i32;
    let (top, bottom) = (pos.y, pos.y + size.y);
    let mut wet = 0.0;
    for row in (top / TILE).floor() as i32..=(bottom / TILE).floor() as i32 {
        if map.flags(col, row) & flags::WATER != 0 {
            let (a, b) = (row as f32 * TILE, (row + 1) as f32 * TILE);
            wet += (bottom.min(b) - top.max(a)).max(0.0);
        }
    }
    (wet / size.y).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Vec2 = Vec2::new(16.0, 16.0);

    fn settle(c: &mut Crate, mut pos: Vec2, map: &TileMap, steps: usize) -> Vec2 {
        for _ in 0..steps {
            pos = c.advance(pos, S, map, &[]);
        }
        pos
    }

    #[test]
    fn falls_and_rests_on_the_floor() {
        let map = TileMap::from_ascii("o...\n....\n....\n####").unwrap();
        let mut c = Crate::new(vec2(16.0, 0.0));
        let pos = settle(&mut c, vec2(16.0, 0.0), &map, 60);
        assert_eq!(pos, vec2(16.0, 32.0));
        assert!(c.grounded);
        assert_eq!(c.vel, Vec2::ZERO);
    }

    #[test]
    fn pushed_slides_then_stops_at_a_wall() {
        let map = TileMap::from_ascii("o.....#\n#######").unwrap();
        let mut c = Crate::new(vec2(16.0, 0.0));
        let mut pos = settle(&mut c, vec2(16.0, 0.0), &map, 5);
        for _ in 0..120 {
            c.push = Some(200.0);
            pos = c.advance(pos, S, &map, &[]);
            assert!(c.vel.x <= PUSH_SPEED);
        }
        assert_eq!(pos.x, 6.0 * 16.0 - 16.0, "stops flush against the wall");
    }

    #[test]
    fn stops_by_friction_when_not_pushed() {
        let map = TileMap::from_ascii("o.........\n##########").unwrap();
        let mut c = Crate::new(vec2(16.0, 0.0));
        let mut pos = settle(&mut c, vec2(16.0, 0.0), &map, 5);
        c.push = Some(PUSH_SPEED);
        pos = c.advance(pos, S, &map, &[]);
        let x = pos.x;
        pos = settle(&mut c, pos, &map, 30);
        assert!(pos.x - x < 4.0, "slid {}", pos.x - x);
    }

    #[test]
    fn stacks_on_another_crate() {
        let map = TileMap::from_ascii("o...\n....\n....\n####").unwrap();
        let below = (vec2(16.0, 32.0), vec2(32.0, 48.0));
        let mut c = Crate::new(vec2(16.0, 0.0));
        let mut pos = vec2(16.0, 0.0);
        for _ in 0..60 {
            pos = c.advance(pos, S, &map, &[below]);
        }
        assert_eq!(pos, vec2(16.0, 16.0));
    }

    #[test]
    fn floats_in_water_and_falls_out_of_the_level() {
        let pool = TileMap::from_ascii("o...\n.~~.\n.~~.\n####").unwrap();
        let mut c = Crate::new(vec2(16.0, 0.0));
        let pos = settle(&mut c, vec2(16.0, 0.0), &pool, 240);
        assert!(
            pos.y > 8.0 && pos.y < 24.0,
            "floats near the surface: {}",
            pos.y
        );

        let pit = TileMap::from_ascii("o...\n....").unwrap();
        let mut c = Crate::new(vec2(16.0, 0.0));
        settle(&mut c, vec2(16.0, 0.0), &pit, 120);
        assert!(c.gone);
    }
}
