use std::{collections::HashMap, rc::Rc, f32::consts::PI};

use winit::event_loop::EventLoop;

use crate::{window::{Dimensions, Window, WindowSettings}, vulkan::{Renderer, Device, Model, GentooRenderError, GameObject, TransformComponent, MAX_FRAMES_IN_FLIGHT, descriptor_set::{DescriptorSetLayout, DescriptorPool, DescriptorSetWriter}, systems::{PointLightSystem, SimpleRenderSystem}, pipeline::PipelineCache, egui::EGuiIntegration, Buffer}, keyboard_movement_controller::KeyboardMovementController, camera::CameraBuilder, FrameInfo, input::Input, GlobalUbo, PointLight, MAX_LIGHTS};

pub struct Application {
    pub window: Window,
    pipeline_cache: Rc<PipelineCache>,
    egui_integration: EGuiIntegration,
    simple_render_system: SimpleRenderSystem,
    point_light_system: PointLightSystem,
    renderer: Renderer,
    game_objects: HashMap<u8, GameObject>,
    viewer_object: GameObject,
    camera_controller: KeyboardMovementController,
    global_pool: Rc<DescriptorPool>,
    global_set_layout: Rc<DescriptorSetLayout>,
    global_descriptor_sets: Vec<ash::vk::DescriptorSet>,
    ubo_buffers: Vec<Buffer<GlobalUbo>>,
    a: glam::Vec3,
}

impl Application {
    pub fn new() -> anyhow::Result<(Self, EventLoop<()>), ApplicationError> {
        let event_loop = EventLoop::new();

        let window = Window::new(
            &event_loop,
            WindowSettings {
                title: "Gentoo",
                dimensions: Dimensions {
                    width: 800, 
                    height: 600,
                },
                resizable: true,
            }
        );

        let device = Device::new(&window.raw_window)?;

        let renderer = Renderer::new(device.clone(), &window)?;

        let global_pool = DescriptorPool::new(device.clone())
            .set_max_sets(MAX_FRAMES_IN_FLIGHT as u32)
            .add_pool_size(ash::vk::DescriptorType::UNIFORM_BUFFER, MAX_FRAMES_IN_FLIGHT as u32)
            .build()?;

        let mut ubo_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let mut buffer = Buffer::new(
                renderer.device.clone(),
                1,
                ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;

            buffer.map(0)?;

            ubo_buffers.push(buffer);
        }

        let global_set_layout = DescriptorSetLayout::new(renderer.device.clone())
            .add_binding(0, ash::vk::DescriptorType::UNIFORM_BUFFER, ash::vk::ShaderStageFlags::ALL_GRAPHICS, 1)
            .build()?;

        let mut global_descriptor_sets = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = ubo_buffers[i].descriptor_info();
            let set = DescriptorSetWriter::new(global_set_layout.clone(), global_pool.clone())
                .write_to_buffer(0, &[buffer_info])
                .build().unwrap();

            global_descriptor_sets.push(set);
        }

        let pipeline_cache = PipelineCache::new(device.clone())?;

        let simple_render_system = SimpleRenderSystem::new(
            device.clone(),
            &renderer.get_swapchain_render_pass(),
            &[global_set_layout.layout],
            &pipeline_cache,
        )?;

        let point_light_system = PointLightSystem::new(
            device.clone(),
            &renderer.get_swapchain_render_pass(),
            &[global_set_layout.layout],
            &pipeline_cache,
        )?;

        let game_objects = Self::load_game_objects(device.clone())?;

        let mut viewer_object = GameObject::new(
            None,
            None,
            None,
        );

        viewer_object.transform.translation.z = -2.5;

        let camera_controller = KeyboardMovementController::new(Some(2.0), Some(2.0));

        let egui_integration = EGuiIntegration::new(
            &window,
            device.clone(),
            &renderer.swapchain,
            renderer.swapchain.swapchain_image_format,
            &pipeline_cache,
        )?;

        let application = Self {
            pipeline_cache,
            egui_integration,
            point_light_system,
            simple_render_system,
            renderer,
            window,
            game_objects,
            viewer_object,
            camera_controller,
            global_pool,
            global_set_layout,
            global_descriptor_sets,
            ubo_buffers,
            a: glam::vec3(0.0, 0.0, 0.0),
        };

