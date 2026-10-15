//! Command-line options.
//!
//! Hand-rolled parser to keep dependencies small. Unknown flags are errors so
//! typos in test scripts fail loudly.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Options {
    /// Level to load on start (used from step 8 on).
    pub level: Option<String>,
    /// Run exactly this many simulation frames, then quit.
    pub frames: Option<u64>,
    /// Input replay file to drive the simulation (used from step 2 on).
    pub replay: Option<PathBuf>,
    /// Save the 240x320 game image to this PNG when the run ends.
    pub screenshot: Option<PathBuf>,
    /// Record live keyboard input to this replay file on exit.
    pub record: Option<PathBuf>,
    /// Start with the debug overlay shown (F1 toggles it).
    pub debug: bool,
    /// Start with sound off (M toggles it). Scripted runs (--frames,
    /// --replay) are always silent.
    pub mute: bool,
    /// Initial window scale factor (window = 240*scale x 320*scale).
    pub scale: u32,
}

pub const DEFAULT_SCALE: u32 = 2;

pub const USAGE: &str = "\
Usage: bounce_tales [OPTIONS]

Options:
  --level <name>       Start in the given level
  --frames <N>         Run N simulation frames, then exit
  --replay <file>      Drive input from a replay file
  --screenshot <png>   Save the game screen to a PNG on exit
                       (needs --frames or --replay)
  --record <file>      Record keyboard input to a replay file on exit
  --debug              Start with the debug overlay on (F1 toggles)
  --mute               Start with sound off (M toggles)
  --scale <N>          Window scale factor (default 2)
  -h, --help           Show this help";

#[derive(Debug, PartialEq)]
pub enum CliError {
    Help,
    Message(String),
}

impl Options {
    pub fn parse<I, S>(args: I) -> Result<Options, CliError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut opts = Options {
            scale: DEFAULT_SCALE,
            ..Default::default()
        };
        let mut it = args.into_iter().map(Into::into);
        while let Some(arg) = it.next() {
            let mut value = |name: &str| {
                it.next()
                    .ok_or_else(|| CliError::Message(format!("{name} needs a value")))
            };
            match arg.as_str() {
                "-h" | "--help" => return Err(CliError::Help),
                "--level" => opts.level = Some(value("--level")?),
                "--frames" => opts.frames = Some(parse_num("--frames", &value("--frames")?)?),
                "--replay" => opts.replay = Some(value("--replay")?.into()),
                "--screenshot" => opts.screenshot = Some(value("--screenshot")?.into()),
                "--record" => opts.record = Some(value("--record")?.into()),
                "--debug" => opts.debug = true,
                "--mute" => opts.mute = true,
                "--scale" => {
                    let s = parse_num("--scale", &value("--scale")?)?;
                    if !(1..=8).contains(&s) {
                        return Err(CliError::Message("--scale must be 1..=8".into()));
                    }
                    opts.scale = s as u32;
                }
                other => return Err(CliError::Message(format!("unknown argument: {other}"))),
            }
        }
        if opts.screenshot.is_some() && opts.frames.is_none() && opts.replay.is_none() {
            return Err(CliError::Message(
                "--screenshot requires --frames or --replay".into(),
            ));
        }
        if opts.record.is_some() && opts.replay.is_some() {
            return Err(CliError::Message(
                "--record cannot be combined with --replay".into(),
            ));
        }
        Ok(opts)
    }
}

fn parse_num(name: &str, s: &str) -> Result<u64, CliError> {
    s.parse()
        .map_err(|_| CliError::Message(format!("{name}: '{s}' is not a number")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, CliError> {
        Options::parse(args.iter().copied())
    }

    #[test]
    fn defaults() {
        let o = parse(&[]).unwrap();
        assert_eq!(o.scale, DEFAULT_SCALE);
        assert_eq!(o.frames, None);
    }

    #[test]
    fn all_flags() {
        let o = parse(&[
            "--level",
            "l1",
            "--frames",
            "300",
            "--replay",
            "r.txt",
            "--screenshot",
            "s.png",
            "--scale",
            "3",
            "--mute",
        ])
        .unwrap();
        assert_eq!(o.level.as_deref(), Some("l1"));
        assert_eq!(o.frames, Some(300));
        assert_eq!(o.replay, Some("r.txt".into()));
        assert_eq!(o.screenshot, Some("s.png".into()));
        assert_eq!(o.scale, 3);
        assert!(o.mute);
    }

    #[test]
    fn screenshot_with_replay_only() {
        let o = parse(&["--replay", "r", "--screenshot", "s.png"]).unwrap();
        assert_eq!(o.frames, None);
        assert!(o.screenshot.is_some());
    }

    #[test]
    fn errors() {
        assert_eq!(parse(&["--help"]), Err(CliError::Help));
        assert!(matches!(parse(&["--frames"]), Err(CliError::Message(_))));
        assert!(matches!(
            parse(&["--frames", "x"]),
            Err(CliError::Message(_))
        ));
        assert!(matches!(parse(&["--bogus"]), Err(CliError::Message(_))));
        assert!(matches!(
            parse(&["--scale", "0"]),
            Err(CliError::Message(_))
        ));
        assert!(matches!(
            parse(&["--screenshot", "a.png"]),
            Err(CliError::Message(_))
        ));
    }
}
