mod device;
mod swapchain;
mod renderer;
mod model;
mod buffer;
mod instance;
mod shader;
mod queue;

pub mod pipeline;
pub mod descriptor_set;
pub mod systems;
pub mod egui;

pub use device::*;
pub use swapchain::*;
pub use renderer::*;
pub use model::*;
pub use buffer::*;
pub use instance::*;
pub use shader::*;
pub use queue::*;

#[repr(align(16))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Align16<T>(pub T);

#[derive(thiserror::Error, Debug)]
pub enum GentooRenderError {
    #[error("")]
    VulkanError(#[from] ash::vk::Result),
    #[error("")]
    LoadingError(#[from] ash::LoadingError),
    #[error("Swapchain image or depth format has changed")]
    CompareSwapFormatsError,
}
