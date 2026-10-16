//! Replays must play out the same on Linux, Windows and macOS. The system
//! maths library's sin/cos/... differ in their last bits between platforms,
//! so simulation code must use `sim::math` (pure-Rust libm) instead.

use std::path::Path;

/// Platform maths calls that are not bit-exact everywhere.
const FORBIDDEN: &[&str] = &[
    ".sin(",
    ".cos(",
    ".tan(",
    ".atan2(",
    ".atan(",
    ".asin(",
    ".acos(",
    ".sin_cos(",
    ".powf(",
    ".exp(",
    ".ln(",
    ".log10(",
    ".log2(",
    ".hypot(",
    ".cbrt(",
];

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn simulation_code_uses_portable_maths() {
    let mut files = Vec::new();
    rust_files(Path::new("src/sim"), &mut files);
    files.extend(["src/game.rs", "src/app.rs", "src/level.rs"].map(Into::into));
    let mut found = Vec::new();
    for file in files {
        if file.ends_with("math.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap();
        // Only the code, not its unit tests.
        let code = text.split("#[cfg(test)]").next().unwrap();
        for (n, line) in code.lines().enumerate() {
            let line = line.split("//").next().unwrap();
            if let Some(f) = FORBIDDEN.iter().find(|f| line.contains(*f)) {
                found.push(format!("{}:{}: {f}", file.display(), n + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "use sim::math instead:\n{}",
        found.join("\n")
    );
}
