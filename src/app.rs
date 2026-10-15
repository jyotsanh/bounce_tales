//! The screens around the game: title, main menu, how-to-play, world map,
//! pause and game over, with fade transitions between them.
//!
//! Deterministic and input-driven like the sim: [`App::step`] takes the
//! held game input plus edge-triggered menu keys, so menus can be unit
//! tested without a window.

use macroquad::math::{Vec2, vec2};

use crate::audio::Volume;
use crate::game::{self, CAMPAIGN, Game, Mode};
use crate::level::Level;
use crate::progress::Progress;
use crate::save::Settings;
use crate::sim::{Carry, GAME_OVER_TICKS, InputFrame, Sim};

/// Steps for a fade out (and the same again for the fade in).
pub const FADE: u32 = 18;
/// Steps of "GAME OVER" before its menu takes input.
pub const GAME_OVER_MENU_AFTER: u32 = 45;

pub const MAIN_MENU: &[&str] = &["PLAY", "OPTIONS", "HOW TO PLAY", "QUIT"];
pub const PAUSE_MENU: &[&str] = &["RESUME", "RESTART LEVEL", "OPTIONS", "WORLD MAP"];
pub const CONFIRM_MENU: &[&str] = &["NO", "YES"];
pub const GAME_OVER_MENU: &[&str] = &["TRY AGAIN", "WORLD MAP"];

/// Map nodes per row of the winding path.
pub const MAP_ROW: usize = 3;
/// Vertical distance between rows of the path, px.
pub const MAP_ROW_GAP: f32 = 76.0;

/// Menu keys pressed this step (edge-triggered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    /// Esc: pause / resume / go back.
    pub back: bool,
}

impl MenuKeys {
    pub const NONE: MenuKeys = MenuKeys {
        up: false,
        down: false,
        left: false,
        right: false,
        confirm: false,
        back: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Menu {
        sel: usize,
    },
    Help,
    /// World map; `sel` = campaign index under the cursor.
    Map {
        sel: usize,
    },
    Playing,
    Paused {
        sel: usize,
    },
    GameOver {
        sel: usize,
    },
    /// Volumes, fullscreen, reset; opened from the main menu or the pause
    /// menu (then the game shows underneath and RESET is left out).
    Options {
        sel: usize,
        from_pause: bool,
    },
    /// "Erase all progress?" NO / YES.
    ConfirmReset {
        sel: usize,
    },
}

/// One row of the options screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionItem {
    Music,
    Sound,
    Fullscreen,
    Reset,
    Back,
}

impl OptionItem {
    pub fn label(self) -> &'static str {
        match self {
            OptionItem::Music => "MUSIC",
            OptionItem::Sound => "SOUND",
            OptionItem::Fullscreen => "FULLSCREEN",
            OptionItem::Reset => "RESET PROGRESS",
            OptionItem::Back => "BACK",
        }
    }
}

/// Keys that work on every screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hotkey {
    Mute,
    MusicDown,
    MusicUp,
    SfxDown,
    SfxUp,
    Fullscreen,
}

/// What happens at the dark midpoint of a fade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Play one campaign level from the map.
    StartLevel(usize),
    /// Restart the current level as it began (lives and score included).
    RestartLevel,
    ToMap,
    ToMenu,
    Quit,
}

/// Feedback for sound: menu cursor moved, item chosen, back / refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    Move,
    Select,
    Back,
}

/// A campaign level as the world map shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct MapLevel {
    pub name: String,
    pub title: String,
    pub eggs_total: u32,
    /// Its music track (also tints its node: forest, cave, castle, boss).
    pub music: String,
}

impl MapLevel {
    fn load(name: &str) -> Option<MapLevel> {
        let level = Level::load(name).ok()?;
        let (title, music) = (level.title.clone(), level.music.clone());
        let eggs_total = Sim::with_level_carry(level, Carry::default()).eggs_total;
        Some(MapLevel {
            name: name.to_owned(),
            title,
            eggs_total,
            music,
        })
    }
}

