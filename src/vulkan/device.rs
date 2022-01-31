use std::{ffi::CStr, rc::Rc};

use super::{GentooRenderError, Instance, ENABLE_VALIDATION_LAYERS, QueueFamilies};

pub struct SwapchainSupportDetails {
    pub capabilities: ash::vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<ash::vk::SurfaceFormatKHR>,
    pub present_modes: Vec<ash::vk::PresentModeKHR>,
}

pub struct Device {
    pub instance: Instance,
    surface: ash::extensions::khr::Surface,
    pub surface_khr: ash::vk::SurfaceKHR,
    physical_device: ash::vk::PhysicalDevice,
    pub properties: ash::vk::PhysicalDeviceProperties,
    pub logical_device: ash::Device,
    pub queue_families: QueueFamilies,
    pub command_pool: ash::vk::CommandPool,
}

impl Device {
    pub fn new(window: &winit::window::Window) -> anyhow::Result<Rc<Self>, GentooRenderError> {
        let instance = Instance::new()?;
        log::debug!("Vulkan Instance created");
        let (surface, surface_khr) = Self::create_surface(&instance, window)?;
        log::debug!("Vulkan Surface created");
        let (physical_device, properties) = Self::pick_physical_device(&instance, &surface, surface_khr)?;
        log::debug!("Vulkan Physical Device created");
        let queue_families = QueueFamilies::new(&instance, &surface, surface_khr, physical_device)?;
        log::debug!("Vulkan Queue Families created");
        let logical_device = Self::create_logical_device(&instance, physical_device, &queue_families)?;
        log::debug!("Vulkan Logical Device created");
        let command_pool = Self::create_command_pool(&logical_device, &queue_families)?;
        log::debug!("Vulkan Command Pool created");

        Ok(Rc::new(Self {
            instance,
            surface,
            surface_khr,
            physical_device,
            properties,
            logical_device,
            queue_families,
            command_pool,
        }))
    }

    pub fn get_swapchain_support(&self) -> anyhow::Result<SwapchainSupportDetails, GentooRenderError> {
        Ok(Self::query_swapchain_support(&self.surface, self.surface_khr, self.physical_device)?)
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        let mem_properties = unsafe {
            self.instance.instance.get_physical_device_memory_properties(self.physical_device)
        };

        let mut memory_type = None;

        for (i, m_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter) & (1 << i) != 0 && (m_type.property_flags & properties) == properties {
                memory_type = Some(i as u32);
                break;
            }
        }

