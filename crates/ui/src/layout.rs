use crate::{
    image_pipeline::{self, ImageInstance},
    quad_pipeline::QuadInstance,
    texture_atlas::{TextureAtlas, TextureId},
};
use std::rc::Rc;
use winit::window::Window;

#[derive(Debug, Clone, Copy)]
pub struct UiNodeId(usize);

#[derive(Debug)]
pub enum Drawables {
    Rect(QuadInstance),
    TexturedRect(ImageInstance),
}

#[derive(Debug, Clone, Copy)]
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

    pub fn to_f32_arr(&self) -> [f32; 4] {
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
    fn layout(&self, view_size: BoundingBox, drawables: &mut Vec<Drawables>) {
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.color.to_f32_arr(),
        }));
    }
}

#[derive(Debug)]
pub struct TexturedRectangle {
    texture_id: TextureId,
    tint: Color,
}

impl TexturedRectangle {
    fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        drawables: &mut Vec<Drawables>,
    ) {
        drawables.push(Drawables::TexturedRect(ImageInstance::add_instance(
            atlas,
            self.texture_id,
            [view_size.min.0, view_size.min.1],
            [view_size.width(), view_size.height()],
            self.tint.to_f32_arr(),
        )));
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
    child: UiNodeId,
    background_color: Color,
}

impl FixedSizedBox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.background_color.to_f32_arr(),
        }));

        // The ceneter of the space we have
        let bbox_center = view_size.center();
        let half_width = self.width / 2.0;
        let half_height = self.height / 2.0;

        let fixed_size_bbox = BoundingBox::new(
            bbox_center.0 - half_width,
            bbox_center.1 - half_height,
            bbox_center.0 + half_width,
            bbox_center.1 + half_height,
        );

        let child = scene.node(self.child);
        child.layout(scene, atlas, fixed_size_bbox, queue, window, drawables);
    }
}

#[derive(Debug)]
pub enum TextAlign {
    Left,
    Center,
}

#[derive(Debug)]
pub struct TextDetails {
    text: Rc<str>,
    font_size: f32,
    text_color: Color,
    background_color: Color,
    align: TextAlign,
}

impl TextDetails {
    fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        drawables: &mut Vec<Drawables>,
    ) {
        // background color
        drawables.push(Drawables::Rect(QuadInstance {
            position: [view_size.min.0, view_size.min.1],
            size: [view_size.width(), view_size.height()],
            color: self.background_color.to_f32_arr(),
        }));

        match self.align {
            TextAlign::Left => drawables.extend(image_pipeline::layout_text(
                view_size,
                &self.text,
                atlas,
                self.font_size,
                queue,
                &self.text_color,
            )),
            TextAlign::Center => drawables.extend(image_pipeline::layout_text_centered(
                view_size,
                &self.text,
                atlas,
                self.font_size,
                queue,
                &self.text_color,
            )),
        }
    }
}

#[derive(Debug)]
pub struct Hbox {
    elements: Vec<UiNodeId>,
}

impl Hbox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        parent_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        for (i, id) in self.elements.iter().enumerate() {
            let child_index = i;
            let child_width = parent_size.width() / self.elements.len() as f32;
            let x0 = parent_size.min.0 + child_width * child_index as f32;

            let view_size =
                BoundingBox::new(x0, parent_size.min.1, x0 + child_width, parent_size.max.1);

            scene
                .node(*id)
                .layout(scene, atlas, view_size, queue, window, drawables);
        }
    }
}

#[derive(Debug)]
pub struct Vbox {
    elements: Vec<UiNodeId>,
}

impl Vbox {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        parent_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        for (i, id) in self.elements.iter().enumerate() {
            let child_index = self.elements.len() - i - 1;
            let child_height = parent_size.height() / self.elements.len() as f32;
            let y0 = parent_size.min.1 + child_height * child_index as f32;

            let view_size =
                BoundingBox::new(parent_size.min.0, y0, parent_size.max.0, y0 + child_height);

            scene
                .node(*id)
                .layout(scene, atlas, view_size, queue, window, drawables);
        }
    }
}

#[derive(Debug)]
pub enum Ui {
    TexturedRectangle(TexturedRectangle),
    FixedSizedBox(FixedSizedBox),
    Rectangle(Rectangle),
    Text(TextDetails),
    Hbox(Hbox),
    Vbox(Vbox),
    Spacer,
}

