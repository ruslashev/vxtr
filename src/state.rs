use glfw_sys::*;

use std::ffi::{c_void, CString};
use std::mem::{size_of, MaybeUninit};
use std::ptr;

const MAX_FRAMES_IN_FLIGHT: usize = 2;

trait CheckVkError {
    fn check_err(self, action: &'static str);
}

pub struct State<'d> {
    glfw_window: *mut GLFWwindow,
    instance: vk::Instance,
    device: vk::Device,
    gfx_queue: VkQueue,
    present_queue: VkQueue,
    swapchain: vk::Swapchain<'d>,
    extent: VkExtent2D,
    image_views: Vec<VkImageView>,
    render_pass: VkRenderPass,
    pipeline_layout: VkPipelineLayout,
    pipeline: VkPipeline,
    framebuffers: Vec<VkFramebuffer>,
    vertex_buffer: VkBuffer,
    vertex_buffer_memory: VkDeviceMemory,
    index_buffer: VkBuffer,
    index_buffer_memory: VkDeviceMemory,
    index_count: u32,
    command_pool: VkCommandPool,
    command_buffers: Vec<VkCommandBuffer>,
    image_available: [VkSemaphore; MAX_FRAMES_IN_FLIGHT],
    render_finished: [VkSemaphore; MAX_FRAMES_IN_FLIGHT],
    is_rendering: [VkFence; MAX_FRAMES_IN_FLIGHT],
    current_frame: usize,
    current_time: f64,
}

#[allow(unused)] // False positive
struct PushConstants {
    time: f32,
    res_x: f32,
    res_y: f32,
}

impl State<'_> {
    pub fn new(glfw_window: *mut GLFWwindow) -> Self {
        let instance = vk::Instance::new("vxtr", (1, 0, 0), glfw_window);
        let device = vk::Device::new(&instance);
        let gfx_queue = device.get_queue(vk::QueueFamily::Graphics).unwrap();
        let present_queue = device.get_queue(vk::QueueFamily::Present).unwrap();
        let swapchain = device.create_swapchain(&instance, true);
        let swapchain_images = swapchain.get_images();
        let image_views = device.create_image_views(&swapchain_images, swapchain.format());
        let render_pass = device.create_render_pass(swapchain.format());
        let pipeline_layout = device.create_pipeline_layout(VK_SHADER_STAGE_FRAGMENT_BIT);
        let pipeline = create_graphics_pipeline(device, extent, render_pass, pipeline_layout);
        let framebuffers = create_framebuffers(device, &image_views, extent, render_pass);
        let command_pool = create_command_pool(device, queue_families.graphics.unwrap());
        let command_buffers = create_command_buffers(device, command_pool, MAX_FRAMES_IN_FLIGHT);
        let (image_available, render_finished, is_rendering) = create_sync_objects(device);

        let vertices: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];

        let (vertex_buffer, vertex_buffer_memory) = create_buffer_of_type(
            phys_device,
            device,
            command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
            &vertices,
        );

        let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

        let (index_buffer, index_buffer_memory) = create_buffer_of_type(
            phys_device,
            device,
            command_pool,
            gfx_queue,
            VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
            &indices,
        );

        Self {
            glfw_window,
            instance,
            phys_device,
            device,
            gfx_queue,
            present_queue,
            swapchain,
            extent,
            image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            framebuffers,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
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
            vkWaitForFences(self.device, 1, &is_rendering, 1, timeout);

            let mut image_index = 0;

            let result = vkAcquireNextImageKHR(
                self.device,
                self.swapchain,
                timeout,
                image_available,
                ptr::null_mut(),
                &mut image_index,
            );

            if result == VK_ERROR_OUT_OF_DATE_KHR {
                self.recreate_swapchain();
                return;
            }

            if result != VK_SUBOPTIMAL_KHR {
                result.check_err("acquire next image");
            }

            vkResetFences(self.device, 1, &is_rendering);

            image_index
        };

        self.record_commands_to_buffer(command_buffer, self.framebuffers[image_index as usize]);

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

    fn record_commands_to_buffer(&self, cmd_buffer: VkCommandBuffer, framebuffer: VkFramebuffer) {
        let begin_info = VkCommandBufferBeginInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            ..Default::default()
        };

        let clear_color = VkClearValue {
            color: VkClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let render_pass_info = VkRenderPassBeginInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            renderPass: self.render_pass,
            framebuffer,
            renderArea: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: self.extent,
            },
            clearValueCount: 1,
            pClearValues: &clear_color,
            ..Default::default()
        };

        let vertex_buffers = [self.vertex_buffer];
        let offsets = [0];

        // Truncates after ~97 days
        #[allow(clippy::cast_possible_truncation)]
        let time_trunc = self.current_time as f32;

        let push_constants = PushConstants {
            time: time_trunc,
            res_x: u32_to_f32_nowarn(self.extent.width),
            res_y: u32_to_f32_nowarn(self.extent.height),
        };
        let push_constants_ptr = ptr::addr_of!(push_constants).cast::<c_void>();
        let push_constants_size = size_of::<PushConstants>().try_into().unwrap();

        unsafe {
            vkResetCommandBuffer(cmd_buffer, 0);

            vkBeginCommandBuffer(cmd_buffer, &begin_info)
                .check_err("begin recording to command buffer");

            vkCmdBeginRenderPass(cmd_buffer, &render_pass_info, VK_SUBPASS_CONTENTS_INLINE);

            vkCmdBindPipeline(cmd_buffer, VK_PIPELINE_BIND_POINT_GRAPHICS, self.pipeline);

            vkCmdBindVertexBuffers(cmd_buffer, 0, 1, vertex_buffers.as_ptr(), offsets.as_ptr());

            vkCmdBindIndexBuffer(cmd_buffer, self.index_buffer, 0, VK_INDEX_TYPE_UINT16);

            vkCmdPushConstants(
                cmd_buffer,
                self.pipeline_layout,
                VK_SHADER_STAGE_FRAGMENT_BIT,
                0,
                push_constants_size,
                push_constants_ptr,
            );

            vkCmdDrawIndexed(cmd_buffer, self.index_count, 1, 0, 0, 0);

            vkCmdEndRenderPass(cmd_buffer);

            vkEndCommandBuffer(cmd_buffer).check_err("end command buffer recording");
        }
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
        let framebuffers = create_framebuffers(self.device, &image_views, extent, self.render_pass);

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

            vkDestroyCommandPool(self.device, self.command_pool, ptr::null());

            self.cleanup_swapchain();

            vkDestroyBuffer(self.device, self.vertex_buffer, ptr::null());
            vkFreeMemory(self.device, self.vertex_buffer_memory, ptr::null());

            vkDestroyBuffer(self.device, self.index_buffer, ptr::null());
            vkFreeMemory(self.device, self.index_buffer_memory, ptr::null());

            vkDestroyPipeline(self.device, self.pipeline, ptr::null());
            vkDestroyPipelineLayout(self.device, self.pipeline_layout, ptr::null());
            vkDestroyRenderPass(self.device, self.render_pass, ptr::null());
        }
    }
}

