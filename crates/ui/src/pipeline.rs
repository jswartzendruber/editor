// use std::{borrow::Cow, fmt::Debug};
// use wgpu::util::DeviceExt;

// pub trait Uniformable {
//     fn layout(&self) -> &wgpu::BindGroupLayout;
//     fn group(&self) -> &wgpu::BindGroup;
//     fn buffer(&self) -> &wgpu::Buffer;
//     fn index(&self) -> u32;
// }

// pub struct Uniform<T> {
//     layout: wgpu::BindGroupLayout,
//     group: wgpu::BindGroup,
//     buffer: wgpu::Buffer,
//     index: u32,
//     value: T,
// }

// impl<T> Uniformable for Uniform<T> {
//     fn buffer(&self) -> &wgpu::Buffer {
//         &self.buffer
//     }

//     fn group(&self) -> &wgpu::BindGroup {
//         &self.group
//     }

//     fn layout(&self) -> &wgpu::BindGroupLayout {
//         &self.layout
//     }

//     fn index(&self) -> u32 {
//         self.index
//     }
// }

// impl<T> Uniform<T>
// where
//     T: bytemuck::Pod,
// {
//     pub fn new(device: &wgpu::Device, layout: wgpu::BindGroupLayout, value: T, index: u32) -> Self {
//         let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("Camera Buffer"),
//             contents: bytemuck::cast_slice(&[value]),
//             usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
//         });

//         let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//             layout: &layout,
//             entries: &[wgpu::BindGroupEntry {
//                 binding: 0,
//                 resource: buffer.as_entire_binding(),
//             }],
//             label: Some("uniform bind group"),
//         });

//         Self {
//             layout,
//             group,
//             buffer,
//             value,
//             index,
//         }
//     }
// }

// /// The projection matrix used in the shaders.
// #[repr(C)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct CameraRaw {
//     projection: [[f32; 4]; 4],
// }

// impl CameraRaw {
//     pub fn new_ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
//         CameraRaw {
//             projection: [
//                 [2.0 / (right - left), 0.0, 0.0, 0.0],
//                 [0.0, 2.0 / (top - bottom), 0.0, 0.0],
//                 [0.0, 0.0, 1.0 / (near - far), 0.0],
//                 [
//                     (right + left) / (left - right),
//                     (top + bottom) / (bottom - top),
//                     near / (near - far),
//                     1.0,
//                 ],
//             ],
//         }
//     }
// }

// pub struct SlotBuffer {
//     buffer: wgpu::Buffer,
//     index: u32,
// }

// impl SlotBuffer {
//     pub fn new(buffer: wgpu::Buffer, index: u32) -> Self {
//         Self { buffer, index }
//     }
// }

// pub trait Meshable {
//     fn instance_buffer(&self) -> &SlotBuffer;
//     fn vertex_buffer(&self) -> &SlotBuffer;
//     fn index_buffer(&self) -> &wgpu::Buffer;
//     fn update(&self, queue: &wgpu::Queue);
//     fn draw<'s: 'rpass, 'rpass>(&'s self, rpass: &mut wgpu::RenderPass<'rpass>);
//     fn desc(&self) -> wgpu::VertexBufferLayout;
// }

// pub struct PMesh<T> {
//     instances: Vec<T>,
//     instance_buffer: SlotBuffer,
//     vertex_buffer: SlotBuffer,
//     index_buffer: wgpu::Buffer,
//     vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
// }

// impl<T> Meshable for PMesh<T>
// where
//     T: bytemuck::Pod + Debug,
// {
//     fn index_buffer(&self) -> &wgpu::Buffer {
//         &self.index_buffer
//     }

//     fn instance_buffer(&self) -> &SlotBuffer {
//         &self.instance_buffer
//     }

//     fn vertex_buffer(&self) -> &SlotBuffer {
//         &self.vertex_buffer
//     }

//     fn update(&self, queue: &wgpu::Queue) {
//         queue.write_buffer(
//             &self.instance_buffer.buffer,
//             0,
//             bytemuck::cast_slice(&self.instances),
//         );
//     }

//     fn draw<'s: 'rpass, 'rpass>(&'s self, rpass: &mut wgpu::RenderPass<'rpass>) {
//         rpass.set_vertex_buffer(
//             self.vertex_buffer.index,
//             self.vertex_buffer.buffer.slice(..),
//         );
//         rpass.set_vertex_buffer(
//             self.instance_buffer.index,
//             self.instance_buffer.buffer.slice(..),
//         );
//         rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//         rpass.draw_indexed(
//             0..PMeshVertex::INDICES.len() as u32,
//             0,
//             0..self.instances.len() as u32,
//         );
//     }

