use glfw_sys::*;

use std::ptr;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

trait CheckVkError {
    fn check_err(self, action: &'static str);
}

pub struct State {
    glfw_window: *mut GLFWwindow,
    instance: vk::Instance,
    device: vk::Device,
    gfx_queue: VkQueue,
    present_queue: VkQueue,
    swapchain: vk::Swapchain,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    is_rendering: Vec<vk::Fence>,
    vertex_buffer: vk::Buffer,
    index_buffer: vk::Buffer,
    index_count: u32,
    current_frame: usize,
    current_time: f64,
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

        let shaders = [vert_shader, frag_shader];

        let pipeline = device.create_pipeline(
            &[vert_shader, frag_shader],
            &swapchain,
            &render_pass,
            &pipeline_layout,
        );

        let framebuffers = device.create_framebuffers(&render_pass, &image_views, &swapchain);
        let command_pool = device.create_command_pool(vk::QueueFamily::Graphics);
        let command_buffers = command_pool.create_command_buffers(MAX_FRAMES_IN_FLIGHT);
        let (image_available, render_finished, is_rendering) = create_sync_objects(&device);

        let vertices: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];

        let vertex_buffer = device.create_buffer_with_data(
            &command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
            &vertices,
        );

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let index_buffer = device.create_buffer_with_data(
            &command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
            &indices,
        );

        Self {
            glfw_window,
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
        let timeout = u64::MAX;

        let command_buffer = self.command_buffers[self.current_frame];
        let image_available = self.image_available[self.current_frame];
        let render_finished = self.render_finished[self.current_frame];
        let is_rendering = self.is_rendering[self.current_frame];

        let image_index = unsafe {
            is_rendering.wait();

            let mut image_index = 0;

            let result = self.swapchain.acquire_next_image(&mut image_available, &mut image_index);

            if result == VK_ERROR_OUT_OF_DATE_KHR {
                self.recreate_swapchain();
                return;
            }

            if result != VK_SUBOPTIMAL_KHR {
                result.check_err("acquire next image");
            }

            is_rendering.reset();

            image_index
        };

        self.record_commands_to_buffer(
            &mut command_buffer,
            &self.framebuffers[image_index as usize],
        );

        let wait_semaphores = [image_available];
        let wait_stages = [VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT];
        let signal_semaphores = [render_finished];

        let submit_info = VkSubmitInfo {
            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: 1,
            pWaitSemaphores: wait_semaphores.as_ptr(),
            pWaitDstStageMask: wait_stages.as_ptr(),
            commandBufferCount: 1,
            pCommandBuffers: &command_buffer,
            signalSemaphoreCount: 1,
            pSignalSemaphores: signal_semaphores.as_ptr(),
            ..Default::default()
        };

        unsafe {
            vkQueueSubmit(self.gfx_queue, 1, &submit_info, is_rendering)
                .check_err("submit to draw queue");
        }

        let swapchains = [self.swapchain];

        let present_info = VkPresentInfoKHR {
            sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            waitSemaphoreCount: 1,
            pWaitSemaphores: signal_semaphores.as_ptr(),
            swapchainCount: 1,
            pSwapchains: swapchains.as_ptr(),
            pImageIndices: &image_index,
            ..Default::default()
        };

        unsafe {
            vkQueuePresentKHR(self.present_queue, &present_info);
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn record_commands_to_buffer(
        &self,
        cmd_buffer: &mut vk::CommandBuffer,
        framebuffer: &vk::Framebuffer,
    ) {
        let clear_color = [0.0, 0.0, 0.0, 1.0];

        let vertex_buffers = [self.vertex_buffer];
        let offsets = [0];

        // Truncates after ~97 days
        #[allow(clippy::cast_possible_truncation)]
        let time_trunc = self.current_time as f32;

        let push_constants = PushConstants {
            time: time_trunc,
            res_x: vk::utils::u32_to_f32_nowarn(self.swapchain.extent().width),
            res_y: vk::utils::u32_to_f32_nowarn(self.swapchain.extent().height),
        };

        cmd_buffer.reset();

        cmd_buffer.record(|handle| {
            handle.begin_render_pass(clear_color, &self.render_pass, framebuffer, &self.swapchain);

            handle.bind_pipeline(VK_PIPELINE_BIND_POINT_GRAPHICS, &self.pipeline);

            handle.bind_vertex_buffers(&vertex_buffers, &offsets);

            handle.bind_index_buffer(&self.index_buffer, 0, VK_INDEX_TYPE_UINT16);

            handle.push_constants(
                &self.pipeline_layout,
                VK_SHADER_STAGE_FRAGMENT_BIT,
                0,
                push_constants,
            );

            handle.draw_indexed(self.index_count);

            handle.end_render_pass();
        });
    }

    fn recreate_swapchain(&mut self) {
        unsafe {
            vkDeviceWaitIdle(self.device);
        }

        self.cleanup_swapchain();

        let (swapchain, image_format, extent) = create_swapchain(
            self.glfw_window,
            self.phys_device,
            self.device,
            self.instance.surface(),
            false,
        );
        let swapchain_images = get_swapchain_images(self.device, swapchain);
        let image_views = create_image_views(self.device, &swapchain_images, image_format);
        let framebuffers = self.device.create_framebuffers(&image_views, &swapchain, &render_pass);

        self.swapchain = swapchain;
        self.extent = extent;
        self.image_views = image_views;
        self.framebuffers = framebuffers;
    }

    fn cleanup_swapchain(&self) {
        unsafe {
            for framebuffer in &self.framebuffers {
                vkDestroyFramebuffer(self.device, *framebuffer, ptr::null());
            }

            for image_view in &self.image_views {
                vkDestroyImageView(self.device, *image_view, ptr::null());
            }

            vkDestroySwapchainKHR(self.device, self.swapchain, ptr::null());
        }
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
        unsafe {
            vkDeviceWaitIdle(self.device);

            for sem in &self.image_available {
                vkDestroySemaphore(self.device, *sem, ptr::null());
            }

            for sem in &self.render_finished {
                vkDestroySemaphore(self.device, *sem, ptr::null());
            }

            for fence in &self.is_rendering {
                vkDestroyFence(self.device, *fence, ptr::null());
            }

            self.cleanup_swapchain();

            vkDestroyBuffer(self.device, self.vertex_buffer, ptr::null());
            vkFreeMemory(self.device, self.vertex_buffer_memory, ptr::null());

            vkDestroyBuffer(self.device, self.index_buffer, ptr::null());
            vkFreeMemory(self.device, self.index_buffer_memory, ptr::null());
        }
    }
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

fn create_sync_objects(
    device: &vk::Device,
) -> (Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>) {
    let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut render_finished = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
    let mut is_rendering = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        image_available.push(device.create_semaphore());
        render_finished.push(device.create_semaphore());
        is_rendering.push(device.create_fence(true));
    }

    (image_available, render_finished, is_rendering)
}
