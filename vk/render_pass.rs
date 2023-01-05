use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, RenderPass};

use std::mem::MaybeUninit;
use std::ptr;

impl RenderPass {
    pub fn new(device: &Device, image_format: u32) -> Self {
        let color_attachment = VkAttachmentDescription {
            format: image_format,
            samples: VK_SAMPLE_COUNT_1_BIT,
            loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
            storeOp: VK_ATTACHMENT_STORE_OP_STORE,
            stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
            finalLayout: VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment_ref = VkAttachmentReference {
            attachment: 0,
            layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass_desc = VkSubpassDescription {
            pipelineBindPoint: VK_PIPELINE_BIND_POINT_GRAPHICS,
            colorAttachmentCount: 1,
            pColorAttachments: &color_attachment_ref,
            ..Default::default()
        };

        let subpass_dependency = VkSubpassDependency {
            srcSubpass: VK_SUBPASS_EXTERNAL as u32,
            dstSubpass: 0,
            srcStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            srcAccessMask: 0,
            dstStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
            ..Default::default()
        };

        let create_info = VkRenderPassCreateInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            attachmentCount: 1,
            pAttachments: &color_attachment,
            subpassCount: 1,
            pSubpasses: &subpass_desc,
            dependencyCount: 1,
            pDependencies: &subpass_dependency,
            ..Default::default()
        };

        let raw = unsafe {
            let mut render_pass = MaybeUninit::<VkRenderPass>::uninit();

            vkCreateRenderPass(
                device.as_raw(),
                &create_info,
                ptr::null_mut(),
                render_pass.as_mut_ptr(),
            )
            .check_err("create render pass");

            render_pass.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }

    pub fn as_raw(&self) -> VkRenderPass {
        self.raw
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            vkDestroyRenderPass(self.device, self.raw, ptr::null());
        }
    }
}
