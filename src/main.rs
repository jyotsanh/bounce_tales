mod render;
mod sound;

use macroquad::prelude::*;

use bounce_tales::cli::{self, CliError, Options};
use bounce_tales::game::Game;
use bounce_tales::replay::Replay;
use bounce_tales::sim::{DEFAULT_LEVEL, FixedStep, InputFrame};
use render::{Renderer, SCREEN_H, SCREEN_W};
use sound::Sounds;

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
    let mut sounds = if frame_limit.is_some() {
        Sounds::silent()
    } else {
        Sounds::load(opts.mute).await
    };
    // A replay's own `level` line wins over --level.
    let level = replay
        .as_ref()
        .and_then(|r| r.level.clone())
        .or(opts.level.clone())
        .unwrap_or_else(|| DEFAULT_LEVEL.to_owned());
    let mut game = Game::new(&level).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    // Game steps taken (the sim's own tick pauses on the finished screen).
    let mut step_count: u64 = 0;
    let mut clock = FixedStep::default();
    let mut recorded = Replay::default();
    let mut failures = Vec::new();
    let mut debug = opts.debug;
    if let Some(r) = &replay {
        failures.extend(r.failures_now(&game.sim));
    }

    loop {
        if is_quit_requested() || is_key_pressed(KeyCode::Escape) {
            break;
        }
        if is_key_pressed(KeyCode::F1) {
            debug = !debug;
        }
        if is_key_pressed(KeyCode::M) {
            sounds.muted = !sounds.muted;
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
            game.step(input);
            step_count += 1;
            renderer.on_step(&game.sim);
            sounds.on_step(&game.sim);
            if opts.record.is_some() {
                recorded.inputs.push(input);
            }
            if let Some(r) = &replay {
                failures.extend(r.failures_now(&game.sim));
            }
        }

        let alpha = if frame_limit.is_some() {
            1.0
        } else {
            clock.alpha()
        };
        renderer.draw_game(&game.sim, alpha);
        renderer.draw_hud(&game.sim);
        renderer.draw_flow(&game);
        if debug {
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
