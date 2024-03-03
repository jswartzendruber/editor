pub mod camera;
pub mod mesh;
pub mod texture;
pub mod texture_atlas;

use camera::CameraUniform;
use mesh::{Material, Mesh, MeshInstance, MeshVertex};
use std::borrow::Cow;
use texture_atlas::TextureAtlas;
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

    material: Material,
    mesh: Mesh,
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

        let atlas_size = 700;
        let mut texture_atlas: TextureAtlas = TextureAtlas::new(&device, &queue, atlas_size);
        let atlas_bind_group_layout =
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
                label: Some("atlas texture_bind_group_layout"),
            });
        let atlas_material = Material::new(
            "atlas".to_string(),
            &device,
            &atlas_bind_group_layout,
            texture_atlas.texture(),
        );

        let bamboo_img = image::io::Reader::open("res/bamboo.png")
            .unwrap()
            .decode()
            .unwrap();
        let tree_img = image::io::Reader::open("res/happy-tree.png")
            .unwrap()
            .decode()
            .unwrap();
        let hello_img = image::io::Reader::open("res/hello.png")
            .unwrap()
            .decode()
            .unwrap();
        let rect_img = image::io::Reader::open("res/rect.png")
            .unwrap()
            .decode()
            .unwrap();
        let tree_alloc = texture_atlas
            .allocate(&queue, &tree_img.to_rgba8())
            .unwrap();
        let bamboo_alloc = texture_atlas
            .allocate(&queue, &bamboo_img.to_rgba8())
            .unwrap();
        let hello_alloc = texture_atlas
            .allocate(&queue, &hello_img.to_rgba8())
            .unwrap();
        let rect_alloc = texture_atlas
            .allocate(&queue, &rect_img.to_rgba8())
            .unwrap();

        let atlas_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas Vertex Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let atlas_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas Index Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_bind_group_layout, &atlas_bind_group_layout],
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

        let atlas_image_instances = vec![
            MeshInstance {
                position: [0.0, 0.0],
                size: [300.0, 300.0],
                atlas_offset: [
                    tree_alloc.rectangle.min.x as f32 / atlas_size as f32,
                    tree_alloc.rectangle.min.y as f32 / atlas_size as f32,
                ],
                atlas_scale: [
                    tree_alloc.rectangle.width() as f32 / atlas_size as f32,
                    tree_alloc.rectangle.height() as f32 / atlas_size as f32,
                ],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            MeshInstance {
                position: [300.0, 300.0],
                size: [300.0, 300.0],
                atlas_offset: [
                    bamboo_alloc.rectangle.min.x as f32 / atlas_size as f32,
                    bamboo_alloc.rectangle.min.y as f32 / atlas_size as f32,
                ],
                atlas_scale: [
                    bamboo_alloc.rectangle.width() as f32 / atlas_size as f32,
                    bamboo_alloc.rectangle.height() as f32 / atlas_size as f32,
                ],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            MeshInstance {
                position: [0.0, 300.0],
                size: [300.0, 300.0],
                atlas_offset: [
                    hello_alloc.rectangle.min.x as f32 / atlas_size as f32,
                    hello_alloc.rectangle.min.y as f32 / atlas_size as f32,
                ],
                atlas_scale: [
                    hello_alloc.rectangle.width() as f32 / atlas_size as f32,
                    hello_alloc.rectangle.height() as f32 / atlas_size as f32,
                ],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            MeshInstance {
                position: [300.0, 150.0],
                size: [300.0, 150.0],
                atlas_offset: [
                    rect_alloc.rectangle.min.x as f32 / atlas_size as f32,
                    rect_alloc.rectangle.min.y as f32 / atlas_size as f32,
                ],
                atlas_scale: [
                    rect_alloc.rectangle.width() as f32 / atlas_size as f32,
                    rect_alloc.rectangle.height() as f32 / atlas_size as f32,
                ],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        let atlas_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas Instance Buffer"),
            contents: bytemuck::cast_slice(&atlas_image_instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let atlas_mesh = Mesh::new(
            String::from("atlas"),
            atlas_vertex_buffer,
            atlas_index_buffer,
            atlas_image_instances,
            atlas_instance_buffer,
        );

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

            mesh: atlas_mesh,
            material: atlas_material,
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
        self.mesh.update();
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
                            g: 0.05,
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
            self.mesh.draw(&mut rpass, &self.material);
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
