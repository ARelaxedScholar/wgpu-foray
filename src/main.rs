#![warn(clippy::all, clippy::pedantic)]

mod colors;
mod prelude;

use core::net;
use prelude::*;
use std::time;

use glfw::{fail_on_errors, Action, Context, Key, MouseButton, Window, WindowEvent};
use wgpu::{
    self, core::device::SHADER_STAGE_COUNT, include_wgsl, rwh::HasDisplayHandle,
    util::RenderEncoder, Backends, Color, InstanceDescriptor, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipelineDescriptor, Surface, VertexStepMode,
};

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: (i32, i32),
    window: &'a mut Window,
    render_pipelines: Vec<wgpu::RenderPipeline>,
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Use the same layout (is probably fine?)
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        // Default Pipeline
        let render_pipeline_0 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Default Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        // The one that uses Position
        let render_pipeline_1 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Position Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main_pos"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        let render_pipelines = vec![render_pipeline_0, render_pipeline_1];

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipelines,
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

    fn draw_triangle(&mut self, toggle: bool) {
        // My conditional here
        let render_pipeline: &wgpu::RenderPipeline = if toggle {
            &self.render_pipelines[1]
        } else {
            &self.render_pipelines[0]
        };
        // We will create a new pipeline
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
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(render_pipeline);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn clear_screen_to(&mut self, color: RgbaColor) {
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
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: color.red(),
                        g: color.green(),
                        b: color.blue(),
                        a: color.alpha(),
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

    fn render(&mut self, use_pos: bool) -> Result<(), wgpu::SurfaceError> {
        // The values passed are in the normalized range

        self.draw_triangle(use_pos);
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

    state.clear_screen_to(colors::Colors::WHITE);
    let mut triangle_toggle = false;
    let mut last_color = wgpu::Color::WHITE;

    while !state.window.should_close() {
        glfw.poll_events();

        state.update(); // does nothing rn

        // Render the screen
        // match state.render(triangle_toggle) {
        //     Ok(_) => {}
        //     Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
        //         state.resize(state.size)
        //     }
        //     Err(wgpu::SurfaceError::OutOfMemory) => {
        //         state.window.set_should_close(true);
        //         todo!("tracing as well")
        //     }
        //     Err(wgpu::SurfaceError::Timeout) => {
        //         todo!("tracing")
        //     }
        //     Err(wgpu::SurfaceError::Other) => eprintln!("Well shit"),
        // }

        // Capture all the events here
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    state.window.set_should_close(true)
                }
                glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                    triangle_toggle = !triangle_toggle;

                    let output = state
                        .surface
                        .get_current_texture()
                        .expect("Failed to get texture");
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        state
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            });

                    // Layer 1: Keep whatever color was already loaded
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(last_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    drop(render_pass);

                    // Elect which Pipeline to use based on toggle
                    let render_pipeline: &wgpu::RenderPipeline = if triangle_toggle {
                        &state.render_pipelines[1]
                    } else {
                        &state.render_pipelines[0]
                    };

                    // Layer 2: Triangle Drawing
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(render_pipeline);
                    render_pass.draw(0..3, 0..1);
                    drop(render_pass);

                    // Submit and Enjoy!?!?!
                    state.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }
                glfw::WindowEvent::Size(width, height) => state.resize((width, height)),
                glfw::WindowEvent::MouseButton(MouseButton::Left, Action::Press, _) => {
                    state.window.set_should_close(true);
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    // Procedural attempt to check something
                    println!("{}, {}", x, y);
                    //state.input(&glfw::WindowEvent::CursorPos(x, y));
                    //state.render(triangle_toggle);

                    let x_normalized = x / (state.size.0 as f64);
                    let y_normalized = y / (state.size.1 as f64);

                    //TODO: Find a way to abstract the boiler plate in some sort of builder pattern
                    let output = state
                        .surface
                        .get_current_texture()
                        .expect("Failed to get texture");
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        state
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            });

                    // Layer 1 Clear Screen to Color Determined by Cursor
                    last_color = wgpu::Color {
                        r: x_normalized,
                        g: y_normalized,
                        b: (x_normalized + y_normalized) / 2.,
                        a: 1.,
                    };
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(last_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    drop(render_pass);

                    // Elect which Pipeline to use based on toggle
                    let render_pipeline: &wgpu::RenderPipeline = if triangle_toggle {
                        &state.render_pipelines[1]
                    } else {
                        &state.render_pipelines[0]
                    };

                    // Layer 2: Triangle Drawing
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render_pass.set_pipeline(render_pipeline);
                    render_pass.draw(0..3, 0..1);
                    drop(render_pass);

                    // Submit and Enjoy!?!?!
                    state.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
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
