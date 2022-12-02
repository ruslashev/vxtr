use window::Window;

mod window;

fn main() {
    let mut window = Window::new(800, 600, "Vulkan tutorial");

    while window.running {
        window.poll_events();
    }
}