/// Position of map node `i` on the winding path, map pixels: the path
/// starts at the bottom (y = 0) and climbs (negative y), snaking left to
/// right and back.
pub fn map_node(i: usize) -> Vec2 {
    const XS: [f32; MAP_ROW] = [48.0, 120.0, 192.0];
    let (row, col) = (i / MAP_ROW, i % MAP_ROW);
    let col = if row % 2 == 0 { col } else { MAP_ROW - 1 - col };
    // The middle node of a row sits a little higher: a gentle wave.
    let lift = if col == 1 { 16.0 } else { 0.0 };
    vec2(XS[col], -(row as f32) * MAP_ROW_GAP - lift)
}

pub struct App {
    pub screen: Screen,
    pub game: Option<Game>,
    /// Fade in progress: steps into it (0..2*FADE) and the action taken at
    /// its midpoint.
    pub fade: Option<(u32, Action)>,
    /// Set once the player chose QUIT and the fade finished.
    pub quit: bool,
    /// Scripted runs (replays): a game over restarts the level by itself
    /// and finished levels go on through the campaign, as the game does,
    /// instead of waiting in menus.
    pub scripted: bool,
    /// Steps since the app started (title animation).
    pub ticks: u64,
    /// Steps on the current screen.
    pub screen_ticks: u32,
    pub ui_events: Vec<UiEvent>,
    /// The game advanced this step (sound/particles/recording follow it).
    pub stepped: bool,
    pub progress: Progress,
    /// The campaign as the map shows it.
    pub levels: Vec<MapLevel>,
    /// Map cursor to come back to.
    pub map_sel: usize,
    /// A level just unlocked (map index): its node flashes.
    pub unlocked_now: Option<usize>,
    /// This level's result is already in `progress`.
    result_recorded: bool,
    prev_jump: bool,
    pub settings: Settings,
    /// Progress or settings changed since the last save.
    pub dirty: bool,
}

impl App {
    /// Starts at the title screen.
    pub fn new() -> App {
        App::with_campaign(CAMPAIGN)
    }

    pub fn with_campaign(campaign: &[&str]) -> App {
        App {
            screen: Screen::Title,
            game: None,
            fade: None,
            quit: false,
            scripted: false,
            ticks: 0,
            screen_ticks: 0,
            ui_events: Vec::new(),
            stepped: false,
            progress: Progress::default(),
            levels: campaign.iter().filter_map(|n| MapLevel::load(n)).collect(),
            map_sel: 0,
            unlocked_now: None,
            result_recorded: false,
            prev_jump: false,
            settings: Settings::default(),
            dirty: false,
        }
    }

    /// Starts straight in a level (`--level`, replays).
    pub fn playing(level: &str, scripted: bool) -> Result<App, String> {
        let mut app = App {
            screen: Screen::Playing,
            game: Some(Game::new(level)?),
            scripted,
            ..App::new()
        };
        app.map_sel = app.level_index(level).unwrap_or(0);
        Ok(app)
    }

    fn names(&self) -> Vec<String> {
        self.levels.iter().map(|l| l.name.clone()).collect()
    }

    fn level_index(&self, name: &str) -> Option<usize> {
        self.levels.iter().position(|l| l.name == name)
    }

    /// Map level `i` can be played.
    pub fn unlocked(&self, i: usize) -> bool {
        self.progress.unlocked(&self.names(), i)
    }

    /// Fade darkness 0..1 (for drawing).
    pub fn fade_alpha(&self) -> f32 {
        match &self.fade {
            Some((t, _)) => 1.0 - (*t as f32 - FADE as f32).abs() / FADE as f32,
            None => 0.0,
        }
    }

    /// Whether the game should be drawn (under any overlay).
    pub fn shows_game(&self) -> bool {
        self.game.is_some()
            && matches!(
                self.screen,
                Screen::Playing
                    | Screen::Paused { .. }
                    | Screen::GameOver { .. }
                    | Screen::Options {
                        from_pause: true,
                        ..
                    }
            )
    }

