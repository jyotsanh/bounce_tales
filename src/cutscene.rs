//! Story cutscenes: comic panels, each a picture with a caption typed out
//! letter by letter. Space shows the whole caption, then turns the page;
//! Esc skips the scene.
//!
//! The story is our own (the original's art and words aren't used): the
//! Hypno Cube hypnotises the land's creatures, and Bounce, too busy
//! bouncing to look into its spiral, sets off to free them.

use crate::app::MapLevel;

/// Caption width in characters (the 5x7 font in a 200 px box).
pub const LINE_CHARS: usize = 32;
/// Letters typed per step.
pub const TYPE_SPEED: usize = 1;
/// Steps a page turn takes.
pub const TURN: u32 = 14;

/// What a panel shows (drawn by the renderer from game sprites).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    /// Bounce and friends playing in the meadow.
    Meadow,
    /// The cube descending from a night sky.
    CubeArrives,
    /// Creatures with spinning spiral eyes.
    Hypnotised,
    /// Bounce rolling off along the path.
    SetsOff,
    /// Dark cave with drips and glowing gems.
    Cave,
    /// Gloomy castle with the spiral glowing in its tower.
    Castle,
    /// The cube face to face with Bounce.
    Showdown,
    /// The cube shattering into sparkles.
    Shatter,
    /// Freed creatures celebrating around Bounce.
    Celebration,
    /// Plain card for the credits.
    Credits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub picture: Picture,
    pub caption: &'static str,
}

/// Scene names.
pub const SCENES: &[&str] = &[
    "intro",
    "world_cave",
    "world_castle",
    "world_boss",
    "ending",
];

/// The panels of a scene.
pub fn scene(name: &str) -> Option<Vec<Panel>> {
    let p = |picture, caption| Panel { picture, caption };
    use Picture::*;
    Some(match name {
        "intro" => vec![
            p(
                Meadow,
                "IN A GREEN AND BOUNCY LAND, BOUNCE THE RED BALL PLAYED ALL DAY WITH FRIENDS.",
            ),
            p(
                CubeArrives,
                "ONE NIGHT A STRANGE CUBE FLOATED DOWN FROM THE SKY. ITS FACE SPUN AND SPUN...",
            ),
            p(
                Hypnotised,
                "ONE BY ONE THE CREATURES STARED INTO THE SPIRAL AND FELL UNDER ITS SPELL.",
            ),
            p(
                SetsOff,
                "BOUNCE WAS TOO BUSY BOUNCING TO LOOK. NOW IT IS UP TO BOUNCE TO SET EVERYONE FREE!",
            ),
        ],
        "world_cave" => vec![
            p(
                Cave,
                "THE TRAIL OF SPIRALS LEADS DOWN INTO DARK, DRIPPING CAVES.",
            ),
            p(Cave, "MIND THE WATER AND THE SHARP ROCKS. FOLLOW THE GLOW!"),
        ],
        "world_castle" => vec![p(
            Castle,
            "BEYOND THE CAVES STANDS A GLOOMY CASTLE. THE SPIRAL LIGHT SHINES FROM ITS TOWER.",
        )],
        "world_boss" => vec![
            p(Showdown, "AT LAST: THE HYPNO CUBE ITSELF!"),
            p(
                Showdown,
                "WHEN IT SHAKES, GET OUT FROM UNDER IT. WHILE IT IS DIZZY, BOUNCE ON ITS HEAD!",
            ),
        ],
        "ending" => vec![
            p(
                Shatter,
                "WITH ONE LAST BOUNCE THE CUBE BURST INTO A THOUSAND SPARKLES.",
            ),
            p(
                Celebration,
                "THE SPELL WAS BROKEN. EVERY CREATURE WOKE UP SMILING.",
            ),
            p(
                Celebration,
                "AND BOUNCE? STILL BOUNCING, OF COURSE.  THE END",
            ),
            p(
                Credits,
                "BOUNCE TALES - A FAN REMAKE. MADE WITH RUST AND MACROQUAD. THANKS FOR PLAYING!",
            ),
        ],
        _ => return None,
    })
}

/// The scene to show after finishing campaign level `i`: the ending after
/// the last one, or the next world's opening when the world changes.
pub fn after_level(levels: &[MapLevel], i: usize) -> Option<&'static str> {
    if i + 1 >= levels.len() {
        return (i + 1 == levels.len()).then_some("ending");
    }
    let (now, next) = (&levels[i].music, &levels[i + 1].music);
    if now == next {
        return None;
    }
    match next.as_str() {
        "cave" => Some("world_cave"),
        "castle" => Some("world_castle"),
        "boss" => Some("world_boss"),
        _ => None,
    }
}

/// Splits `text` into lines of at most `width` characters at spaces.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        let extra = if line.is_empty() { 0 } else { 1 };
        if !line.is_empty() && line.chars().count() + extra + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        } else if extra == 1 {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A scene being played.
#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub name: &'static str,
    pub panels: Vec<Panel>,
    pub page: usize,
    /// Letters of the caption shown so far.
    pub shown: usize,
    /// Steps on this page (picture animation).
    pub ticks: u32,
    /// Page turn in progress: steps left.
    pub turning: u32,
    pub done: bool,
}

/// What a step did (for sounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beat {
    None,
    /// A letter appeared (every few letters, for a soft typing tick).
    Typed,
    /// Next page.
    Turned,
}