impl CheckVkError for VkResult {
    fn check_err(self, action: &'static str) {
        assert!(self == VK_SUCCESS, "Failed to {}: err = {}", action, self);
    }
}

fn create_graphics_pipeline(
    device: VkDevice,
    extent: VkExtent2D,
    render_pass: VkRenderPass,
    pipeline_layout: VkPipelineLayout,
) -> VkPipeline {
    let vert_compiled = include_bytes!("../build/shader.vert.spv");
    let frag_compiled = include_bytes!("../build/shader.frag.spv");

    let vert_shader = device.create_shader(vert_compiled, vk::ShaderType::Vertex);
    let frag_shader = device.create_shader(frag_compiled, vk::ShaderType::Fragment);

    let shader_stage_infos = [vert_shader.stage_info(), frag_shader.stage_info()];

    let binding_desc = get_binding_description();
    let attr_desc = get_attribute_description();

    let vertex_input = create_pipeline_vertex_input_info(&binding_desc, &attr_desc);

    let input_assembly = create_pipeline_input_assembly();

    let viewport = create_pipeline_viewport(extent);
    let scissor = create_pipeline_scissor(extent);
    let viewport_state = create_static_viewport_state_info(&viewport, &scissor);

    let rasterizer = create_rasterizer_info();

    let multisampling = create_multisampling_info();

    let disabled_blending = create_disabled_blending_attachment();
    let blending = create_blending_info(&disabled_blending);

    let create_info = VkGraphicsPipelineCreateInfo {
        sType: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
        stageCount: 2,
        pStages: shader_stage_infos.as_ptr(),
        pVertexInputState: &vertex_input,
        pInputAssemblyState: &input_assembly,
        pViewportState: &viewport_state,
        pRasterizationState: &rasterizer,
        pMultisampleState: &multisampling,
        pColorBlendState: &blending,
        layout: pipeline_layout,
        renderPass: render_pass,
        subpass: 0,
        ..Default::default()
    };

    let graphics_pipeline = unsafe {
        let mut pipeline = MaybeUninit::<VkPipeline>::uninit();

        vkCreateGraphicsPipelines(
            device,
            ptr::null_mut(),
            1,
            &create_info,
            ptr::null_mut(),
            pipeline.as_mut_ptr(),
        )
        .check_err("create pipeline");

        pipeline.assume_init()
    };

    graphics_pipeline
}

