//! Enemies: creatures hypnotized by the Hypno Cube. Landing on one from
//! above frees it (it hops away); touching it any other way costs a life.
//! The heavy stone ball flattens walkers even from the side.
//!
//! Kinds: walkers patrol the ground, flyers glide back and forth in the
//! air, jumpers hop in place, shooters fire slow pellets.

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnemyKind {
    /// Patrols along the ground, turning at walls and ledges.
    Walker,
    /// Glides back and forth from `origin` along `axis` over `range` px
    /// (eased at the ends), ignoring gravity. `phase` in radians.
    Flyer {
        origin: Vec2,
        axis: Vec2,
        range: f32,
        phase: f32,
    },
    /// Sits on the ground and hops every `period` steps.
    Jumper { period: u32, timer: u32 },
    /// Fires a pellet every `period` steps, straight ahead or `aim`ed at
    /// the ball.
    Shooter { period: u32, timer: u32, aim: bool },
}

/// A jumper's hop speed, px/s (~60 px high).
pub const JUMPER_HOP: f32 = 330.0;
/// Pellet speed, px/s, and lifetime, steps.
pub const SHOT_SPEED: f32 = 140.0;
pub const SHOT_LIFE: u32 = 240;
/// Pellet radius, px.
pub const SHOT_RADIUS: f32 = 3.0;
/// Shooters only aim/fire at a ball within this distance, px.
pub const SHOOT_RANGE: f32 = 240.0;

