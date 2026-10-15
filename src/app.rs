//! The screens around the game: title, main menu, how-to-play, pause and
//! game over, with fade transitions between them.
//!
//! Deterministic and input-driven like the sim: [`App::step`] takes the
//! held game input plus edge-triggered menu keys, so menus can be unit
//! tested without a window.

use crate::game::{self, CAMPAIGN, Game, Mode};
use crate::sim::{GAME_OVER_TICKS, InputFrame};

/// Steps for a fade out (and the same again for the fade in).
pub const FADE: u32 = 18;
/// Steps of "GAME OVER" before its menu takes input.
pub const GAME_OVER_MENU_AFTER: u32 = 45;

pub const MAIN_MENU: &[&str] = &["PLAY", "HOW TO PLAY", "QUIT"];
pub const PAUSE_MENU: &[&str] = &["RESUME", "RESTART LEVEL", "MAIN MENU"];
pub const GAME_OVER_MENU: &[&str] = &["TRY AGAIN", "MAIN MENU"];

/// Menu keys pressed this step (edge-triggered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuKeys {
    pub up: bool,
    pub down: bool,
    pub confirm: bool,
    /// Esc: pause / resume / go back.
    pub back: bool,
}

impl MenuKeys {
    pub const NONE: MenuKeys = MenuKeys {
        up: false,
        down: false,
        confirm: false,
        back: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Menu { sel: usize },
    Help,
    Playing,
    Paused { sel: usize },
    GameOver { sel: usize },
}

/// What happens at the dark midpoint of a fade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Start the campaign from its first level.
    NewGame,
    /// Play one level (and the campaign after it, if it is in it).
    StartLevel(String),
    /// Restart the current level as it began (lives and score included).
    RestartLevel,
    /// Leave the game for the main menu.
    ToMenu,
    Quit,
}

/// Feedback for sound: menu cursor moved, item chosen, pause toggled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiEvent {
    Move,
    Select,
    Back,
}

pub struct App {
    pub screen: Screen,
    pub game: Option<Game>,
    /// Fade in progress: steps into it (0..2*FADE) and the action taken at
    /// its midpoint.
    pub fade: Option<(u32, Action)>,
    /// Set once the player chose QUIT and the fade finished.
    pub quit: bool,
    /// Scripted runs (replays): a game over restarts the level by itself,
    /// as the sim does, instead of waiting in a menu.
    pub scripted: bool,
    /// Steps since the app started (title animation).
    pub ticks: u64,
    /// Steps on the current screen.
    pub screen_ticks: u32,
    pub ui_events: Vec<UiEvent>,
    /// The game advanced this step (sound/particles/recording follow it).
    pub stepped: bool,
}

impl App {
    /// Starts at the title screen.
    pub fn new() -> App {
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
        }
    }

    /// Starts straight in a level (`--level`, replays).
    pub fn playing(level: &str, scripted: bool) -> Result<App, String> {
        Ok(App {
            screen: Screen::Playing,
            game: Some(Game::new(level)?),
            scripted,
            ..App::new()
        })
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
                Screen::Playing | Screen::Paused { .. } | Screen::GameOver { .. }
            )
    }

    /// Music for the current screen.
    pub fn music(&self) -> Option<&str> {
        match (&self.screen, &self.game) {
            (Screen::Title | Screen::Menu { .. } | Screen::Help, _) => Some("title"),
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
                        "PLAY" => self.start_fade(Action::NewGame),
                        "HOW TO PLAY" => self.go(Screen::Help),
                        _ => self.start_fade(Action::Quit),
                    }
                }
            }
            Screen::Help => {
                if keys.confirm || keys.back {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Menu { sel: 1 });
                }
            }
            Screen::Playing => {
                if keys.back && !self.scripted {
                    self.ui_events.push(UiEvent::Back);
                    self.go(Screen::Paused { sel: 0 });
                    return;
                }
                // Past the last level's "well done": back to the menu.
                if let Some(g) = &self.game
                    && matches!(g.mode, Mode::Finished { ticks } if ticks >= game::CONTINUE_AFTER)
                    && (keys.confirm || input.jump)
                    && !self.scripted
                {
                    self.ui_events.push(UiEvent::Select);
                    self.start_fade(Action::ToMenu);
                    return;
                }
                self.step_game(input);
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
                        _ => self.start_fade(Action::ToMenu),
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
                        _ => self.start_fade(Action::ToMenu),
                    }
                }
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
            Action::NewGame => self.perform(Action::StartLevel(CAMPAIGN[0].to_owned())),
            Action::StartLevel(name) => match Game::new(&name) {
                Ok(g) => {
                    self.game = Some(g);
                    self.go(Screen::Playing);
                }
                Err(_) => self.go(Screen::Menu { sel: 0 }),
            },
            Action::RestartLevel => {
                if let Some(g) = &mut self.game {
                    g.restart_level();
                }
                self.go(Screen::Playing);
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
    fn title_menu_play_fades_into_the_first_level() {
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
        assert_eq!(app.screen, Screen::Menu { sel: 2 });
        press(&mut app, DOWN);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
        assert_eq!(app.ui_events, vec![UiEvent::Move]);
    }

    #[test]
    fn help_and_back() {
        let mut app = App::new();
        press(&mut app, CONFIRM);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        assert_eq!(app.screen, Screen::Help);
        press(&mut app, BACK);
        assert_eq!(app.screen, Screen::Menu { sel: 1 });
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
    fn pause_restart_and_main_menu() {
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
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
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
    fn game_over_to_main_menu() {
        let mut app = App::playing("flat", false).unwrap();
        lose_all_lives(&mut app);
        press(&mut app, DOWN);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
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
    fn finished_screen_returns_to_the_menu() {
        let mut app = App::playing("flat", false).unwrap();
        app.game.as_mut().unwrap().mode = Mode::Finished { ticks: 0 };
        press(&mut app, CONFIRM);
        assert!(app.fade.is_none(), "not before CONTINUE_AFTER");
        idle(&mut app, game::CONTINUE_AFTER);
        press(&mut app, CONFIRM);
        idle(&mut app, 2 * FADE);
        assert_eq!(app.screen, Screen::Menu { sel: 0 });
    }
}
