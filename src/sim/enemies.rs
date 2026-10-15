//! Enemies: creatures hypnotized by the Hypno Cube. Landing on one from
//! above frees it (it hops away); touching it any other way costs a life.
//! The heavy stone ball flattens walkers even from the side.
//!
//! Step 25 has walkers; flyers, jumpers and shooters join in step 26.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::{Ball, Form};
use super::crates::blockers;
use super::map::{TILE, TileMap};
use super::physics::SolidBox;

pub const WALKER_SPEED: f32 = 30.0;
const GRAVITY: f32 = 900.0;
const MAX_FALL: f32 = 420.0;
/// Upward speed given to the ball after a stomp, px/s.
pub const STOMP_BOUNCE: f32 = 280.0;
/// Points for freeing an enemy.
pub const FREE_POINTS: u32 = 100;
/// Steps a freed enemy takes to hop away and vanish (drawing).
pub const FREED_TICKS: u32 = 45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    /// Patrols along the ground, turning at walls and ledges.
    Walker,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub vel: Vec2,
    /// -1 left, +1 right.
    pub dir: f32,
    pub speed: f32,
    pub grounded: bool,
    /// Top-left before the last step (drawing).
    pub prev_pos: Vec2,
    /// `Some(steps left)` once freed: it hops away, then is gone.
    pub freed: Option<u32>,
    /// Fell out of the level or finished hopping away.
    pub gone: bool,
}

/// What a touch between the ball and an enemy means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Touch {
    /// The ball came down on top: the enemy is freed, the ball bounces.
    Stomp,
    /// A heavy stone rolled into it: freed, no bounce.
    Crush,
    /// The ball is hurt.
    Hurt,
}

impl Enemy {
    pub fn walker(pos: Vec2, dir: f32, speed: f32) -> Enemy {
        Enemy {
            kind: EnemyKind::Walker,
            vel: Vec2::ZERO,
            dir,
            speed,
            grounded: false,
            prev_pos: pos,
            freed: None,
            gone: false,
        }
    }

    /// Still a threat (not freed, not gone).
    pub fn active(&self) -> bool {
        self.freed.is_none() && !self.gone
    }

    /// One step of movement for the enemy at `pos` of `size`. Returns the
    /// new top-left.
    pub fn advance(&mut self, pos: Vec2, size: Vec2, map: &TileMap, solids: &[SolidBox]) -> Vec2 {
        self.prev_pos = pos;
        if self.gone {
            return pos;
        }
        if let Some(t) = self.freed {
            // Hop up and away, then vanish.
            if t <= 1 {
                self.gone = true;
            }
            self.freed = Some(t.saturating_sub(1));
            self.vel.y += GRAVITY * DT;
            return pos + self.vel * DT;
        }
        let mut pos = pos;
        self.vel.x = self.dir * self.speed;
        self.vel.y = (self.vel.y + GRAVITY * DT).min(MAX_FALL);

        pos.x += self.vel.x * DT;
        if !blockers(pos, size, map, solids).is_empty() {
            pos.x = self.prev_pos.x;
            self.dir = -self.dir;
        } else if self.grounded && !ground_ahead(pos, size, self.dir, map, solids) {
            // Never walk off a ledge.
            pos.x = self.prev_pos.x;
            self.dir = -self.dir;
        }

        pos.y += self.vel.y * DT;
        self.grounded = false;
        for (min, _) in blockers(pos, size, map, solids) {
            if self.vel.y > 0.0 {
                pos.y = pos.y.min(min.y - size.y);
                self.grounded = true;
            }
        }
        if self.grounded {
            self.vel.y = 0.0;
        }
        if pos.y > map.pixel_size().y + 2.0 * TILE {
            self.gone = true;
        }
        pos
    }

    /// How the ball touches the enemy box (`pos`, `size`), if at all.
    pub fn touch(&self, ball: &Ball, pos: Vec2, size: Vec2) -> Option<Touch> {
        if !self.active() {
            return None;
        }
        let closest = ball.pos.clamp(pos, pos + size);
        if ball.pos.distance(closest) >= ball.radius {
            return None;
        }
        // Coming down with the centre above the enemy's top edge.
        let from_above = ball.vel.y > 0.0 && ball.pos.y < pos.y + 2.0;
        Some(if from_above {
            Touch::Stomp
        } else if ball.form == Form::Stone {
            Touch::Crush
        } else {
            Touch::Hurt
        })
    }

    /// Frees the enemy: it hops up and fades away.
    pub fn free(&mut self) {
        self.freed = Some(FREED_TICKS);
        self.vel = vec2(-self.dir * 40.0, -200.0);
    }
}

/// Is there something to stand on just past the front foot?
fn ground_ahead(pos: Vec2, size: Vec2, dir: f32, map: &TileMap, solids: &[SolidBox]) -> bool {
    let probe_x = if dir > 0.0 {
        pos.x + size.x + 1.0
    } else {
        pos.x - 1.0
    };
    let probe = vec2(probe_x, pos.y + size.y + 1.0);
    let (cx, cy) = TileMap::cell_at(probe);
    map.is_solid(cx, cy)
        || solids.iter().any(|&(min, max)| {
            probe.x >= min.x && probe.x <= max.x && probe.y >= min.y && probe.y <= max.y
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    const S: Vec2 = Vec2::new(16.0, 16.0);

    fn walk(map: &TileMap, start: Vec2, steps: usize) -> (Enemy, Vec<f32>) {
        let mut e = Enemy::walker(start, 1.0, WALKER_SPEED);
        let mut pos = start;
        let mut xs = Vec::new();
        for _ in 0..steps {
            pos = e.advance(pos, S, map, &[]);
            xs.push(pos.x);
        }
        (e, xs)
    }

    #[test]
    fn turns_at_walls() {
        let map = TileMap::from_ascii("#o....#\n#######").unwrap();
        let (_, xs) = walk(&map, vec2(32.0, 0.0), 600);
        let max = xs.iter().cloned().fold(f32::MIN, f32::max);
        let min = xs.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            max <= 6.0 * 16.0 - 16.0 + 0.01,
            "went through the wall: {max}"
        );
        assert!(min <= 17.0, "came back: {min}");
    }

    #[test]
    fn never_walks_off_a_ledge() {
        let map = TileMap::from_ascii("o.......\n........\n.####...").unwrap();
        let (e, xs) = walk(&map, vec2(32.0, 0.0), 600);
        assert!(!e.gone);
        let max = xs.iter().cloned().fold(f32::MIN, f32::max);
        assert!(max <= 5.0 * 16.0 - 16.0 + 1.0, "walked off: {max}");
    }

    #[test]
    fn stomp_hurt_and_crush() {
        let e = Enemy::walker(vec2(100.0, 100.0), 1.0, WALKER_SPEED);
        let (pos, size) = (vec2(100.0, 100.0), S);
        let mut above = ball_at(108.0, 93.0);
        above.vel.y = 200.0;
        assert_eq!(e.touch(&above, pos, size), Some(Touch::Stomp));
        let side = ball_at(94.0, 108.0);
        assert_eq!(e.touch(&side, pos, size), Some(Touch::Hurt));
        let mut stone = ball_at(94.0, 108.0);
        stone.set_form(Form::Stone);
        assert_eq!(e.touch(&stone, pos, size), Some(Touch::Crush));
        assert_eq!(e.touch(&ball_at(60.0, 108.0), pos, size), None);
        let mut freed = e.clone();
        freed.free();
        assert_eq!(
            freed.touch(&side, pos, size),
            None,
            "freed enemies are harmless"
        );
    }
}
