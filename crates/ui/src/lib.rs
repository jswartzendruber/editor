pub mod camera;
pub mod mesh;
pub mod texture;
pub mod texture_atlas;

use camera::CameraUniform;
use mesh::{Material, Mesh, MeshInstance, MeshVertex};
use std::borrow::Cow;
use wgpu::{util::DeviceExt, Surface};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

struct State<'window> {
    window: &'window Window,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    materials: Vec<Material>,
    meshes: Vec<Mesh>,
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let camera_uniform = CameraUniform::new_ortho(0.0, 800.0, 600.0, 0.0, 1.0, -1.0);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
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
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let mut materials = vec![];

        let bamboo_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("bamboo texture_bind_group_layout"),
            });
        materials.push(Material::load_from_file(
            String::from("res/bamboo.png"),
            &device,
            &queue,
            &bamboo_bind_group_layout,
        ));

        let bamboo_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bamboo Vertex Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bamboo_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bamboo Index Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bamboo_instances = vec![
            MeshInstance {
                position: [0.0, 00.0],
                size: [100.0, 100.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            MeshInstance {
                position: [250.0, 250.0],
                size: [200.0, 200.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let bamboo_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bamboo Instance Buffer"),
            contents: bytemuck::cast_slice(&bamboo_instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bamboo_quad_mesh = Mesh::new(
            String::from("bamboo"),
            bamboo_vertex_buffer,
            bamboo_index_buffer,
            Some(0),
            bamboo_instances,
            bamboo_instance_buffer,
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_bind_group_layout, &bamboo_bind_group_layout],
            push_constant_ranges: &[],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[MeshVertex::desc(), MeshInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

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

        Self {
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            camera_uniform,
            camera_buffer,
            camera_bind_group,

            meshes: vec![bamboo_quad_mesh],
            materials,
        }
    }

    fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);
        self.config.width = width;
        self.config.height = height;
        self.camera_uniform =
            CameraUniform::new_ortho(0.0, width as f32, height as f32, 0.0, 1.0, -1.0);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.surface.configure(&self.device, &self.config);
        self.window.request_redraw();
    }

    fn update(&mut self) {
        for mesh in &mut self.meshes {
            mesh.update();
        }
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
                            r: 0.0,
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
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            for mesh in &self.meshes {
                mesh.draw(&mut rpass, &self.materials);
            }
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
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(&window);
    state.run(event_loop);
}
