use winit::window::Window;

use crate::{
    image_pipeline::{self, ImageInstance},
    quad_pipeline::QuadInstance,
    texture_atlas::{TextureAtlas, TextureId},
};
use std::rc::Rc;

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
    pub fn new(
        text: Rc<str>,
        font_size: f32,
        text_color: Color,
        background_color: Color,
        align: TextAlign,
    ) -> Self {
        Self {
            text,
            font_size,
            text_color,
            background_color,
            align,
        }
    }
}

#[derive(Debug)]
pub enum ButtonState {
    Hovered,
    Pressed,
    Initial,
}

#[derive(Debug)]
pub struct Button {
    state: ButtonState,
    initial_color: Color,
    hover_color: Color,
    pressed_color: Color,
    text: TextDetails,
}

impl Button {
    pub fn new(
        state: ButtonState,
        initial_color: Color,
        hover_color: Color,
        pressed_color: Color,
        text: TextDetails,
    ) -> Self {
        Self {
            state,
            hover_color,
            pressed_color,
            initial_color,
            text,
        }
    }
}

pub struct UiState {
    cursor_x: f32,
    cursor_y: f32,
    layout_tree: Ui,
}

impl UiState {
    pub fn new(layout_tree: Ui) -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            layout_tree,
        }
    }

    pub fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: (f32, f32),
        queue: &wgpu::Queue,
        window: &Window,
    ) -> Vec<Drawables> {
        self.layout_tree
            .layout(atlas, view_size, queue, self, window)
    }

    pub fn update_cursor_pos(&mut self, cx: f32, cy: f32) {
        self.cursor_x = cx;
        self.cursor_y = cy;
    }
}

#[derive(Debug)]
pub enum Ui {
    TexturedRectangle(TexturedRectangle),
    FixedSizedBox(FixedSizedBox),
    Rectangle(Rectangle),
    Text(TextDetails),
    Button(Button),
    Hbox(Hbox),
    Vbox(Vbox),
    Spacer,
}

impl Ui {
    pub fn layout(
        &self,
        atlas: &mut TextureAtlas,
        view_size: (f32, f32),
        queue: &wgpu::Queue,
        ui_state: &UiState,
        window: &Window,
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
                ui_state,
                window,
            ),
            Ui::Vbox(v) => v.layout(
                atlas,
                BoundingBox {
                    min: (0.0, 0.0),
                    max: (view_size.0, view_size.1),
                },
                &mut rects,
                queue,
                ui_state,
                window,
            ),
            _ => unimplemented!(),
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
        (self.min.0, self.min.1)
    }

    // Returns true if pos is inside the bbox.
    pub fn inside(&self, pos: (f32, f32)) -> bool {
        let x_inside = self.min.0 <= pos.0 && pos.0 <= self.max.0;
        let y_inside = self.min.1 <= pos.1 && pos.1 <= self.max.1;

        x_inside && y_inside
    }
}

#[derive(Debug)]
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
        ui_state: &UiState,
        window: &Window,
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
                Ui::Hbox(hbox) => hbox.layout(atlas, child_bbox, rects, queue, ui_state, window),
                Ui::Vbox(vbox) => vbox.layout(atlas, child_bbox, rects, queue, ui_state, window),
                Ui::FixedSizedBox(fsb) => {
                    // The ceneter of the space we have
                    let bbox_center = child_bbox.center();
                    let half_width = fsb.width / 2.0;
                    let half_height = fsb.height / 2.0;

                    let fixed_size_bbox = BoundingBox::new(
                        bbox_center.0 - half_width,
                        bbox_center.1 - half_height,
                        bbox_center.0 + half_width,
                        bbox_center.1 + half_height,
                    );

                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: fsb.background_color.to_f32_arr(),
                    }));

                    fsb.layout(atlas, fixed_size_bbox, rects, queue, ui_state, window);
                }
                Ui::Button(b) => {
                    let color = match b.state {
                        ButtonState::Hovered => &b.hover_color,
                        ButtonState::Pressed => &b.pressed_color,
                        ButtonState::Initial => {
                            if child_bbox.inside((ui_state.cursor_x, ui_state.cursor_y)) {
                                window.set_cursor_icon(winit::window::CursorIcon::Pointer);
                                &b.hover_color
                            } else {
                                window.set_cursor_icon(winit::window::CursorIcon::Default);
                                &b.initial_color
                            }
                        }
                    };

                    // Use the button's background color of text to draw
                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: color.to_f32_arr(),
                    }));

                    // TODO: construct a temporary UI and call layout on it to avoid duplication?
                    match b.text.align {
                        TextAlign::Left => rects.extend(image_pipeline::layout_text(
                            child_bbox,
                            &b.text.text,
                            atlas,
                            b.text.font_size,
                            queue,
                            &b.text.text_color,
                        )),
                        TextAlign::Center => rects.extend(image_pipeline::layout_text_centered(
                            child_bbox,
                            &b.text.text,
                            atlas,
                            b.text.font_size,
                            queue,
                            &b.text.text_color,
                        )),
                    }
                }
                Ui::Text(td) => {
                    // background color
                    rects.push(Drawables::Rect(QuadInstance {
                        position: [child_bbox.min.0, child_bbox.min.1],
                        size: [child_bbox.width(), child_bbox.height()],
                        color: td.background_color.to_f32_arr(),
                    }));

                    match td.align {
                        TextAlign::Left => rects.extend(image_pipeline::layout_text(
                            child_bbox,
                            &td.text,
                            atlas,
                            td.font_size,
                            queue,
                            &td.text_color,
                        )),
                        TextAlign::Center => rects.extend(image_pipeline::layout_text_centered(
                            child_bbox,
                            &td.text,
                            atlas,
                            td.font_size,
                            queue,
                            &td.text_color,
                        )),
                    }
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
