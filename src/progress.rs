//! What the player has achieved: which levels are done and their best
//! score, eggs and time. Levels unlock in campaign order. Saved to disk
//! from step 32 on.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::game::LevelResult;

/// Best results for one level.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LevelRecord {
    pub completed: bool,
    pub best_score: u32,
    /// Most eggs found in one run.
    pub eggs: u32,
    pub best_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// By level name.
    pub records: BTreeMap<String, LevelRecord>,
    /// Cutscenes already shown (each plays by itself once).
    pub seen: BTreeSet<String>,
    /// Every level playable (`--unlock-all`, for testing; never saved).
    #[serde(skip)]
    pub unlock_all: bool,
}

impl Progress {
    pub fn get(&self, level: &str) -> LevelRecord {
        self.records.get(level).cloned().unwrap_or_default()
    }

    /// Keeps the best of each figure. Returns true for a new best score.
    pub fn record(&mut self, r: &LevelResult) -> bool {
        let rec = self.records.entry(r.level.clone()).or_default();
        let new_best = !rec.completed || r.score > rec.best_score;
        rec.completed = true;
        rec.best_score = rec.best_score.max(r.score);
        rec.eggs = rec.eggs.max(r.eggs);
        rec.best_seconds = Some(rec.best_seconds.map_or(r.seconds, |s| s.min(r.seconds)));
        new_best
    }

    /// Level `i` of `campaign` can be played: the first always, the others
    /// once the one before is done.
    pub fn unlocked(&self, campaign: &[String], i: usize) -> bool {
        self.unlock_all || i == 0 || campaign.get(i - 1).is_some_and(|l| self.get(l).completed)
    }

    /// Where the map cursor starts: the first unlocked level not yet done,
    /// else the last one.
    pub fn next_level(&self, campaign: &[String]) -> usize {
        (0..campaign.len())
            .find(|&i| self.unlocked(campaign, i) && !self.get(&campaign[i]).completed)
            .unwrap_or(campaign.len().saturating_sub(1))
    }

    /// Sum of best scores.
    pub fn total_score(&self) -> u32 {
        self.records.values().map(|r| r.best_score).sum()
    }

    /// Eggs found over `campaign`.
    pub fn eggs_found(&self, campaign: &[String]) -> u32 {
        campaign.iter().map(|l| self.get(l).eggs).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(level: &str, score: u32, eggs: u32, seconds: u32) -> LevelResult {
        LevelResult {
            level: level.into(),
            seconds,
            par_seconds: 60,
            gems: 0,
            gems_total: 0,
            eggs,
            eggs_total: 3,
            deaths: 0,
            time_bonus: 0,
            egg_bonus: 0,
            score,
        }
    }

    fn campaign() -> Vec<String> {
        ["a", "b", "c"].map(String::from).to_vec()
    }

    #[test]
    fn levels_unlock_in_order() {
        let mut p = Progress::default();
        let c = campaign();
        assert!(p.unlocked(&c, 0));
        assert!(!p.unlocked(&c, 1));
        assert_eq!(p.next_level(&c), 0);
        p.record(&result("a", 100, 0, 50));
        assert!(p.unlocked(&c, 1));
        assert!(!p.unlocked(&c, 2));
        assert_eq!(p.next_level(&c), 1);
        p.record(&result("b", 100, 0, 50));
        p.record(&result("c", 100, 0, 50));
        assert_eq!(p.next_level(&c), 2, "all done: the last");
        let all = Progress {
            unlock_all: true,
            ..Progress::default()
        };
        assert!(all.unlocked(&c, 2));
    }

    #[test]
    fn keeps_the_best_of_each() {
        let mut p = Progress::default();
        assert!(p.record(&result("a", 500, 1, 90)));
        assert!(!p.record(&result("a", 300, 3, 70)));
        assert!(p.record(&result("a", 800, 0, 120)));
        let r = p.get("a");
        assert_eq!(
            r,
            LevelRecord {
                completed: true,
                best_score: 800,
                eggs: 3,
                best_seconds: Some(70),
            }
        );
        p.record(&result("b", 200, 2, 10));
        assert_eq!(p.total_score(), 1000);
        assert_eq!(p.eggs_found(&campaign()), 5);
    }
}
