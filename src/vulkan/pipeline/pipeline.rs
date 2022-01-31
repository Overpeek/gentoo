use std::{ffi::CString, rc::Rc};

use crate::vulkan::{ShaderModule, GentooRenderError, Device};

use super::PipelineCache;

pub struct Pipeline {
    device: Rc<Device>,
    pub graphics_pipeline: ash::vk::Pipeline,
    vert_shader_module: Rc<ShaderModule>,
    frag_shader_module: Rc<ShaderModule>,
}

impl Pipeline {
    pub fn new(
        device: Rc<Device>,
        vert_file_path: &str,
        frag_file_path: &str,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
        pipeline_cache: &Rc<PipelineCache>,
        binding_descriptions: &[ash::vk::VertexInputBindingDescription],
        attribute_descriptions: &[ash::vk::VertexInputAttributeDescription],
        cull_mode: ash::vk::CullModeFlags,
    ) -> anyhow::Result<Self, GentooRenderError> {
        let (
            graphics_pipeline,
            vert_shader_module,
            frag_shader_module,
        ) = Self::create_graphics_pipeline(
            &device,
            vert_file_path,
            frag_file_path,
            render_pass,
            pipeline_layout,
            pipeline_cache,
            binding_descriptions,
            attribute_descriptions,
            cull_mode,
        )?;

        Ok(Self {
            device,
            graphics_pipeline,
            vert_shader_module,
            frag_shader_module,
        })
    }

    pub unsafe fn bind(&self, logical_device: &ash::Device, command_buffer: ash::vk::CommandBuffer) {
        logical_device.cmd_bind_pipeline(
            command_buffer,
            ash::vk::PipelineBindPoint::GRAPHICS,
            self.graphics_pipeline,
        );
    }

    fn create_graphics_pipeline(
        device: &Rc<Device>,
        vert_file_path: &str,
        frag_file_path: &str,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
        pipeline_cache: &Rc<PipelineCache>,
        binding_descriptions: &[ash::vk::VertexInputBindingDescription],
        attribute_descriptions: &[ash::vk::VertexInputAttributeDescription],
        cull_mode: ash::vk::CullModeFlags,
    ) -> anyhow::Result<(ash::vk::Pipeline, Rc<ShaderModule>, Rc<ShaderModule>), GentooRenderError> {
        assert_ne!(
            pipeline_layout,
            &ash::vk::PipelineLayout::null(),
            "Cannot create graphics pipeline:: no pipeline_layout provided"
        );

        assert_ne!(
            render_pass,
            &ash::vk::RenderPass::null(),
            "Cannot create graphics pipeline:: no render_pass provided"
        );

        let vert_shader_module = ShaderModule::new(device.clone(), vert_file_path)?;
        let frag_shader_module = ShaderModule::new(device.clone(), frag_file_path)?;

        let entry_point_name = CString::new("main").unwrap();

        let pipeline_info = &[
            ash::vk::GraphicsPipelineCreateInfo::builder()
                .stages(&[
                    ash::vk::PipelineShaderStageCreateInfo {
                        stage: ash::vk::ShaderStageFlags::VERTEX,
                        module: vert_shader_module.module,
                        p_name: entry_point_name.as_ptr() as _,
                        ..Default::default()
                    },
                    ash::vk::PipelineShaderStageCreateInfo {
                        stage: ash::vk::ShaderStageFlags::FRAGMENT,
                        module: frag_shader_module.module,
                        p_name: entry_point_name.as_ptr() as _,
                        ..Default::default()
                    },
                ])
                .vertex_input_state(
                    &ash::vk::PipelineVertexInputStateCreateInfo::builder()
                        .vertex_binding_descriptions(binding_descriptions)
                        .vertex_attribute_descriptions(attribute_descriptions)
                )
                .input_assembly_state(
                    &ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
                        .topology(ash::vk::PrimitiveTopology::TRIANGLE_LIST)
                        .primitive_restart_enable(false)
                )
                .viewport_state(
                    &ash::vk::PipelineViewportStateCreateInfo::builder()
                        .viewport_count(1)
                        .scissor_count(1)
                )
                .rasterization_state(
                    &ash::vk::PipelineRasterizationStateCreateInfo::builder()
                        .depth_clamp_enable(false)
                        .rasterizer_discard_enable(false)
                        .polygon_mode(ash::vk::PolygonMode::FILL)
                        .line_width(1.0)
                        .cull_mode(cull_mode) 
                        .front_face(ash::vk::FrontFace::CLOCKWISE) 
                        .depth_bias_enable(false)
                )
                .multisample_state(
                    &ash::vk::PipelineMultisampleStateCreateInfo::builder()
                        .sample_shading_enable(false)
                        .rasterization_samples(ash::vk::SampleCountFlags::TYPE_1)
                )
                .color_blend_state(
                    &ash::vk::PipelineColorBlendStateCreateInfo::builder()
                        .logic_op_enable(false)
                        .attachments(
                            &[ash::vk::PipelineColorBlendAttachmentState {
                                blend_enable: ash::vk::TRUE,
                                src_color_blend_factor: ash::vk::BlendFactor::ONE,
                                dst_color_blend_factor: ash::vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
                                color_blend_op: ash::vk::BlendOp::ADD,
                                src_alpha_blend_factor: ash::vk::BlendFactor::ONE,
                                dst_alpha_blend_factor: ash::vk::BlendFactor::ZERO,
                                alpha_blend_op: ash::vk::BlendOp::ADD,
                                color_write_mask: ash::vk::ColorComponentFlags::RGBA,
                            }]
                        )
                )
                .depth_stencil_state(
                    &ash::vk::PipelineDepthStencilStateCreateInfo::builder()
                        .depth_write_enable(true)
                        .depth_compare_op(ash::vk::CompareOp::LESS)
                        .depth_test_enable(true)
                        .stencil_test_enable(false)
                )
                .dynamic_state(
                    &ash::vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&[
                        ash::vk::DynamicState::VIEWPORT,
                        ash::vk::DynamicState::SCISSOR,
                    ])
                )
                .layout(*pipeline_layout)
                .render_pass(*render_pass)
                .subpass(0)
                .base_pipeline_index(-1)
                .base_pipeline_handle(ash::vk::Pipeline::null())
                .build()
        ];

        let graphics_pipeline = unsafe {
            device.logical_device
                .create_graphics_pipelines(pipeline_cache.cache, pipeline_info, None)
                .map_err(|e| log::error!("Unable to create graphics pipeline: {:?}", e))
                .unwrap()[0] // fix unwrap?
        };

        Ok((graphics_pipeline, vert_shader_module, frag_shader_module))
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        log::debug!("Dropping pipeline");
        
        unsafe {
            self.device.logical_device.destroy_pipeline(self.graphics_pipeline, None);
        }
    }
}
