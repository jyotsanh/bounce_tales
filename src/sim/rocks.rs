//! Falling stalactites ("Rumbling Sounds"): a rock spike hangs from the
//! ceiling; when the ball passes below it rumbles and shakes, then drops,
//! crushing the ball if it hits; it shatters on the ground and grows back
//! after a while. Hanging and shaking ones are harmless.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;
use super::map::TileMap;

/// Size of a stalactite, px (hangs from the top of its cell).
pub const SIZE: Vec2 = Vec2::new(16.0, 24.0);
/// Steps of shaking before it drops.
pub const SHAKE_TICKS: u32 = 30;
/// Steps until a shattered one has grown back.
pub const REGROW_TICKS: u32 = 240;
/// It notices the ball this far either side of it, px...
pub const TRIGGER_SIDE: f32 = 20.0;
/// ...and at most this far below its tip, px.
pub const TRIGGER_BELOW: f32 = 200.0;
const GRAVITY: f32 = 900.0;
const MAX_FALL: f32 = 520.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RockState {
    Hanging,
    Shaking {
        ticks: u32,
    },
    Falling {
        vel: f32,
    },
    /// Shattered; growing back (steps left).
    Broken {
        ticks: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stalactite {
    /// Top-left where it hangs.
    pub home: Vec2,
    pub state: RockState,
    pub prev_pos: Vec2,
}

/// What a step did (the sim turns these into events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RockEvent {
    None,
    /// Started shaking (rumble).
    Rumble,
    /// Hit the ground and shattered.
    Shattered,
}

impl Stalactite {
    pub fn new(home: Vec2) -> Stalactite {
        Stalactite {
            home,
            state: RockState::Hanging,
            prev_pos: home,
        }
    }

    /// One step for the rock at top-left `pos`. Returns its new position and
    /// what happened.
    pub fn advance(&mut self, pos: Vec2, map: &TileMap, ball: Vec2) -> (Vec2, RockEvent) {
        self.prev_pos = pos;
        match &mut self.state {
            RockState::Hanging => {
                let below = ball.y - (pos.y + SIZE.y);
                let beside =
                    ball.x > pos.x - TRIGGER_SIDE && ball.x < pos.x + SIZE.x + TRIGGER_SIDE;
                if beside && below > 0.0 && below < TRIGGER_BELOW {
                    self.state = RockState::Shaking { ticks: SHAKE_TICKS };
                    return (pos, RockEvent::Rumble);
                }
                (pos, RockEvent::None)
            }
            RockState::Shaking { ticks } => {
                *ticks -= 1;
                if *ticks == 0 {
                    self.state = RockState::Falling { vel: 0.0 };
                }
                (pos, RockEvent::None)
            }
            RockState::Falling { vel } => {
                *vel = (*vel + GRAVITY * DT).min(MAX_FALL);
                let next = pos + vec2(0.0, *vel * DT);
                // The tip (bottom centre) meeting solid ground shatters it.
                let tip = next + vec2(SIZE.x / 2.0, SIZE.y);
                let (cx, cy) = TileMap::cell_at(tip);
                if map.is_solid(cx, cy) || tip.y > map.pixel_size().y + 64.0 {
                    self.state = RockState::Broken {
                        ticks: REGROW_TICKS,
                    };
                    let landed = vec2(next.x, (cy as f32) * super::map::TILE - SIZE.y);
                    return (landed, RockEvent::Shattered);
                }
                (next, RockEvent::None)
            }
            RockState::Broken { ticks } => {
                *ticks -= 1;
                if *ticks == 0 {
                    self.state = RockState::Hanging;
                    self.prev_pos = self.home;
                    return (self.home, RockEvent::None);
                }
                (pos, RockEvent::None)
            }
        }
    }

    /// A falling rock at `pos` hitting the ball crushes it.
    pub fn crushes(&self, pos: Vec2, ball: &Ball) -> bool {
        if !matches!(self.state, RockState::Falling { .. }) {
            return false;
        }
        // A slightly narrower box than the picture (it is pointed).
        let min = pos + vec2(3.0, 0.0);
        let max = pos + SIZE - vec2(3.0, 0.0);
        ball.pos.distance(ball.pos.clamp(min, max)) < ball.radius
    }

    /// Grown back, from 0 (just broken) to 1 (hanging); for drawing.
    pub fn grown(&self) -> f32 {
        match self.state {
            RockState::Broken { ticks } => {
                // Invisible at first, then grows in over the last second.
                let grow = 60.0;
                ((grow - ticks as f32) / grow).clamp(0.0, 1.0)
            }
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::ball_at;

    /// A 4x12 room: ceiling row 0, floor row 11.
    fn room() -> TileMap {
        let mut rows = vec!["####".to_string()];
        rows.extend((0..10).map(|_| "o...".to_string()).take(1));
        rows.extend((0..9).map(|_| "....".to_string()));
        rows.push("####".into());
        TileMap::from_ascii(&rows.join("\n")).unwrap()
    }

    fn run(
        r: &mut Stalactite,
        pos: &mut Vec2,
        map: &TileMap,
        ball: Vec2,
        n: u32,
    ) -> Vec<RockEvent> {
        let mut ev = Vec::new();
        for _ in 0..n {
            let (p, e) = r.advance(*pos, map, ball);
            *pos = p;
            ev.push(e);
        }
        ev
    }

    #[test]
    fn waits_until_the_ball_is_below() {
        let map = room();
        let home = vec2(16.0, 16.0);
        let mut r = Stalactite::new(home);
        let mut pos = home;
        // Ball off to the side: nothing happens.
        let ev = run(&mut r, &mut pos, &map, vec2(60.0, 150.0), 100);
        assert!(ev.iter().all(|e| *e == RockEvent::None));
        assert_eq!(pos, home);
        // Ball beneath: rumble, shake, then fall.
        let ev = run(&mut r, &mut pos, &map, vec2(24.0, 150.0), 1);
        assert_eq!(ev, vec![RockEvent::Rumble]);
        run(&mut r, &mut pos, &map, vec2(24.0, 150.0), SHAKE_TICKS);
        assert!(matches!(r.state, RockState::Falling { .. }));
    }

    #[test]
    fn falls_shatters_on_the_floor_and_grows_back() {
        let map = room();
        let home = vec2(16.0, 16.0);
        let mut r = Stalactite::new(home);
        let mut pos = home;
        let ev = run(&mut r, &mut pos, &map, vec2(24.0, 150.0), 120);
        assert!(ev.contains(&RockEvent::Shattered));
        assert!(matches!(r.state, RockState::Broken { .. }));
        // Resting on the floor (row 11 top = 176).
        assert_eq!(pos.y + SIZE.y, 176.0);
        assert!(r.grown() < 1.0);
        run(&mut r, &mut pos, &map, vec2(60.0, 0.0), REGROW_TICKS);
        assert_eq!(r.state, RockState::Hanging);
        assert_eq!(pos, home);
    }

    #[test]
    fn only_a_falling_rock_crushes() {
        let mut r = Stalactite::new(vec2(0.0, 0.0));
        let b = ball_at(8.0, 30.0);
        assert!(!r.crushes(vec2(0.0, 0.0), &b), "hanging is harmless");
        r.state = RockState::Falling { vel: 200.0 };
        assert!(r.crushes(vec2(0.0, 0.0), &b));
        assert!(!r.crushes(vec2(0.0, -40.0), &b));
    }
}
