//! `cargo run -p artgen` rebuilds every sheet in `assets/art` into
//! `assets/gen`. `cargo run -p artgen -- --check` only reports stale outputs
//! (exit code 1 if any).

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let check = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--check") => true,
        Some(other) => {
            eprintln!("usage: artgen [--check]   (unknown argument '{other}')");
            return ExitCode::from(2);
        }
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (src_dir, out_dir) = (root.join("assets/art"), root.join("assets/gen"));

    let sources = match artgen::sources(&src_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut failed = false;
    for path in sources {
        let (name, atlas) = match artgen::build(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("error: {e}");
                failed = true;
                continue;
            }
        };
        if check {
            if let Some(why) = artgen::stale_reason(&atlas, &out_dir, &name) {
                eprintln!("stale: {why}");
                failed = true;
            }
            continue;
        }
        let png = out_dir.join(format!("{name}.png"));
        let idx = out_dir.join(format!("{name}.idx"));
        if let Err(e) = std::fs::write(&png, artgen::encode_png(&atlas))
            .and_then(|_| std::fs::write(&idx, &atlas.index))
        {
            eprintln!("error: writing {name}: {e}");
            failed = true;
            continue;
        }
        println!(
            "{name}: {}x{} px, {} sprites",
            atlas.width,
            atlas.height,
            atlas.index.lines().count()
        );
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
