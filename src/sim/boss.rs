//! The Hypno Cube: the final boss. It hovers over the arena firing pellet
//! spreads at the ball, then shakes above the ball as a warning and slams
//! down. Landed, it sits dazed for a moment: stomping its top then is the
//! only way to hurt it. Three hits beat it; each hit makes it faster,
//! shoot more and recover sooner. Touching its sides or bottom costs a life;
//! touching its top while it isn't dazed just bounces the ball off.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;
use super::crates::blockers;
use super::enemies::{Projectile, SHOT_LIFE, SHOT_RADIUS};
use super::map::TileMap;

pub const BOSS_HP: u32 = 3;
/// Side of the cube, px. A 52 px jump clears a landed 40 px cube by 12 px
/// (at 48 the margin was 4 px — too fiddly to stomp).
pub const BOSS_SIZE: f32 = 40.0;
/// Steps of each state (index = hits taken so far: 0, 1, 2).
const HOVER_TICKS: [u32; 3] = [200, 160, 130];
pub const WARN_TICKS: u32 = 50;
// Each hit makes it harder (checked at compile time).
const _: () = assert!(HOVER_SPEED[2] > HOVER_SPEED[0]);
const _: () = assert!(FIRE_PERIOD[2] < FIRE_PERIOD[0]);
const _: () = assert!(DAZED_TICKS[2] < DAZED_TICKS[0]);

/// Steps at the end of hovering without firing.
const QUIET_BEFORE_SLAM: u32 = 70;
const DAZED_TICKS: [u32; 3] = [130, 105, 85];
const FIRE_PERIOD: [u32; 3] = [70, 55, 42];
const SPREAD: [u32; 3] = [3, 5, 5];
const HOVER_SPEED: [f32; 3] = [45.0, 65.0, 85.0];
const SLAM_SPEED: f32 = 520.0;
const RISE_SPEED: f32 = 110.0;
const BOSS_SHOT_SPEED: f32 = 120.0;
/// Upward speed given to the ball bouncing off the cube, px/s.
pub const BOUNCE_OFF: f32 = 300.0;
/// Sideways speed when bounced off a cube that isn't dazed, px/s.
pub const KNOCK_OFF: f32 = 160.0;
/// Steps the shatter animation lasts before the level is won.
pub const DYING_TICKS: u32 = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BossState {
    /// Drifting side to side at hover height, firing.
    Hover {
        timer: u32,
    },
    /// Lined up over the ball, shaking (the warning before a slam).
    Warn {
        timer: u32,
    },
    /// Dropping fast.
    Slam,
    /// Landed and dizzy: its top can be stomped.
    Dazed {
        timer: u32,
    },
    /// Going back up to hover height.
    Rise,
    /// Beaten: shattering.
    Dying {
        timer: u32,
    },
    Dead,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Boss {
    pub hp: u32,
    pub state: BossState,
    /// Top of the cube while hovering.
    pub hover_y: f32,
    pub dir: f32,
    pub fire_timer: u32,
    pub prev_pos: Vec2,
    /// Steps the hit flash lasts (drawing).
    pub flash: u32,
    /// Pellets fired this step (the sim adds them).
    pub fired: Vec<Projectile>,
}

/// What a touch between the ball and the cube means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossTouch {
    /// Stomped while dazed: a hit.
    Hit,
    /// Landed on top while it isn't dazed: harmless bounce.
    Bounce,
    /// Any other contact: the ball is hurt.
    Hurt,
}

impl Boss {
    pub fn new(pos: Vec2) -> Boss {
        Boss {
            hp: BOSS_HP,
            state: BossState::Hover {
                timer: HOVER_TICKS[0],
            },
            hover_y: pos.y,
            dir: 1.0,
            fire_timer: FIRE_PERIOD[0] / 2,
            prev_pos: pos,
            flash: 0,
            fired: Vec::new(),
        }
    }

    /// Hits taken so far (0..=2 while fighting): picks the phase's numbers.
    fn phase(&self) -> usize {
        (BOSS_HP - self.hp.max(1)) as usize
    }

    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn dazed(&self) -> bool {
        matches!(self.state, BossState::Dazed { .. })
    }

