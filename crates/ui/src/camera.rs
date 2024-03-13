use std::rc::Rc;
use wgpu::util::DeviceExt;

/// The projection matrix used in the shaders.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniformRaw {
    projection: [[f32; 4]; 4],
}

impl CameraUniformRaw {
    pub fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        CameraUniformRaw {
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

pub struct CameraUniform {
    bind_group_layout: Rc<wgpu::BindGroupLayout>,
    bind_group: Rc<wgpu::BindGroup>,
    buffer: wgpu::Buffer,
    raw: CameraUniformRaw,
}

impl CameraUniform {
    pub fn new(
        device: &wgpu::Device,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let raw = CameraUniformRaw::new_ortho(left, right, bottom, top, near, far);
        let buffer = Self::create_buffer(device, &raw);
        let bind_group_layout = Self::create_bind_group_layout(device);
        let bind_group = Self::create_bind_group(device, &buffer, &bind_group_layout);

        Self {
            bind_group_layout,
            bind_group,
            buffer,
            raw,
        }
    }

    fn create_buffer(device: &wgpu::Device, initial: &CameraUniformRaw) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[*initial]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> Rc<wgpu::BindGroupLayout> {
        Rc::new(
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
            }),
        )
    }

    fn create_bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Rc<wgpu::BindGroup> {
        Rc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        }))
    }

    pub fn update_matrix(
        &mut self,
        queue: &wgpu::Queue,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) {
        self.raw = CameraUniformRaw::new_ortho(left, right, bottom, top, near, far);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.raw]));
    }

    pub fn bind_group(&self) -> Rc<wgpu::BindGroup> {
        self.bind_group.clone()
    }

    pub fn bind_group_layout(&self) -> Rc<wgpu::BindGroupLayout> {
        self.bind_group_layout.clone()
    }
}
