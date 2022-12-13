#![allow(clippy::wildcard_imports)]

use state::State;
use window::{Resolution, Window};

use crate::window::{Event, Key};

mod state;
mod window;

fn main() {
    let mut window = Window::new(Resolution::Windowed(800, 600), "Vulkan tutorial");
    window.set_callbacks();

    let mut state = State::new(window.as_inner());

    let updates_per_second: i16 = 60;
    let dt = 1.0 / f64::from(updates_per_second);

    let mut current_time = Window::current_time();
    let mut minimized = false;

    let title_update_delay = 0.1;
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

            let title = format!("Vulkan tutorial | Draw = {:05.2} ms, FPS = {:04.0}", draw_ms, fps);

            window.set_title(title);
        }
    }
}
