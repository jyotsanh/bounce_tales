//! Game flow around the level simulation: play a level, celebrate at the
//! exit, show the results, move on to the next campaign level carrying
//! lives and score, and show a "finished" screen after the last one.
//!
//! Like the sim, this is deterministic and input-driven (no clock), so it
//! can be unit-tested and replayed.

use crate::level::Level;
use crate::sim::{Carry, InputFrame, Sim};

/// Levels in play order. Real levels arrive in steps 34-38.
pub const CAMPAIGN: &[&str] = &["big", "hazards", "hypno"];

/// Steps between touching the exit and the results screen (confetti).
pub const COMPLETE_DELAY: u32 = 75;
/// Steps between results lines appearing.
pub const REVEAL_STEP: u32 = 10;
/// Lines on the results screen (time, gems, eggs, time bonus, egg bonus,
/// score).
pub const RESULT_LINES: u32 = 6;
/// Earliest step a results/finished screen accepts "jump" to continue,
/// so a jump held through the exit doesn't skip it.
pub const CONTINUE_AFTER: u32 = 30;
/// Points per second under par.
pub const TIME_BONUS_PER_SECOND: u32 = 10;
/// Points for finding every egg in a level.
pub const ALL_EGGS_BONUS: u32 = 1000;

/// How a level went (also kept for saving in step 32).
#[derive(Debug, Clone, PartialEq)]
pub struct LevelResult {
    pub level: String,
    /// Whole seconds taken (current attempt).
    pub seconds: u32,
    pub par_seconds: u32,
    pub gems: u32,
    pub gems_total: u32,
    pub eggs: u32,
    pub eggs_total: u32,
    pub deaths: u32,
    pub time_bonus: u32,
    pub egg_bonus: u32,
    /// Total score after the bonuses.
    pub score: u32,
}

