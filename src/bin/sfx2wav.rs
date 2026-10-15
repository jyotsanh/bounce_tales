//! Writes every sound effect as a WAV file, to listen to them outside the
//! game: `cargo run --bin sfx2wav -- <dir>` (default `shots/sfx`).

use bounce_tales::audio::sfx::Sfx;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "shots/sfx".into());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("error: cannot create {dir}: {e}");
        std::process::exit(1);
    }
    for sfx in Sfx::ALL {
        let path = format!("{dir}/{}.wav", format!("{sfx:?}").to_lowercase());
        if let Err(e) = std::fs::write(&path, sfx.wav()) {
            eprintln!("error: cannot write {path}: {e}");
            std::process::exit(1);
        }
    }
    println!("wrote {} sounds to {dir}", Sfx::ALL.len());
}
