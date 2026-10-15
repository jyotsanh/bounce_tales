//! Collision between the ball and the world.
//!
//! For now the world is a flat floor with walls at both screen edges.
//! Tile maps replace this in step 4.

use super::ball::Ball;

/// What the ball touched during one step.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Contacts {
    pub floor: bool,
    pub wall: bool,
    /// Speed into the floor at impact, px/s (0 if no floor hit).
    pub impact_speed: f32,
}

/// Flat world: floor at `floor_y`, walls at x = 0 and x = `width`.
pub fn collide_flat(ball: &mut Ball, floor_y: f32, width: f32) -> Contacts {
    let p = ball.params;
    let r = ball.radius;
    let mut c = Contacts::default();

    if ball.pos.x < r || ball.pos.x > width - r {
        ball.pos.x = ball.pos.x.clamp(r, width - r);
        let into_wall =
            (ball.pos.x <= r && ball.vel.x < 0.0) || (ball.pos.x >= width - r && ball.vel.x > 0.0);
        if into_wall {
            ball.vel.x = -ball.vel.x * p.wall_restitution;
            c.wall = true;
        }
    }

    if ball.pos.y + r >= floor_y && ball.vel.y >= 0.0 {
        ball.pos.y = floor_y - r;
        c.floor = true;
        c.impact_speed = ball.vel.y;
    }

    land(ball, c);
    c
}

