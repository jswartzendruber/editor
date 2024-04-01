use crate::texture::Texture;
use etagere::{Allocation, AtlasAllocator};
use freetype::face::LoadFlag;
use image::{DynamicImage, ImageError, RgbaImage};
use lru::LruCache;

#[derive(Debug)]
pub enum AtlasError {
    ImageLoadingError(ImageError),
}

/// An index into the texture atlas's allocated texture array
#[derive(Debug, Clone, Copy)]
pub struct TextureId(usize);

/// Contains information from the font rasterizer about how
/// to draw and position the glyph.
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance: (f32, f32),
    pub size: (f32, f32),
    pub pos: (f32, f32),
}

#[derive(Debug, Clone, Copy)]
pub struct FontGlyph {
    pub metrics: GlyphMetrics,
    pub texture_id: TextureId,
    allocation_id: etagere::AllocId,
}

impl FontGlyph {
    pub fn new(
        metrics: GlyphMetrics,
        texture_id: TextureId,
        allocation_id: etagere::AllocId,
    ) -> Self {
        Self {
            metrics,
            texture_id,
            allocation_id,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
/// A key for a glyph being inserted into the atlas. We store the character
/// the glyph is of, as well as the font size because glyphs of different
/// font sizes must be re-rasterized instead of simply scaled up.
pub struct GlyphMapKey {
    c: char,
    font_size: u32,
}

/// A dynamically packed bundle of images. If the atlas is full, the least recently used
/// glyphs will be evicted until there is room to allocate a new glyph.
pub struct TextureAtlas {
    /// Stores allocation info from etagere. 
    // TODO: If allocations are evicted, this would still contain old references.
    allocations: Vec<Allocation>,

    regular_face: freetype::Face,
    emoji_face: freetype::Face,

    /// Keeps track of the dynamic allocations we request.
    allocator: AtlasAllocator,
    /// The current atlas texture state
    texture: Texture,
    /// The size of the atlas
    size: u16,
    /// Keeps track of how recently the chars have been used
    cache: LruCache<GlyphMapKey, FontGlyph>,
}

impl TextureAtlas {
    /// Create a new texture atlas. This will also initialize the freetype library, a regular
    /// and an emoji font face, and set up the atlas allocator and cache.
    /// TODO: separate the font related setup?
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: u16) -> Self {
        let library = freetype::Library::init().unwrap();

        let regular_face = library.new_face("res/RobotoMono-Regular.ttf", 0).unwrap();

        let emoji_face = if cfg!(windows) {
            library
                .new_face("C:\\Windows\\Fonts\\seguiemj.ttf", 0)
                .unwrap()
        } else {
            library.new_face("res/NotoColorEmoji.ttf", 0).unwrap()
        };

        Self {
            allocations: vec![],
            regular_face,
            emoji_face,

            allocator: AtlasAllocator::new(etagere::size2(size as i32, size as i32)),
            texture: Texture::from_size(device, queue, size),
            size,
            cache: LruCache::unbounded(),
        }
    }

    /// Using the provided image and character, loads the image into the texture atlas and
    /// saves the character in the glyph cache.
    fn load_char_from_image(
        &mut self,
        queue: &wgpu::Queue,
        img: &RgbaImage,
        c: char,
        metrics: GlyphMetrics,
        font_size: f32,
    ) -> Result<TextureId, AtlasError> {
        let texture_id = self.load_from_image(queue, img)?;
        self.cache.put(
            GlyphMapKey {
                c,
                font_size: font_size as u32,
            },
            FontGlyph::new(metrics, texture_id, self.allocations[texture_id.0].id),
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
        let allocation = self.allocate(queue, img)?;
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

    /// Get the allocation details of a given texture_id, i.e. size and id
    pub fn get_allocation(&self, texture_id: TextureId) -> Allocation {
        self.allocations[texture_id.0]
    }

    /// Get the size of the entire atlas
    pub fn size(&self) -> u16 {
        self.size
    }

    /// Get the atlas's entire GPU texture
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Given a character and font size, uses freetype to rasterize the glyph. Returns
    /// a reference to the rasterized glyph, which can be used to get the glyph bitmap,
    /// font metrics, etc.
    fn load_freetype_glyph(
        face: &freetype::Face,
        font_size: f32,
        c: char,
    ) -> Option<&freetype::GlyphSlot> {
        let glyph_index = face.get_char_index(c as usize)?;

        let mut load_flags = LoadFlag::DEFAULT | LoadFlag::RENDER;
        if face.has_color() {
            // This is the only size noto color emoji provides.
            load_flags |= LoadFlag::COLOR;
            face.set_char_size(109 * 64, 0, 0, 0).ok()?;
        } else {
            face.set_char_size(font_size as isize * 64, 0, 0, 0).ok()?;
        }

        face.load_glyph(glyph_index, load_flags).ok()?;

        face.glyph()
            .render_glyph(freetype::RenderMode::Normal)
            .ok()?;

        Some(face.glyph())
    }

    /// Given the current char and font size, this function checks if the glyph has
    /// been saved in the atlas. If it has, we return the glyph metrics.
    /// If the glyph is not in the atlas, we load the glyph using freetype, rasterize
    /// the glyph, save it in the atlas, and then return the resulting glyph metrics.
    pub fn map_get_or_insert_glyph(
        &mut self,
        c: char,
        font_size: f32,
        queue: &wgpu::Queue,
    ) -> Option<FontGlyph> {
        let glyph_key = GlyphMapKey {
            c,
            font_size: font_size as u32,
        };

        if let Some(res) = self.cache.get(&glyph_key) {
            Some(*res)
        } else {
            let (glyph, is_emoji) = if let Some(glyph) =
                Self::load_freetype_glyph(&self.regular_face, font_size, c)
            {
                (glyph, false)
            } else if let Some(glyph) = Self::load_freetype_glyph(&self.emoji_face, font_size, c) {
                (glyph, true)
            } else {
                return None;
            };

            let mut glyph_width = glyph.bitmap().width() as f32;
            let mut glyph_height = glyph.bitmap().rows() as f32;
            let mut advance_x = glyph.advance().x as f32 / 64.0;
            let mut advance_y = glyph.advance().y as f32 / 64.0;
            let mut bitmap_left = glyph.bitmap_left() as f32;
            let mut bitmap_top = glyph.bitmap_top() as f32;

            let image = if is_emoji {
                // Image comes in BGRA format. Convert it to RGBA.
                RgbaImage::from_raw(
                    glyph_width as u32,
                    glyph_height as u32,
                    glyph
                        .bitmap()
                        .buffer()
                        .chunks(4)
                        .flat_map(|chunk| {
                            use std::iter::once;
                            let chunk = chunk.to_vec();
                            match chunk.len() {
                                4 => once(chunk[2])
                                    .chain(once(chunk[1]))
                                    .chain(once(chunk[0]))
                                    .chain(once(chunk[3]))
                                    .collect(),
                                _ => Vec::new(),
                            }
                        })
                        .collect(),
                )
                .unwrap()
            } else {
                RgbaImage::from_raw(
                    glyph_width as u32,
                    glyph_height as u32,
                    glyph
                        .bitmap()
                        .buffer()
                        .iter()
                        .flat_map(|byte| [255, 255, 255, *byte])
                        .collect(),
                )
                .unwrap()
            };

            let image = if is_emoji {
                let line_height = font_size * 1.2;
                let new_width = ((glyph_width * line_height) / glyph_height).ceil();
                let new_height = line_height;

                glyph_width = new_width;
                glyph_height = new_height;

                advance_x = glyph_width;
                advance_y = 0.0;

                bitmap_left = 0.0;
                bitmap_top = font_size;

                let image = DynamicImage::from(image);
                let image = image.resize(
                    glyph_width as u32,
                    glyph_height as u32,
                    image::imageops::FilterType::Gaussian,
                );
                image.to_rgba8()
            } else {
                image
            };

            let metrics = GlyphMetrics {
                advance: (advance_x, advance_y),
                size: (glyph_width, glyph_height),
                pos: (bitmap_left, bitmap_top),
            };

            self.load_char_from_image(queue, &image, c, metrics, font_size)
                .unwrap();
            self.cache.get(&glyph_key).copied()
        }
    }

    /// Allocates a chunk of space within the atlas and stores the image into the atlas
    /// Returns an error or the size of the successfull allocation
    fn allocate(&mut self, queue: &wgpu::Queue, img: &RgbaImage) -> Result<Allocation, AtlasError> {
        let img_size = img.dimensions();

        // Add a small amount of padding to the image to avoid bleeding when looking up in the atlas
        let allocation_size = etagere::size2(img_size.0 as i32 + 2, img_size.1 as i32 + 2);

        // If there is no space, deallocate until we have room to allocate.
        loop {
            match self.allocator.allocate(allocation_size) {
                Some(mut allocation) => {
                    // We have space, complete the allocation

                    // Adjust the allocated rectangle to hide the padding
                    // TODO: better way of doing this that is not lying about the size of the allocation and re-using
                    // the allocation type from etagere?
                    allocation.rectangle.min.x += 1;
                    allocation.rectangle.min.y += 1;
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

                    return Ok(allocation);
                }
                None => {
                    // Evict the least recently used glyph.
                    let entry = self.cache.pop_lru();
                    if let Some((_, value)) = entry {
                        self.allocator.deallocate(value.allocation_id);
                    }
                }
            }
        }
    }
}
