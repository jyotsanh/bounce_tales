//! Fails if any committed atlas in assets/gen is out of date with its
//! source in assets/art. Fix with `cargo run -p artgen`.

use std::path::PathBuf;

#[test]
fn generated_art_is_up_to_date() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = artgen::sources(&root.join("assets/art")).unwrap();
    assert!(!sources.is_empty(), "no art sources found");
    let mut stale = Vec::new();
    for path in sources {
        let (name, atlas) = artgen::build(&path).unwrap();
        if let Some(why) = artgen::stale_reason(&atlas, &root.join("assets/gen"), &name) {
            stale.push(why);
        }
    }
    assert!(
        stale.is_empty(),
        "run `cargo run -p artgen`:\n{}",
        stale.join("\n")
    );
}
