#![allow(clippy::wildcard_imports)]

use state::State;
use window::Window;

use crate::window::{Event, Key};

mod state;
mod window;

fn main() {
    let mut window = Window::new(800, 600, "Vulkan tutorial");
    window.set_callbacks();

    let mut state = State::new(window.as_inner());

    let mut minimized = false;

    'main_loop: while window.running {
        if minimized {
            Window::block_until_event();
        }

        for event in window.poll_events() {
            match event {
                Event::KeyPress(Key::Escape) => break 'main_loop,
                Event::WindowResize(width, height) => {
                    if width == 0 || height == 0 {
                        minimized = true;
                        continue;
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
