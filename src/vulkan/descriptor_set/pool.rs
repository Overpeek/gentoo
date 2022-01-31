use std::rc::Rc;

use crate::vulkan::{Device, GentooRenderError};

pub struct DescriptorPool {
    pub device: Rc<Device>,
    pub pool: ash::vk::DescriptorPool,
}

pub struct DescriptorPoolBuilder {
    device: Rc<Device>,
    pool_sizes: Vec<ash::vk::DescriptorPoolSize>,
    max_sets: u32,
    pool_flags: ash::vk::DescriptorPoolCreateFlags,
}

impl DescriptorPool {
    pub fn new(
        device: Rc<Device>,
    ) -> DescriptorPoolBuilder {
        DescriptorPoolBuilder {
            device,
            pool_sizes: Vec::new(),
            max_sets: 1000,
            pool_flags: ash::vk::DescriptorPoolCreateFlags::empty(),
        }
    }

    pub fn allocate_descriptor(
        &self,
        layouts: &[ash::vk::DescriptorSetLayout],
    ) -> anyhow::Result<ash::vk::DescriptorSet, GentooRenderError> {
        let alloc_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool)
            .set_layouts(layouts)
            .build();

        Ok(unsafe {
            self.device.logical_device.allocate_descriptor_sets(
                &alloc_info,
            )?[0]
        })
    }

    pub fn free_descriptors(
        &self,
        descriptors: &Vec<ash::vk::DescriptorSet>
    ) -> anyhow::Result<(), GentooRenderError> {
        Ok(unsafe {
            self.device.logical_device.free_descriptor_sets(
                self.pool,
                descriptors,
            )?
        })
    }

    pub fn reset_pool(&self) -> anyhow::Result<(), GentooRenderError> {
        Ok(unsafe {
            self.device.logical_device.reset_descriptor_pool(
                self.pool,
                ash::vk::DescriptorPoolResetFlags::empty(),
            )?
        })
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        log::debug!("Dropping descriptor pool");

        unsafe {
            self.device.logical_device.destroy_descriptor_pool(self.pool, None)
        }
    }
}

impl DescriptorPoolBuilder {
    pub fn add_pool_size(
        mut self,
        descriptor_type: ash::vk::DescriptorType,
        count: u32,
    ) -> Self {
        self.pool_sizes.push(
            ash::vk::DescriptorPoolSize {
                ty: descriptor_type,
                descriptor_count: count,
            }
        );

        self
    }

    pub fn set_pool_flags(
        mut self,
        flags: ash::vk::DescriptorPoolCreateFlags,
    ) -> Self {
        self.pool_flags = flags;

        self
    }

    pub fn set_max_sets(
        mut self,
        max_sets: u32,
    ) -> Self {
        self.max_sets = max_sets;

        self
    }

    pub fn build(self) -> anyhow::Result<Rc<DescriptorPool>, GentooRenderError> {
        let DescriptorPoolBuilder {
            device,
            pool_sizes,
            max_sets,
            pool_flags,
        } = self;
        
        let pool_info = ash::vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(max_sets)
            .flags(pool_flags);

        let pool = unsafe {
            device.logical_device.create_descriptor_pool(&pool_info, None)?
        };

        Ok(Rc::new(DescriptorPool {
            device,
            pool,
        }))
    }
}