fn get_binding_description() -> VkVertexInputBindingDescription {
    let vec2_stride = 2 * size_of::<f32>();

    VkVertexInputBindingDescription {
        binding: 0,
        stride: vec2_stride.try_into().unwrap(),
        inputRate: VK_VERTEX_INPUT_RATE_VERTEX,
    }
}

fn get_attribute_description() -> VkVertexInputAttributeDescription {
    VkVertexInputAttributeDescription {
        location: 0,
        binding: 0,
        format: VK_FORMAT_R32G32_SFLOAT,
        offset: 0,
    }
}

fn create_pipeline_vertex_input_info(
    binding_desc: &VkVertexInputBindingDescription,
    attr_desc: &VkVertexInputAttributeDescription,
) -> VkPipelineVertexInputStateCreateInfo {
    VkPipelineVertexInputStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
        vertexBindingDescriptionCount: 1,
        pVertexBindingDescriptions: binding_desc,
        vertexAttributeDescriptionCount: 1,
        pVertexAttributeDescriptions: attr_desc,
        ..Default::default()
    }
}

fn create_pipeline_input_assembly() -> VkPipelineInputAssemblyStateCreateInfo {
    VkPipelineInputAssemblyStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
        primitiveRestartEnable: 0,
        ..Default::default()
    }
}

fn create_pipeline_viewport(extent: VkExtent2D) -> VkViewport {
    VkViewport {
        x: 0.0,
        y: 0.0,
        width: u32_to_f32_nowarn(extent.width),
        height: u32_to_f32_nowarn(extent.height),
        minDepth: 0.0,
        maxDepth: 1.0,
    }
}

#[allow(clippy::cast_precision_loss)]
fn u32_to_f32_nowarn(x: u32) -> f32 {
    let mantissa = x & 0x007f_ffff; // 23 set bits
    mantissa as f32
}

fn create_pipeline_scissor(extent: VkExtent2D) -> VkRect2D {
    VkRect2D {
        offset: VkOffset2D { x: 0, y: 0 },
        extent,
    }
}

fn create_static_viewport_state_info(
    viewport: &VkViewport,
    scissor: &VkRect2D,
) -> VkPipelineViewportStateCreateInfo {
    VkPipelineViewportStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        viewportCount: 1,
        pViewports: viewport as *const VkViewport,
        scissorCount: 1,
        pScissors: scissor as *const VkRect2D,
        ..Default::default()
    }
}

fn create_rasterizer_info() -> VkPipelineRasterizationStateCreateInfo {
    VkPipelineRasterizationStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        depthClampEnable: 0,
        rasterizerDiscardEnable: 0,
        polygonMode: VK_POLYGON_MODE_FILL,
        lineWidth: 1.0,
        cullMode: VK_CULL_MODE_BACK_BIT,
        frontFace: VK_FRONT_FACE_CLOCKWISE,
        depthBiasEnable: 0,
        ..Default::default()
    }
}

fn create_multisampling_info() -> VkPipelineMultisampleStateCreateInfo {
    VkPipelineMultisampleStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        sampleShadingEnable: 0,
        rasterizationSamples: VK_SAMPLE_COUNT_1_BIT,
        minSampleShading: 1.0,
        pSampleMask: ptr::null(),
        alphaToCoverageEnable: 0,
        alphaToOneEnable: 0,
        ..Default::default()
    }
}

fn create_disabled_blending_attachment() -> VkPipelineColorBlendAttachmentState {
    VkPipelineColorBlendAttachmentState {
        colorWriteMask: VK_COLOR_COMPONENT_R_BIT
            | VK_COLOR_COMPONENT_G_BIT
            | VK_COLOR_COMPONENT_B_BIT
            | VK_COLOR_COMPONENT_A_BIT,
        blendEnable: 0,
        ..Default::default()
    }
}

fn create_blending_info(
    attachment: &VkPipelineColorBlendAttachmentState,
) -> VkPipelineColorBlendStateCreateInfo {
    VkPipelineColorBlendStateCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        logicOpEnable: 0,
        attachmentCount: 1,
        pAttachments: attachment,
        ..Default::default()
    }
}