        Ok((application, event_loop))
    }

    pub fn update(&mut self, event: &winit::event::WindowEvent) {
        self.egui_integration.on_event(event);
    }

    pub fn run(
        &mut self,
        input: &Input,
        frame_time: f32,
        fps: usize,
    ) -> anyhow::Result<(), ApplicationError> {
        let aspect = self.renderer.get_aspect_ratio();

        self.camera_controller.move_in_plane_xz(
            input,
            frame_time,
            &mut self.viewer_object,
        );

        let camera = CameraBuilder::new()
            .set_view_xyz(
                self.viewer_object.transform.translation,
                self.viewer_object.transform.rotation,
            )
            .set_perspective_projection(50_f32.to_radians(), aspect, 0.1, 100.0)
            .build();

        let extent = Renderer::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return Ok(());
        }

        Ok(match self.renderer.begin_frame(&self.window)? {
            Some(command_buffer) => {
                let frame_index = self.renderer.get_frame_index();
                let frame_info = FrameInfo {
                    frame_index,
                    frame_time,
                    command_buffer,
                    camera,
                    game_objects: &self.game_objects,
                    global_descriptor_set: self.global_descriptor_sets[frame_index],
                };

                // update
                let mut ubo = GlobalUbo {
                    projection: frame_info.camera.projection_matrix,
                    view: frame_info.camera.view_matrix,
                    ambient_light_color: glam::vec4(1.0, 1.0, 1.0, 0.02),
                    point_lights: [PointLight { position: Default::default(), color: Default::default() }; MAX_LIGHTS],
                    num_lights: 0,
                };

                self.point_light_system.update(&frame_info, &mut ubo);

                self.ubo_buffers[frame_index].write_to_buffer(&[ubo]);
                self.ubo_buffers[frame_index].flush()?;

                // render
                self.renderer.begin_swapchain_render_pass(command_buffer);

                self.simple_render_system.render(
                    &frame_info,
                );

                self.point_light_system.render(
                    &frame_info,
                );

                self.renderer.end_swapchain_render_pass(command_buffer);

                self.egui_integration.begin_frame(&self.window);

                egui::TopBottomPanel::top("top_panel").show(&self.egui_integration.egui_ctx, |ui| {
                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Test").clicked() {
                                
                            }
                        });
                    });
                });

                egui::SidePanel::left("my_side_panel").show(&self.egui_integration.egui_ctx, |ui| {
                    ui.heading("Hello");
                    ui.label("Hello egui!");
                    ui.separator();
                    ui.label(format!("FPS: {}", fps));
                });

                let shapes = self.egui_integration.end_frame(&mut self.window);
                let clipped_meshes = self.egui_integration.egui_ctx.tessellate(shapes);

                self.egui_integration
                    .paint(command_buffer, self.renderer.get_image_index(), clipped_meshes)?;

                self.renderer.end_frame()?;
            }
            None => { }
        })
    }

    pub fn resize(&mut self) -> anyhow::Result<(), ApplicationError> {
        self.renderer.recreate_swapchain(&self.window)?;
        self.egui_integration.update_swapchain(&self.window, &self.renderer.swapchain, self.renderer.swapchain.swapchain_image_format, &self.pipeline_cache)?;

        Ok(())
    }

    fn load_game_objects(device: Rc<Device>) -> anyhow::Result<HashMap<u8, GameObject>, GentooRenderError> {
        let mut game_objects = HashMap::new();

        let smooth_vase = Model::from_file(device.clone(), "models/smooth_vase.obj")?;

        let smooth_vase_transform = Some(TransformComponent {
            translation: glam::vec3(0.5, 0.5, -5.0),
            scale: glam::vec3(3.0, 1.5, 3.0),
            rotation: glam::vec3(0.0, 0.0, 0.0),
        });

        let smooth_vase_game_object = GameObject::new(Some(smooth_vase), None, smooth_vase_transform);
        game_objects.insert(smooth_vase_game_object.id, smooth_vase_game_object);

        let flat_vase = Model::from_file(device.clone(), "models/flat_vase.obj")?;

        let flat_vase_transform = Some(TransformComponent {
            translation: glam::vec3(-0.5, 0.5, -5.0),
            scale: glam::vec3(3.0, 1.5, 3.0),
            rotation: glam::vec3(0.0, 0.0, 0.0),
        });

        let flat_vase_game_object = GameObject::new(Some(flat_vase), None, flat_vase_transform);
        game_objects.insert(flat_vase_game_object.id, flat_vase_game_object);

        let floor = Model::from_file(device.clone(), "models/quad.obj")?;

        let floor_transform = Some(TransformComponent {
            translation: glam::vec3(0.0, 0.5, -5.0),
            scale: glam::vec3(10.0, 1.0, 10.0),
            rotation: glam::vec3(0.0, 0.0, 0.0),
        });

        let floor_game_object = GameObject::new(Some(floor), None, floor_transform);
        game_objects.insert(floor_game_object.id, floor_game_object);

        let light_colors = vec![
            glam::vec3(1.0, 0.1, 0.1),
            glam::vec3(0.1, 0.1, 1.0),
            glam::vec3(0.1, 1.0, 0.1),
            glam::vec3(1.0, 1.0, 0.1),
            glam::vec3(0.1, 1.0, 1.0),
            glam::vec3(1.0, 1.0, 1.0),
        ];

        for (i, color) in light_colors.iter().enumerate() {
            let mut point_light = GameObject::make_point_light(0.2, 0.1, *color);

            let rotate_light = glam::Mat4::from_axis_angle(glam::vec3(0.0, -1.0, 0.0), i as f32 * (PI * 2.0) / light_colors.len() as f32);
            let xyz = rotate_light * glam::vec4(-1.0, -1.0, -1.0, 1.0);
            point_light.transform.translation = glam::vec3(xyz.x, xyz.y + 1.0, xyz.z - 5.0);
            game_objects.insert(point_light.id, point_light);
        }

        Ok(game_objects)
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        log::debug!("Dropping application");
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ApplicationError {
    #[error("")]
    VulkanError(#[from] GentooRenderError),
}
