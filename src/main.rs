#![warn(clippy::all, clippy::pedantic)]

use core::net;
use std::time;

use glfw::{fail_on_errors, Action, Context, Key, MouseButton, Window, WindowEvent};
use wgpu::{self, rwh::HasDisplayHandle, Backends, InstanceDescriptor, Surface};

struct RgbaColor(f64, f64, f64, f64);
impl RgbaColor {
    fn new<R, G, B, A>((r, g, b, a): (R, G, B, A)) -> Option<Self>
    where
        R: Into<f64>,
        G: Into<f64>,
        B: Into<f64>,
        A: Into<f64>,
    {
        // Shadowing
        let r = r.into();
        let g = g.into();
        let b = b.into();
        let a = a.into();

        // Confirm
        let is_invalid = |x: &f64| {
            if 0.0 > *x || *x > 1.0 {
                false
            } else {
                true
            }
        };

        // I <3 Functional Programming
        let proceed = vec![r, g, b, a].iter().any(|x| is_invalid(x));

        // Return
        if proceed {
            Some(RgbaColor(r.into(), g.into(), b.into(), a.into()))
        } else {
            None
        }
    }
}
struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (i32, i32),
    window: &'a mut Window,
}

impl<'a> State<'a> {
    async fn new(window: &'a mut Window) -> State<'a> {
        let size = window.get_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let target = unsafe { wgpu::SurfaceTargetUnsafe::from_window(window) }
            .expect("Failed to get target");
        let surface =
            unsafe { instance.create_surface_unsafe(target) }.expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to create adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to get device & queue.");

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0.max(1) as u32,
            height: size.1.max(1) as u32,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    fn resize(&mut self, new_size: (i32, i32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.size = new_size;
            self.config.width = new_size.0 as u32;
            self.config.height = new_size.1 as u32;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) {
        match event {
            glfw::WindowEvent::CursorPos(x, y) => {
                let x_normalized = x / (self.size.0 as f64);
                let y_normalized = y / (self.size.1 as f64);
                self.clear_screen_to(
                    RgbaColor::new((
                        x_normalized,
                        y_normalized,
                        (x_normalized + y_normalized) / 2.,
                        1.,
                    ))
                    .expect("Invalid input was passed to clear screen"),
                );
                let sensible_wait_time = std::time::Duration::from_millis(5);
                std::thread::sleep(sensible_wait_time);
            }
            _ => {}
        }
    }

    fn clear_screen_to(&mut self, RgbaColor(red, green, blue, alpha): RgbaColor) {
        let output = self
            .surface
            .get_current_texture()
            .expect("Failed to get texture");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: red,
                        g: green,
                        b: blue,
                        a: alpha,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.clear_screen_to(RgbaColor(1.0, 1.0, 1.0, 1.0));
        Ok(())
    }
}

async fn run() {
    // glfw code
    let mut glfw = glfw::init(fail_on_errors!()).expect("Failed to get glfw");

    glfw.window_hint(glfw::WindowHint::Resizable(true));

    let (mut window, events) = glfw
        .create_window(800, 600, "wGPU training arc", glfw::WindowMode::Windowed)
        .expect("Failed to get window and events");

    window.make_current();
    window.set_key_polling(true);
    window.set_cursor_pos_polling(true);
    window.set_cursor_enter_polling(true);
    let mut state = State::new(&mut window).await;

    state.render();
    let mut cursor_pos_was_not_called = true;

    while !state.window.should_close() {
        glfw.poll_events();

        state.update(); // does nothing rn

        // Render the screen
        if cursor_pos_was_not_called {
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    state.resize(state.size)
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    state.window.set_should_close(true);
                    todo!("tracing as well")
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    todo!("tracing")
                }
                Err(wgpu::SurfaceError::Other) => eprintln!("Well shit"),
            }
        }

        // Capture all the events here
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    state.window.set_should_close(true)
                }
                glfw::WindowEvent::Size(width, height) => state.resize((width, height)),
                glfw::WindowEvent::MouseButton(MouseButton::Left, Action::Press, _) => {
                    state.window.set_should_close(true);
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    println!("{}, {}", x, y);
                    state.input(&glfw::WindowEvent::CursorPos(x, y));
                    cursor_pos_was_not_called = false;
                }
                event => {
                    println!("{:?}", event);
                }
            }
        }
    }
}

fn main() {
    pollster::block_on(run());
}