        memory_type
    }

    pub fn find_supported_format(
        &self,
        candidates: &Vec<ash::vk::Format>,
        tiling: ash::vk::ImageTiling,
        features: ash::vk::FormatFeatureFlags,
    ) -> ash::vk::Format {
        *candidates
            .iter()
            .find(|format| {
                let properties = unsafe {
                    self.instance.instance.get_physical_device_format_properties(self.physical_device, **format)
                };

                if tiling == ash::vk::ImageTiling::LINEAR {
                    return (properties.linear_tiling_features & features) == features;
                } else if tiling == ash::vk::ImageTiling::OPTIMAL {
                    return (properties.optimal_tiling_features & features) == features;
                }

                false
            })
            .expect("Failed to find supported format!")
    }

    pub fn create_buffer(
        &self,
        size: ash::vk::DeviceSize,
        usage: ash::vk::BufferUsageFlags,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> anyhow::Result<(ash::vk::Buffer, ash::vk::DeviceMemory), GentooRenderError> {
        let create_info = ash::vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            self.logical_device.create_buffer(&create_info, None)?
        };

        let mem_requirements = unsafe {
            self.logical_device.get_buffer_memory_requirements(buffer)
        };

        let alloc_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(self.find_memory_type(mem_requirements.memory_type_bits, properties)
                .unwrap()
        );

        let buffer_memory = unsafe {
            self.logical_device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            self.logical_device.bind_buffer_memory(buffer, buffer_memory, 0)?
        };

        Ok((buffer, buffer_memory))
    }

    pub fn begin_single_time_commands(&self) -> anyhow::Result<ash::vk::CommandBuffer, GentooRenderError> {
        let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let command_buffer = unsafe {
            self.logical_device.allocate_command_buffers(&alloc_info)?[0]
        };

        let begin_info = ash::vk::CommandBufferBeginInfo::builder()
            .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.logical_device.begin_command_buffer(command_buffer, &begin_info)?
        };

        Ok(command_buffer)
    }

    pub fn end_single_time_commands(&self, command_buffer: ash::vk::CommandBuffer) -> anyhow::Result<(), GentooRenderError> {
        unsafe {
            self.logical_device.end_command_buffer(command_buffer)?;

            let submit_info = ash::vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&command_buffer));

            let queues = self.queue_families.get_queues(&self).unwrap();

            self.logical_device.queue_submit(queues.graphics, std::slice::from_ref(&submit_info), ash::vk::Fence::null())?;

            self.logical_device.queue_wait_idle(queues.graphics)?;

            self.logical_device.free_command_buffers(self.command_pool, &[command_buffer]);
        };

        Ok(())
    }

    pub fn copy_buffer(
        &self,
        src_buffer: ash::vk::Buffer,
        dst_buffer: ash::vk::Buffer,
        size: ash::vk::DeviceSize,
    ) -> anyhow::Result<(), GentooRenderError> {
        let command_buffer = self.begin_single_time_commands().unwrap(); // fix unwrap

        let copy_region = ash::vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size(size);

        unsafe {
            self.logical_device
                .cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, std::slice::from_ref(&copy_region));
        }

        self.end_single_time_commands(command_buffer)?;

        Ok(())
    }

    pub fn create_image_with_info(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> anyhow::Result<(ash::vk::Image, ash::vk::DeviceMemory), GentooRenderError> {
        let image = unsafe {
            self.logical_device.create_image(image_info, None)?
        };

        let mem_requirements = unsafe {
            self.logical_device.get_image_memory_requirements(image)
        };

        let alloc_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(self.find_memory_type(mem_requirements.memory_type_bits, properties).unwrap());

        let image_memory = unsafe {
            self.logical_device.allocate_memory(&alloc_info, None)?
        };

        unsafe {
            self.logical_device.bind_image_memory(image, image_memory, 0)?
        }

        Ok((image, image_memory))
    }

    fn create_surface(
        instance: &Instance,
        window: &winit::window::Window,
    ) -> anyhow::Result<(ash::extensions::khr::Surface, ash::vk::SurfaceKHR), GentooRenderError> {
        let surface = ash::extensions::khr::Surface::new(&instance.entry, &instance.instance);

        let surface_khr = unsafe {
            ash_window::create_surface(&instance.entry, &instance.instance, window, None)?
        };

        Ok((surface, surface_khr))
    }

    fn pick_physical_device(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
    ) -> anyhow::Result<(ash::vk::PhysicalDevice, ash::vk::PhysicalDeviceProperties), GentooRenderError> {
        let physical_devices = unsafe {
            instance.instance.enumerate_physical_devices()?
        };

        log::debug!("Physical Device count: {}", physical_devices.len());

        let physical_device = physical_devices
            .into_iter()
            .find(|physical_device| Self::is_physical_device_suitable(instance, surface, surface_khr, *physical_device).unwrap()) // TODO: fix unwrap?
            .expect("No suitable physical device found");

        let physical_device_properties = unsafe { instance.instance.get_physical_device_properties(physical_device) };

        log::debug!("Selected Physical Device: {:?}", unsafe {
            CStr::from_ptr(physical_device_properties.device_name.as_ptr())
        });

        Ok((physical_device, physical_device_properties))
    }

    fn is_physical_device_suitable(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<bool, GentooRenderError> {
        let extensions_supported = Self::check_physical_device_extension_support(instance, physical_device)?;

        let mut swapchain_adequate = false;

        if extensions_supported {
            let swapchain_support = Self::query_swapchain_support(surface, surface_khr, physical_device)?;
            swapchain_adequate = {
                !swapchain_support.formats.is_empty()
                    && !swapchain_support.present_modes.is_empty()
            }
        }

        let supported_features = unsafe {
            instance.instance.get_physical_device_features(physical_device)
        };

        Ok({
            // queue_families.is_complete()
                /*&&*/ extensions_supported
                && swapchain_adequate
                && supported_features.sampler_anisotropy != 0
        })
    }

    fn create_logical_device(
        instance: &Instance,
        physical_device: ash::vk::PhysicalDevice,
        queue_families: &QueueFamilies,
    ) -> anyhow::Result<ash::Device, GentooRenderError> {
        let queue_create_infos = queue_families.get_vec().unwrap();

        let physical_device_features = ash::vk::PhysicalDeviceFeatures::builder();

        let (_, logical_device_extensions_ptrs) = Self::get_device_extensions();

        let mut create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&logical_device_extensions_ptrs);

        let (_layer_names, layer_name_ptrs) = Instance::get_enabled_layers();

        if ENABLE_VALIDATION_LAYERS {
            create_info = create_info.enabled_layer_names(&layer_name_ptrs);
        }

        let logical_device = unsafe {
            instance.instance.create_device(physical_device, &create_info, None)?
        };
    
        Ok(logical_device)
    }

    fn create_command_pool(
        logical_device: &ash::Device,
        queue_families: &QueueFamilies,
    ) -> anyhow::Result<ash::vk::CommandPool, GentooRenderError> {
        let create_info = ash::vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families.graphics.unwrap() as u32)
            .flags(
                ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | ash::vk::CommandPoolCreateFlags::TRANSIENT,
            );

        Ok(unsafe {
            logical_device.create_command_pool(&create_info, None)?
        })
    }

    fn get_device_extensions() -> ([&'static CStr; 1], Vec<*const i8>) {
        let device_extensions: [&'static CStr; 1] = [ash::extensions::khr::Swapchain::name()];

        let ext_names_ptrs = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        (device_extensions, ext_names_ptrs)
    }

    fn check_physical_device_extension_support(
        instance: &Instance,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<bool, GentooRenderError> {
        let available_extensions = unsafe {
            instance.instance.enumerate_device_extension_properties(physical_device)?
        };

        let (required_extensions, _) = Self::get_device_extensions();

        for extension in required_extensions.iter() {
            let found = available_extensions.iter().any(|ext| {
                let name = unsafe {
                    CStr::from_ptr(ext.extension_name.as_ptr())
                };

                extension == &name
            });

            if !found {
                log::error!(
                    "Physical Device does not support the following extension: {:?}",
                    extension
                );

                return Ok(false)
            }
        }

        Ok(true)
    }

    fn query_swapchain_support(
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<SwapchainSupportDetails, GentooRenderError> {
        let capabilities = unsafe {
            surface.get_physical_device_surface_capabilities(physical_device, surface_khr)?
        };

        let formats = unsafe {
            surface.get_physical_device_surface_formats(physical_device, surface_khr)?
        };

        let present_modes = unsafe {
            surface.get_physical_device_surface_present_modes(physical_device, surface_khr)?
        };

        Ok(SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        log::debug!("Dropping device");

        unsafe {
            self.logical_device.destroy_command_pool(self.command_pool, None);

            self.logical_device.destroy_device(None);

            self.surface.destroy_surface(self.surface_khr, None);
        }
    }
}
