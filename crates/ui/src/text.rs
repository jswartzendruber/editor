use crate::{
    mesh::{Material, Mesh, MeshInstance, MeshVertex},
    texture_atlas::TextureAtlas,
};
use std::{borrow::Cow, rc::Rc};

pub struct AtlasPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_bind_group: Rc<wgpu::BindGroup>,

    material: Material,
    mesh: Mesh,
}

impl AtlasPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_bind_group: Rc<wgpu::BindGroup>,
        camera_bind_group_layout: Rc<wgpu::BindGroupLayout>,
    ) -> AtlasPipeline {
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_bind_group_layout, &atlas_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("text.wgsl"))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                targets: &[Some(wgpu::TextureFormat::Bgra8UnormSrgb.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let mut atlas = TextureAtlas::new(&device, &queue, 1024);
        let bamboo_atlas_idx = atlas
            .load_image_from_file(&queue, "res/bamboo.png")
            .unwrap();
        let tree_atlas_idx = atlas
            .load_image_from_file(&queue, "res/happy-tree.png")
            .unwrap();
        let hello_atlas_idx = atlas.load_image_from_file(&queue, "res/hello.png").unwrap();
        let rect_atlas_idx = atlas.load_image_from_file(&queue, "res/rect.png").unwrap();

        let material = Material::new(
            "atlas".to_string(),
            &device,
            &atlas_bind_group_layout,
            atlas.texture(),
        );

        let mut mesh = Mesh::new(&device, "Atlas mesh".to_string(), atlas);

        mesh.add_instance(
            bamboo_atlas_idx,
            [0.0, 0.0],
            [300.0, 300.0],
            [1.0, 1.0, 1.0, 1.0],
        );
        mesh.add_instance(
            tree_atlas_idx,
            [300.0, 300.0],
            [300.0, 300.0],
            [1.0, 1.0, 1.0, 1.0],
        );
        mesh.add_instance(
            hello_atlas_idx,
            [0.0, 300.0],
            [300.0, 300.0],
            [1.0, 1.0, 1.0, 1.0],
        );
        mesh.add_instance(
            rect_atlas_idx,
            [300.0, 150.0],
            [300.0, 150.0],
            [1.0, 1.0, 1.0, 1.0],
        );

        AtlasPipeline {
            pipeline,
            camera_bind_group,
            mesh,
            material,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.mesh.update(queue);
    }

    pub fn draw<'rp, 'rpb, 's: 'rp>(&'s self, rpass: &'rpb mut wgpu::RenderPass<'rp>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.camera_bind_group, &[]);
        self.mesh.draw(rpass, &self.material);
    }
}