impl Ui {
    fn layout(
        &self,
        scene: &Scene,
        atlas: &mut TextureAtlas,
        view_size: BoundingBox,
        queue: &wgpu::Queue,
        window: &Window,
        drawables: &mut Vec<Drawables>,
    ) {
        match self {
            Ui::TexturedRectangle(tr) => tr.layout(atlas, view_size, drawables),
            Ui::FixedSizedBox(fsb) => fsb.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Rectangle(r) => r.layout(view_size, drawables),
            Ui::Text(td) => td.layout(atlas, view_size, queue, drawables),
            Ui::Hbox(h) => h.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Vbox(v) => v.layout(scene, atlas, view_size, queue, window, drawables),
            Ui::Spacer => {}
        }
    }
}

#[derive(Debug)]
pub struct Scene {
    nodes: Vec<Ui>,
    node_root: UiNodeId,
    cursor_pos: (f32, f32),
}

impl Scene {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            node_root: UiNodeId(0),
            cursor_pos: (0.0, 0.0),
        }
    }

    pub fn set_root(&mut self, root: UiNodeId) {
        self.node_root = root;
    }

    pub fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: (f32, f32),
        queue: &wgpu::Queue,
        window: &Window,
    ) -> Vec<Drawables> {
        let parent_size = BoundingBox {
            min: (0.0, 0.0),
            max: (view_size.0, view_size.1),
        };

        let mut drawables = vec![];

        self.node(self.node_root)
            .layout(self, atlas, parent_size, queue, window, &mut drawables);

        drawables
    }

    pub fn fixed_size_bbox(
        &mut self,
        width: f32,
        height: f32,
        child: UiNodeId,
        background_color: Color,
    ) -> UiNodeId {
        let obj = FixedSizedBox {
            width,
            height,
            child,
            background_color,
        };
        let idx = self.nodes.len();
        self.nodes.push(Ui::FixedSizedBox(obj));
        UiNodeId(idx)
    }

    pub fn textured_rectangle(&mut self, texture_id: TextureId) -> UiNodeId {
        let obj = TexturedRectangle {
            texture_id,
            tint: Color::new(255, 255, 255, 255),
        };
        let idx = self.nodes.len();
        self.nodes.push(Ui::TexturedRectangle(obj));
        UiNodeId(idx)
    }

    pub fn textured_rectangle_tinted(&mut self, texture_id: TextureId, tint: Color) -> UiNodeId {
        let obj = TexturedRectangle { texture_id, tint };
        let idx = self.nodes.len();
        self.nodes.push(Ui::TexturedRectangle(obj));
        UiNodeId(idx)
    }

    pub fn rectangle(&mut self, color: Color) -> UiNodeId {
        let obj = Rectangle { color };
        let idx = self.nodes.len();
        self.nodes.push(Ui::Rectangle(obj));
        UiNodeId(idx)
    }

    pub fn text_details(
        &mut self,
        text: Rc<str>,
        font_size: f32,
        text_color: Color,
        background_color: Color,
        align: TextAlign,
    ) -> UiNodeId {
        let obj = TextDetails {
            text,
            font_size,
            text_color,
            background_color,
            align,
        };
        let idx = self.nodes.len();
        self.nodes.push(Ui::Text(obj));
        UiNodeId(idx)
    }

    pub fn hbox(&mut self, elements: Vec<UiNodeId>) -> UiNodeId {
        let obj = Hbox { elements };
        let idx = self.nodes.len();
        self.nodes.push(Ui::Hbox(obj));
        UiNodeId(idx)
    }

    pub fn vbox(&mut self, elements: Vec<UiNodeId>) -> UiNodeId {
        let obj = Vbox { elements };
        let idx = self.nodes.len();
        self.nodes.push(Ui::Vbox(obj));
        UiNodeId(idx)
    }

    pub fn update_cursor_pos(&mut self, cx: f32, cy: f32) {
        self.cursor_pos = (cx, cy);
    }

    fn node(&self, id: UiNodeId) -> &Ui {
        &self.nodes[id.0]
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
        (self.min.0, self.min.1)
    }

    // Returns true if pos is inside the bbox.
    pub fn inside(&self, pos: (f32, f32)) -> bool {
        let x_inside = self.min.0 <= pos.0 && pos.0 <= self.max.0;
        let y_inside = self.min.1 <= pos.1 && pos.1 <= self.max.1;

        x_inside && y_inside
    }
}
