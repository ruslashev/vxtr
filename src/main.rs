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

        state.present();
    }
}
