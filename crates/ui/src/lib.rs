pub mod camera;
pub mod layout;
pub mod mesh;
pub mod pipeline;
pub mod text;
pub mod texture;
pub mod texture_atlas;

use std::borrow::Cow;

use camera::CameraUniform;
use pipeline::{CameraRaw, PMesh, Pipeline, Uniform, Uniformable};
use text::AtlasPipeline;
use wgpu::Surface;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::{
    mesh::MeshInstance,
    pipeline::{Meshable, PMeshInstace},
};

struct State<'window> {
    window: &'window Window,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    // texture_atlas_pipeline: AtlasPipeline,
    // camera_uniform: CameraUniform,
    test_pipeline: Pipeline,
}

impl<'window> State<'window> {
    fn new(window: &'window Window) -> State<'window> {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();
        let surface: Surface<'window> = instance.create_surface(window).unwrap();

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("Failed to find an appropriate adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            },
            None,
        ))
        .expect("Failed to create device");

        // let camera_uniform = CameraUniform::new(
        //     &device,
        //     0.0,
        //     size.width as f32,
        //     size.height as f32,
        //     0.0,
        //     1.0,
        //     -1.0,
        // );

        // let texture_atlas_pipeline = AtlasPipeline::new(
        //     &device,
        //     &queue,
        //     camera_uniform.bind_group().clone(),
        //     camera_uniform.bind_group_layout().clone(),
        // );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let size = (1360.0, 720.0);
        let camera_uniform_test = Uniform::new(
            &device,
            camera_bind_group_layout,
            CameraRaw::new_ortho(0.0, size.0 as f32, size.1 as f32, 0.0, 1.0, -1.0),
            0,
        );
        let uniforms = vec![Box::new(camera_uniform_test) as Box<dyn Uniformable>];

        let mesh_instances = vec![PMeshInstace {
            position: [0.0, 0.0],
            size: [300.0, 300.0],
            color: [1.0, 0.0, 0.0, 1.0],
        }];
        let mesh_test = PMesh::new(&device, mesh_instances, PMeshInstace::desc());
        let meshes = vec![Box::new(mesh_test) as Box<dyn Meshable>];
        let test_pipeline = Pipeline::new(
            &device,
            Cow::Borrowed(include_str!("rect.wgsl")),
            uniforms,
            meshes,
        );

        Self {
            window,
            surface,
            device,
            queue,
            config,
            // texture_atlas_pipeline,
            // camera_uniform,
            test_pipeline,
        }
    }

    fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);

        self.config.width = width;
        self.config.height = height;

        // self.camera_uniform.update_matrix(
        //     &self.queue,
        //     0.0,
        //     width as f32,
        //     height as f32,
        //     0.0,
        //     1.0,
        //     -1.0,
        // );

        self.surface.configure(&self.device, &self.config);
        self.window.request_redraw();
    }

    fn update(&mut self) {
        // self.texture_atlas_pipeline.update(&self.queue);
        self.test_pipeline.update(&self.queue);
    }

    fn draw(&mut self) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // self.texture_atlas_pipeline.draw(&mut rpass);
            self.test_pipeline.draw(&mut rpass);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }

    fn run(&mut self, event_loop: EventLoop<()>) {
        event_loop
            .run(move |event, elwt| match event {
                Event::AboutToWait => self.window.request_redraw(),
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() => match event {
                    WindowEvent::Resized(new_size) => self.resize(new_size),
                    WindowEvent::RedrawRequested => {
                        self.update();
                        self.draw();
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(KeyCode::Space),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => elwt.exit(),
                    _ => {}
                },
                _ => {}
            })
            .unwrap();
    }
}

pub fn run() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1360, 720))
        .with_title("WGPU")
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&window);
    state.run(event_loop);
}