/// Shared landing rules: bounce if hitting hard, otherwise come to rest.
/// Also updates the ground/air counters.
fn land(ball: &mut Ball, c: Contacts) {
    let p = ball.params;
    if c.floor {
        ball.jumping = false;
        if c.impact_speed > p.min_bounce_speed {
            ball.vel.y = -c.impact_speed * p.restitution;
            ball.grounded = false;
            // Touched the ground this step, so coyote/buffered jumps count.
            ball.air_ticks = 0;
        } else {
            ball.vel.y = 0.0;
            ball.grounded = true;
            ball.air_ticks = 0;
        }
    } else {
        ball.grounded = false;
        ball.air_ticks = ball.air_ticks.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::ball::{BallParams, ball_at};
    use crate::sim::{DT, InputFrame};

    const FLOOR: f32 = 300.0;
    const WIDE: f32 = 100_000.0;
    /// Start position far from both walls of the wide test world.
    const MID: f32 = WIDE / 2.0;

    /// Ball in a flat world, stepped like the real sim does it.
    struct World {
        ball: Ball,
        prev: InputFrame,
        width: f32,
    }

    impl World {
        fn resting(x: f32) -> World {
            let mut w = World {
                ball: ball_at(x, FLOOR - BallParams::NORMAL.radius),
                prev: InputFrame::NONE,
                width: WIDE,
            };
            w.run("-", 5);
            assert!(w.ball.grounded);
            w
        }

        fn step(&mut self, code: &str) -> Contacts {
            let input = InputFrame::from_code(code).unwrap();
            let pressed = input.jump && !self.prev.jump;
            self.prev = input;
            self.ball.update(input, pressed);
            collide_flat(&mut self.ball, FLOOR, self.width)
        }

        fn run(&mut self, code: &str, steps: usize) {
            for _ in 0..steps {
                self.step(code);
            }
        }

        /// Highest the ball's bottom gets above the floor during `steps`.
        fn max_rise(&mut self, code: &str, steps: usize) -> f32 {
            let rest_y = FLOOR - self.ball.radius;
            let mut best: f32 = 0.0;
            for _ in 0..steps {
                self.step(code);
                best = best.max(rest_y - self.ball.pos.y);
            }
            best
        }
    }

    #[test]
    fn falling_ball_bounces_then_settles() {
        let mut w = World::resting(MID);
        w.ball.pos.y = FLOOR - 100.0;
        w.ball.grounded = false;
        let mut bounces = 0;
        for _ in 0..240 {
            let c = w.step("-");
            if c.floor && w.ball.vel.y < 0.0 {
                bounces += 1;
            }
        }
        assert!(bounces >= 1, "should bounce at least once");
        assert!(w.ball.grounded);
        assert_eq!(w.ball.vel.y, 0.0);
        assert_eq!(w.ball.pos.y, FLOOR - w.ball.radius);
    }

    #[test]
    fn jump_reaches_jump_height() {
        let p = BallParams::NORMAL;
        let mut w = World::resting(MID);
        w.step("J");
        let rise = w.max_rise("-", 60);
        assert!(
            (rise - p.jump_height).abs() < 1.0,
            "rise {rise} vs {}",
            p.jump_height
        );
    }

    #[test]
    fn jump_air_time_is_snappy() {
        let mut w = World::resting(MID);
        w.step("J");
        let mut air = 1;
        while !w.step("-").floor {
            air += 1;
            assert!(air < 120, "never landed");
        }
        // About 0.7 s, like the original's quick hop.
        assert!((36..=48).contains(&air), "air time {air} steps");
    }

    #[test]
    fn holding_jump_keeps_jumping() {
        let mut w = World::resting(MID);
        let mut takeoffs = 0;
        let mut was_rising = false;
        for _ in 0..180 {
            w.step("J");
            let rising = w.ball.vel.y < -200.0;
            if rising && !was_rising {
                takeoffs += 1;
            }
            was_rising = rising;
        }
        assert!(takeoffs >= 3, "only {takeoffs} jumps in 3 s");
    }

    #[test]
    fn buffered_jump_just_before_landing_works() {
        let mut w = World::resting(MID);
        w.step("J");
        // Wait until falling and close to the floor, then tap jump in the air.
        while !(w.ball.vel.y > 0.0 && FLOOR - w.ball.pos.y < 30.0) {
            w.step("-");
        }
        w.step("J");
        w.run("-", 20);
        let rise = w.max_rise("-", 30);
        assert!(rise > 30.0, "buffered jump did not happen (rise {rise})");
    }

    #[test]
    fn early_press_is_forgotten() {
        let mut w = World::resting(MID);
        w.step("J");
        w.run("-", 5);
        w.step("J"); // pressed while still rising: far too early
        w.run("-", 80);
        assert!(w.ball.grounded, "should have landed and stayed down");
    }

    #[test]
    fn no_double_jump_in_air() {
        let mut w = World::resting(MID);
        w.step("J");
        w.run("-", 10);
        let vy = w.ball.vel.y;
        w.step("J");
        assert!(w.ball.vel.y > vy, "second press in air changed velocity");
    }

    #[test]
    fn rolling_is_capped_at_max_speed() {
        let p = BallParams::NORMAL;
        let mut w = World::resting(MID);
        w.run("R", 60);
        assert!(
            (w.ball.vel.x - p.max_speed).abs() < 1e-3,
            "{}",
            w.ball.vel.x
        );
        w.run("L", 120);
        assert!(
            (w.ball.vel.x + p.max_speed).abs() < 1e-3,
            "{}",
            w.ball.vel.x
        );
    }

    #[test]
    fn reaches_top_speed_quickly() {
        let mut w = World::resting(MID);
        w.run("R", 15); // a quarter second
        assert!(w.ball.vel.x > 100.0, "{}", w.ball.vel.x);
    }

    #[test]
    fn friction_stops_the_ball() {
        let mut w = World::resting(MID);
        w.run("R", 60);
        w.run("-", 30); // half a second
        assert_eq!(w.ball.vel.x, 0.0);
    }

    #[test]
    fn extra_speed_bleeds_off_instead_of_snapping() {
        let mut w = World::resting(MID);
        w.ball.vel.x = 300.0; // e.g. from a spring later on
        w.step("R");
        assert!(w.ball.vel.x > 290.0, "{}", w.ball.vel.x);
        w.run("R", 120);
        assert!((w.ball.vel.x - BallParams::NORMAL.max_speed).abs() < 1e-3);
    }

    #[test]
    fn air_control_is_weaker() {
        let mut ground = World::resting(MID);
        ground.run("R", 6);
        let mut air = World::resting(MID);
        air.step("J");
        air.run("R", 6);
        assert!(air.ball.vel.x < ground.ball.vel.x);
        assert!(air.ball.vel.x > 0.0);
    }

    #[test]
    fn walls_stop_and_bounce_back() {
        let mut w = World::resting(100.0);
        w.width = 200.0;
        let mut hit = false;
        for _ in 0..120 {
            hit |= w.step("R").wall;
        }
        assert!(hit);
        assert!(w.ball.pos.x <= 200.0 - w.ball.radius);
        w.ball.vel.x = 120.0;
        w.step("-");
        assert!(w.ball.vel.x < 0.0, "should rebound off the wall");
    }

    #[test]
    fn coyote_jump_after_leaving_ground() {
        // Fake walking off a ledge: ball in the air, not from a jump.
        let mut w = World::resting(MID);
        w.ball.pos.y -= 20.0;
        w.ball.grounded = false;
        w.ball.air_ticks = 2;
        w.ball.update(InputFrame::from_code("J").unwrap(), true);
        assert!(w.ball.vel.y < -200.0, "coyote jump expected");
    }

    #[test]
    fn spin_matches_rolling() {
        let mut w = World::resting(MID);
        w.run("R", 60);
        let expected = w.ball.vel.x / w.ball.radius;
        assert!((w.ball.spin - expected).abs() < 1e-4);
        let a0 = w.ball.angle;
        w.step("R");
        let da = (w.ball.angle - a0).rem_euclid(std::f32::consts::TAU);
        assert!((da - expected * DT).abs() < 1e-4);
    }
}
