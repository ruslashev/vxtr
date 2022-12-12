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

    'main_loop: while window.running {
        for event in window.poll_events() {
            if let Event::KeyPress(Key::Escape) = event {
                break 'main_loop;
            }
        }

        state.present();
    }
}