impl Player {
    pub fn new(name: &str) -> Option<Player> {
        let name = SCENES.iter().find(|s| **s == name)?;
        Some(Player {
            name,
            panels: scene(name)?,
            page: 0,
            shown: 0,
            ticks: 0,
            turning: 0,
            done: false,
        })
    }

    pub fn panel(&self) -> &Panel {
        &self.panels[self.page]
    }

    fn caption_len(&self) -> usize {
        self.panel().caption.chars().count()
    }

    /// The caption as far as it has been typed, wrapped.
    pub fn lines(&self) -> Vec<String> {
        let typed: String = self.panel().caption.chars().take(self.shown).collect();
        // Wrap the full caption so words don't jump lines while typing.
        let full = wrap(self.panel().caption, LINE_CHARS);
        let mut left = typed.chars().count();
        full.into_iter()
            .map(|l| {
                let n = l.chars().count().min(left);
                left = left.saturating_sub(l.chars().count() + 1);
                l.chars().take(n).collect()
            })
            .collect()
    }

    pub fn fully_typed(&self) -> bool {
        self.shown >= self.caption_len()
    }

    /// Advances one step; `next` = Space/Enter/jump pressed, `skip` = Esc.
    pub fn step(&mut self, next: bool, skip: bool) -> Beat {
        if self.done {
            return Beat::None;
        }
        if skip {
            self.done = true;
            return Beat::None;
        }
        self.ticks += 1;
        if self.turning > 0 {
            self.turning -= 1;
            if self.turning == TURN / 2 {
                self.page += 1;
                self.shown = 0;
                self.ticks = 0;
            }
            return Beat::None;
        }
        if next {
            if !self.fully_typed() {
                self.shown = self.caption_len();
                return Beat::None;
            }
            if self.page + 1 >= self.panels.len() {
                self.done = true;
                return Beat::None;
            }
            self.turning = TURN;
            return Beat::Turned;
        }
        if !self.fully_typed() {
            self.shown += TYPE_SPEED;
            if self.shown.is_multiple_of(3) {
                return Beat::Typed;
            }
        }
        Beat::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping() {
        assert_eq!(wrap("AB CD EF", 5), vec!["AB CD", "EF"]);
        assert_eq!(
            wrap("ABCDEFG H", 5),
            vec!["ABCDEFG", "H"],
            "long words stay whole"
        );
        assert_eq!(wrap("", 5), Vec::<String>::new());
    }

    #[test]
    fn every_scene_fits_its_box() {
        for name in SCENES {
            let panels = scene(name).unwrap();
            assert!(!panels.is_empty());
            for p in panels {
                let lines = wrap(p.caption, LINE_CHARS);
                assert!(lines.len() <= 4, "{name}: {lines:?}");
                assert!(lines.iter().all(|l| l.chars().count() <= LINE_CHARS));
            }
        }
        assert!(scene("nope").is_none());
    }

    #[test]
    fn typing_then_turning_then_done() {
        let mut p = Player::new("world_castle").unwrap();
        assert_eq!(p.lines(), vec!["", "", ""]);
        p.step(false, false);
        assert_eq!(p.shown, 1);
        assert_eq!(p.lines()[0], "B");
        // Space shows the rest at once.
        p.step(true, false);
        assert!(p.fully_typed());
        assert_eq!(p.lines(), wrap(p.panel().caption, LINE_CHARS));
        // Last page: Space ends it.
        p.step(true, false);
        assert!(p.done);
    }

    #[test]
    fn pages_turn_halfway_through_the_wipe() {
        let mut p = Player::new("intro").unwrap();
        p.step(true, false);
        assert_eq!(p.step(true, false), Beat::Turned);
        for _ in 0..TURN / 2 {
            p.step(false, false);
        }
        assert_eq!(p.page, 1);
        assert_eq!(p.shown, 0);
        // Keys are ignored mid-turn.
        for _ in 0..TURN / 2 {
            p.step(true, false);
        }
        assert_eq!(p.page, 1);
        assert!(!p.done);
    }

    #[test]
    fn typing_finishes_by_itself() {
        let mut p = Player::new("world_boss").unwrap();
        for _ in 0..200 {
            p.step(false, false);
        }
        assert!(p.fully_typed());
        assert_eq!(p.page, 0, "never turns without a key");
    }

    #[test]
    fn esc_skips() {
        let mut p = Player::new("ending").unwrap();
        p.step(false, true);
        assert!(p.done);
    }

    fn lvl(music: &str) -> MapLevel {
        MapLevel {
            name: String::new(),
            title: String::new(),
            eggs_total: 0,
            music: music.into(),
        }
    }

    #[test]
    fn scenes_between_worlds_and_at_the_end() {
        let levels = [
            lvl("forest"),
            lvl("forest"),
            lvl("cave"),
            lvl("castle"),
            lvl("boss"),
        ];
        assert_eq!(after_level(&levels, 0), None);
        assert_eq!(after_level(&levels, 1), Some("world_cave"));
        assert_eq!(after_level(&levels, 2), Some("world_castle"));
        assert_eq!(after_level(&levels, 3), Some("world_boss"));
        assert_eq!(after_level(&levels, 4), Some("ending"));
        assert_eq!(after_level(&levels, 9), None);
    }
}
