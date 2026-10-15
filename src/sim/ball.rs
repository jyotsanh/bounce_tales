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
    /// Share of the downhill pull felt while pushing uphill (0..1).
    pub uphill_gravity: f32,
    /// Fastest the ball can roll along the ground (downhill), px/s.
    pub max_roll_speed: f32,
    /// Upward push when fully under water, as a multiple of gravity. Above
    /// 1 floats (at 1/buoyancy submerged), below 1 sinks.
    pub buoyancy: f32,
    /// Water drag per second when fully under water (horizontal, vertical).
    pub water_drag: (f32, f32),
    /// Top speed in water as a share of `max_speed`.
    pub water_speed: f32,
    /// Jump strength from the water surface as a share of a normal jump.
    pub water_jump: f32,
    /// Hitting cracked blocks hard enough breaks them (heavy forms only).
    pub smashes: bool,
    /// Weight on seesaws (1 = the red ball).
    pub mass: f32,
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
        uphill_gravity: 0.5,
        max_roll_speed: 260.0,
        buoyancy: 1.6,
        water_drag: (2.5, 6.0),
        water_speed: 0.75,
        water_jump: 0.9,
        smashes: false,
        mass: 1.0,
    };

    /// Take-off speed that reaches `jump_height` under `gravity` with the
    /// sim's semi-implicit Euler integration (which loses about half a step
    /// of rise compared with the continuous formula).
    pub fn jump_speed(&self) -> f32 {
        self.launch_speed(self.jump_height)
    }

    /// Take-off speed that lifts the ball's centre by `height` px (with the
    /// same integration correction as a jump).
    pub fn launch_speed(&self, height: f32) -> f32 {
        (2.0 * self.gravity * height).sqrt() + 0.5 * self.gravity * DT
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
    /// Normal of the floor last touched (points away from it). (0, -1) on
    /// flat ground.
    pub ground_normal: Vec2,
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
    /// How much of the ball is under water (0..1), measured after the last
    /// move (`water::submersion`).
    pub water: f32,
    /// Acceleration from fans this step, px/s² (set by the sim).
    pub wind: Vec2,
    /// Visual squash (> 0 flattened, < 0 stretched tall), springs back to 0.
    /// Drawing only; never affects collision.
    pub squash: f32,
    /// Rate of change of `squash`, per second.
    pub squash_vel: f32,
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
            ground_normal: vec2(0.0, -1.0),
            air_ticks: 0,
            jump_buffer: 0,
            jumping: false,
            angle: 0.0,
            spin: 0.0,
            water: 0.0,
            wind: Vec2::ZERO,
            squash: 0.0,
            squash_vel: 0.0,
        }
    }

    /// Position blended between the last two steps (`alpha` in 0..1).
    pub fn render_pos(&self, alpha: f32) -> Vec2 {
        self.prev_pos.lerp(self.pos, alpha)
    }

    /// Applies input, gravity and jumping for one step (velocity only).
    /// `physics::move_and_collide` then moves the ball, and
    /// [`Ball::update_spin`] / [`Ball::update_squash`] run last. Returns true
    /// if the ball jumped this step.
    pub fn update(&mut self, input: InputFrame, jump_pressed: bool) -> bool {
        let p = self.params;
        self.prev_pos = self.pos;

        let axis = input.axis();
        let wet = self.water;
        if self.grounded {
            self.roll(axis);
        } else {
            let max = if wet > 0.0 {
                p.max_speed * p.water_speed
            } else {
                p.max_speed
            };
            self.vel.x = drive(self.vel.x, axis, p.air_accel, p.air_drag, max);
        }

        // Jump: a press is buffered for a few steps; holding jump keeps
        // re-jumping on every landing, like the original. Jumps go straight
        // up, even on slopes.
        if jump_pressed {
            self.jump_buffer = p.jump_buffer_ticks;
        }
        // Floating at the surface (not deep under, not already rising fast)
        // also allows a jump.
        let at_surface = wet > 0.15 && wet < 0.9 && self.vel.y > -100.0;
        let can_jump =
            self.grounded || at_surface || (!self.jumping && self.air_ticks <= p.coyote_ticks);
        let jumped = can_jump && (self.jump_buffer > 0 || input.jump);
        if jumped {
            let strength = if wet > 0.0 && !self.grounded {
                p.water_jump
            } else {
                1.0
            };
            self.vel.y = -p.jump_speed() * strength;
            self.grounded = false;
            self.jumping = true;
            self.jump_buffer = 0;
        } else {
            self.jump_buffer = self.jump_buffer.saturating_sub(1);
        }

        if !self.grounded {
            self.vel.y = (self.vel.y + p.gravity * DT).min(p.max_fall);
        }
        if self.wind != Vec2::ZERO {
            // Fans push even a grounded ball (an up-draft lifts it off).
            self.vel += self.wind * DT;
            // Air resistance along the stream damps bobbing into a hover.
            let along = self.wind.normalize();
            self.vel -= along * self.vel.dot(along) * WIND_DRAG * DT;
            if self.wind.y < 0.0 {
                self.vel.y = self.vel.y.max(-MAX_WIND_RISE);
            }
        }
        if wet > 0.0 {
            // Buoyancy lifts even a grounded ball (it then leaves the floor).
            self.vel.y -= p.gravity * p.buoyancy * wet * DT;
            self.vel.x *= 1.0 - p.water_drag.0 * wet * DT;
            self.vel.y *= 1.0 - p.water_drag.1 * wet * DT;
        }
        jumped
    }

    /// Squash-and-stretch spring. A landing at `impact` px/s flattens the
    /// ball (harder = flatter, up to `SQUASH_MAX`); a jump stretches it. The
    /// spring is under-damped, so it wobbles past round once or twice.
    pub fn update_squash(&mut self, impact: f32, jumped: bool) {
        if impact > SQUASH_MIN_IMPACT {
            self.squash = self.squash.max((impact / SQUASH_PER_SPEED).min(SQUASH_MAX));
            self.squash_vel = 0.0;
        }
        if jumped {
            self.squash = -SQUASH_JUMP;
            self.squash_vel = 0.0;
        }
        let accel = -SQUASH_STIFFNESS * self.squash - SQUASH_DAMPING * self.squash_vel;
        self.squash_vel += accel * DT;
        self.squash = (self.squash + self.squash_vel * DT).clamp(-SQUASH_MAX, SQUASH_MAX);
        if self.squash.abs() < 0.002 && self.squash_vel.abs() < 0.05 {
            self.squash = 0.0;
            self.squash_vel = 0.0;
        }
    }

    /// Ground movement. Speed is tracked along the surface (the tangent of
    /// `ground_normal`), so the same rules work on flat ground and slopes:
    /// - gravity's pull along the slope rolls the ball downhill;
    /// - pushing uphill only fights part of that pull, so 45° is climbable;
    /// - friction fades out on steep slopes, so the ball never sticks there;
    /// - a small push into the surface keeps contact every step.
    fn roll(&mut self, axis: f32) {
        let p = self.params;
        let n = self.ground_normal;
        let t = vec2(-n.y, n.x); // along the surface, pointing right
        let mut s = self.vel.dot(t);

        // 1 on flat ground, falling to 0 at 45°.
        let grip = ((-n.y - COS_45) / (1.0 - COS_45)).clamp(0.0, 1.0);
        let mut pull = p.gravity * t.y; // gravity along the surface
        if axis * pull < 0.0 {
            pull *= p.uphill_gravity;
        }
        s = drive(
            s,
            axis,
            p.ground_accel,
            p.ground_friction * grip,
            p.max_speed,
        );
        s = (s + pull * DT).clamp(-p.max_roll_speed, p.max_roll_speed);

        let press = p.gravity * -n.y * DT;
        self.vel = t * s - n * press;
    }

    /// Updates rotation after collision. Spin follows speed along the
    /// ground while rolling; in the air it keeps whatever it had.
    pub fn update_spin(&mut self) {
        if self.grounded {
            let n = self.ground_normal;
            self.spin = self.vel.dot(vec2(-n.y, n.x)) / self.radius;
        }
        self.angle = (self.angle + self.spin * DT).rem_euclid(std::f32::consts::TAU);
    }
}

