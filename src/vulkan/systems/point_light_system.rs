use std::rc::Rc;

use crate::{vulkan::{GentooRenderError, Device, pipeline::{Pipeline, PipelineCache}}, FrameInfo, GlobalUbo, MAX_LIGHTS};

#[derive(Debug)]
#[repr(C)]
struct PointLightPushConstants {
    position: glam::Vec4,
    color: glam::Vec4,
    radius: f32,
}

impl PointLightPushConstants {
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let size_in_bytes = std::mem::size_of::<Self>();
        let size_in_u8 = size_in_bytes / std::mem::size_of::<u8>();
        let start_ptr = self as *const Self as *const u8;
        std::slice::from_raw_parts(start_ptr, size_in_u8)
    }
}

pub struct PointLightSystem {
    device: Rc<Device>,
    pipeline: Pipeline,
    pipeline_layout: ash::vk::PipelineLayout,
}

impl PointLightSystem {
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
            "shaders/point_light.vert.spv",
            "shaders/point_light.frag.spv",
            render_pass,
            pipeline_layout,
            pipeline_cache,
            &[],
            &[],
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
            size: std::mem::size_of::<PointLightPushConstants>() as u32,
        }];

        let pipeline_layout_info = ash::vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(global_set_layout)
            .push_constant_ranges(&push_constant_range);

        Ok(unsafe {
            logical_device.create_pipeline_layout(&pipeline_layout_info, None)?
        })
    }

    pub fn update(&self, frame_info: &FrameInfo, ubo: &mut GlobalUbo) {
        let mut light_index = 0;

        for kv in frame_info.game_objects {
            let obj = kv.1;

            assert!(
                light_index < MAX_LIGHTS,
                "Point lights exceed maximum specified",
            );

            match &obj.point_light {
                Some(point_light) => {
                    ubo.point_lights[light_index].position = glam::vec4(obj.transform.translation.x, obj.transform.translation.y, obj.transform.translation.z, 1.0);
                    ubo.point_lights[light_index].color = glam::vec4(obj.color.x, obj.color.y, obj.color.z, point_light.light_intensity);

                    light_index += 1;
                },
                None => { },
            }
        }

        ubo.num_lights = light_index as u32;
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

            for kv in frame_info.game_objects {
                let obj = kv.1;

                match &obj.point_light {
                    Some(point_light) => {
                        let push = PointLightPushConstants {
                            position: glam::vec4(obj.transform.translation.x, obj.transform.translation.y, obj.transform.translation.z, 1.0),
                            color: glam::vec4(obj.color.x, obj.color.y, obj.color.z, point_light.light_intensity),
                            radius: obj.transform.scale.x,
                        };

                        let push_ptr = push.as_bytes();

                        self.device.logical_device.cmd_push_constants(
                            frame_info.command_buffer,
                            self.pipeline_layout,
                            ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
                            0,
                            push_ptr,
                        );

                        self.device.logical_device.cmd_draw(
                            frame_info.command_buffer,
                            6,
                            1,
                            0,
                            0,
                        )
                    },
                    None => { },
                }
            }
        }
    }
}

impl Drop for PointLightSystem {
    fn drop(&mut self) {
        log::debug!("Dropping point light system");

        unsafe {
            self.device.logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