    /// Music for the current screen.
    pub fn music(&self) -> Option<&str> {
        match (&self.screen, &self.game) {
            (
                Screen::Title
                | Screen::Menu { .. }
                | Screen::Help
                | Screen::Map { .. }
                | Screen::ConfirmReset { .. }
                | Screen::Options {
                    from_pause: false, ..
                },
                _,
            ) => Some("title"),
            (Screen::GameOver { .. }, _) => None,
            (_, Some(g)) => crate::audio::music::for_game(g),
            (_, None) => None,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.screen_ticks = 0;
    }

    fn start_fade(&mut self, action: Action) {
        if self.fade.is_none() {
            self.fade = Some((0, action));
        }
    }

    /// Advances one fixed step.
    pub fn step(&mut self, input: InputFrame, keys: MenuKeys) {
        self.ticks += 1;
        self.screen_ticks = self.screen_ticks.saturating_add(1);
        self.ui_events.clear();
        self.stepped = false;
        let jump_pressed = input.jump && !self.prev_jump;
        self.prev_jump = input.jump;

        if let Some((t, action)) = &mut self.fade {
            *t += 1;
            if *t == FADE {
                let action = action.clone();
                self.perform(action);
            } else if *t >= 2 * FADE {
                self.fade = None;
            }
            // The game keeps running under a fade only while playing.
            if self.screen == Screen::Playing && self.fade.is_some() {
                self.step_game(InputFrame::NONE);
            }
            return;
        }

        match self.screen {
            Screen::Title => {
                if keys.confirm || keys.back || (input.jump && self.screen_ticks > 20) {
                    self.ui_events.push(UiEvent::Select);
                    self.go(Screen::Menu { sel: 0 });
                }
            }
            Screen::Menu { sel } => {
                if keys.back {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Title);
                    return;
                }
                let sel = self.navigate(sel, MAIN_MENU.len(), keys);
                self.screen = Screen::Menu { sel };
                if keys.confirm {
                    self.ui_events.push(UiEvent::Select);
                    match MAIN_MENU[sel] {
                        "PLAY" => {
                            self.map_sel = self.progress.next_level(&self.names());
                            self.start_fade(Action::ToMap);
                        }
                        "HOW TO PLAY" => self.go(Screen::Help),
                        "OPTIONS" => self.go(Screen::Options {
                            sel: 0,
                            from_pause: false,
                        }),
                        _ => self.start_fade(Action::Quit),
                    }
                }
            }
            Screen::Help => {
                if keys.confirm || keys.back {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Menu { sel: 2 });
                }
            }
            Screen::Options { sel, from_pause } => self.step_options(sel, from_pause, keys),
            Screen::ConfirmReset { sel } => {
                let back = Screen::Options {
                    sel: 3,
                    from_pause: false,
                };
                if keys.back {
                    self.ui_events.push(UiEvent::Back);
                    self.go(back);
                    return;
                }
                let sel = self.navigate(sel, CONFIRM_MENU.len(), keys);
                self.screen = Screen::ConfirmReset { sel };
                if keys.confirm {
                    if CONFIRM_MENU[sel] == "YES" {
                        self.ui_events.push(UiEvent::Select);
                        self.progress = Progress {
                            unlock_all: self.progress.unlock_all,
                            ..Progress::default()
                        };
                        self.map_sel = 0;
                        self.dirty = true;
                    } else {
                        self.ui_events.push(UiEvent::Back);
                    }
                    self.go(back);
                }
            }
            Screen::Map { sel } => self.step_map(sel, keys),
            Screen::Playing => {
                if keys.back && !self.scripted {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Paused { sel: 0 });
                    return;
                }
                if !self.scripted && self.leave_finished_level(keys.confirm || jump_pressed) {
                    return;
                }
                self.step_game(input);
                self.record_result();
                let over = self.game.as_ref().map_or(0, |g| g.sim.game_over);
                if !self.scripted && over > 0 && over <= GAME_OVER_TICKS - GAME_OVER_MENU_AFTER {
                    self.go(Screen::GameOver { sel: 0 });
                }
            }
            Screen::Paused { sel } => {
                if keys.back {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Playing);
                    return;
                }
                let sel = self.navigate(sel, PAUSE_MENU.len(), keys);
                self.screen = Screen::Paused { sel };
                if keys.confirm {
                    self.ui_events.push(UiEvent::Select);
                    match PAUSE_MENU[sel] {
                        "RESUME" => self.go(Screen::Playing),
                        "RESTART LEVEL" => self.start_fade(Action::RestartLevel),
                        "OPTIONS" => self.go(Screen::Options {
                            sel: 0,
                            from_pause: true,
                        }),
                        _ => self.start_fade(Action::ToMap),
                    }
                }
            }
            Screen::GameOver { sel } => {
                let sel = self.navigate(sel, GAME_OVER_MENU.len(), keys);
                self.screen = Screen::GameOver { sel };
                if keys.confirm {
                    self.ui_events.push(UiEvent::Select);
                    match GAME_OVER_MENU[sel] {
                        "TRY AGAIN" => self.start_fade(Action::RestartLevel),
                        _ => self.start_fade(Action::ToMap),
                    }
                }
            }
        }
    }

    /// Rows of the options screen.
    pub fn option_items(from_pause: bool) -> Vec<OptionItem> {
        let mut v = vec![OptionItem::Music, OptionItem::Sound, OptionItem::Fullscreen];
        if !from_pause {
            v.push(OptionItem::Reset);
        }
        v.push(OptionItem::Back);
        v
    }

    /// The value shown beside an option.
    pub fn option_value(&self, item: OptionItem) -> String {
        let v = &self.settings.volume;
        let level = |l: u8| {
            if v.muted {
                "MUTED".to_owned()
            } else {
                format!("{l}/{}", Volume::MAX)
            }
        };
        match item {
            OptionItem::Music => level(v.music),
            OptionItem::Sound => level(v.sfx),
            OptionItem::Fullscreen => if self.settings.fullscreen {
                "ON"
            } else {
                "OFF"
            }
            .into(),
            OptionItem::Reset | OptionItem::Back => String::new(),
        }
    }

    fn step_options(&mut self, sel: usize, from_pause: bool, keys: MenuKeys) {
        let items = App::option_items(from_pause);
        let leave = if from_pause {
            Screen::Paused { sel: 2 }
        } else {
            Screen::Menu { sel: 1 }
        };
        if keys.back {
            self.ui_events.push(UiEvent::Back);
            self.go(leave);
            return;
        }
        let sel = self.navigate(sel, items.len(), keys);
        self.screen = Screen::Options { sel, from_pause };
        let side = keys.left || keys.right;
        match items[sel] {
            OptionItem::Music | OptionItem::Sound if side => {
                let v = &mut self.settings.volume;
                let level = if items[sel] == OptionItem::Music {
                    &mut v.music
                } else {
                    &mut v.sfx
                };
                Volume::nudge(level, keys.right);
                v.muted = false;
                self.dirty = true;
                self.ui_events.push(UiEvent::Move);
            }
            OptionItem::Fullscreen if side || keys.confirm => {
                self.settings.fullscreen = !self.settings.fullscreen;
                self.dirty = true;
                self.ui_events.push(UiEvent::Select);
            }
            OptionItem::Reset if keys.confirm => {
                self.ui_events.push(UiEvent::Select);
                self.go(Screen::ConfirmReset { sel: 0 });
            }
            OptionItem::Back if keys.confirm => {
                self.ui_events.push(UiEvent::Back);
                self.go(leave);
            }
            _ => {}
        }
    }

    /// A hotkey pressed anywhere; returns the message to show.
    pub fn hotkey(&mut self, key: Hotkey) -> String {
        let v = &mut self.settings.volume;
        self.dirty = true;
        match key {
            Hotkey::Mute => {
                v.muted = !v.muted;
                if v.muted { "SOUND OFF" } else { "SOUND ON" }.into()
            }
            Hotkey::MusicDown | Hotkey::MusicUp => {
                Volume::nudge(&mut v.music, key == Hotkey::MusicUp);
                v.muted = false;
                format!("MUSIC {}/{}", v.music, Volume::MAX)
            }
            Hotkey::SfxDown | Hotkey::SfxUp => {
                Volume::nudge(&mut v.sfx, key == Hotkey::SfxUp);
                v.muted = false;
                format!("SOUND {}/{}", v.sfx, Volume::MAX)
            }
            Hotkey::Fullscreen => {
                self.settings.fullscreen = !self.settings.fullscreen;
                format!(
                    "FULLSCREEN {}",
                    if self.settings.fullscreen {
                        "ON"
                    } else {
                        "OFF"
                    }
                )
            }
        }
    }

    fn step_map(&mut self, sel: usize, keys: MenuKeys) {
        if keys.back {
            self.ui_events.push(UiEvent::Back);
            self.go(Screen::Menu { sel: 0 });
            return;
        }
        let last = self.levels.len().saturating_sub(1);
        // The path climbs: up/right go on, down/left go back.
        let next = if keys.up || keys.right {
            (sel + 1).min(last)
        } else if keys.down || keys.left {
            sel.saturating_sub(1)
        } else {
            sel
        };
        if next != sel {
            self.ui_events.push(UiEvent::Move);
            self.screen = Screen::Map { sel: next };
            self.map_sel = next;
        }
        if keys.confirm && !self.levels.is_empty() {
            if self.unlocked(next) {
                self.ui_events.push(UiEvent::Select);
                self.start_fade(Action::StartLevel(next));
            } else {
                self.ui_events.push(UiEvent::Back);
            }
        }
    }

    /// Results or "well done" screen dismissed: back to the map. Returns
    /// true if it took the step.
    fn leave_finished_level(&mut self, pressed: bool) -> bool {
        let Some(g) = &self.game else {
            return false;
        };
        let done = match g.mode {
            Mode::Results { ticks, .. } | Mode::Finished { ticks } => ticks >= game::CONTINUE_AFTER,
            Mode::Playing => false,
        };
        if !(done && pressed) {
            return false;
        }
        self.ui_events.push(UiEvent::Select);
        self.start_fade(Action::ToMap);
        true
    }

    /// Puts a just-finished level's result into the progress, once.
    fn record_result(&mut self) {
        let Some(g) = &self.game else {
            return;
        };
        let Mode::Results { result, .. } = &g.mode else {
            return;
        };
        if self.result_recorded {
            return;
        }
        self.result_recorded = true;
        self.dirty = true;
        let result = result.clone();
        let index = self.level_index(&result.level);
        let was_unlocked = index.map(|i| self.unlocked(i + 1));
        self.progress.record(&result);
        if let Some(i) = index {
            let next = (i + 1).min(self.levels.len().saturating_sub(1));
            self.map_sel = next;
            if was_unlocked == Some(false) && i + 1 < self.levels.len() {
                self.unlocked_now = Some(i + 1);
            }
        }
    }

    fn navigate(&mut self, sel: usize, len: usize, keys: MenuKeys) -> usize {
        if keys.up {
            self.ui_events.push(UiEvent::Move);
            (sel + len - 1) % len
        } else if keys.down {
            self.ui_events.push(UiEvent::Move);
            (sel + 1) % len
        } else {
            sel
        }
    }

    fn step_game(&mut self, input: InputFrame) {
        if let Some(g) = &mut self.game {
            g.step(input);
            self.stepped = true;
        }
    }

    fn perform(&mut self, action: Action) {
        match action {
            Action::StartLevel(i) => {
                let Some(name) = self.levels.get(i).map(|l| l.name.clone()) else {
                    return;
                };
                match Game::with_campaign(vec![name.clone()], &name) {
                    Ok(g) => {
                        self.game = Some(g);
                        self.map_sel = i;
                        self.result_recorded = false;
                        self.unlocked_now = None;
                        self.go(Screen::Playing);
                    }
                    Err(_) => self.go(Screen::Map { sel: i }),
                }
            }
            Action::RestartLevel => {
                if let Some(g) = &mut self.game {
                    g.restart_level();
                }
                self.result_recorded = false;
                self.go(Screen::Playing);
            }
            Action::ToMap => {
                self.game = None;
                self.go(Screen::Map { sel: self.map_sel });
            }
            Action::ToMenu => {
                self.game = None;
                self.go(Screen::Menu { sel: 0 });
            }
            Action::Quit => self.quit = true,
        }
    }
}