const COS_45: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Fastest an up-draft lifts the ball, px/s (so it hovers, not rockets).
pub const MAX_WIND_RISE: f32 = 220.0;
/// Drag along a fan's stream, per second (damps the hover bob).
const WIND_DRAG: f32 = 4.0;

/// Landings slower than this don't squash the ball, px/s.
const SQUASH_MIN_IMPACT: f32 = 60.0;
/// Impact speed per unit of squash (420 px/s → ~0.35).
const SQUASH_PER_SPEED: f32 = 1200.0;
/// Most squash or stretch (fraction of the ball's size).
pub const SQUASH_MAX: f32 = 0.35;
/// Stretch at take-off.
const SQUASH_JUMP: f32 = 0.15;
/// Spring stiffness (ω² for ω ≈ 21 rad/s) and damping (ζ ≈ 0.3).
const SQUASH_STIFFNESS: f32 = 450.0;
const SQUASH_DAMPING: f32 = 13.0;

/// One axis of input-driven speed: accelerate toward `axis` (faster when
/// reversing), otherwise slow down by `friction`. Input alone cannot push
/// past `max`; faster motion (slopes, springs) is not increased and bleeds
/// off by friction instead of snapping back.
pub fn drive(v: f32, axis: f32, accel: f32, friction: f32, max: f32) -> f32 {
    if axis == 0.0 {
        return approach(v, 0.0, friction * DT);
    }
    let reversing = v * axis < 0.0;
    let a = if reversing { accel + friction } else { accel };
    let before = v * axis;
    let after = v + axis * a * DT;
    let cap = max.max(before - friction * DT);
    if after * axis > cap {
        cap * axis
    } else {
        after
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
