pub mod camera_uniform;
pub mod image_pipeline;
pub mod layout;
pub mod quad_pipeline;
pub mod texture;
pub mod texture_atlas;

use camera_uniform::CameraUniform;
use fontdue::Font;
use image::RgbaImage;
use image_pipeline::ImagePipeline;
use layout::{Color, FixedSizedBox, Hbox, Rectangle, TexturedRectangle, Ui, Vbox};
use quad_pipeline::QuadPipeline;
use std::{cell::RefCell, rc::Rc};
use texture_atlas::{TextureAtlas, TextureId};
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
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    camera_uniform: Rc<RefCell<CameraUniform>>,
    atlas: TextureAtlas,

    quad_pipeline: QuadPipeline,
    image_pipeline: ImagePipeline,

    layout_tree: Ui,

    g_id: TextureId,
    g_metrics: fontdue::Metrics,
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

        let camera_uniform = Rc::new(RefCell::new(CameraUniform::new(
            &device,
            size.width as f32,
            size.height as f32,
            0,
        )));

        let mut atlas = TextureAtlas::new(&device, &queue, 1024);
        let bamboo_atlas_idx = atlas
            .load_image_from_file(&queue, "res/bamboo.png")
            .unwrap();
        let tree_atlas_idx = atlas
            .load_image_from_file(&queue, "res/happy-tree.png")
            .unwrap();
        let hello_atlas_idx = atlas.load_image_from_file(&queue, "res/hello.png").unwrap();
        let rect_atlas_idx = atlas.load_image_from_file(&queue, "res/rect.png").unwrap();

        let quad_pipeline = QuadPipeline::new(&device, camera_uniform.clone());
        let image_pipeline = ImagePipeline::new(&device, camera_uniform.clone(), &atlas);

        let layout_tree = Ui::Hbox(Hbox::new(vec![
            Ui::Vbox(Vbox::new(vec![
                Ui::TexturedRectangle(TexturedRectangle::new(bamboo_atlas_idx)),
                Ui::Rectangle(Rectangle::new(Color::new(0, 255, 0, 255))),
                Ui::TexturedRectangle(TexturedRectangle::new(tree_atlas_idx)),
            ])),
            Ui::FixedSizedBox(FixedSizedBox::new(
                200.0,
                100.0,
                Ui::Rectangle(Rectangle::new(Color::new(100, 100, 100, 255))),
                Color::new(15, 15, 15, 255),
            )),
            Ui::Vbox(Vbox::new(vec![
                Ui::TexturedRectangle(TexturedRectangle::new(hello_atlas_idx)),
                Ui::Rectangle(Rectangle::new(Color::new(0, 255, 0, 255))),
                Ui::TexturedRectangle(TexturedRectangle::new(rect_atlas_idx)),
            ])),
        ]));

        let font_bytes = include_bytes!("../../../res/RobotoMono-Regular.ttf") as &[u8];
        let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();
        let (metrics, bitmap) = font.rasterize('J', 72.0);
        let image = RgbaImage::from_raw(
            metrics.width as u32,
            metrics.height as u32,
            bitmap
                .into_iter()
                .flat_map(|byte| [255, 255, 255, byte])
                .collect(),
        )
        .unwrap();
        let id = atlas.load_from_image(&queue, &image).unwrap();

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

            layout_tree,

            g_id: id,
            g_metrics: metrics,
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
        let instances = self.layout_tree.layout(
            &self.atlas,
            (self.config.width as f32, self.config.height as f32),
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

        image_instances.push(image_pipeline::ImageInstance::add_instance(
            &self.atlas,
            self.g_id,
            [500.0, 450.0],
            [self.g_metrics.width as f32, self.g_metrics.height as f32],
            [1.0, 1.0, 1.0, 1.0],
        ));
        image_instances.push(image_pipeline::ImageInstance::add_instance(
            &self.atlas,
            self.g_id,
            [500.0 + (self.g_metrics.width as f32 * 1.1), 450.0],
            [self.g_metrics.width as f32, self.g_metrics.height as f32],
            [1.0, 1.0, 1.0, 1.0],
        ));

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