fn create_framebuffers(
    device: VkDevice,
    image_views: &[VkImageView],
    extent: VkExtent2D,
    render_pass: VkRenderPass,
) -> Vec<VkFramebuffer> {
    let mut framebuffers = Vec::with_capacity(image_views.len());

    for image_view in image_views {
        let create_info = VkFramebufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            renderPass: render_pass,
            attachmentCount: 1,
            pAttachments: image_view,
            width: extent.width,
            height: extent.height,
            layers: 1,
            ..Default::default()
        };

        let framebuffer = unsafe {
            let mut fb = MaybeUninit::<VkFramebuffer>::uninit();

            vkCreateFramebuffer(device, &create_info, ptr::null_mut(), fb.as_mut_ptr())
                .check_err("create framebuffer");

            fb.assume_init()
        };

        framebuffers.push(framebuffer);
    }

    framebuffers
}

fn create_command_pool(device: VkDevice, graphics_queue_family: u32) -> VkCommandPool {
    let create_info = VkCommandPoolCreateInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
        flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
        queueFamilyIndex: graphics_queue_family,
        ..Default::default()
    };

    unsafe {
        let mut command_pool = MaybeUninit::<VkCommandPool>::uninit();

        vkCreateCommandPool(device, &create_info, ptr::null(), command_pool.as_mut_ptr())
            .check_err("create command pool");

        command_pool.assume_init()
    }
}

fn create_command_buffers(
    device: VkDevice,
    command_pool: VkCommandPool,
    count: usize,
) -> Vec<VkCommandBuffer> {
    let mut command_buffers = Vec::with_capacity(count);
    command_buffers.resize(count, ptr::null_mut());

    let alloc_info = VkCommandBufferAllocateInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        commandPool: command_pool,
        level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
        commandBufferCount: count.try_into().unwrap(),
        ..Default::default()
    };

    unsafe {
        vkAllocateCommandBuffers(device, &alloc_info, command_buffers.as_mut_ptr())
            .check_err("allocate command buffer");
    }

    command_buffers
}

fn create_sync_objects(
    device: VkDevice,
) -> (
    [VkSemaphore; MAX_FRAMES_IN_FLIGHT],
    [VkSemaphore; MAX_FRAMES_IN_FLIGHT],
    [VkFence; MAX_FRAMES_IN_FLIGHT],
) {
    let mut image_available = [ptr::null_mut(); MAX_FRAMES_IN_FLIGHT];
    let mut render_finished = [ptr::null_mut(); MAX_FRAMES_IN_FLIGHT];
    let mut is_rendering = [ptr::null_mut(); MAX_FRAMES_IN_FLIGHT];

    for i in 0..MAX_FRAMES_IN_FLIGHT {
        image_available[i] = create_semaphore(device);
        render_finished[i] = create_semaphore(device);
        is_rendering[i] = create_fence(device);
    }

    (image_available, render_finished, is_rendering)
}

fn create_semaphore(device: VkDevice) -> VkSemaphore {
    let create_info = VkSemaphoreCreateInfo {
        sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
        ..Default::default()
    };

    unsafe {
        let mut semaphore = MaybeUninit::<VkSemaphore>::uninit();

        vkCreateSemaphore(device, &create_info, ptr::null_mut(), semaphore.as_mut_ptr())
            .check_err("create semaphore");

        semaphore.assume_init()
    }
}

fn create_fence(device: VkDevice) -> VkFence {
    let create_info = VkFenceCreateInfo {
        sType: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
        flags: VK_FENCE_CREATE_SIGNALED_BIT,
        ..Default::default()
    };

    unsafe {
        let mut fence = MaybeUninit::<VkFence>::uninit();

        vkCreateFence(device, &create_info, ptr::null_mut(), fence.as_mut_ptr())
            .check_err("create fence");

        fence.assume_init()
    }
}

