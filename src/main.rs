#![allow(clippy::wildcard_imports)]

use state::State;
use window::Window;

mod state;
mod window;

fn main() {
    let window = Window::new(800, 600, "Vulkan tutorial");
    let mut state = State::new(window);

    state.main_loop();
}
