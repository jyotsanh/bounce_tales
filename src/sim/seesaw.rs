//! Seesaws: a plank turning on a pivot. The ball's weight tips it toward
//! the side the ball is on; the ball rides the plank's top (one-way, like a
//! moving platform) with its ground tilted to the plank, so it rolls
//! downhill along it. A plank swinging up pushes the ball with it.
//!
//! Angles are clockwise on screen (y down): positive = right end down.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::ball::Ball;

/// Plank thickness, px.
pub const THICKNESS: f32 = 8.0;
pub const DEFAULT_MAX_ANGLE_DEG: f32 = 25.0;
/// Plank mass relative to the ball (sets how fast it tips).
const PLANK_MASS: f32 = 10.0;
/// Angular damping per second (stops endless swinging).
const DAMPING: f32 = 1.5;
/// Share of a landing's momentum passed to the plank.
const IMPACT_GAIN: f32 = 1.0;
const GRAVITY: f32 = 900.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Seesaw {
    /// Centre of the plank (on the pivot).
    pub pivot: Vec2,
    /// Half the plank's length, px.
    pub half: f32,
    pub angle: f32,
    pub prev_angle: f32,
    /// Angular velocity, rad/s (clockwise positive).
    pub omega: f32,
    pub max_angle: f32,
    /// Where the post meets the ground (drawing only).
    pub base_y: f32,
    /// Constant spin (rad/s) for a motor-driven spinner; `None` for a
    /// seesaw moved by weight.
    pub motor: Option<f32>,
}

impl Seesaw {
    pub fn new(pivot: Vec2, half: f32, max_angle: f32, base_y: f32) -> Seesaw {
        Seesaw {
            pivot,
            half,
            angle: 0.0,
            prev_angle: 0.0,
            omega: 0.0,
            max_angle,
            base_y,
            motor: None,
        }
    }

    /// A plank turning steadily at `speed` rad/s (clockwise positive)
    /// around its centre. Whichever face points up is its top.
    pub fn spinner(centre: Vec2, half: f32, speed: f32) -> Seesaw {
        Seesaw {
            motor: Some(speed),
            omega: speed,
            ..Seesaw::new(centre, half, std::f32::consts::PI, centre.y)
        }
    }

    /// True when the plank's original top face points down (a spinner
    /// past vertical): the other face is the top then.
    fn flipped(&self) -> bool {
        self.angle.cos() < 0.0
    }

    /// Unit vector along the plank (toward its right end).
    pub fn dir(&self) -> Vec2 {
        vec2(self.angle.cos(), self.angle.sin())
    }

    /// Unit normal of the plank's top surface.
    pub fn normal(&self) -> Vec2 {
        let n = vec2(self.angle.sin(), -self.angle.cos());
        if self.flipped() { -n } else { n }
    }

    fn inertia(&self) -> f32 {
        PLANK_MASS * self.half * self.half / 3.0
    }

    /// Position along the plank (signed, from the pivot) and signed distance
    /// above its top surface for a point.
    pub fn locate(&self, p: Vec2) -> (f32, f32) {
        let rel = p - self.pivot;
        (
            rel.dot(self.dir()),
            rel.dot(self.normal()) - THICKNESS / 2.0,
        )
    }

    /// Speed of the top surface at `s` along its normal (px/s, up positive).
    pub fn surface_speed(&self, s: f32) -> f32 {
        let v = -s * self.omega;
        if self.flipped() { -v } else { v }
    }

    /// Turns the plank one step. `load` = (position along the plank, mass)
    /// of a ball resting on it.
    pub fn advance(&mut self, load: Option<(f32, f32)>) {
        self.prev_angle = self.angle;
        if let Some(speed) = self.motor {
            self.omega = speed;
            self.angle += speed * DT;
            // Keep it in -π..π; drawing interpolates across the wrap.
            if self.angle > std::f32::consts::PI {
                self.angle -= std::f32::consts::TAU;
                self.prev_angle -= std::f32::consts::TAU;
            } else if self.angle < -std::f32::consts::PI {
                self.angle += std::f32::consts::TAU;
                self.prev_angle += std::f32::consts::TAU;
            }
            return;
        }
        if let Some((s, mass)) = load {
            let torque = mass * GRAVITY * s * self.angle.cos();
            self.omega += torque / self.inertia() * DT;
        }
        self.omega *= 1.0 - DAMPING * DT;
        self.angle += self.omega * DT;
        if self.angle.abs() >= self.max_angle {
            self.angle = self.angle.clamp(-self.max_angle, self.max_angle);
            self.omega = 0.0;
        }
    }

    /// Spin from a ball of `mass` hitting the top at `s` with speed `speed`
    /// into the surface.
    pub fn kick(&mut self, s: f32, speed: f32, mass: f32) {
        if self.motor.is_some() {
            return;
        }
        self.omega += IMPACT_GAIN * mass * speed * s / self.inertia();
    }

    /// Is a ball at `s` along the plank over it (a little give at the
    /// ends)?
    fn over(&self, s: f32, ball: &Ball) -> bool {
        s.abs() <= self.half + ball.radius * 0.5
    }

    /// Keeps a riding ball on the plank. Returns its position along the
    /// plank, or `None` if it left (jumped, rolled off the end, or flew off
    /// a fast upswing that then stopped).
    pub fn keep_rider(&self, ball: &mut Ball) -> Option<f32> {
        let (s, dist) = self.locate(ball.pos);
        if ball.jumping
            || !self.over(s, ball)
            || dist > ball.radius + 6.0
            || dist < ball.radius - 4.0
        {
            return None;
        }
        self.stand(ball, s, dist);
        Some(s)
    }

