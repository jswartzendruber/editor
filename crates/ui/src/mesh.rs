use crate::texture::Texture;
use wgpu::{Device, Queue};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl MeshVertex {
    pub const VERTICES: &'static [MeshVertex] = &[
        MeshVertex {
            position: [0.0, 1.0],
            tex_coords: [0.0, 1.0],
        },
        MeshVertex {
            position: [0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        MeshVertex {
            position: [1.0, 1.0],
            tex_coords: [1.0, 1.0],
        },
        MeshVertex {
            position: [1.0, 0.0],
            tex_coords: [1.0, 0.0],
        },
    ];

    pub const INDICES: &'static [u16] = &[0, 1, 2, 2, 3, 1];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl MeshInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Material {
    pub name: String,
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn load_from_file(
        path: String,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture_bytes = image::io::Reader::open(&path).unwrap().decode().unwrap();
        let texture = Texture::from_image(device, queue, &texture_bytes, Some(&path));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        Self {
            name: path,
            texture,
            bind_group,
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub material_idx: Option<usize>,
    pub instances: Vec<MeshInstance>,
    pub instance_buffer: wgpu::Buffer,
}

impl Mesh {
    pub fn new(
        name: String,
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        material_idx: Option<usize>,
        instances: Vec<MeshInstance>,
        instance_buffer: wgpu::Buffer,
    ) -> Self {
        Self {
            name,
            vertex_buffer,
            index_buffer,
            material_idx,
            instances,
            instance_buffer,
        }
    }

    pub fn update(&mut self) {}

    pub fn draw<'mats: 'rpass, 'mesh: 'rpass, 'rpass>(
        &'mesh self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        materials: &'mats Vec<Material>,
    ) {
        if let Some(material_idx) = &self.material_idx {
            rpass.set_bind_group(1, &materials[*material_idx].bind_group, &[]);
        }
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(
            0..MeshVertex::INDICES.len() as u32,
            0,
            0..self.instances.len() as u32,
        );
    }
}
