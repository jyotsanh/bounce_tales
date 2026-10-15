//! The player ball: movement rules (rolling, jumping, bouncing).
//! Collision with the world lives in `physics.rs`.

use macroquad::math::{Vec2, vec2};

use super::DT;
use super::input::InputFrame;

/// Radius of the normal ball in pixels (world units are pixels, y points down).
pub const BALL_RADIUS: f32 = 8.0;

/// Tunable movement numbers. Each ball form (normal, stone, balloon… in
/// step 22) gets its own set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallParams {
    pub radius: f32,
    /// Downward acceleration, px/s².
    pub gravity: f32,
    /// Fastest falling speed, px/s.
    pub max_fall: f32,
    /// Acceleration from left/right while touching the ground, px/s².
    pub ground_accel: f32,
    /// Acceleration from left/right in the air, px/s².
    pub air_accel: f32,
    /// Slow-down on the ground with no input, px/s².
    pub ground_friction: f32,
    /// Slow-down in the air with no input, px/s².
    pub air_drag: f32,
    /// Top rolling speed from input, px/s.
    pub max_speed: f32,
    /// How high a jump lifts the ball's centre, px.
    pub jump_height: f32,
    /// Fraction of landing speed kept as an upward bounce.
    pub restitution: f32,
    /// Landings slower than this stop dead instead of bouncing, px/s.
    pub min_bounce_speed: f32,
    /// Fraction of horizontal speed kept when hitting a wall.
    pub wall_restitution: f32,
    /// Steps after leaving the ground during which a jump still works.
    pub coyote_ticks: u32,
    /// Steps a jump press is remembered while in the air.
    pub jump_buffer_ticks: u32,
}

impl BallParams {
    /// The red rubber ball. Tuned to feel like Bounce Tales: a quick roll,
    /// a jump of a bit over three ball heights, a small bounce on landing.
    pub const NORMAL: BallParams = BallParams {
        radius: BALL_RADIUS,
        gravity: 900.0,
        max_fall: 420.0,
        ground_accel: 600.0,
        air_accel: 420.0,
        ground_friction: 450.0,
        air_drag: 60.0,
        max_speed: 120.0,
        jump_height: 52.0,
        restitution: 0.35,
        min_bounce_speed: 60.0,
        wall_restitution: 0.3,
        coyote_ticks: 5,
        jump_buffer_ticks: 8,
    };

    /// Take-off speed that reaches `jump_height` under `gravity` with the
    /// sim's semi-implicit Euler integration (which loses about half a step
    /// of rise compared with the continuous formula).
    pub fn jump_speed(&self) -> f32 {
        (2.0 * self.gravity * self.jump_height).sqrt() + 0.5 * self.gravity * DT
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ball {
    pub pos: Vec2,
    /// Position at the start of the last step, for render interpolation.
    pub prev_pos: Vec2,
    /// Pixels per second.
    pub vel: Vec2,
    pub radius: f32,
    pub params: BallParams,
    /// Resting on the ground (set by collision each step).
    pub grounded: bool,
    /// Steps since the ball last touched the ground.
    pub air_ticks: u32,
    /// Steps left in which a jump press is still remembered.
    pub jump_buffer: u32,
    /// True between taking off and touching the ground again; stops
    /// coyote time from giving a second jump.
    pub jumping: bool,
    /// Rotation for drawing, radians (clockwise = rolling right).
    pub angle: f32,
    /// Rotation speed, radians per second.
    pub spin: f32,
}

impl Ball {
    pub fn new(pos: Vec2) -> Ball {
        let params = BallParams::NORMAL;
        Ball {
            pos,
            prev_pos: pos,
            vel: Vec2::ZERO,
            radius: params.radius,
            params,
            grounded: false,
            air_ticks: 0,
            jump_buffer: 0,
            jumping: false,
            angle: 0.0,
            spin: 0.0,
        }
    }

    /// Position blended between the last two steps (`alpha` in 0..1).
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }

    /// Applies input, gravity and jumping for one step and moves the ball.
    /// Collision is resolved afterwards by the physics module.
    pub fn update(&mut self, input: InputFrame, jump_pressed: bool) {
        let p = self.params;
        self.prev_pos = self.pos;

        // Horizontal: accelerate toward input, faster when reversing,
        // otherwise slow down by friction/drag.
        let axis = input.axis();
        let (accel, friction) = if self.grounded {
            (p.ground_accel, p.ground_friction)
        } else {
            (p.air_accel, p.air_drag)
        };
        if axis != 0.0 {
            let reversing = self.vel.x * axis < 0.0;
            let a = if reversing { accel + friction } else { accel };
            let before = self.vel.x * axis;
            self.vel.x += axis * a * DT;
            // Input alone cannot push past top speed. Faster motion (from
            // springs or slopes later) is not increased and bleeds off by
            // friction.
            let cap = p.max_speed.max(before - friction * DT);
            if self.vel.x * axis > cap {
                self.vel.x = cap * axis;
            }
        } else {
            self.vel.x = approach(self.vel.x, 0.0, friction * DT);
        }

        // Jump: a press is buffered for a few steps; holding jump keeps
        // re-jumping on every landing, like the original.
        if jump_pressed {
            self.jump_buffer = p.jump_buffer_ticks;
        }
        let can_jump = self.grounded || (!self.jumping && self.air_ticks <= p.coyote_ticks);
        if can_jump && (self.jump_buffer > 0 || input.jump) {
            self.vel.y = -p.jump_speed();
            self.grounded = false;
            self.jumping = true;
            self.jump_buffer = 0;
        } else {
            self.jump_buffer = self.jump_buffer.saturating_sub(1);
        }

        if !self.grounded {
            self.vel.y = (self.vel.y + p.gravity * DT).min(p.max_fall);
        }
        self.pos += self.vel * DT;

        // Spin follows the ground speed while rolling; in the air it keeps
        // whatever it had.
        if self.grounded {
            self.spin = self.vel.x / self.radius;
        }
        self.angle = (self.angle + self.spin * DT).rem_euclid(std::f32::consts::TAU);
    }
}

/// Moves `value` toward `target` by at most `step`.
pub fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

/// Convenience for tests and tools.
pub fn ball_at(x: f32, y: f32) -> Ball {
    Ball::new(vec2(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_clamps() {
        assert_eq!(approach(5.0, 0.0, 2.0), 3.0);
        assert_eq!(approach(1.0, 0.0, 2.0), 0.0);
        assert_eq!(approach(-1.0, 0.0, 2.0), 0.0);
    }

    #[test]
    fn jump_speed_is_reasonable() {
        let v = BallParams::NORMAL.jump_speed();
        assert!((300.0..320.0).contains(&v), "{v}");
    }
}
