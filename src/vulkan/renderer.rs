use std::rc::Rc;

use crate::window::Window;

use super::{Device, Swapchain, GentooRenderError, MAX_FRAMES_IN_FLIGHT};

pub struct Renderer {
    pub device: Rc<Device>,
    pub swapchain: Swapchain,
    command_buffers: Vec<ash::vk::CommandBuffer>,
    current_image_index: usize,
    current_frame_index: usize,
    is_frame_started: bool,
}

impl Renderer {
    pub fn new(device: Rc<Device>, window: &Window) -> anyhow::Result<Self, GentooRenderError> {
        let window_extent = Self::get_window_extent(window);

        let swapchain = Swapchain::new(device.clone(), window_extent, None)?;

        let command_buffers = Self::create_command_buffers(&device.logical_device, device.command_pool)?;

        Ok(Self {
            device,
            swapchain,
            command_buffers,
            current_image_index: 0,
            current_frame_index: 0,
            is_frame_started: false,
        })
    }

    pub fn get_frame_index(&self) -> usize {
        assert!(
            self.is_frame_started,
            "Cannot get frame index when frame is not in progress"
        );

        self.current_frame_index
    }

    pub fn get_image_index(&self) -> usize {
        self.current_image_index
    }

    pub fn get_current_command_buffer(&self) -> ash::vk::CommandBuffer {
        assert!(
            self.is_frame_started,
            "Cannot get command buffer when frame not in progress"
        );

        self.command_buffers[self.current_frame_index]
    }

    pub fn get_swapchain_render_pass(&self) -> ash::vk::RenderPass {
        self.swapchain.render_pass
    }

    pub fn get_aspect_ratio(&self) -> f32 {
        self.swapchain.extent_aspect_ratio()
    }

    pub fn begin_frame(&mut self, window: &Window) -> anyhow::Result<Option<ash::vk::CommandBuffer>, GentooRenderError> {
        assert!(
            !self.is_frame_started,
            "Can't call begin_frame while already in progress"
        );

        let result = unsafe {
            self.swapchain.acquire_next_image(&self.device.logical_device)?
        };

        match result {
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::error!("Out of date KHR!");
                self.recreate_swapchain(window)?;
                return Ok(None);
            }
            Err(_) => {
                log::error!("Unable to acquire next image");
                panic!("Unable to handle this error")
            }
            Ok((current_image_index, _is_subopt)) => {
                // match is_subopt {
                //     true => {
                //         log::warn!("Swapchain is suboptimal for surface");
                //         self.recreate_swapchain(window);
                //     }
                //     false => { }
                // }

                self.is_frame_started = true;
                self.current_image_index = current_image_index as usize;
            }
        }

        let command_buffer = self.get_current_command_buffer();

        let begin_info = ash::vk::CommandBufferBeginInfo::builder();

        unsafe {
            self.device.logical_device.begin_command_buffer(command_buffer, &begin_info)?
        };

        Ok(Some(command_buffer))
    }

    pub fn end_frame(&mut self) -> anyhow::Result<(), GentooRenderError> {
        assert!(
            self.is_frame_started,
            "Can't call end_frame while frame is not in progress"
        );

        let command_buffer = self.get_current_command_buffer();

        unsafe {
            self.device.logical_device.end_command_buffer(command_buffer)?
        };

        let queues = self.device.queue_families.get_queues(&self.device).unwrap();

        self.swapchain.submit_command_buffers(
            &self.device.logical_device,
            &queues.present,
            &queues.graphics,
            command_buffer,
            self.current_image_index,
        )?;

        unsafe {
            self.device.logical_device.device_wait_idle()?
        };

        self.is_frame_started = false;
        self.current_frame_index = (self.current_frame_index + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn begin_swapchain_render_pass(&self, command_buffer: ash::vk::CommandBuffer) {
        assert!(
            self.is_frame_started,
            "Can't call begin_swpachain_render_pass while frame is not in progress"
        );

        assert_eq!(
            command_buffer,
            self.get_current_command_buffer(),
            "Can't begin render pass on a command buffer from a different frame"
        );

        let render_area = ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: self.swapchain.swapchain_extent,
        };

        let color_clear = ash::vk::ClearValue {
            color: ash::vk::ClearColorValue {
                float32: [0.01, 0.01, 0.01, 1.0],
            },
        };

        let depth_clear = ash::vk::ClearValue {
            depth_stencil: ash::vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        };

        let clear_values = [color_clear, depth_clear];

        let render_pass_info = ash::vk::RenderPassBeginInfo::builder()
            .render_pass(self.swapchain.render_pass)
            .framebuffer(self.swapchain.swapchain_framebuffers[self.current_image_index])
            .render_area(render_area)
            .clear_values(&clear_values);

        unsafe {
            self.device.logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                ash::vk::SubpassContents::INLINE,
            );

            let viewports = [ash::vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.width() as f32,
                height: self.swapchain.height() as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }];

            let scissors = [ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.swapchain_extent,
            }];

            self.device
                .logical_device
                .cmd_set_viewport(command_buffer, 0, &viewports);
            self.device
                .logical_device
                .cmd_set_scissor(command_buffer, 0, &scissors);
        }
    }

    pub fn end_swapchain_render_pass(&self, command_buffer: ash::vk::CommandBuffer) {
        assert!(
            self.is_frame_started,
            "Can't call end_swpachain_render_pass while frame is not in progress"
        );

        assert_eq!(
            command_buffer,
            self.get_current_command_buffer(),
            "Can't end render pass on a command buffer from a different frame"
        );

        unsafe {
            self.device.logical_device.cmd_end_render_pass(command_buffer);
        }
    }

    pub fn recreate_swapchain(&mut self, window: &Window) -> anyhow::Result<(), GentooRenderError> {
        let extent = Self::get_window_extent(window);

        if extent.width == 0 || extent.height == 0 {
            return Ok(()); // Don't do anything if the window is minimised
        }

        log::debug!("Recreating swapchain");

        unsafe {
            self.device
                .logical_device
                .device_wait_idle()?
        };

        let new_swapchain =
            Swapchain::new(self.device.clone(), extent, self.swapchain.swapchain_khr.take())?;

        self.swapchain.compare_swap_formats(&new_swapchain)?;

        self.swapchain = new_swapchain;

        Ok(())
    }

    pub fn get_window_extent(window: &Window) -> ash::vk::Extent2D {
        let window_inner_size = window.raw_window.inner_size();
        ash::vk::Extent2D {
            width: window_inner_size.width,
            height: window_inner_size.height,
        }
    }

    fn create_command_buffers(
        device: &ash::Device,
        command_pool: ash::vk::CommandPool,
    ) -> anyhow::Result<Vec<ash::vk::CommandBuffer>, GentooRenderError> {
        let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);

        let command_buffers = unsafe {
            device.allocate_command_buffers(&alloc_info)?
        };

        Ok(command_buffers)
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        log::debug!("Dropping renderer");

        unsafe {
            self.device.logical_device.free_command_buffers(self.device.command_pool, &self.command_buffers);
            self.command_buffers.clear();
        }
    }
}
