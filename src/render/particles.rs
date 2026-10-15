//! Little visual effects: dust when the ball lands, jumps or rolls fast,
//! confetti at the exit. Purely cosmetic, so they live in the renderer, but
//! they advance once per sim step with their own seeded RNG so scripted
//! runs (and screenshots) are repeatable.

use macroquad::prelude::*;

use bounce_tales::sim::hazards::Cause;
use bounce_tales::sim::{DT, Event, Rng, Sim};

struct Particle {
    pos: Vec2,
    vel: Vec2,
    /// Seconds left, and the starting value.
    life: f32,
    max_life: f32,
    color: Color,
    size: f32,
    /// Downward acceleration, px/s².
    gravity: f32,
}

pub struct Particles {
    list: Vec<Particle>,
    rng: Rng,
}

const DUST: Color = Color::new(0.93, 0.89, 0.78, 0.85);
const CONFETTI: [Color; 5] = [
    Color::new(0.89, 0.14, 0.11, 1.0),
    Color::new(1.0, 0.82, 0.24, 1.0),
    Color::new(0.96, 0.96, 0.96, 1.0),
    Color::new(0.34, 0.69, 0.19, 1.0),
    Color::new(0.30, 0.55, 0.95, 1.0),
];
/// Colours of the ball's fragments when it pops.
const BALL_BITS: [Color; 3] = [
    Color::new(0.89, 0.14, 0.11, 1.0),
    Color::new(0.66, 0.08, 0.06, 1.0),
    Color::new(1.0, 0.85, 0.82, 1.0),
];
/// Ground speed above which a rolling ball kicks up dust, px/s.
const ROLL_DUST_SPEED: f32 = 110.0;
/// Safety cap so effects can never pile up.
const MAX_PARTICLES: usize = 400;