//     fn desc(&self) -> wgpu::VertexBufferLayout {
//         self.vertex_buffer_layout.clone()
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct PMeshInstace {
//     pub position: [f32; 2],
//     pub size: [f32; 2],
//     pub color: [f32; 4],
// }

// impl PMeshInstace {
//     const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
//         5 => Float32x2,
//         6 => Float32x2,
//         7 => Float32x4,
//     ];

//     pub fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<PMeshInstace>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Instance,
//             attributes: &Self::ATTRIBS,
//         }
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct PMeshVertex {
//     position: [f32; 2],
//     tex_coords: [f32; 2],
// }

// impl PMeshVertex {
//     #[rustfmt::skip]
//     pub const VERTICES: &'static [PMeshVertex] = &[
//         PMeshVertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
//         PMeshVertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
//         PMeshVertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
//         PMeshVertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
//     ];

//     pub const INDICES: &'static [u16] = &[0, 1, 2, 2, 3, 1];

//     const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
//         0 => Float32x2,
//         1 => Float32x2,
//     ];

//     pub fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<PMeshVertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &Self::ATTRIBS,
//         }
//     }
// }

// impl<T> PMesh<T>
// where
//     T: bytemuck::Pod + Debug,
// {
//     pub fn new(
//         device: &wgpu::Device,
//         instances: Vec<T>,
//         vertex_buffer_layout: wgpu::VertexBufferLayout<'static>,
//     ) -> Self {
//         let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("mesh Vertex Buffer"),
//             contents: bytemuck::cast_slice(PMeshVertex::VERTICES),
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//         });
//         let vertex_buffer = SlotBuffer::new(vertex_buffer, 0);

//         println!("{:?}", instances);

//         let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("mesh Instance Buffer"),
//             contents: bytemuck::cast_slice(&instances),
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//         });
//         let instance_buffer = SlotBuffer::new(instance_buffer, 1);

//         let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//             label: Some("mesh Index Buffer"),
//             contents: bytemuck::cast_slice(PMeshVertex::INDICES),
//             usage: wgpu::BufferUsages::INDEX,
//         });

//         Self {
//             instances,
//             instance_buffer,
//             vertex_buffer,
//             index_buffer,
//             vertex_buffer_layout,
//         }
//     }
// }

// pub struct Pipeline {
//     pipeline: wgpu::RenderPipeline,
//     uniforms: Vec<Box<dyn Uniformable>>,
//     meshes: Vec<Box<dyn Meshable>>,
// }

// impl Pipeline {
//     pub fn new<'a>(
//         device: &wgpu::Device,
//         shader_text: Cow<'a, str>,
//         uniforms: Vec<Box<dyn Uniformable>>,
//         meshes: Vec<Box<dyn Meshable>>,
//     ) -> Self {
//         let mut bind_group_layouts = Vec::with_capacity(uniforms.len());
//         for uniform in &uniforms {
//             bind_group_layouts.push(uniform.layout());
//         }
//         let mut vertex_buffer_layouts = Vec::with_capacity(uniforms.len());
//         vertex_buffer_layouts.push(PMeshVertex::desc());
//         for mesh in &meshes {
//             vertex_buffer_layouts.push(mesh.desc())
//         }

//         let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
//             label: None,
//             bind_group_layouts: &bind_group_layouts,
//             push_constant_ranges: &[],
//         });

//         let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
//             label: None,
//             source: wgpu::ShaderSource::Wgsl(shader_text),
//         });

//         let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
//             label: None,
//             layout: Some(&pipeline_layout),
//             vertex: wgpu::VertexState {
//                 module: &shader,
//                 entry_point: "vs_main",
//                 buffers: vertex_buffer_layouts.as_slice(),
//             },
//             fragment: Some(wgpu::FragmentState {
//                 module: &shader,
//                 entry_point: "fs_main",
//                 targets: &[Some(wgpu::TextureFormat::Bgra8UnormSrgb.into())],
//             }),
//             primitive: wgpu::PrimitiveState::default(),
//             depth_stencil: None,
//             multisample: wgpu::MultisampleState::default(),
//             multiview: None,
//         });

//         Self {
//             pipeline,
//             uniforms,
//             meshes,
//         }
//     }

//     pub fn update(&mut self, queue: &wgpu::Queue) {
//         for mesh in &self.meshes {
//             mesh.update(queue);
//         }
//     }

//     pub fn draw<'rp, 'rpb, 's: 'rp>(&'s self, rpass: &'rpb mut wgpu::RenderPass<'rp>) {
//         rpass.set_pipeline(&self.pipeline);
//         for uniform in &self.uniforms {
//             rpass.set_bind_group(uniform.index(), uniform.group(), &[]);
//         }
//         for mesh in &self.meshes {
//             mesh.draw(rpass);
//         }
//     }
// }
