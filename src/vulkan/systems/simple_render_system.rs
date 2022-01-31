use std::rc::Rc;

use crate::{vulkan::{GentooRenderError, Device, pipeline::{Pipeline, PipelineCache}, Vertex}, FrameInfo};

#[derive(Debug)]
#[repr(C)]
pub struct SimplePushConstantData {
    model_matrix: glam::Mat4,
    normal_matrix: glam::Mat4,
}

impl SimplePushConstantData {
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let size_in_bytes = std::mem::size_of::<Self>();
        let size_in_u8 = size_in_bytes / std::mem::size_of::<u8>();
        let start_ptr = self as *const Self as *const u8;
        std::slice::from_raw_parts(start_ptr, size_in_u8)
    }
}

pub struct SimpleRenderSystem {
    device: Rc<Device>,
    pipeline: Pipeline,
    pipeline_layout: ash::vk::PipelineLayout,
}

impl SimpleRenderSystem {
    pub fn new(
        device: Rc<Device>,
        render_pass: &ash::vk::RenderPass,
        global_set_layout: &[ash::vk::DescriptorSetLayout],
        pipeline_cache: &Rc<PipelineCache>,
    ) -> anyhow::Result<Self, GentooRenderError> {
        let pipeline_layout = Self::create_pipeline_layout(&device.logical_device, global_set_layout)?;

        let pipeline = Self::create_pipeline(device.clone(), render_pass, &pipeline_layout, pipeline_cache)?;

        Ok(Self {
            device,
            pipeline,
            pipeline_layout,
        })
    }

    fn create_pipeline(
        device: Rc<Device>,
        render_pass: &ash::vk::RenderPass,
        pipeline_layout: &ash::vk::PipelineLayout,
        pipeline_cache: &Rc<PipelineCache>,
    ) -> anyhow::Result<Pipeline, GentooRenderError> {
        assert!(
            pipeline_layout != &ash::vk::PipelineLayout::null(),
            "Cannot create pipeline before pipeline layout"
        );

        Ok(Pipeline::new(
            device,
            "shaders/simple_shader.vert.spv",
            "shaders/simple_shader.frag.spv",
            render_pass,
            pipeline_layout,
            pipeline_cache,
            &Vertex::get_binding_descriptions(),
            &Vertex::get_attribute_descriptions(),
            ash::vk::CullModeFlags::BACK,
        )?)
    }

    fn create_pipeline_layout(
        logical_device: &ash::Device,
        global_set_layout: &[ash::vk::DescriptorSetLayout],
    ) -> anyhow::Result<ash::vk::PipelineLayout, GentooRenderError> {
        let push_constant_range = [ash::vk::PushConstantRange {
            stage_flags: ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
            offset: 0,
            size: std::mem::size_of::<SimplePushConstantData>() as u32,
        }];

        let pipeline_layout_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(global_set_layout)
            .push_constant_ranges(&push_constant_range);

        Ok(unsafe {
            logical_device.create_pipeline_layout(&pipeline_layout_info, None)?
        })
    }

    pub fn render(&self, frame_info: &FrameInfo) {
        unsafe {
            self.pipeline.bind(&self.device.logical_device, frame_info.command_buffer);

            self.device.logical_device.cmd_bind_descriptor_sets(
                frame_info.command_buffer,
                ash::vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[frame_info.global_descriptor_set],
                &[],
            );
        }

        for kv in frame_info.game_objects.iter() {
            let obj = kv.1;

            match &obj.model {
                Some(model) => {
                    let push = SimplePushConstantData {
                        model_matrix: obj.transform.mat4(),
                        normal_matrix: obj.transform.normal_matrix(),
                    };

                    unsafe {
                        let push_ptr = push.as_bytes();

                        self.device.logical_device.cmd_push_constants(
                            frame_info.command_buffer,
                            self.pipeline_layout,
                            ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
                            0,
                            push_ptr,
                        );

                        model.bind(frame_info.command_buffer);
                        model.draw(&self.device.logical_device, frame_info.command_buffer);
                    }
                },
                None => { },
            }
        }
    }
}

impl Drop for SimpleRenderSystem {
    fn drop(&mut self) {
        log::debug!("Dropping simple render system");

        unsafe {
            self.device.logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
