use std::rc::Rc;

use crate::vulkan::{Device, GentooRenderError};

pub struct PipelineCache {
    device: Rc<Device>,
    pub cache: ash::vk::PipelineCache,
}

impl PipelineCache {
    pub fn new(device: Rc<Device>) -> anyhow::Result<Rc<Self>, GentooRenderError> {
        let data = match std::fs::read("pipeline_cache.bin") {
            Ok(data) => {
                log::debug!("Loaded pipeline cache");
                data
            },
            Err(_) => {
                log::debug!("Failed to load pipeline cache");
                Vec::new()
            },
        };

        let cache_info = ash::vk::PipelineCacheCreateInfo::builder()
            .initial_data(&data);

        let cache = unsafe {
            device.logical_device.create_pipeline_cache(&cache_info, None)?
        };

        Ok(Rc::new(Self {
            device,
            cache,
        }))
    }
}

impl Drop for PipelineCache {
    fn drop(&mut self) {
        log::debug!("Dropping pipeline cache");

        unsafe {
            let data = self.device.logical_device.get_pipeline_cache_data(self.cache).unwrap();

            std::fs::write("pipeline_cache.bin", data)
                .expect("Failed to write pipeline cache");
            
            self.device.logical_device.destroy_pipeline_cache(self.cache, None);
        }
    }
}
