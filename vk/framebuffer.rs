use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, Framebuffer, RenderPass, Swapchain};

use std::mem::MaybeUninit;
use std::ptr;

impl Framebuffer {
    pub fn new(
        device: &Device,
        render_pass: &RenderPass,
        image_view: &VkImageView,
        swapchain: &Swapchain,
    ) -> Self {
        let create_info = VkFramebufferCreateInfo {
            sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            renderPass: render_pass.as_raw(),
            attachmentCount: 1,
            pAttachments: image_view,
            width: swapchain.extent.width,
            height: swapchain.extent.height,
            layers: 1,
            ..Default::default()
        };

        let raw = unsafe {
            let mut fb = MaybeUninit::<VkFramebuffer>::uninit();

            vkCreateFramebuffer(device.as_raw(), &create_info, ptr::null_mut(), fb.as_mut_ptr())
                .check_err("create framebuffer");

            fb.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn as_raw(&self) -> VkFramebuffer {
        self.raw
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            vkDestroyFramebuffer(self.device, self.raw, ptr::null());
        }
    }
}
