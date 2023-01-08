use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, Shader, ShaderType};

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;

impl Shader {
    pub fn from_bytes(device: &Device, compiled: &[u8], sh_type: ShaderType) -> Self {
        let module = create_shader_module(device.as_raw(), compiled);
        let entrypoint = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let stage_info = create_shader_stage_info(module, sh_type, entrypoint);

        Self {
            module,
            stage_info,
            device: device.as_raw(),
        }
    }

    pub fn stage_info(&self) -> VkPipelineShaderStageCreateInfo {
        self.stage_info
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            vkDestroyShaderModule(self.device, self.module, ptr::null_mut());
        }
    }
}

fn create_shader_module(device: VkDevice, bytes: &[u8]) -> VkShaderModule {
    let transmuted_copy = pack_to_u32s(bytes);

    let create_info = VkShaderModuleCreateInfo {
        sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        codeSize: bytes.len(),
        pCode: transmuted_copy.as_ptr(),
        ..Default::default()
    };

    unsafe {
        let mut shader_module = MaybeUninit::<VkShaderModule>::uninit();

        vkCreateShaderModule(device, &create_info, ptr::null_mut(), shader_module.as_mut_ptr())
            .check_err("create shader module");

        shader_module.assume_init()
    }
}

fn pack_to_u32s(bytes: &[u8]) -> Vec<u32> {
    assert!(bytes.len() % 4 == 0, "code length must be a multiple of 4");

    bytes
        .chunks_exact(4)
        .map(|chunk| match chunk {
            &[b0, b1, b2, b3] => u32::from_ne_bytes([b0, b1, b2, b3]),
            _ => unreachable!(),
        })
        .collect()
}

fn create_shader_stage_info(
    shader_module: VkShaderModule,
    sh_type: ShaderType,
    entrypoint: &CStr,
) -> VkPipelineShaderStageCreateInfo {
    let stage = match &sh_type {
        ShaderType::Vertex => VK_SHADER_STAGE_VERTEX_BIT,
        ShaderType::Fragment => VK_SHADER_STAGE_FRAGMENT_BIT,
    };

    VkPipelineShaderStageCreateInfo {
        sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        stage,
        module: shader_module,
        pName: entrypoint.as_ptr(),
        ..Default::default()
    }
}