impl LevelResult {
    /// Builds the result for a sim that has just reached its exit.
    pub fn from_sim(name: &str, sim: &Sim) -> LevelResult {
        let seconds = sim.level_seconds() as u32;
        let par = sim.level.par_seconds;
        let time_bonus = par.saturating_sub(seconds) * TIME_BONUS_PER_SECOND;
        let egg_bonus = if sim.eggs_total > 0 && sim.eggs == sim.eggs_total {
            ALL_EGGS_BONUS
        } else {
            0
        };
        LevelResult {
            level: name.to_owned(),
            seconds,
            par_seconds: par,
            gems: sim.gems,
            gems_total: sim.gems_total,
            eggs: sim.eggs,
            eggs_total: sim.eggs_total,
            deaths: sim.deaths,
            time_bonus,
            egg_bonus,
            score: sim.score + time_bonus + egg_bonus,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Playing,
    /// Level done; `ticks` since the screen appeared.
    Results {
        result: LevelResult,
        ticks: u32,
    },
    /// Campaign (or a single level) done.
    Finished {
        ticks: u32,
    },
}

pub struct Game {
    pub sim: Sim,
    pub mode: Mode,
    /// Level order this game follows.
    pub campaign: Vec<String>,
    /// Position in `campaign`, or `None` for a single level outside it.
    pub index: Option<usize>,
    /// Name of the level being played.
    pub level_name: String,
    /// Results of every finished level, in order.
    pub results: Vec<LevelResult>,
    /// Steps since the exit was touched (for the celebration delay).
    exit_ticks: u32,
    prev_jump: bool,
}

impl Game {
    /// Starts at `level`; if it is in the campaign, play continues from
    /// there through the rest of the campaign.
    pub fn new(level: &str) -> Result<Game, String> {
        Game::with_campaign(CAMPAIGN.iter().map(|s| s.to_string()).collect(), level)
    }

    pub fn with_campaign(campaign: Vec<String>, level: &str) -> Result<Game, String> {
        let index = campaign.iter().position(|l| l == level);
        let sim = Sim::with_level_carry(Level::load(level)?, Carry::default());
        Ok(Game {
            sim,
            mode: Mode::Playing,
            campaign,
            index,
            level_name: level.to_owned(),
            results: Vec::new(),
            exit_ticks: 0,
            prev_jump: false,
        })
    }

    /// Advances one fixed step.
    pub fn step(&mut self, input: InputFrame) {
        let jump_pressed = input.jump && !self.prev_jump;
        self.prev_jump = input.jump;

        match &mut self.mode {
            Mode::Playing => {
                self.sim.step(input);
                if self.sim.exit_reached {
                    self.exit_ticks += 1;
                    if self.exit_ticks >= COMPLETE_DELAY {
                        let result = LevelResult::from_sim(&self.level_name, &self.sim);
                        self.sim.score = result.score;
                        self.mode = Mode::Results { result, ticks: 0 };
                    }
                }
            }
            Mode::Results { result, ticks } => {
                *ticks += 1;
                // Still counts for particles etc.
                self.sim.step(InputFrame::NONE);
                if jump_pressed && *ticks >= CONTINUE_AFTER {
                    let result = result.clone();
                    self.results.push(result.clone());
                    self.next_level(Carry {
                        lives: self.sim.lives,
                        score: result.score,
                    });
                }
            }
            Mode::Finished { ticks } => {
                *ticks += 1;
                // The sim is frozen: its last events must not replay (sound,
                // particles) every step.
                self.sim.events.clear();
                if jump_pressed && *ticks >= CONTINUE_AFTER {
                    // Start the campaign over from the first level.
                    let first = self
                        .campaign
                        .first()
                        .cloned()
                        .unwrap_or_else(|| self.level_name.clone());
                    if let Ok(game) = Game::with_campaign(self.campaign.clone(), &first) {
                        *self = game;
                    }
                }
            }
        }
    }

    fn next_level(&mut self, carry: Carry) {
        let next = self
            .index
            .map(|i| i + 1)
            .filter(|&i| i < self.campaign.len());
        let Some(i) = next else {
            self.mode = Mode::Finished { ticks: 0 };
            return;
        };
        let name = self.campaign[i].clone();
        match Level::load(&name) {
            Ok(level) => {
                self.sim = Sim::with_level_carry(level, carry);
                self.index = Some(i);
                self.level_name = name;
                self.mode = Mode::Playing;
                self.exit_ticks = 0;
            }
            // Levels are checked by tests; never strand the player anyway.
            Err(_) => self.mode = Mode::Finished { ticks: 0 },
        }
    }

    /// Total eggs found across finished levels.
    pub fn eggs_found(&self) -> (u32, u32) {
        self.results
            .iter()
            .fold((0, 0), |(e, t), r| (e + r.eggs, t + r.eggs_total))
    }

    /// Final score so far.
    pub fn score(&self) -> u32 {
        self.sim.score
    }
}

/// "m:ss" for a whole number of seconds.
pub fn format_time(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: &str) -> InputFrame {
        InputFrame::from_code(code).unwrap()
    }

    fn run(game: &mut Game, code: &str, steps: u32) {
        for _ in 0..steps {
            game.step(press(code));
        }
    }

    /// Plays `flat` to the exit and waits for the results screen.
    fn finish_flat(game: &mut Game) {
        run(game, "R", 150);
        assert!(game.sim.exit_reached);
        run(game, "-", COMPLETE_DELAY);
        assert!(matches!(game.mode, Mode::Results { .. }), "{:?}", game.mode);
    }

    #[test]
    fn exit_leads_to_results_with_bonuses() {
        let mut game = Game::with_campaign(vec![], "flat").unwrap();
        finish_flat(&mut game);
        let Mode::Results { result, .. } = &game.mode else {
            unreachable!()
        };
        assert_eq!(result.level, "flat");
        assert!(result.seconds <= 3, "{}", result.seconds);
        assert_eq!(result.par_seconds, 60);
        assert_eq!(
            result.time_bonus,
            (60 - result.seconds) * TIME_BONUS_PER_SECOND
        );
        assert_eq!(result.egg_bonus, 0, "flat has no eggs");
        assert_eq!(game.sim.score, result.score);
    }

    #[test]
    fn jump_held_through_the_exit_does_not_skip_results() {
        let mut game = Game::with_campaign(vec![], "flat").unwrap();
        run(&mut game, "RJ", 150 + COMPLETE_DELAY);
        run(&mut game, "J", 100);
        assert!(
            matches!(game.mode, Mode::Results { .. }),
            "needs a fresh press"
        );
        run(&mut game, "-", 1);
        game.step(press("J"));
        assert!(matches!(game.mode, Mode::Finished { .. }));
    }

    #[test]
    fn next_level_carries_lives_and_score() {
        let campaign = vec!["flat".to_string(), "hazards".to_string()];
        let mut game = Game::with_campaign(campaign, "flat").unwrap();
        finish_flat(&mut game);
        let Mode::Results { result, .. } = game.mode.clone() else {
            unreachable!()
        };
        run(&mut game, "-", CONTINUE_AFTER);
        game.step(press("J"));
        assert_eq!(game.mode, Mode::Playing);
        assert_eq!(game.level_name, "hazards");
        assert_eq!(game.index, Some(1));
        assert_eq!(game.sim.score, result.score);
        assert_eq!(game.sim.lives, 3);
        assert_eq!(game.results, vec![result]);
    }

    #[test]
    fn lives_lost_stay_lost_in_the_next_level() {
        let campaign = vec!["hazards".to_string(), "flat".to_string()];
        let mut game = Game::with_campaign(campaign, "hazards").unwrap();
        // Die once on the spikes, then do the clean run (jump windows from
        // the hazards_course replay).
        run(&mut game, "R", 60);
        run(&mut game, "-", 60);
        assert_eq!(game.sim.lives, 2);
        for t in 0..160 {
            game.step(press(if t == 20 || t == 96 { "RJ" } else { "R" }));
        }
        assert!(game.sim.exit_reached);
        run(&mut game, "-", COMPLETE_DELAY + CONTINUE_AFTER);
        game.step(press("J"));
        assert_eq!(game.level_name, "flat");
        assert_eq!(game.sim.lives, 2);
    }

    #[test]
    fn after_the_last_level_comes_finished_then_a_fresh_start() {
        let campaign = vec!["flat".to_string()];
        let mut game = Game::with_campaign(campaign, "flat").unwrap();
        finish_flat(&mut game);
        run(&mut game, "-", CONTINUE_AFTER);
        game.step(press("J"));
        assert_eq!(game.mode, Mode::Finished { ticks: 0 });
        assert_eq!(game.eggs_found(), (0, 0));
        run(&mut game, "-", CONTINUE_AFTER);
        assert!(game.sim.events.is_empty(), "frozen sim replays nothing");
        game.step(press("J"));
        assert_eq!(game.mode, Mode::Playing);
        assert_eq!(game.sim.score, 0);
        assert!(game.results.is_empty());
    }

    #[test]
    fn every_campaign_level_loads() {
        for name in CAMPAIGN {
            Level::load(name).unwrap_or_else(|e| panic!("{e}"));
        }
    }

    #[test]
    fn time_format() {
        assert_eq!(format_time(0), "0:00");
        assert_eq!(format_time(83), "1:23");
    }
}
