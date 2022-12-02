use crate::window::Window;

pub struct State {
    window: Window,
}

impl State {
    pub fn new(window: Window) -> Self {
        let mut inst = Self {
            window,
        };

        inst.init_vulkan();

        inst
    }

    fn init_vulkan(&mut self) {
    }

    pub fn main_loop(&mut self) {
        while self.window.running {
            self.window.poll_events();
        }
    }
}

impl Drop for State {
    fn drop(&mut self) {
    }
}
