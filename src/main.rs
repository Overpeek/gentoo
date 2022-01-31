use std::time::{Instant, Duration};

use input::Input;
use winit::{dpi::PhysicalSize, event::{Event, WindowEvent}, event_loop::ControlFlow};

use crate::application::Application;

mod application;
mod window;
mod vulkan;
mod frame_info;
mod camera;
mod keyboard_movement_controller;
mod input;

pub use frame_info::*;

fn main() {
    simple_logger::SimpleLogger::new().without_timestamps().init().unwrap();

    let (mut application, event_loop) = Application::new().unwrap();

    let mut current_time = Instant::now();

    let mut input = Input::new();

    let mut frame_count_check_tp = Instant::now();
    let mut frames = 0;
    let mut fps = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let app = &mut application;

        match event {
            Event::WindowEvent { event, .. } => {
                input.update(&event);
                app.update(&event);

                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        if !(app.window.raw_window.inner_size().width == width && app.window.raw_window.inner_size().height == height) {
                            log::debug!("Resizing window");
                            log::info!("New window size: {}x{}", width, height);
                            app.resize().unwrap();
                        }
                    }
                    _ => ()
                }
            }
            Event::MainEventsCleared => {
                app.window.raw_window.request_redraw();
            },
            Event::RedrawRequested(_) => {
                let frame_time = current_time.elapsed().as_secs_f32();
                current_time = Instant::now();
                app.run(&input, frame_time, fps).unwrap();

                frames += 1;

                if frame_count_check_tp.elapsed() > Duration::from_secs(1) {
                    frame_count_check_tp = Instant::now();
                    fps = frames;

                    frames = 0;
                }

            }
            _ => (),
        }
    });
}
