mod cli;
mod render;
mod sim;

use macroquad::prelude::*;

use cli::{CliError, Options};
use render::{Renderer, SCREEN_H, SCREEN_W};
use sim::{FixedStep, Sim};

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

#[macroquad::main(window_conf)]
async fn main() {
    let opts = options();
    if opts.level.is_some() {
        eprintln!("note: --level is ignored until levels exist (step 8)");
    }
    if opts.replay.is_some() {
        eprintln!("note: --replay is ignored until replays exist (step 2)");
    }

    let renderer = Renderer::new();
    let mut sim = Sim::new();
    let mut clock = FixedStep::default();

    loop {
        match opts.frames {
            // Scripted run: exactly one sim step per rendered frame, so the
            // result never depends on how fast the machine is.
            Some(limit) => {
                if sim.tick >= limit {
                    // The previous frame's drawing has been flushed by now.
                    if let Some(path) = &opts.screenshot {
                        renderer.save_screenshot(&path.to_string_lossy());
                        println!("screenshot saved to {}", path.display());
                    }
                    break;
                }
                sim.step();
            }
            None => {
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                for _ in 0..clock.advance(get_frame_time()) {
                    sim.step();
                }
            }
        }

        let alpha = if opts.frames.is_some() {
            1.0
        } else {
            clock.alpha()
        };
        renderer.draw_game(&sim, alpha);
        renderer.present();
        next_frame().await;
    }
}
