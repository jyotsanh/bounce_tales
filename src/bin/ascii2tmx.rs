//! Converts an ASCII map into a Tiled level.
//!
//!     cargo run --bin ascii2tmx -- <map.txt | builtin:NAME> [out.tmx]
//!
//! Without `out.tmx` the XML goes to stdout. ASCII glyphs are documented on
//! `sim::map::Tile` (`#` solid, slopes `/ \ r R L l`, `o` spawn). The result
//! uses the forest tileset (auto-tiled) and is meant to be opened and
//! decorated in the Tiled editor. Remember to list new levels in
//! `level::TMX_LEVELS`.

use std::process::ExitCode;

use bounce_tales::level::ascii_to_tmx;
use bounce_tales::sim::map;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input, output) = match &args[..] {
        [i] => (i, None),
        [i, o] => (i, Some(o)),
        _ => {
            eprintln!("usage: ascii2tmx <map.txt | builtin:NAME> [out.tmx]");
            return ExitCode::from(2);
        }
    };
    let text = match input.strip_prefix("builtin:") {
        Some(name) => match map::builtin_text(name) {
            Some(t) => t.to_owned(),
            None => {
                eprintln!(
                    "error: no built-in map '{name}' (known: {})",
                    map::BUILTIN_NAMES.join(", ")
                );
                return ExitCode::FAILURE;
            }
        },
        None => match std::fs::read_to_string(input) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: {input}: {e}");
                return ExitCode::FAILURE;
            }
        },
    };
    let tmx = match ascii_to_tmx(&text) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    match output {
        None => print!("{tmx}"),
        Some(path) => {
            if let Err(e) = std::fs::write(path, tmx) {
                eprintln!("error: {path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {path}");
        }
    }
    ExitCode::SUCCESS
}
