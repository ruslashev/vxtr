#![allow(clippy::wildcard_imports)]

use state::State;
use window::{Resolution, Window};

use crate::window::{Event, Key};

mod state;
mod window;

fn main() {
    let mut window = Window::new(Resolution::Windowed(800, 600), "vxtr");
    window.set_callbacks();

    let mut state = State::new(window.as_inner());

    if is_benchmark_mode() {
        benchmark(window, state);
        return;
    }

    let updates_per_second: i16 = 60;
    let dt = 1.0 / f64::from(updates_per_second);

    let mut current_time = Window::current_time();
    let mut minimized = false;

    let title_update_delay = 0.03;
    let mut next_title_update_time = 0.0;

    'main_loop: while window.running {
        if minimized {
            Window::block_until_event();
        }

        let real_time = Window::current_time();

        while current_time < real_time {
            current_time += dt;
            state.update(dt, current_time);
        }

        for event in window.poll_events() {
            match event {
                Event::KeyPress(Key::Escape) => break 'main_loop,
                Event::WindowResize(width, height) => {
                    if width == 0 || height == 0 {
                        minimized = true;
                        continue 'main_loop;
                    }

                    minimized = false;

                    state.handle_resize(width, height);
                }
                _ => (),
            }
        }

        let draw_start = Window::current_time();

        state.present();

        let frame_end = Window::current_time();

        if frame_end > next_title_update_time {
            next_title_update_time = frame_end + title_update_delay;

            let draw_time = frame_end - draw_start;
            let frame_time = frame_end - real_time;

            let draw_ms = draw_time * 1000.0;
            let fps = 1.0 / frame_time;

            let title = format!("vxtr | draw = {:05.2} ms, FPS = {:04.0}", draw_ms, fps);

            window.set_title(title);
        }
    }
}

fn is_benchmark_mode() -> bool {
    let mut args = std::env::args();

    args.any(|arg| matches!(arg.as_str(), "--benchmark" | "-b"))
}

fn benchmark(window: Window, state: State) {
    let frames = 5_000;

    match get_time_rendering_n_frames(window, state, frames) {
        Some(seconds) => println!("Rendered {} frames in {} seconds", frames, seconds),
        None => println!("Error"),
    }
}

fn get_time_rendering_n_frames(mut window: Window, mut state: State, n: usize) -> Option<f64> {
    let start_time = Window::current_time();

    let updates_per_second: i16 = 60;
    let dt = 1.0 / f64::from(updates_per_second);

    let mut current_frame = 0;
    let mut current_time = Window::current_time();

    while window.running {
        current_frame += 1;

        if current_frame > n {
            break;
        }

        let real_time = Window::current_time();

        while current_time < real_time {
            current_time += dt;
            state.update(dt, current_time);
        }

        for event in window.poll_events() {
            if matches!(event, Event::WindowResize(_, _)) {
                return None;
            }
        }

        state.present();
    }

    let elapsed = Window::current_time() - start_time;

    Some(elapsed)
}