    /// One step of the fight. Returns the new top-left.
    pub fn advance(&mut self, pos: Vec2, map: &TileMap, ball: Vec2) -> Vec2 {
        self.prev_pos = pos;
        self.fired.clear();
        self.flash = self.flash.saturating_sub(1);
        let size = Vec2::splat(BOSS_SIZE);
        let centre = pos + size / 2.0;
        let ph = self.phase();
        let mut pos = pos;
        match self.state {
            BossState::Hover { timer } => {
                pos.x += self.dir * HOVER_SPEED[ph] * DT;
                if !blockers(pos, size, map, &[]).is_empty() {
                    pos.x = self.prev_pos.x;
                    self.dir = -self.dir;
                }
                self.fire_timer = self.fire_timer.saturating_sub(1);
                // It stops shooting shortly before a slam, so the warning
                // is never mixed with pellets in the air.
                if self.fire_timer == 0 && timer > QUIET_BEFORE_SLAM {
                    self.fire_timer = FIRE_PERIOD[ph];
                    self.fire_spread(centre, ball, SPREAD[ph]);
                }
                self.state = if timer > 1 {
                    BossState::Hover { timer: timer - 1 }
                } else {
                    BossState::Warn { timer: WARN_TICKS }
                };
            }
            BossState::Warn { timer } => {
                // First half: slide over the ball. Second half: locked in
                // place and shaking (drawn), so there is time to get out.
                if timer > WARN_TICKS / 2 {
                    let target = ball.x - BOSS_SIZE / 2.0;
                    let step = 160.0 * DT;
                    let nx = pos.x + (target - pos.x).clamp(-step, step);
                    if blockers(vec2(nx, pos.y), size, map, &[]).is_empty() {
                        pos.x = nx;
                    }
                }
                self.state = if timer > 1 {
                    BossState::Warn { timer: timer - 1 }
                } else {
                    BossState::Slam
                };
            }
            BossState::Slam => {
                pos.y += SLAM_SPEED * DT;
                let hits = blockers(pos, size, map, &[]);
                if let Some(top) = hits.iter().map(|(min, _)| min.y).reduce(f32::min) {
                    pos.y = top - BOSS_SIZE;
                    self.state = BossState::Dazed {
                        timer: DAZED_TICKS[ph],
                    };
                }
            }
            BossState::Dazed { timer } => {
                self.state = if timer > 1 {
                    BossState::Dazed { timer: timer - 1 }
                } else {
                    BossState::Rise
                };
            }
            BossState::Rise => {
                pos.y = (pos.y - RISE_SPEED * DT).max(self.hover_y);
                if pos.y <= self.hover_y {
                    self.state = BossState::Hover {
                        timer: HOVER_TICKS[ph],
                    };
                    // First spread comes soon after reaching hover height.
                    self.fire_timer = FIRE_PERIOD[ph] / 2;
                }
            }
            BossState::Dying { timer } => {
                self.state = if timer > 1 {
                    BossState::Dying { timer: timer - 1 }
                } else {
                    BossState::Dead
                };
            }
            BossState::Dead => {}
        }
        pos
    }

    /// `count` pellets fanned around the direction to the ball.
    fn fire_spread(&mut self, from: Vec2, ball: Vec2, count: u32) {
        let aim = (ball - from).normalize_or(vec2(0.0, 1.0));
        let base = super::math::atan2(aim.y, aim.x);
        let gap = 0.28;
        for i in 0..count {
            let a = base + (i as f32 - (count - 1) as f32 / 2.0) * gap;
            let dir = vec2(super::math::cos(a), super::math::sin(a));
            let start = from + dir * (BOSS_SIZE / 2.0 + SHOT_RADIUS + 2.0);
            self.fired.push(Projectile {
                pos: start,
                prev_pos: start,
                vel: dir * BOSS_SHOT_SPEED,
                life: SHOT_LIFE,
            });
        }
    }

