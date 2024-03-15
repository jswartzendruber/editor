/// The projection matrix used in the shaders.
pub struct CameraUniform {
    raw: CameraRaw,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group_idx: u32,
}

impl CameraUniform {
    pub fn new(device: &wgpu::Device, width: f32, height: f32, bind_group_idx: u32) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: bind_group_idx,
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

        let camera_raw = CameraRaw::new_ortho(0.0, width, height, 0.0, 1.0, -1.0);

        let camera_buffer = wgpu::util::DeviceExt::create_buffer_init(
            device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_raw]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        );

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("uniform bind group"),
        });

        Self {
            raw: camera_raw,
            buffer: camera_buffer,
            bind_group: camera_bind_group,
            bind_group_layout: camera_bind_group_layout,
            bind_group_idx,
        }
    }

    pub fn update_size(&mut self, queue: &wgpu::Queue, width: f32, height: f32) {
        self.raw = CameraRaw::new_ortho(0.0, width, height, 0.0, 1.0, -1.0);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.raw]));
    }

    pub fn index(&self) -> u32 {
        self.bind_group_idx
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraRaw {
    projection: [[f32; 4]; 4],
}

impl CameraRaw {
    fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
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
