use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{CommandPool, Device, QueueFamily};

use std::mem::MaybeUninit;
use std::ptr;

impl CommandPool {
    pub fn new(device: &Device, queue_family: QueueFamily) -> Self {
        let create_info = VkCommandPoolCreateInfo {
            sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
            queueFamilyIndex: device.get_idx_of_queue_family(queue_family).unwrap(),
            ..Default::default()
        };

        let raw = unsafe {
            let mut command_pool = MaybeUninit::<VkCommandPool>::uninit();

            vkCreateCommandPool(
                device.as_raw(),
                &create_info,
                ptr::null(),
                command_pool.as_mut_ptr(),
            )
            .check_err("create command pool");

            command_pool.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn create_command_buffers(&self, count: usize) -> Vec<VkCommandBuffer> {
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

        command_buffers
    }

    pub fn create_command_buffer(&self) -> VkCommandBuffer {
        self.create_command_buffers(1)[0]
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
