#![windows_subsystem = "windows"]
#![deny(clippy::all)]
#![forbid(unsafe_code)]

use error_iter::ErrorIter as _;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use rand::Rng;
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

/// Representation of the application state. In this example, a box will bounce around the screen.
struct World {
    roll_counts: [u64; 20],
    winning_roll_key: Option<usize>,
    losing_roll_key: Option<usize>,
    width: u32,
    height: u32,
    column_width: u32,
    offset: u32,
    colors: [[u8; 4]; 20],
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = {
        let mut builder = WindowBuilder::new();

        builder = builder
            .with_title("d20 visualizer")
            .with_min_inner_size(LogicalSize::new(100., 100.));

        #[cfg(debug_assertions)]
        {
            let monitor = event_loop
                .available_monitors()
                .last()
                .expect("no monitor found");
            let monitor_size = monitor.size();
            builder = builder
                .with_position(monitor.position())
                .with_inner_size(LogicalSize::new(
                    monitor_size.width as f64 * 0.85,
                    monitor_size.height as f64 * 0.85,
                ));
        }

        builder.build(&event_loop).unwrap()
    };

    let inner_size = window.inner_size();
    let mut pixels = {
        let surface_texture = SurfaceTexture::new(inner_size.width, inner_size.height, &window);
        Pixels::new(inner_size.width, inner_size.height, surface_texture)?
    };
    let mut world = World::new(inner_size.width, inner_size.height);

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, window_id } => match event {
            WindowEvent::CloseRequested => {
                if window_id == window.id() {
                    *control_flow = ControlFlow::Exit
                }
            }
            WindowEvent::Resized(_) => {
                let inner_size = window.inner_size();
                pixels
                    .resize_surface(inner_size.width, inner_size.height)
                    .expect("could not resize surface");
                pixels
                    .resize_buffer(inner_size.width, inner_size.height)
                    .expect("could not resize buffer");
                world.set_size(inner_size.width, inner_size.height);
                window.request_redraw()
            }
            WindowEvent::Moved(_) => (),
            WindowEvent::Focused(_) => (),
            WindowEvent::KeyboardInput {
                device_id: _,
                input,
                is_synthetic: _,
            } => match input.virtual_keycode {
                Some(VirtualKeyCode::F5) => {
                    for count in world.roll_counts.iter_mut() {
                        *count = 0;
                    }
                }
                Some(VirtualKeyCode::Escape) => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            WindowEvent::Destroyed
            | WindowEvent::DroppedFile(_)
            | WindowEvent::HoveredFile(_)
            | WindowEvent::HoveredFileCancelled
            | WindowEvent::ReceivedCharacter(_)
            | WindowEvent::ModifiersChanged(_)
            | WindowEvent::Ime(_)
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::TouchpadMagnify { .. }
            | WindowEvent::SmartMagnify { .. }
            | WindowEvent::TouchpadRotate { .. }
            | WindowEvent::TouchpadPressure { .. }
            | WindowEvent::AxisMotion { .. }
            | WindowEvent::Touch(_)
            | WindowEvent::ScaleFactorChanged { .. }
            | WindowEvent::ThemeChanged(_)
            | WindowEvent::Occluded(_) => (),
        },
        Event::MainEventsCleared => {
            world.update();
            window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            world.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
        Event::NewEvents(_)
        | Event::DeviceEvent { .. }
        | Event::UserEvent(_)
        | Event::Suspended
        | Event::Resumed
        | Event::RedrawEventsCleared
        | Event::LoopDestroyed => (),
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

impl World {
    fn new(width: u32, height: u32) -> Self {
        let mut colors = Vec::new();
        let r = 0x00;
        let mut g = 0x00;
        let mut b = 0x00;
        let a = 0xff;
        for _ in 1..=20 {
            g += 0x09;
            b += 0x09;
            colors.push([r, g, b, a]);
        }
        let mut world = Self {
            roll_counts: [0; 20],
            winning_roll_key: None,
            losing_roll_key: None,
            width: 0,
            height: 0,
            column_width: 0,
            offset: 0,
            colors: colors
                .try_into()
                .expect("could not convert colors to an array"),
        };
        world.set_size(width, height);
        world
    }

    fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.column_width = (width as f64 / 20.).floor() as u32;
        self.offset = (width - self.column_width * 20) / 2;
    }

    /// Update the `World` internal state; bounce the box around the screen.
    fn update(&mut self) {
        let mut rng = rand::thread_rng();
        for _ in 1..=10000 {
            let roll = rng.gen_range(1..=20);
            *self
                .roll_counts
                .get_mut(roll - 1)
                .expect("roll value not found") += 1;
            self.roll_counts.get(roll - 1).expect("no value found") as &u64;
        }

        let mut min_found = u64::MAX;
        let mut max_found = 0;
        for (roll_key, count) in self.roll_counts.iter().enumerate() {
            if *count > max_found {
                max_found = *count;
                self.winning_roll_key = Some(roll_key);
            }
            if *count < min_found {
                min_found = *count;
                self.losing_roll_key = Some(roll_key);
            }
        }

        let max_allowed = self.column_width as u64 * self.height as u64;
        if max_found > max_allowed {
            let mut adjustment = max_found - max_allowed;
            adjustment -= adjustment % self.column_width as u64;
            for count in self.roll_counts.iter_mut() {
                let roll_adjustment = adjustment as f64 * (*count as f64 / max_allowed as f64);
                *count -= (roll_adjustment as u64).min(*count);
            }
        }
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let total_x = i as u32 % self.width;
            let y = self.height - 1 - (i as u32 / self.width);

            let cutoff = self.offset + self.column_width * self.roll_counts.len() as u32;
            let roll_key = if total_x >= self.offset && total_x < cutoff {
                Some(((total_x - self.offset) / self.column_width) as usize)
            } else {
                None
            };
            let highlighted = if let Some(roll_key) = roll_key {
                let roll_x = total_x - self.offset - roll_key as u32 * self.column_width;
                let value = y * self.column_width + roll_x + 1;
                value as u64 <= self.roll_counts[roll_key]
            } else {
                false
            };

            let rgba = if highlighted {
                if roll_key == self.winning_roll_key {
                    [0x33, 0xcc, 0x33, 0xff]
                } else if roll_key == self.losing_roll_key {
                    [0xcc, 0x33, 0x33, 0xff]
                } else {
                    self.colors[roll_key.unwrap()]
                }
            } else {
                [0x33, 0x33, 0x33, 0xff]
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}
