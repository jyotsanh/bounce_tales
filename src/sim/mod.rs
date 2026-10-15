//! Pure game simulation. Nothing in here may call macroquad drawing or
//! windowing functions, so it can run headless in tests. Only macroquad's
//! math types (glam) are used.
//!
//! Determinism rule: `Sim::step` must depend only on the current state and
//! the given `InputFrame`. No wall-clock time, no global RNG, no HashMap
//! iteration order. Replays rely on this.

pub mod ball;
pub mod input;
pub mod rng;

use macroquad::math::vec2;

pub use ball::Ball;
pub use input::InputFrame;
pub use rng::Rng;

/// Simulation rate. The sim always advances in steps of exactly `DT`.
pub const TICKS_PER_SECOND: u32 = 60;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Width of the playfield until levels exist (step 8).
pub const WORLD_W: f32 = 240.0;
/// Temporary flat floor (y of the ground surface) until tile maps (step 4).
pub const GROUND_Y: f32 = 260.0;
/// Placeholder roll speed. Real rolling physics comes in step 3.
const PLACEHOLDER_SPEED: f32 = 90.0;

/// Whole game state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    /// Number of steps taken since start.
    pub tick: u64,
    pub ball: Ball,
    pub rng: Rng,
    /// Input used in the most recent step.
    pub input: InputFrame,
    /// Input used in the step before that (for "just pressed" checks).
    pub prev_input: InputFrame,
}

impl Default for Sim {
    fn default() -> Sim {
        Sim::new()
    }
}

impl Sim {
    pub fn new() -> Sim {
        let r = ball::BALL_RADIUS;
        Sim {
            tick: 0,
            ball: Ball::new(vec2(WORLD_W / 2.0, GROUND_Y - r)),
            rng: Rng::new(1),
            input: InputFrame::NONE,
            prev_input: InputFrame::NONE,
        }
    }

    /// Advances the world by exactly one `DT`.
    pub fn step(&mut self, input: InputFrame) {
        self.prev_input = self.input;
        self.input = input;
        self.tick += 1;

        // Placeholder movement: constant-speed roll, clamped to the screen.
        let b = &mut self.ball;
        b.prev_pos = b.pos;
        b.vel = vec2(input.axis() * PLACEHOLDER_SPEED, 0.0);
        b.pos += b.vel * DT;
        b.pos.x = b.pos.x.clamp(b.radius, WORLD_W - b.radius);
    }

    /// True on the first step the jump button is held.
    pub fn jump_pressed(&self) -> bool {
        self.input.jump && !self.prev_input.jump
    }
}

/// Fixed-timestep accumulator: turns variable frame times into a whole number
/// of sim steps, plus a 0..1 blend factor for interpolated rendering.
#[derive(Debug, Clone, Default)]
pub struct FixedStep {
    accumulator: f32,
}

impl FixedStep {
    /// Most steps we run per rendered frame; drops time after a long stall
    /// (window drag, debugger) instead of spiralling.
    pub const MAX_STEPS_PER_FRAME: u32 = 5;

    /// Adds `frame_time` seconds and returns how many sim steps to run now.
    pub fn advance(&mut self, frame_time: f32) -> u32 {
        self.accumulator += frame_time.max(0.0);
        let mut steps = 0;
        while self.accumulator >= DT && steps < Self::MAX_STEPS_PER_FRAME {
            self.accumulator -= DT;
            steps += 1;
        }
        if steps == Self::MAX_STEPS_PER_FRAME {
            self.accumulator = self.accumulator.min(DT);
        }
        steps
    }

    /// How far we are between the last step and the next one (0..1).
    pub fn alpha(&self) -> f32 {
        (self.accumulator / DT).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_second_is_sixty_steps() {
        let mut fs = FixedStep::default();
        let total: u32 = (0..60).map(|_| fs.advance(1.0 / 60.0 + 1e-6)).sum();
        assert_eq!(total, 60);
    }

    #[test]
    fn slow_frames_run_several_steps() {
        let mut fs = FixedStep::default();
        assert_eq!(fs.advance(DT * 3.5), 3);
        assert!((fs.alpha() - 0.5).abs() < 1e-3);
    }

    #[test]
    fn long_stall_is_capped() {
        let mut fs = FixedStep::default();
        assert_eq!(fs.advance(10.0), FixedStep::MAX_STEPS_PER_FRAME);
        assert!(fs.advance(0.0) <= 1);
    }

    fn press(code: &str) -> InputFrame {
        InputFrame::from_code(code).unwrap()
    }

    #[test]
    fn sim_step_counts_ticks() {
        let mut sim = Sim::new();
        for _ in 0..10 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.tick, 10);
    }

    #[test]
    fn jump_pressed_is_an_edge() {
        let mut sim = Sim::new();
        sim.step(press("J"));
        assert!(sim.jump_pressed());
        sim.step(press("J"));
        assert!(!sim.jump_pressed());
        sim.step(press("-"));
        sim.step(press("J"));
        assert!(sim.jump_pressed());
    }

    #[test]
    fn prev_pos_tracks_last_step() {
        let mut sim = Sim::new();
        sim.step(press("R"));
        let before = sim.ball.pos;
        sim.step(press("R"));
        assert_eq!(sim.ball.prev_pos, before);
        assert!(sim.ball.render_pos(0.5).x > before.x);
    }

    #[test]
    fn same_inputs_same_state() {
        let inputs: Vec<_> = ["R", "R", "RJ", "-", "L", "LJ", "-"]
            .iter()
            .cycle()
            .take(500)
            .map(|c| press(c))
            .collect();
        let run = || {
            let mut sim = Sim::new();
            for &i in &inputs {
                sim.step(i);
            }
            sim
        };
        assert_eq!(run(), run());
    }
}
