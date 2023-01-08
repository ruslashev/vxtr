use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::*;

use std::ffi::c_void;
use std::mem::{size_of, MaybeUninit};
use std::ptr;

impl CommandPool {
    pub fn new(device: &Device, queue_family_idx: u32) -> Self {
        let create_info = VkCommandPoolCreateInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: queue_family_idx,
            ..Default::default()
        };

        let raw = unsafe {
            let mut pool = MaybeUninit::<VkCommandPool>::uninit();

            vkCreateCommandPool(device.as_raw(), &create_info, ptr::null(), pool.as_mut_ptr())
                .check_err("create command pool");

            pool.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn create_command_buffers(&self, count: usize) -> Vec<CommandBuffer> {
        let mut command_buffers = Vec::with_capacity(count);
        command_buffers.resize(count, ptr::null_mut());

        let alloc_info = VkCommandBufferAllocateInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            commandPool: self.raw,
            level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: count.try_into().unwrap(),
            ..Default::default()
        };

        unsafe {
            vkAllocateCommandBuffers(self.device, &alloc_info, command_buffers.as_mut_ptr())
                .check_err("allocate command buffer");
        }

        command_buffers.into_iter().map(|raw| CommandBuffer::new(self, raw)).collect()
    }

    pub fn create_command_buffer(&self) -> CommandBuffer {
        let alloc_info = VkCommandBufferAllocateInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            commandPool: self.raw,
            level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
            commandBufferCount: 1,
            ..Default::default()
        };

        let raw = unsafe {
            let mut cmd_buffer = MaybeUninit::<VkCommandBuffer>::uninit();

            vkAllocateCommandBuffers(self.device, &alloc_info, cmd_buffer.as_mut_ptr())
                .check_err("allocate command buffer");

            cmd_buffer.assume_init()
        };

        CommandBuffer::new(self, raw)
    }

    pub fn as_raw(&self) -> VkCommandPool {
        self.raw
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            vkDestroyCommandPool(self.device, self.raw, ptr::null());
        }
    }
}

impl CommandBuffer {
    fn new(pool: &CommandPool, raw: VkCommandBuffer) -> Self {
        Self {
            raw,
            device: pool.device,
            cmd_pool: pool.raw,
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            vkResetCommandBuffer(self.raw, 0);
        }
    }

    pub fn record_with_flags<F>(&mut self, flags: u32, mut closure: F)
    where
        F: FnMut(CommandBufferRecording),
    {
        let begin_info = VkCommandBufferBeginInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            flags,
            ..Default::default()
        };

        let handle = CommandBufferRecording::new(self.raw);

        unsafe {
            vkBeginCommandBuffer(self.raw, &begin_info)
                .check_err("begin recording to command buffer");
        }

        closure(handle);

        unsafe {
            vkEndCommandBuffer(self.raw).check_err("end command buffer recording");
        }
    }

    pub fn record<F>(&mut self, closure: F)
    where
        F: FnMut(CommandBufferRecording),
    {
        self.record_with_flags(0, closure);
    }

    pub fn as_raw(&self) -> VkCommandBuffer {
        self.raw
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            vkFreeCommandBuffers(self.device, self.cmd_pool, 1, &self.raw);
        }
    }
}

impl CommandBufferRecording {
    fn new(cmd_buf: VkCommandBuffer) -> Self {
        Self { cmd_buf }
    }

    pub fn copy_buffer(
        &self,
        src: &Buffer,
        dst: &mut Buffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) {
        let copy_region = VkBufferCopy {
            srcOffset: src_offset,
            dstOffset: dst_offset,
            size,
        };

        unsafe {
            vkCmdCopyBuffer(self.cmd_buf, src.buffer, dst.buffer, 1, &copy_region);
        }
    }

    pub fn copy_buffer_full(&self, src: &Buffer, dst: &mut Buffer, size: u64) {
        self.copy_buffer(src, dst, 0, 0, size);
    }

    pub fn begin_render_pass(
        &self,
        clear_color: [f32; 4],
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        swapchain: &Swapchain,
    ) {
        let clear_color_value = VkClearValue {
            color: VkClearColorValue {
                float32: clear_color,
            },
        };

        let render_pass_info = VkRenderPassBeginInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
            renderPass: render_pass.as_raw(),
            framebuffer: framebuffer.as_raw(),
            renderArea: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: swapchain.extent(),
            },
            clearValueCount: 1,
            pClearValues: &clear_color_value,
            ..Default::default()
        };

        unsafe {
            vkCmdBeginRenderPass(self.cmd_buf, &render_pass_info, VK_SUBPASS_CONTENTS_INLINE);
        }
    }

    pub fn end_render_pass(&self) {
        unsafe {
            vkCmdEndRenderPass(self.cmd_buf);
        }
    }

    pub fn bind_pipeline(&self, bind_point: u32, pipeline: &Pipeline) {
        unsafe {
            vkCmdBindPipeline(self.cmd_buf, bind_point, pipeline.raw);
        }
    }

    pub fn bind_vertex_buffers(&self, buffers: &[&Buffer], offsets: &[u64]) {
        let raw: Vec<VkBuffer> = buffers.iter().map(|buf| buf.buffer).collect();

        unsafe {
            vkCmdBindVertexBuffers(self.cmd_buf, 0, 1, raw.as_ptr(), offsets.as_ptr());
        }
    }

    pub fn bind_index_buffer(&self, buffer: &Buffer, offset: u64, index_type: u32) {
        unsafe {
            vkCmdBindIndexBuffer(self.cmd_buf, buffer.buffer, offset, index_type);
        }
    }

    pub fn push_constants<T>(
        &self,
        pipeline_layout: &PipelineLayout,
        shader_stages: u32,
        offset: u32,
        push_constants: T,
    ) {
        let ptr = ptr::addr_of!(push_constants).cast::<c_void>();
        let size = size_of::<T>().try_into().unwrap();

        unsafe {
            vkCmdPushConstants(self.cmd_buf, pipeline_layout.raw, shader_stages, offset, size, ptr);
        }
    }

    pub fn draw_indexed_instanced(
        &self,
        idx_count: u32,
        inst_count: u32,
        first_idx: u32,
        vert_off: i32,
        first_inst: u32,
    ) {
        unsafe {
            vkCmdDrawIndexed(self.cmd_buf, idx_count, inst_count, first_idx, vert_off, first_inst);
        }
    }

    pub fn draw_indexed(&self, idx_count: u32) {
        unsafe {
            vkCmdDrawIndexed(self.cmd_buf, idx_count, 1, 0, 0, 0);
        }
    }
}
