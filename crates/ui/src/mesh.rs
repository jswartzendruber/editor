use crate::texture::Texture;
use wgpu::{Device, Queue};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl MeshVertex {
    #[rustfmt::skip]
    pub const VERTICES: &'static [MeshVertex] = &[
        MeshVertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
        MeshVertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
        MeshVertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
        MeshVertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
    ];

    pub const INDICES: &'static [u16] = &[0, 1, 2, 2, 3, 1];

    pub fn tex_coords_from(
        atlas_size: u16,
        top_left: (i32, i32),
        bottom_right: (i32, i32),
    ) -> Vec<MeshVertex> {
        let mut vertices = Self::VERTICES.to_vec();

        let atlas_size = atlas_size as f32;

        // Normalize the coordinates from range [0, atlas_size] to [0,1] for displaying
        let ntlx = top_left.0 as f32 / atlas_size;
        let ntly = top_left.1 as f32 / atlas_size;
        let nbrx = bottom_right.0 as f32 / atlas_size;
        let nbry = bottom_right.1 as f32 / atlas_size;

        vertices[0].tex_coords = [ntlx, nbry];
        vertices[1].tex_coords = [ntlx, ntly];
        vertices[2].tex_coords = [nbrx, nbry];
        vertices[3].tex_coords = [nbrx, ntly];

        vertices
    }

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

    /// The top left corner of the requested position in the atlas.
    /// This position should be normalized to the size of the atlas.
    /// So if we want position (0, 128) in a 512x512 atlas, this number
    /// would be (0, 0.25)
    pub atlas_offset: [f32; 2],

    /// How big the atlas tile is relative to the atlas.
    /// Something that is 256x128 in a 512x512 atlas would be (0.5, 0.25)
    pub atlas_scale: [f32; 2],

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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Material {
    pub name: String,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        name: String,
        device: &Device,
        bind_group_layout: &wgpu::BindGroupLayout,
        texture: &Texture,
    ) -> Self {
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
            label: Some("Material texture_bind_group"),
        });

        Self { name, bind_group }
    }

    pub fn load_from_file(
        path: String,
        device: &Device,
        queue: &Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture_bytes = image::io::Reader::open(&path).unwrap().decode().unwrap();
        let texture = Texture::from_image(device, queue, &texture_bytes, Some(&path));
        Self::new(path, device, bind_group_layout, &texture)
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instances: Vec<MeshInstance>,
    pub instance_buffer: wgpu::Buffer,
}

impl Mesh {
    pub fn new(
        name: String,
        vertex_buffer: wgpu::Buffer,
        index_buffer: wgpu::Buffer,
        instances: Vec<MeshInstance>,
        instance_buffer: wgpu::Buffer,
    ) -> Self {
        Self {
            name,
            vertex_buffer,
            index_buffer,
            instances,
            instance_buffer,
        }
    }

    pub fn update(&mut self) {}

    pub fn draw<'mats: 'rpass, 'mesh: 'rpass, 'rpass>(
        &'mesh self,
        rpass: &mut wgpu::RenderPass<'rpass>,
        material: &'mats Material,
    ) {
        rpass.set_bind_group(1, &material.bind_group, &[]);
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
