pub mod camera_uniform;
pub mod image_pipeline;
pub mod layout;
pub mod quad_pipeline;
pub mod texture;
pub mod texture_atlas;

use camera_uniform::CameraUniform;
use image_pipeline::ImagePipeline;
use layout::{Color, Scene};
use quad_pipeline::QuadPipeline;
use std::{cell::RefCell, io::Read, rc::Rc};
use texture_atlas::TextureAtlas;
use wgpu::Surface;
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
    queue: Rc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,

    camera_uniform: Rc<RefCell<CameraUniform>>,
    atlas: TextureAtlas,

    quad_pipeline: QuadPipeline,
    image_pipeline: ImagePipeline,

    scene: Scene,
}

impl<'window> State<'window> {
    fn new(window: &'window Window, file_to_open: Option<String>) -> State<'window> {
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
        let queue = Rc::new(queue);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera_uniform = Rc::new(RefCell::new(CameraUniform::new(
            &device,
            size.width as f32,
            size.height as f32,
            0,
        )));

        let atlas = TextureAtlas::new(&device, queue.clone(), 1024);

        let quad_pipeline = QuadPipeline::new(&device, camera_uniform.clone());
        let image_pipeline = ImagePipeline::new(&device, camera_uniform.clone(), &atlas);

        let mut scene = Scene::default();

        let file_contents = if let Some(file_name) = file_to_open {
            let mut file = std::fs::File::open(file_name).unwrap();
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();
            buf
        } else {
            String::from("")
        };

        let td = scene.text_details(
            file_contents,
            16.0,
            Color::new(255, 255, 255, 255),
            Color::new(5, 5, 5, 255),
        );
        scene.set_focus(td);

        let root = scene.hbox(vec![td]);
        scene.set_root(root);

        Self {
            window,
            surface,
            device,
            queue,
            config,

            camera_uniform,
            atlas,

            quad_pipeline,
            image_pipeline,

            scene,
        }
    }

    fn resize(&mut self, new_size: &PhysicalSize<u32>) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);

        self.config.width = width;
        self.config.height = height;

        self.camera_uniform
            .borrow_mut()
            .update_size(&self.queue, width as f32, height as f32);

        self.surface.configure(&self.device, &self.config);
        self.window.request_redraw();
    }

    fn update(&mut self) {
        let instances = self.scene.layout(
            &mut self.atlas,
            (self.config.width as f32, self.config.height as f32),
            &self.queue,
            self.window,
        );

        let quad_instances = self.quad_pipeline.instances();
        let image_instances = self.image_pipeline.instances();

        quad_instances.clear();
        image_instances.clear();

        for instance in instances {
            match instance {
                layout::Drawables::Rect(qi) => quad_instances.push(qi),
                layout::Drawables::TexturedRect(ii) => image_instances.push(ii),
            }
        }

        self.quad_pipeline.update(&self.queue);
        self.image_pipeline.update(&self.queue);
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
        let camera_uniform = &self.camera_uniform.borrow();
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

            self.quad_pipeline.draw(&mut rpass, camera_uniform);
            self.image_pipeline.draw(&mut rpass, camera_uniform);
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
                    WindowEvent::MouseWheel { delta, .. } => {
                        self.scene.scroll(*delta, &mut self.atlas);
                    }
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                    } => self
                        .scene
                        .update_cursor_pos(position.x as f32, position.y as f32),
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => elwt.exit(),
                    WindowEvent::KeyboardInput { event, .. } => {
                        self.scene.send_keystroke(event, &mut self.atlas)
                    }
                    _ => {}
                },
                _ => {}
            })
            .unwrap();
    }
}

pub fn run(file_to_open: Option<String>) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(1360, 720))
        .with_title("WGPU")
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&window, file_to_open);
    state.run(event_loop);
}
