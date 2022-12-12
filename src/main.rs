#![allow(clippy::wildcard_imports)]

use state::State;
use window::Window;

mod state;
mod window;

fn main() {
    let mut window = Window::new(800, 600, "Vulkan tutorial");
    let mut state = State::new(&mut window);

    state.main_loop();
}
