use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, PipelineLayout};

use std::mem::{size_of, MaybeUninit};
use std::ptr;

impl PipelineLayout {
    pub fn new<PushConstT>(device: &Device, push_const_stages: u32) -> Self {
        let push_constant_range = VkPushConstantRange {
            stageFlags: push_const_stages,
            offset: 0,
            size: size_of::<PushConstT>().try_into().unwrap(),
        };

        let create_info = VkPipelineLayoutCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            pushConstantRangeCount: 1,
            pPushConstantRanges: &push_constant_range,
            ..Default::default()
        };

        let raw = unsafe {
            let mut layout = MaybeUninit::<VkPipelineLayout>::uninit();

            vkCreatePipelineLayout(
                device.as_raw(),
                &create_info,
                ptr::null_mut(),
                layout.as_mut_ptr(),
            )
            .check_err("create pipeline layout");

            layout.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            vkDestroyPipelineLayout(self.device, self.raw, ptr::null());
        }
    }
}
