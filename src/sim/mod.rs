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
pub mod hazards;
pub mod input;
pub mod map;
pub mod physics;
pub mod rng;

pub use ball::Ball;
pub use camera::Camera;
pub use entities::{Entity, EntityKind, Event, Item};
pub use input::InputFrame;
pub use map::TileMap;

use macroquad::math::Vec2;

use crate::level::Level;
use hazards::Cause;
pub use rng::Rng;

/// Simulation rate. The sim always advances in steps of exactly `DT`.
pub const TICKS_PER_SECOND: u32 = 60;
pub const DT: f32 = 1.0 / TICKS_PER_SECOND as f32;

/// Landings slower than this don't produce a `Landed` event, px/s.
pub const LAND_EVENT_MIN_SPEED: f32 = 100.0;

/// Level loaded when none is given.
pub const DEFAULT_LEVEL: &str = "big";

/// Steps between dying and reappearing (1 s: the burst plays out).
pub const DEATH_TICKS: u32 = 60;
/// Lives at the start of a level (and after a game over).
pub const START_LIVES: u32 = 3;
/// Most balls you can hold.
pub const MAX_LIVES: u32 = 9;
/// Steps the "game over" pause lasts before the level restarts.
pub const GAME_OVER_TICKS: u32 = 150;
/// Steps of the grow-in effect after respawning (drawing only).
pub const RESPAWN_ANIM_TICKS: u32 = 12;

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
    /// Where the ball comes back after dying (level start; checkpoints move
    /// it in step 12). Ball centre, px.
    pub respawn_point: Vec2,
    /// Steps left before respawning; 0 = alive. While dying the ball is
    /// hidden and input is ignored.
    pub dying: u32,
    /// Counts down from `RESPAWN_ANIM_TICKS` after a respawn (drawing).
    pub respawn_anim: u32,
    /// Total deaths this level.
    pub deaths: u32,
    /// Balls left, including the one in play. Losing the last one is game
    /// over.
    pub lives: u32,
    /// Steps left of the game-over pause; 0 = not game over.
    pub game_over: u32,
    /// How many times the level was restarted after a game over.
    pub restarts: u32,
    pub score: u32,
    /// Gems and eggs collected this level, and how many the level has.
    pub gems: u32,
    pub gems_total: u32,
    pub eggs: u32,
    pub eggs_total: u32,
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
        let count = |want: Item| {
            entities
                .iter()
                .filter(|e| matches!(e.kind, EntityKind::Pickup { item, .. } if item == want))
                .count() as u32
        };
        let (gems_total, eggs_total) = (count(Item::Gem), count(Item::Egg));
        Sim {
            tick: 0,
            entities,
            events: Vec::new(),
            exit_reached: false,
            respawn_point: level.map.spawn,
            dying: 0,
            respawn_anim: 0,
            deaths: 0,
            lives: START_LIVES,
            game_over: 0,
            restarts: 0,
            score: 0,
            gems: 0,
            gems_total,
            eggs: 0,
            eggs_total,
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
        self.respawn_anim = self.respawn_anim.saturating_sub(1);

        if self.game_over > 0 {
            self.game_over -= 1;
            if self.game_over == 0 {
                self.restart_level();
            }
            return;
        }
        if self.dying > 0 {
            self.dying -= 1;
            if self.dying == 0 {
                if self.lives == 0 {
                    self.game_over = GAME_OVER_TICKS;
                    self.events.push(Event::GameOver);
                } else {
                    self.respawn();
                }
            }
            return;
        }

        let jump_pressed = self.jump_pressed();
        let bottom = |b: &Ball| b.pos + macroquad::math::vec2(0.0, b.radius);
        let jumped = self.ball.update(input, jump_pressed);
        if jumped {
            self.events.push(Event::Jumped {
                pos: bottom(&self.ball),
            });
        }
        let contacts = physics::move_and_collide(&mut self.ball, &self.level.map);
        if contacts.impact_speed > LAND_EVENT_MIN_SPEED {
            self.events.push(Event::Landed {
                pos: self.ball.pos - contacts.floor_normal * self.ball.radius,
                speed: contacts.impact_speed,
            });
        }
        self.ball.update_spin();
        self.ball.update_squash(contacts.impact_speed, jumped);

        self.camera.update(&self.ball, &self.level.map);
        if let Some(cause) = hazards::check(&self.ball, &self.level.map) {
            self.die(cause);
            return;
        }

        for e in &mut self.entities {
            e.update(&self.ball, &mut self.events);
        }
        for e in &self.events {
            match *e {
                Event::ExitReached => self.exit_reached = true,
                Event::CheckpointReached { respawn } => self.respawn_point = respawn,
                Event::Collected { item, .. } => {
                    self.score += item.points();
                    match item {
                        Item::Gem => self.gems += 1,
                        Item::Egg => self.eggs += 1,
                        Item::Life => self.lives = (self.lives + 1).min(MAX_LIVES),
                    }
                }
                _ => {}
            }
        }
    }

    /// Destroys the ball; it comes back after `DEATH_TICKS` steps.
    pub fn die(&mut self, cause: Cause) {
        self.events.push(Event::Died {
            pos: self.ball.pos,
            cause,
        });
        self.deaths += 1;
        self.lives = self.lives.saturating_sub(1);
        self.dying = DEATH_TICKS;
        self.ball.vel = Vec2::ZERO;
    }

    fn respawn(&mut self) {
        self.ball = Ball::new(self.respawn_point);
        self.camera = Camera::snapped_to(&self.ball, &self.level.map);
        self.respawn_anim = RESPAWN_ANIM_TICKS;
        self.events.push(Event::Respawned {
            pos: self.respawn_point,
        });
    }

    /// Starts the level over: fresh entities, full lives, back to the
    /// start. Keeps the step counter, input history and restart count.
    fn restart_level(&mut self) {
        let mut fresh = Sim::with_level(self.level.clone());
        fresh.tick = self.tick;
        fresh.input = self.input;
        fresh.prev_input = self.prev_input;
        fresh.restarts = self.restarts + 1;
        fresh.respawn_anim = RESPAWN_ANIM_TICKS;
        fresh.events.push(Event::LevelRestarted);
        *self = fresh;
    }

    /// True while the ball should be drawn (not popped, not game over).
    pub fn ball_visible(&self) -> bool {
        self.dying == 0 && self.game_over == 0
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

    fn flat() -> Sim {
        let mut sim = Sim::for_level("flat").unwrap();
        for _ in 0..5 {
            sim.step(InputFrame::NONE);
        }
        sim
    }

    #[test]
    fn spikes_kill_then_respawn_at_the_start() {
        let mut sim = Sim::for_level("hazards").unwrap();
        let start = sim.ball.pos;
        let mut died_at = None;
        for i in 0..200 {
            sim.step(press("R"));
            if let Some(Event::Died { cause, .. }) = sim.events.first().copied() {
                assert_eq!(cause, Cause::Spikes);
                died_at = Some(i);
                break;
            }
        }
        assert!(died_at.is_some(), "never hit the spikes");
        assert_eq!(sim.deaths, 1);
        let frozen = sim.ball.pos;
        for _ in 0..DEATH_TICKS - 1 {
            sim.step(press("R"));
            assert_eq!(sim.ball.pos, frozen, "input ignored while dying");
            assert!(sim.dying > 0);
        }
        sim.step(press("R"));
        assert!(
            matches!(sim.events[..], [Event::Respawned { .. }]),
            "{:?}",
            sim.events
        );
        assert_eq!(sim.dying, 0);
        assert_eq!(sim.ball.pos, start);
        assert_eq!(sim.respawn_anim, RESPAWN_ANIM_TICKS);
    }

    /// Runs `code` until the ball dies (or panics after `limit` steps).
    fn until_death(sim: &mut Sim, code: &str, limit: usize) {
        let deaths = sim.deaths;
        for _ in 0..limit {
            sim.step(press(code));
            if sim.deaths > deaths {
                return;
            }
        }
        panic!("no death within {limit} steps");
    }

    fn wait(sim: &mut Sim, steps: u32) {
        for _ in 0..steps {
            sim.step(InputFrame::NONE);
        }
    }

    #[test]
    fn losing_every_life_is_game_over_then_a_fresh_restart() {
        let mut sim = Sim::for_level("hazards").unwrap();
        assert_eq!(sim.lives, START_LIVES);
        for left in (1..START_LIVES).rev() {
            until_death(&mut sim, "R", 200);
            assert_eq!(sim.lives, left);
            wait(&mut sim, DEATH_TICKS);
            assert_eq!(sim.dying, 0, "respawned with lives left");
        }
        until_death(&mut sim, "R", 200);
        assert_eq!(sim.lives, 0);
        wait(&mut sim, DEATH_TICKS);
        assert!(sim.game_over > 0, "game over instead of a respawn");
        assert!(!sim.ball_visible(), "no ball during game over");
        let tick = sim.tick;
        wait(&mut sim, GAME_OVER_TICKS);
        assert!(sim.events.contains(&Event::LevelRestarted));
        assert_eq!(sim.lives, START_LIVES);
        assert_eq!(sim.deaths, 0);
        assert_eq!(sim.restarts, 1);
        assert_eq!(
            sim.tick,
            tick + GAME_OVER_TICKS as u64,
            "tick keeps counting"
        );
        assert_eq!(sim.ball.pos, sim.level.map.spawn);
    }

    #[test]
    fn checkpoint_moves_the_respawn_point() {
        let mut sim = Sim::for_level("hazards").unwrap();
        // Clear the floor spikes (jump window 13-30) and pass the checkpoint.
        for t in 0..60 {
            sim.step(press(if t == 20 { "RJ" } else { "R" }));
        }
        assert!(
            sim.entities
                .iter()
                .any(|e| e.kind == EntityKind::Checkpoint { active: true }),
            "checkpoint not touched"
        );
        let cp = sim.respawn_point;
        assert!(cp.x > sim.level.map.spawn.x);
        until_death(&mut sim, "R", 300); // into the pit
        wait(&mut sim, DEATH_TICKS);
        assert_eq!(sim.ball.pos, cp);
    }

    #[test]
    fn collecting_scores_and_counts() {
        let mut sim = Sim::with_level(Level::from_ascii("o*e+.\n#####").unwrap());
        assert_eq!((sim.gems_total, sim.eggs_total), (1, 1));
        for _ in 0..90 {
            sim.step(press("R"));
        }
        assert_eq!((sim.gems, sim.eggs), (1, 1));
        assert_eq!(sim.lives, START_LIVES + 1);
        assert_eq!(sim.score, 10 + 500 + 100);
    }

    #[test]
    fn lives_are_capped() {
        let mut sim = Sim::with_level(Level::from_ascii("o+++++++++.\n###########").unwrap());
        for _ in 0..200 {
            sim.step(press("R"));
        }
        assert_eq!(sim.lives, MAX_LIVES);
    }

    #[test]
    fn falling_out_of_the_level_kills() {
        let mut sim = Sim::for_level("hazards").unwrap();
        sim.ball.pos = macroquad::math::vec2(288.0, 200.0); // above the pit
        let mut cause = None;
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
            if let Some(Event::Died { cause: c, .. }) = sim.events.first().copied() {
                cause = Some(c);
                break;
            }
        }
        assert_eq!(cause, Some(Cause::Pit));
    }

    #[test]
    fn hard_landing_emits_event_and_squashes() {
        let mut sim = flat();
        sim.ball.pos.y -= 120.0;
        sim.ball.grounded = false;
        let mut landed = None;
        for _ in 0..60 {
            sim.step(InputFrame::NONE);
            if let Some(Event::Landed { pos, speed }) = sim.events.first().copied() {
                landed = Some((pos, speed, sim.ball.squash));
                break;
            }
        }
        let (pos, speed, squash) = landed.expect("no Landed event");
        assert!(speed > 300.0, "speed {speed}");
        assert_eq!(pos.y, 288.0, "contact point is on the floor");
        assert!(squash > 0.2, "squash {squash}");
        for _ in 0..120 {
            sim.step(InputFrame::NONE);
        }
        assert_eq!(sim.ball.squash, 0.0, "springs back to round");
    }

    #[test]
    fn squash_wobbles_past_round() {
        let mut sim = flat();
        sim.ball.pos.y -= 120.0;
        sim.ball.grounded = false;
        let mut min_squash: f32 = 0.0;
        for _ in 0..90 {
            sim.step(InputFrame::NONE);
            min_squash = min_squash.min(sim.ball.squash);
        }
        assert!(min_squash < -0.02, "no rebound stretch: {min_squash}");
    }

    #[test]
    fn jump_emits_event_and_stretches() {
        let mut sim = flat();
        sim.step(press("J"));
        assert!(
            matches!(sim.events[..], [Event::Jumped { .. }]),
            "{:?}",
            sim.events
        );
        assert!(sim.ball.squash < -0.1, "squash {}", sim.ball.squash);
    }

    #[test]
    fn rolling_emits_no_landings() {
        let mut sim = Sim::for_level("slopes").unwrap();
        for _ in 0..300 {
            sim.step(press("R"));
            assert!(
                !sim.events.iter().any(|e| matches!(e, Event::Landed { .. })),
                "landing while rolling at x={}",
                sim.ball.pos.x
            );
        }
    }
}
