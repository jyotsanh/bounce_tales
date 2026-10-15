//! Runs every `tests/replays/*.replay` file through the headless sim and
//! fails if any `expect` line fails. Add a replay file to add a test.

use std::path::{Path, PathBuf};

use bounce_tales::replay::Replay;
use bounce_tales::sim::DEFAULT_LEVEL;

fn replay_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/replays");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("tests/replays exists")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "replay"))
        .collect();
    files.sort();
    files
}

#[test]
fn all_replays_pass() {
    let files = replay_files();
    assert!(!files.is_empty(), "no replay files found");
    let mut report = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy();
        match Replay::load(path) {
            Err(e) => report.push(format!("{name}: {e}")),
            Ok(r) => {
                if r.checks.is_empty() {
                    report.push(format!("{name}: has no expect lines"));
                }
                match r.start_sim(DEFAULT_LEVEL) {
                    Err(e) => report.push(format!("{name}: {e}")),
                    Ok(mut sim) => {
                        for f in r.run(&mut sim) {
                            report.push(format!("{name}: {f}"));
                        }
                    }
                }
            }
        }
    }
    assert!(report.is_empty(), "replay failures:\n{}", report.join("\n"));
}

#[test]
fn replays_are_deterministic() {
    for path in replay_files() {
        let r = Replay::load(&path).unwrap();
        let mut a = r.start_sim(DEFAULT_LEVEL).unwrap();
        let mut b = a.clone();
        r.run(&mut a);
        r.run(&mut b);
        assert_eq!(a, b, "{} diverged between runs", path.display());
    }
}