fn create_buffer(
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    size: u64,
    usage: u32,
    properties: u32,
) -> (VkBuffer, VkDeviceMemory) {
    let create_info = VkBufferCreateInfo {
        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        size,
        usage,
        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
        ..Default::default()
    };

    let buffer = unsafe {
        let mut buffer = MaybeUninit::<VkBuffer>::uninit();

        vkCreateBuffer(device, &create_info, ptr::null_mut(), buffer.as_mut_ptr())
            .check_err("create buffer");

        buffer.assume_init()
    };

    let mem_requirements = unsafe {
        let mut requirements = MaybeUninit::<VkMemoryRequirements>::uninit();

        vkGetBufferMemoryRequirements(device, buffer, requirements.as_mut_ptr());

        requirements.assume_init()
    };

    let memory_type = find_memory_type(phys_device, mem_requirements.memoryTypeBits, properties)
        .expect("failed to find appropriate memory type");

    let alloc_info = VkMemoryAllocateInfo {
        sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        allocationSize: mem_requirements.size,
        memoryTypeIndex: memory_type,
        ..Default::default()
    };

    let memory = unsafe {
        let mut memory = MaybeUninit::<VkDeviceMemory>::uninit();

        vkAllocateMemory(device, &alloc_info, ptr::null_mut(), memory.as_mut_ptr())
            .check_err("allocate memory");

        memory.assume_init()
    };

    unsafe {
        vkBindBufferMemory(device, buffer, memory, 0);
    }

    (buffer, memory)
}

fn find_memory_type(
    phys_device: VkPhysicalDevice,
    req_type: u32,
    req_properties: u32,
) -> Option<u32> {
    let mem_properties = unsafe {
        let mut properties = MaybeUninit::<VkPhysicalDeviceMemoryProperties>::uninit();
        vkGetPhysicalDeviceMemoryProperties(phys_device, properties.as_mut_ptr());
        properties.assume_init()
    };

    for i in 0..mem_properties.memoryTypeCount {
        if req_type & (1 << i) == 0 {
            continue;
        }

        if mem_properties.memoryTypes[i as usize].propertyFlags & req_properties == 0 {
            continue;
        }

        return Some(i);
    }

    None
}

fn create_buffer_of_type<T: Copy>(
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    command_pool: VkCommandPool,
    queue: VkQueue,
    usage: u32,
    data: &[T],
) -> (VkBuffer, VkDeviceMemory) {
    let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

    let (staging_buffer, staging_memory) = create_buffer(
        phys_device,
        device,
        size_bytes,
        VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT,
    );

    upload_to_buffer_memory(device, staging_memory, data);

    let (buffer, memory) = create_buffer(
        phys_device,
        device,
        size_bytes,
        usage | VK_BUFFER_USAGE_TRANSFER_DST_BIT,
        VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
    );

    copy_buffers(device, command_pool, queue, staging_buffer, buffer, size_bytes);

    unsafe {
        vkDestroyBuffer(device, staging_buffer, ptr::null());
        vkFreeMemory(device, staging_memory, ptr::null());
    }

    (buffer, memory)
}

fn upload_to_buffer_memory<T: Copy>(device: VkDevice, memory: VkDeviceMemory, data: &[T]) {
    let size_bytes: u64 = (data.len() * size_of::<T>()).try_into().unwrap();

    let memory_range = VkMappedMemoryRange {
        sType: VK_STRUCTURE_TYPE_MAPPED_MEMORY_RANGE,
        memory,
        offset: 0,
        size: size_bytes,
        ..Default::default()
    };

    unsafe {
        let ptr: *mut T = ptr::null_mut();
        let mut void_ptr = ptr.cast::<c_void>();

        vkMapMemory(device, memory, 0, size_bytes, 0, &mut void_ptr).check_err("map memory");

        let out_ptr = void_ptr.cast::<T>();

        let slice = std::slice::from_raw_parts_mut(out_ptr, data.len());
        slice.copy_from_slice(data);

        vkFlushMappedMemoryRanges(device, 1, &memory_range).check_err("flush mapped memory");

        vkUnmapMemory(device, memory);
    }
}

fn copy_buffers(
    device: VkDevice,
    command_pool: VkCommandPool,
    queue: VkQueue,
    src: VkBuffer,
    dst: VkBuffer,
    size: u64,
) {
    let cmd_buffer = create_command_buffers(device, command_pool, 1)[0];

    let begin_info = VkCommandBufferBeginInfo {
        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
        ..Default::default()
    };

    let copy_region = VkBufferCopy {
        size,
        ..Default::default()
    };

    let submit_info = VkSubmitInfo {
        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
        commandBufferCount: 1,
        pCommandBuffers: &cmd_buffer,
        ..Default::default()
    };

    unsafe {
        vkBeginCommandBuffer(cmd_buffer, &begin_info);

        vkCmdCopyBuffer(cmd_buffer, src, dst, 1, &copy_region);

        vkEndCommandBuffer(cmd_buffer);

        vkQueueSubmit(queue, 1, &submit_info, ptr::null_mut());
        vkQueueWaitIdle(queue);

        vkFreeCommandBuffers(device, command_pool, 1, &cmd_buffer);
    }
}
