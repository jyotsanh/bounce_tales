mod render;
mod sound;

use macroquad::prelude::*;

use bounce_tales::app::{App, Hotkey, MenuKeys, UiEvent};
use bounce_tales::audio::sfx::Sfx;
use bounce_tales::cli::{self, CliError, Options};
use bounce_tales::replay::Replay;
use bounce_tales::save::{self, SaveData};
use bounce_tales::sim::{DEFAULT_LEVEL, FixedStep, InputFrame};
use render::{Renderer, SCREEN_H, SCREEN_W};
use sound::Sounds;

/// How long a volume message stays up, frames.
const TOAST_FRAMES: u32 = 75;

fn options() -> Options {
    match Options::parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(CliError::Help) => {
            println!("{}", cli::USAGE);
            std::process::exit(0);
        }
        Err(CliError::Message(m)) => {
            eprintln!("error: {m}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
    }
}

fn window_conf() -> Conf {
    let opts = options();
    Conf {
        window_title: "Bounce Tales".to_owned(),
        window_width: (SCREEN_W * opts.scale) as i32,
        window_height: (SCREEN_H * opts.scale) as i32,
        window_resizable: true,
        high_dpi: false,
        ..Default::default()
    }
}

fn keyboard_input() -> InputFrame {
    let any = |keys: &[KeyCode]| keys.iter().any(|&k| is_key_down(k));
    InputFrame {
        left: any(&[KeyCode::Left, KeyCode::A]),
        right: any(&[KeyCode::Right, KeyCode::D]),
        jump: any(&[KeyCode::Up, KeyCode::W, KeyCode::Space]),
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let opts = options();
    let replay = opts.replay.as_ref().map(|p| {
        Replay::load(p).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(2);
        })
    });

    // A scripted run steps the sim exactly once per rendered frame, so the
    // result never depends on machine speed. A replay alone runs to its end.
    let frame_limit = opts.frames.or_else(|| {
        replay.as_ref().map(|r| {
            let last_check = r.checks.iter().map(|c| c.after).max().unwrap_or(0);
            (r.inputs.len() as u64).max(last_check)
        })
    });

    prevent_quit();
    let mut renderer = Renderer::new();
    // A replay's own `level` line wins over --level. No level at all: the
    // title screen.
    let level = replay
        .as_ref()
        .and_then(|r| r.level.clone())
        .or(opts.level.clone())
        .or_else(|| replay.as_ref().map(|_| DEFAULT_LEVEL.to_owned()));
    let mut app = match &level {
        Some(level) => App::playing(level, replay.is_some()).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::exit(2);
        }),
        None => App::new(),
    };
    // Scripted runs neither read nor write the save, so they never depend
    // on (or change) the player's progress.
    let save_path = if frame_limit.is_some() || opts.no_save {
        None
    } else {
        opts.save.clone().or_else(save::default_path)
    };
    if let Some(path) = &save_path {
        match SaveData::load(path) {
            Ok(data) => {
                app.progress = data.progress;
                app.settings = data.settings;
            }
            Err(e) => {
                // Keep the broken file for inspection; start afresh.
                let bad = path.with_extension("ron.bad");
                eprintln!("warning: {e}; moved it to {}", bad.display());
                let _ = std::fs::rename(path, &bad);
            }
        }
    }
    app.progress.unlock_all = opts.unlock_all;
    if opts.mute {
        app.settings.volume.muted = true;
    }
    let mut sounds = if frame_limit.is_some() {
        Sounds::silent()
    } else {
        Sounds::load(app.settings.volume).await
    };
    let mut fullscreen = false;
    // Menu keys pressed since the last app step (a frame may run no step).
    let mut pending = MenuKeys::NONE;
    // App steps taken.
    let mut step_count: u64 = 0;
    let mut clock = FixedStep::default();
    let mut recorded = Replay::default();
    let mut failures = Vec::new();
    let mut debug = opts.debug;
    // A short message (volume changes), frames left to show it.
    let mut toast: Option<(String, u32)> = None;
    if let (Some(r), Some(g)) = (&replay, &app.game) {
        failures.extend(r.failures_now(&g.sim));
    }

    loop {
        if is_quit_requested() || app.quit {
            break;
        }
        let pressed = |keys: &[KeyCode]| keys.iter().any(|&k| is_key_pressed(k));
        pending.up |= pressed(&[KeyCode::Up, KeyCode::W]);
        pending.down |= pressed(&[KeyCode::Down, KeyCode::S]);
        pending.left |= pressed(&[KeyCode::Left, KeyCode::A]);
        pending.right |= pressed(&[KeyCode::Right, KeyCode::D]);
        pending.confirm |= pressed(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space]);
        pending.back |= pressed(&[KeyCode::Escape, KeyCode::P]);
        if is_key_pressed(KeyCode::F1) {
            debug = !debug;
        }
        let hotkeys = [
            (KeyCode::M, Hotkey::Mute),
            (KeyCode::Minus, Hotkey::MusicDown),
            (KeyCode::Equal, Hotkey::MusicUp),
            (KeyCode::LeftBracket, Hotkey::SfxDown),
            (KeyCode::RightBracket, Hotkey::SfxUp),
            (KeyCode::F11, Hotkey::Fullscreen),
        ];
        if frame_limit.is_none() {
            for (code, key) in hotkeys {
                if is_key_pressed(code) {
                    toast = Some((app.hotkey(key), TOAST_FRAMES));
                    sounds.set_volume(app.settings.volume);
                    if matches!(key, Hotkey::SfxDown | Hotkey::SfxUp) {
                        sounds.play(Sfx::Gem, 1.0); // a sample at the new level
                    }
                }
            }
        }

        let steps = match frame_limit {
            Some(limit) if step_count >= limit => {
                // The previous frame's drawing has been flushed by now.
                if let Some(path) = &opts.screenshot {
                    renderer.save_screenshot(&path.to_string_lossy());
                    println!("screenshot saved to {}", path.display());
                }
                break;
            }
            Some(_) => 1,
            None => clock.advance(get_frame_time()),
        };
        let live = keyboard_input();
        for _ in 0..steps {
            let input = match &replay {
                Some(r) => r.input_at(step_count),
                None => live,
            };
            app.step(input, std::mem::replace(&mut pending, MenuKeys::NONE));
            step_count += 1;
            for e in &app.ui_events {
                sounds.play(
                    match e {
                        UiEvent::Move => Sfx::MenuMove,
                        UiEvent::Select => Sfx::MenuSelect,
                        UiEvent::Back => Sfx::MenuBack,
                    },
                    1.0,
                );
            }
            let Some(game) = app.game.as_ref().filter(|_| app.stepped) else {
                continue;
            };
            renderer.on_step(&game.sim);
            sounds.on_step(&game.sim);
            if opts.record.is_some() {
                recorded.inputs.push(input);
            }
            if let Some(r) = &replay {
                failures.extend(r.failures_now(&game.sim));
            }
        }

        sounds.set_volume(app.settings.volume);
        sounds.update_music(app.music()).await;
        if app.settings.fullscreen != fullscreen {
            fullscreen = app.settings.fullscreen;
            set_fullscreen(fullscreen);
        }
        if app.dirty {
            app.dirty = false;
            if let Some(path) = &save_path {
                let data = SaveData {
                    settings: app.settings,
                    progress: app.progress.clone(),
                    ..SaveData::default()
                };
                if let Err(e) = data.store(path) {
                    eprintln!("warning: {e}");
                }
            }
        }

        let alpha = if frame_limit.is_some() {
            1.0
        } else {
            clock.alpha()
        };
        match &app.game {
            Some(game) if app.shows_game() => {
                renderer.draw_game(&game.sim, alpha);
                renderer.draw_hud(&game.sim);
                renderer.draw_flow(game);
            }
            _ => {}
        }
        renderer.draw_screens(&app);
        if let Some((msg, frames)) = &mut toast {
            renderer.draw_toast(msg);
            *frames -= 1;
            if *frames == 0 {
                toast = None;
            }
        }
        if let Some(game) = app.game.as_ref().filter(|_| debug) {
            renderer.draw_debug(&game.sim, get_fps());
        }
        renderer.present();
        next_frame().await;
    }

    if let Some(path) = &opts.record {
        match std::fs::write(path, recorded.to_text()) {
            Ok(()) => println!("recorded {step_count} steps to {}", path.display()),
            Err(e) => eprintln!("error: could not write {}: {e}", path.display()),
        }
    }
    if replay.is_some() {
        if failures.is_empty() {
            println!("replay: all checks passed");
        } else {
            for f in &failures {
                eprintln!("replay check failed: {f}");
            }
            std::process::exit(1);
        }
    }
}