/// A shooter's pellet.
#[derive(Debug, Clone, PartialEq)]
pub struct Projectile {
    pub pos: Vec2,
    pub prev_pos: Vec2,
    pub vel: Vec2,
    pub life: u32,
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
    /// Set by a shooter when it fires this step (the sim spawns the
    /// pellet).
    pub fire: Option<Projectile>,
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
            fire: None,
        }
    }

    pub fn flyer(pos: Vec2, axis: Vec2, range: f32, speed: f32) -> Enemy {
        Enemy {
            kind: EnemyKind::Flyer {
                origin: pos,
                axis,
                range,
                phase: 0.0,
            },
            ..Enemy::walker(pos, 1.0, speed)
        }
    }

    pub fn jumper(pos: Vec2, period: u32) -> Enemy {
        Enemy {
            kind: EnemyKind::Jumper {
                period,
                timer: period,
            },
            ..Enemy::walker(pos, -1.0, 0.0)
        }
    }

    pub fn shooter(pos: Vec2, dir: f32, period: u32, aim: bool) -> Enemy {
        Enemy {
            kind: EnemyKind::Shooter {
                period,
                timer: period,
                aim,
            },
            ..Enemy::walker(pos, dir, 0.0)
        }
    }

    /// Still a threat (not freed, not gone).
    pub fn active(&self) -> bool {
        self.freed.is_none() && !self.gone
    }

    /// One step of movement for the enemy at `pos` of `size`. Returns the
    /// new top-left.
    pub fn advance(
        &mut self,
        pos: Vec2,
        size: Vec2,
        map: &TileMap,
        solids: &[SolidBox],
        ball: Vec2,
    ) -> Vec2 {
        self.prev_pos = pos;
        self.fire = None;
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
        match &mut self.kind {
            EnemyKind::Flyer {
                origin,
                axis,
                range,
                phase,
            } => {
                // Smooth back-and-forth: (1 - cos) / 2 eases at both ends.
                let rate = std::f32::consts::PI * self.speed / range.max(1.0);
                *phase += rate * DT;
                let new = *origin + *axis * *range * (1.0 - phase.cos()) / 2.0;
                self.vel = (new - pos) / DT;
                if self.vel.x != 0.0 {
                    self.dir = self.vel.x.signum();
                }
                return new;
            }
            EnemyKind::Jumper { period, timer } => {
                *timer = timer.saturating_sub(1);
                if *timer == 0 && self.grounded {
                    self.vel.y = -JUMPER_HOP;
                    self.grounded = false;
                    *timer = *period;
                }
                // Look towards the ball.
                self.dir = if ball.x < pos.x + size.x / 2.0 {
                    -1.0
                } else {
                    1.0
                };
            }
            EnemyKind::Shooter { period, timer, aim } => {
                *timer = timer.saturating_sub(1);
                let centre = pos + size / 2.0;
                let in_range = ball.distance(centre) <= SHOOT_RANGE;
                if *aim {
                    self.dir = if ball.x < centre.x { -1.0 } else { 1.0 };
                }
                if *timer == 0 && in_range {
                    *timer = *period;
                    let dir = if *aim {
                        (ball - centre).normalize_or(vec2(self.dir, 0.0))
                    } else {
                        vec2(self.dir, 0.0)
                    };
                    let start = centre + dir * (size.x / 2.0 + SHOT_RADIUS + 1.0);
                    self.fire = Some(Projectile {
                        pos: start,
                        prev_pos: start,
                        vel: dir * SHOT_SPEED,
                        life: SHOT_LIFE,
                    });
                }
            }
            EnemyKind::Walker => {}
        }
        let mut pos = pos;
        self.vel.x = self.dir * self.speed;
        self.vel.y = (self.vel.y + GRAVITY * DT).min(MAX_FALL);

        pos.x += self.vel.x * DT;
        // Only walkers turn around; jumpers and shooters keep their facing.
        let walks = self.speed > 0.0;
        if walks && !blockers(pos, size, map, solids).is_empty() {
            pos.x = self.prev_pos.x;
            self.dir = -self.dir;
        } else if walks && self.grounded && !ground_ahead(pos, size, self.dir, map, solids) {
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

impl Projectile {
    /// Moves one step. Returns false when it hits a solid tile or expires.
    pub fn advance(&mut self, map: &TileMap) -> bool {
        self.prev_pos = self.pos;
        self.pos += self.vel * DT;
        self.life = self.life.saturating_sub(1);
        let (cx, cy) = TileMap::cell_at(self.pos);
        self.life > 0 && !map.is_solid(cx, cy)
    }

    /// Does it touch the ball?
    pub fn hits(&self, ball: &Ball) -> bool {
        self.pos.distance(ball.pos) < ball.radius + SHOT_RADIUS
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
            pos = e.advance(pos, S, map, &[], Vec2::ZERO);
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
    fn flyer_glides_back_and_forth_without_falling() {
        let map = TileMap::from_ascii("o.........\n..........").unwrap();
        let start = vec2(32.0, 0.0);
        let mut e = Enemy::flyer(start, vec2(1.0, 0.0), 64.0, 40.0);
        let mut pos = start;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for _ in 0..600 {
            pos = e.advance(pos, S, &map, &[], Vec2::ZERO);
            lo = lo.min(pos.x);
            hi = hi.max(pos.x);
            assert_eq!(pos.y, 0.0, "no gravity");
        }
        assert!((31.99..33.0).contains(&lo), "{lo}");
        assert!((95.0..=96.01).contains(&hi), "{hi}");
    }

    #[test]
    fn jumper_hops_regularly() {
        let map = TileMap::from_ascii("o...\n....\n....\n####").unwrap();
        let mut e = Enemy::jumper(vec2(16.0, 32.0), 60);
        let mut pos = vec2(16.0, 32.0);
        let mut hops = 0;
        let mut was_up = false;
        let mut highest = f32::MAX;
        for _ in 0..300 {
            pos = e.advance(pos, S, &map, &[], vec2(100.0, 40.0));
            highest = highest.min(pos.y);
            let up = pos.y < 30.0;
            if up && !was_up {
                hops += 1;
            }
            was_up = up;
        }
        assert!((4..=6).contains(&hops), "{hops} hops in 5 s");
        assert!(32.0 - highest > 40.0, "hop height {}", 32.0 - highest);
    }

    #[test]
    fn shooter_fires_when_the_ball_is_near_and_pellets_stop_at_walls() {
        let map = TileMap::from_ascii("o......#\n########").unwrap();
        let mut e = Enemy::shooter(vec2(16.0, 0.0), 1.0, 60, false);
        let mut pos = vec2(16.0, 0.0);
        let mut shots = Vec::new();
        for _ in 0..200 {
            pos = e.advance(pos, S, &map, &[], vec2(80.0, 8.0));
            if let Some(p) = e.fire.take() {
                shots.push(p);
            }
        }
        assert_eq!(shots.len(), 3, "every 60 steps");
        let mut p = shots[0].clone();
        let mut steps = 0;
        while p.advance(&map) {
            steps += 1;
        }
        assert!(
            p.pos.x >= 7.0 * 16.0 && p.pos.x < 7.0 * 16.0 + 4.0,
            "stopped at the wall: {}",
            p.pos.x
        );
        assert!(steps < SHOT_LIFE);
        // Far away: no shots.
        let mut far = Enemy::shooter(vec2(16.0, 0.0), 1.0, 60, false);
        for _ in 0..200 {
            far.advance(vec2(16.0, 0.0), S, &map, &[], vec2(900.0, 8.0));
            assert!(far.fire.is_none());
        }
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
