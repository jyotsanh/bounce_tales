//! Linking helper for sound on Linux.
//!
//! macroquad's audio links against ALSA (`-lasound`). The linker wants the
//! development symlink `libasound.so`, which only the `libasound2-dev`
//! package installs; every desktop has the runtime `libasound.so.2`. When the
//! symlink is missing, make one in OUT_DIR so the game builds without the dev
//! package. Other platforms need nothing here.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    #[cfg(unix)]
    link_alsa();
}

#[cfg(unix)]
fn link_alsa() {
    use std::path::Path;

    const LIB_DIRS: &[&str] = &[
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
        "/lib/x86_64-linux-gnu",
    ];

    if LIB_DIRS
        .iter()
        .any(|d| Path::new(d).join("libasound.so").exists())
    {
        return;
    }
    let Some(runtime) = LIB_DIRS
        .iter()
        .map(|d| Path::new(d).join("libasound.so.2"))
        .find(|p| p.exists())
    else {
        println!("cargo:warning=ALSA not found: install libasound2-dev for sound");
        return;
    };
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let link = Path::new(&out).join("libasound.so");
    let _ = std::fs::remove_file(&link);
    if std::os::unix::fs::symlink(&runtime, &link).is_ok() {
        println!("cargo:rustc-link-search=native={out}");
    }
}