impl Default for App {
    fn default() -> App {
        App::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIRM: MenuKeys = MenuKeys {
        confirm: true,
        ..MenuKeys::NONE
    };
    const DOWN: MenuKeys = MenuKeys {
        down: true,
        ..MenuKeys::NONE
    };
    const UP: MenuKeys = MenuKeys {
        up: true,
        ..MenuKeys::NONE
    };
    const BACK: MenuKeys = MenuKeys {
        back: true,
        ..MenuKeys::NONE
    };

    fn idle(app: &mut App, steps: u32) {
        for _ in 0..steps {
            app.step(InputFrame::NONE, MenuKeys::NONE);
        }
    }

    fn press(app: &mut App, keys: MenuKeys) {
        app.step(InputFrame::NONE, keys);
    }

    #[test]
    fn title_menu_map_play_fades_into_the_first_level() {
        let mut app = App::new();
        assert_eq!(app.music(), Some("title"));
        press(&mut app, CONFIRM);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
        press(&mut app, CONFIRM);
        assert!(app.fade.is_some());
        assert_eq!(
            app.screen,
            Screen::Menu { sel: 0 },
            "switches at the dark midpoint"
        );
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Map { sel: 0 });
        press(&mut app, CONFIRM);
        idle(&mut app, FADE);
        assert_eq!(app.screen, Screen::Playing);
        assert_eq!(app.game.as_ref().unwrap().level_name, CAMPAIGN[0]);
        assert!(app.fade_alpha() > 0.9);
        idle(&mut app, FADE);
        assert!(app.fade.is_none());
        assert_eq!(app.fade_alpha(), 0.0);
        idle(&mut app, 1);
        assert!(app.stepped);
    }