    /// How the ball touches the cube at `pos`, if at all.
    pub fn touch(&self, ball: &Ball, pos: Vec2) -> Option<BossTouch> {
        // Retreating after a slam or a hit, it can't hurt (fair after a
        // stomp sends the ball flying right beside it).
        if !self.alive() || self.state == BossState::Rise {
            return None;
        }
        let size = Vec2::splat(BOSS_SIZE);
        let closest = ball.pos.clamp(pos, pos + size);
        if ball.pos.distance(closest) >= ball.radius {
            return None;
        }
        let on_top = ball.vel.y >= 0.0 && ball.pos.y < pos.y + 2.0;
        Some(match (on_top, self.dazed()) {
            (true, true) => BossTouch::Hit,
            (true, false) => BossTouch::Bounce,
            (false, _) => BossTouch::Hurt,
        })
    }

    /// Takes a hit: rises again (faster phase), or starts dying.
    pub fn hit(&mut self) {
        self.hp = self.hp.saturating_sub(1);
        self.flash = 20;
        self.state = if self.hp == 0 {
            BossState::Dying { timer: DYING_TICKS }
        } else {
            BossState::Rise
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arena() -> TileMap {
        let mut rows = vec!["#.............#".to_owned(); 18];
        rows[17].replace_range(1..2, "o");
        rows.push("###############".into());
        rows.push("###############".into());
        TileMap::from_ascii(&rows.join("\n")).unwrap()
    }

    fn run_until(
        b: &mut Boss,
        pos: &mut Vec2,
        map: &TileMap,
        ball: Vec2,
        max: usize,
        done: impl Fn(&Boss) -> bool,
    ) -> usize {
        for i in 0..max {
            *pos = b.advance(*pos, map, ball);
            if done(b) {
                return i;
            }
        }
        panic!("state never reached: {:?}", b.state);
    }

    #[test]
    fn hovers_fires_warns_slams_and_lands_dazed() {
        let map = arena();
        let mut pos = vec2(96.0, 48.0);
        let mut b = Boss::new(pos);
        let ball = vec2(120.0, 280.0);
        let mut shots = 0;
        for _ in 0..HOVER_TICKS[0] {
            pos = b.advance(pos, &map, ball);
            shots += b.fired.len();
        }
        assert!(shots >= 6, "fired {shots}");
        assert!(matches!(b.state, BossState::Warn { .. }));
        run_until(&mut b, &mut pos, &map, ball, 400, |b| b.dazed());
        assert_eq!(pos.y + BOSS_SIZE, 288.0, "landed on the floor");
        assert!(
            (pos.x + BOSS_SIZE / 2.0 - 120.0).abs() < 2.0,
            "slammed over the ball"
        );
        run_until(&mut b, &mut pos, &map, ball, 600, |b| {
            matches!(b.state, BossState::Hover { .. })
        });
        assert_eq!(pos.y, 48.0, "back at hover height");
    }

    #[test]
    fn three_hits_then_it_dies() {
        let mut b = Boss::new(vec2(96.0, 48.0));
        b.hit();
        assert_eq!(b.state, BossState::Rise);
        b.hit();
        b.hit();
        assert!(matches!(b.state, BossState::Dying { .. }));
        let map = arena();
        let mut pos = vec2(96.0, 48.0);
        run_until(&mut b, &mut pos, &map, Vec2::ZERO, 300, |b| {
            b.state == BossState::Dead
        });
    }

    #[test]
    fn touch_rules() {
        let mut b = Boss::new(vec2(100.0, 100.0));
        let pos = vec2(100.0, 100.0);
        let mut top = crate::sim::ball::ball_at(124.0, 93.0);
        top.vel.y = 50.0;
        assert_eq!(b.touch(&top, pos), Some(BossTouch::Bounce));
        b.state = BossState::Dazed { timer: 10 };
        assert_eq!(b.touch(&top, pos), Some(BossTouch::Hit));
        let side = crate::sim::ball::ball_at(94.0, 124.0);
        assert_eq!(b.touch(&side, pos), Some(BossTouch::Hurt));
        b.state = BossState::Rise;
        assert_eq!(b.touch(&side, pos), None, "harmless while rising");
    }
}
