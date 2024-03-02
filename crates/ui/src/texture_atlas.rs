use crate::texture::Texture;
use etagere::{Allocation, AtlasAllocator};
use image::RgbaImage;
use lru::LruCache;
use std::hash::Hash;

/// This is meant to be used for storing glyphs. using the get function, you can
/// look up a stored glyph by it's character which will return information about
/// the glyph such as it's size and advances.
pub struct CachedTextureAtlas<K, V> {
    /// The un-cached texture atlas
    atlas: TextureAtlas,
    /// An unbounded least recently used cache for the allocations
    cache: LruCache<K, (V, Allocation)>,
}

impl<K, V> CachedTextureAtlas<K, V>
where
    K: Eq + Hash,
{
    /// Creates a new square cached texture atlas with dimensions 'size * size'
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        Self {
            atlas: TextureAtlas::new(device, queue, size),
            cache: LruCache::unbounded(),
        }
    }

    /// Allocates space in the atlas and inserts the provided image,
    /// additionally caches the specified key and value for quick
    /// lookups.
    pub fn allocate(
        &mut self,
        queue: &wgpu::Queue,
        img: &RgbaImage,
        k: K,
        v: V,
    ) -> Result<(), AtlasError> {
        let allocation = self.atlas.allocate(queue, img)?;
        self.cache.put(k, (v, allocation));

        Ok(())
    }
}

/// This is used to store images/glyphs together in one texture.
pub struct TextureAtlas {
    /// Keeps track of the dynamic allocations we request.
    allocator: AtlasAllocator,
    /// The current atlas texture state
    texture: Texture,
}

#[derive(Debug)]
pub enum AtlasError {
    /// There is no more space in the atlas.
    /// TODO: use etagere and make this dynamic instead of quitting
    /// when we run out of space
    AtlasFull,
}

impl TextureAtlas {
    /// Creates a new square texture atlas with dimensions 'size * size'
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        Self {
            allocator: AtlasAllocator::new(etagere::size2(size as i32, size as i32)),
            texture: Texture::from_size(device, queue, size),
        }
    }

    /// Allocates a chunk of space within the atlas and stores the image into the atlas
    /// Returns an error or the size of the successfull allocation
    pub fn allocate(
        &mut self,
        queue: &wgpu::Queue,
        img: &RgbaImage,
    ) -> Result<Allocation, AtlasError> {
        let img_size = img.dimensions();
        let allocation_size = etagere::size2(img_size.0 as i32, img_size.1 as i32);

        let allocation = self
            .allocator
            .allocate(allocation_size)
            .ok_or(AtlasError::AtlasFull)?;

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
            &img,
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

    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}