    #[test]
    fn menu_cursor_wraps() {
        let mut app = App::new();
        press(&mut app, CONFIRM);
        press(&mut app, UP);
        assert_eq!(app.screen, Screen::Menu { sel: 3 });
        press(&mut app, DOWN);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
        assert_eq!(app.ui_events, vec![UiEvent::Move]);
    }

    #[test]
    fn help_and_back() {
        let mut app = App::new();
        press(&mut app, CONFIRM);
        press(&mut app, DOWN);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        assert_eq!(app.screen, Screen::Help);
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Menu { sel: 2 });
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Title);
    }

    #[test]
    fn quit_fades_out_then_quits() {
        let mut app = App::new();
        press(&mut app, CONFIRM);
        press(&mut app, UP);
        press(&mut app, CONFIRM);
        idle(&mut app, FADE - 1);
        assert!(!app.quit);
        idle(&mut app, 1);
        assert!(app.quit);
    }

    #[test]
    fn pause_freezes_the_game_and_resumes() {
        let mut app = App::playing("flat", false).unwrap();
        idle(&mut app, 5);
        let tick = app.game.as_ref().unwrap().sim.tick;
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Paused { sel: 0 });
        idle(&mut app, 30);
        assert_eq!(app.game.as_ref().unwrap().sim.tick, tick, "frozen");
        assert!(!app.stepped);
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Playing);
        idle(&mut app, 1);
        assert_eq!(app.game.as_ref().unwrap().sim.tick, tick + 1);
    }

    #[test]
    fn pause_restart_and_world_map() {
        let mut app = App::playing("flat", false).unwrap();
        for _ in 0..40 {
            app.step(InputFrame::from_code("R").unwrap(), MenuKeys::NONE);
        }
        let start_x = Game::new("flat").unwrap().sim.ball.pos.x;
        assert!(app.game.as_ref().unwrap().sim.ball.pos.x > start_x + 20.0);
        press(&mut app, BACK);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Playing);
        assert_eq!(app.game.as_ref().unwrap().sim.ball.pos.x, start_x);

        press(&mut app, BACK);
        press(&mut app, UP);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Map { sel: 0 });
        assert!(app.game.is_none());
    }

    /// Loses every life on the hazards level by rolling into its spikes.
    fn lose_all_lives(app: &mut App) {
        let g = app.game.as_mut().unwrap();
        g.sim.lives = 1;
        g.sim.die(crate::sim::hazards::Cause::Spikes);
        for _ in 0..600 {
            app.step(InputFrame::NONE, MenuKeys::NONE);
            if !matches!(app.screen, Screen::Playing) {
                break;
            }
        }
    }

    #[test]
    fn game_over_waits_in_a_menu() {
        let mut app = App::playing("flat", false).unwrap();
        lose_all_lives(&mut app);
        assert_eq!(app.screen, Screen::GameOver { sel: 0 });
        assert_eq!(app.music(), None);
        let over = app.game.as_ref().unwrap().sim.game_over;
        idle(&mut app, 500);
        assert_eq!(app.game.as_ref().unwrap().sim.game_over, over, "waits");
        press(&mut app, CONFIRM); // try again
        idle(&mut app, 2 * FADE);
        let g = app.game.as_ref().unwrap();
        assert_eq!(app.screen, Screen::Playing);
        assert_eq!(g.sim.game_over, 0);
        assert_eq!(g.sim.lives, crate::sim::START_LIVES);
    }

    #[test]
    fn game_over_to_world_map() {
        let mut app = App::playing("flat", false).unwrap();
        lose_all_lives(&mut app);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Map { sel: 0 });
    }

    #[test]
    fn scripted_runs_restart_by_themselves_and_ignore_esc() {
        let mut app = App::playing("flat", true).unwrap();
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Playing);
        lose_all_lives(&mut app);
        assert_eq!(app.screen, Screen::Playing);
        idle(&mut app, GAME_OVER_TICKS);
        assert_eq!(app.game.as_ref().unwrap().sim.game_over, 0);
    }

    #[test]
    fn finished_screen_returns_to_the_map() {
        let mut app = App::playing("flat", false).unwrap();
        app.game.as_mut().unwrap().mode = Mode::Finished { ticks: 0 };
        press(&mut app, CONFIRM);
        assert!(app.fade.is_none(), "not before CONTINUE_AFTER");
        idle(&mut app, game::CONTINUE_AFTER);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert!(matches!(app.screen, Screen::Map { .. }));
    }

    const RIGHT: MenuKeys = MenuKeys {
        right: true,
        ..MenuKeys::NONE
    };
    const LEFT: MenuKeys = MenuKeys {
        left: true,
        ..MenuKeys::NONE
    };

    fn on_map(campaign: &[&str]) -> App {
        let mut app = App::with_campaign(campaign);
        app.go(Screen::Map { sel: 0 });
        app
    }

    #[test]
    fn map_cursor_stops_at_the_ends_and_locked_levels_refuse() {
        let mut app = on_map(&["flat", "slopes", "hazards"]);
        assert_eq!(app.levels.len(), 3);
        assert_eq!(app.levels[0].title, "FLAT");
        press(&mut app, DOWN);
        assert_eq!(app.screen, Screen::Map { sel: 0 });
        assert!(app.ui_events.is_empty());
        press(&mut app, RIGHT);
        press(&mut app, UP);
        press(&mut app, UP);
        assert_eq!(app.screen, Screen::Map { sel: 2 });
        press(&mut app, CONFIRM);
        assert!(app.fade.is_none(), "locked");
        assert_eq!(app.ui_events, vec![UiEvent::Back]);
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
    }

    #[test]
    fn unlock_all_opens_every_level() {
        let mut app = on_map(&["flat", "slopes"]);
        app.progress.unlock_all = true;
        press(&mut app, UP);
        press(&mut app, CONFIRM);
        idle(&mut app, FADE);
        assert_eq!(app.game.as_ref().unwrap().level_name, "slopes");
    }

    #[test]
    fn finishing_a_level_records_it_unlocks_the_next_and_returns_to_the_map() {
        let mut app = on_map(&["flat", "slopes"]);
        assert!(!app.unlocked(1));
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        let roll = InputFrame::from_code("R").unwrap();
        for _ in 0..200 {
            app.step(roll, MenuKeys::NONE);
        }
        idle(&mut app, game::COMPLETE_DELAY + game::CONTINUE_AFTER);
        let g = app.game.as_ref().unwrap();
        assert!(matches!(g.mode, Mode::Results { .. }), "{:?}", g.mode);
        assert!(app.progress.get("flat").completed);
        assert!(app.unlocked(1));
        assert_eq!(app.unlocked_now, Some(1));
        app.step(InputFrame::from_code("J").unwrap(), MenuKeys::NONE);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Map { sel: 1 }, "cursor on the next");
        assert!(app.game.is_none());
        assert!(app.progress.total_score() > 0);
    }

    fn to_options() -> App {
        let mut app = App::new();
        press(&mut app, CONFIRM);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        assert_eq!(
            app.screen,
            Screen::Options {
                sel: 0,
                from_pause: false
            }
        );
        app
    }

    #[test]
    fn options_change_volumes_and_fullscreen() {
        let mut app = to_options();
        assert!(!app.dirty);
        press(&mut app, RIGHT);
        assert_eq!(app.settings.volume.music, Volume::DEFAULT.music + 1);
        assert!(app.dirty);
        press(&mut app, DOWN);
        press(&mut app, LEFT);
        assert_eq!(app.settings.volume.sfx, Volume::DEFAULT.sfx - 1);
        assert_eq!(app.option_value(OptionItem::Sound), "7/10");
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        assert!(app.settings.fullscreen);
        assert_eq!(app.option_value(OptionItem::Fullscreen), "ON");
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Menu { sel: 1 });
    }

    #[test]
    fn reset_asks_first() {
        let mut app = to_options();
        app.progress
            .records
            .insert("big".into(), Default::default());
        for _ in 0..3 {
            press(&mut app, DOWN);
        }
        press(&mut app, CONFIRM);
        assert_eq!(app.screen, Screen::ConfirmReset { sel: 0 });
        press(&mut app, CONFIRM); // NO
        assert_eq!(app.progress.records.len(), 1);
        assert!(matches!(app.screen, Screen::Options { sel: 3, .. }));
        press(&mut app, CONFIRM);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM); // YES
        assert!(app.progress.records.is_empty());
        assert!(app.dirty);
    }

    #[test]
    fn options_from_pause_keep_the_game_and_skip_reset() {
        let mut app = App::playing("flat", false).unwrap();
        press(&mut app, BACK);
        press(&mut app, DOWN);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        assert!(matches!(
            app.screen,
            Screen::Options {
                from_pause: true,
                ..
            }
        ));
        assert!(app.shows_game());
        assert_eq!(app.music(), Some("forest"));
        assert!(!App::option_items(true).contains(&OptionItem::Reset));
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Paused { sel: 2 });
    }

    #[test]
    fn hotkeys_change_settings() {
        let mut app = App::new();
        assert_eq!(app.hotkey(Hotkey::MusicDown), "MUSIC 5/10");
        assert_eq!(app.hotkey(Hotkey::Mute), "SOUND OFF");
        assert_eq!(app.option_value(OptionItem::Music), "MUTED");
        assert_eq!(app.hotkey(Hotkey::SfxUp), "SOUND 9/10");
        assert!(!app.settings.volume.muted);
        assert_eq!(app.hotkey(Hotkey::Fullscreen), "FULLSCREEN ON");
        assert!(app.dirty);
    }

    #[test]
    fn map_path_snakes_upward() {
        assert_eq!(map_node(0), vec2(48.0, 0.0));
        assert_eq!(map_node(2).x, 192.0);
        assert_eq!(map_node(3), vec2(192.0, -MAP_ROW_GAP));
        assert_eq!(map_node(5).x, 48.0);
        for i in 1..20 {
            let gap = map_node(i).distance(map_node(i - 1));
            assert!(gap > 50.0 && gap < 100.0, "{i}: {gap}");
        }
    }

    #[test]
    fn campaign_levels_all_appear_on_the_map() {
        let app = App::new();
        assert_eq!(app.levels.len(), CAMPAIGN.len());
        assert_eq!(app.levels.last().unwrap().music, "boss");
    }
}
