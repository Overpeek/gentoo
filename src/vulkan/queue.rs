use super::{Instance, GentooRenderError, Device};

const PRIORITY: [f32; 1] = [1.0];

pub struct QueueFamilies {
    pub present: Option<usize>,
    pub graphics: Option<usize>,
}

pub struct Queues {
    pub present: ash::vk::Queue,
    pub graphics: ash::vk::Queue,
}

impl QueueFamilies {
    pub fn new(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<Self, GentooRenderError> {
        let mut queue_families = Self {
            present: None,
            graphics: None,
        };

        let queue_family_properties = unsafe {
            instance.instance.get_physical_device_queue_family_properties(physical_device)
        };

        for (index, queue_family_property) in queue_family_properties.into_iter().enumerate() {
            let present_support = unsafe {
                surface.get_physical_device_surface_support(physical_device, index as u32, surface_khr)?
            };

            let graphics_support = queue_family_property.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS);

            if present_support && queue_families.present.is_none() {
                queue_families.present = Some(index);
            }

            if graphics_support && queue_families.graphics.is_none() {
                queue_families.graphics = Some(index);
            }

            if queue_families.finished() {
                break;
            }
        }

        Ok(queue_families)
    }

    pub fn get_vec(&self) -> Option<Vec<ash::vk::DeviceQueueCreateInfo>> {
        if self.same()? {
            Some(vec![ash::vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(self.present.unwrap() as u32)
                .queue_priorities(&PRIORITY)
                .build(),
            ])
        } else {
            Some(vec![ash::vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(self.present.unwrap() as u32)
                .queue_priorities(&PRIORITY)
                .build(),
            ash::vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(self.present.unwrap() as u32)
                .queue_priorities(&PRIORITY)
                .build(),
            ])
        }
    }

    pub fn get_queues(&self, device: &Device) -> Option<Queues> {
        if !self.finished() {
            None
        } else {
            let graphics = unsafe {
                device.logical_device.get_device_queue(self.graphics.unwrap() as u32, 0)
            };

            let present = unsafe {
                device.logical_device.get_device_queue(self.present.unwrap() as u32, 0)
            };

            Some(Queues {
                present,
                graphics,
            })
        }
    }

    pub fn finished(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }

    pub fn same(&self) -> Option<bool> {
        Some(self.present? == self.graphics?)
    }
}
