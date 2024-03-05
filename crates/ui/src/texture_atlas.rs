use crate::texture::Texture;
use etagere::{Allocation, AtlasAllocator};
use image::{ImageError, RgbaImage};

#[derive(Debug)]
pub enum AtlasError {
    AllocationError(AllocationError),
    ImageLoadingError(ImageError),
}

#[derive(Debug)]
pub enum AllocationError {
    /// There is no more space in the atlas.
    /// TODO: use etagere and make this dynamic instead of quitting
    /// when we run out of space
    AtlasFull,
}

pub struct TextureId(usize);

pub struct TextureAtlas {
    atlas: AtlasInternal,
    allocations: Vec<Allocation>,
}

impl TextureAtlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        Self {
            atlas: AtlasInternal::new(device, queue, size),
            allocations: vec![],
        }
    }

    /// Load an image from a file, and allocate it in the atlas. Returns an ID which
    /// allows for looking up the size and other attributes of the allocation.
    pub fn load_image_from_file(
        &mut self,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<TextureId, AtlasError> {
        let img = image::io::Reader::open(path)
            .unwrap()
            .decode()
            .map_err(AtlasError::ImageLoadingError)?;
        let allocation = self.atlas.allocate(queue, &img.to_rgba8())?;
        let idx = self.allocations.len();
        self.allocations.push(allocation);
        Ok(TextureId(idx))
    }

    pub fn get_allocation(&self, texture_id: TextureId) -> Allocation {
        self.allocations[texture_id.0]
    }

    pub fn size(&self) -> u16 {
        self.atlas.size
    }

    pub fn texture(&self) -> &Texture {
        &self.atlas.texture
    }
}

/// This is used to store images/glyphs together in one texture.
struct AtlasInternal {
    /// Keeps track of the dynamic allocations we request.
    allocator: AtlasAllocator,
    /// The current atlas texture state
    texture: Texture,
    /// The size of the atlas
    size: u16,
}

impl AtlasInternal {
    /// Creates a new square texture atlas with dimensions 'size * size'
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        Self {
            allocator: AtlasAllocator::new(etagere::size2(size as i32, size as i32)),
            texture: Texture::from_size(device, queue, size),
            size,
        }
    }

    /// Allocates a chunk of space within the atlas and stores the image into the atlas
    /// Returns an error or the size of the successfull allocation
    fn allocate(&mut self, queue: &wgpu::Queue, img: &RgbaImage) -> Result<Allocation, AtlasError> {
        let img_size = img.dimensions();
        let allocation_size = etagere::size2(img_size.0 as i32, img_size.1 as i32);

        let allocation = self
            .allocator
            .allocate(allocation_size)
            .ok_or(AtlasError::AllocationError(AllocationError::AtlasFull))?;

        let xmin = allocation.rectangle.min.x;
        let ymin = allocation.rectangle.min.y;

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: xmin as u32,
                    y: ymin as u32,
                    z: 0,
                },
            },
            img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * img.width()),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            },
        );

        Ok(allocation)
    }
}