impl Particles {
    pub fn new() -> Particles {
        Particles {
            list: Vec::new(),
            rng: Rng::new(0xD057),
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Spawns effects for what happened in the latest sim step, then moves
    /// every particle by one step.
    pub fn on_step(&mut self, sim: &Sim) {
        for e in &sim.events {
            match *e {
                Event::Landed { pos, speed } => {
                    let n = (speed / 60.0).clamp(3.0, 10.0) as usize;
                    self.puffs(pos, n, 70.0);
                }
                Event::Jumped { pos } => self.puffs(pos, 4, 40.0),
                Event::ExitReached => self.confetti(sim.ball.pos),
                // A pit death happens off screen: nothing to show.
                Event::Died {
                    cause: Cause::Pit, ..
                } => {}
                Event::Died { pos, .. } => self.burst(pos),
                Event::Respawned { pos } => self.sparkle(pos),
            }
        }
        let b = &sim.ball;
        if sim.dying == 0
            && b.grounded
            && b.vel.length() > ROLL_DUST_SPEED
            && sim.tick.is_multiple_of(6)
        {
            let behind = vec2(-b.vel.x.signum() * 4.0, b.radius);
            self.puffs(b.pos + behind, 1, 20.0);
        }

        for p in &mut self.list {
            p.vel.y += p.gravity * DT;
            p.pos += p.vel * DT;
            p.life -= DT;
        }
        self.list.retain(|p| p.life > 0.0);
        if self.list.len() > MAX_PARTICLES {
            let extra = self.list.len() - MAX_PARTICLES;
            self.list.drain(..extra);
        }
    }

    /// Dust puffs that fly out sideways from `at` (a contact point).
    fn puffs(&mut self, at: Vec2, n: usize, spread: f32) {
        for i in 0..n {
            // Alternate sides so a landing kicks dust both ways.
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let life = self.rng.range(0.25, 0.45);
            self.list.push(Particle {
                pos: at + vec2(side * self.rng.range(2.0, 7.0), -1.0),
                vel: vec2(
                    side * self.rng.range(spread * 0.4, spread),
                    self.rng.range(-spread * 0.5, -5.0),
                ),
                life,
                max_life: life,
                color: DUST,
                size: self.rng.range(1.5, 2.5),
                gravity: 150.0,
            });
        }
    }

    /// The ball pops: rubber fragments fly out in every direction.
    fn burst(&mut self, at: Vec2) {
        for i in 0..16 {
            let angle = i as f32 / 16.0 * std::f32::consts::TAU + self.rng.range(-0.2, 0.2);
            let speed = self.rng.range(70.0, 170.0);
            let life = self.rng.range(0.6, 0.9);
            self.list.push(Particle {
                pos: at,
                vel: vec2(angle.cos(), angle.sin()) * speed - vec2(0.0, 60.0),
                life,
                max_life: life,
                color: BALL_BITS[i % BALL_BITS.len()],
                size: self.rng.range(2.0, 3.5),
                gravity: 500.0,
            });
        }
    }

    /// A ring of sparkles where the ball reappears.
    fn sparkle(&mut self, at: Vec2) {
        for i in 0..10 {
            let angle = i as f32 / 10.0 * std::f32::consts::TAU;
            let life = 0.35;
            self.list.push(Particle {
                pos: at + vec2(angle.cos(), angle.sin()) * 12.0,
                vel: vec2(angle.cos(), angle.sin()) * 40.0,
                life,
                max_life: life,
                color: WHITE,
                size: 2.0,
                gravity: 0.0,
            });
        }
    }

    fn confetti(&mut self, at: Vec2) {
        for i in 0..36 {
            let life = self.rng.range(0.9, 1.4);
            self.list.push(Particle {
                pos: at,
                vel: vec2(
                    self.rng.range(-130.0, 130.0),
                    self.rng.range(-280.0, -110.0),
                ),
                life,
                max_life: life,
                color: CONFETTI[i % CONFETTI.len()],
                size: 2.0,
                gravity: 420.0,
            });
        }
    }

    /// Draws in world coordinates (call with the level camera set). Each
    /// particle shrinks and fades over its last part of life.
    pub fn draw(&self) {
        for p in &self.list {
            let t = (p.life / p.max_life).clamp(0.0, 1.0);
            let size = (p.size * (0.4 + 0.6 * t)).round().max(1.0);
            let mut c = p.color;
            c.a *= (t / 0.4).min(1.0);
            let pos = (p.pos - Vec2::splat(size / 2.0)).round();
            draw_rectangle(pos.x, pos.y, size, size, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bounce_tales::sim::InputFrame;

    fn drop_sim() -> Sim {
        let mut sim = Sim::for_level("flat").unwrap();
        sim.ball.pos.y -= 120.0;
        sim
    }

    #[test]
    fn landing_makes_dust_that_dies_out() {
        let mut sim = drop_sim();
        let mut fx = Particles::new();
        let mut peak = 0;
        for _ in 0..40 {
            sim.step(InputFrame::NONE);
            fx.on_step(&sim);
            peak = peak.max(fx.len());
        }
        assert!(peak >= 3, "no dust ({peak})");
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
            fx.on_step(&sim);
        }
        assert_eq!(fx.len(), 0, "dust should fade away");
    }

    #[test]
    fn effects_are_repeatable() {
        let run = || {
            let mut sim = drop_sim();
            let mut fx = Particles::new();
            for _ in 0..30 {
                sim.step(InputFrame::NONE);
                fx.on_step(&sim);
            }
            fx.list
                .iter()
                .map(|p| (p.pos.x, p.pos.y))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn popping_bursts_into_fragments() {
        let mut sim = Sim::for_level("hazards").unwrap();
        let mut fx = Particles::new();
        let mut burst = 0;
        for _ in 0..200 {
            sim.step(InputFrame::from_code("R").unwrap());
            let before = fx.len();
            fx.on_step(&sim);
            if sim.events.iter().any(|e| matches!(e, Event::Died { .. })) {
                burst = fx.len().saturating_sub(before);
                break;
            }
        }
        assert!(burst >= 12, "burst {burst}");
    }

    #[test]
    fn exit_confetti() {
        let mut sim = Sim::for_level("flat").unwrap();
        let mut fx = Particles::new();
        let mut burst = 0;
        for _ in 0..150 {
            sim.step(InputFrame::from_code("R").unwrap());
            let before = fx.len();
            fx.on_step(&sim);
            if sim.events.contains(&Event::ExitReached) {
                burst = fx.len().saturating_sub(before);
            }
        }
        assert!(burst >= 30, "confetti burst {burst}");
    }
}
