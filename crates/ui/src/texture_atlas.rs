use crate::texture::Texture;
use etagere::{Allocation, AtlasAllocator};
use image::{ImageError, RgbaImage};
use std::collections::HashMap;

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

#[derive(Debug, Clone, Copy)]
pub struct TextureId(usize);

#[derive(Debug, Clone, Copy)]
pub struct SubpixelAlignment(u8);

// Thanks https://www.warp.dev/blog/adventures-text-rendering-kerning-glyph-atlases
impl SubpixelAlignment {
    const STEPS: u8 = 3;

    pub fn new(x: f32) -> Self {
        let scaled = x * Self::STEPS as f32;
        let align = (scaled.round() % Self::STEPS as f32) as u8;
        Self(align)
    }

    pub fn to_offset(&self) -> f32 {
        self.0 as f32 / Self::STEPS as f32
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FontGlyph {
    pub subpixel_alignment: SubpixelAlignment,
    pub metrics: fontdue::Metrics,
    pub texture_id: TextureId,
    pub font_size: f32,
}

impl FontGlyph {
    pub fn new(
        subpixel_alignment: SubpixelAlignment,
        metrics: fontdue::Metrics,
        texture_id: TextureId,
        font_size: f32,
    ) -> Self {
        Self {
            subpixel_alignment,
            metrics,
            texture_id,
            font_size,
        }
    }
}

pub struct TextureAtlas {
    atlas: AtlasInternal,
    allocations: Vec<Allocation>,
    glyph_map: HashMap<char, FontGlyph>,
    font: fontdue::Font,
}

impl TextureAtlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        Self {
            atlas: AtlasInternal::new(device, queue, size),
            allocations: vec![],
            glyph_map: HashMap::new(),
            font: fontdue::Font::from_bytes(
                include_bytes!("../../../res/RobotoMono-Regular.ttf") as &[u8],
                fontdue::FontSettings::default(),
            )
            .unwrap(),
        }
    }

    pub fn load_char_from_image(
        &mut self,
        queue: &wgpu::Queue,
        img: &RgbaImage,
        c: char,
        metrics: fontdue::Metrics,
        font_size: f32,
        subpixel_alignment: SubpixelAlignment,
    ) -> Result<TextureId, AtlasError> {
        let texture_id = self.load_from_image(queue, img)?;
        self.glyph_map.insert(
            c,
            FontGlyph::new(subpixel_alignment, metrics, texture_id, font_size),
        );
        Ok(texture_id)
    }

    /// Allocates the passed in image on the atlas. Returns an ID which allows for
    /// looking up the size and other attributes of the allocation.
    fn load_from_image(
        &mut self,
        queue: &wgpu::Queue,
        img: &RgbaImage,
    ) -> Result<TextureId, AtlasError> {
        let allocation = self.atlas.allocate(queue, img)?;
        let idx = self.allocations.len();
        self.allocations.push(allocation);
        Ok(TextureId(idx))
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
        self.load_from_image(queue, &img.to_rgba8())
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

    pub fn map_get_or_insert_glyph(
        &mut self,
        c: char,
        font_size: f32,
        queue: &wgpu::Queue,
    ) -> Option<FontGlyph> {
        if let Some(res) = self.glyph_map.get(&c) {
            Some(*res)
        } else {
            let (metrics, bitmap) = self.font.rasterize(c, font_size);
            let subpixel_alignment = SubpixelAlignment::new(metrics.advance_width);
            let image = RgbaImage::from_raw(
                metrics.width as u32,
                metrics.height as u32,
                bitmap
                    .into_iter()
                    .flat_map(|byte| [255, 255, 255, byte])
                    .collect(),
            )
            .unwrap();
            self.load_char_from_image(&queue, &image, c, metrics, font_size, subpixel_alignment)
                .unwrap();
            self.glyph_map.get(&c).copied()
        }
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

        // Add a small amount of padding to the image to avoid bleeding when looking up in the atlas
        let allocation_size = etagere::size2(img_size.0 as i32 + 2, img_size.1 as i32 + 2);

        let mut allocation = self
            .allocator
            .allocate(allocation_size)
            .ok_or(AtlasError::AllocationError(AllocationError::AtlasFull))?;

        // Adjust the allocated rectangle to hide the padding
        // TODO: better way of doing this that is not lying about the size of the allocation and re-using
        // the allocation type from etagere?
        allocation.rectangle.min.x = allocation.rectangle.min.x + 1;
        allocation.rectangle.min.y = allocation.rectangle.min.y + 1;
        allocation.rectangle.max.x = allocation.rectangle.min.x + img_size.0 as i32;
        allocation.rectangle.max.y = allocation.rectangle.min.y + img_size.1 as i32;

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
