use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{CommandBuffer, Device, Fence, Queue, Semaphore, Swapchain};

use std::mem::MaybeUninit;
use std::ptr;

impl Queue {
    pub fn new(device: &Device, family_idx: u32) -> Self {
        let raw = unsafe {
            let mut queue = MaybeUninit::<VkQueue>::uninit();
            vkGetDeviceQueue(device.as_raw(), family_idx, 0, queue.as_mut_ptr());
            queue.assume_init()
        };

        Self { raw }
    }

    pub fn submit_wait(
        &self,
        cmd_buf: &CommandBuffer,
        wait_stage_mask: u32,
        wait_semaphore: &Semaphore,
        signal_semaphore: &Semaphore,
        fence: &Fence,
    ) {
        let submit_info = VkSubmitInfo {
            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            waitSemaphoreCount: 1,
            pWaitSemaphores: &wait_semaphore.as_raw(),
            pWaitDstStageMask: &wait_stage_mask,
            commandBufferCount: 1,
            pCommandBuffers: &cmd_buf.as_raw(),
            signalSemaphoreCount: 1,
            pSignalSemaphores: &signal_semaphore.as_raw(),
            ..Default::default()
        };

        unsafe {
            vkQueueSubmit(self.raw, 1, &submit_info, fence.as_raw()).check_err("submit to queue");
        }
    }

    pub fn submit(&self, cmd_buf: &CommandBuffer) {
        let submit_info = VkSubmitInfo {
            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            commandBufferCount: 1,
            pCommandBuffers: &cmd_buf.as_raw(),
            ..Default::default()
        };

        unsafe {
            vkQueueSubmit(self.raw, 1, &submit_info, ptr::null_mut()).check_err("submit to queue");
        }
    }

    pub fn present(&self, wait_semaphore: &Semaphore, swapchain: &Swapchain, image_idx: u32) {
        let present_info = VkPresentInfoKHR {
            sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
            waitSemaphoreCount: 1,
            pWaitSemaphores: &wait_semaphore.as_raw(),
            swapchainCount: 1,
            pSwapchains: &swapchain.as_raw(),
            pImageIndices: &image_idx,
            ..Default::default()
        };

        unsafe {
            vkQueuePresentKHR(self.raw, &present_info);
        }
    }

    pub fn wait_idle(&self) {
        unsafe {
            vkQueueWaitIdle(self.raw);
        }
    }
}
