use glfw_sys::*;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct State {
    gfx_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: vk::Swapchain,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_buffers: Vec<vk::CommandBuffer>,
    command_pool: vk::CommandPool,
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    is_rendering: Vec<vk::Fence>,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: u32,
    current_frame: usize,
    current_time: f64,

    // Must be last
    device: vk::Device,
    instance: vk::Instance,
}

#[allow(unused)] // False positive
struct PushConstants {
    time: f32,
    res_x: f32,
    res_y: f32,
}

impl State {
    pub fn new(glfw_window: *mut GLFWwindow) -> Self {
        let instance = vk::Instance::new("vxtr", (1, 0, 0), glfw_window);
        let device = vk::Device::new(&instance);
        let gfx_queue = device.get_queue(vk::QueueFamily::Graphics).unwrap();
        let present_queue = device.get_queue(vk::QueueFamily::Present).unwrap();
        let swapchain = device.create_swapchain(&instance, true);
        let image_views = swapchain.get_image_views();
        let render_pass = device.create_render_pass(swapchain.format());
        let pipeline_layout =
            device.create_pipeline_layout::<PushConstants>(VK_SHADER_STAGE_FRAGMENT_BIT);

        let vert_compiled = include_bytes!("../build/shader.vert.spv");
        let frag_compiled = include_bytes!("../build/shader.frag.spv");

        let vert_shader = device.create_shader(vert_compiled, vk::ShaderType::Vertex);
        let frag_shader = device.create_shader(frag_compiled, vk::ShaderType::Fragment);

        let pipeline = device.create_pipeline(
            &[vert_shader, frag_shader],
            &swapchain,
            &render_pass,
            &pipeline_layout,
        );

        let framebuffers = device.create_framebuffers(&render_pass, &image_views, &swapchain);
        let command_pool = device.create_command_pool(vk::QueueFamily::Graphics);
        // must ensure that these can't outlive command_pool
        let command_buffers = command_pool.create_command_buffers(MAX_FRAMES_IN_FLIGHT);

        let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut render_finished = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut is_rendering = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available.push(device.create_semaphore());
            render_finished.push(device.create_semaphore());
            is_rendering.push(device.create_fence(true));
        }

        let vertices: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];

        let vertex_buffer = device.create_buffer_with_data(
            &command_pool,
            &gfx_queue,
            VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
            &vertices,
        );

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let index_buffer = device.create_buffer_with_data(
            &command_pool,
            &gfx_queue,
            VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
            &indices,
        );

        Self {
            instance,
            device,
            gfx_queue,
            present_queue,
            swapchain,
            image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            vertex_buffer,
            index_buffer,
            index_count: indices.len().try_into().unwrap(),
            command_pool,
            command_buffers,
            image_available,
            render_finished,
            is_rendering,
            current_frame: 0,
            current_time: 0.0,
        }
    }

    pub fn present(&mut self) {
        let image_index = {
            self.is_rendering[self.current_frame].wait();

            let mut image_index = 0;

            if self
                .swapchain
                .acquire_next_image(&mut self.image_available[self.current_frame], &mut image_index)
            {
                self.recreate_swapchain();
                return;
            }

            self.is_rendering[self.current_frame].reset();

            image_index
        };

        self.record_commands_to_buffer(image_index as usize);

        self.gfx_queue.submit_wait(
            &self.command_buffers[self.current_frame],
            VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            &self.image_available[self.current_frame],
            &self.render_finished[self.current_frame],
            &self.is_rendering[self.current_frame],
        );

        self.present_queue.present(
            &self.render_finished[self.current_frame],
            &self.swapchain,
            image_index,
        );

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn record_commands_to_buffer(&mut self, image_index: usize) {
        let cmd_buffer = &mut self.command_buffers[self.current_frame];
        let framebuffer = &self.framebuffers[image_index];
        let extent = self.swapchain.extent();

        let clear_color = [0.0, 0.0, 0.0, 1.0];

        let vertex_buffers = [&self.vertex_buffer];
        let offsets = [0];

        // Truncates after ~97 days
        #[allow(clippy::cast_possible_truncation)]
        let time_trunc = self.current_time as f32;

        cmd_buffer.reset();

        cmd_buffer.record(|handle| {
            let push_constants = PushConstants {
                time: time_trunc,
                res_x: vk::utils::u32_to_f32_nowarn(extent.width),
                res_y: vk::utils::u32_to_f32_nowarn(extent.height),
            };

            handle.begin_render_pass(clear_color, &self.render_pass, framebuffer, &self.swapchain);

            handle.bind_pipeline(VK_PIPELINE_BIND_POINT_GRAPHICS, &self.pipeline);

            handle.bind_vertex_buffers(&vertex_buffers, &offsets);

            handle.bind_index_buffer(&self.index_buffer, 0, VK_INDEX_TYPE_UINT16);

            handle.push_constants(
                &self.pipeline_layout,
                VK_SHADER_STAGE_FRAGMENT_BIT,
                0,
                &push_constants,
            );

            handle.draw_indexed(self.index_count);

            handle.end_render_pass();
        });
    }

    fn recreate_swapchain(&mut self) {
        self.device.wait_idle();

        let swapchain = self.device.create_swapchain(&self.instance, false);
        let image_views = swapchain.get_image_views();
        let framebuffers =
            self.device.create_framebuffers(&self.render_pass, &image_views, &swapchain);

        self.swapchain = swapchain;
        self.image_views = image_views;
        self.framebuffers = framebuffers;
    }

    pub fn handle_resize(&mut self, _width: i32, _height: i32) {
        self.recreate_swapchain();
    }

    pub fn update(&mut self, _dt: f64, t: f64) {
        self.current_time = t;
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.device.wait_idle();
    }
}
