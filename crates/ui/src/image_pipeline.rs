use crate::{
    camera_uniform::CameraUniform,
    layout::{BoundingBox, Color, Drawables},
    quad_pipeline::QuadInstance,
    texture_atlas::{AllocationInfo, TextureAtlas},
};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
use text_editor::TextEditor;
use wgpu::util::DeviceExt;

pub fn layout_text(
    area: BoundingBox,
    atlas: &mut TextureAtlas,
    font_size: f32,
    font_color: &Color,
    draw_cursor: bool,
    editor: &TextEditor,
) -> Vec<Drawables> {
    let mut drawables = vec![];

    let line_height = font_size * 1.2;
    let mut baseline = area.top_left();
    baseline.1 += line_height;

    let mut drew_cursor = false;
    let mut curr_byte_index = editor.text_start_idx();
    let layout = editor.layout_lines(atlas);

    for line in layout {
        for c in line.chars() {
            let glyph = atlas.map_get_or_insert_glyph(c, font_size).unwrap();
            let metrics = glyph.metrics;

            // Move to next line
            if c == '\n' {
                baseline.1 += line_height;
                baseline.0 = area.min.0;
                continue;
            }

            // Return early if we leave our box
            if !area.inside(baseline) {
                return drawables;
            }

            // TODO: blinking cursor gets out of sync with where we are typing
            // :) :(
            if curr_byte_index == editor.cursor_position() && draw_cursor {
                drew_cursor = true;
                let cursor_height = (font_size * 0.85).floor();
                let cursor_width = (font_size / 10.0).floor();
                drawables.push(Drawables::Rect(QuadInstance {
                    position: [baseline.0, baseline.1 - cursor_height],
                    size: [cursor_width, cursor_height],
                    color: [1.0, 1.0, 1.0, 1.0],
                }));
            }

            drawables.push(Drawables::TexturedRect(ImageInstance::add_instance(
                atlas,
                glyph.allocation_info,
                [baseline.0 + metrics.pos.0, baseline.1 - metrics.pos.1],
                [metrics.size.0, metrics.size.1],
                font_color.to_f32_arr(),
            )));

            baseline.0 += metrics.advance.0;
            baseline.1 += metrics.advance.1;
            curr_byte_index += c.len_utf8();
        }

        // Move to next line
        baseline.1 += line_height;
        baseline.0 = area.min.0;
        continue;
    }

    if !drew_cursor && draw_cursor {
        let cursor_height = (font_size * 0.85).floor();
        let cursor_width = (font_size / 8.5).floor();
        drawables.push(Drawables::Rect(QuadInstance {
            position: [baseline.0, baseline.1 - cursor_height],
            size: [cursor_width, cursor_height],
            color: [1.0, 1.0, 1.0, 1.0],
        }));
    }

    drawables
}

/// The projection matrix used in the shaders.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraRaw {
    projection: [[f32; 4]; 4],
}

impl CameraRaw {
    pub fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        CameraRaw {
            projection: [
                [2.0 / (right - left), 0.0, 0.0, 0.0],
                [0.0, 2.0 / (top - bottom), 0.0, 0.0],
                [0.0, 0.0, 1.0 / (near - far), 0.0],
                [
                    (right + left) / (left - right),
                    (top + bottom) / (bottom - top),
                    near / (near - far),
                    1.0,
                ],
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub atlas_offset: [f32; 2],
    pub atlas_scale: [f32; 2],
    pub color: [f32; 4],
}

impl ImageInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        5 => Float32x2,
        6 => Float32x2,
        7 => Float32x2,
        8 => Float32x2,
        9 => Float32x4,
    ];

    const MAX: usize = 65536;

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn add_instance(
        atlas: &TextureAtlas,
        allocation_info: AllocationInfo,
        position: [f32; 2],
        size: [f32; 2],
        color: [f32; 4],
    ) -> Self {
        let atlas_size = atlas.size() as f32;

        ImageInstance {
            position,
            size,
            atlas_offset: [
                allocation_info.x / atlas_size,
                allocation_info.y / atlas_size,
            ],
            atlas_scale: [
                allocation_info.width / atlas_size,
                allocation_info.height / atlas_size,
            ],
            color,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl ImageVertex {
    #[rustfmt::skip]
    pub const VERTICES: &'static [ImageVertex] = &[
        ImageVertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
        ImageVertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
        ImageVertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
        ImageVertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
    ];

    pub const INDICES: &'static [u16] = &[0, 1, 2, 2, 3, 1];

    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,

    atlas_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instances: Vec<ImageInstance>,
}

impl ImagePipeline {
    pub fn new(
        device: &wgpu::Device,
        camera_uniform: Rc<RefCell<CameraUniform>>,
        atlas: &TextureAtlas,
    ) -> Self {
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

        let atlas_texture = atlas.texture();
        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_texture.sampler),
                },
            ],
            label: Some("atlas texture_bind_group"),
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: (std::mem::size_of::<ImageInstance>() * ImageInstance::MAX) as u64,
            mapped_at_creation: false,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(ImageVertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(ImageVertex::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = vec![];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                camera_uniform.borrow().bind_group_layout(),
                &atlas_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("image.wgsl"))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[ImageVertex::desc(), ImageInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,

            atlas_bind_group,

            vertex_buffer,
            instance_buffer,
            index_buffer,

            instances,
        }
    }

    pub fn instances(&mut self) -> &mut Vec<ImageInstance> {
        &mut self.instances
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
    }

    pub fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, camera_uniform: &'a CameraUniform) {
        rpass.set_pipeline(&self.pipeline);

        rpass.set_bind_group(camera_uniform.index(), camera_uniform.bind_group(), &[]);
        rpass.set_bind_group(1, &self.atlas_bind_group, &[]);

        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(
            0..ImageVertex::INDICES.len() as u32,
            0,
            0..self.instances.len() as u32,
        );
    }
}
