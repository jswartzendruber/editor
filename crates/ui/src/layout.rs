use crate::{
    image_pipeline::{self, ImageInstance},
    quad_pipeline::QuadInstance,
    texture_atlas::{TextureAtlas, TextureId},
};
use std::rc::Rc;

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
        Self {
            texture_id,
            tint: Color::new(255, 255, 255, 255),
        }
    }

    pub fn new_tinted(texture_id: TextureId, tint: Color) -> Self {
        Self { texture_id, tint }
    }
}

pub enum UiType {
    CenteredBox,
    Hbox,
    Vbox,
}

#[derive(Debug)]
pub struct FixedSizedBox {
    width: f32,
    height: f32,
    /// Despite being a vector, this will only hold one item.
    child: Vec<Ui>,
    background_color: Color,
}

impl FixedSizedBox {
    pub fn new(width: f32, height: f32, child: Ui, background_color: Color) -> Self {
        Self {
            width,
            height,
            child: vec![child],
            background_color,
        }
    }
}

impl UiContainer for FixedSizedBox {
    fn elements(&self) -> &Vec<Ui> {
        &self.child
    }

    fn ty(&self) -> UiType {
        UiType::CenteredBox
    }
}

#[derive(Debug)]
pub enum Ui {
    TexturedRectangle(TexturedRectangle),
    FixedSizedBox(FixedSizedBox),
    Rectangle(Rectangle),
    Hbox(Hbox),
    Vbox(Vbox),
    Text(Rc<str>, f32, Color),
    Spacer,
}

impl Ui {
    pub fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: (f32, f32),
        queue: &wgpu::Queue,
    ) -> Vec<Drawables> {
        let mut rects = vec![];

        match self {
            Ui::Hbox(h) => h.layout(
                atlas,
                BoundingBox {
                    min: (0.0, 0.0),
                    max: (view_size.0, view_size.1),
                },
                &mut rects,
                queue,
            ),
            Ui::Vbox(v) => v.layout(
                atlas,
                BoundingBox {
                    min: (0.0, 0.0),
                    max: (view_size.0, view_size.1),
                },
                &mut rects,
                queue,
            ),
            Ui::TexturedRectangle(_) => unimplemented!(),
            Ui::FixedSizedBox(_) => unimplemented!(),
            Ui::Rectangle(_) => unimplemented!(),
            Ui::Text(_, _, _) => unimplemented!(),
            Ui::Spacer => unimplemented!(),
        }

        rects
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
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

pub trait UiContainer {
    fn layout(
        &self,
        atlas: &mut TextureAtlas,
        parent_size: BoundingBox,
        rects: &mut Vec<Drawables>,
        queue: &wgpu::Queue,
    ) {
        for (i, elem) in self.elements().iter().enumerate() {
            let child_bbox = match self.ty() {
                UiType::Hbox => {
                    let child_index = i;
                    let child_width = parent_size.width() / self.elements().len() as f32;
                    let x0 = parent_size.min.0 + child_width * child_index as f32;

                    BoundingBox::new(x0, parent_size.min.1, x0 + child_width, parent_size.max.1)
                }
                UiType::Vbox => {
                    let child_index = self.elements().len() - i - 1;
                    let child_height = parent_size.height() / self.elements().len() as f32;
                    let y0 = parent_size.min.1 + child_height * child_index as f32;

                    BoundingBox::new(parent_size.min.0, y0, parent_size.max.0, y0 + child_height)
                }
                UiType::CenteredBox => parent_size,
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
                Ui::Hbox(hbox) => hbox.layout(atlas, child_bbox, rects, queue),
                Ui::Vbox(vbox) => vbox.layout(atlas, child_bbox, rects, queue),
                Ui::FixedSizedBox(fsb) => {
                    // The ceneter of the space we have
                    let bbox_center = child_bbox.center();
                    let half_width = fsb.width / 2.0;
                    let half_height = fsb.height / 2.0;

                    let fixed_size_bbox = BoundingBox::new(
                        bbox_center.0 - half_width,
                        bbox_center.1 + half_height,
                        bbox_center.0 + half_width,
                        bbox_center.1 - half_height,
                    );

                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: fsb.background_color.to_f32_arr(),
                    }));

                    fsb.layout(atlas, fixed_size_bbox, rects, queue)
                }
                Ui::Text(t, s, bc) => {
                    // background color
                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: bc.to_f32_arr(),
                    }));
                    rects.extend(image_pipeline::layout_text(child_bbox, t, atlas, *s, queue))
                }
                Ui::Spacer => {}
            }
        }
    }

    fn elements(&self) -> &Vec<Ui>;

    fn ty(&self) -> UiType;
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

impl UiContainer for Hbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn ty(&self) -> UiType {
        UiType::Hbox
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

impl UiContainer for Vbox {
    fn elements(&self) -> &Vec<Ui> {
        &self.elements
    }

    fn ty(&self) -> UiType {
        UiType::Vbox
    }
}
