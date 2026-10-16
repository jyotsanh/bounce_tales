//! Linking helper for sound on Linux.
//!
//! macroquad's audio links against ALSA (`-lasound`). The linker wants the
//! development symlink `libasound.so`, which only the `libasound2-dev`
//! package installs; every desktop has the runtime `libasound.so.2`. When the
//! symlink is missing, make one in OUT_DIR so the game builds without the dev
//! package.
//!
//! On Windows, embeds the icon and version information in the .exe.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("linux") => {
            #[cfg(unix)]
            link_alsa();
        }
        Ok("windows") => {
            #[cfg(windows)]
            windows_resources();
        }
        _ => {}
    }
}

/// Icon and file properties shown by Explorer (version from Cargo.toml).
#[cfg(windows)]
fn windows_resources() {
    println!("cargo:rerun-if-changed=packaging/icon/bounce_tales.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("packaging/icon/bounce_tales.ico")
        .set("ProductName", "Bounce Tales")
        .set("FileDescription", "Bounce Tales")
        .set("CompanyName", "Jyotsan Hamal")
        .set("LegalCopyright", "MIT License, (c) 2026 Jyotsan Hamal");
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the Windows icon: {e}");
    }
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
