#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSettings {
    pub title: &'static str,
    pub dimensions: Dimensions,
    pub resizable: bool,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum WindowMode {
//     Windowed,
//     Borderless,
//     Exclusive,
// }

pub struct Window {
    pub raw_window: winit::window::Window,

    // mode: WindowMode,
}

impl Window {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>, settings: WindowSettings) -> Self {
        let raw_window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::LogicalSize::new(settings.dimensions.width, settings.dimensions.height))
            .with_title(settings.title)
            .with_resizable(settings.resizable)
            .build(&event_loop).unwrap();

        Self {
            raw_window,
            // mode: WindowMode::Windowed,
        }
    }

    // pub fn set_mode(&mut self, mode: WindowMode) {
    //     match mode {
    //         WindowMode::Windowed => self.raw_window.set_fullscreen(None),
    //         WindowMode::Borderless => self.raw_window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None))),
    //         WindowMode::Exclusive => {
    //             let vm = self
    //                 .raw_window
    //                 .current_monitor()
    //                 .expect("No monitor detected")
    //                 .video_modes()
    //                 .min()
    //                 .expect("No video modes found");

    //             self.raw_window.set_fullscreen(Some(winit::window::Fullscreen::Exclusive(vm)));
    //         },
    //     }

    //     self.mode = mode;
    // }
}
