use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, Fence, Semaphore};

use std::mem::MaybeUninit;
use std::ptr;

impl Semaphore {
    pub fn new(device: &Device) -> Self {
        let create_info = VkSemaphoreCreateInfo {
            sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
            ..Default::default()
        };

        let raw = unsafe {
            let mut semaphore = MaybeUninit::<VkSemaphore>::uninit();

            vkCreateSemaphore(
                device.as_raw(),
                &create_info,
                ptr::null_mut(),
                semaphore.as_mut_ptr(),
            )
            .check_err("create semaphore");

            semaphore.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn as_raw(&self) -> VkSemaphore {
        self.raw
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            vkDestroySemaphore(self.device, self.raw, ptr::null());
        }
    }
}

impl Fence {
    pub fn new(device: &Device, signaled: bool) -> Self {
        let flags = if signaled { VK_FENCE_CREATE_SIGNALED_BIT } else { 0 };

        let create_info = VkFenceCreateInfo {
            sType: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
            flags,
            ..Default::default()
        };

        let raw = unsafe {
            let mut fence = MaybeUninit::<VkFence>::uninit();

            vkCreateFence(device.as_raw(), &create_info, ptr::null_mut(), fence.as_mut_ptr())
                .check_err("create fence");

            fence.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn wait(&self) {
        unsafe {
            vkWaitForFences(self.device, 1, &self.raw, 1, u64::MAX);
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            vkResetFences(self.device, 1, &self.raw);
        }
    }

    pub fn as_raw(&self) -> VkFence {
        self.raw
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            vkDestroyFence(self.device, self.raw, ptr::null());
        }
    }
}
