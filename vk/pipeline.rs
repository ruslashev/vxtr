use glfw_sys::*;

use crate::utils::CheckVkError;
use crate::{Device, Pipeline, PipelineLayout, RenderPass, Shader, Swapchain};

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

    fn as_raw(&self) -> VkPipelineLayout {
        self.raw
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            vkDestroyPipelineLayout(self.device, self.raw, ptr::null());
        }
    }
}

impl Pipeline {
    pub fn new(
        device: &Device,
        shaders: &[Shader],
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        pipeline_layout: &PipelineLayout,
    ) -> Self {
        let shader_stage_infos: Vec<VkPipelineShaderStageCreateInfo> =
            shaders.iter().map(|shader| shader.stage_info()).collect();

        let binding_desc = get_binding_description();
        let attr_desc = get_attribute_description();

        let vertex_input = create_pipeline_vertex_input_info(&binding_desc, &attr_desc);

        let input_assembly = create_pipeline_input_assembly();

        let viewport = create_pipeline_viewport(swapchain.extent);
        let scissor = create_pipeline_scissor(swapchain.extent);
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
            layout: pipeline_layout.as_raw(),
            renderPass: render_pass.as_raw(),
            subpass: 0,
            ..Default::default()
        };

        let raw = unsafe {
            let mut pipeline = MaybeUninit::<VkPipeline>::uninit();

            vkCreateGraphicsPipelines(
                device.as_raw(),
                ptr::null_mut(),
                1,
                &create_info,
                ptr::null_mut(),
                pipeline.as_mut_ptr(),
            )
            .check_err("create pipeline");

            pipeline.assume_init()
        };

        Self {
            raw,
            device: device.as_raw(),
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            vkDestroyPipeline(self.device, self.raw, ptr::null());
        }
    }
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
