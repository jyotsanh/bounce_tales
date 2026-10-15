//! Pure game simulation. Nothing in here may call macroquad drawing or
//! windowing functions, so it can run headless in tests. Only macroquad's
//! math types (glam) are used.
//!
//! Determinism rule: `Sim::step` must depend only on the current state and
//! the given `InputFrame`. No wall-clock time, no global RNG, no HashMap
//! iteration order. Replays rely on this.

pub mod ball;
pub mod camera;
pub mod entities;
pub mod input;
pub mod map;
pub mod physics;
pub mod rng;

pub use ball::Ball;
pub use camera::Camera;
pub use entities::{Entity, EntityKind, Event};
pub use input::InputFrame;
pub use map::TileMap;

use crate::level::Level;
pub use rng::Rng;

/// Simulation rate. The sim always advances in steps of exactly `DT`.
pub const TICKS_PER_SECOND: u32 = 60;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Level loaded when none is given.
pub const DEFAULT_LEVEL: &str = "big";

/// How far below the map the ball may fall before it is put back at the
/// start. Temporary until deaths and lives exist (steps 11-12).
const FALL_OUT_MARGIN: f32 = 4.0 * map::TILE;

/// Whole game state.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    /// Number of steps taken since start.
    pub tick: u64,
    /// Collision map plus drawable layers (static during play).
    pub level: Level,
    pub ball: Ball,
    pub camera: Camera,
    /// Interactive objects built from the level's object specs.
    pub entities: Vec<Entity>,
    /// What happened during the latest step (cleared at the start of each).
    pub events: Vec<Event>,
    /// The ball has reached the exit (stays true).
    pub exit_reached: bool,
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
    /// The default level.
    pub fn new() -> Sim {
        Sim::for_level(DEFAULT_LEVEL).expect("default level loads")
    }

    /// Starts the named level (see `level::level_names`).
    pub fn for_level(name: &str) -> Result<Sim, String> {
        Level::load(name).map(Sim::with_level)
    }

    pub fn with_level(level: Level) -> Sim {
        let ball = Ball::new(level.map.spawn);
        let entities =
            entities::build_all(&level.objects).expect("objects are validated when loading");
        Sim {
            tick: 0,
            entities,
            events: Vec::new(),
            exit_reached: false,
            camera: Camera::snapped_to(&ball, &level.map),
            ball,
            level,
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
        self.events.clear();

        let jump_pressed = self.jump_pressed();
        self.ball.update(input, jump_pressed);
        physics::move_and_collide(&mut self.ball, &self.level.map);
        self.ball.update_spin();

        if self.ball.pos.y > self.level.map.pixel_size().y + FALL_OUT_MARGIN {
            self.ball = Ball::new(self.level.map.spawn);
            self.camera = Camera::snapped_to(&self.ball, &self.level.map);
        } else {
            self.camera.update(&self.ball, &self.level.map);
        }

        for e in &mut self.entities {
            e.update(&self.ball, &mut self.events);
        }
        if self.events.contains(&Event::ExitReached) {
            self.exit_reached = true;
        }
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
