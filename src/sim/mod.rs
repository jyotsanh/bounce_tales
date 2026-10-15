//! Pure game simulation. Nothing in here may call macroquad drawing or
//! windowing functions, so it can run headless in tests.

/// Simulation rate. The sim always advances in steps of exactly `DT`.
pub const TICKS_PER_SECOND: u32 = 60;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Whole game state. Grows a lot from step 2 on.
#[derive(Debug, Clone, Default)]
pub struct Sim {
    /// Number of steps taken since start.
    pub tick: u64,
}

impl Sim {
    pub fn new() -> Sim {
        Sim::default()
    }

    pub fn step(&mut self) {
        self.tick += 1;
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

    #[test]
    fn sim_step_counts_ticks() {
        let mut sim = Sim::new();
        for _ in 0..10 {
            sim.step();
        }
        assert_eq!(sim.tick, 10);
    }
}
