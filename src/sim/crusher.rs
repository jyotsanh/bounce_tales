//! Crushers ("Trapped in Machine"): a heavy steel head on a rod waits up
//! high, slams down to the floor below, holds, and grinds back up. Being
//! under it while it slams or rests on the floor is fatal; while it waits
//! up or rises it is harmless, which is when to roll underneath.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;
use super::map::{TILE, TileMap};

/// Steps waiting at the top.
pub const UP_TICKS: u32 = 70;
/// Steps resting on the floor after a slam.
pub const HOLD_TICKS: u32 = 30;
/// Slam and rise speeds, px/s.
pub const SLAM_SPEED: f32 = 420.0;
pub const RISE_SPEED: f32 = 70.0;
/// Longest drop looked for below the head, px.
const MAX_DROP: f32 = 20.0 * TILE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrusherState {
    Up { ticks: u32 },
    Slamming,
    Holding { ticks: u32 },
    Rising,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Crusher {
    /// Top-left of the head when raised.
    pub home: Vec2,
    pub size: Vec2,
    pub state: CrusherState,
    /// How far the head has dropped, px.
    pub offset: f32,
    pub prev_offset: f32,
    /// Distance to the floor under the head (found on the first step).
    pub drop: Option<f32>,
}

/// What a step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrusherEvent {
    None,
    /// The head hit the floor.
    Slammed,
}

impl Crusher {
    /// `delay`: steps to wait before the first slam (staggers a row).
    pub fn new(home: Vec2, size: Vec2, delay: u32) -> Crusher {
        Crusher {
            home,
            size,
            state: CrusherState::Up {
                ticks: UP_TICKS + delay,
            },
            offset: 0.0,
            prev_offset: 0.0,
            drop: None,
        }
    }

    /// Distance from the head's bottom to solid ground under its middle.
    fn measure(&self, map: &TileMap) -> f32 {
        let x = self.home.x + self.size.x / 2.0;
        let mut d = 0.0;
        while d < MAX_DROP {
            let probe = vec2(x, self.home.y + self.size.y + d + 0.5);
            let (cx, cy) = TileMap::cell_at(probe);
            if map.is_solid(cx, cy) {
                break;
            }
            d += 1.0;
        }
        d
    }

    /// One step; returns the head's new top-left and what happened.
    pub fn advance(&mut self, map: &TileMap) -> (Vec2, CrusherEvent) {
        let drop = match self.drop {
            Some(d) => d,
            None => {
                let d = self.measure(map);
                self.drop = Some(d);
                d
            }
        };
        self.prev_offset = self.offset;
        let mut event = CrusherEvent::None;
        self.state = match self.state {
            CrusherState::Up { ticks } if ticks > 1 => CrusherState::Up { ticks: ticks - 1 },
            CrusherState::Up { .. } => CrusherState::Slamming,
            CrusherState::Slamming => {
                self.offset += SLAM_SPEED * DT;
                if self.offset >= drop {
                    self.offset = drop;
                    event = CrusherEvent::Slammed;
                    CrusherState::Holding { ticks: HOLD_TICKS }
                } else {
                    CrusherState::Slamming
                }
            }
            CrusherState::Holding { ticks } if ticks > 1 => {
                CrusherState::Holding { ticks: ticks - 1 }
            }
            CrusherState::Holding { .. } => CrusherState::Rising,
            CrusherState::Rising => {
                self.offset -= RISE_SPEED * DT;
                if self.offset <= 0.0 {
                    self.offset = 0.0;
                    CrusherState::Up { ticks: UP_TICKS }
                } else {
                    CrusherState::Rising
                }
            }
        };
        (self.head(), event)
    }

    /// Top-left of the head now.
    pub fn head(&self) -> Vec2 {
        self.home + vec2(0.0, self.offset)
    }

    /// Slamming or resting on the floor, it crushes a ball it touches.
    pub fn crushes(&self, ball: &Ball) -> bool {
        if !matches!(
            self.state,
            CrusherState::Slamming | CrusherState::Holding { .. }
        ) {
            return false;
        }
        // A little forgiving at the sides.
        let min = self.head() + vec2(2.0, 0.0);
        let max = self.head() + self.size - vec2(2.0, 0.0);
        ball.pos.distance(ball.pos.clamp(min, max)) < ball.radius - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    /// Ceiling row 0, floor row 9: a 128 px drop for a 16 px head at row 1.
    fn room() -> TileMap {
        let mut rows = vec!["####".to_string(), "o...".to_string()];
        rows.extend((0..7).map(|_| "....".to_string()));
        rows.push("####".into());
        TileMap::from_ascii(&rows.join("\n")).unwrap()
    }

    fn crusher() -> Crusher {
        Crusher::new(vec2(16.0, 16.0), vec2(32.0, 16.0), 0)
    }

    #[test]
    fn cycle_slams_to_the_floor_holds_and_rises() {
        let map = room();
        let mut c = crusher();
        let mut slams = 0;
        let mut deepest = 0.0f32;
        let mut cycle = 0;
        for t in 0..400 {
            let (_, e) = c.advance(&map);
            if e == CrusherEvent::Slammed {
                slams += 1;
            }
            deepest = deepest.max(c.offset);
            if c.offset == 0.0
                && slams == 1
                && cycle == 0
                && matches!(c.state, CrusherState::Up { .. })
            {
                cycle = t;
            }
        }
        assert_eq!(c.drop, Some(112.0));
        assert_eq!(deepest, 112.0);
        assert!(slams >= 1);
        // Waits, slams in a fraction of a second, holds, rises in ~1.6 s.
        assert!((150..260).contains(&cycle), "{cycle}");
    }

    #[test]
    fn deadly_only_while_slamming_or_down() {
        let map = room();
        let mut c = crusher();
        let under = ball_at(32.0, 136.0);
        while !matches!(c.state, CrusherState::Slamming) {
            c.advance(&map);
            assert!(!c.crushes(&under), "harmless while up");
        }
        let mut hit = false;
        for _ in 0..60 {
            c.advance(&map);
            hit |= c.crushes(&under);
        }
        assert!(hit);
        while !matches!(c.state, CrusherState::Rising) {
            c.advance(&map);
        }
        c.advance(&map);
        assert!(!c.crushes(&under), "harmless while rising");
    }
}
