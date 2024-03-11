use crate::{
    texture::Texture,
    texture_atlas::{TextureAtlas, TextureId},
};
use wgpu::{util::DeviceExt, Device, Queue};

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

    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
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
    const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        5 => Float32x2,
        6 => Float32x2,
        7 => Float32x2,
        8 => Float32x2,
        9 => Float32x4
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MeshInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
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
    pub atlas: TextureAtlas,
    dirty: bool,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, name: String, atlas: TextureAtlas) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atlas Instance Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: (std::mem::size_of::<MeshInstance>() * 1024) as u64,
            mapped_at_creation: false,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas Vertex Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("atlas Index Buffer"),
            contents: bytemuck::cast_slice(MeshVertex::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            name,
            vertex_buffer,
            index_buffer,
            instances: vec![],
            instance_buffer,
            atlas,
            dirty: false,
        }
    }

    pub fn add_instance(
        &mut self,
        texture_id: TextureId,
        position: [f32; 2],
        size: [f32; 2],
        color: [f32; 4],
    ) {
        let atlas_size = self.atlas.size() as f32;
        let subimg_dimensions = self.atlas.get_allocation(texture_id).rectangle;

        self.dirty = true;
        self.instances.push(MeshInstance {
            position,
            size,
            atlas_offset: [
                subimg_dimensions.min.x as f32 / atlas_size,
                subimg_dimensions.min.y as f32 / atlas_size,
            ],
            atlas_scale: [
                subimg_dimensions.width() as f32 / atlas_size,
                subimg_dimensions.height() as f32 / atlas_size,
            ],
            color,
        });
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances),
        );
        self.dirty = false;
    }

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
