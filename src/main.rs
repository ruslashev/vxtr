#![allow(clippy::wildcard_imports)]

use window::Window;
use state::State;

mod bindings;
mod window;
mod state;

fn main() {
    let window = Window::new(800, 600, "Vulkan tutorial");
    let mut state = State::new(window);

    state.main_loop();
}
