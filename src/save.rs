//! The save file: progress and settings, as RON text.
//!
//! Lives in the platform data directory (`~/.local/share/bounce_tales/
//! save.ron` on Linux, honouring `XDG_DATA_HOME`). Written atomically (a
//! temp file renamed over the old one). Missing fields take their defaults,
//! so older saves keep loading as the format grows.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::audio::Volume;
use crate::progress::Progress;

/// Bumped when the format changes in a way that needs converting.
pub const VERSION: u32 = 1;

/// Player settings (the options menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub volume: Volume,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            volume: Volume::DEFAULT,
            fullscreen: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    pub settings: Settings,
    pub progress: Progress,
}

impl Default for SaveData {
    fn default() -> SaveData {
        SaveData {
            version: VERSION,
            settings: Settings::default(),
            progress: Progress::default(),
        }
    }
}

/// Default save file location, if the platform has a data directory.
pub fn default_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "bounce_tales").map(|d| d.data_dir().join("save.ron"))
}

impl SaveData {
    pub fn from_text(text: &str) -> Result<SaveData, String> {
        ron::from_str(text).map_err(|e| format!("bad save file: {e}"))
    }

    pub fn to_text(&self) -> String {
        let pretty = ron::ser::PrettyConfig::default();
        ron::ser::to_string_pretty(self, pretty).expect("save data always serialises")
    }

    /// Reads a save; a missing file is a fresh start.
    pub fn load(path: &Path) -> Result<SaveData, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => SaveData::from_text(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SaveData::default()),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    /// Writes the save, creating its directory; never leaves a half-written
    /// file behind.
    pub fn store(&self, path: &Path) -> Result<(), String> {
        let fail = |e: std::io::Error| format!("cannot write {}: {e}", path.display());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(fail)?;
        }
        let tmp = path.with_extension("ron.tmp");
        std::fs::write(&tmp, self.to_text()).map_err(fail)?;
        std::fs::rename(&tmp, path).map_err(fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::LevelResult;

    fn sample() -> SaveData {
        let mut data = SaveData::default();
        data.settings.volume.music = 3;
        data.settings.fullscreen = true;
        data.progress.record(&LevelResult {
            level: "big".into(),
            seconds: 80,
            par_seconds: 120,
            gems: 5,
            gems_total: 9,
            eggs: 2,
            eggs_total: 3,
            deaths: 1,
            time_bonus: 400,
            egg_bonus: 0,
            score: 1900,
        });
        data
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bounce_tales_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn text_round_trip() {
        let data = sample();
        let text = data.to_text();
        assert!(text.contains("best_score: 1900"), "{text}");
        assert_eq!(SaveData::from_text(&text).unwrap(), data);
    }

    #[test]
    fn unlock_all_is_never_saved() {
        let mut data = sample();
        data.progress.unlock_all = true;
        let back = SaveData::from_text(&data.to_text()).unwrap();
        assert!(!back.progress.unlock_all);
    }

    #[test]
    fn missing_fields_take_defaults_and_bad_text_is_an_error() {
        let d = SaveData::from_text("(settings: (fullscreen: true))").unwrap();
        assert!(d.settings.fullscreen);
        assert_eq!(d.settings.volume, Volume::DEFAULT);
        assert_eq!(d.version, VERSION);
        assert!(d.progress.records.is_empty());
        assert!(SaveData::from_text("(settings: 42").is_err());
    }

    #[test]
    fn store_and_load_files() {
        let dir = temp_dir("store");
        let path = dir.join("nested/save.ron");
        assert_eq!(
            SaveData::load(&path).unwrap(),
            SaveData::default(),
            "missing = fresh"
        );
        let data = sample();
        data.store(&path).unwrap();
        assert_eq!(SaveData::load(&path).unwrap(), data);
        assert!(!path.with_extension("ron.tmp").exists());
        std::fs::write(&path, "garbage").unwrap();
        assert!(SaveData::load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_path_is_in_a_data_dir() {
        if let Some(p) = default_path() {
            assert!(p.ends_with("save.ron"));
            assert!(p.to_string_lossy().contains("bounce_tales"));
        }
    }
}
