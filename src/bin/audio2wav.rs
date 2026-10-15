//! Writes every sound effect and music track as a WAV file, to listen to
//! them outside the game: `cargo run --bin audio2wav -- <dir>` (default
//! `shots/audio`). Effects are `sfx_<name>.wav`, tracks `music_<name>.wav`.

use bounce_tales::audio::music;
use bounce_tales::audio::sfx::Sfx;

fn write(path: &str, bytes: &[u8]) {
    if let Err(e) = std::fs::write(path, bytes) {
        eprintln!("error: cannot write {path}: {e}");
        std::process::exit(1);
    }
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "shots/audio".into());
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("error: cannot create {dir}: {e}");
        std::process::exit(1);
    }
    for sfx in Sfx::ALL {
        let name = format!("{sfx:?}").to_lowercase();
        write(&format!("{dir}/sfx_{name}.wav"), &sfx.wav());
    }
    for name in music::TRACKS {
        let track = music::track(name).expect("listed track");
        let wav = track.wav().unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(1);
        });
        write(&format!("{dir}/music_{name}.wav"), &wav);
    }
    println!(
        "wrote {} effects and {} tracks to {dir}",
        Sfx::ALL.len(),
        music::TRACKS.len()
    );
}
