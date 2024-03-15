use crate::{
    image_pipeline::ImageInstance,
    quad_pipeline::QuadInstance,
    texture_atlas::{TextureAtlas, TextureId},
};

#[derive(Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    fn to_f32_arr(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }
}

#[derive(Debug)]
pub struct Rectangle {
    color: Color,
}

impl Rectangle {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

#[derive(Debug)]
pub struct TexturedRectangle {
    texture_id: TextureId,
    tint: Color,
}

impl TexturedRectangle {
    pub fn new(texture_id: TextureId) -> Self {
        Self { texture_id, tint: Color::new(255, 255, 255, 255) }
    }

    pub fn new_tinted(texture_id: TextureId, tint: Color) -> Self {
        Self { texture_id, tint }
    }
}

#[derive(Debug)]
pub enum Ui {
    TexturedRectangle(TexturedRectangle),
    Rectangle(Rectangle),
    Hbox(Hbox),
    Vbox(Vbox),
    Spacer,
}

impl Ui {
    pub fn layout(&self, atlas: &TextureAtlas, view_size: (f32, f32)) -> Vec<Drawables> {
        let mut rects = vec![];

        match self {
            Ui::Hbox(h) => h.layout(
                atlas,
                BoundingBox {
                    min: (0.0, 0.0),
                    max: (view_size.0, view_size.1),
                },
                &mut rects,
            ),
            Ui::Vbox(v) => v.layout(
                atlas,
                BoundingBox {
                    min: (0.0, 0.0),
                    max: (view_size.0, view_size.1),
                },
                &mut rects,
            ),
            Ui::TexturedRectangle(_) => unimplemented!(),
            Ui::Rectangle(_) => unimplemented!(),
            Ui::Spacer => unimplemented!(),
        }

        rects
    }
}

#[derive(Debug, PartialEq)]
pub struct BoundingBox {
    pub min: (f32, f32),
    pub max: (f32, f32),
}

impl BoundingBox {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self {
            min: (x0, y0),
            max: (x1, y1),
        }
    }

    pub fn width(&self) -> f32 {
        self.max.0 - self.min.0
    }

    pub fn height(&self) -> f32 {
        self.max.1 - self.min.1
    }

    pub fn center(&self) -> (f32, f32) {
        (
            (self.min.0 + self.max.0) / 2.0,
            (self.min.1 + self.max.1) / 2.0,
        )
    }

    pub fn top_left(&self) -> (f32, f32) {
        (self.min.0, self.max.1)
    }

    // Returns true if pos is inside the bbox.
    pub fn inside(&self, pos: (f32, f32)) -> bool {
        let x_inside = self.min.0 <= pos.0 && pos.0 <= self.max.0;
        let y_inside = self.min.1 <= pos.1 && pos.1 <= self.max.1;

        x_inside && y_inside
    }
}

pub enum Drawables {
    Rect(QuadInstance),
    TexturedRect(ImageInstance),
}

pub trait UiElement {
    fn layout(&self, atlas: &TextureAtlas, parent_size: BoundingBox, rects: &mut Vec<Drawables>) {
        for (i, elem) in self.elements().iter().enumerate() {
            let child_bbox = if self.is_hbox() {
                let child_index = i;
                let child_width = parent_size.width() / self.elements().len() as f32;
                let x0 = parent_size.min.0 + child_width * child_index as f32;

                BoundingBox::new(x0, parent_size.min.1, x0 + child_width, parent_size.max.1)
            } else {
                let child_index = self.elements().len() - i - 1;
                let child_height = parent_size.height() / self.elements().len() as f32;
                let y0 = parent_size.min.1 + child_height * child_index as f32;

                BoundingBox::new(parent_size.min.0, y0, parent_size.max.0, y0 + child_height)
            };

            match elem {
                Ui::TexturedRectangle(tr) => {
                    rects.push(Drawables::TexturedRect(ImageInstance::add_instance(
                        atlas,
                        tr.texture_id,
                        [child_bbox.min.0, child_bbox.min.1],
                        [child_bbox.width(), child_bbox.height()],
                        tr.tint.to_f32_arr(),
                    )));
                }
                Ui::Rectangle(r) => {
                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: r.color.to_f32_arr(),
                    }));
                }
                Ui::Hbox(hbox) => hbox.layout(atlas, child_bbox, rects),
                Ui::Vbox(vbox) => vbox.layout(atlas, child_bbox, rects),
                Ui::Spacer => {}
            }
        }
    }

    fn elements(&self) -> &Vec<Ui>;

    fn is_hbox(&self) -> bool;
}

#[derive(Debug)]
pub struct Hbox {
    elements: Vec<Ui>,
}

impl Hbox {
    pub fn new(elements: Vec<Ui>) -> Self {
        Self { elements }
    }
}

impl UiElement for Hbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn is_hbox(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Vbox {
    elements: Vec<Ui>,
}

impl Vbox {
    pub fn new(elements: Vec<Ui>) -> Self {
        Self { elements }
    }
}

impl UiElement for Vbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn is_hbox(&self) -> bool {
        false
    }
}