    /// Lands a ball that came down onto the plank this step. Returns its
    /// position along the plank and the impact speed.
    pub fn try_land(&mut self, ball: &mut Ball) -> Option<(f32, f32)> {
        let (s, dist) = self.locate(ball.pos);
        let (_, prev_dist) = self.locate(ball.prev_pos);
        let n = self.normal();
        let closing = self.surface_speed(s) - ball.vel.dot(n);
        // Coming down onto it, or rolling on from a bank a little lower than
        // its top (the ball's centre still above the top line), but never
        // moving away from it.
        let step_up = ball.radius;
        if !self.over(s, ball)
            || dist >= ball.radius
            || dist < ball.radius - step_up
            || prev_dist < ball.radius - step_up
            || closing < -1.0
        {
            return None;
        }
        let closing = closing.max(0.0);
        self.kick(s, closing, ball.params.mass);
        if closing > ball.params.min_bounce_speed {
            ball.pos += n * (ball.radius - dist);
            ball.vel += n * closing * (1.0 + ball.params.restitution);
            ball.grounded = false;
            ball.jumping = false;
            ball.air_ticks = 0;
        } else {
            self.stand(ball, s, dist);
        }
        Some((s, closing))
    }

    /// Puts the ball on the surface, moving with it along the normal.
    fn stand(&self, ball: &mut Ball, s: f32, dist: f32) {
        let n = self.normal();
        ball.pos += n * (ball.radius - dist);
        let vn = ball.vel.dot(n);
        let surface = self.surface_speed(s);
        if vn < surface {
            ball.vel += n * (surface - vn);
        }
        ball.grounded = true;
        ball.jumping = false;
        ball.air_ticks = 0;
        ball.ground_normal = n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plank() -> Seesaw {
        Seesaw::new(vec2(100.0, 100.0), 48.0, 25f32.to_radians(), 116.0)
    }

    #[test]
    fn geometry() {
        let p = plank();
        assert_eq!(p.dir(), vec2(1.0, 0.0));
        assert_eq!(p.normal(), vec2(0.0, -1.0));
        // A ball resting on the level plank's top at the right end.
        let (s, d) = p.locate(vec2(140.0, 100.0 - 4.0 - 8.0));
        assert_eq!((s, d), (40.0, 8.0));
    }

    #[test]
    fn weight_tips_toward_the_ball_and_stops_at_the_limit() {
        let mut p = plank();
        for _ in 0..60 {
            p.advance(Some((40.0, 1.0)));
        }
        assert!((p.angle - p.max_angle).abs() < 1e-6, "{}", p.angle);
        assert_eq!(p.omega, 0.0);
        let mut q = plank();
        for _ in 0..60 {
            q.advance(Some((-40.0, 1.0)));
        }
        assert!((q.angle + q.max_angle).abs() < 1e-6);
    }

    #[test]
    fn tipping_takes_a_moment() {
        let mut p = plank();
        let mut steps = 0;
        while p.angle < p.max_angle {
            p.advance(Some((48.0, 1.0)));
            steps += 1;
        }
        // About half a second end to end at full lever.
        assert!((15..=45).contains(&steps), "{steps}");
    }

    #[test]
    fn heavier_balls_tip_faster() {
        let (mut a, mut b) = (plank(), plank());
        for _ in 0..10 {
            a.advance(Some((40.0, 1.0)));
            b.advance(Some((40.0, 3.0)));
        }
        assert!(b.angle > a.angle);
    }

    #[test]
    fn unloaded_plank_stays_put() {
        let mut p = plank();
        p.angle = 0.2;
        for _ in 0..60 {
            p.advance(None);
        }
        assert_eq!(p.angle, 0.2);
    }

    #[test]
    fn rolling_on_from_a_slightly_lower_bank_steps_up() {
        let mut p = plank();
        // Rolling flat with its centre 4 px under where it would rest.
        let mut b = crate::sim::ball::ball_at(60.0, 100.0 - 4.0 - 8.0 + 4.0);
        b.prev_pos = b.pos - vec2(2.0, 0.0);
        b.vel = vec2(120.0, 0.0);
        let landed = p.try_land(&mut b);
        assert!(landed.is_some());
        assert!(b.grounded);
        assert!((p.locate(b.pos).1 - 8.0).abs() < 1e-3, "on top");
    }

    #[test]
    fn spinner_turns_steadily_and_keeps_an_upward_top() {
        let mut p = Seesaw::spinner(vec2(100.0, 100.0), 32.0, std::f32::consts::FRAC_PI_2);
        for _ in 0..120 {
            p.advance(Some((30.0, 5.0))); // weight is ignored
        }
        // Half a turn in two seconds: upside down, but the top still faces up.
        assert!(
            (p.angle.abs() - std::f32::consts::PI).abs() < 0.05,
            "{}",
            p.angle
        );
        assert!(p.normal().y < 0.0);
    }

    #[test]
    fn surface_moves_with_the_spin() {
        let mut p = plank();
        p.omega = 1.0; // right end going down
        assert!(p.surface_speed(40.0) < 0.0);
        assert!(p.surface_speed(-40.0) > 0.0);
    }
}
